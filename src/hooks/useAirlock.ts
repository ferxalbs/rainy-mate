import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { ApprovalRequest } from "../types";

export function useAirlock() {
  const [pendingRequests, setPendingRequests] = useState<ApprovalRequest[]>([]);

  useEffect(() => {
    // Listen for new requests
    const unlisten = listen<ApprovalRequest>(
      "airlock:approval_required",
      (event) => {
        console.log("Airlock Request:", event.payload);
        setPendingRequests((prev) => {
          // Prevent duplicates
          if (prev.some((r) => r.commandId === event.payload.commandId))
            return prev;
          return [...prev, event.payload];
        });
      },
    );

    // Check for existing pending requests on mount
    invoke<string[]>("get_pending_airlock_approvals")
      .then((ids) => {
        if (ids && ids.length > 0) {
          console.log("Pending approvals detected:", ids);
          // Note: We don't get the full object here, only IDs.
          // The event carries the full object.
          // If the user refreshed the page, they might see an empty list until next event/refresh logic is improved.
          // Future TODO: Update backend to return full ApprovalRequest objects
        }
      })
      .catch((e) => console.error("Failed to check pending approvals:", e));

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const respond = useCallback(async (commandId: string, approved: boolean) => {
    try {
      await invoke("respond_to_airlock", { commandId, approved });
      setPendingRequests((prev) =>
        prev.filter((r) => r.commandId !== commandId),
      );
    } catch (e) {
      console.error("Failed to respond to airlock:", e);
    }
  }, []);

  return {
    pendingRequests,
    respond,
  };
}
