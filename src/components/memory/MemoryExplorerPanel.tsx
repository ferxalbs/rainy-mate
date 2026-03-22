import { useState, useEffect, useCallback, useMemo } from "react";
import {
  Brain,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Clock,
  Hash,
  RefreshCw,
  Search,
  Shield,
  Sparkles,
  Trash2,
  TriangleAlert,
} from "lucide-react";
import {
  Button,
  Card,
  Chip,
  Input,
  ListBox,
  Select,
  Spinner,
} from "@heroui/react";
import { toast } from "sonner";
import * as tauri from "../../services/tauri";
import type {
  DecryptedMemoryEntry,
  VaultDetailedStats,
  WorkspaceSummary,
} from "../../types/memory";

const PAGE_SIZE = 20;

const SENSITIVITY_STYLES: Record<string, string> = {
  public:
    "border-emerald-500/20 bg-emerald-500/12 text-emerald-700 dark:text-emerald-300",
  internal:
    "border-sky-500/20 bg-sky-500/12 text-sky-700 dark:text-sky-300",
  confidential:
    "border-rose-500/20 bg-rose-500/12 text-rose-700 dark:text-rose-300",
};

const CATEGORY_STYLES: Record<string, string> = {
  preference:
    "border-violet-500/20 bg-violet-500/12 text-violet-700 dark:text-violet-300",
  correction:
    "border-amber-500/20 bg-amber-500/12 text-amber-700 dark:text-amber-300",
  fact: "border-sky-500/20 bg-sky-500/12 text-sky-700 dark:text-sky-300",
  procedure:
    "border-emerald-500/20 bg-emerald-500/12 text-emerald-700 dark:text-emerald-300",
  observation:
    "border-zinc-500/20 bg-zinc-500/12 text-zinc-700 dark:text-zinc-300",
};

const PANEL_CLASS =
  "border border-border/50 dark:border-white/8 bg-background/80 dark:bg-background/24 backdrop-blur-md shadow-[0_18px_42px_-34px_rgba(0,0,0,0.38)]";
const POPOVER_CLASS = `${PANEL_CLASS} rounded-[18px] p-1.5`;
const LISTBOX_CLASS = "bg-transparent p-0";
const LISTBOX_ITEM_CLASS =
  "rounded-[12px] border border-transparent bg-background/72 px-3 py-2 text-sm text-foreground/92 outline-none transition-colors dark:bg-background/18 data-[hovered=true]:border-border/60 data-[hovered=true]:bg-background/92 dark:data-[hovered=true]:border-white/10 dark:data-[hovered=true]:bg-background/34 data-[focused=true]:border-border/60 data-[focused=true]:bg-background/92 dark:data-[focused=true]:border-white/10 dark:data-[focused=true]:bg-background/34 data-[selected=true]:border-primary/20 data-[selected=true]:bg-primary/10 data-[selected=true]:text-foreground";
const BUTTON_BASE_CLASS =
  "rounded-full border border-border/45 bg-background/72 text-foreground/88 shadow-none transition-all hover:border-border/70 hover:bg-background/90 hover:text-foreground dark:border-white/8 dark:bg-background/20 dark:hover:border-white/12 dark:hover:bg-background/30";
const BUTTON_ACTIVE_CLASS =
  "rounded-full border border-primary/18 bg-primary/12 text-foreground shadow-none transition-all hover:bg-primary/16 hover:text-foreground dark:border-primary/18 dark:bg-primary/16 dark:hover:bg-primary/20";
const BUTTON_DANGER_CLASS =
  "rounded-full border border-rose-500/18 bg-rose-500/10 text-rose-700 shadow-none transition-all hover:bg-rose-500/16 hover:text-rose-800 dark:text-rose-300 dark:hover:text-rose-200";

function timeAgo(ts: number): string {
  const diff = Math.floor(Date.now() / 1000 - ts);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  if (diff < 604800) return `${Math.floor(diff / 86400)}d ago`;
  return `${Math.floor(diff / 604800)}w ago`;
}

function selectionToValue(selection: unknown): string | null {
  if (typeof selection === "string") return selection;
  if (selection instanceof Set) {
    const first = selection.values().next().value;
    return typeof first === "string" ? first : null;
  }
  return null;
}

function getCategory(entry: DecryptedMemoryEntry): string {
  return entry.metadata["_category"] ?? "observation";
}

function getImportance(entry: DecryptedMemoryEntry): number | null {
  const raw = entry.metadata["_importance"];
  if (!raw) return null;
  const parsed = Number.parseFloat(raw);
  return Number.isFinite(parsed) ? Math.max(0, Math.min(1, parsed)) : null;
}

function StatTile({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint?: string;
}) {
  return (
    <Card className={`${PANEL_CLASS} min-w-[120px] rounded-[18px] px-3 py-2.5 shadow-none`}>
      <Card.Header className="p-0">
        <div className="space-y-1">
          <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-muted-foreground/80">
            {label}
          </p>
          <p className="text-base font-semibold text-foreground">{value}</p>
          {hint ? (
            <p className="text-xs text-muted-foreground">{hint}</p>
          ) : null}
        </div>
      </Card.Header>
    </Card>
  );
}

function EntryMetadataChip({
  children,
  className = "",
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <Chip
      className={`rounded-full border border-white/15 bg-background/70 px-2 text-[10px] text-foreground/78 dark:bg-background/20 ${className}`}
      variant="soft"
      size="sm"
    >
      {children}
    </Chip>
  );
}

interface EntryCardProps {
  entry: DecryptedMemoryEntry;
  expanded: boolean;
  selected: boolean;
  onToggleExpanded: () => void;
  onToggleSelected: () => void;
}

function EntryCard({
  entry,
  expanded,
  selected,
  onToggleExpanded,
  onToggleSelected,
}: EntryCardProps) {
  const category = getCategory(entry);
  const importance = getImportance(entry);

  return (
    <Card
      className={`${PANEL_CLASS} rounded-[22px] px-4 py-3 transition-all duration-200 ${
        selected
          ? "border-primary/35 bg-primary/[0.08] dark:bg-primary/[0.10]"
          : "hover:border-white/18 hover:bg-background/84 dark:hover:bg-background/28"
      }`}
    >
      <Card.Header className="flex items-start justify-between gap-4 p-0">
        <div className="min-w-0 flex-1 space-y-2.5">
          <div className="flex flex-wrap items-center gap-2">
            <Chip
              className={`rounded-full border px-2 ${CATEGORY_STYLES[category] ?? CATEGORY_STYLES.observation}`}
              variant="soft"
              size="sm"
            >
              {category}
            </Chip>
            <Chip
              className={`rounded-full border px-2 ${SENSITIVITY_STYLES[entry.sensitivity] ?? ""}`}
              variant="soft"
              size="sm"
            >
              <Shield className="mr-1 size-3" />
              {entry.sensitivity}
            </Chip>
            <EntryMetadataChip>
              <Clock className="mr-1 size-3" />
              {timeAgo(entry.created_at)}
            </EntryMetadataChip>
            <EntryMetadataChip>
              <Hash className="mr-1 size-3" />
              {entry.access_count}
            </EntryMetadataChip>
            <EntryMetadataChip>{entry.source}</EntryMetadataChip>
          </div>

          <div className="space-y-2">
            <p className="text-[13px] leading-5.5 text-foreground/92">
              {expanded
                ? entry.content
                : `${entry.content.slice(0, 220)}${entry.content.length > 220 ? "..." : ""}`}
            </p>

            {importance !== null ? (
              <div className="flex items-center gap-2.5">
                <span className="text-[10px] font-semibold uppercase tracking-[0.18em] text-muted-foreground">
                  Importance
                </span>
                <div className="h-1.5 w-24 overflow-hidden rounded-full bg-primary/10">
                  <div
                    className="h-full rounded-full bg-primary/65"
                    style={{ width: `${importance * 100}%` }}
                  />
                </div>
                <span className="text-xs text-muted-foreground">
                  {Math.round(importance * 100)}%
                </span>
              </div>
            ) : null}
          </div>
        </div>

        <div className="flex shrink-0 items-start gap-2">
            <Button
              isIconOnly
              variant={selected ? "primary" : "tertiary"}
              size="sm"
            className="rounded-full opacity-85"
            onPress={onToggleSelected}
          >
            <Sparkles className="size-4" />
          </Button>
            <Button
              isIconOnly
              variant="ghost"
              size="sm"
            className="rounded-full opacity-70 hover:opacity-100"
            onPress={onToggleExpanded}
          >
            <ChevronDown
              className={`size-4 transition-transform ${expanded ? "rotate-180" : ""}`}
            />
          </Button>
        </div>
      </Card.Header>

      {expanded ? (
        <Card.Content className="mt-3 grid gap-3 border-t border-border/40 p-0 pt-3">
          <div className="grid gap-2.5 md:grid-cols-2 xl:grid-cols-4">
            <StatTile label="Workspace" value={entry.workspace_id} />
            <StatTile
              label="Embedding"
              value={entry.embedding_model ?? "None"}
              hint={`${entry.embedding_dim ?? 0} dimensions`}
            />
            <StatTile
              label="Last Accessed"
              value={timeAgo(entry.last_accessed)}
            />
            <StatTile
              label="Entry ID"
              value={`${entry.id.slice(0, 10)}...`}
            />
          </div>

          {entry.tags.length > 0 ? (
            <div className="space-y-2">
              <p className="text-[10px] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                Tags
              </p>
              <div className="flex flex-wrap gap-1.5">
                {entry.tags.map((tag) => (
                  <Chip
                    key={tag}
                    className="rounded-full border border-white/15 bg-background/70 dark:bg-background/20"
                    variant="soft"
                    size="sm"
                  >
                    {tag}
                  </Chip>
                ))}
              </div>
            </div>
          ) : null}

          {Object.keys(entry.metadata).length > 0 ? (
            <div className="space-y-2">
              <p className="text-[10px] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                Metadata
              </p>
              <Card
                className={`${PANEL_CLASS} rounded-[18px] px-3 py-2.5 shadow-none`}
              >
                <Card.Content className="flex flex-wrap gap-1.5 p-0 text-xs text-foreground/78">
                  {Object.entries(entry.metadata).map(([key, value]) => (
                    <Chip
                      key={key}
                      className="rounded-full border border-white/12 bg-background/70 dark:bg-background/20"
                      variant="soft"
                      size="sm"
                    >
                      {key}={value}
                    </Chip>
                  ))}
                </Card.Content>
              </Card>
            </div>
          ) : null}
        </Card.Content>
      ) : null}
    </Card>
  );
}

export function MemoryExplorerPanel() {
  const [entries, setEntries] = useState<DecryptedMemoryEntry[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(false);
  const [workspaces, setWorkspaces] = useState<WorkspaceSummary[]>([]);
  const [selectedWorkspace, setSelectedWorkspace] = useState("__all__");
  const [selectedSensitivity, setSelectedSensitivity] = useState("__all__");
  const [sourceFilter, setSourceFilter] = useState("");
  const [selectedCategory, setSelectedCategory] = useState("__all__");
  const [sortBy, setSortBy] = useState("created_at");
  const [stats, setStats] = useState<VaultDetailedStats | null>(null);
  const [showStats, setShowStats] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const fetchEntries = useCallback(async () => {
    setLoading(true);
    try {
      const result = await tauri.listVaultEntries({
        workspaceId:
          selectedWorkspace === "__all__" ? undefined : selectedWorkspace,
        sensitivity:
          selectedSensitivity === "__all__" ? undefined : selectedSensitivity,
        sourcePrefix: sourceFilter || undefined,
        orderBy: sortBy,
        limit: PAGE_SIZE,
        offset,
      });
      setEntries(result.entries);
      setTotalCount(result.total_count);
    } catch (error) {
      toast.error(`Failed to load entries: ${error}`);
    } finally {
      setLoading(false);
    }
  }, [offset, selectedSensitivity, selectedWorkspace, sortBy, sourceFilter]);

  const fetchWorkspaces = useCallback(async () => {
    try {
      const result = await tauri.listMemoryWorkspaces();
      setWorkspaces(result);
    } catch {
      // ignore sidebar decoration fetch failures
    }
  }, []);

  const fetchStats = useCallback(async () => {
    try {
      const result = await tauri.getVaultDetailedStats(
        selectedWorkspace === "__all__" ? undefined : selectedWorkspace,
      );
      setStats(result);
    } catch {
      // ignore optional stats fetch failures
    }
  }, [selectedWorkspace]);

  useEffect(() => {
    fetchWorkspaces();
  }, [fetchWorkspaces]);

  useEffect(() => {
    fetchEntries();
  }, [fetchEntries]);

  useEffect(() => {
    if (showStats) {
      fetchStats();
    }
  }, [fetchStats, showStats]);

  useEffect(() => {
    setOffset(0);
    setSelected(new Set());
  }, [
    selectedWorkspace,
    selectedSensitivity,
    selectedCategory,
    sourceFilter,
    sortBy,
  ]);

  const filteredEntries = useMemo(() => {
    if (selectedCategory === "__all__") return entries;
    return entries.filter((entry) => getCategory(entry) === selectedCategory);
  }, [entries, selectedCategory]);

  const totalPages = Math.max(1, Math.ceil(totalCount / PAGE_SIZE));
  const currentPage = Math.floor(offset / PAGE_SIZE) + 1;

  const handleRefresh = useCallback(() => {
    fetchEntries();
    fetchWorkspaces();
    if (showStats) fetchStats();
  }, [fetchEntries, fetchStats, fetchWorkspaces, showStats]);

  const handleDeleteSelected = useCallback(async () => {
    if (selected.size === 0) return;
    if (!window.confirm(`Delete ${selected.size} memory entries?`)) return;

    try {
      const result = await tauri.deleteVaultEntriesBatch(Array.from(selected));
      toast.success(`Deleted ${result.deleted} entries`);
      setSelected(new Set());
      fetchEntries();
      if (showStats) fetchStats();
    } catch (error) {
      toast.error(`Delete failed: ${error}`);
    }
  }, [fetchEntries, fetchStats, selected, showStats]);

  const handleClearWorkspace = useCallback(async () => {
    if (selectedWorkspace === "__all__") return;
    if (!window.confirm(`Clear all memory entries for ${selectedWorkspace}?`)) {
      return;
    }

    try {
      await tauri.clearWorkspaceVault(selectedWorkspace);
      toast.success(`Cleared workspace ${selectedWorkspace}`);
      setSelected(new Set());
      fetchEntries();
      fetchWorkspaces();
      if (showStats) fetchStats();
    } catch (error) {
      toast.error(`Clear failed: ${error}`);
    }
  }, [fetchEntries, fetchStats, fetchWorkspaces, selectedWorkspace, showStats]);

  const toggleSelect = useCallback((id: string) => {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const toggleSelectAllVisible = useCallback(() => {
    setSelected((current) => {
      const next = new Set(current);
      const allVisibleSelected = filteredEntries.every((entry) => next.has(entry.id));

      if (allVisibleSelected) {
        filteredEntries.forEach((entry) => next.delete(entry.id));
      } else {
        filteredEntries.forEach((entry) => next.add(entry.id));
      }

      return next;
    });
  }, [filteredEntries]);

  const allVisibleSelected =
    filteredEntries.length > 0 &&
    filteredEntries.every((entry) => selected.has(entry.id));

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden">
      <div className="flex flex-1 min-h-0 flex-col gap-3 p-3">
        <Card className={`${PANEL_CLASS} rounded-[24px] px-4 py-3`}>
          <Card.Header className="flex flex-col gap-4 p-0 md:flex-row md:items-center md:justify-between">
            <div className="flex items-start gap-4">
              <div className="flex size-10 items-center justify-center rounded-[16px] border border-violet-500/16 bg-violet-500/10">
                <Brain className="size-4.5 text-violet-500" />
              </div>
              <div className="space-y-1">
                <Card.Title className="text-lg font-semibold tracking-tight">
                  Memory Vault Explorer
                </Card.Title>
                <Card.Description className="text-[13px] text-muted-foreground/85">
                  Encrypted semantic memory across workspaces, sources, and recall patterns.
                </Card.Description>
                <div className="flex flex-wrap gap-1.5 pt-1.5">
                  <Chip
                    className="rounded-full border border-border/50 bg-background/70 dark:bg-background/20"
                    variant="soft"
                    size="sm"
                  >
                    {totalCount} entries
                  </Chip>
                  {selectedWorkspace !== "__all__" ? (
                    <Chip
                      className="rounded-full border border-primary/20 bg-primary/10"
                      variant="soft"
                      size="sm"
                    >
                      {selectedWorkspace}
                    </Chip>
                  ) : null}
                </div>
              </div>
            </div>

            <div className="flex flex-wrap items-center gap-1.5">
              <Button
                variant={showStats ? "primary" : "tertiary"}
                size="sm"
                className={showStats ? BUTTON_ACTIVE_CLASS : BUTTON_BASE_CLASS}
                onPress={() => setShowStats((current) => !current)}
              >
                <Sparkles className="size-4" />
                Stats
              </Button>
              <Button
                variant="secondary"
                size="sm"
                className={BUTTON_BASE_CLASS}
                onPress={handleRefresh}
              >
                <RefreshCw className="size-4" />
                Refresh
              </Button>
            </div>
          </Card.Header>
        </Card>

        {showStats && stats ? (
          <div className="grid gap-2.5 md:grid-cols-2 xl:grid-cols-5">
            <StatTile
              label="Total Entries"
              value={String(stats.total_entries)}
              hint={selectedWorkspace === "__all__" ? "Global vault" : "Scoped workspace"}
            />
            <StatTile
              label="Embeddings"
              value={`${stats.has_embeddings}/${stats.total_entries}`}
              hint={`${stats.missing_embeddings} missing`}
            />
            <StatTile
              label="Oldest"
              value={stats.oldest_entry ? timeAgo(stats.oldest_entry) : "None"}
            />
            <StatTile
              label="Newest"
              value={stats.newest_entry ? timeAgo(stats.newest_entry) : "None"}
            />
            <Card className={`${PANEL_CLASS} rounded-[18px] px-3 py-2.5 shadow-none`}>
              <Card.Header className="flex flex-col gap-2 p-0">
                <p className="text-[10px] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                  Sensitivity
                </p>
                <div className="flex flex-wrap gap-1.5">
                  {Object.entries(stats.entries_by_sensitivity).map(([key, value]) => (
                    <Chip
                      key={key}
                      className={`rounded-full border px-2 ${SENSITIVITY_STYLES[key] ?? ""}`}
                      variant="soft"
                      size="sm"
                    >
                      {key}: {value}
                    </Chip>
                  ))}
                </div>
              </Card.Header>
            </Card>
          </div>
        ) : null}

        <Card className={`${PANEL_CLASS} rounded-[24px] px-3 py-3`}>
          <Card.Content className="grid gap-2.5 p-0 md:grid-cols-2 xl:grid-cols-[1.1fr_1fr_1fr_1.15fr_0.9fr]">
            <Select
              className="w-full"
              selectedKey={selectedWorkspace}
              placeholder="All workspaces"
              onSelectionChange={(selection) => {
                const value = selectionToValue(selection);
                setSelectedWorkspace(value ?? "__all__");
              }}
            >
              <Select.Trigger className="h-9 rounded-[16px] border border-border/50 bg-background/75 px-3.5 text-sm dark:bg-background/18">
                <Select.Value />
                <Select.Indicator />
              </Select.Trigger>
              <Select.Popover className={POPOVER_CLASS}>
                <ListBox className={LISTBOX_CLASS}>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="__all__" textValue="All workspaces">
                    All workspaces
                  </ListBox.Item>
                  {workspaces.map((workspace) => (
                    <ListBox.Item
                      className={LISTBOX_ITEM_CLASS}
                      key={workspace.workspace_id}
                      id={workspace.workspace_id}
                      textValue={workspace.workspace_id}
                    >
                      {workspace.workspace_id} ({workspace.entry_count})
                    </ListBox.Item>
                  ))}
                </ListBox>
              </Select.Popover>
            </Select>

            <Select
              className="w-full"
              selectedKey={selectedSensitivity}
              placeholder="All sensitivities"
              onSelectionChange={(selection) => {
                const value = selectionToValue(selection);
                setSelectedSensitivity(value ?? "__all__");
              }}
            >
              <Select.Trigger className="h-9 rounded-[16px] border border-border/50 bg-background/75 px-3.5 text-sm dark:bg-background/18">
                <Select.Value />
                <Select.Indicator />
              </Select.Trigger>
              <Select.Popover className={POPOVER_CLASS}>
                <ListBox className={LISTBOX_CLASS}>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="__all__" textValue="All sensitivities">
                    All sensitivities
                  </ListBox.Item>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="public" textValue="Public">
                    Public
                  </ListBox.Item>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="internal" textValue="Internal">
                    Internal
                  </ListBox.Item>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="confidential" textValue="Confidential">
                    Confidential
                  </ListBox.Item>
                </ListBox>
              </Select.Popover>
            </Select>

            <Select
              className="w-full"
              selectedKey={selectedCategory}
              placeholder="All categories"
              onSelectionChange={(selection) => {
                const value = selectionToValue(selection);
                setSelectedCategory(value ?? "__all__");
              }}
            >
              <Select.Trigger className="h-9 rounded-[16px] border border-border/50 bg-background/75 px-3.5 text-sm dark:bg-background/18">
                <Select.Value />
                <Select.Indicator />
              </Select.Trigger>
              <Select.Popover className={POPOVER_CLASS}>
                <ListBox className={LISTBOX_CLASS}>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="__all__" textValue="All categories">
                    All categories
                  </ListBox.Item>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="preference" textValue="Preference">
                    Preference
                  </ListBox.Item>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="correction" textValue="Correction">
                    Correction
                  </ListBox.Item>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="fact" textValue="Fact">
                    Fact
                  </ListBox.Item>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="procedure" textValue="Procedure">
                    Procedure
                  </ListBox.Item>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="observation" textValue="Observation">
                    Observation
                  </ListBox.Item>
                </ListBox>
              </Select.Popover>
            </Select>

            <div className="relative">
              <Search className="pointer-events-none absolute left-4 top-1/2 z-10 size-4 -translate-y-1/2 text-muted-foreground/70" />
              <Input
                className="h-9 rounded-[16px] border border-border/50 bg-background/75 pl-10 text-sm dark:bg-background/18"
                placeholder="Filter by source prefix"
                value={sourceFilter}
                onChange={(event) => setSourceFilter(event.target.value)}
              />
            </div>

            <Select
              className="w-full"
              selectedKey={sortBy}
              placeholder="Sort order"
              onSelectionChange={(selection) => {
                const value = selectionToValue(selection);
                setSortBy(value ?? "created_at");
              }}
            >
              <Select.Trigger className="h-9 rounded-[16px] border border-border/50 bg-background/75 px-3.5 text-sm dark:bg-background/18">
                <Select.Value />
                <Select.Indicator />
              </Select.Trigger>
              <Select.Popover className={POPOVER_CLASS}>
                <ListBox className={LISTBOX_CLASS}>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="created_at" textValue="Created at">
                    Created at
                  </ListBox.Item>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="last_accessed" textValue="Last accessed">
                    Last accessed
                  </ListBox.Item>
                  <ListBox.Item className={LISTBOX_ITEM_CLASS} id="access_count" textValue="Access count">
                    Access count
                  </ListBox.Item>
                </ListBox>
              </Select.Popover>
            </Select>
          </Card.Content>
        </Card>

        <Card className={`${PANEL_CLASS} flex min-h-0 flex-1 rounded-[26px] px-3 py-3`}>
          <Card.Header className="flex flex-col gap-2.5 border-b border-border/40 p-0 pb-3 md:flex-row md:items-center md:justify-between">
            <div className="space-y-1">
              <Card.Title className="text-sm font-semibold text-foreground">
                Vault Records
              </Card.Title>
              <Card.Description className="text-[13px] text-muted-foreground/80">
                Review semantic memory slices, provenance, and retrieval metadata.
              </Card.Description>
            </div>

            <div className="flex flex-wrap items-center gap-1.5">
              <Button
                variant={allVisibleSelected ? "primary" : "tertiary"}
                size="sm"
                className={allVisibleSelected ? BUTTON_ACTIVE_CLASS : BUTTON_BASE_CLASS}
                onPress={toggleSelectAllVisible}
                isDisabled={filteredEntries.length === 0}
              >
                <Sparkles className="size-4" />
                {allVisibleSelected ? "Unselect visible" : "Select visible"}
              </Button>
              <Chip
                className="rounded-full border border-border/50 bg-background/70 dark:bg-background/20"
                variant="soft"
                size="sm"
              >
                {selected.size} selected
              </Chip>
            </div>
          </Card.Header>

          <Card.Content className="min-h-0 flex-1 p-0 pt-3">
            {loading && filteredEntries.length === 0 ? (
              <div className="flex h-full items-center justify-center">
                <div className="flex flex-col items-center gap-3 text-muted-foreground">
                  <Spinner size="lg" />
                  <p className="text-sm">Loading vault records...</p>
                </div>
              </div>
            ) : filteredEntries.length === 0 ? (
              <div className="flex h-full items-center justify-center">
                <div className="max-w-md space-y-1.5 px-6 text-center">
                  <p className="text-base font-semibold">No memory entries found</p>
                  <p className="text-sm text-muted-foreground/80">
                    Adjust the filters or create new semantic memories to populate this vault.
                  </p>
                </div>
              </div>
            ) : (
              <div className="h-full overflow-y-auto pr-1">
                <div className="space-y-2.5">
                  {filteredEntries.map((entry) => (
                    <EntryCard
                      key={entry.id}
                      entry={entry}
                      expanded={expandedId === entry.id}
                      selected={selected.has(entry.id)}
                      onToggleExpanded={() =>
                        setExpandedId((current) =>
                          current === entry.id ? null : entry.id,
                        )
                      }
                      onToggleSelected={() => toggleSelect(entry.id)}
                    />
                  ))}
                </div>
              </div>
            )}
          </Card.Content>
        </Card>

        <Card className={`${PANEL_CLASS} rounded-[20px] px-3 py-2.5`}>
          <Card.Content className="flex flex-col gap-2.5 p-0 md:flex-row md:items-center md:justify-between">
            <div className="flex flex-wrap items-center gap-1.5">
              <Button
                variant="danger"
                size="sm"
                className={BUTTON_DANGER_CLASS}
                isDisabled={selected.size === 0}
                onPress={handleDeleteSelected}
              >
                <Trash2 className="size-4" />
                Delete ({selected.size})
              </Button>

              {selectedWorkspace !== "__all__" ? (
                <Button
                  variant="outline"
                  size="sm"
                  className={BUTTON_BASE_CLASS}
                  onPress={handleClearWorkspace}
                >
                  <TriangleAlert className="size-4" />
                  Clear Workspace
                </Button>
              ) : null}
            </div>

            <div className="flex flex-wrap items-center gap-2.5 text-sm text-muted-foreground">
              <span>
                {totalCount === 0
                  ? "0 entries"
                  : `${offset + 1}-${Math.min(offset + PAGE_SIZE, totalCount)} of ${totalCount}`}
              </span>
              <div className="flex items-center gap-1.5">
                <Button
                  isIconOnly
                  variant="secondary"
                  size="sm"
                  className={BUTTON_BASE_CLASS}
                  isDisabled={offset === 0}
                  onPress={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
                >
                  <ChevronLeft className="size-4" />
                </Button>
                <Chip
                  className="rounded-full border border-border/50 bg-background/70 px-3 dark:bg-background/20"
                  variant="soft"
                  size="sm"
                >
                  {currentPage}/{totalPages}
                </Chip>
                <Button
                  isIconOnly
                  variant="secondary"
                  size="sm"
                  className={BUTTON_BASE_CLASS}
                  isDisabled={offset + PAGE_SIZE >= totalCount}
                  onPress={() => setOffset(offset + PAGE_SIZE)}
                >
                  <ChevronRight className="size-4" />
                </Button>
              </div>
            </div>
          </Card.Content>
        </Card>
      </div>
    </div>
  );
}
