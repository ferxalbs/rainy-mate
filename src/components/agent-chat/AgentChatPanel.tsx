import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowUp,
  Brain,
  ChevronDown,
  Compass,
  Eraser,
  FileText,
  Gamepad2,
  Mic,
  Paperclip,
  PenTool,
  Sparkles,
  Trash2,
} from "lucide-react";

import * as tauri from "../../services/tauri";
import { cn } from "../../lib/utils";
import { useTheme } from "../../hooks/useTheme";
import { useAgentChat } from "../../hooks/useAgentChat";
import type { AgentSpec } from "../../types/agent-spec";
import type { UnifiedModel } from "../ai/UnifiedModelSelector";
import { UnifiedModelSelector, getReasoningOptions } from "../ai/UnifiedModelSelector";
import { MessageBubble } from "./MessageBubble";
import { AgentSelector } from "./AgentSelector";
import { MacOSToggle } from "../layout/MacOSToggle";
import { Badge } from "../ui/badge";
import { Button, buttonVariants } from "../ui/button";
import { ScrollArea } from "../ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";
import { Textarea } from "../ui/textarea";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "../ui/tooltip";

interface AgentChatPanelProps {
  workspacePath: string;
  onClose?: () => void;
  onOpenSettings?: () => void;
  className?: string;
}

const PROMPTS = [
  {
    icon: Gamepad2,
    title: "Build a classic Snake game in this repo.",
    prompt: "Build a classic Snake game in this repo.",
    accent: "text-sky-500",
  },
  {
    icon: FileText,
    title: "Create a one-page PDF that summarizes this app.",
    prompt: "Create a one-page pdf that summarizes this app.",
    accent: "text-rose-500",
  },
  {
    icon: PenTool,
    title: "Create a plan to modernize the current workflow.",
    prompt: "Create a plan to modernize the current workflow.",
    accent: "text-amber-500",
  },
];

function titleCase(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

function useAutoResizeTextarea(
  ref: React.RefObject<HTMLTextAreaElement | null>,
  value: string,
  maxHeight: number,
) {
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    element.style.height = "0px";
    element.style.height = `${Math.min(element.scrollHeight, maxHeight)}px`;
  }, [maxHeight, ref, value]);
}

export function AgentChatPanel({
  workspacePath,
  onClose,
  className,
}: AgentChatPanelProps) {
  const { mode, setMode } = useTheme();
  const [input, setInput] = useState("");
  const [currentModelId, setCurrentModelId] = useState("");
  const [selectedModel, setSelectedModel] = useState<UnifiedModel | null>(null);
  const [reasoningEffort, setReasoningEffort] = useState<string | undefined>(undefined);
  const [agentSpecs, setAgentSpecs] = useState<AgentSpec[]>([]);
  const [selectedAgentId, setSelectedAgentId] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const {
    messages,
    isPlanning,
    isExecuting,
    currentPlan,
    executePlan,
    executeToolCalls,
    runNativeAgent,
    stopAgentRun,
    retryAgentRun,
    clearMessages,
    clearMessagesAndContext,
    hydrateLongChatHistory,
    loadOlderHistory,
    hasMoreHistory,
    isHydratingHistory,
  } = useAgentChat();

  const isProcessing = isPlanning || isExecuting;
  const reasoningOptions = useMemo(() => getReasoningOptions(selectedModel), [selectedModel]);
  const workspaceName = useMemo(
    () => workspacePath.split("/").filter(Boolean).pop() || "workspace",
    [workspacePath],
  );
  const latestTelemetry = [...messages]
    .reverse()
    .find((message) => message.type === "agent" && message.ragTelemetry)?.ragTelemetry;

  useAutoResizeTextarea(textareaRef, input, messages.length === 0 ? 280 : 220);

  useEffect(() => {
    const initModel = async () => {
      try {
        const model = await tauri.getSelectedModel();
        if (model) setCurrentModelId(model);
      } catch (error) {
        console.error("Failed to load selected model", error);
      }
    };

    void initModel();
  }, []);

  useEffect(() => {
    const loadSpecs = async () => {
      try {
        const specs = (await tauri.listAgentSpecs()) as AgentSpec[];
        setAgentSpecs(specs);
        if (specs.length > 0) {
          setSelectedAgentId((previous) => previous || specs[0].id);
        }
      } catch (error) {
        console.error("Failed to load saved agents", error);
      }
    };

    void loadSpecs();
  }, []);

  useEffect(() => {
    void hydrateLongChatHistory();
  }, [hydrateLongChatHistory]);

  useEffect(() => {
    if (!reasoningOptions.length) {
      setReasoningEffort(undefined);
      return;
    }

    setReasoningEffort((current) => {
      if (current && reasoningOptions.includes(current)) return current;
      return reasoningOptions.includes("medium") ? "medium" : reasoningOptions[0];
    });
  }, [reasoningOptions]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const handleModelSelect = async (modelId: string) => {
    setCurrentModelId(modelId);
    try {
      await tauri.setSelectedModel(modelId);
    } catch (error) {
      console.error("Failed to persist model selection", error);
    }
  };

  const focusComposer = () => {
    textareaRef.current?.focus();
  };

  const applyPrompt = (prompt: string) => {
    setInput(prompt);
    window.requestAnimationFrame(focusComposer);
  };

  const handleSubmit = async () => {
    const instruction = input.trim();
    if (!instruction || isProcessing) return;

    setInput("");
    await runNativeAgent(
      instruction,
      currentModelId,
      workspacePath,
      selectedAgentId || undefined,
      reasoningOptions.length ? reasoningEffort : undefined,
    );
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void handleSubmit();
    }
  };

  const glassShell =
    "border border-white/10 bg-background/90 shadow-[0_12px_40px_rgba(0,0,0,0.12)] backdrop-blur-md backdrop-saturate-150 dark:bg-background/20";

  const renderComposer = (centered: boolean) => (
    <div className={cn("mx-auto w-full transition-all duration-300", centered ? "max-w-3xl" : "max-w-2xl")}>
      <div className={cn("relative overflow-hidden rounded-lg", glassShell)}>
        <div className="relative z-10 flex flex-col gap-3 p-3">
          <div className="rounded-lg border border-white/8 bg-background/70 px-3 py-2 backdrop-blur-sm dark:bg-background/10">
            <Textarea
              ref={textareaRef}
              value={input}
              onChange={(event) => setInput(event.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Ask Codex anything, @ to add files, / for commands"
              className={cn(
                "min-h-[64px] w-full resize-none border-none bg-transparent px-0 py-0 text-sm text-foreground shadow-none outline-none ring-0 focus-visible:border-none focus-visible:ring-0",
                centered ? "leading-6" : "min-h-[56px] leading-5",
              )}
              disabled={isProcessing}
            />
          </div>

          <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <div className="flex flex-wrap items-center gap-2">
              <TooltipProvider delay={0}>
                <Tooltip>
                  <TooltipTrigger
                    render={
                      <button
                        type="button"
                        className={cn(
                          buttonVariants({ variant: "ghost", size: "icon-sm" }),
                          "rounded-lg border border-white/8 bg-background/70 backdrop-blur-sm backdrop-saturate-150 hover:bg-background/85 dark:bg-background/10 dark:hover:bg-background/16",
                        )}
                      />
                    }
                  >
                    <Sparkles className="size-4" />
                  </TooltipTrigger>
                  <TooltipContent>Refine prompt</TooltipContent>
                </Tooltip>

                <Tooltip>
                  <TooltipTrigger
                    render={
                      <button
                        type="button"
                        className={cn(
                          buttonVariants({ variant: "ghost", size: "icon-sm" }),
                          "rounded-lg border border-white/8 bg-background/70 backdrop-blur-sm backdrop-saturate-150 hover:bg-background/85 dark:bg-background/10 dark:hover:bg-background/16",
                        )}
                      />
                    }
                  >
                    <Paperclip className="size-4" />
                  </TooltipTrigger>
                  <TooltipContent>Attach files</TooltipContent>
                </Tooltip>
              </TooltipProvider>

              <div className="h-5 w-px bg-border/60" />

              <AgentSelector
                selectedAgentId={selectedAgentId}
                onSelect={setSelectedAgentId}
                agentSpecs={agentSpecs}
                className="min-w-[10rem]"
              />

              <UnifiedModelSelector
                selectedModelId={currentModelId}
                onSelect={handleModelSelect}
                onModelResolved={setSelectedModel}
              />

              {reasoningOptions.length > 0 && (
                <Select
                  value={reasoningEffort}
                  onValueChange={(value) => setReasoningEffort(value ?? undefined)}
                >
                  <SelectTrigger className="h-9 min-w-[9.5rem] rounded-lg border-white/8 bg-background/70 px-3 text-sm shadow-none backdrop-blur-sm backdrop-saturate-150 dark:bg-background/10">
                    <Brain className="size-4 text-amber-500" />
                    <SelectValue placeholder="Thought effort" />
                  </SelectTrigger>
                  <SelectContent
                    align="start"
                    sideOffset={8}
                    className="rounded-lg border-white/10 bg-background/90 p-1.5 shadow-[0_16px_48px_rgba(0,0,0,0.16)] backdrop-blur-xl backdrop-saturate-150 dark:bg-background/20"
                  >
                    {reasoningOptions.map((option) => (
                      <SelectItem key={option} value={option} className="rounded-md px-3 py-2">
                        <span className="flex items-center gap-2">
                          <span className="inline-flex rounded-md bg-amber-500/10 px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-amber-700 dark:text-amber-300">
                            {titleCase(option)}
                          </span>
                          <span className="text-xs text-muted-foreground">
                            {option === "minimal"
                              ? "Lean, quick thinking"
                              : option === "low"
                                ? "Fast reasoning"
                                : option === "medium"
                                  ? "Balanced depth"
                                  : "Maximum deliberation"}
                          </span>
                        </span>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            </div>

            <div className="flex items-center justify-between gap-2 lg:justify-end">
              <div className="flex items-center gap-2">
                <Badge variant="outline" className="rounded-md border-white/8 bg-background/70 px-2 py-1 text-[10px] uppercase tracking-[0.16em] backdrop-blur-sm backdrop-saturate-150 dark:bg-background/10">
                  Local
                </Badge>
                {reasoningOptions.length > 0 && reasoningEffort && (
                  <Badge className="rounded-md bg-amber-500/12 px-2 py-1 text-[10px] uppercase tracking-[0.14em] text-amber-800 dark:text-amber-200">
                    {titleCase(reasoningEffort)} thought
                  </Badge>
                )}
              </div>

              <div className="flex items-center gap-2">
                <Button
                  variant="ghost"
                  size="icon"
                  className="size-9 rounded-lg border border-white/8 bg-background/70 backdrop-blur-sm backdrop-saturate-150 hover:bg-background/85 dark:bg-background/10 dark:hover:bg-background/16"
                >
                  <Mic className="size-4" />
                </Button>
                <Button
                  size="icon"
                  onClick={() => void handleSubmit()}
                  disabled={!input.trim() || isProcessing}
                  className={cn(
                    "size-9 rounded-lg bg-foreground text-background shadow-none transition-all hover:bg-foreground/90",
                    (!input.trim() || isProcessing) && "scale-95 opacity-50",
                  )}
                >
                  <ArrowUp className="size-4" />
                </Button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );

  return (
    <div className={cn("relative h-full w-full overflow-hidden bg-transparent text-foreground", className)}>
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(255,255,255,0.08),transparent_35%),linear-gradient(180deg,rgba(0,0,0,0.02),transparent_24%,rgba(0,0,0,0.08))] dark:bg-[radial-gradient(circle_at_top,rgba(255,255,255,0.08),transparent_28%),linear-gradient(180deg,rgba(0,0,0,0),transparent_24%,rgba(0,0,0,0.22))]" />

      <div className="pointer-events-none absolute left-0 right-0 top-0 z-40 px-4 pt-5 md:px-6">
        <div data-tauri-drag-region className="absolute inset-x-0 top-0 h-24" />

        <div className="pointer-events-auto mx-auto flex w-full max-w-5xl items-center justify-between gap-3 rounded-lg border border-white/10 bg-background/95 px-3 py-2 shadow-[0_10px_30px_rgba(0,0,0,0.1)] backdrop-blur-md backdrop-saturate-150 dark:bg-background/20">
          <div className="flex min-w-0 items-center gap-2 md:gap-3">
            <div className="flex items-center gap-2 rounded-lg border border-white/8 bg-background/75 px-3 py-2 backdrop-blur-sm dark:bg-background/10">
              <Compass className="size-4 text-primary" />
              <span className="text-sm font-medium tracking-[-0.02em]">New thread</span>
            </div>
            <button
              type="button"
              className="flex min-w-0 items-center gap-1 rounded-lg px-2 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-foreground/5 hover:text-foreground"
            >
              <span className="truncate">{workspaceName}</span>
              <ChevronDown className="size-4" />
            </button>
          </div>

          <div className="flex items-center gap-1.5">
            <MacOSToggle
              isDark={mode === "dark"}
              onToggle={(checked) => setMode(checked ? "dark" : "light")}
            />
            <div className="mx-1 hidden h-4 w-px bg-border/50 sm:block" />
            <TooltipProvider delay={0}>
              <Tooltip>
                <TooltipTrigger
                  onClick={clearMessages}
                  render={
                    <button
                      type="button"
                      className={cn(
                        buttonVariants({ variant: "ghost", size: "icon-sm" }),
                "rounded-lg hover:bg-foreground/6",
                      )}
                    />
                  }
                >
                  <Eraser className="size-4" />
                </TooltipTrigger>
                <TooltipContent>Clear UI only</TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger
                  onClick={() => void clearMessagesAndContext(workspacePath)}
                  render={
                    <button
                      type="button"
                      className={cn(
                        buttonVariants({ variant: "ghost", size: "icon-sm" }),
                        "rounded-lg hover:bg-destructive/10 hover:text-destructive",
                      )}
                    />
                  }
                >
                  <Trash2 className="size-4" />
                </TooltipTrigger>
                <TooltipContent>Delete persisted context</TooltipContent>
              </Tooltip>
            </TooltipProvider>
            {onClose && (
              <Button variant="ghost" size="icon-sm" onClick={onClose} className="rounded-full" />
            )}
          </div>
        </div>
      </div>

      <ScrollArea className="absolute inset-0 z-10 h-full w-full">
        <div
          className={cn(
            "mx-auto flex w-full max-w-6xl flex-col px-4 transition-all duration-300 md:px-6",
            messages.length === 0 ? "min-h-full justify-center pb-12 pt-24" : "min-h-full pb-44 pt-28",
          )}
        >
          {messages.length === 0 ? (
            <div className="flex flex-1 flex-col items-center justify-center">
              <div className="mb-5 flex size-16 items-center justify-center rounded-lg border border-white/10 bg-background/85 shadow-[0_10px_30px_rgba(0,0,0,0.12)] backdrop-blur-md backdrop-saturate-150 dark:bg-background/20">
                <Sparkles className="size-8 text-primary" />
              </div>

              <div className="mb-9 text-center">
                <h1 className="text-4xl font-semibold tracking-[-0.05em] md:text-5xl">Let&apos;s build</h1>
                <p className="mt-3 text-sm text-muted-foreground md:text-base">
                  Faster controls, cleaner chat, and a workspace-first command surface.
                </p>
              </div>

              <div className="mb-8 grid w-full max-w-4xl gap-3 md:grid-cols-3">
                {PROMPTS.map(({ accent, icon: Icon, prompt, title }) => (
                  <button
                    key={title}
                    type="button"
                    onClick={() => applyPrompt(prompt)}
                    className="group relative overflow-hidden rounded-lg border border-white/10 bg-background/85 p-4 text-left shadow-[0_10px_30px_rgba(0,0,0,0.08)] backdrop-blur-md backdrop-saturate-150 transition-all hover:bg-background/95 dark:bg-background/20 dark:hover:bg-background/28"
                  >
                    <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(135deg,rgba(255,255,255,0.08),transparent_45%)] opacity-80" />
                    <div className="relative z-10 flex items-start justify-between gap-4">
                      <div>
                        <div className={cn("mb-3 flex size-10 items-center justify-center rounded-lg bg-foreground/5 dark:bg-white/10", accent)}>
                          <Icon className="size-4.5" />
                        </div>
                        <p className="max-w-[18rem] text-sm font-medium leading-6 tracking-[-0.02em] text-foreground/90">
                          {title}
                        </p>
                      </div>
                      <span className="rounded-md border border-white/10 px-2 py-1 text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                        Explore
                      </span>
                    </div>
                  </button>
                ))}
              </div>

              {renderComposer(true)}
            </div>
          ) : (
            <div className="space-y-8">
              <div className="flex flex-wrap items-center justify-center gap-2">
                <Badge variant="outline" className="rounded-md border-white/10 bg-background/80 px-2 py-1 text-[10px] uppercase tracking-[0.14em] backdrop-blur-sm backdrop-saturate-150 dark:bg-background/10">
                  History: {latestTelemetry?.historySource || "persisted_long_chat"}
                </Badge>
                <Badge variant="outline" className="rounded-md border-white/10 bg-background/80 px-2 py-1 text-[10px] uppercase tracking-[0.14em] backdrop-blur-sm backdrop-saturate-150 dark:bg-background/10">
                  Retrieval: {latestTelemetry?.retrievalMode || "unavailable"}
                </Badge>
                <Badge variant="outline" className="rounded-md border-white/10 bg-background/80 px-2 py-1 text-[10px] uppercase tracking-[0.14em] backdrop-blur-sm backdrop-saturate-150 dark:bg-background/10">
                  Embedding: {latestTelemetry?.embeddingProfile || "gemini-embedding-2-preview"}
                </Badge>
                {latestTelemetry?.compressionApplied && (
                  <Badge variant="outline" className="rounded-md border-white/10 bg-background/80 px-2 py-1 text-[10px] uppercase tracking-[0.14em] backdrop-blur-sm backdrop-saturate-150 dark:bg-background/10">
                    Compression @{latestTelemetry.compressionTriggerTokens || 80000}
                  </Badge>
                )}
              </div>

              {hasMoreHistory && (
                <div className="flex justify-center">
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={loadOlderHistory}
                    disabled={isHydratingHistory}
                    className="rounded-lg border border-white/10 bg-background/80 px-4 backdrop-blur-sm backdrop-saturate-150 dark:bg-background/10"
                  >
                    {isHydratingHistory ? "Loading..." : "Load older messages"}
                  </Button>
                </div>
              )}

              {messages.map((message) => (
                <MessageBubble
                  key={message.id}
                  message={message}
                  currentPlan={currentPlan}
                  isExecuting={isExecuting}
                  onExecute={executePlan}
                  onExecuteToolCalls={executeToolCalls}
                  onStopRun={stopAgentRun}
                  onRetryRun={retryAgentRun}
                  workspaceId={workspacePath}
                />
              ))}
              <div ref={messagesEndRef} />
            </div>
          )}
        </div>
      </ScrollArea>

      {messages.length > 0 && (
        <div className="pointer-events-none absolute inset-x-0 bottom-6 z-30 px-4 md:px-6">
          <div className="pointer-events-auto mx-auto w-full max-w-6xl">{renderComposer(false)}</div>
        </div>
      )}
    </div>
  );
}
