import type { AgentMessage } from "../../../../types/agent";

interface UserMessageRowProps {
  message: AgentMessage;
}

export function UserMessageRow({ message }: UserMessageRowProps) {
  const images = message.attachments?.filter(a => a.type === "image") || [];

  return (
    <div className="flex w-full justify-end mb-6">
      <div className="group relative max-w-[85%] rounded-3xl rounded-br-md bg-secondary/60 px-5 py-4 shadow-sm border border-white/5 backdrop-blur-md">
        {images.length > 0 && (
          <div className="mb-3 grid grid-cols-2 gap-2 max-w-[420px]">
            {images.map((img) => (
              <div key={img.id} className="overflow-hidden rounded-xl border border-white/10">
                {img.thumbnailDataUri ? (
                  <img src={img.thumbnailDataUri} alt={img.filename} className="h-auto w-full max-h-[200px] object-cover" />
                ) : (
                   <div className="p-4 text-xs text-muted-foreground">{img.filename}</div>
                )}
              </div>
            ))}
          </div>
        )}
        <div className="whitespace-pre-wrap text-[15px] font-normal leading-relaxed text-foreground/90">
          {message.content}
        </div>
      </div>
    </div>
  );
}
