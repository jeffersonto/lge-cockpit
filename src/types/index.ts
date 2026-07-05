export type TaskStatus = "pending" | "in_progress" | "completed";
export type TaskSource = "manual" | "jira";
export type InjectionPhase = "planning" | "builder" | "review" | "guardian";

export interface TaskAttachment {
  id: string;
  task_id: string;
  file_name: string;
  file_size: number;
  mime_type: string;
  content: string;
  injection_phases: InjectionPhase[];
  created_at: string;
}

export interface Repository {
  id: string;
  name: string;
  path: string;
  created_at: string;
  updated_at: string;
  max_worktrees: number;
  active_worktree_count: number;
}

export interface Task {
  id: string;
  repository_id: string;
  title: string;
  description: string | null;
  status: TaskStatus;
  source: TaskSource;
  jira_key: string | null;
  jira_url: string | null;
  git_branch: string | null;
  worktree_path: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateTaskInput {
  repository_id: string;
  title: string;
  description?: string;
}

export interface ImportJiraInput {
  repository_id: string;
  jira_key: string;
}

// LGE Process types
export type LgePhaseId = "planning" | "builder" | "review" | "guardian";
export type LgePhaseStatus = "pending" | "running" | "queued" | "completed" | "failed";

// Health check types
export interface DependencyStatus {
  name: string;
  available: boolean;
  path: string | null;
  version: string | null;
  install_command: string | null;
  description: string;
}

export interface HealthCheckResult {
  all_ok: boolean;
  dependencies: DependencyStatus[];
}

export interface StaleWorktreeInfo {
  task_id: string;
  task_title: string;
  jira_key: string | null;
  worktree_path: string;
  repository_id: string;
  repository_name: string;
}

export interface LgePhaseResult {
  phase: string;
  artifact_content: string;
  artifact_path: string;
}

// Architecture Diff types
export interface ArchitectureDiff {
  phase: string;
  base_commit: string;
  head_commit: string;
  summary: ChangeSummary;
  file_tree: FileNode[];
  dependency_graph: DependencyGraph;
  api_surface: ApiChange[];
}

export interface ChangeSummary {
  files_added: number;
  files_modified: number;
  files_deleted: number;
  lines_added: number;
  lines_removed: number;
  new_dependencies: string[];
  risk_score: number;
  risk_factors: string[];
}

export interface FileNode {
  path: string;
  change_type: "added" | "modified" | "deleted";
  additions: number;
  deletions: number;
  is_directory: boolean;
  children: FileNode[];
}

export interface DependencyGraph {
  mermaid_source: string;
  new_edges: DependencyEdge[];
  existing_edges: DependencyEdge[];
}

export interface DependencyEdge {
  from_module: string;
  to_module: string;
  import_path: string;
  is_new: boolean;
}

export interface ApiChange {
  file: string;
  symbol: string;
  kind: string;
  change_type: "added" | "modified" | "removed";
  signature: string | null;
}

export interface DeleteTaskResult {
  worktree_cleaned: boolean;
  branch_cleaned: boolean;
  worktree_path: string | null;
  branch_name: string | null;
  errors: string[];
}

export interface ProjectDeletePreview {
  task_count: number;
  worktree_count: number;
  branch_count: number;
}

export interface JiraSelf {
  account_id: string;
  display_name: string;
  email: string;
}

export interface JiraConfig {
  baseUrl: string;
  email: string;
  apiToken: string;
}
