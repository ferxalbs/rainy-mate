import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  getPendingAirlockApprovals,
  respondToAirlock,
} from "../services/tauri";
import type { ApprovalRequest } from "../services/tauri";

export function useAirlock() {
  const [pendingRequests, setPendingRequests] = useState<ApprovalRequest[]>([]);

  useEffect(() => {
    // Listen for new requests
    const unlistenNew = listen<ApprovalRequest>(
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

    // Listen for resolved/timed-out requests (cleanup)
    const unlistenResolved = listen<string>(
      "airlock:approval_resolved",
      (event) => {
        const commandId = event.payload;
        console.log("Airlock Resolved:", commandId);
        setPendingRequests((prev) =>
          prev.filter((r) => r.commandId !== commandId),
        );
      },
    );

    const unlistenClicked = listen<string | null>(
      "airlock:notification_clicked",
      (event) => {
        const commandId = event.payload;
        if (!commandId) return;
        setPendingRequests((prev) => {
          const index = prev.findIndex((request) => request.commandId === commandId);
          if (index <= 0) return prev;
          const next = [...prev];
          const [request] = next.splice(index, 1);
          next.unshift(request);
          return next;
        });
      },
    );

    // Check for existing pending requests on mount
    getPendingAirlockApprovals()
      .then((requests) => {
        if (!requests || requests.length === 0) return;

        setPendingRequests((prev) => {
          const existingIds = new Set(prev.map((r) => r.commandId));
          const next = [...prev];
          for (const request of requests) {
            if (!existingIds.has(request.commandId)) {
              next.push(request);
            }
          }
          return next;
        });
      })
      .catch((e) => console.error("Failed to check pending approvals:", e));

    return () => {
      unlistenNew.then((f) => f());
      unlistenResolved.then((f) => f());
      unlistenClicked.then((f) => f());
    };
  }, []);

  const respond = useCallback(async (commandId: string, approved: boolean) => {
    await respondToAirlock(commandId, approved);
    setPendingRequests((prev) => prev.filter((r) => r.commandId !== commandId));
  }, []);

  return {
    pendingRequests,
    respond,
  };
}
