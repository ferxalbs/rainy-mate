// src/components/agents/AIResearchPanel.tsx
// AI Research Panel - Premium UI
// Uses HeroUI v3 compound components
// Part of Phase 3 - Rainy Cowork

import { useState, useEffect } from "react";
import {
  Card,
  Button,
  Spinner,
  Input,
  Label,
  Radio,
  RadioGroup,
  Select,
  ListBox,
} from "@heroui/react";
import { Search, Globe, Copy } from "lucide-react";
import { useWebResearch } from "../../hooks/useWebResearch";
import { useCoworkModels } from "../../hooks/useCoworkModels";
import { MarkdownRenderer } from "../shared/MarkdownRenderer";

export function AIResearchPanel() {
  const { researchTopic, researchResult, isResearching, error } =
    useWebResearch();

  const { models, loading: modelsLoading } = useCoworkModels();
  const [topic, setTopic] = useState("");
  const [depth, setDepth] = useState<"basic" | "advanced">("basic");
  const [provider, setProvider] = useState<"exa" | "tavily">("exa");
  const [selectedModel, setSelectedModel] = useState<string>("");
  const [thinkingLevel, setThinkingLevel] = useState<
    "minimal" | "low" | "medium" | "high"
  >("medium");
  const [copied, setCopied] = useState(false);

  // Set default model when loaded
  useEffect(() => {
    if (models.length > 0 && !selectedModel) {
      setSelectedModel(models[0]);
    }
  }, [models, selectedModel]);

  const handleResearch = async () => {
    if (!topic.trim()) return;
    await researchTopic(
      topic,
      depth,
      5,
      provider,
      selectedModel || undefined,
      selectedModel?.includes("gemini-3") ? thinkingLevel : undefined,
    );
  };

  const handleCopy = async () => {
    if (!researchResult?.content) return;
    await navigator.clipboard.writeText(researchResult.content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const showThinkingLevel = selectedModel?.includes("gemini-3");
  const isProModel = selectedModel?.includes("gemini-3-pro");
  const availableLevels = isProModel
    ? ["low", "high"]
    : ["minimal", "low", "medium", "high"];

  // Reset to valid level when switching models
  useEffect(() => {
    if (
      showThinkingLevel &&
      isProModel &&
      (thinkingLevel === "minimal" || thinkingLevel === "medium")
    ) {
      setThinkingLevel("low");
    }
  }, [selectedModel, thinkingLevel, showThinkingLevel, isProModel]);

  return (
    <div className="space-y-6 max-w-4xl mx-auto">
      {/* Header */}
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-2">
          <Globe className="size-5 text-primary" />
          <h2 className="text-xl font-semibold tracking-tight">
            AI Web Research
          </h2>
        </div>
        <p className="text-muted-foreground text-sm">
          Perform deep web research and gather insights automatically.
        </p>
      </div>

      {/* Input Section */}
      <Card className="border-border/50 shadow-sm bg-card/50 backdrop-blur-sm">
        <Card.Content className="p-6 space-y-6">
          {/* Topic Input */}
          <div className="space-y-2">
            <Label className="text-sm font-medium">Research Topic</Label>
            <Input
              type="text"
              placeholder="What would you like to research? (e.g., 'Competitor analysis of AI coding tools')"
              value={topic}
              onChange={(e) => setTopic(e.target.value)}
              disabled={isResearching}
              className="w-full bg-background/50 border-border/50 focus-within:bg-background transition-colors h-12"
            />
          </div>

          {/* Depth Selection */}
          <div className="space-y-2">
            <Label className="text-sm font-medium">Research Depth</Label>
            <RadioGroup
              value={depth}
              onChange={(val) => setDepth(val as "basic" | "advanced")}
              orientation="horizontal"
              className="flex gap-4"
            >
              <Radio value="basic" className="max-w-[200px]">
                <div className="flex flex-col gap-1">
                  <span className="text-sm font-medium">Basic</span>
                  <span className="text-xs text-muted-foreground">
                    Faster, broad overview
                  </span>
                </div>
              </Radio>
              <Radio value="advanced" className="max-w-[200px]">
                <div className="flex flex-col gap-1">
                  <span className="text-sm font-medium">Advanced</span>
                  <span className="text-xs text-muted-foreground">
                    Thorough, deep dive
                  </span>
                </div>
              </Radio>
            </RadioGroup>
          </div>

          {/* Model Selection */}
          <div className="space-y-2">
            <Label className="text-sm font-medium">AI Model</Label>
            <div className="flex gap-4 items-start flex-wrap">
              <Select
                className="max-w-xs"
                placeholder="Select an AI model"
                selectedKey={selectedModel || null}
                onSelectionChange={(key) => {
                  if (key) setSelectedModel(key.toString());
                }}
                isDisabled={isResearching || modelsLoading}
              >
                <Label>Select Agent Model</Label>
                <Select.Trigger>
                  <Select.Value />
                  <Select.Indicator />
                </Select.Trigger>
                <Select.Popover>
                  <ListBox>
                    {models.map((model) => (
                      <ListBox.Item key={model} id={model} textValue={model}>
                        {model}
                      </ListBox.Item>
                    ))}
                  </ListBox>
                </Select.Popover>
              </Select>

              {showThinkingLevel && (
                <div className="flex flex-col gap-2 min-w-[200px] animate-in fade-in slide-in-from-left-2">
                  <Select
                    className="w-full"
                    placeholder="Thinking Level"
                    selectedKey={thinkingLevel}
                    onSelectionChange={(key) => {
                      if (key) setThinkingLevel(key.toString() as any);
                    }}
                    isDisabled={isResearching}
                  >
                    <Label>Thinking Level</Label>
                    <Select.Trigger>
                      <Select.Value />
                      <Select.Indicator />
                    </Select.Trigger>
                    <Select.Popover>
                      <ListBox>
                        {availableLevels.map((level) => (
                          <ListBox.Item
                            key={level}
                            id={level}
                            textValue={`⚡ ${level.charAt(0).toUpperCase() + level.slice(1)}`}
                          >
                            <div className="flex items-center gap-2">
                              <span>
                                {level === "minimal"
                                  ? "⚡"
                                  : level === "low"
                                    ? "⚡⚡"
                                    : level === "medium"
                                      ? "⚡⚡⚡"
                                      : "⚡⚡⚡⚡"}
                              </span>
                              <span>
                                {level.charAt(0).toUpperCase() + level.slice(1)}
                              </span>
                            </div>
                          </ListBox.Item>
                        ))}
                      </ListBox>
                    </Select.Popover>
                  </Select>
                </div>
              )}
            </div>
            <p className="text-xs text-muted-foreground">
              Select the AI model that will analyze your research.{" "}
              {showThinkingLevel &&
                "Higher thinking levels provide deeper reasoning."}
            </p>
          </div>

          {/* Provider Selection */}
          <div className="space-y-2">
            <Label className="text-sm font-medium">Search Provider</Label>
            <RadioGroup
              value={provider}
              onChange={(val) => setProvider(val as "exa" | "tavily")}
              orientation="horizontal"
              className="flex gap-4"
            >
              <Radio value="exa" className="max-w-[200px]">
                <div className="flex flex-col gap-1">
                  <span className="text-sm font-medium">Exa AI</span>
                  <span className="text-xs text-muted-foreground">
                    Best for deep context
                  </span>
                </div>
              </Radio>
              <Radio value="tavily" className="max-w-[200px]">
                <div className="flex flex-col gap-1">
                  <span className="text-sm font-medium">Tavily</span>
                  <span className="text-xs text-muted-foreground">
                    Best for latest news
                  </span>
                </div>
              </Radio>
            </RadioGroup>
          </div>

          {/* Error Display */}
          {error && (
            <div className="p-3 text-sm text-red-500 bg-red-500/10 rounded-lg border border-red-500/20">
              {error}
            </div>
          )}

          {/* Research Button */}
          <div className="flex justify-end pt-2">
            <Button
              variant="primary"
              size="lg"
              className="w-full sm:w-auto min-w-[140px] font-medium shadow-md shadow-primary/20"
              onPress={handleResearch}
              isDisabled={!topic.trim() || isResearching}
            >
              {isResearching ? (
                <>
                  <Spinner size="sm" color="current" />
                  <span>Researching...</span>
                </>
              ) : (
                <>
                  <Search className="size-4" />
                  <span>Start Research</span>
                </>
              )}
            </Button>
          </div>
        </Card.Content>
      </Card>

      {/* Research Results */}
      {researchResult && researchResult.success && (
        <div className="animate-in fade-in slide-in-from-bottom-4 duration-500">
          <Card className="border-border/50 shadow-md overflow-hidden">
            <Card.Header className="flex justify-between items-center bg-muted/30 border-b border-border/50 px-6 py-4">
              <div className="flex items-center gap-2">
                <Globe className="size-4 text-primary" />
                <span className="font-medium">Research Results</span>
              </div>
              <div className="flex gap-2">
                <Button
                  size="sm"
                  variant="ghost"
                  onPress={handleCopy}
                  className="hover:bg-background/80"
                >
                  <Copy className="size-4" />
                  {copied ? "Copied" : "Copy"}
                </Button>
              </div>
            </Card.Header>
            <Card.Content className="p-0">
              <div className="bg-background">
                <div className="p-8 max-h-[600px] overflow-auto custom-scrollbar">
                  <MarkdownRenderer content={researchResult.content || ""} />
                </div>
              </div>
            </Card.Content>
            <Card.Footer className="px-6 py-3 bg-muted/30 border-t border-border/50 text-xs text-muted-foreground flex justify-between">
              <span>
                Generated{" "}
                {researchResult.generatedAt
                  ? new Date(researchResult.generatedAt).toLocaleTimeString()
                  : "just now"}
              </span>
            </Card.Footer>
          </Card>
        </div>
      )}

      {/* Error result */}
      {researchResult && !researchResult.success && (
        <div className="p-4 rounded-xl border border-red-500/20 bg-red-500/10 animate-appear">
          <p className="text-sm text-red-600 dark:text-red-400 font-medium">
            Research Error: {researchResult.error}
          </p>
        </div>
      )}
    </div>
  );
}

export default AIResearchPanel;
