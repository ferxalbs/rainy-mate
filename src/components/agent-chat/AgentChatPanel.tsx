import React, { useState, useMemo, useEffect, useRef } from "react";
import { AnimatedThemeToggler } from "../ui/animated-theme-toggler";
import {
  ArrowUp,
  Check,
  ChevronDown,
  Compass,
  Eraser,
  FileText,
  Gamepad2,
  Mic,
  Plus,
  Sparkles,
  Trash2,
} from "lucide-react";

import * as tauri from "../../services/tauri";
import { cn } from "../../lib/utils";
import { useTheme } from "../../hooks/useTheme";
import { useAgentChat } from "../../hooks/useAgentChat";
import type { AgentSpec } from "../../types/agent-spec";
import type { UnifiedModel } from "../ai/UnifiedModelSelector";
import {
  UnifiedModelSelector,
  getReasoningOptions,
} from "../ai/UnifiedModelSelector";

import { MessageBubble } from "./MessageBubble";
import { AgentSelector } from "./AgentSelector";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { ScrollArea } from "../ui/scroll-area";
import { Popover, PopoverContent, PopoverTrigger } from "../ui/popover";
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
    icon: Eraser, // Changed from PenTool
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
  const { mode: _mode } = useTheme();
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
    "border border-white/5 bg-background/30 backdrop-blur-md";

  const renderComposer = (centered: boolean) => (
    <div className={cn("mx-auto w-full transition-all duration-300", centered ? "max-w-3xl" : "max-w-2xl")}>
      <div className={cn("relative overflow-hidden rounded-[1.5rem] p-2 transition-all", glassShell)}>
        <div className="relative z-10 flex flex-col">
          {/* Row 1: Textarea */}
          <Textarea
            ref={textareaRef}
            value={input}
            onChange={(event) => setInput(event.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Ask Codex anything, @ to add files, / for commands"
            className={cn(
              "w-full resize-none border-none bg-transparent px-3 py-3 text-sm text-foreground shadow-none outline-none ring-0 placeholder:text-muted-foreground/50 focus-visible:border-none focus-visible:ring-0",
              centered ? "min-h-[100px]" : "min-h-[64px]",
            )}
            disabled={isProcessing}
          />

          {/* Row 2: Selectors and Tools */}
          <div className="flex items-center justify-between pb-1 pl-1 pr-1">
            <div className="flex items-center gap-1">
              <button
                type="button"
                className="flex size-8 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-white/5 hover:text-foreground"
              >
                <Plus className="size-4" />
              </button>

              <UnifiedModelSelector
                selectedModelId={currentModelId}
                onSelect={handleModelSelect}
                onModelResolved={setSelectedModel}
              />

              <AgentSelector
                selectedAgentId={selectedAgentId}
                onSelect={setSelectedAgentId}
                agentSpecs={agentSpecs}
              />

              {reasoningOptions.length > 0 && (
                <Popover>
                  <PopoverTrigger>
                    <button
                      type="button"
                      className="group flex items-center gap-1.5 rounded-md px-1.5 py-1 text-xs font-medium text-muted-foreground transition-colors hover:text-foreground"
                    >
                      <span className="truncate">
                        {reasoningEffort ? titleCase(reasoningEffort) : "Reasoning"}
                      </span>
                      <ChevronDown className="size-3 opacity-50 transition-transform group-data-[state=open]:rotate-180" />
                    </button>
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
                            onClick={() => setReasoningEffort(option)}
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
                onClick={() => void handleSubmit()}
                disabled={!input.trim() || isProcessing}
                className={cn(
                  "size-8 rounded-full bg-white/90 text-black shadow-sm transition-all hover:bg-white dark:bg-white/90 dark:text-black",
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
  );

  return (
    <div className={cn("relative h-full w-full overflow-hidden bg-transparent text-foreground", className)}>
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(255,255,255,0.08),transparent_35%),linear-gradient(180deg,rgba(0,0,0,0.02),transparent_24%,rgba(0,0,0,0.08))] dark:bg-[radial-gradient(circle_at_top,rgba(255,255,255,0.08),transparent_28%),linear-gradient(180deg,rgba(0,0,0,0),transparent_24%,rgba(0,0,0,0.22))]" />

      <div className="pointer-events-none absolute left-0 right-0 top-0 z-40 px-4 pt-4 md:px-6">
        <div data-tauri-drag-region className="absolute inset-x-0 top-0 h-20" />

        <div className="pointer-events-auto mx-auto flex w-full max-w-5xl items-center justify-between gap-3 rounded-full border border-black/5 bg-background/90 px-3 py-1.5 shadow-sm backdrop-blur-md dark:border-white/10 dark:bg-background/20">
          <div className="flex min-w-0 items-center gap-2 md:gap-3">
            <div className="flex items-center gap-2 pl-1">
              <Compass className="size-4 text-primary" />
              <span className="text-sm font-medium tracking-tight">New thread</span>
            </div>
            <button
              type="button"
              className="ml-1 flex min-w-0 items-center gap-1 rounded-full px-2 py-1 text-xs font-medium text-muted-foreground transition-colors hover:bg-foreground/5 hover:text-foreground"
            >
              <span className="truncate uppercase tracking-wide">{workspaceName}</span>
              <ChevronDown className="size-3.5" />
            </button>
          </div>

          <div className="flex items-center gap-1">
            <AnimatedThemeToggler />
            <div className="mx-2 hidden h-4 w-px bg-border/50 sm:block" />
            <TooltipProvider delay={0}>
              <Tooltip>
                <TooltipTrigger
                  onClick={clearMessages}
                  render={
                    <button
                      type="button"
                      className="rounded-full p-2 text-muted-foreground transition-colors hover:bg-foreground/5 hover:text-foreground"
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
                      className="rounded-full p-2 text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
                    />
                  }
                >
                  <Trash2 className="size-4" />
                </TooltipTrigger>
                <TooltipContent>Delete persisted context</TooltipContent>
              </Tooltip>
            </TooltipProvider>
            {onClose && (
              <Button variant="ghost" size="icon-sm" onClick={onClose} className="rounded-full ml-1" />
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
              <div className="mb-4 flex size-12 items-center justify-center rounded-xl border border-black/5 bg-background shadow-sm dark:border-white/10 dark:bg-background/20">
                <Sparkles className="size-6 text-primary" />
              </div>

              <div className="mb-8 text-center">
                <h1 className="text-4xl font-semibold tracking-[-0.04em] text-foreground">Let&apos;s build</h1>
                <p className="mt-2 text-sm text-muted-foreground">
                  Faster controls, cleaner chat, and a workspace-first command surface.
                </p>
              </div>

              <div className="mb-8 w-full max-w-[760px] px-2 flex flex-col md:flex-row gap-3 justify-center">
                {PROMPTS.map(({ accent, icon: Icon, prompt, title }) => (
                  <button
                    key={title}
                    type="button"
                    onClick={() => applyPrompt(prompt)}
                    className="group relative flex-1 min-w-0 overflow-hidden rounded-2xl border border-black/5 bg-background p-4 text-left shadow-sm transition-colors hover:bg-muted/40 dark:border-white/10 dark:bg-background/20 dark:hover:bg-background/30"
                  >
                    <div className="relative z-10 flex h-full flex-col gap-3">
                      <div className="flex items-center justify-between">
                        <div className={cn("flex size-8 items-center justify-center rounded-lg bg-foreground/5 dark:bg-white/10", accent)}>
                          <Icon className="size-4" />
                        </div>
                        <span className="text-[9px] font-bold uppercase tracking-[0.1em] text-muted-foreground/60 transition-colors group-hover:text-muted-foreground">
                          Explore
                        </span>
                      </div>
                      <p className="text-sm font-medium leading-relaxed tracking-[-0.01em] text-foreground/85">
                        {title}
                      </p>
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
