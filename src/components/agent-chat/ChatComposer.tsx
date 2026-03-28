import React from "react";
import { ArrowUp, Check, ChevronDown, FileText, Image, Mic, Plus, X } from "lucide-react";

import { cn } from "../../lib/utils";
import type { ChatAttachment } from "../../types/agent";
import type { AgentSpec } from "../../types/agent-spec";
import type { UnifiedModel } from "../ai/UnifiedModelSelector";
import { AgentSelector } from "./AgentSelector";
import { UnifiedModelSelector } from "../ai/UnifiedModelSelector";
import { Button } from "../ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "../ui/popover";
import { Textarea } from "../ui/textarea";

interface ChatComposerProps {
  input: string;
  onInputChange: (value: string) => void;
  onKeyDown: (event: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  onSubmit: () => void;
  disabled: boolean;
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
  currentModelId: string;
  onSelectModel: (modelId: string) => void;
  onModelResolved?: (model: UnifiedModel | null) => void;
  selectedAgentId: string;
  onSelectAgent: (agentId: string) => void;
  agentSpecs: AgentSpec[];
  reasoningOptions: string[];
  reasoningEffort?: string;
  onSelectReasoningEffort: (value: string) => void;
  centered: boolean;
  attachments: ChatAttachment[];
  onAddAttachments: () => void;
  onRemoveAttachment: (id: string) => void;
}

function titleCase(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

export function ChatComposer({
  input,
  onInputChange,
  onKeyDown,
  onSubmit,
  disabled,
  textareaRef,
  currentModelId,
  onSelectModel,
  onModelResolved,
  selectedAgentId,
  onSelectAgent,
  agentSpecs,
  reasoningOptions,
  reasoningEffort,
  onSelectReasoningEffort,
  centered,
  attachments,
  onAddAttachments,
  onRemoveAttachment,
}: ChatComposerProps) {
  const canSubmit = (input.trim().length > 0 || attachments.length > 0) && !disabled;
  return (
    <div
      className={cn(
        "mx-auto w-full transition-all duration-300",
        centered ? "max-w-3xl" : "max-w-[58rem]",
      )}
    >
      <div className="relative overflow-hidden rounded-[1.75rem] border border-white/10 bg-background/72 p-2 shadow-[0_24px_80px_rgba(0,0,0,0.14)] backdrop-blur-2xl backdrop-saturate-150">
        <div className="relative z-10 flex flex-col">
          <Textarea
            ref={textareaRef}
            value={input}
            onChange={(event) => onInputChange(event.target.value)}
            onKeyDown={onKeyDown}
            placeholder="Ask MaTE anything, @ to add files, / for commands"
            className={cn(
              "w-full resize-none border-none bg-transparent px-3 py-3 text-sm text-foreground shadow-none outline-none ring-0 placeholder:text-muted-foreground/50 focus-visible:border-none focus-visible:ring-0",
              centered ? "min-h-[100px]" : "min-h-[68px]",
            )}
            disabled={disabled}
          />

          {/* Attachment preview strip */}
          {attachments.length > 0 && (
            <div className="flex flex-wrap gap-1.5 px-3 pb-2">
              {attachments.map((att) => (
                <div
                  key={att.id}
                  className="group relative flex items-center gap-1.5 rounded-lg border border-white/10 bg-white/5 px-2 py-1 text-xs text-muted-foreground"
                >
                  {att.type === "image" && att.thumbnailDataUri ? (
                    <img
                      src={att.thumbnailDataUri}
                      alt={att.filename}
                      className="size-7 rounded object-cover"
                    />
                  ) : att.type === "image" ? (
                    <Image className="size-4 shrink-0" />
                  ) : (
                    <FileText className="size-4 shrink-0" />
                  )}
                  <span className="max-w-[120px] truncate">{att.filename}</span>
                  <button
                    type="button"
                    onClick={() => onRemoveAttachment(att.id)}
                    className="ml-0.5 rounded-full p-0.5 opacity-50 transition-opacity hover:opacity-100"
                  >
                    <X className="size-3" />
                  </button>
                </div>
              ))}
            </div>
          )}

          <div className="flex flex-wrap items-center justify-between gap-2 pb-1 pl-1 pr-1">
            <div className="flex flex-wrap items-center gap-1">
              <button
                type="button"
                onClick={onAddAttachments}
                className="flex size-8 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-white/5 hover:text-foreground"
              >
                <Plus className="size-4" />
              </button>

              <UnifiedModelSelector
                selectedModelId={currentModelId}
                onSelect={onSelectModel}
                onModelResolved={onModelResolved}
                filter="chat"
              />

              <AgentSelector
                selectedAgentId={selectedAgentId}
                onSelect={onSelectAgent}
                agentSpecs={agentSpecs}
              />

              {reasoningOptions.length > 0 && (
                <Popover>
                  <PopoverTrigger
                    render={
                      <button
                        type="button"
                        className="group flex items-center gap-1.5 rounded-full px-2 py-1 text-xs font-medium text-muted-foreground transition-colors hover:bg-white/5 hover:text-foreground"
                      />
                    }
                  >
                    <span className="truncate">
                      {reasoningEffort ? titleCase(reasoningEffort) : "Reasoning"}
                    </span>
                    <ChevronDown className="size-3 opacity-50 transition-transform group-data-[state=open]:rotate-180" />
                  </PopoverTrigger>
                  <PopoverContent
                    align="start"
                    sideOffset={12}
                    className="w-[200px] overflow-hidden rounded-xl border border-white/10 bg-background/20 p-1 shadow-2xl backdrop-blur-md"
                  >
                    <div className="flex flex-col">
                      <div className="px-3 pb-1.5 pt-2 text-[10px] font-bold uppercase tracking-wider text-muted-foreground/40">
                        Reasoning effort
                      </div>
                      {reasoningOptions.map((option) => {
                        const active = reasoningEffort === option;
                        return (
                          <button
                            key={option}
                            type="button"
                            onClick={() => onSelectReasoningEffort(option)}
                            className={cn(
                              "flex w-full items-center justify-between gap-3 rounded-lg px-3 py-2 text-left text-xs transition-colors",
                              active
                                ? "bg-white/10 text-foreground"
                                : "text-muted-foreground hover:bg-white/5 hover:text-foreground",
                            )}
                          >
                            <span>{titleCase(option)}</span>
                            {active && <Check className="size-3.5 shrink-0" />}
                          </button>
                        );
                      })}
                    </div>
                  </PopoverContent>
                </Popover>
              )}
            </div>

            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="icon"
                className="size-8 rounded-full text-muted-foreground hover:bg-white/5 hover:text-foreground"
              >
                <Mic className="size-4" />
              </Button>
              <Button
                size="icon"
                onClick={onSubmit}
                disabled={!canSubmit}
                className={cn(
                  "size-8 rounded-full bg-white/90 text-black shadow-sm transition-all hover:bg-white dark:bg-white/90 dark:text-black",
                  !canSubmit && "scale-95 opacity-50",
                )}
              >
                <ArrowUp className="size-4" />
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
