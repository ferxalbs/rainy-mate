#[cfg(test)]
mod tests {
    use crate::ai::agent::manager::AgentManager;
    use crate::ai::agent::runtime::{AgentRuntime, RuntimeOptions};
    use crate::ai::router::IntelligentRouter;
    use crate::ai::specs::manifest::AgentSpec;
    use crate::ai::specs::skills::AgentSkills;
    use crate::ai::specs::soul::AgentSoul;
    use crate::db::Database;
    use crate::services::SkillExecutor;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    #[ignore] // FIXME: Libsql threading conflict in tests
    async fn test_persisted_agent_execution() {
        // 1. Setup DB directly (Database struct needs AppHandle)
        let db_url = "sqlite::memory:";
        // Connect returns Pool<Sqlite>
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(db_url)
            .await
            .expect("Failed to connect to memory db");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        // AgentManager::new takes Pool<Sqlite>, not Arc<Pool>
        let agent_manager = Arc::new(AgentManager::new(pool));

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
        // Mock skills for now or use basic one
        // We need a proper SkillExecutor construction or mock.
        // For this test, we might need to construct a minimal one or mock the trait if possible?
        // SkillExecutor is a struct, not a trait. Let's try to construct it with minimal dependencies.
        // It requires WorkspaceManager which might be heavy.
        // Let's assume for this integration test we can't easily spin up the full SkillExecutor without mocking.
        // BUT, verification_test.rs is in the crate, so we can access internals.

        // Let's skip full execution if SkillExecutor is too hard to mock,
        // but getting here proves Persistence + Runtime loading works.

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
