import { invoke } from "@tauri-apps/api/core";
import type { Repository, Task, CreateTaskInput, LgePhaseResult, HealthCheckResult, StaleWorktreeInfo, ArchitectureDiff, TaskAttachment, DeleteTaskResult, ProjectDeletePreview, JiraSelf } from "../types";

export async function listRepositories(): Promise<Repository[]> {
  return invoke("list_repositories");
}

export async function addRepository(path: string): Promise<Repository> {
  return invoke("add_repository", { path });
}

export async function removeRepository(id: string): Promise<void> {
  return invoke("remove_repository", { id });
}

export async function listTasks(repositoryId: string): Promise<Task[]> {
  return invoke("list_tasks", { repositoryId });
}

export async function createTask(input: CreateTaskInput): Promise<Task> {
  return invoke("create_task", {
    repositoryId: input.repository_id,
    title: input.title,
    description: input.description ?? null,
  });
}

export async function updateTaskStatus(
  id: string,
  status: string
): Promise<Task> {
  return invoke("update_task_status", { id, status });
}

export async function updateTask(input: {
  id: string;
  title: string;
  description?: string;
}): Promise<Task> {
  return invoke("update_task", {
    id: input.id,
    title: input.title,
    description: input.description ?? null,
  });
}

export async function deleteTask(id: string): Promise<DeleteTaskResult> {
  return invoke("delete_task", { id });
}

export async function importJiraTask(
  repositoryId: string,
  jiraKey: string
): Promise<Task> {
  return invoke("import_jira_task", { repositoryId, jiraKey });
}

export async function verifyJiraConnection(): Promise<JiraSelf> {
  return invoke("verify_jira_connection");
}

export async function runLgePhase(
  taskId: string,
  phase: string,
  taskTitle: string,
  taskDescription: string,
  extraContext?: string
): Promise<LgePhaseResult> {
  return invoke("run_lge_phase", {
    taskId,
    phase,
    taskTitle,
    taskDescription,
    extraContext: extraContext || null,
  });
}

export async function checkDependencies(): Promise<HealthCheckResult> {
  return invoke("check_dependencies");
}

export async function cancelLgePhase(
  taskId: string,
  phase: string
): Promise<void> {
  return invoke("cancel_lge_phase", { taskId, phase });
}

export async function loadLgeArtifacts(
  taskId: string
): Promise<Record<string, string>> {
  return invoke("load_lge_artifacts", { taskId });
}

export async function saveLgeArtifact(
  taskId: string,
  phase: string,
  content: string
): Promise<void> {
  return invoke("save_lge_artifact", { taskId, phase, content });
}

export async function getCurrentGitBranch(repoPath: string): Promise<string> {
  return invoke("get_current_git_branch", { repoPath });
}

export async function createGitBranch(
  taskId: string,
  repoPath: string,
  branchName: string,
  baseBranch: string
): Promise<string> {
  return invoke("create_git_branch", { taskId, repoPath, branchName, baseBranch });
}

export async function commitAndPush(
  taskId: string,
  branchName: string,
  message: string
): Promise<string> {
  return invoke("commit_and_push", { taskId, branchName, message });
}

export async function createPullRequest(
  taskId: string,
  baseBranch: string
): Promise<string> {
  return invoke("create_pull_request", { taskId, baseBranch });
}

export async function getPhaseModels(): Promise<Record<string, string>> {
  return invoke("get_phase_models");
}

export async function savePhaseModels(
  models: Record<string, string>
): Promise<void> {
  return invoke("save_phase_models", { models });
}

export async function generateCommitMessage(
  taskId: string,
  taskTitle: string,
  jiraKey: string | null
): Promise<string> {
  return invoke("generate_commit_message", { taskId, taskTitle, jiraKey });
}

export async function removeWorktree(taskId: string): Promise<void> {
  return invoke("remove_worktree", { taskId });
}

export async function removeCompletedWorktrees(repositoryId: string): Promise<string[]> {
  return invoke("remove_completed_worktrees", { repositoryId });
}

export async function checkStaleWorktrees(): Promise<StaleWorktreeInfo[]> {
  return invoke("check_stale_worktrees");
}

export async function openInEditor(path: string): Promise<void> {
  return invoke("open_in_editor", { path });
}

export async function getShellEnv(): Promise<string> {
  return invoke("get_shell_env");
}

export async function saveShellEnv(shellEnv: string): Promise<void> {
  return invoke("save_shell_env", { shellEnv });
}

export async function getJiraBaseUrl(): Promise<string> {
  return invoke("get_jira_base_url");
}

export async function saveJiraBaseUrl(jiraBaseUrl: string): Promise<void> {
  return invoke("save_jira_base_url", { jiraBaseUrl });
}

export async function getJiraEmail(): Promise<string> {
  return invoke("get_jira_email");
}

export async function saveJiraEmail(jiraEmail: string): Promise<void> {
  return invoke("save_jira_email", { jiraEmail });
}

export async function getJiraApiToken(): Promise<string> {
  return invoke("get_jira_api_token");
}

export async function saveJiraApiToken(jiraApiToken: string): Promise<void> {
  return invoke("save_jira_api_token", { jiraApiToken });
}

export async function getHeadCommit(taskId: string): Promise<string> {
  return invoke("get_head_commit", { taskId });
}

export async function analyzeArchitectureDiff(
  taskId: string,
  baseCommit: string,
  headCommit: string
): Promise<ArchitectureDiff> {
  return invoke("analyze_architecture_diff", { taskId, baseCommit, headCommit });
}

export async function analyzeWorkingTreeDiff(taskId: string): Promise<ArchitectureDiff> {
  return invoke("analyze_working_tree_diff", { taskId });
}

export async function addTaskAttachment(
  taskId: string,
  filePath: string,
  injectionPhases: string[]
): Promise<TaskAttachment> {
  return invoke("add_task_attachment", { taskId, filePath, injectionPhases });
}

export async function listTaskAttachments(taskId: string): Promise<TaskAttachment[]> {
  return invoke("list_task_attachments", { taskId });
}

export async function removeTaskAttachment(attachmentId: string): Promise<void> {
  return invoke("remove_task_attachment", { attachmentId });
}

export async function setAttachmentPhases(
  attachmentId: string,
  injectionPhases: string[]
): Promise<void> {
  return invoke("set_attachment_phases", { attachmentId, injectionPhases });
}

export async function getProjectDeletePreview(id: string): Promise<ProjectDeletePreview> {
  return invoke("get_project_delete_preview", { id });
}
