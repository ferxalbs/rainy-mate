import {
  Activity,
  LayoutDashboard,
  // Settings, // @TODO: Remove
  // ShieldAlert, // @TODO: Remove
  Users,
} from "lucide-react";

interface NeuralSidebarProps {
  activeTab: string;
  onTabChange: (tab: string) => void;
  isDark?: boolean;
}

export function NeuralSidebar({
  activeTab,
  onTabChange,
  isDark = true,
}: NeuralSidebarProps) {
  const NavItem = ({
    id,
    icon: Icon,
    label,
    description,
  }: {
    id: string;
    icon: any;
    label: string;
    description: string;
  }) => {
    const isActive = activeTab === id;
    return (
      <button
        onClick={() => onTabChange(id)}
        className={`w-full text-left px-4 py-3 rounded-2xl transition-all duration-300 group relative overflow-hidden ${
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
            <span
              className={`text-[10px] uppercase tracking-wider ${isActive ? "text-primary-foreground/70" : "text-muted-foreground"}`}
            >
              {description}
            </span>
          </div>
        </div>
      </button>
    );
  };

  return (
    <aside
      className={`w-[260px] shrink-0 rounded-[1.5rem] border border-border/40 flex flex-col shadow-xl overflow-hidden relative z-10 ${
        isDark ? "bg-card/20" : "bg-card/60"
      } backdrop-blur-2xl`}
    >
      <div className="p-6 pb-2" data-tauri-drag-region>
        <h1 className="text-xl font-bold text-foreground tracking-tight leading-tight pointer-events-none">
          Neural
          <br />
          Engine
        </h1>
      </div>

      <div className="flex-1 px-3 space-y-1 overflow-y-auto relative z-20">
        <NavItem
          id="dashboard"
          icon={LayoutDashboard}
          label="Dashboard"
          description="Overview & Status"
        />
        <NavItem
          id="agents"
          icon={Users}
          label="Agents"
          description="Manage Fleet"
        />
        <NavItem
          id="activity"
          icon={Activity}
          label="Activity"
          description="Command History"
        />
        {/* @TODO: Remove in next version - Legacy panels */}
        {/* <NavItem
          id="health"
          icon={ShieldAlert}
          label="Health"
          description="Metrics & SLOs"
        />
        <NavItem
          id="settings"
          icon={Settings}
          label="Settings"
          description="Admin & Policy"
        /> */}
      </div>

      <div className="p-4 pt-2">
        <div className="text-[10px] text-muted-foreground font-mono text-center opacity-50 pointer-events-none">
          Rainy Cowork
        </div>
      </div>
    </aside>
  );
}
