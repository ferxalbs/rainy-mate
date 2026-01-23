// Rainy Cowork - Settings Page
// Full-page settings with AI model selection, API keys, and preferences

import { useState, useEffect, useCallback } from "react";
import {
  Card,
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
  Palette,
  Shield,
  Check,
  Lock,
  Sparkles,
  TrendingUp,
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
  const [models, setModels] = useState<tauri.ModelOption[]>([]);
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

  // Load models and current selection
  useEffect(() => {
    async function loadData() {
      try {
        const [availableModels, currentModel] = await Promise.all([
          tauri.getAvailableModels(),
          tauri.getSelectedModel(),
        ]);
        setModels(availableModels);
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
  const getProviderId = (type: ProviderType) =>
    type === "rainyApi" ? "rainy_api" : "gemini";

  const handleApiKeyChange = (provider: ProviderType, value: string) => {
    setApiKeyInputs((prev) => ({ ...prev, [provider]: value }));
    setValidationStatus((prev) => ({ ...prev, [provider]: "idle" }));
  };

  const handleValidateKey = async (provider: ProviderType) => {
    const key = apiKeyInputs[provider];
    if (!key?.trim()) return;

    setValidationStatus((prev) => ({ ...prev, [provider]: "validating" }));

    try {
      const providerId = getProviderId(provider);
      const isValid = await validateApiKey(providerId, key);
      setValidationStatus((prev) => ({
        ...prev,
        [provider]: isValid ? "valid" : "invalid",
      }));
    } catch {
      setValidationStatus((prev) => ({ ...prev, [provider]: "invalid" }));
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

  // Group models by tier
  const freeModels = models.filter((m) => !m.isPremium);
  const premiumModels = models.filter((m) => m.isPremium);

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center gap-3 p-4 border-b border-border">
        {onBack && (
          <Button variant="secondary" size="sm" onPress={onBack}>
            <ArrowLeft className="size-4" />
          </Button>
        )}
        <h1 className="text-xl font-semibold">Settings</h1>
      </div>

      {/* Tabs Content */}
      <div className="flex-1 overflow-auto">
        <Tabs
          selectedKey={activeTab}
          onSelectionChange={(key) => setActiveTab(key as string)}
          className="w-full"
        >
          <Tabs.List className="mb-4 bg-muted/50 p-1 rounded-xl gap-1">
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
          <Tabs.Panel id="models" className="space-y-6">
            {isLoading ? (
              <div className="flex items-center justify-center py-12">
                <Spinner size="lg" />
              </div>
            ) : (
              <>
                {/* Current Plan */}
                <Card className="p-4 bg-accent/5 border-accent/20">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Sparkles className="size-5 text-accent" />
                      <span className="font-medium">
                        Current Plan: {planName}
                      </span>
                    </div>
                    {!hasPaidPlan && (
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
                        <TrendingUp className="size-4" />
                        Upgrade
                      </Button>
                    )}
                  </div>
                </Card>

                {/* Free Tier Models */}
                <div>
                  <h3 className="text-sm font-medium text-muted-foreground mb-3">
                    Free Tier (Gemini BYOK)
                  </h3>
                  <div className="grid gap-3">
                    {freeModels.map((model) => (
                      <Card
                        key={model.id}
                        className={`p-4 cursor-pointer transition-all hover:border-accent/50 ${
                          selectedModel === model.id
                            ? "border-accent bg-accent/5"
                            : ""
                        }`}
                        onClick={() => handleSelectModel(model.id)}
                      >
                        <div className="flex items-start justify-between">
                          <div className="flex-1">
                            <div className="flex items-center gap-2">
                              <span className="font-medium">{model.name}</span>
                              {selectedModel === model.id && (
                                <span className="text-xs bg-accent/20 text-accent px-2 py-0.5 rounded-full flex items-center gap-1">
                                  <Check className="size-3" />
                                  Active
                                </span>
                              )}
                            </div>
                            <p className="text-sm text-muted-foreground mt-1">
                              {model.description}
                            </p>
                            <div className="flex items-center gap-3 mt-2 text-xs text-muted-foreground">
                              <span>Provider: {model.provider}</span>
                              <span>•</span>
                              <span>Thinking: {model.thinkingLevel}</span>
                            </div>
                          </div>
                          <div
                            className={`size-5 rounded-full border-2 flex items-center justify-center ${
                              selectedModel === model.id
                                ? "border-accent bg-accent"
                                : "border-muted-foreground"
                            }`}
                          >
                            {selectedModel === model.id && (
                              <Check className="size-3 text-white" />
                            )}
                          </div>
                        </div>
                      </Card>
                    ))}
                  </div>
                </div>

                {/* Premium Models */}
                <div>
                  <h3 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
                    <Lock className="size-4" />
                    Premium Models (Rainy API)
                  </h3>
                  <div className="grid gap-3">
                    {premiumModels.map((model) => (
                      <Card
                        key={model.id}
                        className={`p-4 transition-all ${
                          model.isAvailable && selectedModel === model.id
                            ? "border-accent bg-accent/5 cursor-pointer"
                            : model.isAvailable
                              ? "cursor-pointer hover:border-accent/50"
                              : "opacity-60 cursor-not-allowed"
                        }`}
                        onClick={() =>
                          model.isAvailable && handleSelectModel(model.id)
                        }
                      >
                        <div className="flex items-start justify-between">
                          <div className="flex-1">
                            <div className="flex items-center gap-2">
                              <span className="font-medium">{model.name}</span>
                              {!model.isAvailable && (
                                <Lock className="size-3 text-muted-foreground" />
                              )}
                              {model.isAvailable &&
                                selectedModel === model.id && (
                                  <span className="text-xs bg-accent/20 text-accent px-2 py-0.5 rounded-full flex items-center gap-1">
                                    <Check className="size-3" />
                                    Active
                                  </span>
                                )}
                            </div>
                            <p className="text-sm text-muted-foreground mt-1">
                              {model.description}
                            </p>
                          </div>
                          {model.isAvailable ? (
                            <div
                              className={`size-5 rounded-full border-2 flex items-center justify-center ${
                                selectedModel === model.id
                                  ? "border-accent bg-accent"
                                  : "border-muted-foreground"
                              }`}
                            >
                              {selectedModel === model.id && (
                                <Check className="size-3 text-white" />
                              )}
                            </div>
                          ) : (
                            <Button
                              variant="secondary"
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
                      </Card>
                    ))}
                  </div>
                </div>

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
                <Card key={provider.id} className="p-4">
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
                          <p className="text-xs text-red-600 flex items-center gap-1">
                            <X className="size-3" /> Invalid API key
                          </p>
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
                </Card>
              );
            })}
          </Tabs.Panel>

          {/* Subscription Tab */}
          <Tabs.Panel id="subscription" className="space-y-6">
            {/* Current Plan & Usage */}
            <Card className="p-4">
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
            </Card>

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
                        <Card
                          key={plan.id}
                          className={`p-4 transition-all ${
                            isCurrentPlan
                              ? "border-accent bg-accent/5"
                              : "hover:border-accent/50 cursor-pointer"
                          }`}
                        >
                          <div className="space-y-3">
                            <div className="flex items-center justify-between">
                              <span className="font-semibold">{plan.name}</span>
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
                        </Card>
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
                <Card className="p-6 text-center">
                  <Key className="size-8 mx-auto text-muted-foreground mb-2" />
                  <p className="text-muted-foreground text-sm">
                    No API keys yet
                  </p>
                  <p className="text-muted-foreground text-xs mt-1">
                    Create a key to use with the Rainy SDK
                  </p>
                </Card>
              ) : (
                <div className="space-y-2">
                  {coworkApiKeys.map((key) => (
                    <Card
                      key={key.id}
                      className="p-3 flex items-center justify-between"
                    >
                      <div>
                        <span className="font-medium">{key.name}</span>
                        <p className="text-xs text-muted-foreground">
                          Created {new Date(key.createdAt).toLocaleDateString()}
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
                    </Card>
                  ))}
                </div>
              )}

              {/* New Key Modal */}
              {showNewKeyModal && (
                <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
                  <Card className="p-6 w-96 max-w-[90vw]">
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
                  </Card>
                </div>
              )}
            </div>
          </Tabs.Panel>

          {/* Appearance Tab */}
          <Tabs.Panel id="appearance" className="space-y-4">
            <ThemeSelector />

            {/* Premium Animations Group */}
            {/* Premium Animations Group */}
            <div className="space-y-6 px-4">
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
            <Card className="p-4 space-y-4">
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
            </Card>
          </Tabs.Panel>
        </Tabs>
      </div>
    </div>
  );
}
