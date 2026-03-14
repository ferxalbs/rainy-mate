import { Bell, ShieldCheck, Zap } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { Separator } from "@/components/ui/separator";

export function PermissionsTab() {
  return (
    <div className="space-y-10 animate-in fade-in slide-in-from-bottom-2 duration-500">
      <div className="space-y-6">
        <div className="flex items-center justify-between p-4 rounded-2xl bg-muted/10 border border-border/5 hover:bg-muted/20 transition-all group">
          <div className="flex flex-col gap-1">
            <span className="text-sm font-bold flex items-center gap-2 tracking-tight uppercase opacity-80">
              <Bell className="size-4 text-primary" />
              Desktop Notifications
            </span>
            <span className="text-xs text-muted-foreground max-w-sm">
              Receive alerts for task completions and system status.
            </span>
          </div>
          <Switch defaultChecked />
        </div>

        <div className="flex items-center justify-between p-4 rounded-2xl bg-muted/10 border border-border/5 hover:bg-muted/20 transition-all group">
          <div className="flex flex-col gap-1">
            <span className="text-sm font-bold flex items-center gap-2 tracking-tight uppercase opacity-80">
              <ShieldCheck className="size-4 text-primary" />
              Auto-Execute Safe Tools
            </span>
            <span className="text-xs text-muted-foreground max-w-sm">
              Allow L0 tools to run without explicit approval.
            </span>
          </div>
          <Switch defaultChecked />
        </div>

        <div className="flex items-center justify-between p-4 rounded-2xl bg-muted/10 border border-border/5 hover:bg-muted/20 transition-all group">
          <div className="flex flex-col gap-1">
            <span className="text-sm font-bold flex items-center gap-2 tracking-tight uppercase opacity-80">
              <Zap className="size-4 text-primary" />
              High Speed Mode
            </span>
            <span className="text-xs text-muted-foreground max-w-sm">
              Prioritize execution speed over confirmation verbosity.
            </span>
          </div>
          <Switch />
        </div>
      </div>

      <Separator className="opacity-10" />

      <section className="space-y-4">
        <h3 className="text-xs font-bold uppercase tracking-widest text-muted-foreground/60 ml-1">
          Airlock Policy
        </h3>
        <div className="p-4 rounded-2xl bg-primary/5 border border-primary/10 text-xs text-foreground/70 leading-relaxed italic">
          Your security policy is managed by the Airlock Service. Level 2 (Dangerous) tools always require manual cryptographic signature.
        </div>
      </section>
    </div>
  );
}
