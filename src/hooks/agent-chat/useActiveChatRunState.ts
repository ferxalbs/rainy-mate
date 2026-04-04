import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import * as tauri from "../../services/tauri";

interface UseActiveChatRunStateResult {
  activeChatRun: tauri.ActiveSessionInfo | null;
  otherActiveRunsCount: number;
}

export function useActiveChatRunState(
  activeChatId?: string | null,
): UseActiveChatRunStateResult {
  const [activeSessions, setActiveSessions] = useState<tauri.ActiveSessionInfo[]>([]);

  const refreshActiveSessions = useCallback(async () => {
    try {
      const sessions = await tauri.listActiveSessions();
      setActiveSessions(sessions);
    } catch (error) {
      console.error("Failed to list active sessions:", error);
    }
  }, []);

  useEffect(() => {
    void refreshActiveSessions();
  }, [refreshActiveSessions]);

  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];

    const attach = async () => {
      const started = await listen("session://started", () => {
        void refreshActiveSessions();
      });
      if (cancelled) {
        started();
      } else {
        unlisteners.push(started);
      }

      const finished = await listen("session://finished", () => {
        void refreshActiveSessions();
      });
      if (cancelled) {
        finished();
      } else {
        unlisteners.push(finished);
      }
    };

    void attach();

    return () => {
      cancelled = true;
      unlisteners.forEach((fn) => fn());
    };
  }, [refreshActiveSessions]);

  const activeChatRun = activeSessions.find((session) => (
    activeChatId ? session.chatId === activeChatId : false
  )) ?? null;

  const otherActiveRunsCount = activeSessions.reduce((count, session) => {
    if (activeChatId && session.chatId === activeChatId) {
      return count;
    }
    return count + 1;
  }, 0);

  return {
    activeChatRun,
    otherActiveRunsCount,
  };
}
