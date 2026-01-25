// Rainy Cowork - Settings Panel
// AI Provider configuration with API key management and Cowork subscription

import { useState } from "react";
import {
  Modal,
  Button,
  TextField,
  Input,
  Tabs,
  Switch,
  Label,
  Card,
  Separator,
} from "@heroui/react";
import {
  Settings,
  Key,
  Zap,
  Sparkles,
  Check,
  X,
  Eye,
  EyeOff,
  CreditCard,
  TrendingUp,
} from "lucide-react";
import { useAIProvider, useCoworkStatus } from "../../hooks";
import { AI_PROVIDERS, type ProviderType } from "../../types";

interface SettingsPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

export function SettingsPanel({ isOpen, onClose }: SettingsPanelProps) {
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

  const [activeTab, setActiveTab] = useState("providers");
  const [apiKeyInputs, setApiKeyInputs] = useState<Record<string, string>>({});
  const [showKeys, setShowKeys] = useState<Record<string, boolean>>({});
  const [validationStatus, setValidationStatus] = useState<
    Record<string, "idle" | "validating" | "valid" | "invalid">
  >({});
  const [savingStatus, setSavingStatus] = useState<Record<string, boolean>>({});

  // Provider ID mapping
  const getProviderId = (type: ProviderType) =>
    type === "rainyApi" ? "rainy_api" : "gemini";

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
      setValidationStatus((prev) => ({ ...prev, [provider]: "valid" }));
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
      // Refresh cowork status after saving key
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
    // Refresh cowork status after deleting key
    await refreshCowork();
  };

  const toggleShowKey = (provider: ProviderType) => {
    setShowKeys((prev) => ({ ...prev, [provider]: !prev[provider] }));
  };

  // Format reset date
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

  return (
    <Modal isOpen={isOpen} onOpenChange={(open) => !open && onClose()}>
      <Modal.Backdrop variant="blur">
        <Modal.Container>
          <Modal.Dialog className="max-w-lg">
            <Modal.CloseTrigger />
            <Modal.Header>
              <Modal.Heading className="flex items-center gap-2">
                <Settings className="size-5" />
                Settings
              </Modal.Heading>
            </Modal.Header>
            <Modal.Body className="space-y-4">
              <Tabs
                selectedKey={activeTab}
                onSelectionChange={(key) => setActiveTab(key as string)}
              >
                <Tabs.List>
                  <Tabs.Tab id="subscription">
                    <CreditCard className="size-4" />
                    Subscription
                  </Tabs.Tab>
                  <Tabs.Tab id="providers">
                    <Key className="size-4" />
                    API Keys
                  </Tabs.Tab>
                  <Tabs.Tab id="general">
                    <Zap className="size-4" />
                    General
                  </Tabs.Tab>
                </Tabs.List>

                {/* Subscription Tab */}
                <Tabs.Panel id="subscription" className="pt-4 space-y-4">
                  <Card className="p-4">
                    <div className="space-y-4">
                      {/* Plan Header */}
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <Sparkles className="size-5 text-primary" />
                          <span className="font-semibold text-lg">
                            {planName}
                          </span>
                          {hasPaidPlan && (
                            <span className="text-xs bg-primary/10 text-primary px-2 py-0.5 rounded-full">
                              Active
                            </span>
                          )}
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

                      <Separator />

                      {/* Usage Stats */}
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
                              {coworkStatus.usage.used} /{" "}
                              {coworkStatus.usage.limit} uses
                            </span>
                          </div>

                          {/* Progress Bar */}
                          <div className="w-full bg-muted rounded-full h-2">
                            <div
                              className={`h-2 rounded-full transition-all ${
                                usagePercent >= 90
                                  ? "bg-red-500"
                                  : usagePercent >= 70
                                    ? "bg-yellow-500"
                                    : "bg-primary"
                              }`}
                              style={{
                                width: `${Math.min(100, usagePercent)}%`,
                              }}
                            />
                          </div>

                          <div className="flex justify-between text-xs text-muted-foreground">
                            <span>{remainingUses} remaining</span>
                            <span>
                              Resets{" "}
                              {formatResetDate(coworkStatus.usage.resets_at)}
                            </span>
                          </div>

                          {/* Credit Usage (for paid plans) */}
                          {hasPaidPlan &&
                            coworkStatus.usage.credits_ceiling > 0 && (
                              <>
                                <Separator />
                                <div className="flex justify-between text-sm">
                                  <span className="text-muted-foreground">
                                    Credit Usage
                                  </span>
                                  <span>
                                    $
                                    {coworkStatus.usage.credits_used.toFixed(2)}{" "}
                                    / $
                                    {coworkStatus.usage.credits_ceiling.toFixed(
                                      2,
                                    )}
                                  </span>
                                </div>
                              </>
                            )}

                          {/* Upgrade Message */}
                          {coworkStatus.upgrade_message && (
                            <p className="text-sm text-muted-foreground bg-muted/50 p-2 rounded">
                              {coworkStatus.upgrade_message}
                            </p>
                          )}
                        </div>
                      )}

                      {coworkLoading && (
                        <div className="text-sm text-muted-foreground text-center py-4">
                          Loading...
                        </div>
                      )}

                      {/* Features */}
                      {!coworkLoading && coworkStatus && (
                        <>
                          <Separator />
                          <div className="space-y-2">
                            <span className="text-sm font-medium">
                              Features
                            </span>
                            <div className="grid grid-cols-2 gap-2 text-sm">
                              <div className="flex items-center gap-2">
                                {coworkStatus.features.web_research ? (
                                  <Check className="size-4 text-green-500" />
                                ) : (
                                  <X className="size-4 text-muted-foreground" />
                                )}
                                <span
                                  className={
                                    coworkStatus.features.web_research
                                      ? ""
                                      : "text-muted-foreground"
                                  }
                                >
                                  Web Research
                                </span>
                              </div>
                              <div className="flex items-center gap-2">
                                {coworkStatus.features.document_export ? (
                                  <Check className="size-4 text-green-500" />
                                ) : (
                                  <X className="size-4 text-muted-foreground" />
                                )}
                                <span
                                  className={
                                    coworkStatus.features.document_export
                                      ? ""
                                      : "text-muted-foreground"
                                  }
                                >
                                  Doc Export
                                </span>
                              </div>
                              <div className="flex items-center gap-2">
                                {coworkStatus.features.image_analysis ? (
                                  <Check className="size-4 text-green-500" />
                                ) : (
                                  <X className="size-4 text-muted-foreground" />
                                )}
                                <span
                                  className={
                                    coworkStatus.features.image_analysis
                                      ? ""
                                      : "text-muted-foreground"
                                  }
                                >
                                  Image Analysis
                                </span>
                              </div>
                              <div className="flex items-center gap-2">
                                {coworkStatus.features.priority_support ? (
                                  <Check className="size-4 text-green-500" />
                                ) : (
                                  <X className="size-4 text-muted-foreground" />
                                )}
                                <span
                                  className={
                                    coworkStatus.features.priority_support
                                      ? ""
                                      : "text-muted-foreground"
                                  }
                                >
                                  Priority Support
                                </span>
                              </div>
                            </div>
                          </div>
                        </>
                      )}
                    </div>
                  </Card>
                </Tabs.Panel>

                {/* Providers Tab */}
                <Tabs.Panel id="providers" className="pt-4 space-y-4">
                  {AI_PROVIDERS.map((provider) => {
                    const providerId = getProviderId(provider.id);
                    const hasKey = hasApiKey(providerId);
                    const status = validationStatus[provider.id] || "idle";
                    const isSaving = savingStatus[provider.id];
                    const showKey = showKeys[provider.id];

                    return (
                      <Card key={provider.id} className="p-4">
                        <div className="space-y-3">
                          <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                              <Sparkles className="size-4 text-primary" />
                              <span className="font-medium">
                                {provider.name}
                              </span>
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
                                    !apiKeyInputs[provider.id]?.trim() ||
                                    isSaving
                                  }
                                >
                                  {isSaving ? "Saving..." : "Save to Keychain"}
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

                {/* General Tab */}
                <Tabs.Panel id="general" className="pt-4 space-y-4">
                  <div className="space-y-3">
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
                        <Label className="font-medium">
                          Auto-execute tasks
                        </Label>
                        <p className="text-sm text-muted-foreground">
                          Start tasks immediately after creation
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
            </Modal.Body>
            <Modal.Footer>
              <Button variant="secondary" onPress={onClose}>
                Close
              </Button>
            </Modal.Footer>
          </Modal.Dialog>
        </Modal.Container>
      </Modal.Backdrop>
    </Modal>
  );
}
