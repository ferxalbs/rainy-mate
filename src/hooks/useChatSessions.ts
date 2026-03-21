import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { ChatSession } from "../services/tauri";
import * as tauri from "../services/tauri";

interface UseChatSessionsOptions {
  activeWorkspaceId: string;
}

type SessionsByWorkspace = Record<string, ChatSession[]>;

function hasChat(list: ChatSession[], chatId: string | null): boolean {
  return Boolean(chatId && list.some((session) => session.id === chatId));
}

function isEmptyDraft(session: ChatSession): boolean {
  return session.message_count === 0 && !session.last_message_at;
}

export function useChatSessions({
  activeWorkspaceId,
}: UseChatSessionsOptions) {
  const [sessionsByWorkspace, setSessionsByWorkspace] = useState<SessionsByWorkspace>({});
  const [activeChatId, setActiveChatId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [activeRunChatIds, setActiveRunChatIds] = useState<Set<string>>(new Set());
  const creationInFlightRef = useRef<Record<string, Promise<ChatSession | null>>>({});

  const refreshWorkspaceSessions = useCallback(async (workspaceId: string) => {
    if (!workspaceId) return [];
    try {
      const list = await tauri.listChatSessions(workspaceId);
      setSessionsByWorkspace((prev) => ({
        ...prev,
        [workspaceId]: list,
      }));
      return list;
    } catch (error) {
      console.error(`Failed to load chat sessions for ${workspaceId}:`, error);
      return [];
    }
  }, []);

  const ensureWorkspaceChat = useCallback(async (workspaceId: string) => {
    if (!workspaceId) return null;

    const nextActiveChatId = await tauri.getOrCreateWorkspaceChat(workspaceId);
    const sessions = await refreshWorkspaceSessions(workspaceId);

    setActiveChatId((current) => (
      hasChat(sessions, current) ? current : nextActiveChatId
    ));

    return nextActiveChatId;
  }, [refreshWorkspaceSessions]);

  useEffect(() => {
    if (!activeWorkspaceId) return;

    let cancelled = false;
    setIsLoading(true);

    void ensureWorkspaceChat(activeWorkspaceId)
      .catch((error) => {
        console.error("Failed to initialize active workspace chat:", error);
      })
      .finally(() => {
        if (!cancelled) {
          setIsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [activeWorkspaceId, ensureWorkspaceChat]);

  // Seed active run IDs on mount
  useEffect(() => {
    tauri.listActiveSessions()
      .then((sessions) => {
        setActiveRunChatIds(new Set(sessions.map((s) => s.chatId)));
      })
      .catch(() => {/* non-critical */});
  }, []);

  // Listen for remote sessions started/finished
  useEffect(() => {
    let unlisten1: (() => void) | undefined;
    let unlisten2: (() => void) | undefined;

    void listen<{ chatId: string; workspaceId: string }>("session://started", (event) => {
      const { chatId, workspaceId } = event.payload;
      setActiveRunChatIds((prev) => new Set([...prev, chatId]));
      void refreshWorkspaceSessions(workspaceId);
    }).then((fn) => { unlisten1 = fn; });

    void listen<{ chatId: string }>("session://finished", (event) => {
      const { chatId } = event.payload;
      setActiveRunChatIds((prev) => {
        const next = new Set(prev);
        next.delete(chatId);
        return next;
      });
      void refreshWorkspaceSessions(activeWorkspaceId);
    }).then((fn) => { unlisten2 = fn; });

    return () => {
      unlisten1?.();
      unlisten2?.();
    };
  }, [activeWorkspaceId, refreshWorkspaceSessions]);

  const createNewChat = useCallback(async (workspaceId = activeWorkspaceId) => {
    if (!workspaceId) return null;

    const existingSessions = sessionsByWorkspace[workspaceId] || [];
    const activeSession = existingSessions.find((session) => session.id === activeChatId);
    if (activeSession && isEmptyDraft(activeSession)) {
      setActiveChatId(activeSession.id);
      return activeSession;
    }

    const reusableDraft = existingSessions.find(isEmptyDraft);
    if (reusableDraft) {
      setActiveChatId(reusableDraft.id);
      return reusableDraft;
    }

    const inFlight = creationInFlightRef.current[workspaceId];
    if (inFlight) {
      const pendingChat = await inFlight;
      if (pendingChat) {
        setActiveChatId(pendingChat.id);
      }
      return pendingChat;
    }

    const creationPromise = (async () => {
      try {
        const chat = await tauri.createOrReuseEmptyChatSession(workspaceId);
        setSessionsByWorkspace((prev) => {
          const prior = prev[workspaceId] || [];
          const deduped = prior.filter((session) => session.id !== chat.id);
          return {
            ...prev,
            [workspaceId]: [chat, ...deduped],
          };
        });
        setActiveChatId(chat.id);
        return chat;
      } catch (error) {
        console.error(`Failed to create chat session for ${workspaceId}:`, error);
        return null;
      } finally {
        delete creationInFlightRef.current[workspaceId];
      }
    })();

    creationInFlightRef.current[workspaceId] = creationPromise;

    try {
      return await creationPromise;
    } finally {
      delete creationInFlightRef.current[workspaceId];
    }
  }, [activeWorkspaceId, activeChatId, sessionsByWorkspace]);

  const switchToChat = useCallback((chatId: string) => {
    setActiveChatId(chatId);
  }, []);

  const deleteChat = useCallback(async (workspaceId: string, chatId: string) => {
    try {
      await tauri.deleteChatSession(chatId);

      const updated = (sessionsByWorkspace[workspaceId] || []).filter((session) => session.id !== chatId);
      setSessionsByWorkspace((prev) => ({
        ...prev,
        [workspaceId]: updated,
      }));

      if (activeChatId !== chatId) {
        return;
      }

      if (updated.length > 0) {
        setActiveChatId(updated[0].id);
        return;
      }

      const replacement = await createNewChat(workspaceId);
      if (replacement) {
        setActiveChatId(replacement.id);
      }
    } catch (error) {
      console.error(`Failed to delete chat session ${chatId}:`, error);
    }
  }, [activeChatId, createNewChat, sessionsByWorkspace]);

  const refreshActiveWorkspace = useCallback(async () => {
    return refreshWorkspaceSessions(activeWorkspaceId);
  }, [activeWorkspaceId, refreshWorkspaceSessions]);

  return {
    sessionsByWorkspace,
    activeChatId,
    activeRunChatIds,
    isLoading,
    createNewChat,
    switchToChat,
    deleteChat,
    refreshWorkspaceSessions,
    refreshActiveWorkspace,
    ensureWorkspaceChat,
  };
}
