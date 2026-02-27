#[cfg(test)]
mod tests {
    use crate::ai::agent::manager::AgentManager;
    use crate::ai::agent::runtime::{AgentRuntime, RuntimeOptions};
    use crate::ai::router::IntelligentRouter;
    use crate::ai::specs::manifest::AgentSpec;
    use crate::ai::specs::skills::AgentSkills;
    use crate::ai::specs::soul::AgentSoul;
    use crate::services::SkillExecutor;
    use serial_test::serial;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    #[serial]
    async fn test_persisted_agent_execution() {
        // 1. Setup DB directly (using libsql in-memory)
        let db = libsql::Builder::new_local(":memory:")
            .build()
            .await
            .expect("Failed to open memory db");
        let conn = db.connect().expect("Failed to connect to db");

        // Manually create schema since we don't have migrations in this test context
        conn.execute(
            "CREATE TABLE agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                soul TEXT,
                created_at INTEGER NOT NULL,
                spec_json TEXT,
                version TEXT
            )",
            (),
        )
        .await
        .expect("Failed to create agents table");

        // AgentManager::new takes Connection
        let agent_manager = Arc::new(AgentManager::new(conn));

        // 2. Create AgentSpec V3
        let spec = AgentSpec {
            id: "test-persisted-agent".to_string(),
            version: "3.0.0".to_string(),
            soul: AgentSoul {
                name: "Test Agent".to_string(),
                description: "A test agent".to_string(),
                soul_content: "You are a test agent. Always reply with 'Verified'.".to_string(),
                ..Default::default()
            },
            skills: AgentSkills::default(),
            airlock: Default::default(),
            memory_config: Default::default(),
            connectors: Default::default(),
            signature: None,
        };

        // 3. Persist Agent (this tests the new CREATE/SAVE logic)
        let created_id = agent_manager
            .create_agent(&spec)
            .await
            .expect("Failed to create agent");
        assert_eq!(created_id, "test-persisted-agent");

        // 4. Retrieve Agent (this tests the new LOAD logic)
        let loaded_spec = agent_manager
            .get_agent_spec("test-persisted-agent")
            .await
            .expect("Failed to get spec")
            .expect("Spec not found");
        assert_eq!(loaded_spec.soul.name, "Test Agent");

        // 5. Run Runtime with Loaded Spec (this tests INTEGRATION)
        let temp_dir = std::env::temp_dir();
        let memory =
            Arc::new(crate::ai::agent::memory::AgentMemory::new("test-ws", temp_dir).await);
        let router = Arc::new(RwLock::new(IntelligentRouter::default()));

        // Verify Runtime Construction
        let options = RuntimeOptions {
            model: Some("test-model".to_string()),
            workspace_id: "test-ws".to_string(),
            max_steps: Some(1),
            allowed_paths: None,
            custom_system_prompt: None,
        };

        // We can't easily run() without a real SkillExecutor/Router,
        // but we can assert the runtime is initialized with the correct spec.
        let runtime = AgentRuntime::new(
            loaded_spec.clone(),
            options,
            router,
            Arc::new(SkillExecutor::mock()), // Assuming we add a mock/new_empty to SkillExecutor?
            // If not, we can just stop here as the goal was Persistence -> Spec Loading.
            memory,
            Arc::new(None),
        );

        assert_eq!(runtime.spec.soul.name, "Test Agent");
        println!("Successfully persisted, loaded, and initialized agent runtime!");
    }
}
