use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// One of the four sequential LGE pipeline phases: planning -> builder -> review -> guardian.
///
/// Wire format: lowercase strings ("planning" | "builder" | "review" | "guardian"),
/// produced/consumed by serde. The frontend stays string-based and untouched.
///
/// Pure value-object: no IO. Artifact retrieval (reading the artifact bytes back,
/// including planning's `~/.claude/plans` scan) is NOT Phase's job — it belongs to
/// the future PhaseRunner module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Planning,
    Builder,
    Review,
    Guardian,
}

/// Domain permission concept for a phase. Phase does NOT know CLI flag strings;
/// translating `Permission::Plan` / `Permission::SkipPermissions` /
/// `Permission::None` into `--permission-mode plan` /
/// `--dangerously-skip-permissions` / (no flag) is the ClaudeInvocation
/// module's job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Plan,
    SkipPermissions,
    /// Emit no permission flag at all — used by callers outside the LGE
    /// pipeline (e.g. `CommitMessageRunner`) that run Claude one-shot
    /// without plan-mode or skip-permissions.
    None,
}

/// Per-task substitution data for `Phase::build_prompt`. The fields always
/// travel together; `extra_context` is the pre-merged user-context + attachment
/// context (the caller merges, since fetching attachments is IO).
#[derive(Debug, Clone, Copy)]
pub struct PromptContext<'a> {
    pub task_code: &'a str,
    pub task_title: &'a str,
    pub task_description: &'a str,
    pub extra_context: Option<&'a str>,
}

/// Hot-path bundle returned by `Phase::prepare`. Groups the three values
/// `run_lge_phase` always co-needs (filename + permission + prompt).
///
/// Deliberately does NOT carry `model`: the effective model is the DB override
/// (read_phase_model / the future Settings module), not Phase's static default.
/// Bundling `default_model()` here would lie (caller overwrites it) or echo
/// (passed-in model = zero leverage).
#[derive(Debug, Clone)]
pub(crate) struct PhasePlan {
    pub(crate) filename: &'static str,
    pub(crate) permission: Permission,
    pub(crate) prompt: String,
}

#[derive(Debug, Clone)]
pub struct ParsePhaseError(pub String);

/// Static per-phase contract. Private — the single source of truth behind the
/// public accessors. All fields are `&'static` so the struct is `Copy` and the
/// accessors are zero-allocation.
#[derive(Debug, Clone, Copy)]
struct PhaseSpec {
    wire_name: &'static str,
    artifact_filename: &'static str,
    legacy_filenames: &'static [&'static str],
    default_model: &'static str,
    permission: Permission,
}

// Private helpers internal to `build_prompt`. Not part of the interface.
const ARTIFACT_LOCATION_RULES: &str = r#"
ARTIFACT LOCATION RULES (MANDATORY — NO EXCEPTIONS):
- ALL files you create or write during this phase MUST be saved inside docs/tasks/{TASK_CODE}/
- NEVER write files to the project root, to a temp directory, or to any other location.
- The artifact for this phase MUST be saved as: docs/tasks/{TASK_CODE}/{PHASE_ARTIFACT_FILENAME}
- Write exactly ONE artifact file per phase. Do not create additional summary or details files.
- No other naming patterns are allowed. Do not invent file names.
- If you are unsure where to write a file, the answer is always docs/tasks/{TASK_CODE}/
"#;

const MARKDOWN_FORMAT_RULES: &str = r#"
MARKDOWN FORMATTING RULES (MANDATORY):
- Tables MUST have NO blank lines between the header row, separator row, and data rows. All rows must be consecutive.
- Use proper GFM table syntax: | Header | Header | then |---|---| then | data | data |
- Use proper heading hierarchy: # for title, ## for sections, ### for subsections
- Use - for bullet lists, 1. for numbered lists
- Use ``` for code blocks with language identifier
- Use **bold** for emphasis, `inline code` for code references
- Checkboxes: - [ ] for pending, - [x] for done
"#;

impl Phase {
    /// The four phases in canonical pipeline order.
    pub const ALL: [Phase; 4] =
        [Phase::Planning, Phase::Builder, Phase::Review, Phase::Guardian];

    /// Lowercase wire identity. Used for settings-key derivation
    /// (`format!("model_{}", phase.as_str())`) and IPC round-trip.
    pub fn as_str(self) -> &'static str {
        self.spec().wire_name
    }

    /// Canonical artifact filename (planning -> "plan.md", builder -> "builder.md", ...).
    pub fn artifact_filename(self) -> &'static str {
        self.spec().artifact_filename
    }

    /// Historical filenames tried as read-fallback during load.
    /// Planning -> `[]`; builder -> `["builder-model-summary.md"]`; etc.
    /// Sunset TODO: a one-shot migration renames old files and this is removed.
    pub fn legacy_filenames(self) -> &'static [&'static str] {
        self.spec().legacy_filenames
    }

    /// Static default model (planning -> opus, builder -> haiku, review -> sonnet,
    /// guardian -> opus). The future Settings module owns the dynamic DB override
    /// and falls back to this.
    pub fn default_model(self) -> &'static str {
        self.spec().default_model
    }

    /// Domain permission. Planning -> Plan; the rest -> SkipPermissions.
    pub fn permission_mode(self) -> Permission {
        self.spec().permission
    }

    /// Assemble the phase prompt. Planning omits the `ARTIFACT_LOCATION_RULES`
    /// block; the other three inject it with the per-phase filename substituted.
    /// All phases prepend `MARKDOWN_FORMAT_RULES`. `extra_context` (pre-merged
    /// user + attachment context) is appended when non-empty.
    ///
    /// Pure: prompt text is baked at compile time via `include_str!`; identical
    /// inputs yield identical output.
    pub fn build_prompt(self, ctx: &PromptContext<'_>) -> String {
        let base = if self == Phase::Planning {
            format!("{}\n", MARKDOWN_FORMAT_RULES)
        } else {
            let rules = ARTIFACT_LOCATION_RULES
                .replace("{TASK_CODE}", ctx.task_code)
                .replace("{PHASE_ARTIFACT_FILENAME}", self.artifact_filename());
            format!("{}\n{}\n", rules, MARKDOWN_FORMAT_RULES)
        };

        let context_block = match ctx.extra_context {
            Some(c) if !c.trim().is_empty() => {
                format!("\nADDITIONAL CONTEXT FROM USER:\n{}\n\n", c)
            }
            _ => String::new(),
        };

        // Each template passes only the substitution tokens it actually uses.
        // format! then enforces at compile time that template and args agree:
        // adding `{task_description}` to builder.md without updating its call
        // here fails to build, and vice versa.
        let tmpl = match self {
            Phase::Planning => format!(
                include_str!("../../prompts/planning.md"),
                task_code = ctx.task_code,
                task_title = ctx.task_title,
                task_description = ctx.task_description,
            ),
            Phase::Builder => format!(
                include_str!("../../prompts/builder.md"),
                task_code = ctx.task_code,
                task_title = ctx.task_title,
            ),
            Phase::Review => format!(
                include_str!("../../prompts/review.md"),
                task_code = ctx.task_code,
                task_title = ctx.task_title,
            ),
            Phase::Guardian => format!(
                include_str!("../../prompts/guardian.md"),
                task_code = ctx.task_code,
                task_title = ctx.task_title,
            ),
        };

        format!("{}{}{}", base, context_block, tmpl)
    }

    /// Hot-path bundle: filename + permission + prompt in one call. The only
    /// value it does NOT carry is `model` (the effective model is a DB override,
    /// see `PhasePlan`).
    pub fn prepare(self, ctx: &PromptContext<'_>) -> PhasePlan {
        PhasePlan {
            filename: self.artifact_filename(),
            permission: self.permission_mode(),
            prompt: self.build_prompt(ctx),
        }
    }

    /// Single internal source of truth for the per-phase contract. Private.
    fn spec(self) -> PhaseSpec {
        match self {
            Phase::Planning => PhaseSpec {
                wire_name: "planning",
                artifact_filename: "plan.md",
                legacy_filenames: &[],
                default_model: "opus",
                permission: Permission::Plan,
            },
            Phase::Builder => PhaseSpec {
                wire_name: "builder",
                artifact_filename: "builder.md",
                legacy_filenames: &["builder-model-summary.md"],
                default_model: "haiku",
                permission: Permission::SkipPermissions,
            },
            Phase::Review => PhaseSpec {
                wire_name: "review",
                artifact_filename: "review.md",
                legacy_filenames: &["reviewer-model-summary.md"],
                default_model: "sonnet",
                permission: Permission::SkipPermissions,
            },
            Phase::Guardian => PhaseSpec {
                wire_name: "guardian",
                artifact_filename: "guardian.md",
                legacy_filenames: &["guardian-model.md"],
                default_model: "opus",
                permission: Permission::SkipPermissions,
            },
        }
    }
}

impl FromStr for Phase {
    type Err = ParsePhaseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "planning" => Ok(Phase::Planning),
            "builder" => Ok(Phase::Builder),
            "review" => Ok(Phase::Review),
            "guardian" => Ok(Phase::Guardian),
            _ => Err(ParsePhaseError(s.to_string())),
        }
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for ParsePhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown phase: {}", self.0)
    }
}

impl std::error::Error for ParsePhaseError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(code: &str) -> PromptContext<'_> {
        PromptContext {
            task_code: code,
            task_title: "Title",
            task_description: "Description",
            extra_context: None,
        }
    }

    #[test]
    fn artifact_filenames() {
        assert_eq!(Phase::Planning.artifact_filename(), "plan.md");
        assert_eq!(Phase::Builder.artifact_filename(), "builder.md");
        assert_eq!(Phase::Review.artifact_filename(), "review.md");
        assert_eq!(Phase::Guardian.artifact_filename(), "guardian.md");
    }

    #[test]
    fn legacy_filenames() {
        assert!(Phase::Planning.legacy_filenames().is_empty());
        assert_eq!(Phase::Builder.legacy_filenames(), &["builder-model-summary.md"]);
        assert_eq!(Phase::Review.legacy_filenames(), &["reviewer-model-summary.md"]);
        assert_eq!(Phase::Guardian.legacy_filenames(), &["guardian-model.md"]);
    }

    #[test]
    fn default_models() {
        assert_eq!(Phase::Planning.default_model(), "opus");
        assert_eq!(Phase::Builder.default_model(), "haiku");
        assert_eq!(Phase::Review.default_model(), "sonnet");
        assert_eq!(Phase::Guardian.default_model(), "opus");
    }

    #[test]
    fn permission_modes() {
        assert_eq!(Phase::Planning.permission_mode(), Permission::Plan);
        assert_eq!(Phase::Builder.permission_mode(), Permission::SkipPermissions);
        assert_eq!(Phase::Review.permission_mode(), Permission::SkipPermissions);
        assert_eq!(Phase::Guardian.permission_mode(), Permission::SkipPermissions);
    }

    #[test]
    fn all_in_pipeline_order() {
        assert_eq!(
            Phase::ALL,
            [Phase::Planning, Phase::Builder, Phase::Review, Phase::Guardian]
        );
    }

    #[test]
    fn as_str_is_lowercase_wire() {
        assert_eq!(Phase::Planning.as_str(), "planning");
        assert_eq!(Phase::Builder.as_str(), "builder");
        assert_eq!(Phase::Review.as_str(), "review");
        assert_eq!(Phase::Guardian.as_str(), "guardian");
    }

    #[test]
    fn from_str_roundtrips() {
        for p in Phase::ALL {
            let s = p.as_str();
            let parsed: Phase = s.parse().unwrap();
            assert_eq!(parsed, p);
            assert_eq!(s, p.to_string());
        }
    }

    #[test]
    fn from_str_rejects_unknown() {
        assert!("foobar".parse::<Phase>().is_err());
        assert!("".parse::<Phase>().is_err());
        assert!("Planning".parse::<Phase>().is_err()); // case-sensitive
    }

    #[test]
    fn serde_roundtrip() {
        for p in Phase::ALL {
            let json = serde_json::to_string(&p).unwrap();
            let back: Phase = serde_json::from_str(&json).unwrap();
            assert_eq!(back, p);
        }
        assert_eq!(serde_json::to_string(&Phase::Planning).unwrap(), "\"planning\"");
    }

    #[test]
    fn serde_rejects_unknown() {
        assert!(serde_json::from_str::<Phase>("\"bogus\"").is_err());
    }

    #[test]
    fn planning_prompt_omits_location_rules() {
        let prompt = Phase::Planning.build_prompt(&ctx("TASK-1"));
        assert!(!prompt.contains("ARTIFACT LOCATION RULES"));
        assert!(prompt.contains("MARKDOWN FORMATTING RULES"));
    }

    #[test]
    fn non_planning_prompts_include_location_rules_with_filename() {
        for p in [Phase::Builder, Phase::Review, Phase::Guardian] {
            let prompt = p.build_prompt(&ctx("TASK-1"));
            assert!(prompt.contains("ARTIFACT LOCATION RULES"), "missing for {:?}", p);
            assert!(
                prompt.contains(&format!("docs/tasks/TASK-1/{}", p.artifact_filename())),
                "missing per-phase filename for {:?}",
                p
            );
        }
    }

    #[test]
    fn prompt_substitutes_task_fields() {
        // Builder uses {task_code} and {task_title} (but not {task_description}).
        let c = PromptContext {
            task_code: "LGE-42",
            task_title: "My Title",
            task_description: "desc here",
            extra_context: None,
        };
        let prompt = Phase::Builder.build_prompt(&c);
        assert!(prompt.contains("LGE-42"));
        assert!(prompt.contains("My Title"));
        assert!(!prompt.contains("{task_code}"));
        assert!(!prompt.contains("{task_title}"));
        // Planning additionally substitutes {task_description}.
        let planning = Phase::Planning.build_prompt(&c);
        assert!(planning.contains("desc here"));
        assert!(!planning.contains("{task_description}"));
    }

    #[test]
    fn prompt_appends_extra_context_when_non_empty() {
        let c = PromptContext {
            task_code: "T1",
            task_title: "t",
            task_description: "d",
            extra_context: Some("extra stuff"),
        };
        let prompt = Phase::Builder.build_prompt(&c);
        assert!(prompt.contains("ADDITIONAL CONTEXT FROM USER:"));
        assert!(prompt.contains("extra stuff"));
    }

    #[test]
    fn prompt_ignores_blank_extra_context() {
        let c = PromptContext {
            task_code: "T1",
            task_title: "t",
            task_description: "d",
            extra_context: Some("   "),
        };
        let prompt = Phase::Builder.build_prompt(&c);
        assert!(!prompt.contains("ADDITIONAL CONTEXT FROM USER"));
    }

    #[test]
    fn prepare_bundles_filename_permission_prompt() {
        let plan = Phase::Builder.prepare(&ctx("T1"));
        assert_eq!(plan.filename, "builder.md");
        assert_eq!(plan.permission, Permission::SkipPermissions);
        assert!(plan.prompt.contains("T1"));
    }

    #[test]
    fn prepare_planning_has_plan_permission() {
        let plan = Phase::Planning.prepare(&ctx("T1"));
        assert_eq!(plan.filename, "plan.md");
        assert_eq!(plan.permission, Permission::Plan);
    }
}
