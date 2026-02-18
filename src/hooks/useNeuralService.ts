import { useState, useCallback, useEffect, useRef } from "react";
import {
  registerNode,
  sendHeartbeat,
  DesktopNodeStatus,
  setHeadlessMode,
} from "../services/tauri";
import { useAirlock } from "./useAirlock";
import { toast } from "@heroui/react";

export function useNeuralService() {
  const [status, setStatus] = useState<DesktopNodeStatus>("pending-pairing");
  const [nodeId, setNodeId] = useState<string | null>(null);
  const { pendingRequests: pendingApprovals, respond: respondAirlock } =
    useAirlock();
  const [lastHeartbeat, setLastHeartbeat] = useState<Date | null>(null);
  const heartbeatIntervalRef = useRef<ReturnType<typeof setInterval> | null>(
    null,
  );

  const stopHeartbeat = useCallback(() => {
    if (heartbeatIntervalRef.current) {
      clearInterval(heartbeatIntervalRef.current);
      heartbeatIntervalRef.current = null;
    }
  }, []);

  const startHeartbeat = useCallback(() => {
    stopHeartbeat();
    heartbeatIntervalRef.current = setInterval(async () => {
      try {
        await sendHeartbeat("online");
        setLastHeartbeat(new Date());
      } catch (err) {
        console.error("Heartbeat failed:", err);
      }
    }, 5000);
  }, [stopHeartbeat]);

  // Initial Registration / Handshake
  const connect = useCallback(async () => {
    try {
      console.log("Registering Neural Node...");
      const id = await registerNode();
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
  }, [startHeartbeat]);

  useEffect(() => {
    return () => {
      stopHeartbeat();
    };
  }, [stopHeartbeat]);

  const respond = useCallback(async (commandId: string, approved: boolean) => {
    try {
      await respondAirlock(commandId, approved);
      toast.success(approved ? "Request Approved" : "Request Denied");
    } catch (error) {
      console.error("Failed to respond to airlock:", error);
      toast.danger("Failed to process response");
    }
  }, [respondAirlock]);

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
