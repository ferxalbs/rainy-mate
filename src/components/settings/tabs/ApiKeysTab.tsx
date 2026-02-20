import { useState } from "react";
import { TextField, Input, Button, Spinner } from "@heroui/react";
import {
  Sparkles,
  Check,
  X,
  EyeOff,
  Eye,
  Copy,
  ExternalLink,
  Trash2,
} from "lucide-react";
import { useAIProvider } from "../../../hooks";
import { AI_PROVIDERS, type ProviderType } from "../../../types";

export function ApiKeysTab() {
  const { hasApiKey, validateApiKey, storeApiKey, getApiKey, deleteApiKey } =
    useAIProvider();

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
  const [validationError, setValidationError] = useState<
    Record<string, string>
  >({});

  const getProviderId = (type: ProviderType) => {
    if (type === "rainyapi") return "rainy_api";
    return "gemini";
  };

  const handleApiKeyChange = (provider: ProviderType, value: string) => {
    setApiKeyInputs((prev) => ({ ...prev, [provider]: value }));
    setValidationStatus((prev) => ({ ...prev, [provider]: "idle" }));
  };

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

    if (visibleKeys[providerId]) {
      setVisibleKeys((prev) => {
        const next = { ...prev };
        delete next[providerId];
        return next;
      });
      return;
    }

    const key = await getApiKey(providerId);
    if (key) {
      setVisibleKeys((prev) => ({ ...prev, [providerId]: key }));
    }
  };

  const handleReplaceKey = (provider: ProviderType) => {
    const providerId = getProviderId(provider);
    setReplacingKeys((prev) => ({ ...prev, [providerId]: true }));
    setApiKeyInputs((prev) => ({ ...prev, [providerId]: "" }));
    setVisibleKeys((prev) => {
      const next = { ...prev };
      delete next[providerId];
      return next;
    });
  };

  return (
    <div className="space-y-4">
      {AI_PROVIDERS.map((provider) => {
        const providerId = getProviderId(provider.id);
        const hasKey = hasApiKey(providerId);
        const status = validationStatus[provider.id] || "idle";
        const saving = savingStatus[provider.id];
        const showKey = showKeys[provider.id];
        const visibleKey = visibleKeys[providerId];
        const isReplacing = replacingKeys[providerId];

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
                <div className="space-y-4 pt-1">
                  <div className="relative group">
                    <TextField
                      aria-label={`${provider.name} API Key`}
                      className="w-full"
                      name={`api-key-${provider.id}`}
                      type={showKey ? "text" : "password"}
                      onChange={(value) =>
                        handleApiKeyChange(provider.id, value)
                      }
                    >
                      <Input
                        className="w-full rounded-xl border border-border/40 bg-muted/30 px-4 py-2.5 text-sm outline-none transition-all placeholder:text-muted-foreground/40 focus:border-primary/50 focus:bg-muted/40 focus:ring-2 focus:ring-primary/10 pr-10"
                        placeholder={
                          isReplacing
                            ? "Enter new API key..."
                            : "Enter API key to enable..."
                        }
                        value={apiKeyInputs[provider.id] || ""}
                      />
                    </TextField>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="absolute right-2 top-1/2 -translate-y-1/2 h-7 w-7 min-w-0 p-0 text-muted-foreground hover:text-foreground z-10"
                      onPress={() => toggleShowKey(provider.id)}
                    >
                      {showKey ? (
                        <EyeOff className="size-4" />
                      ) : (
                        <Eye className="size-4" />
                      )}
                    </Button>
                  </div>

                  <div className="flex items-center justify-between gap-3">
                    <div className="flex items-center gap-2">
                      {status === "valid" && (
                        <span className="text-xs text-green-500 font-medium flex items-center gap-1.5 animate-appear">
                          <Check className="size-3.5" />
                          Key Valid
                        </span>
                      )}
                      {status === "invalid" && (
                        <span className="text-xs text-red-500 font-medium flex items-center gap-1.5 animate-appear">
                          <X className="size-3.5" />
                          Invalid Key
                        </span>
                      )}
                    </div>

                    <div className="flex items-center gap-2">
                      {isReplacing && (
                        <Button
                          variant="ghost"
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
                      <Button
                        variant="secondary"
                        size="sm"
                        onPress={() => handleValidateKey(provider.id)}
                        isDisabled={
                          !apiKeyInputs[provider.id]?.trim() ||
                          status === "validating"
                        }
                      >
                        {status === "validating" ? (
                          <Spinner size="sm" color="current" />
                        ) : (
                          "Validate"
                        )}
                      </Button>
                      <Button
                        variant="primary"
                        size="sm"
                        className="font-medium shadow-lg shadow-primary/10"
                        onPress={async () => {
                          await handleSaveKey(provider.id);
                          setReplacingKeys((prev) => ({
                            ...prev,
                            [providerId]: false,
                          }));
                        }}
                        isDisabled={
                          !apiKeyInputs[provider.id]?.trim() || saving
                        }
                      >
                        {saving ? "Saving..." : "Save Key"}
                      </Button>
                    </div>
                  </div>

                  {validationError[provider.id] && (
                    <p className="text-xs text-red-500/80 pl-1">
                      {validationError[provider.id]}
                    </p>
                  )}
                </div>
              ) : (
                <div className="flex flex-col gap-2">
                  <div className="flex items-center gap-2 flex-wrap">
                    <Button
                      variant="ghost"
                      size="sm"
                      className="bg-muted/30 hover:bg-muted/50 text-foreground border border-border/20"
                      onPress={() => handleViewKey(provider.id)}
                    >
                      {visibleKey ? (
                        <>
                          <EyeOff className="size-4 mr-1" />
                          Hide
                        </>
                      ) : (
                        <>
                          <Eye className="size-4 mr-1" />
                          View
                        </>
                      )}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="bg-muted/30 hover:bg-muted/50 text-foreground border border-border/20"
                      onPress={() => handleReplaceKey(provider.id)}
                    >
                      <ExternalLink className="size-4 mr-1" />
                      Replace
                    </Button>
                    <Button
                      variant="danger-soft"
                      size="sm"
                      onPress={() => handleDeleteKey(provider.id)}
                    >
                      <Trash2 className="size-4 mr-1" />
                      Remove
                    </Button>
                  </div>
                  {visibleKey && (
                    <div className="mt-2 p-3 bg-muted rounded-lg border border-border/50 text-xs font-mono break-all relative group animate-appear">
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
                </div>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
