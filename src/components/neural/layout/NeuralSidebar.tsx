import { Activity, LayoutDashboard, Users, Radar, PlugZap } from "lucide-react";
import { Button } from "@heroui/react";

interface NeuralSidebarProps {
  activeTab: string;
  onTabChange: (tab: string) => void;
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
    <Button
      variant={isActive ? "secondary" : "ghost"}
      className={`w-full justify-start gap-3 h-auto py-3 px-3 transition-all duration-200 group relative mb-1 ${
        isActive
          ? "bg-primary/10 text-primary font-medium shadow-sm"
          : "text-muted-foreground hover:text-foreground hover:bg-muted/50"
      }`}
      onPress={onPress}
    >
      <div
        className={`p-1 rounded-lg shrink-0 transition-colors ${
          isActive
            ? "bg-primary/10 text-primary"
            : "bg-transparent group-hover:bg-muted"
        }`}
      >
        <Icon className="size-5" />
      </div>

      <div className="flex flex-col items-start min-w-0 text-left">
        <span
          className={`text-sm font-medium ${isActive ? "text-primary" : "text-foreground"}`}
        >
          {label}
        </span>
        <span className="text-[10px] text-muted-foreground/80 font-normal truncate w-full">
          {description}
        </span>
      </div>
    </Button>
  );
};

export function NeuralSidebar({ activeTab, onTabChange }: NeuralSidebarProps) {
  return (
    <aside className="flex flex-col h-full border-r border-border/50 bg-sidebar w-[260px] pb-4 transition-all duration-300">
      {/* Sidebar Header */}
      <div className="p-6 pb-6 flex items-center gap-3" data-tauri-drag-region>
        <div
          className="w-10 h-10 bg-foreground shrink-0"
          style={{
            maskImage: `url(/whale-dnf.png)`,
            maskSize: "contain",
            maskRepeat: "no-repeat",
            maskPosition: "center",
            WebkitMaskImage: `url(/whale-dnf.png)`,
            WebkitMaskSize: "contain",
            WebkitMaskRepeat: "no-repeat",
            WebkitMaskPosition: "center",
          }}
        />
        <h1 className="text-xl font-bold text-foreground tracking-tight leading-none pointer-events-none">
          Neural
          <br />
          Engine
        </h1>
      </div>

      {/* Navigation */}
      <div className="flex-1 px-3 space-y-1 overflow-y-auto scrollbar-hide">
        <div className="px-3 py-2 mb-1">
          <span className="text-[10px] font-bold text-muted-foreground/60 uppercase tracking-widest">
            Modules
          </span>
        </div>

        <NavItem
          icon={LayoutDashboard}
          label="Dashboard"
          description="Overview & Status"
          isActive={activeTab === "dashboard"}
          onPress={() => onTabChange("dashboard")}
        />
        <NavItem
          icon={Users}
          label="Agents"
          description="Manage Fleet"
          isActive={activeTab === "agents"}
          onPress={() => onTabChange("agents")}
        />
        <NavItem
          icon={Activity}
          label="Activity"
          description="Command History"
          isActive={activeTab === "activity"}
          onPress={() => onTabChange("activity")}
        />
        <NavItem
          icon={Radar}
          label="Fleet"
          description="Command Center"
          isActive={activeTab === "fleet"}
          onPress={() => onTabChange("fleet")}
        />
        <NavItem
          icon={PlugZap}
          label="MCP"
          description="Server Control"
          isActive={activeTab === "mcp"}
          onPress={() => onTabChange("mcp")}
        />
      </div>

      {/* Footer / Info */}
      <div className="p-4 mt-auto">
        <div className="px-3 py-2 rounded-xl bg-muted/30 border border-border/50">
          <div className="text-[10px] text-muted-foreground font-mono text-center opacity-70">
            Rainy MaTE
            <span className="mx-1">•</span>
            v2.1.0
          </div>
        </div>
      </div>
    </aside>
  );
}
