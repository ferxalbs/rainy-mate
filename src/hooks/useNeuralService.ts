import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  registerNode,
  sendHeartbeat,
  respondToAirlock,
  getPendingAirlockApprovals,
  ApprovalRequest,
  SkillManifest,
  DesktopNodeStatus,
  setHeadlessMode,
  AirlockLevels,
} from "../services/tauri";
import { toast } from "@heroui/react";

// Default skills this Desktop Node exposes
const DEFAULT_SKILLS: SkillManifest[] = [
  {
    name: "file_ops",
    version: "1.0.0",
    methods: [
      {
        name: "read_file",
        description: "Read file content",
        airlockLevel: AirlockLevels.Safe,
        parameters: {
          path: {
            type: "string",
            description: "Absolute path to file",
            required: true,
          },
        },
      },
      {
        name: "write_file",
        description: "Write content to file",
        airlockLevel: AirlockLevels.Sensitive,
        parameters: {
          path: {
            type: "string",
            description: "Absolute path to file",
            required: true,
          },
          content: {
            type: "string",
            description: "Content to write",
            required: true,
          },
        },
      },
    ],
  },
  {
    name: "terminal",
    version: "1.0.0",
    methods: [
      {
        name: "exec",
        description: "Execute terminal command",
        airlockLevel: AirlockLevels.Dangerous,
        parameters: {
          command: {
            type: "string",
            description: "Command to execute",
            required: true,
          },
          cwd: {
            type: "string",
            description: "Working directory",
            required: false,
          },
        },
      },
    ],
  },
];

export function useNeuralService() {
  const [status, setStatus] = useState<DesktopNodeStatus>("pending-pairing");
  const [nodeId, setNodeId] = useState<string | null>(null);
  const [pendingApprovals, setPendingApprovals] = useState<ApprovalRequest[]>(
    [],
  );
  const [lastHeartbeat, setLastHeartbeat] = useState<Date | null>(null);

  // Initial Registration / Handshake
  const connect = useCallback(async () => {
    try {
      console.log("Registering Neural Node...");
      const id = await registerNode(DEFAULT_SKILLS, []);
      setNodeId(id);
      setStatus("connected");
      toast.success("Connected to Neural Network");

      // Start Heartbeat Loop
      startHeartbeat();
    } catch (error) {
      console.error("Failed to register node:", error);
      setStatus("error");
      toast.danger("Failed to connect to Neural Network");
    }
  }, []);

  const startHeartbeat = useCallback(() => {
    const interval = setInterval(async () => {
      try {
        await sendHeartbeat();
        setLastHeartbeat(new Date());
      } catch (err) {
        console.error("Heartbeat failed:", err);
        // Optionally set status to offline if multiple fail
      }
    }, 5000); // 5 seconds

    return () => clearInterval(interval);
  }, []);

  // Listen for Airlock Events
  useEffect(() => {
    const unlisten = listen<ApprovalRequest>(
      "airlock:approval_required",
      (event) => {
        console.log("Airlock Approval Required:", event.payload);
        setPendingApprovals((prev) => [...prev, event.payload]);
        toast("Security Alert", {
          description: `Permission required for ${event.payload.command_type}`,
          actionProps: {
            children: "Review",
            onPress: () => {
              /* Navigate to review */
            },
          },
        });
      },
    );

    // Load initial pending approvals
    getPendingAirlockApprovals().then(setPendingApprovals).catch(console.error);

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const respond = useCallback(async (requestId: string, approved: boolean) => {
    try {
      await respondToAirlock(requestId, approved);
      setPendingApprovals((prev) => prev.filter((req) => req.id !== requestId));
      toast.success(approved ? "Request Approved" : "Request Denied");
    } catch (error) {
      console.error("Failed to respond to airlock:", error);
      toast.danger("Failed to process response");
    }
  }, []);

  const [isHeadless, setIsHeadless] = useState(false);

  const toggleHeadless = useCallback(async (enabled: boolean) => {
    try {
      await setHeadlessMode(enabled);
      setIsHeadless(enabled);
      toast.success(`Headless Mode ${enabled ? "Enabled" : "Disabled"}`);
    } catch (error) {
      console.error("Failed to set headless mode:", error);
      toast.danger("Failed to update settings");
    }
  }, []);

  return {
    status,
    nodeId,
    pendingApprovals,
    lastHeartbeat,
    connect,
    respond,
    isPending: status === "pending-pairing",
    isHeadless,
    toggleHeadless,
  };
}
