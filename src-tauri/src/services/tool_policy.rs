use crate::models::neural::AirlockLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSkill {
    Filesystem,
    Browser,
    Shell,
    Web,
}

impl ToolSkill {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Filesystem => "filesystem",
            Self::Browser => "browser",
            Self::Shell => "shell",
            Self::Web => "web",
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
                "screenshot"
                | "get_page_content"
                | "get_page_snapshot"
                | "wait_for_selector"
                | "extract_links" => {
                    ToolSkill::Browser
                }
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
        "browse_url" | "click_element" | "navigate" | "open_new_tab" | "type_text"
        | "go_back" => ToolPolicy {
            skill: ToolSkill::Browser,
            airlock_level: AirlockLevel::Sensitive,
        },

        // Level 2: destructive or external command execution
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
