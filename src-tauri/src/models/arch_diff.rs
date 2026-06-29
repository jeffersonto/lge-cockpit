use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureDiff {
    pub phase: String,
    pub base_commit: String,
    pub head_commit: String,
    pub summary: ChangeSummary,
    pub file_tree: Vec<FileNode>,
    pub dependency_graph: DependencyGraph,
    pub api_surface: Vec<ApiChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSummary {
    pub files_added: u32,
    pub files_modified: u32,
    pub files_deleted: u32,
    pub lines_added: u32,
    pub lines_removed: u32,
    pub new_dependencies: Vec<String>,
    pub risk_score: u32,
    pub risk_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub path: String,
    pub change_type: String, // "added" | "modified" | "deleted"
    pub additions: u32,
    pub deletions: u32,
    pub is_directory: bool,
    pub children: Vec<FileNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub mermaid_source: String,
    pub new_edges: Vec<DependencyEdge>,
    pub existing_edges: Vec<DependencyEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from_module: String,
    pub to_module: String,
    pub import_path: String,
    pub is_new: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiChange {
    pub file: String,
    pub symbol: String,
    pub kind: String,        // "function" | "type" | "interface" | "struct" | "enum" | "trait" | "class"
    pub change_type: String, // "added" | "modified" | "removed"
    pub signature: Option<String>,
}
