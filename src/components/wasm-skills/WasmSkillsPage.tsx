import { useCallback, useEffect, useRef, useState } from "react";
import { Button, Switch } from "@heroui/react";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  ChevronUp,
  CpuIcon,
  FolderOpen,
  Loader2,
  Package,
  RefreshCw,
  ShieldCheck,
  Trash2,
  Zap,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import {
  installLocalSkill,
  installSkillFromAtm,
  listInstalledSkills,
  removeInstalledSkill,
  setInstalledSkillEnabled,
  type InstalledSkillRecord,
} from "../../services/tauri";
import { useTheme } from "../../hooks/useTheme";

// ─── helpers ─────────────────────────────────────────────────────────────────

const AIRLOCK_LABELS: Record<number, { label: string; color: string }> = {
  0: { label: "L0 · Safe", color: "text-emerald-400" },
  1: { label: "L1 · Sensitive", color: "text-amber-400" },
  2: { label: "L2 · Dangerous", color: "text-red-400" },
};

function airlockLabel(level: number) {
  return (
    AIRLOCK_LABELS[level] ?? {
      label: `L${level}`,
      color: "text-muted-foreground",
    }
  );
}

function formatDate(ts: number) {
  return new Date(ts * 1000).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

// ─── sub-components ───────────────────────────────────────────────────────────

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <span className="text-[10px] font-bold text-muted-foreground/60 uppercase tracking-widest">
      {children}
    </span>
  );
}

interface SkillCardProps {
  skill: InstalledSkillRecord;
  onToggle: (id: string, version: string, enabled: boolean) => Promise<void>;
  onRemove: (id: string, version: string) => Promise<void>;
}

function SkillCard({ skill, onToggle, onRemove }: SkillCardProps) {
  const [expanded, setExpanded] = useState(false);
  const [busy, setBusy] = useState(false);

  const handleToggle = async (val: boolean) => {
    setBusy(true);
    try {
      await onToggle(skill.id, skill.version, val);
    } finally {
      setBusy(false);
    }
  };

  const handleRemove = async () => {
    if (
      !confirm(
        `Remove "${skill.name} v${skill.version}"? This cannot be undone.`,
      )
    )
      return;
    setBusy(true);
    try {
      await onRemove(skill.id, skill.version);
    } finally {
      setBusy(false);
    }
  };

  const isUnsigned = skill.trustState === "unsigned_dev";

  return (
    <div
      className={`rounded-2xl border transition-all duration-200 bg-card/30 backdrop-blur-sm ${
        skill.enabled ? "border-border/30" : "border-border/10 opacity-60"
      }`}
    >
      {/* Header row */}
      <div className="p-4 flex items-start gap-3">
        {/* Icon */}
        <div className="mt-0.5 size-9 rounded-xl bg-primary/10 flex items-center justify-center shrink-0">
          <Package className="size-4 text-primary" />
        </div>

        {/* Identity */}
        <div className="flex-1 min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-semibold text-sm text-foreground truncate">
              {skill.name}
            </span>
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded-md bg-muted/50 text-muted-foreground border border-border/30">
              {skill.id}@{skill.version}
            </span>
            {isUnsigned && (
              <span className="text-[10px] px-1.5 py-0.5 rounded-md bg-amber-500/10 text-amber-400 border border-amber-500/20 font-medium">
                unsigned_dev
              </span>
            )}
          </div>
          <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">
            {skill.description}
          </p>
          <div className="flex flex-wrap gap-3 mt-1.5 text-[10px] text-muted-foreground/70 font-mono">
            <span>{skill.runtime}</span>
            <span>·</span>
            <span>{skill.installSource}</span>
            <span>·</span>
            <span>{formatDate(skill.installedAt)}</span>
          </div>
        </div>

        {/* Controls */}
        <div className="flex items-center gap-2 shrink-0">
          <Switch
            isSelected={skill.enabled}
            isDisabled={busy}
            onChange={(val) => handleToggle(val)}
            size="sm"
          />
          <Button
            variant="ghost"
            size="sm"
            isIconOnly
            onPress={() => setExpanded((v) => !v)}
            className="text-muted-foreground hover:text-foreground"
          >
            {expanded ? (
              <ChevronUp className="size-4" />
            ) : (
              <ChevronDown className="size-4" />
            )}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            isIconOnly
            isDisabled={busy}
            onPress={handleRemove}
            className="text-muted-foreground hover:text-red-400"
          >
            <Trash2 className="size-4" />
          </Button>
        </div>
      </div>

      {/* Expanded detail */}
      {expanded && (
        <div className="px-4 pb-4 border-t border-border/20 pt-3 grid grid-cols-1 sm:grid-cols-2 gap-4">
          {/* Permissions */}
          <div>
            <SectionLabel>Permissions</SectionLabel>
            <div className="mt-2 space-y-1 text-xs text-muted-foreground">
              <div>
                FS:{" "}
                {skill.permissions.filesystem.length > 0
                  ? skill.permissions.filesystem
                      .map((p) => p.hostPath)
                      .join(", ")
                  : "none"}
              </div>
              <div>
                Net:{" "}
                {skill.permissions.networkDomains.length > 0
                  ? skill.permissions.networkDomains.join(", ")
                  : "none"}
              </div>
            </div>
          </div>

          {/* Methods */}
          <div>
            <SectionLabel>Methods</SectionLabel>
            <div className="mt-2 space-y-1">
              {skill.methods.map((m) => {
                const al = airlockLabel(m.airlockLevel);
                return (
                  <div key={m.name} className="flex items-center gap-2 text-xs">
                    <Zap className="size-3 text-primary/60 shrink-0" />
                    <span className="font-mono text-foreground">{m.name}</span>
                    <span
                      className={`text-[10px] font-medium ml-auto ${al.color}`}
                    >
                      {al.label}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ─── Install panels ──────────────────────────────────────────────────────────

interface InstallLocalPanelProps {
  onInstalled: () => void;
}

function InstallLocalPanel({ onInstalled }: InstallLocalPanelProps) {
  const [path, setPath] = useState("");
  const [allowUnsigned, setAllowUnsigned] = useState(false);
  const [busy, setBusy] = useState(false);

  const handleBrowse = async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === "string") setPath(selected);
    } catch {
      toast.error("Could not open directory picker");
    }
  };

  const handleInstall = async () => {
    if (!path.trim()) {
      toast.error("Select a skill folder first");
      return;
    }
    setBusy(true);
    try {
      const rec = await installLocalSkill({
        sourceDir: path,
        allowUnsignedDev: allowUnsigned,
      });
      toast.success(`Installed "${rec.name} v${rec.version}"`);
      setPath("");
      onInstalled();
    } catch (err: any) {
      toast.error(err?.message ?? "Install failed");
    } finally {
      setBusy(false);
    }
  };

  const inputClass =
    "w-full bg-background/40 backdrop-blur-sm border border-white/5 hover:border-white/10 focus:border-primary/50 transition-colors h-10 rounded-xl px-4 text-sm text-foreground placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-primary/20";

  return (
    <div className="rounded-2xl border border-border/30 bg-card/20 backdrop-blur-sm p-5 space-y-4">
      <div className="flex items-center gap-2 mb-1">
        <FolderOpen className="size-4 text-primary" />
        <SectionLabel>Install Local Skill</SectionLabel>
      </div>

      {/* Path input + browse */}
      <div className="flex gap-2">
        <input
          className={`${inputClass} flex-1`}
          placeholder="/path/to/skill-folder"
          value={path}
          readOnly
          onClick={handleBrowse}
        />
        <Button
          onPress={handleBrowse}
          className="shrink-0 h-10 px-3 bg-background/30 backdrop-blur-md border border-white/10 hover:bg-white/10 text-foreground text-xs"
        >
          Browse
        </Button>
      </div>

      {/* Allow unsigned toggle */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <AlertTriangle className="size-3.5 text-amber-400" />
          <span className="text-xs text-muted-foreground">
            Allow unsigned dev install
          </span>
        </div>
        <Switch
          isSelected={allowUnsigned}
          onChange={(val) => setAllowUnsigned(val)}
          size="sm"
        />
      </div>

      <Button
        onPress={handleInstall}
        isDisabled={busy || !path}
        className="w-full bg-background/30 backdrop-blur-md border border-white/10 hover:bg-white/10 text-foreground h-10"
      >
        {busy ? (
          <Loader2 className="size-4 animate-spin" />
        ) : (
          <Package className="size-4" />
        )}
        {busy ? "Installing…" : "Install Local"}
      </Button>
    </div>
  );
}

interface InstallAtmPanelProps {
  onInstalled: () => void;
}

function InstallAtmPanel({ onInstalled }: InstallAtmPanelProps) {
  const [baseUrl, setBaseUrl] = useState(
    "https://rainy-atm-cfe3gvcwua-uc.a.run.app",
  );
  const [skillId, setSkillId] = useState("");
  const [busy, setBusy] = useState(false);

  const inputClass =
    "w-full bg-background/40 backdrop-blur-sm border border-white/5 hover:border-white/10 focus:border-primary/50 transition-colors h-10 rounded-xl px-4 text-sm text-foreground placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-primary/20";

  const handleInstall = async () => {
    if (!skillId.trim()) {
      toast.error("Enter a skill ID");
      return;
    }
    setBusy(true);
    try {
      const rec = await installSkillFromAtm({
        baseUrl: baseUrl.trim(),
        skillId: skillId.trim(),
        platformKey: "",
      });
      toast.success(`Installed "${rec.name} v${rec.version}" from MaTE Bridge`);
      setSkillId("");
      onInstalled();
    } catch (err: any) {
      toast.error(err?.message ?? "Bridge install failed");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="rounded-2xl border border-border/30 bg-card/20 backdrop-blur-sm p-5 space-y-4">
      <div className="flex items-center gap-2 mb-1">
        <ShieldCheck className="size-4 text-primary" />
        <SectionLabel>Install from Bridge</SectionLabel>
      </div>

      <input
        className={inputClass}
        placeholder="https://rainy-atm-…"
        value={baseUrl}
        onChange={(e) => setBaseUrl(e.target.value)}
      />
      <input
        className={inputClass}
        placeholder="skill-id"
        value={skillId}
        onChange={(e) => setSkillId(e.target.value)}
      />

      <Button
        onPress={handleInstall}
        isDisabled={busy || !skillId}
        className="w-full bg-background/30 backdrop-blur-md border border-white/10 hover:bg-white/10 text-foreground h-10"
      >
        {busy ? (
          <Loader2 className="size-4 animate-spin" />
        ) : (
          <ShieldCheck className="size-4" />
        )}
        {busy ? "Installing…" : "Install From Bridge"}
      </Button>
    </div>
  );
}

// ─── Policy Audit Log ─────────────────────────────────────────────────────────

function PolicyAuditLog() {
  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <SectionLabel>Policy Audit Log</SectionLabel>
        <span className="text-[10px] text-muted-foreground font-mono">
          0 events
        </span>
      </div>
      <div className="rounded-2xl border border-border/20 bg-card/10 p-8 text-center">
        <CheckCircle2 className="size-8 text-muted-foreground/30 mx-auto mb-2" />
        <p className="text-xs text-muted-foreground">
          No audit events recorded.
        </p>
      </div>
    </div>
  );
}

// ─── Main Page ────────────────────────────────────────────────────────────────

export function WasmSkillsPage() {
  const { mode } = useTheme();
  const isDark = mode === "dark";
  const [skills, setSkills] = useState<InstalledSkillRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const loadingRef = useRef(false);

  const refresh = useCallback(async () => {
    if (loadingRef.current) return;
    loadingRef.current = true;
    setLoading(true);
    try {
      const list = await listInstalledSkills();
      setSkills(list);
    } catch (err: any) {
      toast.error(err?.message ?? "Failed to load skills");
    } finally {
      setLoading(false);
      loadingRef.current = false;
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleToggle = async (
    id: string,
    version: string,
    enabled: boolean,
  ) => {
    try {
      await setInstalledSkillEnabled({ skillId: id, version, enabled });
      setSkills((prev) =>
        prev.map((s) =>
          s.id === id && s.version === version ? { ...s, enabled } : s,
        ),
      );
    } catch (err: any) {
      toast.error(err?.message ?? "Failed to update skill");
    }
  };

  const handleRemove = async (id: string, version: string) => {
    try {
      await removeInstalledSkill({ skillId: id, version });
      setSkills((prev) =>
        prev.filter((s) => !(s.id === id && s.version === version)),
      );
      toast.success("Skill removed");
    } catch (err: any) {
      toast.error(err?.message ?? "Failed to remove skill");
    }
  };

  return (
    <div
      className={`h-full w-full bg-background p-3 flex gap-3 overflow-hidden font-sans selection:bg-primary selection:text-primary-foreground relative`}
    >
      {/* drag region background */}
      <div
        className="absolute inset-0 w-full h-full z-0 pointer-events-none"
        data-tauri-drag-region
      />

      {/* ── Left sidebar ──────────────────────────────────────────────────── */}
      <aside className="flex flex-col h-full border-r border-border/50 bg-sidebar w-[260px] pb-4 shrink-0 rounded-[1.5rem] overflow-hidden">
        {/* Header */}
        <div
          className="p-6 pb-4 flex items-center gap-3"
          data-tauri-drag-region
        >
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
            Wasm
            <br />
            Skills
          </h1>
        </div>

        {/* Info cards */}
        <div className="flex-1 px-4 space-y-3 overflow-y-auto scrollbar-hide">
          <div className="px-1 py-2">
            <SectionLabel>Overview</SectionLabel>
          </div>

          {/* Stats */}
          <div className="rounded-xl bg-muted/30 border border-border/40 p-4 space-y-3">
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground text-xs">
                Total Skills
              </span>
              <span className="font-semibold text-foreground tabular-nums">
                {skills.length}
              </span>
            </div>
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground text-xs">Active</span>
              <span className="font-semibold text-emerald-400 tabular-nums">
                {skills.filter((s) => s.enabled).length}
              </span>
            </div>
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground text-xs">Disabled</span>
              <span className="font-semibold text-muted-foreground tabular-nums">
                {skills.filter((s) => !s.enabled).length}
              </span>
            </div>
          </div>

          {/* Runtime badge */}
          <div className="rounded-xl bg-primary/5 border border-primary/10 p-4">
            <div className="flex items-center gap-2 mb-2">
              <CpuIcon className="size-4 text-primary" />
              <span className="text-xs font-semibold text-primary">
                Wasm Runtime
              </span>
            </div>
            <p className="text-[11px] text-muted-foreground leading-relaxed">
              Skills run inside a Wasmtime sandbox with strict memory limits,
              timeouts, and Ed25519 signature verification.
            </p>
          </div>

          {/* Airlock legend */}
          <div className="rounded-xl bg-muted/20 border border-border/30 p-4 space-y-2">
            <SectionLabel>Airlock Levels</SectionLabel>
            <div className="mt-2 space-y-1.5">
              {Object.entries(AIRLOCK_LABELS).map(([lvl, { label, color }]) => (
                <div key={lvl} className={`text-xs font-mono ${color}`}>
                  {label}
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="px-4 pt-3">
          <div className="px-3 py-2 rounded-xl bg-muted/30 border border-border/50">
            <div className="text-[10px] text-muted-foreground font-mono text-center opacity-70">
              Rainy Cowork • WASM Sandbox
            </div>
          </div>
        </div>
      </aside>

      {/* ── Main content ──────────────────────────────────────────────────── */}
      <main
        className={`flex-1 rounded-[1.5rem] border border-border/40 shadow-xl flex flex-col overflow-hidden relative z-10 ${
          isDark ? "bg-card/20" : "bg-card/60"
        } backdrop-blur-2xl`}
      >
        {/* Ambient glow */}
        <div className="absolute top-0 right-0 w-[400px] h-[400px] bg-primary/[0.03] blur-[100px] rounded-full pointer-events-none z-0" />

        {/* Page header */}
        <header
          className="h-16 shrink-0 flex items-center justify-between px-8 border-b border-border/10 bg-background/20 backdrop-blur-xl z-20 relative"
          data-tauri-drag-region
        >
          <div className="flex items-center gap-3">
            <Package className="size-4 text-primary" />
            <h2 className="text-lg font-bold text-foreground tracking-tight">
              Wasm Skill Sandbox
            </h2>
            <div className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-foreground/5 border border-foreground/5">
              <span className="text-xs text-muted-foreground">
                Install and manage third-party skills (hash/signature verified,
                fail-closed runtime).
              </span>
            </div>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onPress={refresh}
            isDisabled={loading}
            className="text-muted-foreground hover:text-foreground gap-2"
          >
            <RefreshCw className={`size-4 ${loading ? "animate-spin" : ""}`} />
            {!loading && "Refresh"}
          </Button>
        </header>

        {/* Scrollable body */}
        <div className="flex-1 overflow-y-auto p-6 sm:p-8 z-10 scrollbar-hide">
          <div className="max-w-5xl mx-auto pb-16 space-y-10">
            {/* ── Install Section ─────────────────────────────────────────── */}
            <section>
              <div className="mb-4">
                <SectionLabel>Install Skills</SectionLabel>
              </div>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <InstallLocalPanel onInstalled={refresh} />
                <InstallAtmPanel onInstalled={refresh} />
              </div>
            </section>

            {/* ── Installed Skills ─────────────────────────────────────────── */}
            <section>
              <div className="flex items-center justify-between mb-4">
                <SectionLabel>Installed Skills</SectionLabel>
                <span className="text-[10px] text-muted-foreground font-mono">
                  {skills.length} skill{skills.length !== 1 ? "s" : ""}
                </span>
              </div>

              {loading ? (
                <div className="flex items-center justify-center py-16 gap-3 text-muted-foreground">
                  <Loader2 className="size-5 animate-spin" />
                  <span className="text-sm">Loading skills…</span>
                </div>
              ) : skills.length === 0 ? (
                <div className="rounded-2xl border border-dashed border-border/30 p-12 text-center">
                  <Package className="size-10 text-muted-foreground/20 mx-auto mb-3" />
                  <p className="text-sm text-muted-foreground">
                    No skills installed yet.
                  </p>
                  <p className="text-xs text-muted-foreground/60 mt-1">
                    Use the panels above to install your first Wasm skill.
                  </p>
                </div>
              ) : (
                <div className="space-y-3">
                  {skills.map((skill) => (
                    <SkillCard
                      key={`${skill.id}@${skill.version}`}
                      skill={skill}
                      onToggle={handleToggle}
                      onRemove={handleRemove}
                    />
                  ))}
                </div>
              )}
            </section>

            {/* ── Audit Log ─────────────────────────────────────────────────── */}
            <section>
              <PolicyAuditLog />
            </section>
          </div>
        </div>
      </main>
    </div>
  );
}
