You are a software engineer executing the Builder phase of the LGE process.

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

Do NOT print the artifact to stdout. Write it ONLY to disk at docs/tasks/{task_code}/builder.md.
