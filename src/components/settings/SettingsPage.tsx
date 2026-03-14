// Rainy Cowork - Settings Page
// Full-page settings with AI model selection, API keys, and preferences

import { Bot, Key, User, ChevronLeft, Palette, Shield } from "lucide-react";

import { ModelsTab } from "./tabs/ModelsTab";
import { ApiKeysTab } from "./tabs/ApiKeysTab";
import { AppearanceTab } from "./tabs/AppearanceTab";
import { PermissionsTab } from "./tabs/PermissionsTab";
import { ProfileTab } from "./tabs/ProfileTab";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";

interface SettingsPageProps {
  initialTab?: string;
  onBack?: () => void;
}

export function SettingsPage({
  initialTab = "models",
  onBack,
}: SettingsPageProps) {
  return (
    <div className="h-full flex flex-col relative z-20 bg-background/30 backdrop-blur-md">
      {/* Header - macOS Style Premium */}
      <div className="flex items-center gap-2 p-6 shrink-0 border-b border-border/10">
        {onBack && (
          <Button
            variant="ghost"
            size="icon"
            className="rounded-full h-8 w-8 min-w-[32px] hover:bg-foreground/5 text-foreground/80 -ml-2"
            onClick={onBack}
          >
            <ChevronLeft className="size-5" />
          </Button>
        )}
        <h1 className="text-xl font-bold tracking-tight pl-1 bg-gradient-to-r from-foreground to-foreground/60 bg-clip-text text-transparent">
          Settings
        </h1>
      </div>

      {/* Settings Framework - Sidebar Layout */}
      <div className="flex-1 flex overflow-hidden">
        <Tabs
          defaultValue={initialTab}
          orientation="vertical"
          className="flex-1 flex"
        >
          {/* Sidebar */}
          <aside className="w-64 border-r border-border/5 bg-muted/20 hidden md:block">
            <ScrollArea className="h-full">
              <div className="p-4 space-y-4">
                <TabsList variant="line" className="flex flex-col h-auto bg-transparent w-full gap-1">
                  <TabsTrigger
                    value="models"
                    className="w-full justify-start gap-3 px-4 py-2.5 rounded-xl data-active:bg-background/80 data-active:shadow-sm"
                  >
                    <Bot className="size-4.5" />
                    <span>AI Models</span>
                  </TabsTrigger>
                  <TabsTrigger
                    value="keys"
                    className="w-full justify-start gap-3 px-4 py-2.5 rounded-xl data-active:bg-background/80 data-active:shadow-sm"
                  >
                    <Key className="size-4.5" />
                    <span>API Keys</span>
                  </TabsTrigger>
                  <TabsTrigger
                    value="appearance"
                    className="w-full justify-start gap-3 px-4 py-2.5 rounded-xl data-active:bg-background/80 data-active:shadow-sm"
                  >
                    <Palette className="size-4.5" />
                    <span>Appearance</span>
                  </TabsTrigger>
                  <TabsTrigger
                    value="permissions"
                    className="w-full justify-start gap-3 px-4 py-2.5 rounded-xl data-active:bg-background/80 data-active:shadow-sm"
                  >
                    <Shield className="size-4.5" />
                    <span>Permissions</span>
                  </TabsTrigger>
                  <TabsTrigger
                    value="profile"
                    className="w-full justify-start gap-3 px-4 py-2.5 rounded-xl data-active:bg-background/80 data-active:shadow-sm"
                  >
                    <User className="size-4.5" />
                    <span>Profile</span>
                  </TabsTrigger>
                </TabsList>
              </div>
            </ScrollArea>
          </aside>

          {/* Mobile Navigation (Condensed List) */}
          <div className="md:hidden w-full border-b border-border/10">
             <TabsList className="w-full flex justify-around p-2 bg-transparent h-auto overflow-x-auto">
                <TabsTrigger value="models" className="flex-col gap-1 text-[10px] h-auto py-2"><Bot className="size-4" />Models</TabsTrigger>
                <TabsTrigger value="keys" className="flex-col gap-1 text-[10px] h-auto py-2"><Key className="size-4" />Keys</TabsTrigger>
                <TabsTrigger value="appearance" className="flex-col gap-1 text-[10px] h-auto py-2"><Palette className="size-4" />Theme</TabsTrigger>
                <TabsTrigger value="permissions" className="flex-col gap-1 text-[10px] h-auto py-2"><Shield className="size-4" />Shield</TabsTrigger>
                <TabsTrigger value="profile" className="flex-col gap-1 text-[10px] h-auto py-2"><User className="size-4" />User</TabsTrigger>
             </TabsList>
          </div>

          {/* Content Area */}
          <main className="flex-1 bg-background/5 overflow-hidden">
            <ScrollArea className="h-full">
              <div className="max-w-3xl mx-auto p-6 md:p-10 space-y-12 pb-32">
                <TabsContent value="models">
                  <header className="mb-8 space-y-1">
                    <h2 className="text-2xl font-bold tracking-tight">AI Models</h2>
                    <p className="text-muted-foreground text-sm">Configure preferred models and memory providers.</p>
                  </header>
                  <ModelsTab />
                </TabsContent>

                <TabsContent value="keys">
                  <header className="mb-8 space-y-1">
                    <h2 className="text-2xl font-bold tracking-tight">API Keys</h2>
                    <p className="text-muted-foreground text-sm">Manage your keys for different AI providers.</p>
                  </header>
                  <ApiKeysTab />
                </TabsContent>

                <TabsContent value="appearance">
                  <header className="mb-8 space-y-1">
                    <h2 className="text-2xl font-bold tracking-tight">Appearance</h2>
                    <p className="text-muted-foreground text-sm">Customize themes and premium animations.</p>
                  </header>
                  <AppearanceTab />
                </TabsContent>

                <TabsContent value="permissions">
                  <header className="mb-8 space-y-1">
                    <h2 className="text-2xl font-bold tracking-tight">Permissions</h2>
                    <p className="text-muted-foreground text-sm">Global app behavior and security settings.</p>
                  </header>
                  <PermissionsTab />
                </TabsContent>

                <TabsContent value="profile">
                  <header className="mb-8 space-y-1">
                    <h2 className="text-2xl font-bold tracking-tight">Profile</h2>
                    <p className="text-muted-foreground text-sm">Manage your personal identity across the ecosystem.</p>
                  </header>
                  <ProfileTab />
                </TabsContent>
              </div>
            </ScrollArea>
          </main>
        </Tabs>
      </div>
    </div>
  );
}
