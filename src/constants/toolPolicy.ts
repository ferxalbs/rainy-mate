import { AirlockLevel, AirlockLevels } from "../services/tauri";

export type ToolSkillName = "filesystem" | "shell" | "web" | "browser";

type ToolPolicy = {
  skill: ToolSkillName;
  airlockLevel: AirlockLevel;
};

const DEFAULT_POLICY: ToolPolicy = {
  skill: "filesystem",
  airlockLevel: AirlockLevels.Sensitive,
};

const TOOL_POLICY_MAP: Record<string, ToolPolicy> = {
  // Level 0: read-only
  read_file: { skill: "filesystem", airlockLevel: AirlockLevels.Safe },
  read_many_files: { skill: "filesystem", airlockLevel: AirlockLevels.Safe },
  list_files: { skill: "filesystem", airlockLevel: AirlockLevels.Safe },
  search_files: { skill: "filesystem", airlockLevel: AirlockLevels.Safe },
  file_exists: { skill: "filesystem", airlockLevel: AirlockLevels.Safe },
  get_file_info: { skill: "filesystem", airlockLevel: AirlockLevels.Safe },
  read_file_chunk: { skill: "filesystem", airlockLevel: AirlockLevels.Safe },
  git_status: { skill: "shell", airlockLevel: AirlockLevels.Safe },
  git_diff: { skill: "shell", airlockLevel: AirlockLevels.Safe },
  git_log: { skill: "shell", airlockLevel: AirlockLevels.Safe },
  web_search: { skill: "web", airlockLevel: AirlockLevels.Safe },
  read_web_page: { skill: "web", airlockLevel: AirlockLevels.Safe },
  http_get_json: { skill: "web", airlockLevel: AirlockLevels.Safe },
  screenshot: { skill: "browser", airlockLevel: AirlockLevels.Safe },
  get_page_content: { skill: "browser", airlockLevel: AirlockLevels.Safe },
  get_page_snapshot: { skill: "browser", airlockLevel: AirlockLevels.Safe },
  wait_for_selector: { skill: "browser", airlockLevel: AirlockLevels.Safe },
  extract_links: { skill: "browser", airlockLevel: AirlockLevels.Safe },

  // Level 1: state-changing but non-destructive
  write_file: { skill: "filesystem", airlockLevel: AirlockLevels.Sensitive },
  append_file: { skill: "filesystem", airlockLevel: AirlockLevels.Sensitive },
  mkdir: { skill: "filesystem", airlockLevel: AirlockLevels.Sensitive },
  browse_url: { skill: "browser", airlockLevel: AirlockLevels.Sensitive },
  open_new_tab: { skill: "browser", airlockLevel: AirlockLevels.Sensitive },
  click_element: { skill: "browser", airlockLevel: AirlockLevels.Sensitive },
  navigate: { skill: "browser", airlockLevel: AirlockLevels.Sensitive },
  go_back: { skill: "browser", airlockLevel: AirlockLevels.Sensitive },
  type_text: { skill: "browser", airlockLevel: AirlockLevels.Sensitive },

  // Level 2: destructive or command execution
  execute_command: { skill: "shell", airlockLevel: AirlockLevels.Dangerous },
  http_post_json: { skill: "web", airlockLevel: AirlockLevels.Dangerous },
  submit_form: { skill: "browser", airlockLevel: AirlockLevels.Dangerous },
  delete_file: { skill: "filesystem", airlockLevel: AirlockLevels.Dangerous },
  move_file: { skill: "filesystem", airlockLevel: AirlockLevels.Dangerous },
};

export const KNOWN_TOOL_NAMES = Object.keys(TOOL_POLICY_MAP).sort((a, b) =>
  a.localeCompare(b),
);

export function getToolPolicy(toolName: string): ToolPolicy {
  return TOOL_POLICY_MAP[toolName] ?? DEFAULT_POLICY;
}

export function getToolAirlockLevel(toolName: string): AirlockLevel {
  return getToolPolicy(toolName).airlockLevel;
}

export function getToolSkill(toolName: string): ToolSkillName {
  return getToolPolicy(toolName).skill;
}
