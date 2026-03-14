import { useState } from "react";
import { Button } from "@heroui/react";
import { Bot, Key, User, ArrowLeft, Palette, Shield } from "lucide-react";
import { useTheme } from "../../hooks/useTheme";

import { ModelsTab } from "./tabs/ModelsTab";
import { ApiKeysTab } from "./tabs/ApiKeysTab";
import { AppearanceTab } from "./tabs/AppearanceTab";
import { PermissionsTab } from "./tabs/PermissionsTab";
import { ProfileTab } from "./tabs/ProfileTab";

interface SettingsPageProps {
  initialTab?: string;
  onBack?: () => void;
}

const NavItem = ({
  icon: Icon,
  label,
  description,
  isActive,
  onPress,
}: {
  icon: any;
  label: string;
  description: string;
  isActive: boolean;
  onPress: () => void;
}) => {
  return (
    <button
      onClick={onPress}
      className={`w-full text-left px-4 py-3 rounded-2xl transition-all duration-300 group relative overflow-hidden flex-shrink-0 ${
        isActive
          ? "bg-primary text-primary-foreground shadow-md shadow-primary/10"
          : "hover:bg-foreground/5 text-muted-foreground hover:text-foreground"
      }`}
    >
      <div className="flex items-center gap-3 relative z-10">
        <div
          className={`p-1.5 rounded-full ${
            isActive ? "bg-black/10" : "bg-white/5 group-hover:bg-white/10"
          }`}
        >
          <Icon className="size-4" />
        </div>
        <div>
          <span
            className={`block text-sm font-bold ${isActive ? "text-primary-foreground" : "text-foreground"}`}
          >
            {label}
          </span>
          {description && (
            <span
              className={`text-[10px] uppercase tracking-wider ${isActive ? "text-primary-foreground/70" : "text-muted-foreground"}`}
            >
              {description}
            </span>
          )}
        </div>
      </div>
    </button>
  );
};

export function SettingsPage({
  initialTab = "models",
  onBack,
}: SettingsPageProps) {
  const [activeTab, setActiveTab] = useState<string>(initialTab);
  const { mode } = useTheme();
  const isDark = mode === "dark";

  return (
    <div className="h-full w-full bg-background p-3 flex gap-3 overflow-hidden font-sans selection:bg-primary selection:text-primary-foreground relative z-20">
      <div
        className="absolute inset-0 w-full h-full z-0 block md:hidden"
        data-tauri-drag-region
      />

      {/* Sidebar Navigation */}
      <aside
        className={`hidden md:flex flex-col w-[260px] shrink-0 rounded-[1.5rem] border border-border/40 shadow-xl overflow-hidden relative z-10 ${isDark ? "bg-card/20" : "bg-card/60"} backdrop-blur-2xl`}
      >
        <div className="p-6 pb-2" data-tauri-drag-region>
          {onBack && (
            <button
              onClick={onBack}
              className="flex items-center gap-2 text-muted-foreground hover:text-primary transition-colors mb-4 group relative z-50 window-no-drag"
            >
              <ArrowLeft className="size-3 group-hover:-translate-x-1 transition-transform" />
              <span className="text-xs font-medium tracking-wide uppercase">
                Back
              </span>
            </button>
          )}
          <h1 className="text-xl font-bold text-foreground tracking-tight leading-tight pointer-events-none">
            Settings
          </h1>
        </div>

        <div className="flex-1 px-3 space-y-1 overflow-y-auto relative z-20 py-2">
          <NavItem
            icon={Bot}
            label="AI Models"
            description="Core Providers"
            isActive={activeTab === "models"}
            onPress={() => setActiveTab("models")}
          />
          <NavItem
            icon={Key}
            label="API Keys"
            description="Credentials"
            isActive={activeTab === "keys"}
            onPress={() => setActiveTab("keys")}
          />
          <NavItem
            icon={Palette}
            label="Appearance"
            description="Theme & Display"
            isActive={activeTab === "appearance"}
            onPress={() => setActiveTab("appearance")}
          />
          <NavItem
            icon={Shield}
            label="Permissions"
            description="Security"
            isActive={activeTab === "permissions"}
            onPress={() => setActiveTab("permissions")}
          />
          <NavItem
            icon={User}
            label="Profile"
            description="Account"
            isActive={activeTab === "profile"}
            onPress={() => setActiveTab("profile")}
          />
        </div>

        <div className="p-4 pt-2">
          <div className="text-[10px] text-muted-foreground font-mono text-center opacity-50 pointer-events-none">
            Rainy Cowork
          </div>
        </div>
      </aside>

      {/* Main Content Area */}
      <main
        className={`flex-1 rounded-[1.5rem] border border-border/40 shadow-xl flex flex-col overflow-hidden relative z-10 ${isDark ? "bg-card/20" : "bg-card/60"} backdrop-blur-2xl`}
      >
        <div className="absolute top-0 right-0 w-[400px] h-[400px] bg-primary/[0.03] blur-[100px] rounded-full pointer-events-none z-0" />

        {/* Mobile Nav Top Bar */}
        <div className="md:hidden flex overflow-x-auto p-2 border-b border-border/10 bg-background/20 backdrop-blur-xl shrink-0 gap-2 z-20">
          <Button
            size="sm"
            variant="ghost"
            className={activeTab === "models" ? "bg-primary/20 text-primary" : ""}
            onPress={() => setActiveTab("models")}
          >
            <Bot className="size-4 mr-2" /> Models
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className={activeTab === "keys" ? "bg-primary/20 text-primary" : ""}
            onPress={() => setActiveTab("keys")}
          >
            <Key className="size-4 mr-2" /> Keys
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className={activeTab === "appearance" ? "bg-primary/20 text-primary" : ""}
            onPress={() => setActiveTab("appearance")}
          >
            <Palette className="size-4 mr-2" /> Theme
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className={activeTab === "permissions" ? "bg-primary/20 text-primary" : ""}
            onPress={() => setActiveTab("permissions")}
          >
            <Shield className="size-4 mr-2" /> Shield
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className={activeTab === "profile" ? "bg-primary/20 text-primary" : ""}
            onPress={() => setActiveTab("profile")}
          >
            <User className="size-4 mr-2" /> Profile
          </Button>
        </div>

        {/* Content Scroll Area */}
        <div className="flex-1 overflow-y-auto p-6 md:p-8 z-10 scrollbar-hide">
          <div className="max-w-3xl mx-auto pb-16">
            {activeTab === "models" && (
              <div className="animate-in fade-in slide-in-from-bottom-2 duration-300">
                <header className="mb-8 space-y-1 pl-2">
                  <h2 className="text-2xl font-bold tracking-tight text-foreground">
                    AI Models
                  </h2>
                  <p className="text-muted-foreground text-sm uppercase tracking-wider font-medium">
                    Configure preferred models and memory providers
                  </p>
                </header>
                <ModelsTab />
              </div>
            )}

            {activeTab === "keys" && (
              <div className="animate-in fade-in slide-in-from-bottom-2 duration-300">
                <header className="mb-8 space-y-1 pl-2">
                  <h2 className="text-2xl font-bold tracking-tight text-foreground">
                    API Keys
                  </h2>
                  <p className="text-muted-foreground text-sm uppercase tracking-wider font-medium">
                    Manage your keys for different AI providers
                  </p>
                </header>
                <ApiKeysTab />
              </div>
            )}

            {activeTab === "appearance" && (
              <div className="animate-in fade-in slide-in-from-bottom-2 duration-300">
                <header className="mb-8 space-y-1 pl-2">
                  <h2 className="text-2xl font-bold tracking-tight text-foreground">
                    Appearance
                  </h2>
                  <p className="text-muted-foreground text-sm uppercase tracking-wider font-medium">
                    Customize themes and premium animations
                  </p>
                </header>
                <AppearanceTab />
              </div>
            )}

            {activeTab === "permissions" && (
              <div className="animate-in fade-in slide-in-from-bottom-2 duration-300">
                <header className="mb-8 space-y-1 pl-2">
                  <h2 className="text-2xl font-bold tracking-tight text-foreground">
                    Permissions
                  </h2>
                  <p className="text-muted-foreground text-sm uppercase tracking-wider font-medium">
                    Global app behavior and security settings
                  </p>
                </header>
                <PermissionsTab />
              </div>
            )}

            {activeTab === "profile" && (
              <div className="animate-in fade-in slide-in-from-bottom-2 duration-300">
                <header className="mb-8 space-y-1 pl-2">
                  <h2 className="text-2xl font-bold tracking-tight text-foreground">
                    Profile
                  </h2>
                  <p className="text-muted-foreground text-sm uppercase tracking-wider font-medium">
                    Manage your personal identity across the ecosystem
                  </p>
                </header>
                <ProfileTab />
              </div>
            )}
          </div>
        </div>
      </main>
    </div>
  );
}
