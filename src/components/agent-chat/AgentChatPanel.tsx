import React, { useState, useRef, useEffect } from "react";
import {
  Button,
  TextArea,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@heroui/react";
import * as tauri from "../../services/tauri";
import {
  Paperclip,
  ArrowUp,
  Eraser,
  Trash2,
  Zap,
  Sparkles,
  Circle,
  Square,
  Wrench,
} from "lucide-react";
import { useAgentChat } from "../../hooks/useAgentChat";
import { useTheme } from "../../hooks/useTheme";
import { MacOSToggle } from "../layout/MacOSToggle";
import { toast } from "sonner";

import { UnifiedModelSelector } from "../ai/UnifiedModelSelector";
import { AgentSelector } from "./AgentSelector";
import { MessageBubble } from "./MessageBubble";
import { AgentSpec } from "../../types/agent-spec";

interface AgentChatPanelProps {
  workspacePath: string;
  onClose?: () => void;
  onOpenSettings?: () => void;
  className?: string;
}

export function AgentChatPanel({
  workspacePath,
  onClose,
  // onOpenSettings,
}: AgentChatPanelProps) {
  const { mode, setMode } = useTheme();
  const [input, setInput] = useState("");
  const [currentModelId, setCurrentModelId] = useState<string>("");
  const [agentSpecs, setAgentSpecs] = useState<AgentSpec[]>([]);
  const [selectedAgentId, setSelectedAgentId] = useState<string>("");
  const [forgeRecordingId, setForgeRecordingId] = useState<string | null>(null);
  const [isForgeRecording, setIsForgeRecording] = useState(false);
  const [isForgeBusy, setIsForgeBusy] = useState(false);
  const [pendingForgeSpec, setPendingForgeSpec] = useState<any | null>(null);
  const [forgeStatusText, setForgeStatusText] = useState<string>("");
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Initialize with default model if none selected
  useEffect(() => {
    const initModel = async () => {
      try {
        // Reuse existing command or use new one
        const model = await tauri.getSelectedModel();
        if (model) setCurrentModelId(model);
      } catch (e) {
        console.error("Failed to load selected model", e);
      }
    };
    initModel();
  }, []);

  const loadSpecs = async () => {
    try {
      const specs = (await tauri.listAgentSpecs()) as AgentSpec[];
      setAgentSpecs(specs);
      if (specs.length > 0) {
        setSelectedAgentId((prev) => prev || specs[0].id);
      }
    } catch (e) {
      console.error("Failed to load saved agents", e);
    }
  };

  useEffect(() => {
    loadSpecs();
  }, []);

  useEffect(() => {
    const hydrateForgeState = async () => {
      try {
        const active = await tauri.getActiveWorkflowRecording();
        if (active) {
          setForgeRecordingId(active.id);
          setIsForgeRecording(true);
        }
      } catch (e) {
        console.error("Failed to hydrate forge recording state", e);
      }
    };
    hydrateForgeState();
  }, []);

  const handleModelSelect = async (modelId: string) => {
    setCurrentModelId(modelId);
    try {
      await tauri.setSelectedModel(modelId);
    } catch (e) {
      console.error("Failed to persist model selection", e);
    }
  };

  const {
    messages,
    isPlanning,
    isExecuting,
    currentPlan,
    executePlan,
    executeToolCalls,
    runNativeAgent,
    clearMessages,
    clearMessagesAndContext,
    hydrateLongChatHistory,
    loadOlderHistory,
    hasMoreHistory,
    isHydratingHistory,
  } = useAgentChat();

  const isProcessing = isPlanning || isExecuting;
  const latestTelemetry = [...messages]
    .reverse()
    .find((m) => m.type === "agent" && m.ragTelemetry)?.ragTelemetry;

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  useEffect(() => {
    hydrateLongChatHistory();
  }, [hydrateLongChatHistory]);

  const handleSubmit = async () => {
    if (!input.trim() || isProcessing) return;
    const instruction = input.trim();
    setInput("");

    if (isForgeRecording && forgeRecordingId) {
      try {
        await tauri.recordWorkflowStep({
          kind: "user_instruction",
          label: instruction,
          payload: {
            modelId: currentModelId,
            workspacePath,
            selectedAgentId: selectedAgentId || null,
          },
        });
      } catch (e) {
        console.error("Failed to record forge step", e);
      }
    }

    // Always use Native Agent
    await runNativeAgent(
      instruction,
      currentModelId,
      workspacePath,
      selectedAgentId || undefined,
    );

    if (isForgeRecording && forgeRecordingId) {
      try {
        await tauri.recordWorkflowStep({
          kind: "agent_run",
          label: "run_agent_workflow",
          payload: {
            selectedAgentId: selectedAgentId || null,
            modelId: currentModelId,
          },
        });
      } catch (e) {
        console.error("Failed to record forge run step", e);
      }
    }
  };

  const handleStartForgeRecording = async () => {
    if (isForgeBusy || isForgeRecording) return;
    setIsForgeBusy(true);
    try {
      const rec = await tauri.startWorkflowRecording({
        title: `Forge Session ${new Date().toLocaleString()}`,
      });
      setForgeRecordingId(rec.id);
      setIsForgeRecording(true);
      setForgeStatusText("Recording in progress.");
      toast.success("Workflow recording started");
    } catch (e: any) {
      console.error("Failed to start workflow recording", e);
      toast.error(e?.message ?? "Failed to start workflow recording");
    } finally {
      setIsForgeBusy(false);
    }
  };

  const handleStopAndGenerateForgeAgent = async () => {
    if (isForgeBusy || !isForgeRecording || !forgeRecordingId) return;
    setIsForgeBusy(true);
    try {
      const recording = await tauri.stopWorkflowRecording();
      const generated = await tauri.generateAgentSpecFromRecording({
        recordingId: recording.id,
        agentName: `Forge Agent ${new Date().toLocaleTimeString()}`,
      });
      setForgeRecordingId(null);
      setIsForgeRecording(false);
      setPendingForgeSpec(generated.generatedSpec);
      setForgeStatusText(
        `Generated draft from recording (${recording.stepCount} steps). Review before saving.`,
      );
      toast.success("Forge draft generated");
    } catch (e: any) {
      console.error("Failed to stop/generate forge agent", e);
      toast.error(e?.message ?? "Failed to generate Forge agent");
    } finally {
      setIsForgeBusy(false);
    }
  };

  const handleSaveForgeDraft = async () => {
    if (!pendingForgeSpec || isForgeBusy) return;
    setIsForgeBusy(true);
    try {
      const saved = await tauri.saveGeneratedAgent(pendingForgeSpec);
      await tauri.saveAgentSpec(pendingForgeSpec);
      setSelectedAgentId(pendingForgeSpec.id);
      setPendingForgeSpec(null);
      setForgeStatusText(`Generated agent "${saved.name}" saved and activated.`);
      toast.success(`Generated "${saved.name}"`);
      await loadSpecs();
    } catch (e: any) {
      console.error("Failed to save Forge draft", e);
      toast.error(e?.message ?? "Failed to save Forge draft");
    } finally {
      setIsForgeBusy(false);
    }
  };

  const handleDiscardForgeDraft = () => {
    setPendingForgeSpec(null);
    setForgeStatusText("Forge draft discarded.");
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const renderInputArea = (centered: boolean) => (
    <div
      className={`w-full max-w-2xl lg:max-w-3xl mx-auto transition-all duration-500 ${
        centered ? "scale-100 opacity-100" : "scale-100 opacity-100"
      }`}
    >
      <div
        className={`relative group rounded-[28px] border transition-all duration-300 bg-background/40 backdrop-blur-xl border-white/10 shadow-lg`}
      >
        <TextArea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Message Agent..."
          rows={centered ? 2 : 1}
          className={`w-full bg-transparent border-none shadow-none text-foreground placeholder:text-muted-foreground/40 focus:ring-0 px-5 py-4 resize-none ${
            centered
              ? "text-lg tracking-tight min-h-[90px]"
              : "text-sm min-h-[50px]"
          }`}
          disabled={isProcessing}
        />

        {/* Input Footer Controls */}
        <div className="flex items-center justify-between px-3 pb-3 mt-2">
          <div className="flex items-center gap-2">
            <Tooltip delay={0}>
              <TooltipTrigger>
                <Button
                  size="sm"
                  variant="ghost"
                  isIconOnly
                  onPress={handleStartForgeRecording}
                  isDisabled={isForgeBusy || isForgeRecording}
                  className="rounded-full w-8 h-8 text-muted-foreground hover:text-red-400 hover:bg-red-400/10"
                >
                  <Circle className="size-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <span className="text-xs">Start Forge recording</span>
              </TooltipContent>
            </Tooltip>
            <Tooltip delay={0}>
              <TooltipTrigger>
                <Button
                  size="sm"
                  variant="ghost"
                  isIconOnly
                  onPress={handleStopAndGenerateForgeAgent}
                  isDisabled={isForgeBusy || !isForgeRecording}
                  className="rounded-full w-8 h-8 text-muted-foreground hover:text-emerald-400 hover:bg-emerald-400/10"
                >
                  {isForgeBusy ? (
                    <Wrench className="size-3.5 animate-spin" />
                  ) : (
                    <Square className="size-3.5" />
                  )}
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <span className="text-xs">Stop & generate Forge agent</span>
              </TooltipContent>
            </Tooltip>
            <Tooltip delay={0}>
              <TooltipTrigger>
                <Button
                  size="sm"
                  variant="ghost"
                  isIconOnly
                  className="text-muted-foreground/50 hover:text-foreground hover:bg-muted/50 rounded-full w-8 h-8"
                >
                  <Paperclip className="size-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <span className="text-xs">Attach files</span>
              </TooltipContent>
            </Tooltip>
          </div>

          <div className="flex items-center gap-2">
            <Button
              size="sm"
              isIconOnly
              onPress={handleSubmit}
              isDisabled={!input.trim() || isProcessing}
              isPending={isProcessing}
              className={`rounded-full transition-all duration-300 shadow-sm ${
                input.trim()
                  ? "bg-foreground text-background scale-100 opacity-100 translate-y-0"
                  : "bg-muted text-muted-foreground scale-90 opacity-0 translate-y-2 pointer-events-none"
              }`}
            >
              {!isProcessing && <ArrowUp className="size-4" />}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );

  return (
    <div className="h-full w-full relative bg-transparent overflow-hidden text-foreground">
      {/* Background Ambience / Base Layer */}
      <div className="absolute inset-0 bg-gradient-to-b from-primary/5 via-background/50 to-background/80 pointer-events-none z-0" />

      {/* Top Bar - Absolute & Layered - Z-50 */}
      <div className="absolute top-0 left-0 right-0 z-50 flex justify-center pt-6 pointer-events-none">
        {/* Drag Region */}
        <div
          data-tauri-drag-region
          className="absolute inset-x-0 top-0 h-20 pointer-events-auto z-0"
        />

        <div className="relative z-10 flex items-center gap-3 p-1.5 pl-3 rounded-full bg-background/60 backdrop-blur-2xl border border-white/10 shadow-lg pointer-events-auto transition-all hover:bg-background/80">
          <AgentSelector
            selectedAgentId={selectedAgentId}
            onSelect={setSelectedAgentId}
            agentSpecs={agentSpecs}
          />

          <UnifiedModelSelector
            selectedModelId={currentModelId}
            onSelect={handleModelSelect}
          />

          <div className="w-px h-4 bg-border/20 mx-1" />

          <MacOSToggle
            isDark={mode === "dark"}
            onToggle={(checked) => setMode(checked ? "dark" : "light")}
          />

          <div className="w-px h-4 bg-border/20 mx-1" />

          <div className="flex items-center gap-1 pr-1">
            <Tooltip delay={0}>
              <TooltipTrigger>
                <Button
                  size="sm"
                  variant="ghost"
                  isIconOnly
                  onPress={clearMessages}
                  className="rounded-full w-8 h-8 text-muted-foreground hover:text-foreground hover:bg-muted/40"
                >
                  <Eraser className="size-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <span className="text-xs">Clear UI only</span>
              </TooltipContent>
            </Tooltip>
            <Tooltip delay={0}>
              <TooltipTrigger>
                <Button
                  size="sm"
                  variant="ghost"
                  isIconOnly
                  onPress={async () => {
                    try {
                      await clearMessagesAndContext(workspacePath);
                    } catch (e) {
                      console.error(
                        "Failed to clear persisted chat context:",
                        e,
                      );
                    }
                  }}
                  className="rounded-full w-8 h-8 text-muted-foreground hover:text-red-400 hover:bg-red-400/10"
                >
                  <Trash2 className="size-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <span className="text-xs">Delete context (memory)</span>
              </TooltipContent>
            </Tooltip>
            {onClose && (
              <Button
                size="sm"
                variant="ghost"
                isIconOnly
                onPress={onClose}
                className="rounded-full w-8 h-8 text-muted-foreground hover:text-foreground"
              >
                <Zap className="size-3.5" />
              </Button>
            )}
          </div>
        </div>
      </div>

      {pendingForgeSpec && (
        <div className="absolute left-1/2 -translate-x-1/2 top-24 z-40 w-[min(960px,92vw)] rounded-2xl border border-border/20 bg-background/85 backdrop-blur-xl p-4 shadow-xl">
          <div className="flex items-center justify-between mb-3">
            <h4 className="text-sm font-semibold">Forge Draft Review</h4>
            <span className="text-xs text-muted-foreground">
              {pendingForgeSpec.id}
            </span>
          </div>
          <div className="grid gap-3 md:grid-cols-2">
            <input
              value={pendingForgeSpec?.soul?.name ?? ""}
              onChange={(event) =>
                setPendingForgeSpec((prev: any) => ({
                  ...prev,
                  soul: { ...prev.soul, name: event.target.value },
                }))
              }
              className="h-10 rounded-lg border border-border bg-background/40 px-3 text-sm"
              placeholder="Agent name"
            />
            <input
              value={pendingForgeSpec?.soul?.description ?? ""}
              onChange={(event) =>
                setPendingForgeSpec((prev: any) => ({
                  ...prev,
                  soul: { ...prev.soul, description: event.target.value },
                }))
              }
              className="h-10 rounded-lg border border-border bg-background/40 px-3 text-sm"
              placeholder="Agent description"
            />
          </div>
          <TextArea
            value={pendingForgeSpec?.soul?.soul_content ?? ""}
            onChange={(event) =>
              setPendingForgeSpec((prev: any) => ({
                ...prev,
                soul: { ...prev.soul, soul_content: event.target.value },
              }))
            }
            rows={6}
            className="mt-3"
            placeholder="System prompt"
          />
          <div className="mt-3 flex items-center gap-2">
            <Button
              size="sm"
              onPress={handleSaveForgeDraft}
              isDisabled={isForgeBusy}
              className="bg-emerald-600 text-white"
            >
              Save & Activate
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onPress={handleDiscardForgeDraft}
              isDisabled={isForgeBusy}
            >
              Discard
            </Button>
          </div>
        </div>
      )}

      {/* Scrollable Content Area - Absolute Inset - Z-10 */}
      <div className="absolute inset-0 overflow-y-auto w-full h-full scrollbar-none z-10">
        {/* Padding to clear top bar and bottom input */}
        <div
          className={`flex flex-col px-4 md:px-8 w-full md:max-w-3xl lg:max-w-4xl mx-auto transition-all duration-300 ${
            messages.length === 0
              ? "h-full justify-center pb-10"
              : "min-h-full pt-32 pb-40"
          }`}
        >
          {forgeStatusText && (
            <div className="mb-4 text-xs text-muted-foreground text-center">
              {forgeStatusText}
            </div>
          )}

          {messages.length === 0 ? (
            <div className="flex-1 flex flex-col items-center justify-center">
              <div className="mb-8 relative group">
                <div className="absolute inset-0 bg-primary/20 blur-3xl rounded-full opacity-50" />
                <Sparkles className="size-16 text-foreground/20 relative z-10" />
              </div>

              <h1 className="text-3xl font-medium text-foreground mb-3 tracking-tight text-center">
                How can I help you?
              </h1>
              <p className="text-muted-foreground text-sm mb-10 text-center max-w-sm font-light">
                {(agentSpecs.find((s) => s.id === selectedAgentId)?.soul.name ||
                  "Rainy Agent") +
                  " is ready to assist with your workspace tasks."}
              </p>

              {renderInputArea(true)}

              {/* Suggestions */}
              {/* <div className="mt-12 grid grid-cols-2 gap-4 max-w-lg w-full px-4 mb-20">
              </div> */}
            </div>
          ) : (
            <div className="space-y-8">
              <div className="flex flex-wrap items-center gap-2 justify-center">
                <span className="px-2.5 py-1 rounded-full text-[10px] uppercase tracking-wide bg-muted/40 border border-border/40 text-muted-foreground">
                  History: {latestTelemetry?.historySource || "persisted_long_chat"}
                </span>
                <span className="px-2.5 py-1 rounded-full text-[10px] uppercase tracking-wide bg-muted/40 border border-border/40 text-muted-foreground">
                  Retrieval: {latestTelemetry?.retrievalMode || "unavailable"}
                </span>
                <span className="px-2.5 py-1 rounded-full text-[10px] uppercase tracking-wide bg-muted/40 border border-border/40 text-muted-foreground">
                  Embedding: {latestTelemetry?.embeddingProfile || "gemini-embedding-2-preview"}
                </span>
                {latestTelemetry?.compressionApplied && (
                  <span className="px-2.5 py-1 rounded-full text-[10px] uppercase tracking-wide bg-muted/40 border border-border/40 text-muted-foreground">
                    Compression: auto @
                    {latestTelemetry.compressionTriggerTokens || 80000} (best-practice)
                  </span>
                )}
              </div>
              {hasMoreHistory && (
                <div className="flex justify-center">
                  <Button
                    size="sm"
                    variant="ghost"
                    onPress={loadOlderHistory}
                    isDisabled={isHydratingHistory}
                    className="rounded-full text-muted-foreground hover:text-foreground hover:bg-muted/40"
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
                  workspaceId={workspacePath}
                />
              ))}
              <div ref={messagesEndRef} />
            </div>
          )}
        </div>
      </div>

      {/* Floating Input Area - Absolute Bottom - ONLY SHOW WHEN MESSAGES EXIST */}
      {messages.length > 0 && (
        <div className="absolute bottom-6 left-0 right-0 z-40 px-4 pointer-events-none flex justify-center">
          <div className="w-full max-w-2xl lg:max-w-3xl pointer-events-auto">
            {renderInputArea(false)}
          </div>
        </div>
      )}
    </div>
  );
}
