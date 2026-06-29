use std::collections::{HashMap, HashSet};
use tauri::State;

use crate::commands::claude_utils::shell_env_prefix;
use crate::commands::git::run_git;
use crate::db::queries;
use crate::models::arch_diff::{
    ApiChange, ArchitectureDiff, ChangeSummary, DependencyEdge, DependencyGraph, FileNode,
};
use crate::AppState;

// Max bytes of full diff to parse (50KB)
const MAX_DIFF_BYTES: usize = 50_000;
// Max nodes in the Mermaid diagram
const MAX_MERMAID_NODES: usize = 30;

/// Analyzes the architecture diff of uncommitted working tree changes vs HEAD.
/// Includes both tracked modified files (git diff HEAD) and untracked new files (git status).
#[tauri::command]
pub async fn analyze_working_tree_diff(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
) -> Result<ArchitectureDiff, String> {
    let (working_dir, env_prefix) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        (
            queries::resolve_working_dir(&conn, &task_id)?,
            shell_env_prefix(&conn),
        )
    };

    // 1. git status --porcelain -uall: captures ALL changes including untracked new files.
    //    Without -uall, new directories show as a single "?? dir/" entry instead of individual files.
    //    -uall expands each untracked directory into its individual files.
    let status_output = run_git(
        &app,
        &working_dir,
        "status --porcelain -uall",
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    if status_output.trim().is_empty() {
        return Ok(empty_diff("HEAD", "HEAD"));
    }

    // 2. Line counts for tracked files (staged + unstaged vs HEAD)
    let numstat_tracked = run_git(
        &app,
        &working_dir,
        "diff HEAD --numstat",
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    // Also include staged-only changes (files staged but not in HEAD)
    let numstat_staged = run_git(
        &app,
        &working_dir,
        "diff --cached --numstat",
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    // Merge numstat from tracked and staged (tracked takes precedence if same file)
    let mut numstat_combined = numstat_staged;
    numstat_combined.push('\n');
    numstat_combined.push_str(&numstat_tracked);

    // 3. Full diff for tracked files (for import/API parsing)
    let full_diff_tracked = run_git(
        &app,
        &working_dir,
        "diff HEAD",
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    // 4. Full diff for staged new files
    let full_diff_staged = run_git(
        &app,
        &working_dir,
        "diff --cached",
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    let full_diff_raw = format!("{}\n{}", full_diff_tracked, full_diff_staged);
    let full_diff: String = full_diff_raw.chars().take(MAX_DIFF_BYTES).collect();

    // 5. Parse name-status from git status --porcelain (includes untracked)
    let name_status = build_name_status_from_porcelain(&status_output);

    // 6. Build numstat: tracked file counts + line counts for untracked files from filesystem
    let numstat = build_numstat_with_untracked(&numstat_combined, &status_output, &working_dir);

    run_analysis(&app, &working_dir, &env_prefix, "HEAD", "working-tree", name_status, numstat, full_diff).await
}

/// Converts `git status --porcelain` output into the same format as `git diff --name-status`.
fn build_name_status_from_porcelain(porcelain: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in porcelain.lines() {
        if line.len() < 4 {
            continue;
        }
        let xy = &line[..2];
        let path = line[3..].trim();
        // Skip empty paths and ignored files
        if path.is_empty() {
            continue;
        }
        // XY codes: first char = staged, second = unstaged
        // ?? = untracked (new file), !! = ignored
        let change_type = match xy.trim() {
            "??" => "A", // untracked = added
            s if s.contains('D') => "D",
            s if s.contains('A') || s.contains('C') => "A",
            s if s.contains('R') => {
                // Renamed: "old -> new" format — take the new path
                if let Some(new_path) = path.split(" -> ").last() {
                    lines.push(format!("A\t{}", new_path.trim()));
                }
                continue;
            }
            _ => "M",
        };
        lines.push(format!("{}\t{}", change_type, path));
    }
    lines.join("\n")
}

/// Builds a numstat string that includes line counts for untracked new files read from disk.
fn build_numstat_with_untracked(tracked_numstat: &str, porcelain: &str, working_dir: &str) -> String {
    let mut result = tracked_numstat.to_string();

    // Collect paths already covered by tracked numstat to avoid duplicates
    let covered: std::collections::HashSet<String> = tracked_numstat
        .lines()
        .filter_map(|l| {
            let parts: Vec<&str> = l.splitn(3, '\t').collect();
            if parts.len() == 3 { Some(parts[2].trim().to_string()) } else { None }
        })
        .collect();

    for line in porcelain.lines() {
        if line.len() < 4 {
            continue;
        }
        let xy = line[..2].trim();
        let path = line[3..].trim();
        // Only process untracked files (??) not already counted
        if xy == "??" && !path.is_empty() && !covered.contains(path) {
            let full_path = format!("{}/{}", working_dir, path);
            let line_count = std::fs::read_to_string(&full_path)
                .map(|content| content.lines().count() as u64)
                .unwrap_or(0);
            if line_count > 0 {
                result.push('\n');
                result.push_str(&format!("{}\t0\t{}", line_count, path));
            }
        }
    }
    result
}

/// Analyzes the architecture diff between two commits (kept for backward compatibility).
#[tauri::command]
pub async fn analyze_architecture_diff(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    base_commit: String,
    head_commit: String,
) -> Result<ArchitectureDiff, String> {
    // If no change, return empty diff
    if base_commit == head_commit {
        return Ok(empty_diff(&base_commit, &head_commit));
    }

    let (working_dir, env_prefix) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        (
            queries::resolve_working_dir(&conn, &task_id)?,
            shell_env_prefix(&conn),
        )
    };

    let range = format!("{}..{}", base_commit, head_commit);

    // 1. Get file-level changes: status + path
    let name_status = run_git(
        &app,
        &working_dir,
        &format!("diff --name-status {}", range),
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    // 2. Get line counts per file
    let numstat = run_git(
        &app,
        &working_dir,
        &format!("diff --numstat {}", range),
        &env_prefix,
    )
    .await
    .unwrap_or_default();

    // 3. Get full patch (capped)
    let full_diff_raw = run_git(
        &app,
        &working_dir,
        &format!("diff {}", range),
        &env_prefix,
    )
    .await
    .unwrap_or_default();
    let full_diff: String = full_diff_raw.chars().take(MAX_DIFF_BYTES).collect();

    run_analysis(&app, &working_dir, &env_prefix, &base_commit, &head_commit, name_status, numstat, full_diff).await
}

/// Shared analysis logic used by both commit-based and working-tree diffs.
#[allow(clippy::too_many_arguments)]
async fn run_analysis(
    _app: &tauri::AppHandle,
    _working_dir: &str,
    _env_prefix: &str,
    base_ref: &str,
    head_ref: &str,
    name_status: String,
    numstat: String,
    full_diff: String,
) -> Result<ArchitectureDiff, String> {
    // Parse file changes
    let file_changes = parse_name_status(&name_status);
    let line_counts = parse_numstat(&numstat);

    // Build flat list of changed files with stats
    let mut flat_files: Vec<FlatFile> = file_changes
        .iter()
        .map(|(path, change_type)| {
            let (additions, deletions) = line_counts
                .get(path.as_str())
                .copied()
                .unwrap_or((0, 0));
            FlatFile {
                path: path.clone(),
                change_type: change_type.clone(),
                additions,
                deletions,
            }
        })
        .collect();

    // Summary counts
    let files_added = flat_files.iter().filter(|f| f.change_type == "added").count() as u32;
    let files_modified = flat_files.iter().filter(|f| f.change_type == "modified").count() as u32;
    let files_deleted = flat_files.iter().filter(|f| f.change_type == "deleted").count() as u32;
    let lines_added: u32 = flat_files.iter().map(|f| f.additions).sum();
    let lines_removed: u32 = flat_files.iter().map(|f| f.deletions).sum();

    // Parse imports and API surface from full diff
    let (new_import_edges, api_changes) = parse_diff_content(&full_diff, &flat_files);

    // Extract new external dependencies (package.json / Cargo.toml / go.mod changes)
    let new_dependencies = detect_new_dependencies(&full_diff);

    // Build file tree
    flat_files.sort_by(|a, b| a.path.cmp(&b.path));
    let file_tree = build_file_tree(&flat_files);

    // Build Mermaid diagram
    let dependency_graph = build_dependency_graph(&flat_files, &new_import_edges);

    // Calculate risk score
    let (risk_score, risk_factors) = calculate_risk(
        &flat_files,
        &api_changes,
        &new_dependencies,
        &full_diff,
    );

    let summary = ChangeSummary {
        files_added,
        files_modified,
        files_deleted,
        lines_added,
        lines_removed,
        new_dependencies,
        risk_score,
        risk_factors,
    };

    Ok(ArchitectureDiff {
        phase: String::new(),
        base_commit: base_ref.to_string(),
        head_commit: head_ref.to_string(),
        summary,
        file_tree,
        dependency_graph,
        api_surface: api_changes,
    })
}

// ─── Internal structs ────────────────────────────────────────────────────────

struct FlatFile {
    path: String,
    change_type: String,
    additions: u32,
    deletions: u32,
}

// ─── Parsers ─────────────────────────────────────────────────────────────────

fn parse_name_status(output: &str) -> Vec<(String, String)> {
    output
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, '\t');
            let status = parts.next()?.trim();
            let path = parts.next()?.trim().to_string();
            let change_type = match status.chars().next()? {
                'A' => "added",
                'D' => "deleted",
                _ => "modified",
            };
            Some((path, change_type.to_string()))
        })
        .collect()
}

fn parse_numstat(output: &str) -> HashMap<String, (u32, u32)> {
    let mut map = HashMap::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let added: u32 = parts[0].parse().unwrap_or(0);
        let deleted: u32 = parts[1].parse().unwrap_or(0);
        let path = parts[2].trim().to_string();
        map.insert(path, (added, deleted));
    }
    map
}

// ─── Diff content parsing ────────────────────────────────────────────────────

struct ImportEdge {
    from_file: String,  // full file path (e.g. "internal/api/app.go")
    import_path: String, // raw import string (e.g. "internal/consumer/bocasecompleted")
}

fn parse_diff_content(diff: &str, flat_files: &[FlatFile]) -> (Vec<ImportEdge>, Vec<ApiChange>) {
    let mut import_edges: Vec<ImportEdge> = Vec::new();
    let mut api_added: HashMap<String, (String, String, Option<String>)> = HashMap::new(); // key -> (file, kind, sig)
    let mut api_removed: HashSet<String> = HashSet::new();

    let mut current_file = String::new();

    for line in diff.lines() {
        // Track current file from diff headers
        if line.starts_with("diff --git ") {
            // e.g. "diff --git a/src/foo.ts b/src/foo.ts"
            if let Some(b_part) = line.split(" b/").nth(1) {
                current_file = b_part.trim().to_string();
            }
            continue;
        }

        let is_add = line.starts_with('+') && !line.starts_with("+++");
        let is_del = line.starts_with('-') && !line.starts_with("---");

        if !is_add && !is_del {
            continue;
        }

        let content = &line[1..];

        if is_add {
            // Detect imports — store raw paths for accurate matching later
            if let Some(import_path) = detect_import(content) {
                if !current_file.is_empty() && !import_path.is_empty() {
                    import_edges.push(ImportEdge {
                        from_file: current_file.clone(),
                        import_path,
                    });
                }
            }

            // Detect API additions
            if let Some((symbol, kind, sig)) = detect_api_symbol(content) {
                api_added.insert(symbol.clone(), (current_file.clone(), kind, sig));
                api_removed.remove(&symbol);
            }
        } else {
            // Detect API removals
            if let Some((symbol, kind, sig)) = detect_api_symbol(content) {
                if !api_added.contains_key(&symbol) {
                    api_removed.insert(symbol.clone());
                    // We'll emit as "removed" later
                    api_added.entry(symbol).or_insert((current_file.clone(), kind, sig));
                }
            }
        }
    }

    let api_changes: Vec<ApiChange> = {
        // Build from api_added and api_removed sets
        let mut changes = Vec::new();

        // Find files that actually changed (for context)
        let changed_paths: HashSet<&str> = flat_files.iter().map(|f| f.path.as_str()).collect();

        for (symbol, (file, kind, sig)) in &api_added {
            if !changed_paths.contains(file.as_str()) && !file.is_empty() {
                continue;
            }
            let change_type = if api_removed.contains(symbol) {
                "modified"
            } else {
                "added"
            };
            changes.push(ApiChange {
                file: file.clone(),
                symbol: symbol.clone(),
                kind: kind.clone(),
                change_type: change_type.to_string(),
                signature: sig.clone(),
            });
        }

        // Any symbol in removed but not added = truly removed
        for symbol in &api_removed {
            if !api_added.contains_key(symbol) {
                changes.push(ApiChange {
                    file: current_file.clone(),
                    symbol: symbol.clone(),
                    kind: "unknown".to_string(),
                    change_type: "removed".to_string(),
                    signature: None,
                });
            }
        }

        changes.sort_by(|a, b| a.file.cmp(&b.file).then(a.symbol.cmp(&b.symbol)));
        changes.truncate(50);
        changes
    };

    (import_edges, api_changes)
}

fn detect_import(line: &str) -> Option<String> {
    let trimmed = line.trim();

    // TypeScript/JavaScript: import ... from '...' or require('...')
    if let Some(from_idx) = trimmed.find(" from ") {
        let after = &trimmed[from_idx + 6..];
        let path = extract_quoted(after)?;
        return Some(path);
    }
    if trimmed.contains("require(") {
        if let Some(start) = trimmed.find("require(") {
            let after = &trimmed[start + 8..];
            let path = extract_quoted(after)?;
            return Some(path);
        }
    }

    // Rust: use crate::path or use external::path
    if trimmed.starts_with("use ") && trimmed.contains("::") {
        let after = &trimmed[4..];
        let path = after.split('{').next()?.trim().trim_end_matches(';').to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }

    // Go: "package/path" inside import block
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.contains('/') {
        return Some(trimmed.trim_matches('"').to_string());
    }

    None
}

fn extract_quoted(s: &str) -> Option<String> {
    let s = s.trim();
    let quote = s.chars().next()?;
    if quote != '\'' && quote != '"' && quote != '`' {
        return None;
    }
    let end = s[1..].find(quote)?;
    Some(s[1..end + 1].to_string())
}

fn detect_api_symbol(line: &str) -> Option<(String, String, Option<String>)> {
    let trimmed = line.trim();

    // TypeScript/JavaScript exports
    // export function foo, export const foo, export class Foo, export type Foo, export interface Foo
    if trimmed.starts_with("export ") {
        let rest = &trimmed[7..];
        let (kind, rest) = if rest.starts_with("default ") {
            return None; // skip default exports — no named symbol
        } else if rest.starts_with("async function ") {
            ("function", &rest[15..])
        } else if rest.starts_with("function ") {
            ("function", &rest[9..])
        } else if rest.starts_with("const ") {
            ("function", &rest[6..]) // arrow functions stored as const
        } else if rest.starts_with("class ") {
            ("class", &rest[6..])
        } else if rest.starts_with("type ") {
            ("type", &rest[5..])
        } else if rest.starts_with("interface ") {
            ("interface", &rest[10..])
        } else if rest.starts_with("enum ") {
            ("enum", &rest[5..])
        } else if rest.starts_with("abstract class ") {
            ("class", &rest[15..])
        } else {
            return None;
        };
        let name = rest
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()?
            .to_string();
        if name.is_empty() {
            return None;
        }
        // Capture signature (first 100 chars of line)
        let sig: String = trimmed.chars().take(100).collect();
        return Some((name, kind.to_string(), Some(sig)));
    }

    // Rust public symbols
    if trimmed.starts_with("pub ") || trimmed.starts_with("pub(") {
        let rest = trimmed
            .trim_start_matches("pub(crate)")
            .trim_start_matches("pub(super)")
            .trim_start_matches("pub ")
            .trim_start_matches("pub(")
            .trim();
        let (kind, after) = if rest.starts_with("async fn ") {
            ("function", &rest[9..])
        } else if rest.starts_with("fn ") {
            ("function", &rest[3..])
        } else if rest.starts_with("struct ") {
            ("struct", &rest[7..])
        } else if rest.starts_with("enum ") {
            ("enum", &rest[5..])
        } else if rest.starts_with("trait ") {
            ("trait", &rest[6..])
        } else if rest.starts_with("type ") {
            ("type", &rest[5..])
        } else {
            return None;
        };
        let name = after
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()?
            .to_string();
        if name.is_empty() {
            return None;
        }
        let sig: String = trimmed.chars().take(100).collect();
        return Some((name, kind.to_string(), Some(sig)));
    }

    // Go: exported (capitalized) functions and types
    if trimmed.starts_with("func ") {
        let after = &trimmed[5..];
        // Skip receiver methods: func (r Receiver) Name(
        let name_part = if after.starts_with('(') {
            after.find(')').and_then(|i| after.get(i + 2..))?
        } else {
            after
        };
        let name = name_part
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()?;
        if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            let sig: String = trimmed.chars().take(100).collect();
            return Some((name.to_string(), "function".to_string(), Some(sig)));
        }
    }
    if trimmed.starts_with("type ") {
        let after = &trimmed[5..];
        let name = after
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()?;
        if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            let sig: String = trimmed.chars().take(100).collect();
            return Some((name.to_string(), "type".to_string(), Some(sig)));
        }
    }

    None
}

// ─── Dependency detection ────────────────────────────────────────────────────

fn detect_new_dependencies(diff: &str) -> Vec<String> {
    let mut deps: Vec<String> = Vec::new();
    let mut in_dep_file = false;

    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            let is_dep_file = line.contains("package.json")
                || line.contains("Cargo.toml")
                || line.contains("go.mod")
                || line.contains("requirements.txt")
                || line.contains("pyproject.toml");
            in_dep_file = is_dep_file;
            continue;
        }

        if !in_dep_file {
            continue;
        }

        if !line.starts_with('+') || line.starts_with("+++") {
            continue;
        }

        let content = line[1..].trim();

        // package.json: "package": "version"
        if content.contains('"') && content.contains(':') {
            // Look for lines like: "react": "^18.0.0"
            let parts: Vec<&str> = content.splitn(2, ':').collect();
            if parts.len() == 2 {
                let name = parts[0].trim().trim_matches('"').trim();
                // Skip non-package keys (scripts, main, etc.)
                let version_part = parts[1].trim().trim_matches(',').trim().trim_matches('"');
                let looks_like_version = version_part.starts_with('^')
                    || version_part.starts_with('~')
                    || version_part.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                    || version_part.starts_with(">=");
                if looks_like_version && !name.is_empty() && !name.starts_with('/') {
                    deps.push(format!("{} {}", name, version_part));
                }
            }
        }

        // Cargo.toml: name = "version" or name = { version = "..." }
        if content.contains(" = ") && !content.starts_with('[') {
            let parts: Vec<&str> = content.splitn(2, '=').collect();
            if parts.len() == 2 {
                let name = parts[0].trim();
                let val = parts[1].trim().trim_matches('{').trim();
                if !name.is_empty()
                    && !name.starts_with('#')
                    && name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    let version = val
                        .split('"')
                        .nth(1)
                        .unwrap_or("")
                        .to_string();
                    if !version.is_empty() {
                        deps.push(format!("{} {}", name, version));
                    }
                }
            }
        }

        // go.mod: require package version
        if content.starts_with("require ") {
            let rest = &content[8..];
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                deps.push(format!("{} {}", parts[0], parts[1]));
            }
        }
    }

    deps.sort();
    deps.dedup();
    deps.truncate(20);
    deps
}

// ─── File tree ───────────────────────────────────────────────────────────────

fn build_file_tree(files: &[FlatFile]) -> Vec<FileNode> {
    // Build a nested directory structure
    let mut root: Vec<FileNode> = Vec::new();

    for file in files {
        let segments: Vec<&str> = file.path.split('/').collect();
        insert_into_tree(&mut root, &segments, file);
    }

    root
}

fn insert_into_tree(nodes: &mut Vec<FileNode>, segments: &[&str], file: &FlatFile) {
    if segments.len() == 1 {
        // Leaf node
        nodes.push(FileNode {
            path: file.path.clone(),
            change_type: file.change_type.clone(),
            additions: file.additions,
            deletions: file.deletions,
            is_directory: false,
            children: Vec::new(),
        });
        return;
    }

    // Find or create directory node
    let dir_name = segments[0];
    let dir_path = file.path.splitn(segments.len(), '/').next().unwrap_or(dir_name).to_string();

    if let Some(dir_node) = nodes.iter_mut().find(|n| n.is_directory && n.path == dir_path) {
        insert_into_tree(&mut dir_node.children, &segments[1..], file);
        // Roll up stats
        dir_node.additions += file.additions;
        dir_node.deletions += file.deletions;
    } else {
        let mut dir_node = FileNode {
            path: dir_path,
            change_type: "modified".to_string(),
            additions: file.additions,
            deletions: file.deletions,
            is_directory: true,
            children: Vec::new(),
        };
        insert_into_tree(&mut dir_node.children, &segments[1..], file);
        nodes.push(dir_node);
    }
}

// ─── Mermaid diagram ─────────────────────────────────────────────────────────

/// Returns the parent directory of a file path (e.g. "internal/api/app.go" → "internal/api")
fn parent_dir(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => path,
    }
}

/// Last segment of a path (e.g. "internal/consumer/bocasecompleted" → "bocasecompleted")
fn last_segment(path: &str) -> &str {
    path.trim_end_matches('/').split('/').last().unwrap_or(path)
}

/// Sanitize a string to a valid Mermaid node ID
fn node_id(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn build_dependency_graph(flat_files: &[FlatFile], import_edges: &[ImportEdge]) -> DependencyGraph {
    // ── Step 1: Build package-level nodes (one node per unique parent directory) ──
    // In Go/Rust, files in the same directory = same package. Node = directory.
    let mut pkg_change_map: HashMap<String, &str> = HashMap::new();
    for f in flat_files {
        let pkg = parent_dir(&f.path).to_string();
        // "added" > "modified" > "deleted" for aggregation
        let entry = pkg_change_map.entry(pkg).or_insert(f.change_type.as_str());
        if f.change_type == "added" || (*entry == "deleted" && f.change_type == "modified") {
            *entry = f.change_type.as_str();
        }
    }

    let mut all_pkgs: Vec<String> = pkg_change_map.keys().cloned().collect();
    all_pkgs.sort();

    if all_pkgs.is_empty() {
        return DependencyGraph {
            mermaid_source: String::new(),
            new_edges: Vec::new(),
            existing_edges: Vec::new(),
        };
    }

    // ── Step 2: Resolve edges using suffix matching ──
    // Import path "internal/foo/bar" matches package dir ".../.../internal/foo/bar"
    // or any dir whose suffix == the import path's suffix.
    let resolved_edges: Vec<(String, String, String)> = import_edges
        .iter()
        .filter_map(|edge| {
            let from_pkg = parent_dir(&edge.from_file).to_string();
            let import = edge.import_path.trim_end_matches('/');
            // Skip standard library / very short paths
            if !import.contains('/') {
                return None;
            }
            // Try to find a package directory that ends with the import path
            let to_pkg = all_pkgs.iter().find(|pkg| {
                let p = pkg.as_str();
                p == import || p.ends_with(&format!("/{}", import)) || import.ends_with(&format!("/{}", last_segment(p)))
            });
            let to_pkg = to_pkg.cloned().unwrap_or_else(|| import.to_string());
            if from_pkg == to_pkg {
                return None;
            }
            Some((from_pkg, to_pkg, edge.import_path.clone()))
        })
        .collect();

    // Add external targets (import destinations not in our package list)
    let mut extra_pkgs: Vec<String> = resolved_edges
        .iter()
        .filter_map(|(_, to, _)| {
            if !all_pkgs.contains(to) { Some(to.clone()) } else { None }
        })
        .collect();
    extra_pkgs.sort();
    extra_pkgs.dedup();

    // Limit total nodes
    let mut nodes = all_pkgs.clone();
    for ep in extra_pkgs {
        if nodes.len() >= MAX_MERMAID_NODES { break; }
        nodes.push(ep);
    }

    // ── Step 3: Group by grandparent dir for subgraphs ──
    let mut subgraph_map: HashMap<String, Vec<String>> = HashMap::new();
    for pkg in &nodes {
        // Use last 2 segments of path before the package as group key
        let segments: Vec<&str> = pkg.split('/').collect();
        let group = if segments.len() >= 2 {
            segments[segments.len() - 2].to_string()
        } else {
            segments.last().copied().unwrap_or(pkg).to_string()
        };
        subgraph_map.entry(group).or_default().push(pkg.clone());
    }

    // ── Step 4: Emit Mermaid ──
    let mut lines: Vec<String> = Vec::new();
    lines.push("graph LR".to_string());
    lines.push("    classDef newNode fill:#22c55e20,stroke:#22c55e,color:#22c55e".to_string());
    lines.push("    classDef modNode fill:#f59e0b20,stroke:#f59e0b,color:#f59e0b".to_string());
    lines.push("    classDef delNode fill:#ef444420,stroke:#ef4444,color:#ef4444".to_string());
    lines.push("    classDef extNode fill:#7c3aed20,stroke:#7c3aed,color:#94a3b8".to_string());

    let mut sg_idx = 0;
    let mut sorted_groups: Vec<(&String, &Vec<String>)> = subgraph_map.iter().collect();
    sorted_groups.sort_by_key(|(k, _)| k.as_str());

    for (group, pkgs) in &sorted_groups {
        if pkgs.len() > 1 {
            lines.push(format!("    subgraph sg{}[{}]", sg_idx, group));
            for pkg in pkgs.iter() {
                let id = node_id(pkg);
                let label = last_segment(pkg);
                let css = match pkg_change_map.get(pkg).copied() {
                    Some("added") => "newNode",
                    Some("deleted") => "delNode",
                    Some(_) => "modNode",
                    None => "extNode",
                };
                lines.push(format!("        {}[\"{}\"]:::{}", id, label, css));
            }
            lines.push("    end".to_string());
            sg_idx += 1;
        } else if let Some(pkg) = pkgs.first() {
            let id = node_id(pkg);
            let label = last_segment(pkg);
            let css = match pkg_change_map.get(pkg).copied() {
                Some("added") => "newNode",
                Some("deleted") => "delNode",
                Some(_) => "modNode",
                None => "extNode",
            };
            lines.push(format!("    {}[\"{}\"]:::{}", id, label, css));
        }
    }

    // ── Step 5: Emit edges ──
    let mut seen: HashSet<String> = HashSet::new();
    let mut new_edges: Vec<DependencyEdge> = Vec::new();

    for (from_pkg, to_pkg, import_path) in &resolved_edges {
        let key = format!("{}->{}", from_pkg, to_pkg);
        if seen.contains(&key) { continue; }
        seen.insert(key);

        let from_id = node_id(from_pkg);
        let to_id = node_id(to_pkg);

        // Only emit edge if from_pkg is among our tracked nodes
        if nodes.contains(from_pkg) {
            lines.push(format!("    {} -->|import| {}", from_id, to_id));
            new_edges.push(DependencyEdge {
                from_module: last_segment(from_pkg).to_string(),
                to_module: last_segment(to_pkg).to_string(),
                import_path: import_path.clone(),
                is_new: true,
            });
        }
    }

    DependencyGraph {
        mermaid_source: lines.join("\n"),
        new_edges,
        existing_edges: Vec::new(),
    }
}

// ─── Risk score ───────────────────────────────────────────────────────────────

fn calculate_risk(
    flat_files: &[FlatFile],
    api_changes: &[ApiChange],
    new_dependencies: &[String],
    full_diff: &str,
) -> (u32, Vec<String>) {
    let mut score: u32 = 0;
    let mut factors: Vec<String> = Vec::new();

    // Per-file scoring
    for file in flat_files {
        match file.change_type.as_str() {
            "deleted" => {
                score += 8;
                factors.push(format!("Deleted: {}", file.path));
            }
            "modified" => score += 5,
            "added" => score += 3,
            _ => {}
        }
        let total_lines = file.additions + file.deletions;
        if total_lines > 100 {
            score += 5;
            factors.push(format!("Large change: {} ({} lines)", file.path, total_lines));
        }
    }

    // API removals
    let removed_count = api_changes
        .iter()
        .filter(|a| a.change_type == "removed")
        .count() as u32;
    if removed_count > 0 {
        let penalty = (removed_count * 10).min(30);
        score += penalty;
        factors.push(format!("Removed {} public API(s)", removed_count));
    }

    // New dependencies
    if !new_dependencies.is_empty() {
        let penalty = ((new_dependencies.len() as u32) * 15).min(30);
        score += penalty;
        factors.push(format!("Added {} new dependency(ies)", new_dependencies.len()));
    }

    // Infrastructure files touched
    let infra_patterns = [
        "Dockerfile",
        "docker-compose",
        ".github/",
        "Makefile",
        ".env",
        "ci.yml",
        "cd.yml",
        "workflow",
    ];
    let infra_touched = flat_files.iter().any(|f| {
        infra_patterns.iter().any(|p| f.path.contains(p))
    });
    if infra_touched {
        score += 10;
        factors.push("Infrastructure/CI files changed".to_string());
    }

    // Large diff overall
    if full_diff.len() > 20_000 {
        score += 5;
        factors.push("Very large diff (>20KB)".to_string());
    }

    let final_score = score.min(100);
    (final_score, factors)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn empty_diff(base: &str, head: &str) -> ArchitectureDiff {
    ArchitectureDiff {
        phase: String::new(),
        base_commit: base.to_string(),
        head_commit: head.to_string(),
        summary: ChangeSummary {
            files_added: 0,
            files_modified: 0,
            files_deleted: 0,
            lines_added: 0,
            lines_removed: 0,
            new_dependencies: Vec::new(),
            risk_score: 0,
            risk_factors: Vec::new(),
        },
        file_tree: Vec::new(),
        dependency_graph: DependencyGraph {
            mermaid_source: String::new(),
            new_edges: Vec::new(),
            existing_edges: Vec::new(),
        },
        api_surface: Vec::new(),
    }
}
