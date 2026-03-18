use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ParsedPromptSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub body_markdown: String,
    pub scripts: Vec<String>,
    pub references: Vec<String>,
    pub source_hash: String,
    pub source_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct PromptSkillFrontmatter {
    name: String,
    description: String,
}

pub fn parse_prompt_skill(skill_dir: &Path) -> Result<ParsedPromptSkill, String> {
    let skill_file = skill_dir.join("SKILL.md");
    let raw = std::fs::read_to_string(&skill_file)
        .map_err(|e| format!("Failed to read {}: {}", skill_file.display(), e))?;
    let (frontmatter, body) = split_frontmatter(&raw)?;
    let metadata: PromptSkillFrontmatter = serde_yaml::from_str(frontmatter)
        .map_err(|e| format!("Invalid YAML frontmatter in {}: {}", skill_file.display(), e))?;

    let canonical_dir = std::fs::canonicalize(skill_dir).unwrap_or_else(|_| skill_dir.to_path_buf());
    let id = build_skill_id(&metadata.name, &canonical_dir);

    Ok(ParsedPromptSkill {
        id,
        name: metadata.name.trim().to_string(),
        description: metadata.description.trim().to_string(),
        body_markdown: body.trim().to_string(),
        scripts: list_asset_names(&canonical_dir.join("scripts")),
        references: list_asset_names(&canonical_dir.join("references")),
        source_hash: hash_content(&raw),
        source_path: canonical_dir,
    })
}

pub fn parse_instruction_skill(file_path: &Path, name: &str, description: &str) -> Result<ParsedPromptSkill, String> {
    let raw = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;
    let canonical = std::fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf());
    let id = build_skill_id(name, &canonical);

    Ok(ParsedPromptSkill {
        id,
        name: name.to_string(),
        description: description.to_string(),
        body_markdown: raw.trim().to_string(),
        scripts: Vec::new(),
        references: Vec::new(),
        source_hash: hash_content(&raw),
        source_path: canonical,
    })
}

fn split_frontmatter(raw: &str) -> Result<(&str, &str), String> {
    if !raw.starts_with("---\n") {
        return Err("Missing YAML frontmatter".to_string());
    }

    let mut offset = 4usize;
    for line in raw[4..].split_inclusive('\n') {
        if line.trim_end() == "---" {
            let frontmatter = &raw[4..offset];
            let body = &raw[offset + line.len()..];
            return Ok((frontmatter, body));
        }
        offset += line.len();
    }

    Err("Unterminated YAML frontmatter".to_string())
}

fn list_asset_names(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return out,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|v| v.to_str()) {
            out.push(name.to_string());
        }
    }
    out.sort();
    out
}

fn build_skill_id(name: &str, source_path: &Path) -> String {
    let slug = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let hash = hash_content(&source_path.to_string_lossy());
    format!("{}-{}", if slug.is_empty() { "skill" } else { &slug }, &hash[..8])
}

fn hash_content(value: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_and_body() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("SKILL.md"),
            "---\nname: Reviewer\ndescription: Reviews code\n---\nUse ripgrep first.",
        )
        .expect("write");

        let parsed = parse_prompt_skill(dir.path()).expect("parse");
        assert_eq!(parsed.name, "Reviewer");
        assert_eq!(parsed.description, "Reviews code");
        assert_eq!(parsed.body_markdown, "Use ripgrep first.");
    }
}
