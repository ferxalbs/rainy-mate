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
  Sparkles,
  Trash2,
  Zap,
  Info,
  Eraser,
} from "lucide-react";
import { useAgentChat } from "../../hooks/useAgentChat";
import { useTheme } from "../../hooks/useTheme";
import { MacOSToggle } from "../layout/MacOSToggle";

import { UnifiedModelSelector } from "../ai/UnifiedModelSelector";
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

  useEffect(() => {
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
    loadSpecs();
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
    sendInstruction,
    streamChat,
    executePlan,
    executeDiscussedPlan,
    executeToolCalls,
    clearMessages,
    clearMessagesAndContext,
    runNativeAgent,
  } = useAgentChat();

  const isProcessing = isPlanning || isExecuting;

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const handleSubmit = async () => {
    if (!input.trim() || isProcessing) return;
    const instruction = input.trim();
    setInput("");

    if (isNativeMode) {
      await runNativeAgent(
        instruction,
        currentModelId,
        workspacePath,
        selectedAgentId || undefined,
      );
      return;
    }

    const selectedSpec = agentSpecs.find((s) => s.id === selectedAgentId);
    const selectedAgentContext = selectedSpec
      ? `[ACTIVE AGENT PROFILE]
Name: ${selectedSpec.soul.name}
Description: ${selectedSpec.soul.description}
Personality: ${selectedSpec.soul.personality}
Tone: ${selectedSpec.soul.tone}
Soul:
${selectedSpec.soul.soul_content}`
      : undefined;

    // In Deep Mode, inject system context that tells AI about our specific file tools
    const hiddenContext = isDeepProcessing
      ? `[SYSTEM: You are a Planning Agent with access to these FILE OPERATIONS:
- write_file(path, content) - Creates or overwrites a file
- append_file(path, content) - Appends content to a file
- read_file(path) - Reads file contents
- list_files(path) - Lists files in a directory
- search_files(query, path) - Searches for content in files

IMPORTANT: When the user asks you to create/modify/read files, propose a plan using THESE EXACT OPERATIONS.
DO NOT suggest shell commands like 'touch', 'echo', 'mkdir'. Use our tools instead.
After you propose the plan, the user will click "Execute Task" to run it.

Example: If user says "create a test file", respond with:
"I'll use write_file to create the file with the content you need.

**Plan:**
1. write_file("test.txt", "Hello World")

Click 'Execute Task' when ready."]`
      : undefined;

    const mergedHiddenContext =
      selectedAgentContext && hiddenContext
        ? `${selectedAgentContext}\n\n${hiddenContext}`
        : (selectedAgentContext ?? hiddenContext);

    await streamChat(instruction, currentModelId, mergedHiddenContext);
  };

  const handlePlan = async () => {
    // If in Deep Mode, we execute the plan from the chat discussion
    if (isDeepProcessing) {
      // We don't need input for execution if we have chat history
      if (messages.length > 0) {
        await executeDiscussedPlan(workspacePath, currentModelId);
        return;
      }
    }

    // Legacy/Fallback Plan Mode (creates task from input)
    if (!input.trim() || isProcessing) return;
    const instruction = input.trim();
    setInput("");
    await sendInstruction(instruction, workspacePath, currentModelId);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      // If Deep Processing is on, we still default to Chat for "Ask before Plan"
      // But maybe we allow generic Cmd+Enter to force Plan?
      if ((e.metaKey || e.ctrlKey) && isDeepProcessing) {
        handlePlan();
      } else {
        handleSubmit();
      }
    }
  };

  // Dynamic state for processing mode
  const [isDeepProcessing, setIsDeepProcessing] = useState(false);
  const [isNativeMode, setIsNativeMode] = useState(false);

  const renderInputArea = (centered: boolean) => (
    <div
      className={`w-full max-w-2xl mx-auto transition-all duration-500 ${
        centered ? "scale-100 opacity-100" : "scale-100 opacity-100"
      }`}
    >
      <div
        className={`relative group rounded-[28px] border transition-all duration-300 ${
          isDeepProcessing
            ? "bg-background/40 backdrop-blur-xl border-white/10 shadow-xl shadow-purple-500/5"
            : "bg-background/40 backdrop-blur-xl border-white/10 shadow-lg"
        }`}
      >
        <TextArea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={
            isDeepProcessing
              ? "Discuss a complex task (⌘+Enter to Plan)..."
              : "Message Agent..."
          }
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
                  className="text-muted-foreground/50 hover:text-foreground hover:bg-muted/50 rounded-full w-8 h-8"
                >
                  <Paperclip className="size-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <span className="text-xs">Attach files</span>
              </TooltipContent>
            </Tooltip>

            {/* Deep Think Toggle */}
            <Tooltip delay={0}>
              <TooltipTrigger>
                <button
                  onClick={() => setIsDeepProcessing(!isDeepProcessing)}
                  className={`flex items-center gap-1.5 px-2 py-1 rounded-full border transition-all duration-300 ${
                    isDeepProcessing
                      ? "bg-purple-500/10 border-purple-500/20 text-purple-400"
                      : "bg-transparent border-transparent text-muted-foreground/50 hover:bg-muted/30"
                  }`}
                >
                  <Sparkles
                    className={`size-3 ${isDeepProcessing ? "text-purple-400" : "text-current"}`}
                  />
                  <span className="text-[10px] font-medium">Deep Mode</span>
                </button>
              </TooltipTrigger>
              <TooltipContent>
                <span className="text-xs">
                  Enable advanced reasoning (Chat first, Plan optionally)
                </span>
              </TooltipContent>
            </Tooltip>

            {/* Native Agent Toggle */}
            <Tooltip delay={0}>
              <TooltipTrigger>
                <button
                  onClick={() => setIsNativeMode(!isNativeMode)}
                  className={`flex items-center gap-1.5 px-2 py-1 rounded-full border transition-all duration-300 ${
                    isNativeMode
                      ? "bg-green-500/10 border-green-500/20 text-green-400"
                      : "bg-transparent border-transparent text-muted-foreground/50 hover:bg-muted/30"
                  }`}
                >
                  <Zap
                    className={`size-3 ${isNativeMode ? "text-green-400" : "text-current"}`}
                  />
                  <span className="text-[10px] font-medium">
                    Native Runtime
                  </span>
                </button>
              </TooltipTrigger>
              <TooltipContent>
                <span className="text-xs">
                  Run Autonomous Agent directly on Rust Runtime
                </span>
              </TooltipContent>
            </Tooltip>
          </div>

          <div className="flex items-center gap-2">
            {/* Explicit Plan Button (Visible in Deep Mode or if text > 10 chars) */}
            {(isDeepProcessing || input.length > 10) && (
              <Button
                size="sm"
                onPress={handlePlan}
                isDisabled={
                  isDeepProcessing
                    ? isProcessing || messages.length === 0
                    : !input.trim() || isProcessing
                }
                className={`rounded-full h-8 px-3 text-xs font-medium transition-all duration-300 ${
                  isDeepProcessing
                    ? "bg-purple-500/10 text-purple-400 hover:bg-purple-500/20"
                    : "bg-muted/50 text-muted-foreground hover:bg-muted/80"
                }`}
              >
                {isDeepProcessing ? "Execute Task" : "Plan Task"}
              </Button>
            )}

            <Button
              size="sm"
              isIconOnly
              onPress={handleSubmit}
              isDisabled={!input.trim() || isProcessing}
              isPending={isProcessing}
              className={`rounded-full transition-all duration-300 shadow-sm ${
                input.trim()
                  ? isDeepProcessing
                    ? "bg-purple-600 hover:bg-purple-500 text-white scale-100 opacity-100 translate-y-0"
                    : "bg-foreground text-background scale-100 opacity-100 translate-y-0"
                  : "bg-muted text-muted-foreground scale-90 opacity-0 translate-y-2 pointer-events-none"
              }`}
            >
              {!isProcessing && <ArrowUp className="size-4" />}
            </Button>
          </div>
        </div>
      </div>
      {isDeepProcessing && centered && (
        <div className="mt-2 text-center">
          <p className="text-[10px] text-muted-foreground flex items-center justify-center gap-1">
            <Info className="size-3" />
            Deep Mode: Chat to analyze & plan (Enter). Cmd+Enter to execute.
          </p>
        </div>
      )}
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
          <div className="flex items-center gap-2 rounded-full border border-white/10 bg-background/50 px-2 py-1">
            <span className="text-[10px] uppercase tracking-wide text-muted-foreground/80">
              Agent
            </span>
            <select
              value={selectedAgentId}
              onChange={(e) => setSelectedAgentId(e.target.value)}
              className="bg-transparent text-xs outline-none min-w-[160px]"
            >
              {agentSpecs.length === 0 && (
                <option value="">Default Agent</option>
              )}
              {agentSpecs.map((spec) => (
                <option key={spec.id} value={spec.id}>
                  {spec.soul.name || "Untitled Agent"}
                </option>
              ))}
            </select>
          </div>

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
                      console.error("Failed to clear persisted chat context:", e);
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

      {/* Scrollable Content Area - Absolute Inset - Z-10 */}
      <div className="absolute inset-0 overflow-y-auto w-full h-full scrollbar-none z-10">
        {/* Padding to clear top bar and bottom input */}
        <div
          className={`flex flex-col px-4 max-w-3xl mx-auto ${
            messages.length === 0
              ? "h-full justify-center pt-20"
              : "min-h-full pt-32 pb-40"
          }`}
        >
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
                  "Rainy Agent") + " is ready to assist with your workspace tasks."}
              </p>

              {renderInputArea(true)}

              {/* Suggestions */}
              <div className="mt-12 grid grid-cols-2 gap-4 max-w-lg w-full px-4 mb-20">
                <SuggestionCard
                  icon={<Zap className="text-amber-400" />}
                  title="Quick Question"
                  desc="Fast answers using lightweight models"
                  onClick={() => {
                    const fastModel = "gemini-2.5-flash";
                    handleModelSelect(fastModel);
                    setIsDeepProcessing(false);
                    setInput("How do I...");
                  }}
                />
                <SuggestionCard
                  icon={<Sparkles className="text-indigo-400" />}
                  title="Deep Analysis"
                  desc="Complex tasks using reasoning models"
                  onClick={() => {
                    const deepModel = "gemini-2.5-pro";
                    handleModelSelect(deepModel);
                    setIsDeepProcessing(true);
                    setInput("Analyze this project and...");
                  }}
                />
              </div>
            </div>
          ) : (
            <div className="space-y-8">
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
          <div className="w-full max-w-2xl pointer-events-auto">
            {renderInputArea(false)}
          </div>
        </div>
      )}
    </div>
  );
}

function SuggestionCard({
  icon,
  title,
  desc,
  onClick,
}: {
  icon: React.ReactNode;
  title: string;
  desc: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="flex flex-col gap-2 p-5 rounded-2xl bg-white/5 hover:bg-white/10 hover:scale-[1.02] border border-white/5 hover:border-white/10 transition-all text-left group backdrop-blur-sm"
    >
      <div className="size-10 rounded-xl bg-background/50 flex items-center justify-center mb-1 group-hover:bg-background transition-colors shadow-sm">
        {React.cloneElement(
          icon as React.ReactElement<{ className?: string }>,
          {
            className: "size-5",
          },
        )}
      </div>
      <div>
        <span className="block text-sm font-medium mb-0.5">{title}</span>
        <span className="text-xs text-muted-foreground/80 font-light leading-relaxed">
          {desc}
        </span>
      </div>
    </button>
  );
}
