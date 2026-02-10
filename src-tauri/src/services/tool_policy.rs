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
/// Unknown tools default to Filesystem + Sensitive for conservative handling.
pub fn get_tool_policy(function_name: &str) -> ToolPolicy {
    match function_name {
        // Level 0: read-only
        "read_file" | "list_files" | "search_files" | "web_search" | "read_web_page"
        | "screenshot" | "get_page_content" => ToolPolicy {
            skill: match function_name {
                "web_search" | "read_web_page" => ToolSkill::Web,
                "screenshot" | "get_page_content" => ToolSkill::Browser,
                _ => ToolSkill::Filesystem,
            },
            airlock_level: AirlockLevel::Safe,
        },

        // Level 1: state-changing but non-destructive
        "write_file" | "append_file" | "mkdir" | "create_file" => ToolPolicy {
            skill: ToolSkill::Filesystem,
            airlock_level: AirlockLevel::Sensitive,
        },
        "browse_url" | "click_element" | "navigate" => ToolPolicy {
            skill: ToolSkill::Browser,
            airlock_level: AirlockLevel::Sensitive,
        },

        // Level 2: destructive or external command execution
        "execute_command" => ToolPolicy {
            skill: ToolSkill::Shell,
            airlock_level: AirlockLevel::Dangerous,
        },
        "delete_file" | "move_file" => ToolPolicy {
            skill: ToolSkill::Filesystem,
            airlock_level: AirlockLevel::Dangerous,
        },

        _ => ToolPolicy {
            skill: ToolSkill::Filesystem,
            airlock_level: AirlockLevel::Sensitive,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_core_tools() {
        let browse = get_tool_policy("browse_url");
        assert_eq!(browse.skill, ToolSkill::Browser);
        assert_eq!(browse.airlock_level, AirlockLevel::Sensitive);

        let shell = get_tool_policy("execute_command");
        assert_eq!(shell.skill, ToolSkill::Shell);
        assert_eq!(shell.airlock_level, AirlockLevel::Dangerous);

        let web = get_tool_policy("web_search");
        assert_eq!(web.skill, ToolSkill::Web);
        assert_eq!(web.airlock_level, AirlockLevel::Safe);
    }

    #[test]
    fn defaults_unknown_to_sensitive_filesystem() {
        let unknown = get_tool_policy("future_tool");
        assert_eq!(unknown.skill, ToolSkill::Filesystem);
        assert_eq!(unknown.airlock_level, AirlockLevel::Sensitive);
    }
}
