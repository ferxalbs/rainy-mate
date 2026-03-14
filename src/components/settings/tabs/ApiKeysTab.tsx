import { useState } from "react";
import {
  Check,
  X,
  EyeOff,
  Eye,
  Copy,
  Trash2,
  RefreshCw,
  Key as KeyIcon,
} from "lucide-react";
import { useAIProvider } from "../../../hooks";
import { AI_PROVIDERS, type ProviderType } from "../../../types";
import { Input, Button, Card } from "@heroui/react";

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
    <div className="space-y-6 animate-in fade-in duration-500">
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
          <Card
            key={provider.id}
            className="group overflow-hidden border border-border/10 bg-muted/20 backdrop-blur-xl transition-all hover:bg-muted/30 hover:border-border/20 shadow-none rounded-2xl"
          >
            <div className="p-5 space-y-4 overflow-visible">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="p-2 rounded-lg bg-primary/10">
                    <KeyIcon className="size-4 text-primary" />
                  </div>
                  <div>
                    <h4 className="font-semibold text-sm tracking-tight">{provider.name}</h4>
                    <p className="text-[10px] text-muted-foreground uppercase tracking-widest font-medium opacity-60">Credential Locked</p>
                  </div>
                </div>
                {hasKey && !isReplacing && (
                  <div className="flex items-center gap-1.5 px-2 py-1 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-emerald-500 text-[10px] font-bold uppercase tracking-wider">
                    <Check className="size-3" />
                    Secure Vault
                  </div>
                )}
              </div>

              <p className="text-xs text-muted-foreground leading-relaxed italic">
                {provider.description}
              </p>

              <div className="h-px bg-border/5 w-full opacity-5" />

              {showInput ? (
                <div className="space-y-4">
                  <div className="space-y-2">
                    <label className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground/80 ml-1 pb-1 block">
                      {isReplacing ? "New Secret Key" : "Secret Key"}
                    </label>
                    <div className="relative">
                      <Input
                        type={showKey ? "text" : "password"}
                        placeholder={isReplacing ? "sk-..." : "Paste your key here..."}
                        value={apiKeyInputs[provider.id] || ""}
                        onChange={(e) => handleApiKeyChange(provider.id, e.target.value)}
                        className="h-11 bg-background/40 border-border/10 pr-10 focus:ring-primary/20 transition-all rounded-xl w-full"
                      />
                      <button
                        type="button"
                        onClick={() => toggleShowKey(provider.id)}
                        className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground/50 hover:text-foreground transition-colors outline-none focus:outline-none"
                      >
                        {showKey ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
                      </button>
                    </div>
                  </div>

                  <div className="flex items-center justify-between gap-3">
                    <div className="flex items-center gap-2">
                      {status === "validating" && (
                        <RefreshCw className="size-3.5 animate-spin text-primary" />
                      )}
                      {status === "valid" && (
                        <div className="text-[11px] text-emerald-500 font-bold uppercase flex items-center gap-1.5 animate-in zoom-in-95">
                          <Check className="size-3.5" />
                          Validated
                        </div>
                      )}
                      {status === "invalid" && (
                        <div className="text-[11px] text-danger font-bold uppercase flex items-center gap-1.5 animate-in zoom-in-95">
                          <X className="size-3.5" />
                          Denied
                        </div>
                      )}
                    </div>

                    <div className="flex items-center gap-2">
                      {isReplacing && (
                        <Button
                          variant="ghost"
                          size="sm"
                          onPress={() => setReplacingKeys((prev) => ({ ...prev, [providerId]: false }))}
                          className="h-8 rounded-lg text-xs"
                        >
                          Abort
                        </Button>
                      )}
                      <Button
                        variant="secondary"
                        size="sm"
                        onPress={() => handleValidateKey(provider.id)}
                        isDisabled={!apiKeyInputs[provider.id]?.trim() || status === "validating"}
                        className="h-8 rounded-lg text-xs px-4"
                      >
                        {status === "validating" ? "Checking..." : "Verify"}
                      </Button>
                      <Button
                        variant="primary"
                        size="sm"
                        onPress={async () => {
                          await handleSaveKey(provider.id);
                          setReplacingKeys((prev) => ({ ...prev, [providerId]: false }));
                        }}
                        isDisabled={!apiKeyInputs[provider.id]?.trim() || saving}
                        className="h-8 rounded-lg text-xs px-4 font-bold"
                      >
                        {saving ? "Storing..." : "Lock in Vault"}
                      </Button>
                    </div>
                  </div>

                  {validationError[provider.id] && (
                    <div className="p-3 rounded-lg bg-danger/10 border border-danger/20 text-[11px] text-danger italic">
                      Error: {validationError[provider.id]}
                    </div>
                  )}
                </div>
              ) : (
                <div className="flex flex-col gap-3">
                  <div className="flex items-center gap-2 flex-wrap">
                    <Button
                      variant="outline"
                      size="sm"
                      onPress={() => handleViewKey(provider.id)}
                      className="h-8 border-border/10 bg-muted/20 hover:bg-muted/40 text-xs rounded-lg"
                    >
                      {visibleKey ? <EyeOff className="size-3.5 mr-1" /> : <Eye className="size-3.5 mr-1" />}
                      {visibleKey ? "Hide Secret" : "Reveal Secret"}
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onPress={() => handleReplaceKey(provider.id)}
                      className="h-8 border-border/10 bg-muted/20 hover:bg-muted/40 text-xs rounded-lg"
                    >
                      <RefreshCw className="size-3.5 mr-1" />
                      Rotate Key
                    </Button>
                    <Button
                      variant="danger"
                      size="sm"
                      onPress={() => handleDeleteKey(provider.id)}
                      className="h-8 text-xs rounded-lg px-4 opacity-70 hover:opacity-100 transition-opacity"
                    >
                      <Trash2 className="size-3.5 mr-1" />
                      Purge
                    </Button>
                  </div>
                  {visibleKey && (
                    <div className="mt-2 p-3 bg-background/50 backdrop-blur-md rounded-xl border border-border/10 text-[11px] font-mono break-all relative group/key animate-in fade-in zoom-in-95 duration-300">
                      <span className="text-foreground/80">{visibleKey}</span>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="absolute -top-2 -right-2 h-7 w-7 min-w-7 rounded-full bg-background border border-border/10 opacity-0 group-hover/key:opacity-100 transition-all shadow-xl p-0 flex items-center justify-center"
                        onPress={() => {
                          navigator.clipboard.writeText(visibleKey);
                        }}
                      >
                         <Copy className="size-3 text-primary" />
                      </Button>
                    </div>
                  )}
                </div>
              )}
            </div>
          </Card>
        );
      })}
    </div>
  );
}
