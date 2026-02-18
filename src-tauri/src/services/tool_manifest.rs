use crate::models::neural::{ParameterSchema, SkillManifest, SkillMethod};
use crate::services::tool_policy::get_tool_policy;
use crate::services::SkillExecutor;
use std::collections::{BTreeMap, HashMap};

fn parse_method_parameters(
    parameters: &serde_json::Value,
) -> Result<HashMap<String, ParameterSchema>, String> {
    let mut out: HashMap<String, ParameterSchema> = HashMap::new();
    let required: Vec<String> = parameters
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    let Some(properties) = parameters.get("properties").and_then(|v| v.as_object()) else {
        return Ok(out);
    };

    for (name, schema) in properties {
        let param_type = schema
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("string")
            .to_string();
        let description = schema
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let is_required = required.iter().any(|entry| entry == name);
        out.insert(
            name.clone(),
            ParameterSchema {
                param_type,
                required: Some(is_required),
                description,
            },
        );
    }

    Ok(out)
}

pub fn build_skill_manifest_from_runtime() -> Result<Vec<SkillManifest>, String> {
    let tools = SkillExecutor::get_registered_tool_definitions();
    let mut grouped: BTreeMap<String, Vec<SkillMethod>> = BTreeMap::new();

    for tool in tools {
        let name = tool.function.name;
        let description = tool.function.description;
        let Some(policy) = get_tool_policy(&name) else {
            return Err(format!(
                "Tool '{}' has no explicit policy; refusing to build manifest",
                name
            ));
        };

        let parameters = parse_method_parameters(&tool.function.parameters)?;
        grouped
            .entry(policy.skill.as_str().to_string())
            .or_default()
            .push(SkillMethod {
                name,
                description,
                airlock_level: policy.airlock_level,
                parameters,
            });
    }

    let manifests = grouped
        .into_iter()
        .map(|(skill_name, mut methods)| {
            methods.sort_by(|a, b| a.name.cmp(&b.name));
            SkillManifest {
                name: skill_name,
                version: "1.0.0".to_string(),
                methods,
            }
        })
        .collect::<Vec<_>>();

    Ok(manifests)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn manifest_covers_every_registered_tool() {
        let manifests = build_skill_manifest_from_runtime().expect("manifest generation should work");
        let mut from_manifest: Vec<String> = manifests
            .into_iter()
            .flat_map(|skill| skill.methods.into_iter().map(|method| method.name))
            .collect();
        from_manifest.sort();

        let mut from_registry: Vec<String> = SkillExecutor::get_registered_tool_definitions()
            .into_iter()
            .map(|tool| tool.function.name)
            .collect();
        from_registry.sort();

        assert_eq!(from_manifest, from_registry);
    }

    #[test]
    fn legacy_agents_directory_is_absent() {
        let legacy_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("agents");
        assert!(
            !legacy_dir.exists(),
            "Legacy agents directory must stay removed: {}",
            legacy_dir.display()
        );
    }
}
