use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LgePhaseResult {
    pub phase: String,
    pub artifact_content: String,
    pub artifact_path: String,
}
