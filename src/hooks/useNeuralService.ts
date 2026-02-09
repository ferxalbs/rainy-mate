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
    name: "filesystem",
    version: "1.0.0",
    methods: [
      {
        name: "read_file",
        description: "Read file content",
        airlockLevel: AirlockLevels.Safe,
        parameters: {
          path: {
            type: "string",
            description: "Path to file",
            required: true,
          },
        },
      },
      {
        name: "list_files",
        description: "List files in a directory",
        airlockLevel: AirlockLevels.Safe,
        parameters: {
          path: {
            type: "string",
            description: "Directory path",
            required: true,
          },
        },
      },
      {
        name: "search_files",
        description: "Search files by query",
        airlockLevel: AirlockLevels.Safe,
        parameters: {
          query: {
            type: "string",
            description: "Search query (regex supported)",
            required: true,
          },
          path: {
            type: "string",
            description: "Root path to search",
            required: false,
          },
          search_content: {
            type: "boolean",
            description: "Search within file contents",
            required: false,
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
            description: "Path to file",
            required: true,
          },
          content: {
            type: "string",
            description: "Content to write",
            required: true,
          },
        },
      },
      {
        name: "append_file",
        description: "Append content to file",
        airlockLevel: AirlockLevels.Sensitive,
        parameters: {
          path: {
            type: "string",
            description: "Path to file",
            required: true,
          },
          content: {
            type: "string",
            description: "Content to append",
            required: true,
          },
        },
      },
    ],
  },
  {
    name: "shell",
    version: "1.0.0",
    methods: [
      {
        name: "execute_command",
        description: "Execute a shell command",
        airlockLevel: AirlockLevels.Dangerous,
        parameters: {
          command: {
            type: "string",
            description: "Command to execute (whitelisted)",
            required: true,
          },
          args: {
            type: "array",
            description: "Command arguments",
            required: true,
          },
        },
      },
    ],
  },
  {
    name: "web",
    version: "1.0.0",
    methods: [
      {
        name: "web_search",
        description: "Search the web",
        airlockLevel: AirlockLevels.Safe,
        parameters: {
          query: {
            type: "string",
            description: "Search query",
            required: true,
          },
        },
      },
      {
        name: "read_web_page",
        description: "Read a web page",
        airlockLevel: AirlockLevels.Safe,
        parameters: {
          url: {
            type: "string",
            description: "URL to read",
            required: true,
          },
        },
      },
    ],
  },
  {
    name: "browser",
    version: "1.0.0",
    methods: [
      {
        name: "browse_url",
        description: "Open a URL in the browser",
        airlockLevel: AirlockLevels.Safe,
        parameters: {
          url: {
            type: "string",
            description: "URL to open",
            required: true,
          },
        },
      },
      {
        name: "click_element",
        description: "Click an element by CSS selector",
        airlockLevel: AirlockLevels.Sensitive,
        parameters: {
          selector: {
            type: "string",
            description: "CSS selector",
            required: true,
          },
        },
      },
      {
        name: "screenshot",
        description: "Take a screenshot of the current page",
        airlockLevel: AirlockLevels.Safe,
        parameters: {},
      },
      {
        name: "get_page_content",
        description: "Get HTML content of the current page",
        airlockLevel: AirlockLevels.Safe,
        parameters: {},
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
        await sendHeartbeat("connected");
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
        setPendingApprovals((prev) => {
          if (prev.some((req) => req.commandId === event.payload.commandId)) {
            return prev;
          }
          return [...prev, event.payload];
        });
        toast("Security Alert", {
          description: `Permission required for ${event.payload.intent}`,
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
    getPendingAirlockApprovals()
      .then((ids) => {
        if (ids.length > 0) {
          console.log("Pending approval IDs detected:", ids);
        }
      })
      .catch(console.error);

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const respond = useCallback(async (commandId: string, approved: boolean) => {
    try {
      await respondToAirlock(commandId, approved);
      setPendingApprovals((prev) =>
        prev.filter((req) => req.commandId !== commandId),
      );
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
