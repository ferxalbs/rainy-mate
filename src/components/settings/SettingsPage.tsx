// Rainy Cowork - Settings Page
// Full-page settings with AI model selection, API keys, and preferences

import { useState, useEffect } from "react";
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
  User,
  Check,
  Sparkles,
  Eye,
  EyeOff,
  X,
  ChevronLeft,
  Copy,
  Trash2,
  ExternalLink,
  Zap,
  Palette,
  Shield,
  Building2,
  Briefcase,
  Mail,
} from "lucide-react";
import * as tauri from "../../services/tauri";
import { useAIProvider } from "../../hooks";
import { AI_PROVIDERS, type ProviderType } from "../../types";
import { ThemeSelector } from "./ThemeSelector";
import { ThemeContext } from "../../providers/ThemeProvider";
import { useContext } from "react";
import { useUserProfile } from "../../hooks/useUserProfile";
import type { UserProfile } from "../../services/tauri";

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
  const [isLoading, setIsLoading] = useState(true);
  const [isSavingProfile, setIsSavingProfile] = useState(false);
  const {
    profile,
    isLoading: isLoadingProfile,
    saveProfile,
  } = useUserProfile();
  const [profileForm, setProfileForm] = useState<UserProfile>({
    displayName: "",
    email: "",
    organization: "",
    role: "",
  });

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

  // Load available models
  useEffect(() => {
    async function loadData() {
      try {
        const [rainyModels, geminiModelsList] = await Promise.all([
          tauri.getProviderModels("rainy_api").catch(() => []),
          tauri.getProviderModels("gemini").catch(() => []),
        ]);

        setRainyApiModels(rainyModels || []);
        setGeminiModels(geminiModelsList || []);
      } catch (error) {
        console.error("Failed to load settings:", error);
      } finally {
        setIsLoading(false);
      }
    }
    loadData();
  }, []);

  useEffect(() => {
    setProfileForm(profile);
  }, [profile]);

  const updateProfileField = (key: keyof UserProfile, value: string) => {
    setProfileForm((prev) => ({
      ...prev,
      [key]: value,
    }));
  };

  const handleSaveProfile = async () => {
    setIsSavingProfile(true);
    try {
      await saveProfile(profileForm);
    } finally {
      setIsSavingProfile(false);
    }
  };

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

    // Toggle visibility: If already visible, hide it
    if (visibleKeys[providerId]) {
      setVisibleKeys((prev) => {
        const next = { ...prev };
        delete next[providerId];
        return next;
      });
      return;
    }

    // Otherwise fetch and show
    const key = await getApiKey(providerId);
    if (key) {
      setVisibleKeys((prev) => ({ ...prev, [providerId]: key }));
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

  // Helper to render model card (display-only, no selection)
  const ModelCard = ({
    name,
    description,
  }: {
    name: string;
    description: string;
  }) => (
    <div className="p-4 rounded-xl border border-transparent bg-transparent">
      <div className="flex-1">
        <span className="font-medium">{name}</span>
        <p className="text-sm text-muted-foreground mt-1">{description}</p>
      </div>
    </div>
  );

  return (
    <div className="h-full flex flex-col relative z-20">
      {/* Header */}
      {/* Header - macOS Style Premium */}
      <div className="flex items-center gap-2 p-4 shrink-0">
        {onBack && (
          <Button
            isIconOnly
            variant="ghost"
            size="sm"
            className="rounded-full h-8 w-8 min-w-[32px] hover:bg-foreground/5 text-foreground/80 -ml-1"
            onPress={onBack}
          >
            <ChevronLeft className="size-5" />
          </Button>
        )}
        <h1 className="text-lg font-semibold tracking-tight pl-1">Settings</h1>
      </div>

      {/* Tabs Content - Scrollable */}
      <div className="flex-1 overflow-y-auto min-h-0 pb-24">
        <div className="px-4 pt-4">
          <Tabs
            selectedKey={activeTab}
            onSelectionChange={(key) => setActiveTab(key as string)}
            className="w-full"
          >
            <Tabs.List className="mb-4 bg-muted/50 p-1 rounded-xl gap-1 border border-border/30">
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
              <Tabs.Tab
                id="profile"
                className="px-3 py-1.5 rounded-lg text-sm font-medium text-muted-foreground hover:text-foreground data-[selected=true]:text-foreground data-[selected=true]:bg-background data-[selected=true]:shadow-sm transition-all focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ring-offset-background"
              >
                <div className="flex items-center gap-2">
                  <User className="size-4" />
                  Profile
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
                            name={model}
                            description="Billed per usage (1:1 Token)"
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
                            name={model}
                            description="Uses your own Gemini API Key"
                          />
                        ))}
                      </div>
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
                        <div className="space-y-4 pt-1">
                          <div className="relative group">
                            <TextField
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

            {/* Profile Tab */}
            {/* Profile Tab */}
            <Tabs.Panel id="profile" className="space-y-6 pb-32">
              {isLoadingProfile ? (
                <div className="flex items-center justify-center p-12 text-muted-foreground animate-pulse">
                  <Spinner size="lg" color="current" />
                </div>
              ) : (
                <div className="p-6 rounded-2xl border bg-card/40 backdrop-blur-md border-border/40 shadow-sm space-y-6 animate-appear">
                  {/* Header */}
                  <div className="flex items-start justify-between border-b border-border/30 pb-4">
                    <div className="space-y-1">
                      <h3 className="text-base font-semibold text-foreground flex items-center gap-2">
                        <User className="size-4 text-primary" />
                        User Identity
                      </h3>
                      <p className="text-sm text-muted-foreground">
                        This profile is used by the desktop app and cloud bridge
                        for personalization.
                      </p>
                    </div>
                    <div className="hidden sm:block">
                      <div className="h-8 w-8 rounded-full bg-primary/10 flex items-center justify-center">
                        <User className="size-4 text-primary" />
                      </div>
                    </div>
                  </div>

                  {/* Form Grid */}
                  <div className="grid gap-5">
                    {/* Display Name - Full Width */}
                    <div className="space-y-2 group">
                      <Label className="text-xs font-medium text-muted-foreground ml-1 group-focus-within:text-primary transition-colors">
                        Display Name
                      </Label>
                      <TextField
                        name="profile-display-name"
                        className="w-full"
                        onChange={(value) =>
                          updateProfileField("displayName", value)
                        }
                      >
                        <div className="relative">
                          <User className="absolute left-3.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/50 z-10 pointer-events-none" />
                          <Input
                            className="w-full h-11 rounded-xl border border-border/40 bg-muted/20 pl-10 pr-4 text-sm outline-none transition-all placeholder:text-muted-foreground/30 focus:border-primary/50 focus:bg-background focus:ring-4 focus:ring-primary/10 hover:bg-muted/30"
                            placeholder="Create a display name..."
                            value={profileForm.displayName}
                          />
                        </div>
                      </TextField>
                    </div>

                    {/* Email - Full Width */}
                    <div className="space-y-2 group">
                      <Label className="text-xs font-medium text-muted-foreground ml-1 group-focus-within:text-primary transition-colors">
                        Email Address
                      </Label>
                      <TextField
                        name="profile-email"
                        className="w-full"
                        onChange={(value) => updateProfileField("email", value)}
                      >
                        <div className="relative">
                          <Mail className="absolute left-3.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/50 z-10 pointer-events-none" />
                          <Input
                            className="w-full h-11 rounded-xl border border-border/40 bg-muted/20 pl-10 pr-4 text-sm outline-none transition-all placeholder:text-muted-foreground/30 focus:border-primary/50 focus:bg-background focus:ring-4 focus:ring-primary/10 hover:bg-muted/30"
                            placeholder="name@company.com"
                            type="email"
                            value={profileForm.email}
                          />
                        </div>
                      </TextField>
                    </div>

                    {/* Org & Role - 2 Cols on Tablet+ */}
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-5">
                      <div className="space-y-2 group">
                        <Label className="text-xs font-medium text-muted-foreground ml-1 group-focus-within:text-primary transition-colors">
                          Organization
                        </Label>
                        <TextField
                          name="profile-organization"
                          className="w-full"
                          onChange={(value) =>
                            updateProfileField("organization", value)
                          }
                        >
                          <div className="relative">
                            <Building2 className="absolute left-3.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/50 z-10 pointer-events-none" />
                            <Input
                              className="w-full h-11 rounded-xl border border-border/40 bg-muted/20 pl-10 pr-4 text-sm outline-none transition-all placeholder:text-muted-foreground/30 focus:border-primary/50 focus:bg-background focus:ring-4 focus:ring-primary/10 hover:bg-muted/30"
                              placeholder="Company Name"
                              value={profileForm.organization}
                            />
                          </div>
                        </TextField>
                      </div>

                      <div className="space-y-2 group">
                        <Label className="text-xs font-medium text-muted-foreground ml-1 group-focus-within:text-primary transition-colors">
                          Role
                        </Label>
                        <TextField
                          name="profile-role"
                          className="w-full"
                          onChange={(value) =>
                            updateProfileField("role", value)
                          }
                        >
                          <div className="relative">
                            <Briefcase className="absolute left-3.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/50 z-10 pointer-events-none" />
                            <Input
                              className="w-full h-11 rounded-xl border border-border/40 bg-muted/20 pl-10 pr-4 text-sm outline-none transition-all placeholder:text-muted-foreground/30 focus:border-primary/50 focus:bg-background focus:ring-4 focus:ring-primary/10 hover:bg-muted/30"
                              placeholder="e.g. Developer"
                              value={profileForm.role}
                            />
                          </div>
                        </TextField>
                      </div>
                    </div>
                  </div>

                  {/* Actions Footer */}
                  <div className="flex flex-col sm:flex-row items-center justify-between gap-4 pt-6 border-t border-border/30">
                    <p className="text-xs text-muted-foreground flex items-center gap-1.5 opacity-70">
                      <div className="size-1.5 rounded-full bg-emerald-500/50" />
                      Changes are encrypted locally
                    </p>
                    <Button
                      variant="primary"
                      className="w-full sm:w-auto min-w-[140px] rounded-xl font-medium shadow-lg shadow-primary/20 hover:shadow-primary/30 active:scale-95 transition-all h-10"
                      onPress={handleSaveProfile}
                      isDisabled={isSavingProfile}
                    >
                      {isSavingProfile ? (
                        <>
                          <Spinner size="sm" color="current" className="mr-2" />
                          Saving...
                        </>
                      ) : (
                        <>
                          Save Changes
                          <Check className="size-4 ml-2" />
                        </>
                      )}
                    </Button>
                  </div>
                </div>
              )}
            </Tabs.Panel>
          </Tabs>
        </div>
      </div>
    </div>
  );
}
