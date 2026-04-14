import { useEffect, useState } from "react";
import { Bell, Power, ShieldCheck, Zap } from "lucide-react";
import { Button, Switch } from "@heroui/react";
import {
  getUserSettings,
  getLaunchAtLoginStatus,
  getNotificationStatus,
  openLaunchAtLoginSettings,
  requestNotificationPermission,
  sendTestNotification,
  setAuditLegacyModeEnabled,
  setLaunchAtLoginEnabled,
  setNotifications,
} from "../../../services/tauri";
import { toast } from "sonner";

type PermissionState = "unknown" | "granted" | "denied" | "unsupported";

export function PermissionsTab() {
  const [notificationsEnabled, setNotificationsEnabled] = useState(true);
  const [launchAtLoginEnabled, setLaunchAtLoginState] = useState(false);
  const [launchAtLoginSupported, setLaunchAtLoginSupported] = useState(false);
  const [launchAtLoginRequiresApproval, setLaunchAtLoginRequiresApproval] =
    useState(false);
  const [auditLegacyModeEnabled, setAuditLegacyModeState] = useState(false);
  const [notificationPermission, setNotificationPermission] =
    useState<PermissionState>("unknown");
  const [isBusy, setIsBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function loadState() {
      try {
        const status = await getNotificationStatus();

        if (cancelled) return;
        setNotificationsEnabled(status.enabled);
        setNotificationPermission(
          status.permission === "granted" ||
            status.permission === "denied" ||
            status.permission === "unsupported"
            ? status.permission
            : "unknown",
        );

        const launchStatus = await getLaunchAtLoginStatus();
        if (cancelled) return;
        setLaunchAtLoginState(launchStatus.enabled);
        setLaunchAtLoginSupported(launchStatus.supported);
        setLaunchAtLoginRequiresApproval(launchStatus.requiresApproval);

        const userSettings = await getUserSettings();
        if (cancelled) return;
        setAuditLegacyModeState(userSettings.auditLegacyModeEnabled);
      } catch (error) {
        if (cancelled) return;
        console.error("Failed to load notification status:", error);
        setNotificationPermission("unknown");
      }
    }

    void loadState();

    return () => {
      cancelled = true;
    };
  }, []);

  const handleNotificationToggle = async (enabled: boolean) => {
    setIsBusy(true);
    try {
      await setNotifications(enabled);
      setNotificationsEnabled(enabled);
      if (enabled) {
        const granted = await requestNotificationPermission();
        setNotificationPermission(granted ? "granted" : "denied");
      }
    } catch (error) {
      console.error("Failed to update notifications:", error);
      toast.error("Failed to update notifications");
    } finally {
      setIsBusy(false);
    }
  };

  const handleLaunchAtLoginToggle = async (enabled: boolean) => {
    setIsBusy(true);
    try {
      const status = await setLaunchAtLoginEnabled(enabled);
      setLaunchAtLoginState(status.enabled);
      setLaunchAtLoginSupported(status.supported);
      setLaunchAtLoginRequiresApproval(status.requiresApproval);

      if (status.requiresApproval) {
        toast.warning("macOS requires approval in Login Items settings");
      }
    } catch (error) {
      console.error("Failed to update launch at login:", error);
      toast.error("Failed to update launch at login");
    } finally {
      setIsBusy(false);
    }
  };

  const handleOpenLaunchSettings = async () => {
    setIsBusy(true);
    try {
      await openLaunchAtLoginSettings();
    } catch (error) {
      console.error("Failed to open Login Items settings:", error);
      toast.error("Failed to open Login Items settings");
    } finally {
      setIsBusy(false);
    }
  };

  const handleAuditLegacyModeToggle = async (enabled: boolean) => {
    setIsBusy(true);
    try {
      await setAuditLegacyModeEnabled(enabled);
      setAuditLegacyModeState(enabled);
      toast.success(
        enabled
          ? "AUDIT (Legacy) enabled"
          : "Modern stream pipeline enabled",
      );
    } catch (error) {
      console.error("Failed to update runtime mode:", error);
      toast.error("Failed to update runtime mode");
    } finally {
      setIsBusy(false);
    }
  };

  const handleSendTest = async () => {
    setIsBusy(true);
    try {
      const granted = await requestNotificationPermission();
      setNotificationPermission(granted ? "granted" : "denied");
      if (!granted) {
        toast.warning("macOS notifications are not enabled for Rainy MaTE");
        return;
      }

      await sendTestNotification();
      toast.success("Test notification queued");
    } catch (error) {
      console.error("Failed to send test notification:", error);
      toast.error("Failed to send test notification");
    } finally {
      setIsBusy(false);
    }
  };

  const permissionLabel =
    notificationPermission === "granted"
      ? "Granted"
      : notificationPermission === "denied"
        ? "Denied"
        : notificationPermission === "unsupported"
          ? "Unavailable in raw debug launch"
        : "Unknown";

  return (
    <div className="space-y-10 animate-in fade-in duration-500">
      <div className="space-y-6">
        <div className="flex items-center justify-between gap-4 p-4 rounded-2xl bg-muted/10 border border-border/5 hover:bg-muted/20 transition-all group">
          <div className="flex flex-col gap-1">
            <span className="text-sm font-bold flex items-center gap-2 tracking-tight uppercase opacity-80">
              <Bell className="size-4 text-primary" />
              Desktop Notifications
            </span>
            <span className="text-xs text-muted-foreground max-w-sm">
              Airlock and runtime alerts use macOS notifications, but approvals remain in-app for security.
            </span>
            <span className="text-[11px] text-muted-foreground/80">
              macOS permission: {permissionLabel}
            </span>
          </div>
          <Switch
            isDisabled={isBusy}
            isSelected={notificationsEnabled}
            onChange={handleNotificationToggle}
          >
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
          </Switch>
        </div>

        <div className="flex items-center justify-between gap-4 p-4 rounded-2xl bg-muted/10 border border-border/5 hover:bg-muted/20 transition-all group">
          <div className="flex flex-col gap-1">
            <span className="text-sm font-bold flex items-center gap-2 tracking-tight uppercase opacity-80">
              <Power className="size-4 text-primary" />
              Launch At Login
            </span>
            <span className="text-xs text-muted-foreground max-w-sm">
              Keeps recurring workspace runs reliable after macOS restart by reopening Rainy MaTE automatically.
            </span>
            <span className="text-[11px] text-muted-foreground/80">
              {launchAtLoginSupported
                ? launchAtLoginRequiresApproval
                  ? "Needs approval in Login Items"
                  : launchAtLoginEnabled
                    ? "Enabled"
                    : "Disabled"
                : "Unavailable in raw debug launch"}
            </span>
          </div>
          <div className="flex items-center gap-2">
            {launchAtLoginRequiresApproval ? (
              <Button
                isDisabled={isBusy}
                onPress={handleOpenLaunchSettings}
                variant="outline"
              >
                Open Settings
              </Button>
            ) : null}
            <Switch
              isDisabled={isBusy || !launchAtLoginSupported}
              isSelected={launchAtLoginEnabled}
              onChange={handleLaunchAtLoginToggle}
            >
              <Switch.Control>
                <Switch.Thumb />
              </Switch.Control>
            </Switch>
          </div>
        </div>

        <div className="flex items-center justify-between gap-4 p-4 rounded-2xl bg-muted/10 border border-border/5 hover:bg-muted/20 transition-all group">
          <div className="flex flex-col gap-1">
            <span className="text-sm font-bold flex items-center gap-2 tracking-tight uppercase opacity-80">
              <ShieldCheck className="size-4 text-primary" />
              Auto-Execute Safe Tools
            </span>
            <span className="text-xs text-muted-foreground max-w-sm">
              Level 0 tools stay automatic. Sensitive and dangerous work still goes through Airlock.
            </span>
          </div>
          <Switch isSelected isDisabled>
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
          </Switch>
        </div>

        <div className="flex items-center justify-between gap-4 p-4 rounded-2xl bg-muted/10 border border-border/5 hover:bg-muted/20 transition-all group">
          <div className="flex flex-col gap-1">
            <span className="text-sm font-bold flex items-center gap-2 tracking-tight uppercase opacity-80">
              <ShieldCheck className="size-4 text-primary" />
              AUDIT (Legacy)
            </span>
            <span className="text-xs text-muted-foreground max-w-sm">
              Re-enable the older buffered event transport for comparison and debugging. Leave this off for the modern Rainy stream pipeline.
            </span>
            <span className="text-[11px] text-muted-foreground/80">
              {auditLegacyModeEnabled
                ? "Legacy batching and status-string audit mode active"
                : "Modern typed streaming active"}
            </span>
          </div>
          <Switch
            isDisabled={isBusy}
            isSelected={auditLegacyModeEnabled}
            onChange={handleAuditLegacyModeToggle}
          >
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
          </Switch>
        </div>

        <div className="flex items-center justify-between gap-4 p-4 rounded-2xl bg-muted/10 border border-border/5 hover:bg-muted/20 transition-all group">
          <div className="flex flex-col gap-1">
            <span className="text-sm font-bold flex items-center gap-2 tracking-tight uppercase opacity-80">
              <Zap className="size-4 text-primary" />
              Notification Probe
            </span>
            <span className="text-xs text-muted-foreground max-w-sm">
              Send a real local notification to verify your current macOS permission state.
            </span>
          </div>
          <Button
            isDisabled={isBusy || !notificationsEnabled}
            onPress={handleSendTest}
            variant="primary"
          >
            Send Test
          </Button>
        </div>
      </div>

      <div className="h-px bg-success/10 w-full opacity-10" />

      <section className="space-y-4">
        <h3 className="text-xs font-bold uppercase tracking-widest text-muted-foreground/60 ml-1">
          Airlock Policy
        </h3>
        <div className="p-4 rounded-2xl bg-primary/5 border border-primary/10 text-xs text-foreground/70 leading-relaxed italic">
          Rainy MaTE now treats macOS notifications as an alert channel only. Airlock approvals always resolve inside the app, and dangerous tools remain blocked until you explicitly approve them.
        </div>
      </section>
    </div>
  );
}
