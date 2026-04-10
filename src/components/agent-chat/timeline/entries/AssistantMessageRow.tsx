import type { AgentMessage } from "../../../../types/agent";
import { MarkdownRenderer } from "../../MarkdownRenderer";

interface AssistantMessageRowProps {
  message: AgentMessage;
}

export function AssistantMessageRow({ message }: AssistantMessageRowProps) {
  const isStreaming = message.runState === "running" && !!message.content;

  return (
    <div className="w-full flex justify-start mb-6">
      <div className="flex flex-col w-full max-w-[85%]">
        <div className="rounded-[22px] px-1 py-1">
          {message.content ? (
             <MarkdownRenderer
                content={message.content}
                tone="assistant"
                isStreaming={isStreaming}
             />
          ) : (
             <span className="text-sm italic text-muted-foreground/50">Empty response</span>
          )}
        </div>
        <div className="px-2 mt-2 flex items-center justify-between">
           <span className="text-[10px] text-muted-foreground/40 font-medium">
             {message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
           </span>
        </div>
      </div>
    </div>
  );
}
