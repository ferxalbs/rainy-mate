import React from "react";
import {
  ArrowUp,
  Check,
  ChevronDown,
  FileText,
  Image,
  Paperclip,
  Square,
  X,
} from "lucide-react";

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
  inputDisabled: boolean;
  submitDisabled: boolean;
  stopDisabled?: boolean;
  showStopButton?: boolean;
  onStop?: () => void;
  submitLabel?: string;
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
  inputDisabled,
  submitDisabled,
  stopDisabled = false,
  showStopButton = false,
  onStop,
  submitLabel,
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
  const canSubmit = (input.trim().length > 0 || attachments.length > 0) && !submitDisabled;
  const actionLabel = submitLabel || (showStopButton ? "Stop run" : "Send");

  return (
    <div className={cn("mx-auto w-full transition-all duration-300", centered ? "max-w-[47rem]" : "max-w-[46rem]")}>
      <div
        className={cn(
          "overflow-hidden border bg-[linear-gradient(180deg,color-mix(in_srgb,var(--card)_82%,transparent),color-mix(in_srgb,var(--background)_68%,transparent))] shadow-[0_36px_120px_-76px_rgba(0,0,0,0.95)] backdrop-blur-[24px]",
          showStopButton ? "border-primary/70 shadow-[0_40px_140px_-84px_color-mix(in_srgb,var(--primary)_28%,transparent)]" : "border-white/10",
          centered ? "rounded-[26px]" : "rounded-[24px]",
        )}
      >
        <div className={cn("px-4 pb-2.5 pt-3", centered && "px-4.5 pb-2.5 pt-3.5")}>

          {attachments.length > 0 ? (
            <div className="mb-2.5 flex flex-wrap gap-2">
            {attachments.map((att) => (
              <div
                key={att.id}
                className="group flex items-center gap-2 rounded-2xl border border-border/45 bg-background/34 px-2 py-1.5 text-xs text-muted-foreground"
              >
                {att.type === "image" && att.thumbnailDataUri ? (
                  <img
                    src={att.thumbnailDataUri}
                    alt={att.filename}
                    className="size-8 rounded-xl object-cover"
                  />
                ) : att.type === "image" ? (
                  <Image className="size-4 shrink-0" />
                ) : (
                  <FileText className="size-4 shrink-0" />
                )}
                <span className="max-w-[140px] truncate text-foreground/90">{att.filename}</span>
                <button
                  type="button"
                  onClick={() => onRemoveAttachment(att.id)}
                  className="rounded-full p-0.5 opacity-55 transition-opacity hover:opacity-100"
                >
                  <X className="size-3" />
                </button>
              </div>
            ))}
            </div>
          ) : null}

          <Textarea
            ref={textareaRef}
            value={input}
            onChange={(event) => onInputChange(event.target.value)}
            onKeyDown={onKeyDown}
            placeholder="Ask MaTE anything, use @ for files or launch a concrete task."
            className={cn(
              "w-full resize-none border-none bg-transparent px-0 py-0 text-[14px] leading-7 text-foreground shadow-none outline-none ring-0 placeholder:text-muted-foreground/28 focus-visible:border-none focus-visible:ring-0",
              centered ? "min-h-[88px] text-[15px] leading-7" : "min-h-[58px]",
            )}
            disabled={inputDisabled}
          />
        </div>

        <div
          className={cn(
            "flex flex-wrap items-center justify-between gap-3 border-t border-white/8 bg-black/10 px-3.5 py-2",
            centered && "border-t-0 bg-transparent px-4 pb-3 pt-1",
          )}
        >
          <div className="flex min-w-0 flex-wrap items-center gap-1.5 text-[12px] text-muted-foreground">
            <Button
              variant="ghost"
              size="sm"
              className="h-7 rounded-full px-2 text-muted-foreground hover:bg-muted/30 hover:text-foreground"
              onClick={onAddAttachments}
            >
              <Paperclip className="size-4" />
            </Button>

            <UnifiedModelSelector
              selectedModelId={currentModelId}
              onSelect={onSelectModel}
              onModelResolved={onModelResolved}
              filter="chat"
            />

            <div className="hidden h-3.5 w-px bg-border/45 sm:block" />

            {reasoningOptions.length > 0 ? (
              <Popover>
                <PopoverTrigger
                  render={
                    <button
                      type="button"
                      className="group flex h-7 items-center gap-1.5 rounded-full px-2 py-1 text-[12px] text-muted-foreground transition-colors hover:bg-muted/30 hover:text-foreground"
                    />
                  }
                >
                  <span>{reasoningEffort ? titleCase(reasoningEffort) : "Reasoning"}</span>
                  <ChevronDown className="size-3 opacity-60 transition-transform group-data-[state=open]:rotate-180" />
                </PopoverTrigger>
                <PopoverContent
                  align="start"
                  sideOffset={12}
                  className="w-[220px] overflow-hidden rounded-2xl border border-border/70 bg-popover/95 p-1 shadow-2xl backdrop-blur-xl"
                >
                  <div className="px-3 pb-1.5 pt-2 text-[10px] font-semibold uppercase tracking-[0.18em] text-muted-foreground/70">
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
                          "flex w-full items-center justify-between gap-3 rounded-xl px-3 py-2 text-left text-xs transition-colors",
                          active
                            ? "bg-primary/10 text-foreground"
                            : "text-muted-foreground hover:bg-muted/70 hover:text-foreground",
                        )}
                      >
                        <span>{titleCase(option)}</span>
                        {active ? <Check className="size-3.5 shrink-0" /> : null}
                      </button>
                    );
                  })}
                </PopoverContent>
              </Popover>
            ) : null}

            <div className="hidden h-3.5 w-px bg-border/45 sm:block" />

            <AgentSelector
              selectedAgentId={selectedAgentId}
              onSelect={onSelectAgent}
              agentSpecs={agentSpecs}
            />
          </div>

          <Button
            size="icon"
            onClick={showStopButton ? onStop : onSubmit}
            disabled={showStopButton ? stopDisabled : !canSubmit}
            aria-label={actionLabel}
            title={actionLabel}
            className={cn(
              "size-9 rounded-full bg-primary text-primary-foreground shadow-sm transition-all hover:scale-[1.02]",
              showStopButton && "bg-destructive text-destructive-foreground",
              (showStopButton ? stopDisabled : !canSubmit) && "scale-95 opacity-55",
            )}
          >
            {showStopButton ? <Square className="size-4 fill-current" /> : <ArrowUp className="size-4" />}
          </Button>
        </div>
      </div>
    </div>
  );
}
