import { RefreshCw, Trash2 } from "lucide-react";
import { Button } from "@heroui/react";
import type { ChatSession } from "../../services/tauri";

interface ChatThreadListProps {
  sessions: ChatSession[];
  activeChatId: string | null;
  onSwitchChat: (chatId: string) => void;
  onRefresh?: () => void;
  onDeleteChat?: (chatId: string) => void;
  emptyLabel?: string;
  showRefresh?: boolean;
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
}: ChatThreadListProps) {
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
                onPress={() => onDeleteChat(session.id)}
              >
                <Trash2 className="size-3.5" />
              </Button>
            )}
          </div>
        );
      })}
    </div>
  );
}
