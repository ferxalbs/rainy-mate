// Rainy Cowork - Settings Page
// Full-page settings with AI model selection, API keys, and preferences

import { useState } from "react";
import { Button, Tabs } from "@heroui/react";
import { Bot, Key, User, ChevronLeft, Palette, Shield } from "lucide-react";

import { ModelsTab } from "./tabs/ModelsTab";
import { ApiKeysTab } from "./tabs/ApiKeysTab";
import { AppearanceTab } from "./tabs/AppearanceTab";
import { PermissionsTab } from "./tabs/PermissionsTab";
import { ProfileTab } from "./tabs/ProfileTab";

interface SettingsPageProps {
  initialTab?: string;
  onBack?: () => void;
}

export function SettingsPage({
  initialTab = "models",
  onBack,
}: SettingsPageProps) {
  const [activeTab, setActiveTab] = useState(initialTab);

  return (
    <div className="h-full flex flex-col relative z-20">
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
              <ModelsTab />
            </Tabs.Panel>

            {/* API Keys Tab */}
            <Tabs.Panel id="keys" className="space-y-4">
              <ApiKeysTab />
            </Tabs.Panel>

            {/* Appearance Tab */}
            <Tabs.Panel id="appearance" className="space-y-4">
              <AppearanceTab />
            </Tabs.Panel>

            {/* Permissions Tab */}
            <Tabs.Panel id="permissions" className="space-y-4">
              <PermissionsTab />
            </Tabs.Panel>

            {/* Profile Tab */}
            <Tabs.Panel id="profile" className="space-y-6 pb-32">
              <ProfileTab />
            </Tabs.Panel>
          </Tabs>
        </div>
      </div>
    </div>
  );
}
