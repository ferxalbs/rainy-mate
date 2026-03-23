import {
  Loader2,
  Brain,
  Terminal,
  Sparkles,
  Zap,
  Globe,
  Eye,
  MousePointer,
  Radio,
  Trash2,
  BookMarked,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

export type NeuralState =
  | "thinking"
  | "planning"
  | "executing"
  | "creating"
  | "reading"
  | "observing"
  | "browsing"
  | "communicating"
  | "pruning"
  | "remembering"
  | "idle";

// Tool to State Mapping
export const TOOL_STATE_MAP: Record<string, NeuralState> = {
  // Reading (Web)
  web_search: "reading",
  fetch_web_content: "reading",
  read_url: "reading",
  google_search: "reading",
  brave_search: "reading",

  // Creating (Filesystem Write)
  write_file: "creating",
  append_file: "creating",
  mkdir: "creating",
  move_file: "creating",

  // Pruning (Filesystem Delete)
  delete_file: "pruning",

  // Observing (Filesystem Read)
  read_file: "observing",
  read_many_files: "observing",
  list_files: "observing",
  search_files: "observing",
  file_exists: "observing",
  get_file_info: "observing",
  ingest_document: "observing",

  // Browsing (Browser Interaction)
  browse_url: "browsing",
  open_new_tab: "browsing",
  click_element: "browsing",
  wait_for_selector: "browsing",
  type_text: "browsing",
  submit_form: "browsing",
  go_back: "browsing",
  screenshot: "browsing",
  get_page_content: "browsing",
  get_page_snapshot: "browsing",
  extract_links: "browsing",

  // Communicating (API/Network)
  http_get_json: "communicating",
  http_post_json: "communicating",

  // Executing (Shell/System)
  execute_command: "executing",
  git_status: "executing",
  git_diff: "executing",
  git_log: "executing",
  list_installed_skills: "observing",
  install_local_skill: "creating",
  install_skill_from_atm: "communicating",
  set_installed_skill_enabled: "creating",
  remove_installed_skill: "pruning",

  // Memory tools
  save_memory: "remembering",
  recall_memory: "remembering",

  // IRONMILL — Document Generation (KINGFALL Phase 1)
  pdf_create: "creating",
  excel_write: "creating",
  docx_create: "creating",
  archive_create: "creating",
  pdf_read: "observing",
  excel_read: "observing",
};

/** Human-readable display names for raw tool function names */
const TOOL_DISPLAY_NAMES: Record<string, string> = {
  read_file: "Reading File",
  read_many_files: "Reading Files",
  write_file: "Writing File",
  append_file: "Appending to File",
  delete_file: "Deleting File",
  list_files: "Listing Files",
  search_files: "Searching Files",
  file_exists: "Checking File",
  get_file_info: "Inspecting File",
  ingest_document: "Ingesting Document",
  mkdir: "Creating Directory",
  move_file: "Moving File",
  web_search: "Searching the Web",
  google_search: "Searching Google",
  brave_search: "Searching the Web",
  fetch_web_content: "Fetching URL",
  read_url: "Reading URL",
  browse_url: "Browsing URL",
  execute_command: "Running Command",
  git_status: "Checking Git Status",
  git_diff: "Reading Git Diff",
  git_log: "Reading Git Log",
  http_get_json: "Fetching API",
  http_post_json: "Calling API",
  screenshot: "Taking Screenshot",
  list_installed_skills: "Listing Installed Skills",
  install_local_skill: "Installing Local Skill",
  install_skill_from_atm: "Installing Skill From ATM",
  set_installed_skill_enabled: "Updating Skill State",
  remove_installed_skill: "Removing Installed Skill",
  save_memory: "Saving to Memory",
  recall_memory: "Recalling Memory",
  // IRONMILL — Document Generation (KINGFALL Phase 1)
  pdf_create: "Generating PDF Document",
  pdf_read: "Reading PDF Document",
  excel_write: "Writing Spreadsheet",
  excel_read: "Reading Spreadsheet",
  docx_create: "Generating Word Document",
  archive_create: "Bundling Archive",
};

/** Resolves a function name to a human-readable display name */
export const getToolDisplayName = (functionName: string): string => {
  if (functionName.startsWith("mcp_")) {
    const parts = functionName.replace(/^mcp_/, "").split("_");
    const server = parts.shift() || "server";
    const tool = parts.join("_") || "tool";
    return `MCP ${server}: ${tool.replace(/_/g, " ")}`;
  }
  return TOOL_DISPLAY_NAMES[functionName] || functionName.replace(/_/g, " ");
};

/** Resolves a function name to the corresponding NeuralState */
export const resolveNeuralState = (functionName: string): NeuralState => {
  if (functionName.startsWith("mcp_")) return "communicating";
  return TOOL_STATE_MAP[functionName] || "executing";
};

export interface NeuralStateConfig {
  icon: LucideIcon;
  text: string;
  color: string;
  bgColor: string;
}

export const getNeuralStateConfig = (state: NeuralState): NeuralStateConfig => {
  switch (state) {
    case "thinking":
      return {
        icon: Brain,
        text: "Analyzing Neural Pathways...",
        color: "text-purple-500",
        bgColor: "bg-purple-500/10",
      };
    case "planning":
      return {
        icon: Zap,
        text: "Formulating Execution Strategy...",
        color: "text-amber-500",
        bgColor: "bg-amber-500/10",
      };
    case "creating":
      return {
        icon: Sparkles,
        text: "Generating Digital Assets...",
        color: "text-pink-500",
        bgColor: "bg-pink-500/10",
      };
    case "pruning":
      return {
        icon: Trash2,
        text: "Pruning Obsolete Data...",
        color: "text-red-500",
        bgColor: "bg-red-500/10",
      };
    case "remembering":
      return {
        icon: BookMarked,
        text: "Writing to Long-Term Memory...",
        color: "text-violet-500",
        bgColor: "bg-violet-500/10",
      };
    case "reading":
      return {
        icon: Globe,
        text: "Absorbing Global Knowledge...",
        color: "text-blue-500",
        bgColor: "bg-blue-500/10",
      };
    case "observing":
      return {
        icon: Eye,
        text: "Scanning Local Environment...",
        color: "text-emerald-500",
        bgColor: "bg-emerald-500/10",
      };
    case "browsing":
      return {
        icon: MousePointer,
        text: "Navigating Cyber-Space...",
        color: "text-orange-500",
        bgColor: "bg-orange-500/10",
      };
    case "communicating":
      return {
        icon: Radio,
        text: "Establishing Uplink...",
        color: "text-indigo-500",
        bgColor: "bg-indigo-500/10",
      };
    case "executing":
      return {
        icon: Terminal,
        text: "Executing Protocols...",
        color: "text-cyan-500",
        bgColor: "bg-cyan-500/10",
      };
    default:
      return {
        icon: Loader2,
        text: "Processing...",
        color: "text-muted-foreground",
        bgColor: "bg-muted/10",
      };
  }
};
