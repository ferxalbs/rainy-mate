import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Eraser, FileText, Gamepad2, Sparkles } from "lucide-react";

import * as tauri from "../../services/tauri";
import { cn } from "../../lib/utils";
import { useAgentChat } from "../../hooks/useAgentChat";
import type { Folder } from "../../types";
import type { AgentSpec } from "../../types/agent-spec";
import type { UnifiedModel } from "../ai/UnifiedModelSelector";
import { getReasoningOptions } from "../ai/UnifiedModelSelector";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { ScrollArea } from "../ui/scroll-area";
import { MemoizedMessageBubble } from "./MessageBubble";
import { ChatComposer } from "./ChatComposer";
import { ChatTopbar } from "./ChatTopbar";

interface AgentChatPanelProps {
  workspacePath: string;
  folders: Folder[];
  activeFolderId?: string;
  onSelectWorkspace?: (folder: Folder) => void | Promise<void>;
  onAddWorkspace?: () => void;
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
    icon: Eraser,
    title: "Create a plan to modernize the current workflow.",
    prompt: "Create a plan to modernize the current workflow.",
    accent: "text-amber-500",
  },
];

const EMPTY_REASONING: string[] = [];

// ─── Extracted sub-components for proper reconciliation ──────────────

const EmptyStatePrompts = React.memo(function EmptyStatePrompts({
  onApplyPrompt,
}: {
  onApplyPrompt: (prompt: string) => void;
}) {
  return (
    <div className="mb-8 flex w-full max-w-2xl flex-col gap-2.5 px-2 md:flex-row">
      {PROMPTS.map(({ accent, icon: Icon, prompt, title }) => (
        <button
          key={title}
          type="button"
          onClick={() => onApplyPrompt(prompt)}
          className="group relative flex-1 overflow-hidden rounded-2xl border border-white/10 bg-background/66 p-4 text-left shadow-sm transition-colors hover:bg-white/5"
        >
          <div className="relative z-10 flex h-full flex-col gap-4">
            <div className="flex items-center justify-between">
              <div
                className={cn(
                  "flex size-8 items-center justify-center rounded-xl bg-white/10",
                  accent,
                )}
              >
                <Icon className="size-3.5" />
              </div>
              <span className="text-[9px] font-bold uppercase tracking-[0.14em] text-muted-foreground/60">
                Explore
              </span>
            </div>
            <p className="text-xs font-medium leading-relaxed tracking-[-0.01em] text-foreground/90">
              {title}
            </p>
          </div>
        </button>
      ))}
    </div>
  );
});

const TelemetryBar = React.memo(function TelemetryBar({
  telemetry,
}: {
  telemetry: {
    historySource?: string;
    retrievalMode?: string;
    embeddingProfile?: string;
    compressionApplied?: boolean;
    compressionTriggerTokens?: number;
  } | undefined;
}) {
  return (
    <div className="flex flex-wrap items-center justify-center gap-2">
      <Badge
        variant="outline"
        className="rounded-md border-white/10 bg-background/80 px-2 py-1 text-[10px] uppercase tracking-[0.14em] backdrop-blur-sm backdrop-saturate-150 dark:bg-background/10"
      >
        History: {telemetry?.historySource || "persisted_long_chat"}
      </Badge>
      <Badge
        variant="outline"
        className="rounded-md border-white/10 bg-background/80 px-2 py-1 text-[10px] uppercase tracking-[0.14em] backdrop-blur-sm backdrop-saturate-150 dark:bg-background/10"
      >
        Retrieval: {telemetry?.retrievalMode || "unavailable"}
      </Badge>
      <Badge
        variant="outline"
        className="rounded-md border-white/10 bg-background/80 px-2 py-1 text-[10px] uppercase tracking-[0.14em] backdrop-blur-sm backdrop-saturate-150 dark:bg-background/10"
      >
        Embedding: {telemetry?.embeddingProfile || "gemini-embedding-2-preview"}
      </Badge>
      {telemetry?.compressionApplied && (
        <Badge
          variant="outline"
          className="rounded-md border-white/10 bg-background/80 px-2 py-1 text-[10px] uppercase tracking-[0.14em] backdrop-blur-sm backdrop-saturate-150 dark:bg-background/10"
        >
          Compression @{telemetry.compressionTriggerTokens || 80000}
        </Badge>
      )}
    </div>
  );
});

// ─── Main component ──────────────────────────────────────────────────

export function AgentChatPanel({
  workspacePath,
  folders,
  activeFolderId,
  onSelectWorkspace,
  onAddWorkspace,
  onOpenSettings,
  className,
}: AgentChatPanelProps) {
  const [input, setInput] = useState("");
  const [currentModelId, setCurrentModelId] = useState("");
  const [selectedModel, setSelectedModel] = useState<UnifiedModel | null>(null);
  const [reasoningEffortOverride, setReasoningEffortOverride] = useState<string | undefined>(undefined);
  const [agentSpecs, setAgentSpecs] = useState<AgentSpec[]>([]);
  const [selectedAgentId, setSelectedAgentId] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const latestMessageRef = useRef<HTMLDivElement>(null);
  const prevMessagesLengthRef = useRef(0);

  const {
    messages,
    chatSession,
    chatTitleStatus,
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
  const stableReasoningOptions = reasoningOptions.length > 0 ? reasoningOptions : EMPTY_REASONING;

  const latestTelemetry = useMemo(() => {
    for (let i = messages.length - 1; i >= 0; i--) {
      if (messages[i].type === "agent" && messages[i].ragTelemetry) {
        return messages[i].ragTelemetry;
      }
    }
    return undefined;
  }, [messages]);

  // ─── Textarea auto-resize (external DOM mutation, not cascading setState) ──
  useEffect(() => {
    const element = textareaRef.current;
    if (!element) return;
    const maxHeight = messages.length === 0 ? 280 : 220;
    element.style.height = "0px";
    element.style.height = `${Math.min(element.scrollHeight, maxHeight)}px`;
  }, [input, messages.length]);

  // ─── One-time init effects ─────────────────────────────────────────
  useEffect(() => {
    let cancelled = false;
    tauri.getSelectedModel().then((model) => {
      if (!cancelled && model) setCurrentModelId(model);
    }).catch((error) => console.error("Failed to load selected model", error));
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    let cancelled = false;
    tauri.listAgentSpecs().then((specs) => {
      if (cancelled) return;
      const typed = specs as AgentSpec[];
      setAgentSpecs(typed);
      if (typed.length > 0) {
        setSelectedAgentId((prev) => prev || typed[0].id);
      }
    }).catch((error) => console.error("Failed to load saved agents", error));
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    void hydrateLongChatHistory();
  }, [hydrateLongChatHistory]);

  // Derive reasoning effort from options + user override — no effect needed
  const reasoningEffort = useMemo(() => {
    if (!stableReasoningOptions.length) return undefined;
    if (reasoningEffortOverride && stableReasoningOptions.includes(reasoningEffortOverride)) {
      return reasoningEffortOverride;
    }
    return stableReasoningOptions.includes("medium") ? "medium" : stableReasoningOptions[0];
  }, [stableReasoningOptions, reasoningEffortOverride]);

  // ─── Scroll management — coalesced via rAF ─────────────────────────
  const scrollRafRef = useRef<number | null>(null);
  useEffect(() => {
    const prevLength = prevMessagesLengthRef.current;
    const currentLength = messages.length;
    prevMessagesLengthRef.current = currentLength;

    if (currentLength > prevLength) {
      latestMessageRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
    } else {
      if (scrollRafRef.current != null) return;
      scrollRafRef.current = requestAnimationFrame(() => {
        scrollRafRef.current = null;
        messagesEndRef.current?.scrollIntoView({ behavior: "auto", block: "end" });
      });
    }
    return () => {
      if (scrollRafRef.current != null) {
        cancelAnimationFrame(scrollRafRef.current);
        scrollRafRef.current = null;
      }
    };
  }, [messages]);

  // ─── Stable callbacks ──────────────────────────────────────────────
  const handleModelSelect = useCallback(async (modelId: string) => {
    setCurrentModelId(modelId);
    try {
      await tauri.setSelectedModel(modelId);
    } catch (error) {
      console.error("Failed to persist model selection", error);
    }
  }, []);

  const applyPrompt = useCallback((prompt: string) => {
    setInput(prompt);
    window.requestAnimationFrame(() => textareaRef.current?.focus());
  }, []);

  const handleSubmit = useCallback(async () => {
    const instruction = input.trim();
    if (!instruction || isProcessing) return;

    setInput("");
    await runNativeAgent(
      instruction,
      currentModelId,
      workspacePath,
      selectedAgentId || undefined,
      stableReasoningOptions.length ? reasoningEffort : undefined,
    );
  }, [input, isProcessing, runNativeAgent, currentModelId, workspacePath, selectedAgentId, stableReasoningOptions, reasoningEffort]);

  const handleKeyDown = useCallback((event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void handleSubmit();
    }
  }, [handleSubmit]);

  const handleNewChat = useCallback(() => {
    void clearMessagesAndContext(workspacePath);
  }, [clearMessagesAndContext, workspacePath]);

  const handleComposerSubmit = useCallback(() => {
    void handleSubmit();
  }, [handleSubmit]);

  // ─── Render ────────────────────────────────────────────────────────
  const hasMessages = messages.length > 0;

  return (
    <div className={cn("relative h-full w-full overflow-hidden bg-transparent text-foreground", className)}>
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(255,255,255,0.12),transparent_30%),radial-gradient(circle_at_20%_20%,rgba(255,184,76,0.08),transparent_26%),linear-gradient(180deg,rgba(0,0,0,0.02),transparent_20%,rgba(0,0,0,0.08))] dark:bg-[radial-gradient(circle_at_top,rgba(255,255,255,0.08),transparent_28%),radial-gradient(circle_at_12%_18%,rgba(255,184,76,0.08),transparent_26%),linear-gradient(180deg,rgba(0,0,0,0),transparent_24%,rgba(0,0,0,0.2))]" />

      <ChatTopbar
        chatSession={chatSession}
        titleStatus={chatTitleStatus}
        workspacePath={workspacePath}
        folders={folders}
        activeFolderId={activeFolderId}
        onSelectFolder={onSelectWorkspace}
        onAddFolder={onAddWorkspace}
        onNewChat={handleNewChat}
        onClearUi={clearMessages}
        onOpenSettings={onOpenSettings}
      />

      <ScrollArea className="absolute inset-0 z-10 h-full w-full">
        <div
          className={cn(
            "mx-auto flex w-full max-w-6xl flex-col px-4 md:px-6",
            hasMessages ? "min-h-full pb-44 pt-24" : "min-h-full justify-center pb-12 pt-16",
          )}
        >
          {!hasMessages ? (
            <div className="flex flex-1 flex-col items-center justify-center">
              <div className="mb-4 flex size-10 items-center justify-center rounded-xl border border-black/5 bg-background shadow-sm dark:border-white/10 dark:bg-background/20">
                <Sparkles className="size-5 text-primary" />
              </div>

              <div className="mb-6 text-center">
                <h1 className="text-2xl font-semibold tracking-[-0.03em] text-foreground">
                  Conversation-first workspace control
                </h1>
                <p className="mt-1.5 text-xs text-muted-foreground">
                  The new shell starts here: faster context, cleaner workspace switching, and auto-titled sessions.
                </p>
              </div>

              <div className="mb-6 flex flex-wrap items-center justify-center gap-2">
                <Badge variant="outline" className="rounded-full border-white/10 bg-background/60 px-2.5 py-0.5 text-[9px] uppercase tracking-[0.14em] backdrop-blur-md">
                  Single persistent scope
                </Badge>
                <Badge variant="outline" className="rounded-full border-white/10 bg-background/60 px-2.5 py-0.5 text-[9px] uppercase tracking-[0.14em] backdrop-blur-md">
                  GPT-5 Nano titles
                </Badge>
                <Badge variant="outline" className="rounded-full border-white/10 bg-background/60 px-2.5 py-0.5 text-[9px] uppercase tracking-[0.14em] backdrop-blur-md">
                  Dynamic chats next
                </Badge>
              </div>

              <EmptyStatePrompts onApplyPrompt={applyPrompt} />

              <ChatComposer
                input={input}
                onInputChange={setInput}
                onKeyDown={handleKeyDown}
                onSubmit={handleComposerSubmit}
                disabled={isProcessing}
                textareaRef={textareaRef}
                currentModelId={currentModelId}
                onSelectModel={handleModelSelect}
                onModelResolved={setSelectedModel}
                selectedAgentId={selectedAgentId}
                onSelectAgent={setSelectedAgentId}
                agentSpecs={agentSpecs}
                reasoningOptions={stableReasoningOptions}
                reasoningEffort={reasoningEffort}
                onSelectReasoningEffort={setReasoningEffortOverride}
                centered
              />
            </div>
          ) : (
            <div className="space-y-8">
              <TelemetryBar telemetry={latestTelemetry} />

              {hasMoreHistory && (
                <div className="flex justify-center">
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={loadOlderHistory}
                    disabled={isHydratingHistory}
                    className="rounded-full border border-white/10 bg-background/80 px-4 backdrop-blur-sm backdrop-saturate-150 dark:bg-background/10"
                  >
                    {isHydratingHistory ? "Loading..." : "Load older messages"}
                  </Button>
                </div>
              )}

              {messages.map((message, index) => (
                <div key={message.id} ref={index === messages.length - 1 ? latestMessageRef : undefined}>
                  <MemoizedMessageBubble
                    message={message}
                    currentPlan={currentPlan}
                    isExecuting={isExecuting}
                    onExecute={executePlan}
                    onExecuteToolCalls={executeToolCalls}
                    onStopRun={stopAgentRun}
                    onRetryRun={retryAgentRun}
                    workspaceId={workspacePath}
                  />
                </div>
              ))}
              <div ref={messagesEndRef} />
            </div>
          )}
        </div>
      </ScrollArea>

      {hasMessages && (
        <div className="pointer-events-none absolute inset-x-0 bottom-6 z-30 px-4 md:px-6">
          <div className="pointer-events-auto mx-auto w-full max-w-6xl">
            <ChatComposer
              input={input}
              onInputChange={setInput}
              onKeyDown={handleKeyDown}
              onSubmit={handleComposerSubmit}
              disabled={isProcessing}
              textareaRef={textareaRef}
              currentModelId={currentModelId}
              onSelectModel={handleModelSelect}
              onModelResolved={setSelectedModel}
              selectedAgentId={selectedAgentId}
              onSelectAgent={setSelectedAgentId}
              agentSpecs={agentSpecs}
              reasoningOptions={stableReasoningOptions}
              reasoningEffort={reasoningEffort}
              onSelectReasoningEffort={setReasoningEffortOverride}
              centered={false}
            />
          </div>
        </div>
      )}
    </div>
  );
}
