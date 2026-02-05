import React, { useState, useRef, useEffect } from "react";
import {
  Button,
  TextArea,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@heroui/react";
import * as tauri from "../../services/tauri";
import { Paperclip, ArrowUp, Sparkles, Trash2, Zap, Info } from "lucide-react";
import { useAgentChat } from "../../hooks/useAgentChat";
import { useTheme } from "../../hooks/useTheme";
import { MacOSToggle } from "../layout/MacOSToggle";

import { UnifiedModelSelector } from "../ai/UnifiedModelSelector";
import { MessageBubble } from "./MessageBubble";

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
    cancelPlan,
    clearMessages,
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

    if (isDeepProcessing) {
      // Deep processing use legacy Cowork Agent (Plan -> Execute)
      await sendInstruction(instruction, workspacePath);
    } else {
      // Fast chat uses Unified Streaming
      await streamChat(instruction, currentModelId);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  // Always use agent capabilities (plan -> execute flow)
  // This ensures file editing, research, and other agent features work
  // regardless of which model is selected
  const isDeepProcessing = true;

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
          placeholder={"Message Agent..."}
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
            {isDeepProcessing && (
              <div className="flex items-center gap-1.5 px-2 py-1 bg-purple-500/5 rounded-full border border-purple-500/10 cursor-help select-none">
                <Sparkles className="size-3 text-purple-400" />
                <span className="text-[10px] font-medium text-purple-400/80">
                  Deep Think
                </span>
              </div>
            )}
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
      {isDeepProcessing && centered && (
        <div className="mt-2 text-center">
          <p className="text-[10px] text-muted-foreground flex items-center justify-center gap-1">
            <Info className="size-3" />
            Uses advanced reasoning models. May take longer to respond.
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
            <Button
              size="sm"
              variant="ghost"
              isIconOnly
              onPress={clearMessages}
              className="rounded-full w-8 h-8 text-muted-foreground hover:text-red-400 hover:bg-red-400/10"
            >
              <Trash2 className="size-3.5" />
            </Button>
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
                Rainy Agent is ready to assist with your workspace tasks.
              </p>

              {renderInputArea(true)}

              {/* Suggestions */}
              <div className="mt-12 grid grid-cols-2 gap-4 max-w-lg w-full px-4 mb-20">
                <SuggestionCard
                  icon={<Zap className="text-amber-400" />}
                  title="Quick Question"
                  desc="Fast answers using lightweight models"
                  onClick={() => {
                    const fastModel = "rainy:gemini-2.0-flash";
                    handleModelSelect(fastModel);
                    setInput("How do I...");
                  }}
                />
                <SuggestionCard
                  icon={<Sparkles className="text-indigo-400" />}
                  title="Deep Analysis"
                  desc="Complex tasks using reasoning models"
                  onClick={() => {
                    const deepModel = "cowork:gemini-2.5-pro";
                    handleModelSelect(deepModel);
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
                  onCancel={cancelPlan}
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
