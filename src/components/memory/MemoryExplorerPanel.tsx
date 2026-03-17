import { useState, useEffect, useCallback, useMemo } from "react";
import {
  Brain,
  Trash2,
  ChevronLeft,
  ChevronRight,
  BarChart3,
  Shield,
  Clock,
  Hash,
  Search,
  RefreshCw,
  AlertTriangle,
} from "lucide-react";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Badge } from "../ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";
import { toast } from "sonner";
import * as tauri from "../../services/tauri";
import type {
  DecryptedMemoryEntry,
  WorkspaceSummary,
  VaultDetailedStats,
} from "../../types/memory";

const PAGE_SIZE = 20;

const SENSITIVITY_COLORS: Record<string, string> = {
  public: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400",
  internal: "bg-blue-500/15 text-blue-600 dark:text-blue-400",
  confidential: "bg-red-500/15 text-red-600 dark:text-red-400",
};

const CATEGORY_COLORS: Record<string, string> = {
  preference: "bg-purple-500/15 text-purple-600 dark:text-purple-400",
  correction: "bg-orange-500/15 text-orange-600 dark:text-orange-400",
  fact: "bg-blue-500/15 text-blue-600 dark:text-blue-400",
  procedure: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400",
  observation: "bg-zinc-500/15 text-zinc-600 dark:text-zinc-400",
};

function timeAgo(ts: number): string {
  const diff = Math.floor(Date.now() / 1000 - ts);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  if (diff < 604800) return `${Math.floor(diff / 86400)}d ago`;
  return `${Math.floor(diff / 604800)}w ago`;
}

export function MemoryExplorerPanel() {
  const [entries, setEntries] = useState<DecryptedMemoryEntry[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(false);

  // Filters
  const [workspaces, setWorkspaces] = useState<WorkspaceSummary[]>([]);
  const [selectedWorkspace, setSelectedWorkspace] = useState<string>("__all__");
  const [selectedSensitivity, setSelectedSensitivity] =
    useState<string>("__all__");
  const [sourceFilter, setSourceFilter] = useState("");
  const [selectedCategory, setSelectedCategory] = useState<string>("__all__");
  const [sortBy, setSortBy] = useState("created_at");

  // Stats
  const [stats, setStats] = useState<VaultDetailedStats | null>(null);
  const [showStats, setShowStats] = useState(false);

  // Selection
  const [selected, setSelected] = useState<Set<string>>(new Set());

  // Expanded entry
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
    } catch (e) {
      toast.error(`Failed to load entries: ${e}`);
    } finally {
      setLoading(false);
    }
  }, [selectedWorkspace, selectedSensitivity, sourceFilter, sortBy, offset]);

  const fetchWorkspaces = useCallback(async () => {
    try {
      const ws = await tauri.listMemoryWorkspaces();
      setWorkspaces(ws);
    } catch {
      /* ignore */
    }
  }, []);

  const fetchStats = useCallback(async () => {
    try {
      const s = await tauri.getVaultDetailedStats(
        selectedWorkspace === "__all__" ? undefined : selectedWorkspace,
      );
      setStats(s);
    } catch {
      /* ignore */
    }
  }, [selectedWorkspace]);

  useEffect(() => {
    fetchWorkspaces();
  }, [fetchWorkspaces]);

  useEffect(() => {
    fetchEntries();
  }, [fetchEntries]);

  useEffect(() => {
    if (showStats) fetchStats();
  }, [showStats, fetchStats]);

  // Reset offset when filters change
  useEffect(() => {
    setOffset(0);
    setSelected(new Set());
  }, [selectedWorkspace, selectedSensitivity, selectedCategory, sourceFilter, sortBy]);

  const handleDeleteSelected = async () => {
    if (selected.size === 0) return;
    const ids = Array.from(selected);
    try {
      const result = await tauri.deleteVaultEntriesBatch(ids);
      toast.success(`Deleted ${result.deleted} entries`);
      setSelected(new Set());
      fetchEntries();
      if (showStats) fetchStats();
    } catch (e) {
      toast.error(`Delete failed: ${e}`);
    }
  };

  const handleClearWorkspace = async () => {
    if (selectedWorkspace === "__all__") return;
    try {
      await tauri.clearWorkspaceVault(selectedWorkspace);
      toast.success(`Cleared workspace ${selectedWorkspace}`);
      setSelected(new Set());
      fetchEntries();
      fetchWorkspaces();
      if (showStats) fetchStats();
    } catch (e) {
      toast.error(`Clear failed: ${e}`);
    }
  };

  const toggleSelect = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleSelectAll = () => {
    if (selected.size === entries.length) {
      setSelected(new Set());
    } else {
      setSelected(new Set(entries.map((e) => e.id)));
    }
  };

  const filteredEntries = useMemo(() => {
    if (selectedCategory === "__all__") return entries;
    return entries.filter(
      (e) => (e.metadata["_category"] ?? "observation") === selectedCategory,
    );
  }, [entries, selectedCategory]);

  const totalPages = Math.ceil(totalCount / PAGE_SIZE);
  const currentPage = Math.floor(offset / PAGE_SIZE) + 1;

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border/40 px-6 py-4">
        <div className="flex items-center gap-3">
          <div className="flex size-10 items-center justify-center rounded-xl bg-purple-500/10">
            <Brain className="size-5 text-purple-500" />
          </div>
          <div>
            <h1 className="text-lg font-semibold">Memory Vault Explorer</h1>
            <p className="text-xs text-muted-foreground">
              {totalCount} entries
              {selectedWorkspace !== "__all__" &&
                ` in ${selectedWorkspace}`}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setShowStats(!showStats)}
          >
            <BarChart3 className="mr-1.5 size-3.5" />
            Stats
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              fetchEntries();
              fetchWorkspaces();
              if (showStats) fetchStats();
            }}
          >
            <RefreshCw className="mr-1.5 size-3.5" />
            Refresh
          </Button>
        </div>
      </div>

      {/* Stats Bar */}
      {showStats && stats && (
        <div className="border-b border-border/40 bg-muted/30 px-6 py-3">
          <div className="flex flex-wrap gap-6 text-xs">
            <div>
              <span className="text-muted-foreground">Total:</span>{" "}
              <span className="font-medium">{stats.total_entries}</span>
            </div>
            <div>
              <span className="text-muted-foreground">Embeddings:</span>{" "}
              <span className="font-medium">
                {stats.has_embeddings}/{stats.has_embeddings + stats.missing_embeddings}
              </span>
              <span className="ml-1 text-muted-foreground">
                ({stats.total_entries > 0
                  ? Math.round(
                      (stats.has_embeddings / stats.total_entries) * 100,
                    )
                  : 0}
                %)
              </span>
            </div>
            {stats.oldest_entry && (
              <div>
                <span className="text-muted-foreground">Oldest:</span>{" "}
                <span className="font-medium">
                  {timeAgo(stats.oldest_entry)}
                </span>
              </div>
            )}
            {stats.newest_entry && (
              <div>
                <span className="text-muted-foreground">Newest:</span>{" "}
                <span className="font-medium">
                  {timeAgo(stats.newest_entry)}
                </span>
              </div>
            )}
            {Object.entries(stats.entries_by_sensitivity).map(([k, v]) => (
              <div key={k}>
                <Badge variant="outline" className={`text-[10px] ${SENSITIVITY_COLORS[k] ?? ""}`}>
                  {k}: {v}
                </Badge>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Filter Bar */}
      <div className="flex flex-wrap items-center gap-2 border-b border-border/40 px-6 py-3">
        <Select value={selectedWorkspace} onValueChange={(v) => setSelectedWorkspace(v ?? "__all__")}>
          <SelectTrigger className="h-8 w-[180px] text-xs">
            <SelectValue placeholder="All workspaces" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="__all__">All workspaces</SelectItem>
            {workspaces.map((ws) => (
              <SelectItem key={ws.workspace_id} value={ws.workspace_id}>
                {ws.workspace_id} ({ws.entry_count})
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <Select
          value={selectedSensitivity}
          onValueChange={(v) => setSelectedSensitivity(v ?? "__all__")}
        >
          <SelectTrigger className="h-8 w-[140px] text-xs">
            <SelectValue placeholder="All levels" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="__all__">All levels</SelectItem>
            <SelectItem value="public">Public</SelectItem>
            <SelectItem value="internal">Internal</SelectItem>
            <SelectItem value="confidential">Confidential</SelectItem>
          </SelectContent>
        </Select>

        <Select
          value={selectedCategory}
          onValueChange={(v) => setSelectedCategory(v ?? "__all__")}
        >
          <SelectTrigger className="h-8 w-[140px] text-xs">
            <SelectValue placeholder="All categories" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="__all__">All categories</SelectItem>
            <SelectItem value="preference">Preference</SelectItem>
            <SelectItem value="correction">Correction</SelectItem>
            <SelectItem value="fact">Fact</SelectItem>
            <SelectItem value="procedure">Procedure</SelectItem>
            <SelectItem value="observation">Observation</SelectItem>
          </SelectContent>
        </Select>

        <div className="relative">
          <Search className="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Filter source..."
            value={sourceFilter}
            onChange={(e) => setSourceFilter(e.target.value)}
            className="h-8 w-[160px] pl-7 text-xs"
          />
        </div>

        <Select value={sortBy} onValueChange={(v) => setSortBy(v ?? "created_at")}>
          <SelectTrigger className="h-8 w-[140px] text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="created_at">Created</SelectItem>
            <SelectItem value="last_accessed">Last accessed</SelectItem>
            <SelectItem value="access_count">Access count</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Entry List */}
      <div className="flex-1 overflow-y-auto">
        {loading && filteredEntries.length === 0 ? (
          <div className="flex items-center justify-center py-12 text-muted-foreground">
            Loading...
          </div>
        ) : filteredEntries.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
            <Brain className="mb-2 size-8 opacity-30" />
            <p className="text-sm">No memory entries found</p>
          </div>
        ) : (
          <div className="divide-y divide-border/30">
            {/* Select all row */}
            <div className="flex items-center gap-2 px-6 py-1.5 text-xs text-muted-foreground">
              <input
                type="checkbox"
                checked={selected.size === filteredEntries.length && filteredEntries.length > 0}
                onChange={toggleSelectAll}
                className="size-3.5 rounded"
              />
              <span>
                {selected.size > 0
                  ? `${selected.size} selected`
                  : "Select all"}
              </span>
            </div>

            {filteredEntries.map((entry) => (
              <div
                key={entry.id}
                className={`group cursor-pointer px-6 py-3 transition-colors hover:bg-muted/30 ${
                  selected.has(entry.id) ? "bg-primary/5" : ""
                }`}
                onClick={() => setExpandedId(expandedId === entry.id ? null : entry.id)}
              >
                <div className="flex items-start gap-3">
                  <input
                    type="checkbox"
                    checked={selected.has(entry.id)}
                    onChange={(e) => {
                      e.stopPropagation();
                      toggleSelect(entry.id);
                    }}
                    onClick={(e) => e.stopPropagation()}
                    className="mt-1 size-3.5 rounded"
                  />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm">
                      {entry.content.slice(0, 200)}
                      {entry.content.length > 200 && "..."}
                    </p>
                    <div className="mt-1.5 flex flex-wrap items-center gap-2">
                      {entry.metadata["_category"] && (
                        <Badge
                          variant="outline"
                          className={`text-[10px] ${CATEGORY_COLORS[entry.metadata["_category"]] ?? ""}`}
                        >
                          {entry.metadata["_category"]}
                        </Badge>
                      )}
                      {entry.metadata["_importance"] && (
                        <div className="flex items-center gap-1" title={`Importance: ${Math.round(parseFloat(entry.metadata["_importance"]) * 100)}%`}>
                          <div className="h-1 w-12 rounded-full bg-muted">
                            <div
                              className="h-1 rounded-full bg-primary/60"
                              style={{ width: `${parseFloat(entry.metadata["_importance"]) * 100}%` }}
                            />
                          </div>
                        </div>
                      )}
                      <Badge
                        variant="outline"
                        className={`text-[10px] ${SENSITIVITY_COLORS[entry.sensitivity] ?? ""}`}
                      >
                        <Shield className="mr-0.5 size-2.5" />
                        {entry.sensitivity}
                      </Badge>
                      <span className="text-[10px] text-muted-foreground">
                        <Clock className="mr-0.5 inline size-2.5" />
                        {timeAgo(entry.created_at)}
                      </span>
                      <span className="text-[10px] text-muted-foreground">
                        <Hash className="mr-0.5 inline size-2.5" />
                        {entry.access_count}
                      </span>
                      <span className="text-[10px] text-muted-foreground">
                        {entry.source}
                      </span>
                      {entry.tags.slice(0, 3).map((tag) => (
                        <Badge
                          key={tag}
                          variant="secondary"
                          className="text-[10px]"
                        >
                          {tag}
                        </Badge>
                      ))}
                      {entry.tags.length > 3 && (
                        <span className="text-[10px] text-muted-foreground">
                          +{entry.tags.length - 3}
                        </span>
                      )}
                    </div>
                  </div>
                </div>

                {/* Expanded detail */}
                {expandedId === entry.id && (
                  <div className="mt-3 ml-7 space-y-2 rounded-lg border border-border/40 bg-muted/20 p-3 text-xs">
                    <div>
                      <span className="font-medium text-muted-foreground">
                        Full content:
                      </span>
                      <p className="mt-1 whitespace-pre-wrap break-words">
                        {entry.content}
                      </p>
                    </div>
                    <div className="flex flex-wrap gap-4 text-muted-foreground">
                      <span>ID: {entry.id.slice(0, 12)}...</span>
                      <span>Workspace: {entry.workspace_id}</span>
                      <span>
                        Embedding:{" "}
                        {entry.embedding_model ?? "none"}{" "}
                        ({entry.embedding_dim ?? 0}d)
                      </span>
                      <span>
                        Last accessed: {timeAgo(entry.last_accessed)}
                      </span>
                    </div>
                    {entry.tags.length > 0 && (
                      <div className="flex flex-wrap gap-1">
                        {entry.tags.map((tag) => (
                          <Badge
                            key={tag}
                            variant="secondary"
                            className="text-[10px]"
                          >
                            {tag}
                          </Badge>
                        ))}
                      </div>
                    )}
                    {Object.keys(entry.metadata).length > 0 && (
                      <div>
                        <span className="font-medium text-muted-foreground">
                          Metadata:
                        </span>
                        <div className="mt-1 flex flex-wrap gap-2">
                          {Object.entries(entry.metadata).map(([k, v]) => (
                            <span key={k} className="text-muted-foreground">
                              {k}={v}
                            </span>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Footer: Pagination + Bulk Actions */}
      <div className="flex items-center justify-between border-t border-border/40 px-6 py-3">
        <div className="flex items-center gap-2">
          <Button
            variant="destructive"
            size="sm"
            disabled={selected.size === 0}
            onClick={handleDeleteSelected}
          >
            <Trash2 className="mr-1.5 size-3.5" />
            Delete ({selected.size})
          </Button>
          {selectedWorkspace !== "__all__" && (
            <Button
              variant="outline"
              size="sm"
              onClick={handleClearWorkspace}
            >
              <AlertTriangle className="mr-1.5 size-3.5" />
              Clear Workspace
            </Button>
          )}
        </div>

        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span>
            {offset + 1}-{Math.min(offset + PAGE_SIZE, totalCount)} of{" "}
            {totalCount}
          </span>
          <Button
            variant="outline"
            size="icon"
            className="size-7"
            disabled={offset === 0}
            onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
          >
            <ChevronLeft className="size-3.5" />
          </Button>
          <span>
            {currentPage}/{totalPages || 1}
          </span>
          <Button
            variant="outline"
            size="icon"
            className="size-7"
            disabled={offset + PAGE_SIZE >= totalCount}
            onClick={() => setOffset(offset + PAGE_SIZE)}
          >
            <ChevronRight className="size-3.5" />
          </Button>
        </div>
      </div>
    </div>
  );
}
