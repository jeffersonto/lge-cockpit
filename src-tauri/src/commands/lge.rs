use std::collections::HashMap;

use chrono::Utc;
use tauri::{Emitter, State};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_shell::ShellExt;

use crate::commands::claude_utils::{resolve_claude_path, shell_env_prefix, shell_escape, user_shell};
use crate::db::queries;
use crate::models::lge::LgePhaseResult;
use crate::AppState;

fn get_phase_artifact(phase: &str) -> Result<&'static str, String> {
    match phase {
        "planning" => Ok("plan.md"),
        "builder" => Ok("builder.md"),
        "review" => Ok("review.md"),
        "guardian" => Ok("guardian.md"),
        _ => Err(format!("Unknown phase: {}", phase)),
    }
}

const DEFAULT_MODELS: &[(&str, &str)] = &[
    ("planning", "opus"),
    ("builder", "haiku"),
    ("review", "sonnet"),
    ("guardian", "opus"),
];

fn read_phase_model(conn: &rusqlite::Connection, phase: &str) -> String {
    let key = format!("model_{}", phase);
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get::<_, String>(0),
    )
    .unwrap_or_else(|_| {
        DEFAULT_MODELS
            .iter()
            .find(|(p, _)| *p == phase)
            .map(|(_, m)| m.to_string())
            .unwrap_or_else(|| "sonnet".to_string())
    })
}

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

fn build_phase_prompt(
    phase: &str,
    task_code: &str,
    task_title: &str,
    task_description: &str,
    extra_context: Option<&str>,
) -> String {
    let artifact_filename = match phase {
        "builder"  => "builder.md",
        "review"   => "review.md",
        "guardian"  => "guardian.md",
        _ => "artifact.md",
    };
    // Planning uses --permission-mode plan (read-only), so artifact location rules
    // don't apply — the plan is resolved from ~/.claude/plans/ by resolve_plan_file().
    let base = if phase == "planning" {
        format!("{}\n", MARKDOWN_FORMAT_RULES)
    } else {
        let artifact_rules = ARTIFACT_LOCATION_RULES
            .replace("{TASK_CODE}", task_code)
            .replace("{PHASE_ARTIFACT_FILENAME}", artifact_filename);
        format!("{}\n{}\n", artifact_rules, MARKDOWN_FORMAT_RULES)
    };
    let phase_prompt = match phase {
        "planning" => format!(
            r#"You are a senior software architect executing the Planning phase of the LGE process.

Task: {task_code} - {task_title}
Description: {task_description}

## Step 1 — Analyze the codebase
Read CLAUDE.md (or AGENTS.md) and the directory tree. Identify and store explicitly:
- **{{LANGUAGE}}** — primary language of the area being changed (Go, Python, TypeScript, Java, Rust, Kotlin, ...)
- Naming conventions, architectural patterns, test patterns
- Existing interfaces near the task's domain

Also check if docs/tasks/lessons.md exists and read it — it contains past mistakes to avoid.

## Step 2 — Classify the task type
Pick exactly ONE row from the Quality Gates reference table below. The classification drives the targets you will write into the plan.

| Task type | Code Health | Test Quality | Security | Observability |
|---|:---:|:---:|:---:|:---:|
| Simple bugfix (1-3 files) | >= 70 (C) | >= 70 (C) | >= 80 (B) | >= 70 (C) |
| New feature (no sensitive data) | >= 80 (B) | >= 80 (B) | >= 80 (B) | >= 70 (C) |
| New feature (PII / sensitive data) | >= 80 (B) | >= 80 (B) | >= 90 (A) | >= 80 (B) |
| New feature (critical SLA flow) | >= 80 (B) | >= 80 (B) | >= 80 (B) | >= 90 (A) |
| Refactoring / tech debt | >= 90 (A) | >= 80 (B) | >= 80 (B) | >= 70 (C) |

Scale: 90-100=A, 80-89=B, 70-79=C, 60-69=D, <60=F.

## Step 3 — Generate the implementation plan

CRITICAL — Placeholder substitution rules:
- Replace EVERY `{{LANGUAGE}}` token with the language detected in Step 1.
- Replace EVERY `NN` placeholder with the concrete numeric target (70/80/90) chosen in Step 2.
- Replace EVERY `(X)` letter-grade placeholder with the matching grade letter (A/B/C/D).
- Replace EVERY bracketed example `[e.g. Go: ...]` with a concrete rule for the detected language. Drop the `[e.g.]` notation — write the final rule as plain text.
- Replace EVERY `...` placeholder in tables and bullet lists with real, task-specific content.
- Do NOT leave any template token, NN, X, or `[e.g. ...]` notation unfilled in the final plan.

The plan MUST include ALL sections below, fully filled in:

# Implementation Plan — {task_code}

## Business Objective
> [What this task solves and why it matters]

## Current Project Analysis
- Language(s): ...
- Patterns identified: ...
- Naming conventions: ...
- Existing architecture: ...

## Implementation Tasks
- [ ] 1. [Task] — file(s): `path/to/file`
- [ ] 2. [Task] — file(s): `path/to/file`

## Modules and Interfaces
| Module | File | Responsibility |
|--------|------|----------------|
| ... | ... | ... |

## Component Contracts
- Interface X: ...

## External Dependencies
- ...

## Required Tests
- [ ] [Unit test] — `path/to/file_test`
- [ ] [Integration test] — ...

## Quality Gates — Targets for This Task

Detected language: {{LANGUAGE}}
Task type: [chosen row from the table in Step 2]

| Dimension | Minimum Target | Justification |
|---|:---:|---|
| Code Health | >= NN (X) | [why this target fits the task] |
| Test Quality | >= NN (X) | [why this target fits the task] |
| Security | >= NN (X) | [why this target fits the task] |
| Observability | >= NN (X) | [why this target fits the task] |

## Quality Cheat Sheet for Builder (DO / DON'T) — {{LANGUAGE}}

> The Builder reads this section directly from plan.md. Replace bracketed examples with concrete rules for {{LANGUAGE}}.

### Code
DO:
- Functions max 40 lines and 4 parameters
- Early return to reduce nesting
- Descriptive names (no obscure abbreviations)
- Errors always handled and propagated with context
- Named constants (no magic numbers)
- [Language-specific DO #1 — e.g. Go: small interfaces 1-3 methods defined at consumer; Python: type hints on public signatures; TS: discriminated unions over enums]
- [Language-specific DO #2 — pick another idiomatic rule for {{LANGUAGE}}]

DON'T:
- [PRINT_STATEMENT for {{LANGUAGE}} — e.g. Go: fmt.Println; Python: print(); Java: System.out; TS: console.log] in production
- Hardcoded strings that should be config
- God functions (>80 lines) or God structs/classes (>7 fields)
- Dead code or TODOs without issue reference
- [Language-specific DON'T — e.g. Go: goroutines without errgroup; Python: bare `except:`; Java: ObjectInputStream with external data]

### Tests
DO:
- [TEST_PATTERN for {{LANGUAGE}} — e.g. Go: table-driven tests; Python: pytest.parametrize; Java: @ParameterizedTest; TS: test.each] with descriptive names ("Should X when Y")
- Cover happy path + error path + boundary values
- Assertions with clear messages
- [PARALLEL_PATTERN for {{LANGUAGE}} — e.g. Go: t.Parallel(); Python: pytest-xdist; Java: @Execution(CONCURRENT)]

DON'T:
- [SLEEP_IN_TEST for {{LANGUAGE}} — e.g. Go: time.Sleep; Python: time.sleep; TS: setTimeout in tests] (Sleepy Test)
- Multiple asserts without message (Assertion Roulette)
- Conditional logic inside tests (if/else)
- Tests without assertions (Empty Test)

### Security
DO:
- Parameterized queries [QUERY_EXAMPLE for {{LANGUAGE}} — e.g. Go: database/sql placeholders; Python: parameterized queries (not f-strings); Java: PreparedStatement]
- Validate input at system boundaries
- Structured logging without PII

DON'T:
- Concatenate user input into queries or commands
- Hardcode secrets, tokens or credentials
- Expose stack traces or internals in error responses
- [LANGUAGE_SECURITY_DONT — e.g. Python: shell=True in subprocess; Java: ObjectInputStream with external data]

### Observability
DO:
- Structured logs with context (IDs, operation, error) [LOG_EXAMPLE for {{LANGUAGE}} — e.g. Go: log.LoggerFrom(ctx); Python: structlog; Java: MDC; TS: pino/winston]
- Propagate trace/context in external calls

DON'T:
- [PRINT_STATEMENT for {{LANGUAGE}}] in production
- Logs without identifiable context

## Quality Constraints
- Performance: ...
- Security: ...
- Compatibility: ...

Output the complete plan as a markdown document. This plan is the execution contract for all subsequent phases."#
        ),
        "builder" => format!(
            r#"You are a software engineer executing the Builder phase of the LGE process.

Task: {task_code} - {task_title}

## Step 1 — Read the plan
Read docs/tasks/{task_code}/plan.md completely. Pay special attention to:
- Every implementation task and its file paths
- The "Quality Cheat Sheet for Builder (DO / DON'T)" section
- Quality Gate targets

Also check if docs/tasks/lessons.md exists and read it before starting.

## Step 2 — Implement ALL tasks
For each task in the plan:
1. Read the relevant existing code first
2. Implement changes following the plan exactly — no shortcuts, no skipped tasks
3. Apply the DO/DON'T rules from the Quality Cheat Sheet in plan.md
4. Create or update tests as specified in the plan
5. After each task, confirm it is done and mark its checkbox in plan.md before moving to the next

## Step 3 — Quality self-check (MANDATORY before moving to verification)
For every function/method created or modified, verify:
- [ ] Max 40 lines and 4 parameters?
- [ ] Early return used to reduce nesting?
- [ ] All error paths handled and propagated with context?
- [ ] Structured logging (not print statements) for important operations?
- [ ] No magic numbers — named constants used?
- [ ] No hardcoded secrets or credentials?
- [ ] No dead code or TODOs without issue reference?

If any check fails, fix it before proceeding.

## Step 4 — Verify
After implementing all tasks:
1. Run all tests (e.g. `make test`, `go test ./...`, `npm test` — adapt to the project)
2. Run the build (e.g. `go build ./...`, `npm run build`)
3. Run any linter or architecture checks if available (e.g. `make arch-go`, `make lint`)
4. Show the actual output of each verification command

If tests fail, fix the issues before declaring this phase complete.

## Step 5 — Write builder.md to disk

Save a SINGLE file at docs/tasks/{task_code}/builder.md with this exact structure:

# Builder — {task_code}

## Quick Context
One-line status (e.g. "All 8 tasks completed successfully").

| # | Task | Status | Notes |
|---|------|--------|-------|
(one row per task from the plan — Done / Partial / Not done)

---

## Files Changed
For EVERY file created or modified:
- **Path:** `path/to/file`
- **Change:** created / modified / deleted
- **What changed and why:** (2-5 lines minimum)
- **Reviewer note:** any important implementation detail

## Technical Decisions
Every non-trivial decision made:
- What alternatives were considered
- Why this approach was chosen
- Trade-offs or constraints

## Quality Self-Check Results
| Check | Result | Notes |
|---|:---:|---|
| Functions <= 40 lines / 4 params | ok/partial/fail | ... |
| Error paths handled | ok/partial/fail | ... |
| No print statements in production | ok/fail | ... |
| No magic numbers | ok/partial/fail | ... |
| No hardcoded secrets | ok/fail | ... |

## Verification Results
Actual output of all commands run:
- Test results (pass/fail counts, coverage if available)
- Build result
- Linter/architecture check result

## Context for Reviewer
- Most complex or risky parts
- Any deviations from the plan and reason
- Edge cases handled (or not handled and why)
- Any technical debt introduced
- Anything that needs extra scrutiny

Do NOT print the artifact to stdout. Write it ONLY to disk at docs/tasks/{task_code}/builder.md."#
        ),
        "review" => format!(
            r#"You are a senior code reviewer executing the Review phase of the LGE process.

Task: {task_code} - {task_title}

## Step 1 — Read all context
Read ALL of the following before touching any code:
- docs/tasks/{task_code}/plan.md (execution contract + quality gate targets + cheat sheet)
- docs/tasks/{task_code}/builder.md (what the builder did, decisions, and Quick Context)
- Every file listed in the "Files Changed" section of builder.md
- "Context for Reviewer" section in builder.md

Also check if docs/tasks/lessons.md exists and read it.

## Step 2 — Perform 4 independent reviews

### Review A — Plan Adherence
- Was every task from the plan implemented?
- Were all interfaces and contracts respected?
- Were all required tests created?
- Produce a table: Task | Implemented? | Compliant? | Note

### Review B — Code Health
Analyze production files looking for:
- Cognitive complexity > 15 (flag; > 25 = Critical)
- Code smells: Long Method (>40 lines), God Class/Struct (>7 fields), Feature Envy, Primitive Obsession, Shotgun Surgery
- Duplication: repeated logic blocks (5+ similar lines)
- SRP violation: modules/functions doing more than one thing
- Tech debt: magic numbers, hardcoded strings, TODOs without issue, dead code

**Code Health Score** — start at 100, apply deductions:
| Finding | Deduction |
|---|---|
| Function with cognitive complexity > 25 | -20 |
| Function with cognitive complexity 16-25 | -10 |
| Critical code smell (God Class, severe Shotgun Surgery) | -15 |
| High code smell (Long Method, Feature Envy) | -10 |
| Medium code smell (Primitive Obsession, duplication) | -5 |
| SRP violation | -10 |
| Tech debt item (magic number, hardcoded string, TODO) | -5 |

Show explicit calculation: "100 - 10 (Long Method in X) - 5 (duplication in Y) = 85 (B)"

### Review C — Test Quality
Apply mutation reasoning for each assertion: would the test fail if you:
- Negated a condition (`if a > b` → `if a <= b`)
- Swapped an operator (`a + b` → `a - b`)
- Removed a call (`service.Save(entity)` → removed)
- Changed a return value (`return true` → `return false`)
- Altered a boundary (`>= 0` → `> 0`)

Identify: coverage gaps (untested error paths, boundaries, branches), test anti-patterns (Empty Test, Assertion Roulette, Sleepy Test, Conditional Test Logic, Magic Number Test).

**Test Quality Score** — weighted formula:
| Dimension | Weight | Calculation |
|---|---|---|
| Critical Path Coverage | 20% | (paths with test / total critical paths) x 100 |
| Test Anti-Patterns | 15% | 100 - (Critical x 25 + High x 15 + Medium x 5 + Low x 2) |
| Mutation Readiness | 25% | (assertions that detect mutants / total assertions) x 100 |
| Async Flow Testing | 10% | (async flows tested / total async flows) x 100 — if no async = 100 |
| Coverage Completeness | 20% | (branches/paths with test / total branches/paths) x 100 |
| Test Pyramid Health | 10% | 100 if healthy, -20 per deviation |

Final score = weighted sum. Show explicit calculation per dimension.

### Review D — Security & Observability
**Security** — check OWASP Top 10:
- SQL/command injection (parameterized queries?)
- Path traversal, XSS, CSRF
- Sensitive data in logs/errors?
- Secrets/credentials hardcoded? Scan for: `password =`, `secret =`, `api_key`, `token`, `sk-`, `AKIA`
- Input validation at boundaries?

**Security Score** — start at 100:
| Finding | Deduction |
|---|---|
| Critical vulnerability / exposed secret | -30 |
| High vulnerability / abandoned dependency | -15 |
| Medium finding | -5 |
| Low finding | -2 |

**Observability** — check:
- Errors and critical ops have structured logs with context (IDs, operation, error)?
- SLA operations have metrics?
- External calls propagate trace/context?
- No print/console.log/fmt.Println in production?

**Observability Score** — start at 100:
| Finding | Deduction |
|---|---|
| Critical path without adequate logging | -20 |
| SLA operation without metrics | -15 |
| External call without trace | -10 |
| Debug print in production | -5 |

## Step 3 — Fix Critical and High findings
For every Critical or High issue identified: fix it directly in the code.
After all fixes, re-run the full test suite and build to confirm nothing broke.
Note which scores improved after corrections.

## Step 4 — Write review.md to disk

Save a SINGLE file at docs/tasks/{task_code}/review.md with this exact structure:

# Review — {task_code}

## Quick Context
One-line verdict (e.g. "3 Critical fixed, Code Health 85/B, Test Quality 78/C — ready for Guardian").

## Plan Adherence (Review A)
| Task from Plan | Implemented? | Compliant? | Note |
|---|:---:|:---:|---|
| 1. ... | yes/no | yes/warn | ... |

## Quality Gates Scorecard (Reviewer)
| Dimension | Score | Grade | Target (plan.md) | Status |
|---|:---:|:---:|:---:|:---:|
| Code Health | NN | X | >= NN (X) | MEETS / BELOW |
| Test Quality | NN | X | >= NN (X) | MEETS / BELOW |
| Security | NN | X | >= NN (X) | MEETS / BELOW / N/A |
| Observability | NN | X | >= NN (X) | MEETS / BELOW / N/A |

## Findings by Severity
| Severity | Code Health | Test Quality | Security | Observability | Total |
|---|:---:|:---:|:---:|:---:|:---:|
| Critical | N | N | N | N | N |
| High | N | N | N | N | N |
| Medium | N | N | N | N | N |
| Low | N | N | N | N | N |

## Detailed Findings
| # | Severity | Dimension | File | Issue | Fix Applied |
|---|---|---|---|---|---|
(list EVERY finding, even minor ones)

## Code Health Details (Review B)
### Technical Findings
(severity, location, reason, suggestion for each finding)

### Refactor Opportunities
For each smell: current pattern -> suggested pattern -> before/after -> benefit

### Score Calculation
Explicit deductions: `100 - X (smell in file:line) - Y (...) = NN (X)`

## Test Quality Details (Review C)
### Mutation Reasoning Notes
Assertions that would NOT detect a mutant — list with file:line and the specific mutation that escapes.

### Coverage Gaps
| Path / Scenario | File | Tested? | Risk |
|---|---|:---:|---|

### Test Anti-Patterns Found
| Anti-Pattern | File | Line | Severity | Description |
|---|---|---|---|---|

### Suggested Tests (to be added)
(name, input, expected output, why it matters)

### Score Calculation
Per-dimension calculation with weights:
- Critical Path Coverage (20%): NN
- Test Anti-Patterns (15%): NN
- Mutation Readiness (25%): NN
- Async Flow Testing (10%): NN
- Coverage Completeness (20%): NN
- Test Pyramid Health (10%): NN
- **Final: NN (X)**

## Security & Observability Details (Review D)
### OWASP Vulnerabilities
| Issue | File | Line | Severity | Mitigation |
|---|---|---|---|---|

### Secrets & Credentials Scan
Report findings as `[REDACTED]` — never expose actual values. If none found, write "No exposed secrets detected".

### Dependency Health (if dependencies changed)
| Package | Last Release | CVEs | Maintenance | License | Risk |
|---|---|---|---|---|---|

### Observability Findings
| Aspect | Status | Detail |
|---|:---:|---|
| Logging coverage | ok/warn/fail | ... |
| SLA metrics | ok/warn/fail/N/A | ... |
| Trace propagation | ok/warn/fail/N/A | ... |
| No debug prints in production | ok/fail | ... |

### Score Calculations
- Security: `100 - [deductions] = NN (X)`
- Observability: `100 - [deductions] = NN (X)`

## Files Modified by Reviewer
For each file changed beyond what the builder did:
- **Path:** `path/to/file`
- **What was wrong:** ...
- **What was fixed:** ...

## Verification Results After Fixes
Actual command output after all fixes:
- Test results (pass/fail counts)
- Build result
- Coverage output

## Notes for Guardian
- Dimensions below target and root cause
- Critical/High findings NOT fully fixed and why
- Residual risks
- Anything the Guardian must verify end-to-end

Do NOT print the artifact to stdout. Write it ONLY to disk at docs/tasks/{task_code}/review.md."#
        ),
        "guardian" => format!(
            r#"You are a tech lead executing the Guardian/Assurance phase of the LGE process.

Task: {task_code} - {task_title}

## Step 1 — Read ALL artifacts and code
Read in this order:
- docs/tasks/{task_code}/plan.md — original contract + quality gate targets
- docs/tasks/{task_code}/builder.md — what was built, decisions made, Quick Context
- Every file listed in "Files Changed" of builder.md
- docs/tasks/{task_code}/review.md — findings, fixes, scores, residual risks
- Every file listed in "Files Modified by Reviewer" of review.md
- Pay special attention to "Notes for Guardian" in review.md

Also check if docs/tasks/lessons.md exists and read it.

IMPORTANT: Read the Quality Gates Scorecard from review.md (disk) — do not rely on conversation memory.

## Step 2 — Run concrete verification (MANDATORY)
Run ALL of the following and show the actual output:
1. Full test suite (e.g. `make test`, `go test ./...`, `npm test`)
2. Build (e.g. `go build ./...`, `npm run build`)
3. Static analysis / linter if available (e.g. `go vet ./...`, `make lint`)
4. `git diff --stat` to show total change impact

If any check fails, fix the issue and re-run. Show output before AND after fixes.

## Step 3 — Systemic validation (7 criteria)
Validate the SYSTEM as a whole (not individual files in isolation):
1. **Plan Adherence** — Every task from the plan implemented and verifiable in code?
2. **Module Consistency** — Interfaces, types, contracts consistent across modules?
3. **Functional Completeness** — Any business requirement gaps?
4. **Integration Integrity** — End-to-end flows work together?
5. **Error Handling** — All error paths covered and tested?
6. **Test Coverage** — Success, failure, and edge case scenarios all tested?
7. **Quality Gates** — Do the Reviewer scores meet the targets in plan.md?

For criterion 7: read scores from review.md and targets from plan.md. If you disagree with a score, you may adjust it with concrete evidence. Apply the verdict logic below.

## Step 4 — Address residual risks
For each item in "Notes for Guardian" from review.md:
- Confirm it is resolved, fix it, or document why it is acceptable.
If you find a critical architectural problem → stop, document, request re-planning.

## Step 5 — Determine the verdict (top-down — first match wins)
| # | Condition | If YES |
|:---:|---|---|
| 1 | Any Critical finding? | REJECTED |
| 2 | Any score F (< 60)? | REJECTED |
| 3 | Exposed secret? | REJECTED |
| 4 | Any High finding? | APPROVED WITH CAVEATS |
| 5 | Any score D (60-69)? | APPROVED WITH CAVEATS |
| 6 | Test anti-pattern with Critical severity? | APPROVED WITH CAVEATS |
| 7 | Any score below target in plan.md? | APPROVED WITH CAVEATS (or justify deviation) |
| 8 | All conditions 1-7 = NO? | APPROVED |

## Step 6 — Record lessons learned (Self-Improvement Loop)

If this run surfaced a recurring pattern of mistakes, a User correction during the flow, or a quality-gate failure rooted in a generalizable cause, append a new entry to docs/tasks/lessons.md.

Create the file with a `# Lessons Learned — LGE` heading if it does not exist. Append entries in this format (most recent on top, do NOT rewrite past entries):

```
## [YYYY-MM-DD] — {task_code}: [Short title of the lesson]
**Mistake:** [What happened]
**Root cause:** [Why it happened]
**Rule:** [What to do differently next time — actionable]
**Layer affected:** Planning / Builder / Reviewer / Guardian
**Quality Gate:** Code Health / Test Quality / Security / Observability / N/A
```

Skip this step if the run was clean (APPROVED with no Critical/High findings, no User corrections, all targets met). Save only generalizable rules — never raw scores or task-specific noise.

If the file would exceed 50 entries, archive the oldest entries to docs/tasks/lessons-archive.md before appending.

## Step 7 — Write guardian.md to disk

Save a SINGLE file at docs/tasks/{task_code}/guardian.md with this exact structure:

# Guardian — {task_code}

## Quick Context
Final verdict: **APPROVED** / **APPROVED WITH CAVEATS** / **REJECTED**

## Concrete Verification
- Build: PASS / FAIL
- Tests: X executed, Y passed, Z failed
- Coverage: X%
- Static analysis: PASS / FAIL / N/A
- Impact (git diff --stat): X files, +Y/-Z lines

## Systemic Validation
| Criterion | Status | Evidence | Notes |
|---|:---:|---|---|
| Plan Adherence | ok/warn/fail | ... | ... |
| Module Consistency | ok/warn/fail | ... | ... |
| Functional Completeness | ok/warn/fail | ... | ... |
| Integration Integrity | ok/warn/fail | ... | ... |
| Error Handling | ok/warn/fail | ... | ... |
| Test Coverage | ok/warn/fail | ... | ... |
| Quality Gates | ok/warn/fail | [see Scorecard] | ... |

## Quality Gates — Final Scorecard
| Dimension | Reviewer | Guardian | Target (plan.md) | Status |
|---|:---:|:---:|:---:|:---:|
| Code Health | NN (X) | NN (X) | >= NN (X) | MEETS / BELOW |
| Test Quality | NN (X) | NN (X) | >= NN (X) | MEETS / BELOW |
| Security | NN (X) | NN (X) | >= NN (X) | MEETS / BELOW / N/A |
| Observability | NN (X) | NN (X) | >= NN (X) | MEETS / BELOW / N/A |

## Verdict Checklist (top-down evaluation)
| # | Condition | Result | Verdict |
|:---:|---|:---:|---|
| 1 | Any Critical finding? | YES/NO | REJECTED if YES |
| 2 | Any score F (< 60)? | YES/NO | REJECTED if YES |
| 3 | Exposed secret? | YES/NO | REJECTED if YES |
| 4 | Any High finding? | YES/NO | APPROVED WITH CAVEATS if YES |
| 5 | Any score D (60-69)? | YES/NO | APPROVED WITH CAVEATS if YES |
| 6 | Test anti-pattern Critical? | YES/NO | APPROVED WITH CAVEATS if YES |
| 7 | Any score below plan.md target? | YES/NO | APPROVED WITH CAVEATS if YES |
| 8 | All 1-7 = NO? | YES/NO | APPROVED if YES |

## Progressive Quality (evolution across layers)
| Aspect | Post-Builder | Post-Reviewer | Post-Guardian |
|---|:---:|:---:|:---:|
| Tests executed | X | Y | Z |
| Tests passing | X | Y | Z |
| Critical/High findings | — | X found, Y fixed | Z fixed |
| Refactors applied | — | X | Y |

## Residual Risk Resolution
For each item flagged in "Notes for Guardian":
- **Flagged:** what was flagged
- **Resolution:** how it was resolved (or why it is acceptable)

## Final Corrections Applied
- ...

## Delivery Statement

> **APPROVED** — All quality gates meet targets. Scorecard: Code Health NN/X, Test Quality NN/X, Security NN/X, Observability NN/X. Code is production-ready.

or

> **APPROVED WITH CAVEATS** — [Dimension(s) below target or High findings]. Justification: [concrete reason to accept deviation]. Recommend [action] in the next iteration.

or

> **REJECTED** — [Condition that caused the block: Critical finding / score F / exposed secret]. Must return to [Builder/Reviewer] to remediate.

Do NOT print the artifact to stdout. Write it ONLY to disk at docs/tasks/{task_code}/guardian.md."#
        ),
        _ => String::new(),
    };
    let context_block = match extra_context {
        Some(ctx) if !ctx.trim().is_empty() => format!("\nADDITIONAL CONTEXT FROM USER:\n{}\n\n", ctx),
        _ => String::new(),
    };
    format!("{}{}{}", base, context_block, phase_prompt)
}

#[derive(serde::Serialize, Clone)]
struct PlanningQueueEvent {
    task_id: String,
    phase: String,
}

fn send_phase_notification(app: &tauri::AppHandle, phase: &str, task_title: &str, success: bool) {
    let phase_label = match phase {
        "planning" => "Planning",
        "builder" => "Builder",
        "review" => "Review",
        "guardian" => "Guardian",
        _ => phase,
    };
    let body = if success {
        format!("{} concluída — {}", phase_label, task_title)
    } else {
        format!("{} falhou — {}", phase_label, task_title)
    };
    let _ = app
        .notification()
        .builder()
        .title("LGE Cockpit")
        .body(&body)
        .show();
}

#[tauri::command]
pub async fn run_lge_phase(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    phase: String,
    task_title: String,
    task_description: String,
    extra_context: Option<String>,
) -> Result<LgePhaseResult, String> {
    let artifact_filename = get_phase_artifact(&phase)?;

    // Planning phase must run sequentially — queue if another is already running
    let _planning_permit = if phase == "planning" {
        // Notify frontend: this task is now queued
        app.emit("lge_phase_queued", PlanningQueueEvent {
            task_id: task_id.clone(),
            phase: phase.clone(),
        }).ok();

        // Block until the semaphore is available (other planning completes)
        let permit = state.planning_semaphore
            .acquire()
            .await
            .map_err(|e| e.to_string())?;

        // Check if cancelled while waiting in queue
        {
            let mut cancelled = state.planning_cancelled.lock().map_err(|e| e.to_string())?;
            if cancelled.remove(&task_id) {
                return Err("Planning cancelled while queued".to_string());
            }
        }

        // Notify frontend: this task is now executing
        app.emit("lge_phase_dequeued", PlanningQueueEvent {
            task_id: task_id.clone(),
            phase: phase.clone(),
        }).ok();

        Some(permit)
    } else {
        None
    };

    // Get repo path, task code, configured model, and worktree info
    let (repo_path, task_code, phase_model, repository_id, git_branch) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;

        // Get task to find repository_id, jira_key, worktree_path, and git_branch
        let task = conn
            .prepare("SELECT repository_id, jira_key, worktree_path, git_branch FROM tasks WHERE id = ?1")
            .map_err(|e| e.to_string())?
            .query_row(rusqlite::params![task_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|e| format!("Task not found: {}", e))?;

        let repo_path = queries::get_repository_path(&conn, &task.0)?;
        let code = task.1.unwrap_or_else(|| task_id[..8].to_string());
        let model = read_phase_model(&conn, &phase);
        (repo_path, code, model, task.0, task.3)
    };

    // Resolve working directory: use existing worktree, create one if branch exists, or fall back to repo root
    let (resolved, env_prefix) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        (queries::resolve_working_dir(&conn, &task_id)?, shell_env_prefix(&conn))
    };

    let working_dir = if resolved == repo_path {
        if let Some(ref branch) = git_branch {
            match crate::commands::git::ensure_worktree(
                &app, &state, &task_id, &repo_path, &repository_id, &task_code, Some(branch.as_str()), &env_prefix,
            ).await {
                Ok(wt) => wt,
                Err(_) => repo_path.clone(), // fallback gracefully
            }
        } else {
            resolved
        }
    } else {
        resolved
    };

    // Create artifacts directory
    let artifacts_dir = format!("{}/docs/tasks/{}", working_dir, task_code);
    std::fs::create_dir_all(&artifacts_dir)
        .map_err(|e| format!("Failed to create artifacts dir: {}", e))?;

    // Fetch attachments scoped to this phase and build context block
    let attachment_context = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let attachments = queries::list_attachments_by_task_and_phase(&conn, &task_id, &phase)?;
        if attachments.is_empty() {
            String::new()
        } else {
            let mut ctx = String::from("## Contexto Adicional\n\n");
            for att in &attachments {
                ctx.push_str(&format!("### {}\n{}\n\n---\n\n", att.file_name, att.content));
            }
            ctx
        }
    };

    let combined_context = match (extra_context.as_deref(), attachment_context.is_empty()) {
        (Some(ec), false) => Some(format!("{}\n\n{}", attachment_context, ec)),
        (None, false) => Some(attachment_context),
        (Some(ec), true) => Some(ec.to_string()),
        (None, true) => None,
    };

    let prompt = build_phase_prompt(&phase, &task_code, &task_title, &task_description, combined_context.as_deref());
    let claude_bin = resolve_claude_path();

    // Planning runs in plan mode (read-only analysis); other phases need write access
    let permissions_flag = match phase.as_str() {
        "planning" => "--permission-mode plan",
        _ => "--dangerously-skip-permissions",
    };

    let shell = app.shell();
    let full_cmd = format!(
        "{}echo {} | {} --print --model {} {} -p {}",
        env_prefix,
        shell_escape(&prompt),
        claude_bin,
        phase_model,
        permissions_flag,
        shell_escape(&working_dir),
    );

    let pid_key = format!("{}:{}", task_id, phase);

    // Spawn process so we can track the PID for cancel_lge_phase
    let (mut rx, child) = shell
        .command(&user_shell())
        .args(["-l", "-i", "-c", &full_cmd])
        .spawn()
        .map_err(|e| format!("Failed to invoke Claude CLI: {}", e))?;

    // Store PID immediately so cancel_lge_phase can kill it
    {
        let mut pids = state.running_pids.lock().map_err(|e| e.to_string())?;
        pids.insert(pid_key.clone(), child.pid());
    }

    // Collect output events from the spawned process
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    let mut exit_code: Option<i32> = None;

    use tauri_plugin_shell::process::CommandEvent;
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(data) => stdout_bytes.extend_from_slice(&data),
            CommandEvent::Stderr(data) => stderr_bytes.extend_from_slice(&data),
            CommandEvent::Terminated(payload) => {
                exit_code = payload.code;
                break;
            }
            CommandEvent::Error(err) => stderr_bytes.extend_from_slice(err.as_bytes()),
            _ => {}
        }
    }

    // Clean up PID tracking
    {
        let mut pids = state.running_pids.lock().map_err(|e| e.to_string())?;
        pids.remove(&pid_key);
    }

    let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
    let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();

    if exit_code != Some(0) && stdout.trim().is_empty() {
        send_phase_notification(&app, &phase, &task_title, false);
        return Err(format!("Claude CLI error: {}", stderr));
    }

    // Resolve artifact content.
    // - Planning: Claude saves to ~/.claude/plans/*.md, we read it and copy to task dir.
    // - Builder/Review/Guardian: Claude writes directly to disk, we just read the file.
    let artifact_path = format!("{}/{}", artifacts_dir, artifact_filename);

    let artifact_content = if phase == "planning" {
        let content = resolve_plan_file(&stdout).unwrap_or_else(|| extract_artifact(&stdout));
        std::fs::write(&artifact_path, &content)
            .map_err(|e| format!("Failed to write artifact: {}", e))?;
        content
    } else {
        std::fs::read_to_string(&artifact_path).map_err(|e| {
            format!(
                "Phase '{}' did not produce the expected artifact at '{}'. \
                 Ensure Claude wrote the file during execution. Error: {}",
                phase, artifact_path, e
            )
        })?
    };

    send_phase_notification(&app, &phase, &task_title, true);

    // Update task status
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let status = match phase.as_str() {
            "planning" => "in_progress",
            "guardian" => "completed",
            _ => "in_progress",
        };
        let _ = queries::update_task_status(&conn, &task_id, status, &Utc::now().to_rfc3339());
    }

    Ok(LgePhaseResult {
        phase,
        artifact_content,
        artifact_path,
    })
}

fn extract_artifact(claude_output: &str) -> String {
    // Try to parse as JSON (--output-format json wrapper)
    if let Ok(wrapper) = serde_json::from_str::<serde_json::Value>(claude_output) {
        if let Some(result) = wrapper.get("result").and_then(|r| r.as_str()) {
            return result.to_string();
        }
    }
    // Otherwise return raw output (when not using --output-format json)
    claude_output.trim().to_string()
}

/// When --permission-mode plan is used, Claude saves the plan to ~/.claude/plans/<name>.md.
/// Finds the most recently modified .md file in that directory created within the last 5 minutes.
fn resolve_plan_file(_claude_output: &str) -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let plans_dir = std::path::Path::new(&home).join(".claude").join("plans");

    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(300))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    let mut candidates: Vec<(std::time::SystemTime, std::path::PathBuf)> = std::fs::read_dir(&plans_dir)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()?.to_str()? != "md" { return None; }
            let modified = entry.metadata().ok()?.modified().ok()?;
            if modified >= cutoff { Some((modified, path)) } else { None }
        })
        .collect();

    candidates.sort_by(|a, b| b.0.cmp(&a.0));

    let latest = candidates.into_iter().next()?.1;
    std::fs::read_to_string(latest).ok()
}

#[tauri::command]
pub fn load_lge_artifacts(
    state: State<AppState>,
    task_id: String,
) -> Result<HashMap<String, String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // Get task info
    let (repo_id, jira_key, worktree_path): (String, Option<String>, Option<String>) = conn
        .prepare("SELECT repository_id, jira_key, worktree_path FROM tasks WHERE id = ?1")
        .map_err(|e| e.to_string())?
        .query_row(rusqlite::params![task_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| format!("Task not found: {}", e))?;

    let repo_path = queries::get_repository_path(&conn, &repo_id)?;
    let base_path = worktree_path
        .filter(|p| std::path::Path::new(p).exists())
        .unwrap_or(repo_path);
    let task_code = jira_key.unwrap_or_else(|| task_id[..8].to_string());
    let artifacts_dir = format!("{}/docs/tasks/{}", base_path, task_code);

    let mut artifacts = HashMap::new();
    // (phase, current_filename, legacy_filename_fallback)
    let files: &[(&str, &str, Option<&str>)] = &[
        ("planning", "plan.md", None),
        ("builder", "builder.md", Some("builder-model-summary.md")),
        ("review", "review.md", Some("reviewer-model-summary.md")),
        ("guardian", "guardian.md", Some("guardian-model.md")),
    ];

    for (phase, filename, fallback) in files {
        let path = format!("{}/{}", artifacts_dir, filename);
        if let Ok(content) = std::fs::read_to_string(&path) {
            artifacts.insert(phase.to_string(), content);
        } else if let Some(legacy) = fallback {
            let legacy_path = format!("{}/{}", artifacts_dir, legacy);
            if let Ok(content) = std::fs::read_to_string(&legacy_path) {
                artifacts.insert(phase.to_string(), content);
            }
        }
    }

    Ok(artifacts)
}

#[tauri::command]
pub fn save_lge_artifact(
    state: State<AppState>,
    task_id: String,
    phase: String,
    content: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let (repo_id, jira_key, worktree_path): (String, Option<String>, Option<String>) = conn
        .prepare("SELECT repository_id, jira_key, worktree_path FROM tasks WHERE id = ?1")
        .map_err(|e| e.to_string())?
        .query_row(rusqlite::params![task_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| format!("Task not found: {}", e))?;

    let repo_path = queries::get_repository_path(&conn, &repo_id)?;
    let base_path = worktree_path
        .filter(|p| std::path::Path::new(p).exists())
        .unwrap_or(repo_path);
    let task_code = jira_key.unwrap_or_else(|| task_id[..8].to_string());

    let filename = match phase.as_str() {
        "planning" => "plan.md",
        "builder" => "builder.md",
        "review" => "review.md",
        "guardian" => "guardian.md",
        _ => return Err(format!("Unknown phase: {}", phase)),
    };

    let artifact_path = format!("{}/docs/tasks/{}/{}", base_path, task_code, filename);

    std::fs::write(&artifact_path, content)
        .map_err(|e| format!("Failed to save artifact: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn cancel_lge_phase(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    phase: String,
) -> Result<(), String> {
    let pid_key = format!("{}:{}", task_id, phase);

    // If planning is queued (not yet started), mark it for cancellation
    if phase == "planning" {
        if let Ok(mut cancelled) = state.planning_cancelled.lock() {
            cancelled.insert(task_id.clone());
        }
    }

    // Try to get stored PID
    let pid = {
        let mut pids = state.running_pids.lock().map_err(|e| e.to_string())?;
        pids.remove(&pid_key)
    };

    // Kill claude processes for this context — find by command pattern
    let shell = app.shell();
    let kill_cmd = if let Some(pid) = pid {
        format!("kill -TERM -{} 2>/dev/null; kill -TERM {} 2>/dev/null", pid, pid)
    } else {
        // Fallback: kill claude processes that match our pattern
        "pkill -f 'claude --print' 2>/dev/null || true".to_string()
    };

    let _ = shell
        .command(&user_shell())
        .args(["-l", "-i", "-c", &kill_cmd])
        .output()
        .await;

    Ok(())
}
