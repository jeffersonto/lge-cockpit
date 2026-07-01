use serde::{Deserialize, Serialize};

use crate::models::phase::Phase;

/// Result of running one LGE phase. `phase` serializes to the lowercase wire
/// string ("planning" | "builder" | "review" | "guardian"), so the frontend's
/// `LgePhaseResult` type (which expects a string) is unaffected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LgePhaseResult {
    pub phase: Phase,
    pub artifact_content: String,
    pub artifact_path: String,
}
