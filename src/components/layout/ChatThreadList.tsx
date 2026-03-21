import { useMemo, useState } from "react";
import { AlertTriangle, RefreshCw, Trash2 } from "lucide-react";
import { Button, Modal } from "@heroui/react";
import type { ChatSession } from "../../services/tauri";

interface ChatThreadListProps {
  sessions: ChatSession[];
  activeChatId: string | null;
  onSwitchChat: (chatId: string) => void;
  onRefresh?: () => void;
  onDeleteChat?: (chatId: string) => void;
  emptyLabel?: string;
  showRefresh?: boolean;
  activeRunChatIds?: Set<string>;
}

function timeAgo(dateStr: string): string {
  const then = new Date(dateStr).getTime();
  const now = Date.now();
  const diffMs = now - then;
  const minutes = Math.floor(diffMs / 60000);
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d`;
  const weeks = Math.floor(days / 7);
  return `${weeks}w`;
}

export function ChatThreadList({
  sessions,
  activeChatId,
  onSwitchChat,
  onRefresh,
  onDeleteChat,
  emptyLabel = "No chats yet",
  showRefresh = false,
  activeRunChatIds = new Set<string>(),
}: ChatThreadListProps) {
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);

  const pendingDeleteSession = useMemo(
    () => sessions.find((session) => session.id === pendingDeleteId) ?? null,
    [pendingDeleteId, sessions],
  );

  const closeDeleteDialog = () => {
    setPendingDeleteId(null);
  };

  if (sessions.length === 0) {
    return (
      <div className="rounded-2xl border border-dashed border-border/50 bg-background/25 px-3 py-4 text-center">
        <p className="text-[11px] text-muted-foreground">{emptyLabel}</p>
      </div>
    );
  }

  return (
    <div className="space-y-1">
      {sessions.map((session) => {
        const isActive = session.id === activeChatId;
        const isRunning = activeRunChatIds.has(session.id);
        const title = session.title?.trim() || "New chat";
        const timeLabel = session.last_message_at
          ? timeAgo(session.last_message_at)
          : session.updated_at
            ? timeAgo(session.updated_at)
            : "";

        return (
          <div
            key={session.id}
            className={`group flex items-center gap-1 rounded-2xl pr-1 transition-colors ${
              isActive ? "bg-primary/10" : "hover:bg-white/8"
            }`}
          >
            <button
              type="button"
              onClick={() => onSwitchChat(session.id)}
              className={`flex min-w-0 flex-1 items-center gap-2 px-3 py-2.5 text-left ${
                isActive ? "text-primary" : "text-muted-foreground hover:text-foreground"
              }`}
            >
              {isRunning && (
                <span className="inline-block h-2 w-2 flex-shrink-0 rounded-full bg-primary animate-pulse" />
              )}
              <span className="truncate text-[13px] font-medium">{title}</span>
              {timeLabel && (
                <span className="shrink-0 text-[10px] font-medium text-muted-foreground/55">
                  {timeLabel}
                </span>
              )}
            </button>

            {showRefresh && isActive && onRefresh && (
              <Button
                variant="ghost"
                size="sm"
                isIconOnly
                className="size-7 rounded-xl text-muted-foreground/50 hover:bg-white/10 hover:text-foreground"
                onPress={onRefresh}
              >
                <RefreshCw className="size-3.5" />
              </Button>
            )}

            {onDeleteChat && (
              <Button
                variant="ghost"
                size="sm"
                isIconOnly
                className="size-7 rounded-xl text-muted-foreground/40 opacity-0 transition-opacity hover:bg-destructive/10 hover:text-destructive group-hover:opacity-100"
                onPress={() => setPendingDeleteId(session.id)}
              >
                <Trash2 className="size-3.5" />
              </Button>
            )}
          </div>
        );
      })}

      <Modal.Backdrop
        isOpen={pendingDeleteSession !== null}
        onOpenChange={(open) => {
          if (!open) {
            closeDeleteDialog();
          }
        }}
        className="z-[9999] bg-background/80 backdrop-blur-md dark:bg-background/30"
      >
        <Modal.Container>
          <Modal.Dialog className="w-full max-w-md overflow-hidden rounded-2xl border border-border/20 bg-background/85 shadow-2xl backdrop-blur-md dark:bg-background/20">
            <Modal.Header className="relative overflow-hidden border-b border-border/10 px-6 pb-4 pt-5">
              <div className="pointer-events-none rounded-xl absolute inset-0 bg-gradient-to-r from-destructive/10 to-transparent" />
              <div className="relative z-10 flex items-center gap-3">
                <div className="flex size-10 items-center justify-center rounded-2xl bg-destructive/10 text-destructive shadow-inner shadow-destructive/10">
                  <AlertTriangle className="size-5" />
                </div>
                <div>
                  <Modal.Heading className="text-xl font-bold tracking-tight text-foreground">
                    Delete chat?
                  </Modal.Heading>
                  <p className="mt-1 text-sm text-muted-foreground">
                    This permanently removes the transcript, compaction state, and runtime telemetry for this chat.
                  </p>
                </div>
              </div>
            </Modal.Header>

            <Modal.Body className="space-y-5 px-6 py-5">
              <div className="rounded-xl border border-border/10 bg-background/30 p-3.5 dark:bg-background/10">
                <p className="mb-1 text-[10px] font-bold uppercase tracking-widest text-muted-foreground">
                  Target Chat
                </p>
                <div className="truncate text-sm font-semibold text-foreground">
                  {pendingDeleteSession?.title?.trim() || "New chat"}
                </div>
                <div className="mt-3 grid grid-cols-2 gap-3 text-xs text-muted-foreground">
                  <div>
                    <div className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground/70">
                      Messages
                    </div>
                    <div className="mt-1 text-foreground/80">
                      {pendingDeleteSession?.message_count ?? 0}
                    </div>
                  </div>
                  <div>
                    <div className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground/70">
                      Last Activity
                    </div>
                    <div className="mt-1 text-foreground/80">
                      {pendingDeleteSession?.last_message_at
                        ? timeAgo(pendingDeleteSession.last_message_at)
                        : pendingDeleteSession?.updated_at
                          ? timeAgo(pendingDeleteSession.updated_at)
                          : "none"}
                    </div>
                  </div>
                </div>
              </div>

              {pendingDeleteSession?.id === activeChatId && (
                <div className="rounded-xl border border-amber-500/20 bg-amber-500/12 px-4 py-3 text-sm text-amber-800 dark:text-amber-300">
                  This is the active chat. After deletion, the app will switch to another available chat or create a fresh empty one.
                </div>
              )}

              <div className="rounded-xl border border-emerald-500/15 bg-emerald-500/8 px-4 py-3 text-sm text-muted-foreground">
                Workspace memory is preserved. Only this chat session is deleted.
              </div>
            </Modal.Body>

            <Modal.Footer className="flex justify-end gap-3 border-t border-border/10 bg-background/20 px-6 py-4 dark:bg-background/5">
              <Button
                className="rounded-xl border border-border/10 bg-background/40 px-5 font-medium text-muted-foreground backdrop-blur-sm transition-all hover:bg-background/60 hover:text-foreground dark:bg-background/10 dark:hover:bg-background/20"
                onPress={closeDeleteDialog}
              >
                Cancel
              </Button>
              <Button
                className="rounded-xl px-6 font-semibold text-white shadow-lg shadow-destructive/20 transition-all hover:bg-destructive/90 bg-destructive"
                onPress={() => {
                  if (pendingDeleteId && onDeleteChat) {
                    onDeleteChat(pendingDeleteId);
                  }
                  closeDeleteDialog();
                }}
              >
                Delete chat
              </Button>
            </Modal.Footer>
          </Modal.Dialog>
        </Modal.Container>
      </Modal.Backdrop>
    </div>
  );
}
