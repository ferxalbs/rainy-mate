import { Paperclip } from "lucide-react";

import type { AgentMessage } from "../../../../types/agent";

interface UserMessageRowProps {
  message: AgentMessage;
}

export function UserMessageRow({ message }: UserMessageRowProps) {
  const attachments = message.attachments ?? [];
  const imageAttachments = attachments.filter((attachment) => attachment.type === "image");
  const fileAttachments = attachments.filter((attachment) => attachment.type !== "image");

  return (
    <div className="mb-6 flex w-full justify-end">
      <div className="w-full max-w-[min(100%,56rem)]">
        <div className="ml-auto w-fit max-w-[min(100%,42rem)] rounded-[22px] border border-border/70 bg-secondary/72 px-5 py-4 shadow-[0_24px_80px_-64px_rgba(0,0,0,0.6)] backdrop-blur-xl">
          {imageAttachments.length > 0 ? (
            <div className="mb-3 grid max-w-[420px] grid-cols-2 gap-2">
              {imageAttachments.map((img) => (
                <div key={img.id} className="overflow-hidden rounded-2xl border border-border/70 bg-background/80">
                  {img.thumbnailDataUri ? (
                    <img
                      src={img.thumbnailDataUri}
                      alt={img.filename}
                      className="h-auto max-h-[220px] w-full object-cover"
                    />
                  ) : (
                    <div className="p-4 text-xs text-muted-foreground">{img.filename}</div>
                  )}
                </div>
              ))}
            </div>
          ) : null}

          {fileAttachments.length > 0 ? (
            <div className="mb-3 flex flex-wrap gap-2">
              {fileAttachments.map((attachment) => (
                <div
                  key={attachment.id}
                  className="inline-flex items-center gap-2 rounded-full border border-border/70 bg-background/85 px-3 py-1.5 text-xs text-foreground/85"
                >
                  <Paperclip className="size-3.5 text-muted-foreground" />
                  <span className="max-w-[180px] truncate">{attachment.filename}</span>
                </div>
              ))}
            </div>
          ) : null}

          <div className="whitespace-pre-wrap text-[15px] leading-relaxed text-foreground/92">
            {message.content}
          </div>
        </div>

        <div className="mt-1.5 px-1 text-right text-[10px] text-muted-foreground/40">
          {message.timestamp.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
        </div>
      </div>
    </div>
  );
}
