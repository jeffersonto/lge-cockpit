//! The DiffAnalysis module — owns the pure transformation of git diff
//! outputs (name-status, numstat, and full diff) into a structured
//! [`DiffAnalysis`]. Lifts the ~800 lines of pure parsers, file-tree
//! builders, dependency-graph builders, and risk scoring out of
//! `commands/arch_diff.rs` so the two `#[tauri::command]`s shrink to
//! git-plumbing adapters that collect strings and call [`analyze`].
//!
//! Boundary: pure. No Tauri, no git, no filesystem. Callers feed three
//! strings in (plus base/head labels and an optional diff-size cap) and
//! receive a fully-populated `DiffAnalysis` out. Tests can drive the whole
//! surface from in-process fixtures in `tests/`.
//!
//! See `CONTEXT.md` for the design.

use std::collections::{HashMap, HashSet};

use crate::models::arch_diff::{
    ApiChange, ChangeSummary, DependencyEdge, DependencyGraph, FileNode,
};

/// Max nodes in the Mermaid dependency diagram. Internal — not tunable from
/// the caller; only `max_diff_bytes` (which affects parse correctness) is
/// part of the public surface via [`AnalysisInput`].
const MAX_MERMAID_NODES: usize = 30;

/// Input bundle for [`analyze`]: three git outputs plus base/head labels and
/// the caller-chosen diff-size cap. Grouping prevents accidental argument
/// reordering (three of the fields would otherwise be positional `&str`).
#[derive(Debug, Clone)]
pub struct AnalysisInput {
    pub base: String,
    pub head: String,
    pub name_status: String,
    pub numstat: String,
    pub full_diff: String,
    /// Maximum bytes of `full_diff` the analyzer will read. The caller is
    /// responsible for any pre-truncation; this field is consumed but not
    /// applied — pass the already-truncated string. Kept as a field so tests
    /// can document what cap was used.
    pub max_diff_bytes: usize,
}

/// The fully-resolved diff analysis. Deliberately does NOT carry a `phase`
/// field — that's an LGE-pipeline concept owned by the IPC adapter, not by
/// this pure module. The adapter wraps this into [`crate::models::arch_diff::ArchitectureDiff`]
/// when crossing the IPC seam.
#[derive(Debug, Clone)]
pub struct DiffAnalysis {
    pub base_commit: String,
    pub head_commit: String,
    pub summary: ChangeSummary,
    pub file_tree: Vec<FileNode>,
    pub dependency_graph: DependencyGraph,
    pub api_surface: Vec<ApiChange>,
}

// ─── Public entry ───────────────────────────────────────────────────────────

/// Analyzes a set of git diff strings into a structured `DiffAnalysis`.
/// Pure: no IO. Applies `input.max_diff_bytes` to `full_diff` before
/// parsing — callers (e.g. the IPC adapter) pass the raw diff and the cap;
/// tests can pass a small cap to exercise truncation with a short fixture.
pub fn analyze(input: &AnalysisInput) -> DiffAnalysis {
    let full_diff: String = input.full_diff.chars().take(input.max_diff_bytes).collect();
    let file_changes = parse_name_status(&input.name_status);
    let line_counts = parse_numstat(&input.numstat);

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

    let files_added = flat_files.iter().filter(|f| f.change_type == "added").count() as u32;
    let files_modified = flat_files.iter().filter(|f| f.change_type == "modified").count() as u32;
    let files_deleted = flat_files.iter().filter(|f| f.change_type == "deleted").count() as u32;
    let lines_added: u32 = flat_files.iter().map(|f| f.additions).sum();
    let lines_removed: u32 = flat_files.iter().map(|f| f.deletions).sum();

    let (new_import_edges, api_changes) = parse_diff_content(&full_diff, &flat_files);
    let new_dependencies = detect_new_dependencies(&full_diff);

    flat_files.sort_by(|a, b| a.path.cmp(&b.path));
    let file_tree = build_file_tree(&flat_files);
    let dependency_graph = build_dependency_graph(&flat_files, &new_import_edges);
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

    DiffAnalysis {
        base_commit: input.base.clone(),
        head_commit: input.head.clone(),
        summary,
        file_tree,
        dependency_graph,
        api_surface: api_changes,
    }
}

/// Convenience for adapters that need an empty result (e.g. when `git status`
/// is clean or `base_commit == head_commit`). Returns a `DiffAnalysis` with
/// zero counts and no edges.
pub fn empty(base: &str, head: &str) -> DiffAnalysis {
    DiffAnalysis {
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

// ─── Internal structs ──────────────────────────────────────────────────────

struct FlatFile {
    path: String,
    change_type: String,
    additions: u32,
    deletions: u32,
}

struct ImportEdge {
    from_file: String,
    import_path: String,
}

// ─── Parsers ───────────────────────────────────────────────────────────────

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

// ─── Diff content parsing ──────────────────────────────────────────────────

fn parse_diff_content(diff: &str, flat_files: &[FlatFile]) -> (Vec<ImportEdge>, Vec<ApiChange>) {
    let mut import_edges: Vec<ImportEdge> = Vec::new();
    let mut api_added: HashMap<String, (String, String, Option<String>)> = HashMap::new();
    let mut api_removed: HashSet<String> = HashSet::new();

    let mut current_file = String::new();

    for line in diff.lines() {
        if line.starts_with("diff --git ") {
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
            if let Some(import_path) = detect_import(content) {
                if !current_file.is_empty() && !import_path.is_empty() {
                    import_edges.push(ImportEdge {
                        from_file: current_file.clone(),
                        import_path,
                    });
                }
            }

            if let Some((symbol, kind, sig)) = detect_api_symbol(content) {
                api_added.insert(symbol.clone(), (current_file.clone(), kind, sig));
                api_removed.remove(&symbol);
            }
        } else {
            if let Some((symbol, kind, sig)) = detect_api_symbol(content) {
                if !api_added.contains_key(&symbol) {
                    api_removed.insert(symbol.clone());
                    api_added.entry(symbol).or_insert((current_file.clone(), kind, sig));
                }
            }
        }
    }

    let api_changes: Vec<ApiChange> = {
        let mut changes = Vec::new();

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

    if trimmed.starts_with("use ") && trimmed.contains("::") {
        let after = &trimmed[4..];
        let path = after.split('{').next()?.trim().trim_end_matches(';').to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }

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

    if trimmed.starts_with("export ") {
        let rest = &trimmed[7..];
        let (kind, rest) = if rest.starts_with("default ") {
            return None;
        } else if rest.starts_with("async function ") {
            ("function", &rest[15..])
        } else if rest.starts_with("function ") {
            ("function", &rest[9..])
        } else if rest.starts_with("const ") {
            ("function", &rest[6..])
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
        let sig: String = trimmed.chars().take(100).collect();
        return Some((name, kind.to_string(), Some(sig)));
    }

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

    if trimmed.starts_with("func ") {
        let after = &trimmed[5..];
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

// ─── Dependency detection ──────────────────────────────────────────────────

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

        if content.contains('"') && content.contains(':') {
            let parts: Vec<&str> = content.splitn(2, ':').collect();
            if parts.len() == 2 {
                let name = parts[0].trim().trim_matches('"').trim();
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

// ─── File tree ─────────────────────────────────────────────────────────────

fn build_file_tree(files: &[FlatFile]) -> Vec<FileNode> {
    let mut root: Vec<FileNode> = Vec::new();

    for file in files {
        let segments: Vec<&str> = file.path.split('/').collect();
        insert_into_tree(&mut root, &segments, file);
    }

    root
}

fn insert_into_tree(nodes: &mut Vec<FileNode>, segments: &[&str], file: &FlatFile) {
    if segments.len() == 1 {
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

    let dir_name = segments[0];
    let dir_path = file.path.splitn(segments.len(), '/').next().unwrap_or(dir_name).to_string();

    if let Some(dir_node) = nodes.iter_mut().find(|n| n.is_directory && n.path == dir_path) {
        insert_into_tree(&mut dir_node.children, &segments[1..], file);
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

// ─── Mermaid diagram ───────────────────────────────────────────────────────

fn parent_dir(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => path,
    }
}

fn last_segment(path: &str) -> &str {
    path.trim_end_matches('/').split('/').last().unwrap_or(path)
}

fn node_id(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn build_dependency_graph(flat_files: &[FlatFile], import_edges: &[ImportEdge]) -> DependencyGraph {
    let mut pkg_change_map: HashMap<String, &str> = HashMap::new();
    for f in flat_files {
        let pkg = parent_dir(&f.path).to_string();
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

    let resolved_edges: Vec<(String, String, String)> = import_edges
        .iter()
        .filter_map(|edge| {
            let from_pkg = parent_dir(&edge.from_file).to_string();
            let import = edge.import_path.trim_end_matches('/');
            if !import.contains('/') {
                return None;
            }
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

    let mut extra_pkgs: Vec<String> = resolved_edges
        .iter()
        .filter_map(|(_, to, _)| {
            if !all_pkgs.contains(to) { Some(to.clone()) } else { None }
        })
        .collect();
    extra_pkgs.sort();
    extra_pkgs.dedup();

    let mut nodes = all_pkgs.clone();
    for ep in extra_pkgs {
        if nodes.len() >= MAX_MERMAID_NODES { break; }
        nodes.push(ep);
    }

    let mut subgraph_map: HashMap<String, Vec<String>> = HashMap::new();
    for pkg in &nodes {
        let segments: Vec<&str> = pkg.split('/').collect();
        let group = if segments.len() >= 2 {
            segments[segments.len() - 2].to_string()
        } else {
            segments.last().copied().unwrap_or(pkg).to_string()
        };
        subgraph_map.entry(group).or_default().push(pkg.clone());
    }

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

    let mut seen: HashSet<String> = HashSet::new();
    let mut new_edges: Vec<DependencyEdge> = Vec::new();

    for (from_pkg, to_pkg, import_path) in &resolved_edges {
        let key = format!("{}->{}", from_pkg, to_pkg);
        if seen.contains(&key) { continue; }
        seen.insert(key);

        let from_id = node_id(from_pkg);
        let to_id = node_id(to_pkg);

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

// ─── Risk score ───────────────────────────────────────────────────────────

fn calculate_risk(
    flat_files: &[FlatFile],
    api_changes: &[ApiChange],
    new_dependencies: &[String],
    full_diff: &str,
) -> (u32, Vec<String>) {
    let mut score: u32 = 0;
    let mut factors: Vec<String> = Vec::new();

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

    let removed_count = api_changes
        .iter()
        .filter(|a| a.change_type == "removed")
        .count() as u32;
    if removed_count > 0 {
        let penalty = (removed_count * 10).min(30);
        score += penalty;
        factors.push(format!("Removed {} public API(s)", removed_count));
    }

    if !new_dependencies.is_empty() {
        let penalty = ((new_dependencies.len() as u32) * 15).min(30);
        score += penalty;
        factors.push(format!("Added {} new dependency(ies)", new_dependencies.len()));
    }

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

    if full_diff.len() > 20_000 {
        score += 5;
        factors.push("Very large diff (>20KB)".to_string());
    }

    let final_score = score.min(100);
    (final_score, factors)
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn input(base: &str, head: &str, name_status: &str, numstat: &str, full_diff: &str) -> AnalysisInput {
        AnalysisInput {
            base: base.to_string(),
            head: head.to_string(),
            name_status: name_status.to_string(),
            numstat: numstat.to_string(),
            full_diff: full_diff.to_string(),
            max_diff_bytes: 50_000,
        }
    }

    // --- parse_name_status (via analyze) ----------------------------------

    #[test]
    fn parses_added_modified_deleted() {
        let ns = "A\tsrc/new.ts\nM\tsrc/old.ts\nD\tsrc/gone.ts";
        let out = analyze(&input("b", "h", ns, "", ""));
        assert_eq!(out.summary.files_added, 1);
        assert_eq!(out.summary.files_modified, 1);
        assert_eq!(out.summary.files_deleted, 1);
    }

    #[test]
    fn ignores_malformed_name_status_lines() {
        let ns = "garbage\nA\t\n?? untracked";
        let out = analyze(&input("b", "h", ns, "", ""));
        assert_eq!(out.summary.files_added + out.summary.files_modified + out.summary.files_deleted, 1);
    }

    // --- parse_numstat (via analyze) --------------------------------------

    #[test]
    fn numstat_attaches_line_counts_to_files() {
        let ns = "A\tsrc/a.ts\nM\tsrc/b.ts";
        let numstat = "10\t2\tsrc/a.ts\n5\t3\tsrc/b.ts";
        let out = analyze(&input("b", "h", ns, numstat, ""));
        assert_eq!(out.summary.lines_added, 15);
        assert_eq!(out.summary.lines_removed, 5);
    }

    #[test]
    fn numstat_missing_for_a_file_defaults_to_zero() {
        let ns = "A\tsrc/a.ts";
        let numstat = "";
        let out = analyze(&input("b", "h", ns, numstat, ""));
        assert_eq!(out.summary.lines_added, 0);
        assert_eq!(out.summary.lines_removed, 0);
    }

    // --- detect_new_dependencies ------------------------------------------

    #[test]
    fn detects_new_package_json_dependency() {
        let diff = include_str!("fixtures/package_json_added_dep.txt");
        let ns = "A\tpackage.json";
        let out = analyze(&input("b", "h", ns, "", diff));
        assert!(out.summary.new_dependencies.iter().any(|d| d.contains("react")));
    }

    #[test]
    fn detects_new_cargo_dependency() {
        let diff = include_str!("fixtures/cargo_toml_added_dep.txt");
        let ns = "M\tCargo.toml";
        let out = analyze(&input("b", "h", ns, "", diff));
        assert!(out.summary.new_dependencies.iter().any(|d| d.contains("tokio")));
    }

    #[test]
    fn ignores_dep_files_when_not_in_diff() {
        let diff = "diff --git a/src/foo.ts b/src/foo.ts\n+export const x = 1;\n";
        let out = analyze(&input("b", "h", "M\tsrc/foo.ts", "", diff));
        assert!(out.summary.new_dependencies.is_empty());
    }

    // --- detect_api_symbol ------------------------------------------------

    #[test]
    fn detects_exported_ts_function_addition() {
        let diff = "diff --git a/src/foo.ts b/src/foo.ts\n+export function foo() { return 1 }\n";
        let out = analyze(&input("b", "h", "M\tsrc/foo.ts", "", diff));
        let api = out.api_surface.iter().find(|a| a.symbol == "foo").unwrap();
        assert_eq!(api.kind, "function");
        assert_eq!(api.change_type, "added");
    }

    #[test]
    fn detects_rust_pub_struct_addition() {
        let diff = "diff --git a/src/lib.rs b/src/lib.rs\n+pub struct Bar { x: u32 }\n";
        let out = analyze(&input("b", "h", "M\tsrc/lib.rs", "", diff));
        assert!(out.api_surface.iter().any(|a| a.symbol == "Bar" && a.kind == "struct"));
    }

    #[test]
    fn ignores_default_exports() {
        let diff = "diff --git a/src/foo.ts b/src/foo.ts\n+export default function () {}\n";
        let out = analyze(&input("b", "h", "M\tsrc/foo.ts", "", diff));
        assert!(out.api_surface.is_empty());
    }

    // --- detect_import + dependency graph ---------------------------------

    #[test]
    fn builds_edge_when_import_targets_changed_package() {
        // Two files in different packages; one imports from the other's package
        // path, which is how TS path-mapped / Go-like imports name a package dir.
        let ns = "A\tpkg/foo.ts\nA\tpkg/sub/bar.ts";
        let numstat = "1\t0\tpkg/foo.ts\n1\t0\tpkg/sub/bar.ts";
        let diff = "diff --git a/pkg/foo.ts b/pkg/foo.ts\n+import { thing } from 'pkg/sub';\n";
        let out = analyze(&input("b", "h", ns, numstat, diff));
        assert!(
            out.dependency_graph.new_edges.iter().any(|e| e.from_module == "pkg" && e.to_module == "sub"),
            "expected an edge pkg -> sub, got {:?}",
            out.dependency_graph.new_edges
        );
    }

    #[test]
    fn dependency_graph_is_empty_when_no_changes() {
        let out = analyze(&input("b", "h", "", "", ""));
        assert!(out.dependency_graph.mermaid_source.is_empty());
        assert!(out.dependency_graph.new_edges.is_empty());
    }

    // --- risk score -------------------------------------------------------

    #[test]
    fn risk_score_caps_at_100() {
        // Many deleted files to blow past 100.
        let mut ns = String::new();
        for i in 0..30 {
            if !ns.is_empty() { ns.push('\n'); }
            ns.push_str(&format!("D\tfile{}", i));
        }
        let out = analyze(&input("b", "h", &ns, "", ""));
        assert_eq!(out.summary.risk_score, 100);
        assert!(!out.summary.risk_factors.is_empty());
    }

    #[test]
    fn risk_score_zero_on_clean_diff() {
        let out = analyze(&input("b", "h", "", "", ""));
        assert_eq!(out.summary.risk_score, 0);
    }

    #[test]
    fn risk_score_flags_infra_files() {
        let out = analyze(&input("b", "h", "M\t.github/workflows/ci.yml", "", ""));
        assert!(out.summary.risk_factors.iter().any(|f| f.contains("Infrastructure")));
    }

    // --- file tree --------------------------------------------------------

    #[test]
    fn file_tree_groups_by_directory() {
        let ns = "A\tsrc/a.ts\nA\tsrc/b.ts";
        let out = analyze(&input("b", "h", ns, "", ""));
        assert_eq!(out.file_tree.len(), 1);
        let dir = &out.file_tree[0];
        assert!(dir.is_directory);
        assert_eq!(dir.path, "src");
        assert_eq!(dir.children.len(), 2);
    }

    // --- empty() ----------------------------------------------------------

    #[test]
    fn empty_helper_returns_zero_counts() {
        let out = empty("base", "head");
        assert_eq!(out.base_commit, "base");
        assert_eq!(out.head_commit, "head");
        assert_eq!(out.summary.files_added, 0);
        assert_eq!(out.summary.risk_score, 0);
        assert!(out.api_surface.is_empty());
        assert!(out.file_tree.is_empty());
        assert!(out.dependency_graph.new_edges.is_empty());
    }

    // --- max_diff_bytes injection ----------------------------------------

    #[test]
    fn max_diff_bytes_truncates_full_diff_before_parsing() {
        // A diff whose only API symbol lives past byte 50.
        let long_padding = "x".repeat(60);
        let diff = format!(
            "diff --git a/src/foo.ts b/src/foo.ts\n+{}\n+export function farPastCap(): void {{}}\n",
            long_padding
        );
        let ns = "M\tsrc/foo.ts";
        let mut inp = input("b", "h", ns, "", &diff);
        inp.max_diff_bytes = 50;
        let out = analyze(&inp);
        // The `export function` line begins past the cap, so the symbol
        // is truncated away and never detected.
        assert!(out.api_surface.iter().all(|a| a.symbol != "farPastCap"));
    }
}