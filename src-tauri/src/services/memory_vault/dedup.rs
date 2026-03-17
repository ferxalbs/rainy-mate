use super::types::{DistilledMemory, MemoryCategory};
use super::MemoryVaultService;
use std::sync::Arc;

const SKIP_THRESHOLD: f32 = 0.05;
const UPDATE_THRESHOLD: f32 = 0.15;

pub enum DedupDecision {
    Insert(DistilledMemory),
    Update {
        existing_id: String,
        merged: DistilledMemory,
    },
    Skip,
}

pub struct DedupGate {
    vault: Arc<MemoryVaultService>,
}

impl DedupGate {
    pub fn new(vault: Arc<MemoryVaultService>) -> Self {
        Self { vault }
    }

    pub async fn gate(
        &self,
        workspace_id: &str,
        distilled: DistilledMemory,
        query_embedding: &[f32],
    ) -> DedupDecision {
        let results = match self
            .vault
            .search_workspace_vector(workspace_id, query_embedding, 3)
            .await
        {
            Ok(results) => results,
            Err(_) => return DedupDecision::Insert(distilled),
        };

        for (entry, distance) in &results {
            let existing_cat = entry
                .metadata
                .get("_category")
                .map(|s| MemoryCategory::from_str_loose(s))
                .unwrap_or(MemoryCategory::Observation);

            let same_category = existing_cat == distilled.category;

            if *distance < SKIP_THRESHOLD && same_category {
                return DedupDecision::Skip;
            }

            if *distance < UPDATE_THRESHOLD && same_category {
                let existing_importance = entry
                    .metadata
                    .get("_importance")
                    .and_then(|v| v.parse::<f32>().ok())
                    .unwrap_or(0.5);

                return DedupDecision::Update {
                    existing_id: entry.id.clone(),
                    merged: DistilledMemory {
                        content: distilled.content,
                        category: distilled.category,
                        importance: distilled.importance.max(existing_importance),
                    },
                };
            }
        }

        DedupDecision::Insert(distilled)
    }
}
