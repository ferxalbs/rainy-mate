use crate::models::neural::AirlockLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSkill {
    Filesystem,
    Browser,
    Shell,
    Web,
    Memory,
    RemoteSession,
    /// IRONMILL — document generation and reading (KINGFALL Phase 1)
    Documents,
    Workspace,
    /// Beam RPC + Secure Local Signing Bridge
    Evm,
}

impl ToolSkill {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Filesystem => "filesystem",
            Self::Browser => "browser",
            Self::Shell => "shell",
            Self::Web => "web",
            Self::Memory => "memory",
            Self::RemoteSession => "remote_session",
            Self::Documents => "documents",
            Self::Workspace => "workspace",
            Self::Evm => "evm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolPolicy {
    pub skill: ToolSkill,
    pub airlock_level: AirlockLevel,
}

/// Canonical tool policy for desktop runtime.
/// Unknown tools have no policy entry and must be denied by caller logic.
pub fn get_tool_policy(function_name: &str) -> Option<ToolPolicy> {
    let policy = match function_name {
        // Level 0: read-only
        "read_file"
        | "ingest_document"
        | "read_many_files"
        | "list_files"
        | "list_files_detailed"
        | "search_files"
        | "file_exists"
        | "get_file_info"
        | "read_file_chunk"
        | "git_status"
        | "git_diff"
        | "git_log"
        | "git_show"
        | "git_branch_list"
        | "web_search"
        | "read_web_page"
        | "http_get_json"
        | "http_get_text"
        | "screenshot"
        | "get_page_content"
        | "get_page_snapshot"
        | "wait_for_selector"
        | "extract_links" => ToolPolicy {
            skill: match function_name {
                "web_search" | "read_web_page" | "http_get_json" | "http_get_text" => {
                    ToolSkill::Web
                }
                "screenshot" | "get_page_content" | "get_page_snapshot" | "wait_for_selector"
                | "extract_links" => ToolSkill::Browser,
                "git_status" | "git_diff" | "git_log" | "git_show" | "git_branch_list" => {
                    ToolSkill::Shell
                }
                _ => ToolSkill::Filesystem,
            },
            airlock_level: AirlockLevel::Safe,
        },

        // Level 1: state-changing but non-destructive
        "write_file" | "append_file" | "mkdir" | "create_file" => ToolPolicy {
            skill: ToolSkill::Filesystem,
            airlock_level: AirlockLevel::Sensitive,
        },
        "browse_url" | "click_element" | "navigate" | "open_new_tab" | "type_text" | "go_back" => {
            ToolPolicy {
                skill: ToolSkill::Browser,
                airlock_level: AirlockLevel::Sensitive,
            }
        }

        // Level 2: destructive or external command execution
        "remote_workspace_access" => ToolPolicy {
            skill: ToolSkill::RemoteSession,
            airlock_level: AirlockLevel::Dangerous,
        },
        "execute_command" => ToolPolicy {
            skill: ToolSkill::Shell,
            airlock_level: AirlockLevel::Dangerous,
        },
        "http_post_json" => ToolPolicy {
            skill: ToolSkill::Web,
            airlock_level: AirlockLevel::Dangerous,
        },
        "submit_form" => ToolPolicy {
            skill: ToolSkill::Browser,
            airlock_level: AirlockLevel::Dangerous,
        },
        "delete_file" | "move_file" => ToolPolicy {
            skill: ToolSkill::Filesystem,
            airlock_level: AirlockLevel::Dangerous,
        },

        // Memory tools — L0 (safe reads) / L1 (writes to encrypted vault)
        "recall_memory" => ToolPolicy {
            skill: ToolSkill::Memory,
            airlock_level: AirlockLevel::Safe,
        },
        "save_memory" => ToolPolicy {
            skill: ToolSkill::Memory,
            airlock_level: AirlockLevel::Sensitive,
        },

        // IRONMILL — Document tools (KINGFALL Phase 1)
        // L0: Read-only document parsing
        "pdf_read" | "excel_read" | "docx_read" => ToolPolicy {
            skill: ToolSkill::Documents,
            airlock_level: AirlockLevel::Safe,
        },
        // L1: Document creation (writes new files, non-destructive)
        "pdf_create" | "excel_write" | "docx_create" | "archive_create" => ToolPolicy {
            skill: ToolSkill::Documents,
            airlock_level: AirlockLevel::Sensitive,
        },
        "list_recurring_tasks" => ToolPolicy {
            skill: ToolSkill::Workspace,
            airlock_level: AirlockLevel::Safe,
        },
        "schedule_recurring_task" | "delete_recurring_task" | "update_recurring_task" => {
            ToolPolicy {
                skill: ToolSkill::Workspace,
                airlock_level: AirlockLevel::Sensitive,
            }
        }

        // ── Beam RPC + Secure Local Signing Bridge ──────────────────────
        // L0: read-only — chain configs, wallet info, gas estimation
        "beam_get_wallet" | "beam_list_wallets" | "beam_estimate_gas" => ToolPolicy {
            skill: ToolSkill::Evm,
            airlock_level: AirlockLevel::Safe,
        },
        // L1: writes config file to workspace
        "beam_rpc_connect" => ToolPolicy {
            skill: ToolSkill::Evm,
            airlock_level: AirlockLevel::Sensitive,
        },
        // L2: wallet creation/import (touches encrypted key store) and all signing/send
        "beam_create_wallet"
        | "beam_import_wallet"
        | "beam_sign_transaction"
        | "beam_send_transaction" => ToolPolicy {
            skill: ToolSkill::Evm,
            airlock_level: AirlockLevel::Dangerous,
        },

        _ => return None,
    };

    Some(policy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_core_tools() {
        let browse = get_tool_policy("browse_url").expect("browse_url should have policy");
        assert_eq!(browse.skill, ToolSkill::Browser);
        assert_eq!(browse.airlock_level, AirlockLevel::Sensitive);

        let shell = get_tool_policy("execute_command").expect("execute_command should have policy");
        assert_eq!(shell.skill, ToolSkill::Shell);
        assert_eq!(shell.airlock_level, AirlockLevel::Dangerous);

        let web = get_tool_policy("web_search").expect("web_search should have policy");
        assert_eq!(web.skill, ToolSkill::Web);
        assert_eq!(web.airlock_level, AirlockLevel::Safe);
    }

    #[test]
    fn unknown_tool_has_no_policy() {
        let unknown = get_tool_policy("future_tool");
        assert!(unknown.is_none());
    }
}
