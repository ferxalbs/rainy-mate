import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { AppWindow, Eraser, FileText, Gamepad2, Layers3 } from "lucide-react";

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
import { CURRENT_STORAGE_KEYS, getStoredValue, setStoredValue } from "../../lib/appIdentity";
import { TextAnimate } from "../ui/text-animate";

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
    title: "Build game",
    prompt: "Build a polished Snake game in this repo with responsive UI, score persistence, and keyboard controls.",
    accent: "text-sky-500",
  },
  {
    icon: Layers3,
    title: "Create slides",
    prompt: "Create a one-page PDF that summarizes this app for technical founders and investors.",
    accent: "text-rose-500",
  },
  {
    icon: FileText,
    title: "Build website",
    prompt: "Design and implement a crisp landing page for this project with production-ready copy, layout, and responsive styling.",
    accent: "text-emerald-500",
  },
  {
    icon: AppWindow,
    title: "Desktop app",
    prompt: "Design and implement a professional desktop workflow for this repository, including UX improvements and the core UI states.",
    accent: "text-amber-500",
  },
  {
    icon: Eraser,
    title: "More",
    prompt:
      "Inspect this repository, identify the highest-leverage next improvement, and implement it with clean modular changes and verification.",
    accent: "text-violet-500",
  },
];

const EMPTY_REASONING: string[] = [];

function resolveTimeGreeting(date: Date): {
  headline: string;
  subline: string;
} {
  const hour = date.getHours();

  if (hour >= 0 && hour < 2) {
    return {
      headline: "Past midnight. What still needs to get done?",
      subline: "Late-night execution, clean handoffs, and focused work.",
    };
  }

  if (hour >= 2 && hour < 5) {
    return {
      headline: "Early morning. What should I take over?",
      subline: "Quiet hours are good for decisive work and cleanup.",
    };
  }

  if (hour >= 5 && hour < 7) {
    return {
      headline: "At dawn already? What are we starting?",
      subline: "Use the first block of the day for the highest-leverage task.",
    };
  }

  if (hour >= 7 && hour < 12) {
    return {
      headline: "Good morning. What can I do for you?",
      subline: "Assign a task, ask a question, or launch a complete workflow.",
    };
  }

  if (hour >= 12 && hour < 18) {
    return {
      headline: "Good afternoon. What are we building?",
      subline: "Keep momentum high with a clear prompt and a concrete outcome.",
    };
  }

  if (hour >= 18 && hour < 21) {
    return {
      headline: "Good evening. What still matters today?",
      subline: "Finish the important work, not the noisy work.",
    };
  }

  return {
    headline: "Working late? What can I take off your plate?",
    subline: "Use the remaining hours for decisive execution and review.",
  };
}

// ─── Extracted sub-components for proper reconciliation ──────────────

const EmptyStatePrompts = React.memo(function EmptyStatePrompts({
  onApplyPrompt,
}: {
  onApplyPrompt: (prompt: string) => void;
}) {
  return (
    <div className="mt-4 flex w-full max-w-3xl flex-wrap items-center justify-center gap-2.5 px-2">
      {PROMPTS.map(({ accent, icon: Icon, prompt, title }) => (
        <button
          key={title}
          type="button"
          onClick={() => onApplyPrompt(prompt)}
          className="group inline-flex items-center gap-2 rounded-full border border-border/55 bg-card/44 px-3.5 py-1.5 text-[13px] text-foreground/86 shadow-[0_18px_60px_-48px_rgba(0,0,0,0.65)] backdrop-blur-xl transition-colors hover:bg-card/64"
        >
          <span
            className={cn(
              "flex size-4.5 items-center justify-center rounded-full border border-border/50 bg-background/65",
              accent,
            )}
          >
            <Icon className="size-3" />
          </span>
          <span>{title}</span>
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
    `embedding:${telemetry?.embeddingProfile || "gemini-embedding-2-preview"}`,
  ];

  if (telemetry?.lastModel) {
    items.push(`model:${telemetry.lastModel}`);
  }
  if (typeof telemetry?.totalTokens === "number" && telemetry.totalTokens > 0) {
    items.push(`tokens:${telemetry.totalTokens.toLocaleString()}`);
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
  const [showTelemetryChips, setShowTelemetryChips] = useState<boolean>(() => {
    return getStoredValue(CURRENT_STORAGE_KEYS.chatTelemetryChips) === "true";
  });
  const [timeGreeting, setTimeGreeting] = useState(() => resolveTimeGreeting(new Date()));
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

  useEffect(() => {
    const syncGreeting = () => {
      setTimeGreeting(resolveTimeGreeting(new Date()));
    };

    syncGreeting();
    const intervalId = window.setInterval(syncGreeting, 60_000);
    return () => {
      window.clearInterval(intervalId);
    };
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

  const handleToggleTelemetryChips = useCallback(() => {
    setShowTelemetryChips((current) => {
      const next = !current;
      setStoredValue(CURRENT_STORAGE_KEYS.chatTelemetryChips, String(next));
      return next;
    });
  }, []);

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
        showTelemetryChips={showTelemetryChips}
        onToggleTelemetryChips={handleToggleTelemetryChips}
      />

      <ScheduleTaskDialog
        open={scheduleDialogOpen}
        workspacePath={workspacePath}
        defaultPrompt={schedulePromptSeed}
        onClose={() => setScheduleDialogOpen(false)}
      />
      <div className="relative z-10 flex h-full flex-col pt-16">
        {!hasMessages ? (
          <div className="min-h-0 flex-1 overflow-y-auto">
            <div className="mx-auto flex min-h-full w-full max-w-6xl flex-col justify-center px-4 pb-12 pt-8 md:px-6">
              <div className="flex flex-1 flex-col items-center justify-center">
                <div className="mb-5 max-w-2xl text-center">
                  <TextAnimate
                    as="h1"
                    by="word"
                    animation="blurInUp"
                    duration={0.28}
                    className="text-[clamp(2rem,4vw,3rem)] font-semibold tracking-[-0.055em] text-foreground"
                  >
                    {timeGreeting.headline}
                  </TextAnimate>
                  <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                    {timeGreeting.subline}
                  </p>
                </div>

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

                <EmptyStatePrompts onApplyPrompt={applyPrompt} />
              </div>
            </div>
          </div>
        ) : (
          <>
            {showTelemetryChips ? (
              <div className="flex shrink-0 justify-center px-4 pb-2 pt-1 md:px-6">
                <TelemetryBar telemetry={latestTelemetry} />
              </div>
            ) : null}
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
