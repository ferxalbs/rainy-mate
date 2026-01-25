// Rainy Cowork - CoworkPanel Component
// Chat-style AI agent interface for file operations

import { useState, useRef, useEffect, useMemo } from "react";
import { Button, TextArea, Spinner } from "@heroui/react";
import * as tauri from "../../services/tauri";
import {
  Settings as SettingsIcon,
  BrainCircuit,
  FolderSearch,
  Play,
  Send,
  Sparkles,
  Trash2,
  X,
  AlertCircle,
} from "lucide-react";
import { useCoworkAgent, AgentMessage } from "../../hooks/useCoworkAgent";
import { useCoworkStatus } from "../../hooks/useCoworkStatus";
import { useAIProvider } from "../../hooks/useAIProvider";

interface CoworkPanelProps {
  workspacePath: string;
  onClose?: () => void;
  onOpenSettings?: () => void;
}

export function CoworkPanel({
  workspacePath,
  onClose,
  onOpenSettings,
}: CoworkPanelProps) {
  const [input, setInput] = useState("");
  const [currentModel, setCurrentModel] = useState<string>("");
  const [availableModels, setAvailableModels] = useState<tauri.ModelOption[]>([]);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Fetch current model and available models on mount
  useEffect(() => {
    const fetchData = async () => {
      try {
        const [model, models] = await Promise.all([
          tauri.getSelectedModel(),
          tauri.getAvailableModels()
        ]);
        setCurrentModel(model);
        setAvailableModels(models);
      } catch (err) {
        console.error("Failed to fetch model data", err);
      }
    };
    fetchData();
  }, []);

  const {
    messages,
    isPlanning,
    isExecuting,
    currentPlan,
    sendInstruction,
    executePlan,
    cancelPlan,
    analyzeWorkspace,
    clearMessages,
  } = useCoworkAgent();

  const isProcessing = isPlanning || isExecuting;

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const handleSubmit = async () => {
    if (!input.trim() || isProcessing) return;

    const instruction = input.trim();
    setInput("");
    await sendInstruction(instruction, workspacePath);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const quickActions = [
    {
      label: "Analyze",
      icon: FolderSearch,
      action: () => analyzeWorkspace(workspacePath),
    },
    {
      label: "Organize by type",
      icon: Sparkles,
      action: () =>
        sendInstruction("Organize all files by type", workspacePath),
    },
  ];

  // Cowork status for validation
  const { hasPaidPlan, isLoading: statusLoading } = useCoworkStatus();

  // AI provider for API key checks
  const { hasApiKey } = useAIProvider();

  // Find the current model in available models to get provider info
  const currentModelInfo = availableModels.find(model => model.id === currentModel);

  // Check if current model is actually available
  const isModelAvailable = useMemo(() => {
    if (!currentModelInfo) return false;

    switch (currentModelInfo.provider) {
      case "Rainy API":
        return hasApiKey("rainy_api");
      case "Cowork Subscription":
        return hasPaidPlan;
      case "Google Gemini":
        return hasApiKey("gemini");
      default:
        return false;
    }
  }, [currentModelInfo, hasPaidPlan, hasApiKey]);

  // Determine model display logic based on actual provider
  const isCoworkModel = currentModelInfo?.provider === "Rainy API" || currentModelInfo?.provider === "Cowork Subscription";
  const isByokModel = currentModelInfo?.provider === "Google Gemini";
  const isFallback = !isModelAvailable && !statusLoading;

  // Display model name - show actual if fallback
  const modelDisplay = isFallback
    ? "Gemini Flash (Fallback)"
    : currentModelInfo?.name || currentModel || "Loading...";

  const isCoworkBadge = isCoworkModel && isModelAvailable;
  const isByokBadge = isByokModel && isModelAvailable;

  return (
    <div className="flex flex-col h-full bg-neutral-950/50 backdrop-blur-xl rounded-2xl border border-white/10">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <Sparkles className="w-5 h-5 text-purple-400" />
            <span className="font-medium text-white">AI Cowork Agent</span>
          </div>

          {/* Model Indicator */}
          <div
            className={`flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs border cursor-pointer hover:bg-white/5 transition-colors ${
              isFallback
                ? "bg-orange-500/10 border-orange-500/30 text-orange-300"
                : isCoworkBadge
                  ? "bg-purple-500/10 border-purple-500/30 text-purple-300"
                  : isByokBadge
                    ? "bg-blue-500/10 border-blue-500/30 text-blue-300"
                    : "bg-gray-500/10 border-gray-500/30 text-gray-300"
            }`}
            onClick={onOpenSettings}
            title={
              isFallback
                ? "Subscription required. Using free model fallback."
                : "Click to change model in settings"
            }
          >
            {isFallback ? (
              <AlertCircle className="w-3 h-3" />
            ) : (
              <BrainCircuit className="w-3 h-3" />
            )}
            <span className="font-medium truncate max-w-[120px]">
              {modelDisplay}
            </span>
            <span className="opacity-60 text-[10px] uppercase tracking-wider">
              {isFallback ? "FREE" : isCoworkBadge ? "COWORK" : isByokBadge ? "BYOK" : "UNKNOWN"}
            </span>
          </div>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            isIconOnly
            onPress={onOpenSettings}
            className="text-neutral-400 hover:text-white"
          >
            <SettingsIcon className="w-4 h-4" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            isIconOnly
            onPress={clearMessages}
            isDisabled={messages.length === 0}
          >
            <Trash2 className="w-4 h-4" />
          </Button>
          {onClose && (
            <Button variant="ghost" size="sm" isIconOnly onPress={onClose}>
              <X className="w-4 h-4" />
            </Button>
          )}
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {messages.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center text-neutral-400">
            <Sparkles className="w-12 h-12 mb-4 opacity-50" />
            <p className="text-lg font-medium mb-2">AI File Assistant</p>
            <p className="text-sm max-w-xs">
              Describe what you want to do with your files. For example:
              "Organize my downloads by file type" or "Rename all photos with
              date prefix"
            </p>

            {/* Quick Actions */}
            <div className="flex gap-2 mt-6">
              {quickActions.map((action) => (
                <Button
                  key={action.label}
                  variant="secondary"
                  size="sm"
                  onPress={action.action}
                  isDisabled={isProcessing}
                >
                  <action.icon className="w-4 h-4 mr-1" />
                  {action.label}
                </Button>
              ))}
            </div>
          </div>
        ) : (
          messages.map((message) => (
            <MessageBubble
              key={message.id}
              message={message}
              currentPlan={currentPlan}
              isExecuting={isExecuting}
              onExecute={executePlan}
              onCancel={cancelPlan}
            />
          ))
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="p-4 border-t border-white/10">
        <div className="flex gap-2">
          <TextArea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Describe what you want to do..."
            rows={2}
            className="flex-1 bg-neutral-900/50 border-white/10 text-white placeholder:text-neutral-500"
            disabled={isProcessing}
          />
          <Button
            variant="primary"
            isIconOnly
            onPress={handleSubmit}
            isDisabled={!input.trim() || isProcessing}
            isPending={isProcessing}
          >
            <Send className="w-4 h-4" />
          </Button>
        </div>
        <p className="text-xs text-neutral-500 mt-2">
          Press Enter to send • Shift+Enter for new line
        </p>
      </div>
    </div>
  );
}

// Message bubble component
interface MessageBubbleProps {
  message: AgentMessage;
  currentPlan: ReturnType<typeof useCoworkAgent>["currentPlan"];
  isExecuting: boolean;
  onExecute: (planId: string) => void;
  onCancel: (planId: string) => void;
}

function MessageBubble({
  message,
  currentPlan,
  isExecuting,
  onExecute,
  onCancel,
}: MessageBubbleProps) {
  const isUser = message.type === "user";
  const isSystem = message.type === "system";

  return (
    <div className={`flex ${isUser ? "justify-end" : "justify-start"}`}>
      <div
        className={`max-w-[85%] rounded-2xl px-4 py-3 ${
          isUser
            ? "bg-purple-600 text-white"
            : isSystem
              ? "bg-amber-500/20 text-amber-200 border border-amber-500/30"
              : "bg-neutral-800 text-neutral-100"
        }`}
      >
        {message.isLoading && (
          <div className="flex items-center gap-2 mb-2">
            <Spinner size="sm" color="current" />
            <span className="text-sm opacity-70">Processing...</span>
          </div>
        )}

        <div className="whitespace-pre-wrap text-sm">{message.content}</div>

        {/* Plan Actions */}
        {message.plan &&
          currentPlan?.id === message.plan.id &&
          !message.result && (
            <div className="flex gap-2 mt-3 pt-3 border-t border-white/10">
              <Button
                variant="primary"
                size="sm"
                onPress={() => onExecute(message.plan!.id)}
                isDisabled={isExecuting}
                isPending={isExecuting}
              >
                <Play className="w-3 h-3 mr-1" />
                Execute
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onPress={() => onCancel(message.plan!.id)}
                isDisabled={isExecuting}
              >
                Cancel
              </Button>
            </div>
          )}

        {/* Execution Result */}
        {message.result && (
          <div className="mt-2 pt-2 border-t border-white/10 text-xs opacity-70">
            {message.result.completedSteps}/{message.result.totalSteps} steps •
            {message.result.totalChanges} changes •{message.result.durationMs}ms
          </div>
        )}
      </div>
    </div>
  );
}

export default CoworkPanel;
