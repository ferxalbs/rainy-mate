import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Eraser, FileText, Gamepad2, Sparkles } from "lucide-react";

import * as tauri from "../../services/tauri";
import { cn } from "../../lib/utils";
import { useAgentChat } from "../../hooks/useAgentChat";
import { useActiveChatRunState } from "../../hooks/agent-chat/useActiveChatRunState";
import type { Folder } from "../../types";
import type { ChatAttachment } from "../../types/agent";
import type { AgentSpec } from "../../types/agent-spec";
import type { UnifiedModel } from "../ai/UnifiedModelSelector";
import { getReasoningOptions } from "../ai/UnifiedModelSelector";
import { Badge } from "../ui/badge";
import { ChatComposer } from "./ChatComposer";
import { ChatTopbar } from "./ChatTopbar";
import { MessagesTimeline } from "./timeline/MessagesTimeline";
import { ScheduleTaskDialog } from "../scheduling/ScheduleTaskDialog";

interface AgentChatPanelProps {
  workspacePath: string;
  folders: Folder[];
  activeFolderId?: string;
  onSelectWorkspace?: (folder: Folder) => void | Promise<void>;
  onAddWorkspace?: () => void;
  onClose?: () => void;
  onOpenSettings?: () => void;
  className?: string;

  // Multi-chat props
  chatScopeId?: string | null;
  activeSessionBinding?: tauri.ActiveChatRunBinding | null;
  pendingLaunch?: {
    requestId: string;
    prompt: string;
    preflight: tauri.WorkspaceLaunchPreflight;
    scenarioId: string;
    workspaceId: string;
    chatId: string;
  } | null;
  onPendingLaunchConsumed?: (requestId: string) => boolean | Promise<boolean>;
  onPendingLaunchCompleted?: (result: {
    requestId: string;
    scenarioId: string;
    workspaceId: string;
    chatId: string | null;
    success: boolean;
    actualToolIds: string[];
    actualTouchedPaths: string[];
    producedArtifactPaths: string[];
  }) => void | Promise<void>;
  onNewChat?: () => void;
  onRefreshSessions?: () => Promise<void>;
}

const PROMPTS = [
  {
    icon: Gamepad2,
    title: "Ship a polished Snake game for this repo.",
    prompt: "Build a polished Snake game in this repo with responsive UI, score persistence, and keyboard controls.",
    accent: "text-sky-500",
  },
  {
    icon: FileText,
    title: "Generate an investor-grade one-page brief.",
    prompt: "Create a one-page PDF that summarizes this app for technical founders and investors.",
    accent: "text-rose-500",
  },
  {
    icon: Eraser,
    title: "Inspect T3CODE chat patterns and port them here.",
    prompt:
      "Inspect the local T3CODE repo and return the most important chat UX patterns to replicate here, including streaming behavior and timeline density. Answer in English with exact file paths.",
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
          className="group relative flex-1 overflow-hidden rounded-[26px] border border-border/70 bg-card/72 p-4 text-left shadow-[0_24px_80px_-64px_rgba(0,0,0,0.7)] transition-colors hover:bg-card"
        >
          <div className="relative z-10 flex h-full flex-col gap-4">
            <div className="flex items-center justify-between">
              <div
                className={cn(
                  "flex size-8 items-center justify-center rounded-xl border border-border/60 bg-background/75",
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
    executionMode?: string;
    workspaceMemoryEnabled?: boolean;
    lastModel?: string;
    totalTokens?: number;
    compressionApplied?: boolean;
    compressionTriggerTokens?: number;
  } | undefined;
}) {
  const items = [
    `run:${telemetry?.executionMode || "local"}`,
    `memory:${telemetry?.workspaceMemoryEnabled ? "on" : "off"}`,
    `history:${telemetry?.historySource || "persisted_long_chat"}`,
    `retrieval:${telemetry?.retrievalMode || "unavailable"}`,
  ];

  if (telemetry?.lastModel) {
    items.push(`model:${telemetry.lastModel}`);
  }

  return (
    <div className="flex flex-wrap items-center justify-center gap-2">
      {items.map((item) => (
        <Badge
          key={item}
          variant="outline"
          className="rounded-full border-border/60 bg-card/82 px-2.5 py-1 text-[10px] uppercase tracking-[0.14em] text-muted-foreground backdrop-blur-xl"
        >
          {item}
        </Badge>
      ))}
      {telemetry?.compressionApplied ? (
        <Badge
          variant="outline"
          className="rounded-full border-border/60 bg-card/82 px-2.5 py-1 text-[10px] uppercase tracking-[0.14em] text-muted-foreground backdrop-blur-xl"
        >
          compression@{telemetry.compressionTriggerTokens || 80000}
        </Badge>
      ) : null}
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
  chatScopeId: externalChatScopeId,
  activeSessionBinding: propActiveSessionBinding,
  pendingLaunch,
  onPendingLaunchConsumed,
  onPendingLaunchCompleted,
  onNewChat: externalOnNewChat,
  // @RESERVED — will be used for sidebar title refresh after auto-title generation
  onRefreshSessions: _onRefreshSessions,
}: AgentChatPanelProps) {
  const [input, setInput] = useState("");
  const [currentModelId, setCurrentModelId] = useState("");
  const [selectedModel, setSelectedModel] = useState<UnifiedModel | null>(null);
  const [reasoningEffortOverride, setReasoningEffortOverride] = useState<string | undefined>(undefined);
  const [agentSpecs, setAgentSpecs] = useState<AgentSpec[]>([]);
  const [selectedAgentId, setSelectedAgentId] = useState("");
  const [pendingAttachments, setPendingAttachments] = useState<ChatAttachment[]>([]);
  const [scheduleDialogOpen, setScheduleDialogOpen] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const lastLaunchRequestIdRef = useRef<string | null>(null);

  const {
    messages,
    chatSession,
    chatTitleStatus,
    activeSessionBinding,
    runNativeAgent,
    stopAgentRun,
    stopAgentRunByRunId,
    clearMessagesAndContext,
    refreshActiveChat,
    isHydratingHistory,
    // @RESERVED — will be used for in-panel chat tab switching
    switchChat: _switchChat,
  } = useAgentChat(
    externalChatScopeId,
    _onRefreshSessions,
    propActiveSessionBinding,
  );

  const { activeChatRun } = useActiveChatRunState(externalChatScopeId);
  const resolvedActiveChatRun = activeSessionBinding ?? activeChatRun;
  const activeRunningMessage = useMemo(
    () =>
      [...messages]
        .reverse()
        .find(
          (message) =>
            message.type === "agent" &&
            message.runState === "running" &&
            (!resolvedActiveChatRun?.runId ||
              message.requestContext?.runId === resolvedActiveChatRun.runId),
        ) ??
      [...messages]
        .reverse()
        .find(
          (message) =>
            message.type === "agent" && message.runState === "running",
        ) ??
      null,
    [messages, resolvedActiveChatRun?.runId],
  );
  const hasCurrentThreadRun = Boolean(resolvedActiveChatRun || activeRunningMessage);
  const inputDisabled = isHydratingHistory;
  const submitDisabled = hasCurrentThreadRun || isHydratingHistory;
  const reasoningOptions = useMemo(() => getReasoningOptions(selectedModel), [selectedModel]);
  const stableReasoningOptions = reasoningOptions.length > 0 ? reasoningOptions : EMPTY_REASONING;

  const resolveActiveModelId = useCallback(async (): Promise<string | null> => {
    if (currentModelId.trim()) {
      return currentModelId;
    }

    try {
      const persistedModelId = (await tauri.getSelectedModel()).trim();
      if (!persistedModelId) {
        return null;
      }
      setCurrentModelId((prev) => prev || persistedModelId);
      return persistedModelId;
    } catch (error) {
      console.error("Failed to resolve selected model", error);
      return null;
    }
  }, [currentModelId]);

  const latestTelemetry = useMemo(() => {
    for (let i = messages.length - 1; i >= 0; i--) {
      if (messages[i].type === "agent" && messages[i].ragTelemetry) {
        return messages[i].ragTelemetry;
      }
    }
    return undefined;
  }, [messages]);

  const schedulePromptSeed = useMemo(() => {
    const composerDraft = input.trim();
    if (composerDraft) {
      return composerDraft;
    }

    for (let index = messages.length - 1; index >= 0; index -= 1) {
      const message = messages[index];
      if (message.type !== "user") {
        continue;
      }

      const content = message.content.trim();
      if (content) {
        return content;
      }
    }

    return "";
  }, [input, messages]);

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

  // Derive reasoning effort from options + user override — no effect needed
  const reasoningEffort = useMemo(() => {
    if (!stableReasoningOptions.length) return undefined;
    if (reasoningEffortOverride && stableReasoningOptions.includes(reasoningEffortOverride)) {
      return reasoningEffortOverride;
    }
    return stableReasoningOptions.includes("medium") ? "medium" : stableReasoningOptions[0];
  }, [stableReasoningOptions, reasoningEffortOverride]);

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

  const handleAddAttachments = useCallback(async () => {
    void open({
      multiple: true,
      filters: [
        { name: "Images", extensions: ["jpg", "jpeg", "png", "gif", "webp"] },
        { name: "Documents", extensions: ["pdf", "docx", "xlsx", "xls", "txt", "md", "csv"] },
        { name: "All Files", extensions: ["*"] },
      ],
    })
      .then(async (selected) => {
        if (!selected) return;
        const paths = Array.isArray(selected) ? selected : [selected];
        const previews = await tauri.prepareAttachmentPreviews(paths);
        setPendingAttachments((prev) =>
          [
            ...prev,
            ...previews.map((p) => ({
              id: `${p.path}-${Date.now()}`,
              path: p.path,
              filename: p.filename,
              mimeType: p.mime_type,
              sizeBytes: p.size_bytes,
              type: p.attachment_type,
              thumbnailDataUri: p.thumbnail_data_uri ?? undefined,
            })),
          ].slice(0, 5),
        );
      })
      .catch((error) => {
        console.error("Failed to open file picker:", error);
      });
  }, []);

  const handleRemoveAttachment = useCallback((id: string) => {
    setPendingAttachments((prev) => prev.filter((a) => a.id !== id));
  }, []);

  const handleSubmit = useCallback(async () => {
    const instruction = input.trim();
    if ((!instruction && pendingAttachments.length === 0) || submitDisabled) return;

    const activeModelId = await resolveActiveModelId();
    if (!activeModelId) {
      console.error("Cannot start agent run without a selected model");
      return;
    }

    const attachmentsToSend = pendingAttachments.slice();
    setInput("");
    setPendingAttachments([]);
    await runNativeAgent(
      instruction,
      activeModelId,
      workspacePath,
      selectedAgentId || undefined,
      stableReasoningOptions.length ? reasoningEffort : undefined,
      attachmentsToSend.length ? attachmentsToSend : undefined,
    );
  }, [input, pendingAttachments, reasoningEffort, resolveActiveModelId, runNativeAgent, selectedAgentId, stableReasoningOptions, submitDisabled, workspacePath]);

  const handleKeyDown = useCallback((event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void handleSubmit();
    }
  }, [handleSubmit]);

  const handleNewChat = useCallback(() => {
    if (externalOnNewChat) {
      void externalOnNewChat();
    } else {
      void clearMessagesAndContext(workspacePath);
    }
  }, [externalOnNewChat, clearMessagesAndContext, workspacePath]);

  const handleRefreshChat = useCallback(() => {
    void refreshActiveChat();
  }, [refreshActiveChat]);

  const handleComposerSubmit = useCallback(() => {
    void handleSubmit();
  }, [handleSubmit]);

  const handleStopActiveRun = useCallback(() => {
    if (resolvedActiveChatRun?.runId) {
      void stopAgentRunByRunId(resolvedActiveChatRun.runId);
      return;
    }
    if (activeRunningMessage) {
      void stopAgentRun(activeRunningMessage.id);
    }
  }, [activeRunningMessage, resolvedActiveChatRun?.runId, stopAgentRun, stopAgentRunByRunId]);

  useEffect(() => {
    if (!pendingLaunch) return;
    if (pendingLaunch.chatId !== externalChatScopeId) return;
    if (isHydratingHistory) return;
    if (lastLaunchRequestIdRef.current === pendingLaunch.requestId) return;

    void (async () => {
      const activeModelId = await resolveActiveModelId();
      if (!activeModelId) {
        return;
      }

      const consumed = await onPendingLaunchConsumed?.(pendingLaunch.requestId);
      if (consumed === false) {
        return;
      }

      lastLaunchRequestIdRef.current = pendingLaunch.requestId;
      const result = await runNativeAgent(
        pendingLaunch.prompt,
        activeModelId,
        workspacePath,
        selectedAgentId || undefined,
        stableReasoningOptions.length ? reasoningEffort : undefined,
        undefined,
        `Launchpad: ${pendingLaunch.preflight.scenarioTitle}\n${pendingLaunch.preflight.intentSummary}`,
      );

      await onPendingLaunchCompleted?.({
        requestId: pendingLaunch.requestId,
        scenarioId: pendingLaunch.scenarioId,
        workspaceId: pendingLaunch.workspaceId,
        chatId: result.chatScopeId,
        success: result.ok,
        actualToolIds: result.actualToolIds,
        actualTouchedPaths: result.actualTouchedPaths,
        producedArtifactPaths: result.producedArtifactPaths,
      });
    })();
  }, [
    externalChatScopeId,
    isHydratingHistory,
    onPendingLaunchCompleted,
    onPendingLaunchConsumed,
    pendingLaunch,
    reasoningEffort,
    resolveActiveModelId,
    runNativeAgent,
    selectedAgentId,
    stableReasoningOptions.length,
    workspacePath,
  ]);

  const scrollContainerRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    const container = scrollContainerRef.current;
    if (!container) return;

    const distanceFromBottom =
      container.scrollHeight - container.clientHeight - container.scrollTop;
    const isNearBottom = distanceFromBottom < 120;
    if (!isNearBottom && !hasCurrentThreadRun) {
      return;
    }

    container.scrollTo({
      top: container.scrollHeight,
      behavior: messages.length > 4 ? "smooth" : "auto",
    });
  }, [messages, hasCurrentThreadRun]);

  // ─── Render ────────────────────────────────────────────────────────
  const hasMessages = messages.length > 0;

  return (
    <div className={cn("relative h-full w-full overflow-hidden bg-transparent text-foreground", className)}>
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top,color-mix(in_srgb,var(--primary)_16%,transparent),transparent_30%),linear-gradient(180deg,color-mix(in_srgb,var(--background)_30%,transparent),transparent_26%,color-mix(in_srgb,var(--background)_92%,transparent))]" />

      <ChatTopbar
        chatSession={chatSession}
        titleStatus={chatTitleStatus}
        workspacePath={workspacePath}
        folders={folders}
        activeFolderId={activeFolderId}
        onSelectFolder={onSelectWorkspace}
        onAddFolder={onAddWorkspace}
        onNewChat={handleNewChat}
        onRefreshChat={handleRefreshChat}
        onOpenSettings={onOpenSettings}
      />

      <ScheduleTaskDialog
        open={scheduleDialogOpen}
        workspacePath={workspacePath}
        defaultPrompt={schedulePromptSeed}
        onClose={() => setScheduleDialogOpen(false)}
      />
      <div className="relative z-10 flex h-full flex-col pt-20">
        {!hasMessages ? (
          <div className="min-h-0 flex-1 overflow-y-auto">
            <div className="mx-auto flex min-h-full w-full max-w-6xl flex-col justify-center px-4 pb-12 pt-10 md:px-6">
              <div className="flex flex-1 flex-col items-center justify-center">
                <div className="mb-4 flex size-11 items-center justify-center rounded-2xl border border-border/70 bg-card/80 shadow-[0_24px_80px_-64px_rgba(0,0,0,0.7)] backdrop-blur-xl">
                  <Sparkles className="size-5 text-primary" />
                </div>

                <div className="mb-7 max-w-2xl text-center">
                  <div className="mb-3 inline-flex items-center rounded-full border border-border/70 bg-card/75 px-3 py-1 text-[11px] uppercase tracking-[0.18em] text-muted-foreground backdrop-blur-xl">
                    Nightless
                  </div>
                  <h1 className="text-3xl font-semibold tracking-[-0.04em] text-foreground">
                    Precision chat for governed workspace execution
                  </h1>
                  <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                    Dense timeline, cleaner streaming, tighter control surface. Built to feel deliberate instead of inflated.
                  </p>
                </div>

                <div className="mb-6 flex flex-wrap items-center justify-center gap-2">
                  <Badge variant="outline" className="rounded-full border-border/70 bg-card/72 px-2.5 py-0.5 text-[9px] uppercase tracking-[0.14em] backdrop-blur-xl">
                    Stable streaming
                  </Badge>
                  <Badge variant="outline" className="rounded-full border-border/70 bg-card/72 px-2.5 py-0.5 text-[9px] uppercase tracking-[0.14em] backdrop-blur-xl">
                    Dense work logs
                  </Badge>
                  <Badge variant="outline" className="rounded-full border-border/70 bg-card/72 px-2.5 py-0.5 text-[9px] uppercase tracking-[0.14em] backdrop-blur-xl">
                    Launchpad-ready
                  </Badge>
                </div>

                <EmptyStatePrompts onApplyPrompt={applyPrompt} />

                <ChatComposer
                  input={input}
                  onInputChange={setInput}
                  onKeyDown={handleKeyDown}
                  onSubmit={handleComposerSubmit}
                  inputDisabled={inputDisabled}
                  submitDisabled={submitDisabled}
                  stopDisabled={!hasCurrentThreadRun}
                  showStopButton={hasCurrentThreadRun}
                  onStop={handleStopActiveRun}
                  submitLabel={hasCurrentThreadRun ? "Stop active run" : "Send message"}
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
                  attachments={pendingAttachments}
                  onAddAttachments={handleAddAttachments}
                  onRemoveAttachment={handleRemoveAttachment}
                />
              </div>
            </div>
          </div>
        ) : (
          <>
            <div className="flex shrink-0 justify-center px-4 pb-3 pt-2 md:px-6">
              <TelemetryBar telemetry={latestTelemetry} />
            </div>
            <div ref={scrollContainerRef} className="min-h-0 flex-1 overflow-y-auto pb-34">
              <MessagesTimeline
                messages={messages}
                scrollContainer={scrollContainerRef.current}
              />
            </div>
          </>
        )}

        {hasMessages ? (
          <div className="pointer-events-none absolute inset-x-0 bottom-4 z-20 px-4 md:px-6">
            <div className="pointer-events-auto mx-auto w-full max-w-6xl">
              <ChatComposer
                input={input}
                onInputChange={setInput}
                onKeyDown={handleKeyDown}
                onSubmit={handleComposerSubmit}
                inputDisabled={inputDisabled}
                submitDisabled={submitDisabled}
                stopDisabled={!hasCurrentThreadRun}
                showStopButton={hasCurrentThreadRun}
                onStop={handleStopActiveRun}
                submitLabel={hasCurrentThreadRun ? "Stop active run" : "Send message"}
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
                attachments={pendingAttachments}
                onAddAttachments={handleAddAttachments}
                onRemoveAttachment={handleRemoveAttachment}
              />
            </div>
          </div>
        ) : null}
      </div>
    </div>
  );
}
