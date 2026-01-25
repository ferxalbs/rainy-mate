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
  CreditCard,
  X,
  ArrowLeft,
  Plus,
  Copy,
  Trash2,
  ExternalLink,
  Zap,
  Palette,
  Shield,
} from "lucide-react";
import * as tauri from "../../services/tauri";
import {
  useAIProvider,
  useCoworkStatus,
  useCoworkBilling,
  useCoworkKeys,
} from "../../hooks";
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
  const { hasApiKey, validateApiKey, storeApiKey, deleteApiKey } =
    useAIProvider();

  const {
    planName,
    hasPaidPlan,
    usagePercent,
    remainingUses,
    isOverLimit,
    isLoading: coworkLoading,
    status: coworkStatus,
    refresh: refreshCowork,
    error: coworkError,
  } = useCoworkStatus();

  // Cowork billing and API keys
  const {
    plans: coworkPlans,
    subscription,
    checkout,
    openPortal,
    isLoading: billingLoading,
  } = useCoworkBilling();

  const {
    keys: coworkApiKeys,
    createKey,
    revokeKey,
    isLoading: keysLoading,
  } = useCoworkKeys();

  // State for new API key modal
  const [showNewKeyModal, setShowNewKeyModal] = useState(false);
  const [newKeyName, setNewKeyName] = useState("");
  const [newKeyValue, setNewKeyValue] = useState<string | null>(null);
  const [isCreatingKey, setIsCreatingKey] = useState(false);
  const [checkoutLoading, setCheckoutLoading] = useState<string | null>(null);

  const [apiKeyInputs, setApiKeyInputs] = useState<Record<string, string>>({});
  const [showKeys, setShowKeys] = useState<Record<string, boolean>>({});
  const [validationStatus, setValidationStatus] = useState<
    Record<string, "idle" | "validating" | "valid" | "invalid">
  >({});
  const [savingStatus, setSavingStatus] = useState<Record<string, boolean>>({});

  const [coworkModelsData, setCoworkModelsData] =
    useState<tauri.CoworkModelsResponse | null>(null);
  const [rainyApiModels, setRainyApiModels] = useState<string[]>([]);
  const [geminiModels, setGeminiModels] = useState<string[]>([]);

  // Load models and current selection
  useEffect(() => {
    async function loadData() {
      try {
        const [coworkData, rainyModels, geminiModelsList, currentModel] =
          await Promise.all([
            tauri.getCoworkModels().catch(() => null),
            tauri.getProviderModels("rainy_api").catch(() => []),
            tauri.getProviderModels("gemini").catch(() => []),
            tauri.getSelectedModel(),
          ]);

        setCoworkModelsData(coworkData);
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
  }, [coworkStatus?.plan]); // Reload if plan changes

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
  const getProviderId = (type: ProviderType) => {
    if (type === "rainyApi") return "rainy_api";
    if (type === "coworkApi") return "cowork_api";
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
      await refreshCowork();
    } catch (error) {
      console.error("Failed to save API key:", error);
    } finally {
      setSavingStatus((prev) => ({ ...prev, [provider]: false }));
    }
  };

  const handleDeleteKey = async (provider: ProviderType) => {
    const providerId = getProviderId(provider);
    await deleteApiKey(providerId);
    await refreshCowork();
  };

  const toggleShowKey = (provider: ProviderType) => {
    setShowKeys((prev) => ({ ...prev, [provider]: !prev[provider] }));
  };

  const formatResetDate = (isoDate: string) => {
    if (!isoDate) return "N/A";
    try {
      return new Date(isoDate).toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
      });
    } catch {
      return "N/A";
    }
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
                className="px-3 py-1.5 rounded-lg text-sm font-medium text-muted-foreground hover:text-foreground data-[selected=true]:text-foreground data-[selected=true]:bg-background data-[selected=true]:shadow-sm transition-all focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ring-offset-background"
              >
                <div className="flex items-center gap-2">
                  <Bot className="size-4" />
                  AI Models
                </div>
              </Tabs.Tab>
              <Tabs.Tab
                id="keys"
                className="px-3 py-1.5 rounded-lg text-sm font-medium text-muted-foreground hover:text-foreground data-[selected=true]:text-foreground data-[selected=true]:bg-background data-[selected=true]:shadow-sm transition-all focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ring-offset-background"
              >
                <div className="flex items-center gap-2">
                  <Key className="size-4" />
                  API Keys
                </div>
              </Tabs.Tab>
              <Tabs.Tab
                id="subscription"
                className="px-3 py-1.5 rounded-lg text-sm font-medium text-muted-foreground hover:text-foreground data-[selected=true]:text-foreground data-[selected=true]:bg-background data-[selected=true]:shadow-sm transition-all focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ring-offset-background"
              >
                <div className="flex items-center gap-2">
                  <CreditCard className="size-4" />
                  Subscription
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
                  {/* Current Plan Badge */}
                  <div className="p-4 rounded-xl bg-muted/50 border border-border/50 flex justify-between items-center">
                    <div className="flex items-center gap-2">
                      <Sparkles className="size-5 text-primary" />
                      <span className="font-medium">
                        Current Plan: {coworkModelsData?.plan_name || "Free"}
                      </span>
                    </div>
                    {coworkModelsData?.plan === "free" && (
                      <Button
                        variant="primary"
                        size="sm"
                        onPress={() =>
                          window.open(
                            "https://enosislabs.com/pricing",
                            "_blank",
                          )
                        }
                      >
                        Upgrade
                      </Button>
                    )}
                  </div>

                  {/* 1. Cowork Subscription Models */}
                  <div>
                    <h3 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
                      <Sparkles className="size-4" />
                      Subscription Models (Cowork)
                    </h3>
                    {coworkModelsData && coworkModelsData.models.length > 0 ? (
                      <div className="grid gap-3">
                        {coworkModelsData.models.map((model) => (
                          <ModelCard
                            key={model}
                            id={model}
                            name={model}
                            description="Included in your plan"
                            isSelected={selectedModel === model}
                          />
                        ))}
                      </div>
                    ) : (
                      <div className="p-6 rounded-xl border border-dashed text-center">
                        <p className="text-muted-foreground text-sm">
                          Upgrade to access premium Cowork models
                        </p>
                      </div>
                    )}
                  </div>

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
                        {hasKey && (
                          <span className="text-xs text-green-600 flex items-center gap-1">
                            <Check className="size-3" />
                            Connected
                          </span>
                        )}
                      </div>

                      <p className="text-sm text-muted-foreground">
                        {provider.description}
                      </p>

                      {!hasKey ? (
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
                                placeholder="Enter API key..."
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
                              onPress={() => handleSaveKey(provider.id)}
                              isDisabled={
                                !apiKeyInputs[provider.id]?.trim() || saving
                              }
                            >
                              {saving ? "Saving..." : "Save to Keychain"}
                            </Button>
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
                        <div className="flex items-center justify-between">
                          <span className="text-sm text-muted-foreground">
                            API key stored in Keychain
                          </span>
                          <Button
                            variant="secondary"
                            size="sm"
                            onPress={() => handleDeleteKey(provider.id)}
                          >
                            Remove
                          </Button>
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </Tabs.Panel>

            {/* Subscription Tab */}
            <Tabs.Panel id="subscription" className="space-y-6">
              {/* Error Banner */}
              {coworkError && (
                <div className="p-4 rounded-xl bg-red-500/10 border border-red-500/20 text-red-500 text-sm flex items-center gap-2">
                  <Shield className="size-4" />
                  <span>Failed to load subscription: {coworkError}</span>
                </div>
              )}

              {/* Current Plan & Usage */}
              <div className="p-4 rounded-xl border bg-muted/50 border-border/50">
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Sparkles className="size-5 text-accent" />
                      <span className="font-semibold text-lg">{planName}</span>
                      {hasPaidPlan && (
                        <span className="text-xs bg-accent/10 text-accent px-2 py-0.5 rounded-full">
                          Active
                        </span>
                      )}
                    </div>
                    {subscription?.hasSubscription && (
                      <Button
                        variant="secondary"
                        size="sm"
                        onPress={async () => {
                          const url = await openPortal();
                          if (url) window.open(url, "_blank");
                        }}
                      >
                        <ExternalLink className="size-4" />
                        Manage Billing
                      </Button>
                    )}
                  </div>

                  <Separator />

                  {!coworkLoading && coworkStatus && (
                    <div className="space-y-3">
                      <div className="flex justify-between text-sm">
                        <span className="text-muted-foreground">
                          Monthly Usage
                        </span>
                        <span
                          className={
                            isOverLimit ? "text-red-500 font-medium" : ""
                          }
                        >
                          {coworkStatus.usage.used} / {coworkStatus.usage.limit}{" "}
                          uses
                        </span>
                      </div>

                      <div className="w-full bg-muted rounded-full h-2">
                        <div
                          className={`h-2 rounded-full transition-all ${
                            usagePercent >= 90
                              ? "bg-red-500"
                              : usagePercent >= 70
                                ? "bg-yellow-500"
                                : "bg-accent"
                          }`}
                          style={{ width: `${Math.min(100, usagePercent)}%` }}
                        />
                      </div>

                      <div className="flex justify-between text-xs text-muted-foreground">
                        <span>{remainingUses} remaining</span>
                        <span>
                          Resets {formatResetDate(coworkStatus.usage.resets_at)}
                        </span>
                      </div>
                    </div>
                  )}

                  {coworkLoading && (
                    <div className="flex items-center justify-center py-4">
                      <Spinner size="sm" />
                    </div>
                  )}
                </div>
              </div>

              {/* Available Plans */}
              <div>
                <h3 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
                  <CreditCard className="size-4" />
                  Available Plans
                </h3>
                {billingLoading ? (
                  <div className="flex items-center justify-center py-8">
                    <Spinner size="sm" />
                  </div>
                ) : (
                  <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
                    {coworkPlans
                      .filter((p) => p.id !== "free")
                      .map((plan) => {
                        const isCurrentPlan = subscription?.plan === plan.id;
                        const isLoading = checkoutLoading === plan.id;

                        return (
                          <div
                            key={plan.id}
                            className={`p-4 rounded-xl border transition-all ${
                              isCurrentPlan
                                ? "border-accent bg-accent/5"
                                : "bg-transparent border-border/50 hover:bg-muted/30 cursor-pointer"
                            }`}
                          >
                            <div className="space-y-3">
                              <div className="flex items-center justify-between">
                                <span className="font-semibold">
                                  {plan.name}
                                </span>
                                {isCurrentPlan && (
                                  <Check className="size-4 text-accent" />
                                )}
                              </div>

                              <div className="flex items-baseline gap-1">
                                <span className="text-2xl font-bold">
                                  ${plan.price}
                                </span>
                                <span className="text-sm text-muted-foreground">
                                  /month
                                </span>
                              </div>

                              <div className="text-xs text-muted-foreground space-y-1">
                                <div className="flex items-center gap-1">
                                  <Zap className="size-3" />
                                  {plan.usageLimit} uses/month
                                </div>
                                <div className="capitalize">
                                  {plan.modelAccessLevel} models
                                </div>
                              </div>

                              {!isCurrentPlan && plan.hasStripePrice && (
                                <Button
                                  variant="primary"
                                  size="sm"
                                  className="w-full"
                                  isDisabled={isLoading}
                                  onPress={async () => {
                                    setCheckoutLoading(plan.id);
                                    const url = await checkout(plan.id);
                                    if (url) window.open(url, "_blank");
                                    setCheckoutLoading(null);
                                  }}
                                >
                                  {isLoading ? (
                                    <Spinner size="sm" />
                                  ) : (
                                    "Subscribe"
                                  )}
                                </Button>
                              )}
                            </div>
                          </div>
                        );
                      })}
                  </div>
                )}
              </div>

              {/* Cowork API Keys */}
              <div>
                <div className="flex items-center justify-between mb-3">
                  <h3 className="text-sm font-medium text-muted-foreground flex items-center gap-2">
                    <Key className="size-4" />
                    Cowork API Keys
                  </h3>
                  <Button
                    variant="secondary"
                    size="sm"
                    onPress={() => setShowNewKeyModal(true)}
                  >
                    <Plus className="size-4" />
                    Create Key
                  </Button>
                </div>

                {keysLoading ? (
                  <div className="flex items-center justify-center py-8">
                    <Spinner size="sm" />
                  </div>
                ) : coworkApiKeys.length === 0 ? (
                  <div className="p-6 text-center rounded-xl border border-dashed border-border/50">
                    <Key className="size-8 mx-auto text-muted-foreground mb-2" />
                    <p className="text-muted-foreground text-sm">
                      No API keys yet
                    </p>
                    <p className="text-muted-foreground text-xs mt-1">
                      Create a key to use with the Rainy SDK
                    </p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    {coworkApiKeys.map((key) => (
                      <div
                        key={key.id}
                        className="p-3 flex items-center justify-between rounded-xl border border-border/50 bg-muted/50"
                      >
                        <div>
                          <span className="font-medium">{key.name}</span>
                          <p className="text-xs text-muted-foreground">
                            Created{" "}
                            {new Date(key.createdAt).toLocaleDateString()}
                            {key.lastUsed &&
                              ` • Last used ${new Date(key.lastUsed).toLocaleDateString()}`}
                          </p>
                        </div>
                        <Button
                          variant="secondary"
                          size="sm"
                          onPress={async () => {
                            if (confirm(`Revoke "${key.name}"?`)) {
                              await revokeKey(key.id);
                            }
                          }}
                        >
                          <Trash2 className="size-4 text-red-500" />
                        </Button>
                      </div>
                    ))}
                  </div>
                )}

                {/* New Key Modal */}
                {showNewKeyModal && (
                  <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
                    <div className="p-6 w-96 max-w-[90vw] rounded-xl border bg-background shadow-xl">
                      <h3 className="font-semibold text-lg mb-4">
                        {newKeyValue ? "API Key Created" : "Create New API Key"}
                      </h3>

                      {newKeyValue ? (
                        <div className="space-y-4">
                          <div className="p-3 bg-muted rounded-lg font-mono text-sm break-all">
                            {newKeyValue}
                          </div>
                          <p className="text-xs text-muted-foreground">
                            ⚠️ Copy this key now. You won't be able to see it
                            again.
                          </p>
                          <div className="flex gap-2">
                            <Button
                              variant="primary"
                              className="flex-1"
                              onPress={() => {
                                navigator.clipboard.writeText(newKeyValue);
                              }}
                            >
                              <Copy className="size-4" />
                              Copy Key
                            </Button>
                            <Button
                              variant="secondary"
                              onPress={() => {
                                setShowNewKeyModal(false);
                                setNewKeyValue(null);
                                setNewKeyName("");
                              }}
                            >
                              Done
                            </Button>
                          </div>
                        </div>
                      ) : (
                        <div className="space-y-4">
                          <TextField>
                            <Label>Key Name</Label>
                            <Input
                              placeholder="e.g., Development Key"
                              value={newKeyName}
                              onChange={(e) => setNewKeyName(e.target.value)}
                            />
                          </TextField>
                          <div className="flex gap-2">
                            <Button
                              variant="secondary"
                              onPress={() => {
                                setShowNewKeyModal(false);
                                setNewKeyName("");
                              }}
                            >
                              Cancel
                            </Button>
                            <Button
                              variant="primary"
                              className="flex-1"
                              isDisabled={!newKeyName.trim() || isCreatingKey}
                              onPress={async () => {
                                setIsCreatingKey(true);
                                const result = await createKey(newKeyName);
                                if (result) {
                                  setNewKeyValue(result.key);
                                }
                                setIsCreatingKey(false);
                              }}
                            >
                              {isCreatingKey ? (
                                <Spinner size="sm" />
                              ) : (
                                "Create Key"
                              )}
                            </Button>
                          </div>
                        </div>
                      )}
                    </div>
                  </div>
                )}
              </div>
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
