// Rainy Cowork - Settings Page
// Full-page settings with AI model selection, API keys, and preferences

import { useState, useEffect, useCallback } from "react";
import {
  Button,
  Tabs,
  Switch,
  Label,
  Separator,
  TextField,
  Input,
  Spinner,
} from "@heroui/react";
import {
  Bot,
  Key,
  Check,
  Lock,
  Sparkles,
  Eye,
  EyeOff,
  X,
  ArrowLeft,
  Copy,
  Trash2,
  ExternalLink,
  Zap,
  Palette,
  Shield,
} from "lucide-react";
import * as tauri from "../../services/tauri";
import { useAIProvider } from "../../hooks";
import { AI_PROVIDERS, type ProviderType } from "../../types";
import { ThemeSelector } from "./ThemeSelector";
import { ThemeContext } from "../../providers/ThemeProvider";
import { useContext } from "react";

interface SettingsPageProps {
  initialTab?: string;
  onBack?: () => void;
}

export function SettingsPage({
  initialTab = "models",
  onBack,
}: SettingsPageProps) {
  const themeContext = useContext(ThemeContext);
  const [activeTab, setActiveTab] = useState(initialTab);
  const [selectedModel, setSelectedModel] = useState<string>("");
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);

  // API key management
  const { hasApiKey, validateApiKey, storeApiKey, getApiKey, deleteApiKey } =
    useAIProvider();

  // State for new API key modal REMOVED

  const [apiKeyInputs, setApiKeyInputs] = useState<Record<string, string>>({});
  const [showKeys, setShowKeys] = useState<Record<string, boolean>>({});
  const [validationStatus, setValidationStatus] = useState<
    Record<string, "idle" | "validating" | "valid" | "invalid">
  >({});
  const [savingStatus, setSavingStatus] = useState<Record<string, boolean>>({});
  const [visibleKeys, setVisibleKeys] = useState<Record<string, string>>({});
  const [replacingKeys, setReplacingKeys] = useState<Record<string, boolean>>(
    {},
  );

  const [rainyApiModels, setRainyApiModels] = useState<string[]>([]);
  const [geminiModels, setGeminiModels] = useState<string[]>([]);

  // Load models and current selection
  useEffect(() => {
    async function loadData() {
      try {
        const [rainyModels, geminiModelsList, currentModel] = await Promise.all(
          [
            tauri.getProviderModels("rainy_api").catch(() => []),
            tauri.getProviderModels("gemini").catch(() => []),
            tauri.getSelectedModel(),
          ],
        );

        setRainyApiModels(rainyModels || []);
        setGeminiModels(geminiModelsList || []);
        setSelectedModel(currentModel);
      } catch (error) {
        console.error("Failed to load settings:", error);
      } finally {
        setIsLoading(false);
      }
    }
    loadData();
  }, []);

  // Handle model selection
  const handleSelectModel = useCallback(async (modelId: string) => {
    setIsSaving(true);
    try {
      await tauri.setSelectedModel(modelId);
      setSelectedModel(modelId);
    } catch (error) {
      console.error("Failed to set model:", error);
    } finally {
      setIsSaving(false);
    }
  }, []);

  // API key handlers
  // API key handlers
  const getProviderId = (type: ProviderType) => {
    if (type === "rainyapi") return "rainy_api";
    return "gemini";
  };

  const handleApiKeyChange = (provider: ProviderType, value: string) => {
    setApiKeyInputs((prev) => ({ ...prev, [provider]: value }));
    setValidationStatus((prev) => ({ ...prev, [provider]: "idle" }));
  };

  const [validationError, setValidationError] = useState<
    Record<string, string>
  >({});

  const handleValidateKey = async (provider: ProviderType) => {
    const key = apiKeyInputs[provider];
    if (!key?.trim()) return;

    setValidationStatus((prev) => ({ ...prev, [provider]: "validating" }));
    setValidationError((prev) => ({ ...prev, [provider]: "" }));

    try {
      const providerId = getProviderId(provider);
      await validateApiKey(providerId, key);
      setValidationStatus((prev) => ({
        ...prev,
        [provider]: "valid",
      }));
    } catch (error) {
      setValidationStatus((prev) => ({ ...prev, [provider]: "invalid" }));
      setValidationError((prev) => ({
        ...prev,
        [provider]: error instanceof Error ? error.message : String(error),
      }));
    }
  };

  const handleSaveKey = async (provider: ProviderType) => {
    const key = apiKeyInputs[provider];
    if (!key?.trim()) return;

    setSavingStatus((prev) => ({ ...prev, [provider]: true }));

    try {
      const providerId = getProviderId(provider);
      await storeApiKey(providerId, key);
      setApiKeyInputs((prev) => ({ ...prev, [provider]: "" }));
      setValidationStatus((prev) => ({ ...prev, [provider]: "idle" }));
    } catch (error) {
      console.error("Failed to save API key:", error);
    } finally {
      setSavingStatus((prev) => ({ ...prev, [provider]: false }));
    }
  };

  const handleDeleteKey = async (provider: ProviderType) => {
    const providerId = getProviderId(provider);
    await deleteApiKey(providerId);
  };

  const toggleShowKey = (provider: ProviderType) => {
    setShowKeys((prev) => ({ ...prev, [provider]: !prev[provider] }));
  };

  const handleViewKey = async (provider: ProviderType) => {
    const providerId = getProviderId(provider);
    const key = await getApiKey(providerId);
    if (key) {
      setVisibleKeys((prev) => ({ ...prev, [getProviderId(provider)]: key }));
    }
  };

  const handleReplaceKey = (provider: ProviderType) => {
    // Clear the current key input and enable replacing mode
    const providerId = getProviderId(provider);
    setReplacingKeys((prev) => ({ ...prev, [providerId]: true }));
    setApiKeyInputs((prev) => ({ ...prev, [providerId]: "" }));
    // Also clear visibility if it was shown
    setVisibleKeys((prev) => {
      const next = { ...prev };
      delete next[providerId];
      return next;
    });
  };

  // Helper to render model card
  const ModelCard = ({
    id,
    name,
    description,
    isLocked = false,
    isSelected = false,
  }: {
    id: string;
    name: string;
    description: string;
    isLocked?: boolean;
    isSelected?: boolean;
  }) => (
    <div
      className={`p-4 rounded-xl border transition-all ${
        isSelected
          ? "bg-muted/50 border-primary/50 cursor-pointer"
          : !isLocked
            ? "bg-transparent border-transparent hover:bg-muted/30 cursor-pointer"
            : "opacity-60 cursor-not-allowed bg-transparent border-transparent"
      }`}
      onClick={() => !isLocked && handleSelectModel(id)}
    >
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            <span className="font-medium">{name}</span>
            {isLocked && <Lock className="size-3 text-muted-foreground" />}
            {isSelected && (
              <span className="text-xs bg-primary/10 text-primary px-2 py-0.5 rounded-full flex items-center gap-1">
                <Check className="size-3" />
                Active
              </span>
            )}
          </div>
          <p className="text-sm text-muted-foreground mt-1">{description}</p>
        </div>
        {isSelected ? (
          <div className="size-5 rounded-full border-2 border-primary bg-primary flex items-center justify-center">
            <Check className="size-3 text-white" />
          </div>
        ) : !isLocked ? (
          <div className="size-5 rounded-full border-2 border-muted-foreground/30" />
        ) : (
          <Button
            variant="secondary"
            size="sm"
            onPress={() =>
              window.open("https://enosislabs.com/pricing", "_blank")
            }
          >
            Upgrade
          </Button>
        )}
      </div>
    </div>
  );

  return (
    <div className="h-full flex flex-col overflow-hidden">
      {/* Header */}
      <div className="flex items-center gap-3 p-4 border-b border-border shrink-0">
        {onBack && (
          <Button variant="secondary" size="sm" onPress={onBack}>
            <ArrowLeft className="size-4" />
          </Button>
        )}
        <h1 className="text-xl font-semibold">Settings</h1>
      </div>

      {/* Tabs Content - Scrollable */}
      <div className="flex-1 overflow-y-auto min-h-0 pb-24">
        <div className="px-4 pt-4">
          <Tabs
            selectedKey={activeTab}
            onSelectionChange={(key) => setActiveTab(key as string)}
            className="w-full"
          >
            <Tabs.List className="mb-4 bg-muted/50 p-1 rounded-xl gap-1 backdrop-blur-xl border border-border/30">
              <Tabs.Tab
                id="models"
                className="px-3 py-1.5 rounded-lg text-sm font-medium text-muted-foreground hover:text-foreground hover:bg-muted/50 data-[selected=true]:text-foreground data-[selected=true]:bg-background transition-all focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ring-offset-background"
              >
                <div className="flex items-center gap-2">
                  <Bot className="size-4" />
                  AI Models
                </div>
              </Tabs.Tab>
              <Tabs.Tab
                id="keys"
                className="px-3 py-1.5 rounded-lg text-sm font-medium text-muted-foreground hover:text-foreground hover:bg-muted/50 data-[selected=true]:text-foreground data-[selected=true]:bg-background transition-all focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ring-offset-background"
              >
                <div className="flex items-center gap-2">
                  <Key className="size-4" />
                  API Keys
                </div>
              </Tabs.Tab>
              <Tabs.Tab
                id="appearance"
                className="px-3 py-1.5 rounded-lg text-sm font-medium text-muted-foreground hover:text-foreground data-[selected=true]:text-foreground data-[selected=true]:bg-background data-[selected=true]:shadow-sm transition-all focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ring-offset-background"
              >
                <div className="flex items-center gap-2">
                  <Palette className="size-4" />
                  Appearance
                </div>
              </Tabs.Tab>
              <Tabs.Tab
                id="permissions"
                className="px-3 py-1.5 rounded-lg text-sm font-medium text-muted-foreground hover:text-foreground data-[selected=true]:text-foreground data-[selected=true]:bg-background data-[selected=true]:shadow-sm transition-all focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ring-offset-background"
              >
                <div className="flex items-center gap-2">
                  <Shield className="size-4" />
                  Permissions
                </div>
              </Tabs.Tab>
            </Tabs.List>

            {/* Models Tab */}
            <Tabs.Panel id="models" className="space-y-8">
              {isLoading ? (
                <div className="flex items-center justify-center py-12">
                  <Spinner size="lg" />
                </div>
              ) : (
                <>
                  {/* 2. Rainy API (PAYG) */}
                  {hasApiKey("rainy_api") && (
                    <div>
                      <h3 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
                        <Zap className="size-4" />
                        Pay-As-You-Go Models (Rainy API)
                      </h3>
                      <div className="grid gap-3">
                        {rainyApiModels.map((model) => (
                          <ModelCard
                            key={model}
                            id={model}
                            name={model}
                            description="Billed per usage (1:1 Token)"
                            isSelected={selectedModel === model}
                          />
                        ))}
                      </div>
                    </div>
                  )}

                  {/* 3. Free Tier (Gemini BYOK) */}
                  {hasApiKey("gemini") && (
                    <div>
                      <h3 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
                        <Bot className="size-4" />
                        Free Tier (Gemini BYOK)
                      </h3>
                      <div className="grid gap-3">
                        {geminiModels.map((model) => (
                          <ModelCard
                            key={model}
                            id={model}
                            name={model}
                            description="Uses your own Gemini API Key"
                            isSelected={selectedModel === model}
                          />
                        ))}
                      </div>
                    </div>
                  )}

                  {isSaving && (
                    <div className="fixed bottom-4 right-4 bg-accent text-white px-4 py-2 rounded-lg flex items-center gap-2">
                      <Spinner size="sm" />
                      Saving...
                    </div>
                  )}
                </>
              )}
            </Tabs.Panel>

            {/* API Keys Tab */}
            <Tabs.Panel id="keys" className="space-y-4">
              {AI_PROVIDERS.map((provider) => {
                const providerId = getProviderId(provider.id);
                const hasKey = hasApiKey(providerId);
                const status = validationStatus[provider.id] || "idle";
                const saving = savingStatus[provider.id];
                const showKey = showKeys[provider.id];
                const visibleKey = visibleKeys[providerId];
                const isReplacing = replacingKeys[providerId];

                // Show input if: No key stored OR replacing mode is active
                const showInput = !hasKey || isReplacing;

                return (
                  <div
                    key={provider.id}
                    className="p-4 rounded-xl border bg-muted/50 border-border/50"
                  >
                    <div className="space-y-3">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <Sparkles className="size-4 text-accent" />
                          <span className="font-medium">{provider.name}</span>
                        </div>
                        {hasKey && !isReplacing && (
                          <span className="text-xs text-green-600 flex items-center gap-1">
                            <Check className="size-3" />
                            Stored in Keychain
                          </span>
                        )}
                      </div>

                      <p className="text-sm text-muted-foreground">
                        {provider.description}
                      </p>

                      {showInput ? (
                        <div className="space-y-2">
                          <div className="flex gap-2">
                            <TextField
                              className="flex-1"
                              name={`api-key-${provider.id}`}
                              type={showKey ? "text" : "password"}
                              onChange={(value) =>
                                handleApiKeyChange(provider.id, value)
                              }
                            >
                              <Input
                                placeholder={
                                  isReplacing
                                    ? "Enter new API key..."
                                    : "Enter API key..."
                                }
                                value={apiKeyInputs[provider.id] || ""}
                              />
                            </TextField>
                            <Button
                              variant="secondary"
                              size="sm"
                              onPress={() => toggleShowKey(provider.id)}
                            >
                              {showKey ? (
                                <EyeOff className="size-4" />
                              ) : (
                                <Eye className="size-4" />
                              )}
                            </Button>
                          </div>
                          <div className="flex gap-2">
                            <Button
                              variant="secondary"
                              size="sm"
                              onPress={() => handleValidateKey(provider.id)}
                              isDisabled={
                                !apiKeyInputs[provider.id]?.trim() ||
                                status === "validating"
                              }
                            >
                              {status === "validating"
                                ? "Validating..."
                                : "Validate"}
                            </Button>
                            <Button
                              variant="primary"
                              size="sm"
                              onPress={async () => {
                                await handleSaveKey(provider.id);
                                // If successful (handleSaveKey handles logic), disable replacing mode
                                setReplacingKeys((prev) => ({
                                  ...prev,
                                  [providerId]: false,
                                }));
                              }}
                              isDisabled={
                                !apiKeyInputs[provider.id]?.trim() || saving
                              }
                            >
                              {saving ? "Saving..." : "Save to Keychain"}
                            </Button>
                            {isReplacing && (
                              <Button
                                variant="tertiary"
                                size="sm"
                                onPress={() =>
                                  setReplacingKeys((prev) => ({
                                    ...prev,
                                    [providerId]: false,
                                  }))
                                }
                              >
                                Cancel
                              </Button>
                            )}
                          </div>
                          {status === "valid" && (
                            <p className="text-xs text-green-600 flex items-center gap-1">
                              <Check className="size-3" /> API key is valid
                            </p>
                          )}
                          {status === "invalid" && (
                            <div className="flex flex-col gap-1">
                              <p className="text-xs text-red-600 flex items-center gap-1">
                                <X className="size-3" /> Invalid API key
                              </p>
                              {validationError[provider.id] && (
                                <p className="text-xs text-red-500/80 pl-4">
                                  {validationError[provider.id]}
                                </p>
                              )}
                            </div>
                          )}
                        </div>
                      ) : (
                        <div className="flex flex-col gap-2">
                          {activeTab === "keys" && visibleKey && (
                            <div className="p-3 bg-muted rounded-lg border border-border/50 text-xs font-mono break-all relative group">
                              {visibleKey}
                              <Button
                                variant="ghost"
                                size="sm"
                                className="absolute top-1 right-1 opacity-0 group-hover:opacity-100 transition-opacity"
                                onPress={() =>
                                  navigator.clipboard.writeText(visibleKey)
                                }
                              >
                                <Copy className="size-3" />
                              </Button>
                            </div>
                          )}
                          <div className="flex items-center gap-2">
                            <Button
                              variant="secondary"
                              size="sm"
                              onPress={() => handleViewKey(provider.id)}
                              isDisabled={!!visibleKey}
                            >
                              <Eye className="size-4 mr-1" />
                              View
                            </Button>
                            <Button
                              variant="secondary"
                              size="sm"
                              onPress={() => handleReplaceKey(provider.id)}
                            >
                              <ExternalLink className="size-4 mr-1" />
                              Replace
                            </Button>
                            <Button
                              variant="tertiary"
                              size="sm"
                              className="text-red-500 hover:text-red-600"
                              onPress={() => handleDeleteKey(provider.id)}
                            >
                              <Trash2 className="size-4 mr-1" />
                              Remove
                            </Button>
                          </div>
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </Tabs.Panel>

            {/* Appearance Tab */}
            <Tabs.Panel id="appearance" className="space-y-4">
              <ThemeSelector />

              {/* Premium Animations Group */}

              <div className="space-y-6">
                <div className="flex items-center justify-between">
                  <div className="flex flex-col gap-1">
                    <span className="text-sm font-medium flex items-center gap-2">
                      <Sparkles className="size-4 text-primary" />
                      Premium Animations
                    </span>
                    <span className="text-xs text-muted-foreground">
                      Enable dynamic background effects (may impact battery)
                    </span>
                  </div>
                  <Switch
                    isSelected={themeContext?.enableAnimations}
                    onChange={(e) =>
                      themeContext?.setEnableAnimations(e.valueOf())
                    }
                  >
                    <Switch.Control>
                      <Switch.Thumb />
                    </Switch.Control>
                  </Switch>
                </div>

                <div className="flex items-center justify-between">
                  <div>
                    <Label className="font-medium">Compact Mode</Label>
                    <p className="text-sm text-muted-foreground">
                      Reduce spacing in UI
                    </p>
                  </div>
                  <Switch>
                    <Switch.Control>
                      <Switch.Thumb />
                    </Switch.Control>
                  </Switch>
                </div>
              </div>
            </Tabs.Panel>

            {/* Permissions Tab */}
            <Tabs.Panel id="permissions" className="space-y-4">
              <div className="space-y-6">
                <div className="flex items-center justify-between">
                  <div>
                    <Label className="font-medium">Notifications</Label>
                    <p className="text-sm text-muted-foreground">
                      Show task completion alerts
                    </p>
                  </div>
                  <Switch defaultSelected>
                    <Switch.Control>
                      <Switch.Thumb />
                    </Switch.Control>
                  </Switch>
                </div>

                <Separator />

                <div className="flex items-center justify-between">
                  <div>
                    <Label className="font-medium">Auto-execute Tasks</Label>
                    <p className="text-sm text-muted-foreground">
                      Start tasks immediately
                    </p>
                  </div>
                  <Switch>
                    <Switch.Control>
                      <Switch.Thumb />
                    </Switch.Control>
                  </Switch>
                </div>
              </div>
            </Tabs.Panel>
          </Tabs>
        </div>
      </div>
    </div>
  );
}
