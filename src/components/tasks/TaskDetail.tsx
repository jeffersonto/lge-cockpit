import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useTaskStore } from "../../stores/taskStore";
import { useLgeStore, PHASE_ORDER } from "../../stores/lgeStore";
import { useRepositoryStore } from "../../stores/repositoryStore";
import { Badge } from "../ui/Badge";
import { Button } from "../ui/Button";
import { Input, TextArea } from "../ui/Input";
import { Dialog } from "../ui/Dialog";
import type { InjectionPhase, TaskStatus } from "../../types";
import * as api from "../../lib/tauri";
import { fixMarkdownTables } from "../../lib/markdown";

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, "")
    .trim()
    .replace(/\s+/g, "-")
    .slice(0, 40);
}

const STATUS_STYLES: Record<
  TaskStatus,
  { color: string; bgColor: string }
> = {
  pending: { color: "text-text-muted", bgColor: "bg-text-muted/20" },
  in_progress: { color: "text-warning", bgColor: "bg-warning/20" },
  completed: { color: "text-success", bgColor: "bg-success/20" },
};

const PHASE_LABELS: Record<InjectionPhase, string> = {
  planning: "Planning",
  builder: "Builder",
  review: "Review",
  guardian: "Guardian",
};

const PHASE_COLORS: Record<InjectionPhase, { color: string; bgColor: string }> = {
  planning: { color: "text-blue-400", bgColor: "bg-blue-400/20" },
  builder: { color: "text-accent", bgColor: "bg-accent/20" },
  review: { color: "text-warning", bgColor: "bg-warning/20" },
  guardian: { color: "text-success", bgColor: "bg-success/20" },
};

export function TaskDetail() {
  const { t } = useTranslation();
  const { tasks, selectedTaskId, selectTask, updateTask, createGitBranch, removeWorktree, attachments, fetchAttachments, addAttachment, removeAttachment, setAttachmentPhases } = useTaskStore();
  const lgeStore = useLgeStore();
  const { repositories, selectedRepoId } = useRepositoryStore();

  const task = tasks.find((t) => t.id === selectedTaskId);

  const [existingArtifacts, setExistingArtifacts] = useState<Record<string, string>>({});
  const [loadingArtifacts, setLoadingArtifacts] = useState(true);

  const [isEditing, setIsEditing] = useState(false);
  const [editTitle, setEditTitle] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [saving, setSaving] = useState(false);

  const renderedDescription = useMemo(
    () => (task?.description ? fixMarkdownTables(task.description) : null),
    [task?.description]
  );

  const [lgeContextOpen, setLgeContextOpen] = useState(false);
  const [lgeContextText, setLgeContextText] = useState("");

  const [copiedPath, setCopiedPath] = useState(false);
  const [branchDialogOpen, setBranchDialogOpen] = useState(false);
  const [branchName, setBranchName] = useState("");
  const [baseBranch, setBaseBranch] = useState("develop");
  const [branchLoading, setBranchLoading] = useState(false);
  const [branchError, setBranchError] = useState<string | null>(null);

  const [attachmentError, setAttachmentError] = useState<string | null>(null);
  const [attachmentLoading, setAttachmentLoading] = useState(false);

  const handleEditStart = () => {
    setEditTitle(task!.title);
    setEditDescription(task!.description ?? "");
    setIsEditing(true);
  };

  const handleEditSave = async () => {
    if (!task || !editTitle.trim()) return;
    setSaving(true);
    try {
      await updateTask(task.id, editTitle.trim(), editDescription.trim() || undefined);
      setIsEditing(false);
    } catch {
      // error already logged in store
    } finally {
      setSaving(false);
    }
  };

  const handleEditCancel = () => {
    setIsEditing(false);
  };

  const handleOpenBranchDialog = () => {
    if (task) {
      const suggestion = task.jira_key
        ? `feature/${task.jira_key}`
        : `feature/${slugify(task.title)}`;
      setBranchName(suggestion);
    }
    setBranchError(null);
    setBranchDialogOpen(true);
  };

  const handleCreateBranch = async () => {
    if (!task || !branchName.trim()) return;
    const repo = repositories.find((r) => r.id === (selectedRepoId ?? task.repository_id));
    if (!repo) return;
    setBranchLoading(true);
    setBranchError(null);
    try {
      await createGitBranch(task.id, repo.path, branchName.trim(), baseBranch);
      lgeStore.setBranchCreated(task.id, true);
      setBranchDialogOpen(false);
    } catch (err) {
      setBranchError(String(err));
    } finally {
      setBranchLoading(false);
    }
  };

  const handleAttachFile = async () => {
    if (!task) return;
    setAttachmentError(null);
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Documents", extensions: ["md", "txt", "json", "csv", "pdf", "docx"] }],
      });
      if (!selected) return;
      setAttachmentLoading(true);
      await addAttachment(task.id, selected as string, ["planning"]);
    } catch (err) {
      setAttachmentError(String(err));
    } finally {
      setAttachmentLoading(false);
    }
  };

  const handleRemoveAttachment = async (attachmentId: string) => {
    if (!task) return;
    setAttachmentError(null);
    try {
      await removeAttachment(task.id, attachmentId);
    } catch (err) {
      setAttachmentError(String(err));
    }
  };

  const handleTogglePhase = async (attachmentId: string, currentPhases: InjectionPhase[], phase: InjectionPhase) => {
    if (!task) return;
    const next = currentPhases.includes(phase)
      ? currentPhases.filter((p) => p !== phase)
      : [...currentPhases, phase];
    if (next.length === 0) return; // must keep at least one
    try {
      await setAttachmentPhases(task.id, attachmentId, next);
    } catch (err) {
      setAttachmentError(String(err));
    }
  };

  // Check if this task has a running LGE process in memory
  const activeProcess = task ? lgeStore.getProcess(task.id) : null;
  const hasRunningPhase = task ? lgeStore.isAnyPhaseRunning(task.id) : false;

  useEffect(() => {
    if (task?.id) {
      setLoadingArtifacts(true);
      lgeStore.loadExistingArtifacts(task.id).then((artifacts) => {
        setExistingArtifacts(artifacts);
        setLoadingArtifacts(false);
      });
      fetchAttachments(task.id);
    }
  }, [task?.id]);

  if (!task) {
    return (
      <div className="flex flex-1 items-center justify-center">
        <p className="text-sm text-text-muted">Task not found</p>
      </div>
    );
  }

  const statusStyle = STATUS_STYLES[task.status];
  const hasArtifacts = Object.keys(existingArtifacts).length > 0 || activeProcess !== null;
  const nextPhase = lgeStore.getNextPhaseFromArtifacts(existingArtifacts);

  const handleStartLge = () => {
    lgeStore.startProcess(task.id, task.title, task.description ?? "");
    lgeStore.runPhase(task.id, "planning", lgeContextText.trim() || undefined);
    setLgeContextText("");
    setLgeContextOpen(false);
  };

  const handleResumeLge = () => {
    if (!activeProcess) {
      lgeStore.resumeProcess(task.id, task.title, task.description ?? "", existingArtifacts, task.git_branch);
    }
    if (nextPhase) {
      lgeStore.runPhase(task.id, nextPhase, lgeContextText.trim() || undefined);
      setLgeContextText("");
      setLgeContextOpen(false);
    } else {
      lgeStore.openView(task.id);
    }
  };

  const handleViewLge = () => {
    if (!activeProcess) {
      lgeStore.resumeProcess(task.id, task.title, task.description ?? "", existingArtifacts, task.git_branch);
    }
    lgeStore.openView(task.id);
  };

  // Determine which phases are completed (from disk or memory)
  const completedPhases = PHASE_ORDER.filter((p) => {
    if (activeProcess?.phases[p]?.status === "completed") return true;
    return p in existingArtifacts;
  });

  const formatDate = (dateStr: string) => {
    try {
      return new Date(dateStr).toLocaleString();
    } catch {
      return dateStr;
    }
  };

  return (
    <div className="flex flex-1 flex-col overflow-hidden p-6">
      {/* Back button */}
      <button
        onClick={() => selectTask(null)}
        className="mb-6 flex items-center gap-2 text-sm text-text-secondary transition-colors hover:text-text-primary"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M10 2L4 8l6 6" />
        </svg>
        {t("taskDetail.backToList")}
      </button>

      {/* Scrollable content */}
      <div className="flex-1 overflow-y-auto">
        {/* Header */}
        <div className="mb-6">
          {isEditing ? (
            <Input
              label={t("taskDetail.editTitle")}
              value={editTitle}
              onChange={(e) => setEditTitle(e.target.value)}
              placeholder={t("taskDetail.titlePlaceholder")}
              className="text-base font-bold"
              autoFocus
            />
          ) : (
            <div className="flex items-start gap-3">
              <h1 className="text-xl font-bold text-text-primary">{task.title}</h1>
              {task.source === "jira" && task.jira_key ? (
                <Badge color="text-blue-400" bgColor="bg-blue-400/20">{task.jira_key}</Badge>
              ) : (
                <Badge color="text-text-muted" bgColor="bg-text-muted/20">{t("taskDetail.sourceManual")}</Badge>
              )}
              <button
                onClick={handleEditStart}
                className="ml-auto flex shrink-0 items-center gap-1 rounded-md px-2 py-1 text-xs text-text-secondary transition-colors hover:bg-bg-card hover:text-text-primary"
              >
                <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M11 2a2.828 2.828 0 0 1 4 4L5 16H1v-4L11 2z" />
                </svg>
                {t("taskDetail.edit")}
              </button>
            </div>
          )}
          {!isEditing && (
            <div className="mt-3">
              <Badge color={statusStyle.color} bgColor={statusStyle.bgColor}>
                {t(`status.${task.status}`)}
              </Badge>
            </div>
          )}
        </div>

        {/* Description */}
        <div className="mb-6">
          {isEditing ? (
            <>
              <TextArea
                label={t("taskDetail.editDescription")}
                value={editDescription}
                onChange={(e) => setEditDescription(e.target.value)}
                placeholder={t("taskDetail.descriptionPlaceholder")}
                rows={6}
              />
              <div className="mt-3 flex gap-2">
                <Button onClick={handleEditSave} disabled={saving || !editTitle.trim()} size="sm">
                  {t("taskDetail.save")}
                </Button>
                <Button variant="secondary" onClick={handleEditCancel} disabled={saving} size="sm">
                  {t("taskDetail.cancel")}
                </Button>
              </div>
            </>
          ) : (
            <>
              <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-text-muted">
                {t("taskDetail.description")}
              </h3>
              <div className="rounded-xl border border-border bg-bg-card p-4">
                {renderedDescription ? (
                  <div className="prose prose-sm max-w-none">
                    <Markdown remarkPlugins={[remarkGfm]}>{renderedDescription}</Markdown>
                  </div>
                ) : (
                  <p className="text-sm italic text-text-muted">{t("taskDetail.noDescription")}</p>
                )}
              </div>
            </>
          )}
        </div>

        {/* Jira link */}
        {task.source === "jira" && task.jira_url && (
          <div className="mb-6">
            <a href={task.jira_url} target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-1.5 text-sm text-blue-400 transition-colors hover:text-blue-300">
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                <path d="M6 1H1v12h12V8" /><path d="M8 1h5v5" /><path d="M13 1L6 8" />
              </svg>
              {t("taskDetail.openInJira")}
            </a>
          </div>
        )}

        {/* Git Branch */}
        <div className="mb-6">
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-text-muted">
            {t("git.branch.title")}
          </h3>
          {task.git_branch ? (
            <div className="flex flex-col gap-2">
              <div className="flex items-center gap-2 rounded-xl border border-border bg-bg-card px-4 py-3">
                <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-success shrink-0">
                  <path d="M6 3a3 3 0 1 0 0 6 3 3 0 0 0 0-6z" />
                  <path d="M6 9v4" />
                  <path d="M10 3a3 3 0 0 1 0 6" />
                  <path d="M10 9v4" />
                </svg>
                <Badge color="text-success" bgColor="bg-success/20">{task.git_branch}</Badge>
              </div>
              {task.worktree_path && (
                <div className="rounded-lg border border-accent/20 bg-accent/5">
                  {/* Header row: icon + label + action buttons */}
                  <div className="flex items-center gap-2 px-3 py-2">
                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" className="shrink-0 text-accent">
                      <path d="M1 3h10v7H1z" /><path d="M1 3l2-2h4l2 2" />
                    </svg>
                    <span className="flex-1 text-xs font-medium text-accent">{t("git.worktree.active")}</span>
                    <div className="flex items-center gap-1">
                      {/* Open in IDE */}
                      <button
                        onClick={() => api.openInEditor(task.worktree_path!)}
                        className="rounded p-1.5 text-text-muted transition-colors hover:bg-accent/20 hover:text-accent"
                        title={t("git.worktree.openInIde")}
                      >
                        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                          <path d="M4.5 1.5L1 5l3.5 3.5" /><path d="M9.5 1.5L13 5l-3.5 3.5" /><path d="M8 1l-2 8" />
                        </svg>
                      </button>
                      {/* Copy path */}
                      <button
                        onClick={() => {
                          navigator.clipboard.writeText(task.worktree_path!);
                          setCopiedPath(true);
                          setTimeout(() => setCopiedPath(false), 2000);
                        }}
                        className="rounded p-1.5 text-text-muted transition-colors hover:bg-accent/20 hover:text-accent"
                        title={copiedPath ? t("git.worktree.copied") : t("git.worktree.copyPath")}
                      >
                        {copiedPath ? (
                          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="text-success">
                            <path d="M3 7l3 3 5-5" />
                          </svg>
                        ) : (
                          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                            <rect x="4" y="4" width="8" height="8" rx="1" /><path d="M10 4V2.5A1.5 1.5 0 008.5 1h-6A1.5 1.5 0 001 2.5v6A1.5 1.5 0 002.5 10H4" />
                          </svg>
                        )}
                      </button>
                      {/* Remove worktree */}
                      <button
                        onClick={async () => {
                          try { await removeWorktree(task.id); } catch { /* handled in store */ }
                        }}
                        className="rounded p-1.5 text-text-muted transition-colors hover:bg-error/20 hover:text-error"
                        title={t("git.worktree.remove")}
                      >
                        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                          <path d="M2 4h10M4 4V2.5A1.5 1.5 0 015.5 1h3A1.5 1.5 0 0110 2.5V4M5 7v3M9 7v3" /><path d="M3 4v8a1 1 0 001 1h6a1 1 0 001-1V4" />
                        </svg>
                      </button>
                    </div>
                  </div>
                  {/* Path display — truncated, monospace */}
                  <div className="border-t border-accent/10 px-3 py-1.5">
                    <p className="truncate font-mono text-[11px] text-text-muted" title={task.worktree_path}>
                      {task.worktree_path}
                    </p>
                  </div>
                </div>
              )}
            </div>
          ) : (
            <Button variant="secondary" size="sm" onClick={handleOpenBranchDialog} className="w-full">
              {t("git.branch.create")}
            </Button>
          )}
        </div>

        {/* Git Branch Dialog */}
        <Dialog
          open={branchDialogOpen}
          onClose={() => setBranchDialogOpen(false)}
          title={t("git.branch.dialogTitle")}
        >
          <div className="flex flex-col gap-4">
            <Input
              label={t("git.branch.nameLabel")}
              value={branchName}
              onChange={(e) => setBranchName(e.target.value)}
              placeholder="feature/my-task"
            />
            <div className="flex flex-col gap-1">
              <label className="text-xs font-medium text-text-secondary">{t("git.branch.baseLabel")}</label>
              <input
                type="text"
                value={baseBranch}
                onChange={(e) => setBaseBranch(e.target.value)}
                className="rounded-lg border border-border bg-bg-card px-3 py-2 text-sm text-text-primary placeholder:text-text-muted focus:border-accent focus:outline-none"
                placeholder="develop"
              />
              <p className="text-xs text-text-muted">{t("git.branch.baseHint")}</p>
            </div>
            {branchError && (
              <p className="text-xs text-error">{branchError}</p>
            )}
            <div className="flex gap-2">
              <Button onClick={handleCreateBranch} disabled={branchLoading || !branchName.trim()} className="flex-1">
                {branchLoading ? t("git.branch.creating") : t("git.branch.confirm")}
              </Button>
              <Button variant="secondary" onClick={() => setBranchDialogOpen(false)} disabled={branchLoading}>
                {t("taskDetail.cancel")}
              </Button>
            </div>
          </div>
        </Dialog>

        {/* Attachments */}
        <div className="mb-6">
          <h3 className="mb-2 text-xs font-semibold uppercase tracking-wider text-text-muted">
            {t("attachments.title")}
          </h3>
          <div className="flex flex-col gap-2">
            {(attachments[task.id] ?? []).map((att) => {
              const activePhases = att.injection_phases as InjectionPhase[];
              return (
                <div key={att.id} className="rounded-xl border border-border bg-bg-card px-3 py-2">
                  {/* File name row */}
                  <div className="flex items-center gap-2">
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="shrink-0 text-text-muted">
                      <path d="M8 1H3a1 1 0 00-1 1v10a1 1 0 001 1h8a1 1 0 001-1V5L8 1z" />
                      <path d="M8 1v4h4" />
                    </svg>
                    <span className="flex-1 truncate text-xs text-text-primary" title={att.file_name}>{att.file_name}</span>
                    <button
                      onClick={() => handleRemoveAttachment(att.id)}
                      className="shrink-0 rounded p-1 text-text-muted transition-colors hover:bg-error/20 hover:text-error"
                      title={t("attachments.remove")}
                    >
                      <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                        <path d="M2 2l8 8M10 2l-8 8" />
                      </svg>
                    </button>
                  </div>
                  {/* Phase badges — always visible */}
                  <div className="mt-1.5 flex flex-wrap gap-1">
                    {(["planning", "builder", "review", "guardian"] as InjectionPhase[]).map((p) => {
                      const active = activePhases.includes(p);
                      const style = PHASE_COLORS[p];
                      return (
                        <button
                          key={p}
                          onClick={() => handleTogglePhase(att.id, activePhases, p)}
                          className={`rounded px-2 py-0.5 text-[10px] font-medium transition-opacity ${active ? "opacity-100" : "opacity-30"}`}
                          style={{ color: "inherit" }}
                          title={active ? `Remove from ${PHASE_LABELS[p]}` : `Add to ${PHASE_LABELS[p]}`}
                        >
                          <Badge color={style.color} bgColor={active ? style.bgColor : "bg-transparent"}>
                            {PHASE_LABELS[p]}
                          </Badge>
                        </button>
                      );
                    })}
                  </div>
                </div>
              );
            })}
            {(attachments[task.id] ?? []).length === 0 && (
              <p className="text-xs italic text-text-muted">{t("attachments.empty")}</p>
            )}
            {attachmentError && (
              <p className="text-xs text-error">{attachmentError}</p>
            )}
            <Button
              variant="secondary"
              size="sm"
              onClick={handleAttachFile}
              disabled={attachmentLoading}
              className="w-full"
            >
              {attachmentLoading ? "..." : t("attachments.add")}
            </Button>
          </div>
        </div>

        {/* LGE buttons */}
        {!loadingArtifacts && (
          <div className="mb-6 flex flex-col gap-2">
            {/* Context injection */}
            {!hasRunningPhase && (
              <div className="rounded-lg border border-border bg-bg-card">
                <button
                  onClick={() => setLgeContextOpen(!lgeContextOpen)}
                  className="flex w-full items-center justify-between px-3 py-2 text-sm text-text-secondary hover:text-text-primary transition-colors"
                >
                  <span>{t("lge.context.toggle")}</span>
                  <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor"
                    strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"
                    className={`transition-transform ${lgeContextOpen ? "rotate-180" : ""}`}>
                    <path d="M3 5l4 4 4-4" />
                  </svg>
                </button>
                {lgeContextOpen && (
                  <div className="px-3 pb-3">
                    <TextArea
                      value={lgeContextText}
                      onChange={(e) => setLgeContextText(e.target.value)}
                      placeholder={t("lge.context.placeholder")}
                      rows={4}
                    />
                    <p className="mt-1 text-xs text-text-muted">{t("lge.context.hint")}</p>
                  </div>
                )}
              </div>
            )}

            {/* Running indicator */}
            {hasRunningPhase && (
              <div className="mb-2 flex items-center gap-2 rounded-lg border border-accent/30 bg-accent/5 px-3 py-2">
                <div className="h-3 w-3 animate-spin rounded-full border-2 border-accent border-t-transparent" />
                <span className="text-sm text-accent">{t("lge.status.running")}</span>
                <Button variant="ghost" size="sm" onClick={handleViewLge} className="ml-auto">
                  {t("lge.action.viewArtifacts")}
                </Button>
              </div>
            )}

            {hasArtifacts || hasRunningPhase ? (
              <>
                {/* Completed phases */}
                {completedPhases.length > 0 && (
                  <div className="mb-2 flex items-center gap-2">
                    <span className="text-xs text-text-muted">{t("lge.title")}:</span>
                    {completedPhases.map((p) => (
                      <Badge key={p} color="text-success" bgColor="bg-success/20">
                        {t(`lge.phase.${p}`)}
                      </Badge>
                    ))}
                  </div>
                )}

                {/* Resume next phase */}
                {nextPhase && !hasRunningPhase && (
                  <Button onClick={handleResumeLge} className="w-full py-3 text-base">
                    <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <polygon points="6,3 16,9 6,15" fill="currentColor" />
                    </svg>
                    {t("lge.action.continue")} — {t(`lge.phase.${nextPhase}`)}
                  </Button>
                )}

                {/* View artifacts */}
                {!hasRunningPhase && (
                  <Button variant="secondary" onClick={handleViewLge} className="w-full">
                    {t("lge.action.viewArtifacts")}
                  </Button>
                )}
              </>
            ) : (
              <Button onClick={handleStartLge} className="w-full py-3 text-base">
                <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <polygon points="6,3 16,9 6,15" fill="currentColor" />
                </svg>
                {t("taskDetail.startLge")}
              </Button>
            )}
          </div>
        )}

        {/* Timestamps */}
        <div className="flex gap-6 text-xs text-text-muted">
          <span>{t("taskDetail.createdAt")}: {formatDate(task.created_at)}</span>
          <span>{t("taskDetail.updatedAt")}: {formatDate(task.updated_at)}</span>
        </div>
      </div>
    </div>
  );
}
