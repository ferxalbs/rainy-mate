import React, { useState, useRef, useEffect } from "react";
import {
  Button,
  TextArea,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@heroui/react";
import * as tauri from "../../services/tauri";
import { Paperclip, ArrowUp, Sparkles, Info, Trash2, Zap } from "lucide-react";
import { useCoworkAgent } from "../../hooks/useCoworkAgent";

import { UnifiedModelSelector } from "../ai/UnifiedModelSelector";

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
  } = useCoworkAgent();

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
        className={`relative group rounded-3xl border transition-all ${
          isDeepProcessing
            ? "bg-purple-500/5 border-purple-500/10 focus-within:bg-purple-500/10"
            : "bg-muted/20 border-border/10 focus-within:bg-muted/30"
        }`}
      >
        <TextArea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={"Message Agent..."}
          rows={centered ? 3 : 1}
          className={`w-full bg-transparent border-none shadow-none text-foreground placeholder:text-muted-foreground/50 focus:ring-0 px-4 py-3 resize-none ${
            centered ? "text-base min-h-[80px]" : "text-sm min-h-[44px]"
          }`}
          disabled={isProcessing}
        />

        {/* Input Footer Controls */}
        <div className="flex items-center justify-between px-2 pb-2 mt-1">
          <div className="flex items-center gap-1">
            <Tooltip delay={0}>
              <TooltipTrigger>
                <Button
                  size="sm"
                  variant="ghost"
                  isIconOnly
                  className="text-muted-foreground hover:text-foreground rounded-full"
                >
                  <Paperclip className="size-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <span className="text-xs">Attach files</span>
              </TooltipContent>
            </Tooltip>
            {isDeepProcessing && (
              <Tooltip delay={500}>
                <TooltipTrigger>
                  <div className="flex items-center gap-1.5 px-2 py-1 bg-purple-500/10 rounded text-[10px] text-purple-400 border border-purple-500/20 cursor-help">
                    <Sparkles className="size-3 text-purple-500" />
                    <span className="text-[10px] font-medium text-purple-400">
                      Deep Processing
                    </span>
                  </div>
                </TooltipTrigger>
                <TooltipContent>
                  <span className="text-xs">Detailed reasoning & planning</span>
                </TooltipContent>
              </Tooltip>
            )}
          </div>

          <div className="flex items-center gap-2">
            <UnifiedModelSelector
              selectedModelId={currentModelId}
              onSelect={handleModelSelect}
            />

            <Button
              size="sm"
              isIconOnly
              onPress={handleSubmit}
              isDisabled={!input.trim() || isProcessing}
              isPending={isProcessing}
              className={`rounded-full transition-all duration-200 ${
                input.trim()
                  ? isDeepProcessing
                    ? "bg-purple-600 text-white"
                    : "bg-foreground text-background"
                  : "bg-muted text-muted-foreground"
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
    <div className="flex flex-col h-full w-full relative bg-background/50">
      <div className="absolute inset-0 bg-gradient-to-tr from-primary/5 via-transparent to-secondary/5 pointer-events-none -z-10 opacity-30" />

      {/* Header/Toolbar */}
      <div className="h-14 shrink-0 border-b border-border/40 flex items-center justify-between px-4 bg-background/60 backdrop-blur-md sticky top-0 z-10">
        <div className="flex items-center gap-2">
          <div className="size-8 rounded-lg bg-primary/10 flex items-center justify-center">
            <Sparkles className="size-4 text-primary" />
          </div>
          <div>
            <h2 className="text-sm font-semibold">Agent Chat</h2>
            <p className="text-[10px] text-muted-foreground">
              AI-powered workspace assistant
            </p>
          </div>
        </div>
        <div className="flex items-center gap-1">
          <Button size="sm" variant="ghost" isIconOnly onPress={clearMessages}>
            <Trash2 className="size-4 text-muted-foreground" />
          </Button>
          {onClose && (
            <Button size="sm" variant="ghost" isIconOnly onPress={onClose}>
              <Zap className="size-4 text-muted-foreground" />
            </Button>
          )}
        </div>
      </div>

      {/* Messages Area */}
      <div className="flex-1 overflow-y-auto px-4 py-6 scrollbar-thin">
        {messages.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center -mt-20 animate-in fade-in zoom-in duration-500">
            <h1 className="text-2xl font-medium text-foreground mb-8 tracking-tight text-center">
              How can I help you today?
            </h1>
            {renderInputArea(true)}

            <div className="mt-8 grid grid-cols-2 gap-3 max-w-lg w-full">
              <SuggestionCard
                icon={<Zap className="text-yellow-500" />}
                title="Quick Question"
                desc="Fast answers using lightweight models"
                onClick={() => {
                  // Switch to fast model if needed
                  const fastModel = "rainy:gemini-2.0-flash";
                  handleModelSelect(fastModel);
                  setInput("How do I...");
                }}
              />
              <SuggestionCard
                icon={<Sparkles className="text-purple-500" />}
                title="Deep Analysis"
                desc="Complex tasks using reasoning models"
                onClick={() => {
                  // Switch to deep model
                  const deepModel = "cowork:gemini-2.5-pro";
                  handleModelSelect(deepModel);
                  setInput("Analyze this project and...");
                }}
              />
            </div>
          </div>
        ) : (
          <div className="space-y-6 max-w-3xl mx-auto pb-4">
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

      {/* Bottom Input Area (Visible when messages exist) */}
      {messages.length > 0 && (
        <div className="relative z-20 shrink-0 bg-background/80 backdrop-blur-xl border-t border-border/10 p-4">
          {renderInputArea(false)}
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
      className="flex flex-col gap-1 p-4 rounded-2xl bg-muted/40 hover:bg-muted/60 border border-transparent hover:border-border/20 transition-all text-left group"
    >
      <div className="size-8 rounded-full bg-background flex items-center justify-center mb-1 group-hover:scale-110 transition-transform">
        {React.cloneElement(
          icon as React.ReactElement<{ className?: string }>,
          {
            className: "size-4",
          },
        )}
      </div>
      <span className="text-sm font-medium">{title}</span>
      <span className="text-xs text-muted-foreground">{desc}</span>
    </button>
  );
}

// Reuse MessageBubble from CoworkPanel or extract to shared component
// For brevity, assuming MessageBubble is similar but imported or defined here.
// Ideally should be a shared component.
import { MessageBubble } from "./MessageBubble";
// Note: You might need to export MessageBubble from CoworkPanel.tsx if not already exported.
