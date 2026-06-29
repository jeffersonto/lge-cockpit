import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useLgeStore, PHASE_ORDER } from "../../stores/lgeStore";
import { useTaskStore } from "../../stores/taskStore";
import { useRepositoryStore } from "../../stores/repositoryStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { Badge } from "../ui/Badge";
import { Button } from "../ui/Button";
import { TextArea } from "../ui/Input";
import type { LgePhaseId, LgePhaseStatus } from "../../types";
import * as api from "../../lib/tauri";

const MODEL_DISPLAY: Record<string, string> = {
  opus: "Opus",
  sonnet: "Sonnet",
  haiku: "Haiku",
};

const STATUS_STYLES: Record<LgePhaseStatus, { color: string; bgColor: string }> = {
  pending: { color: "text-text-muted", bgColor: "bg-text-muted/20" },
  queued: { color: "text-warning", bgColor: "bg-warning/20" },
  running: { color: "text-accent", bgColor: "bg-accent/20" },
  completed: { color: "text-success", bgColor: "bg-success/20" },
  failed: { color: "text-error", bgColor: "bg-error/20" },
};

function PrErrorBlock({ error }: { error: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const delimiter = "<!--CMD-->";
  const hasCmd = error.includes(delimiter);
  const message = hasCmd ? error.split(delimiter)[0] : error;
  const command = hasCmd ? error.split(delimiter)[1] : null;

  const handleCopy = () => {
    if (!command) return;
    navigator.clipboard.writeText(command);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="flex flex-col gap-2 rounded-lg border border-error/30 bg-error/5 p-3">
      <p className="text-xs text-error">{message}</p>
      {command && (
        <>
          <p className="text-xs text-text-secondary">{t("git.pr.manualHint")}</p>
          <div className="flex items-center gap-2">
            <code className="flex-1 overflow-x-auto rounded bg-bg-primary px-2 py-1.5 text-xs text-text-primary">
              {command}
            </code>
            <button
              onClick={handleCopy}
              className="shrink-0 rounded bg-accent/20 px-2 py-1.5 text-xs text-accent transition-colors hover:bg-accent/30"
            >
              {copied ? t("git.pr.copied") : t("git.pr.copyCommand")}
            </button>
          </div>
        </>
      )}
    </div>
  );
}

export function LgePhasePipeline() {
  const { t } = useTranslation();
  const { viewingTaskId, processes, runPhase, closeView, getNextPhase, cancelPhase } = useLgeStore();
  const { tasks } = useTaskStore();
  const { repositories, selectedRepoId } = useRepositoryStore();
  const { phaseModels, loaded: settingsLoaded, fetchPhaseModels } = useSettingsStore();

  useEffect(() => {
    if (!settingsLoaded) fetchPhaseModels();
  }, [settingsLoaded, fetchPhaseModels]);
  const [showGoTo, setShowGoTo] = useState(false);
  const [contextOpen, setContextOpen] = useState(false);
  const [contextText, setContextText] = useState("");

  const [prBaseBranch, setPrBaseBranch] = useState("develop");
  const [prLoading, setPrLoading] = useState(false);
  const [prUrl, setPrUrl] = useState<string | null>(null);
  const [prError, setPrError] = useState<string | null>(null);

  type CommitMode = "idle" | "manual" | "ai-loading" | "ai-ready";
  const [commitMode, setCommitMode] = useState<CommitMode>("idle");
  const [commitMessage, setCommitMessage] = useState("");
  const [worktreeCleaned, setWorktreeCleaned] = useState(false);
  const [cleaningWorktree, setCleaningWorktree] = useState(false);

  if (!viewingTaskId || !processes[viewingTaskId]) return null;

  const process = processes[viewingTaskId];
  const { phases, currentPhaseId, waitingForUserAction, prReady } = process;
  const nextPhase = getNextPhase(viewingTaskId);
  const isRunning = currentPhaseId && (
    phases[currentPhaseId]?.status === "running" ||
    phases[currentPhaseId]?.status === "queued"
  );

  const currentTask = tasks.find((t) => t.id === viewingTaskId);
  const currentRepo = repositories.find(
    (r) => r.id === (selectedRepoId ?? currentTask?.repository_id)
  );

  const handleSelectAi = async () => {
    if (!currentTask) return;
    setCommitMode("ai-loading");
    try {
      const msg = await api.generateCommitMessage(
        currentTask.id,
        currentTask.title,
        currentTask.jira_key
      );
      setCommitMessage(msg);
      setCommitMode("ai-ready");
    } catch {
      const scope = currentTask.jira_key ? `(${currentTask.jira_key})` : "";
      setCommitMessage(`feat${scope}: ${currentTask.title}`);
      setCommitMode("ai-ready");
    }
  };

  const handleSelectManual = () => {
    if (commitMessage === "") {
      const scope = currentTask?.jira_key ? `(${currentTask.jira_key})` : "";
      setCommitMessage(`feat${scope}: ${currentTask?.title ?? ""}`);
    }
    setCommitMode("manual");
  };

  const handleCommitAndPr = async () => {
    if (!currentTask?.git_branch || !commitMessage.trim()) return;
    setPrLoading(true);
    setPrError(null);
    try {
      await api.commitAndPush(currentTask.id, currentTask.git_branch, commitMessage.trim());
      const url = await api.createPullRequest(currentTask.id, prBaseBranch);
      setPrUrl(url);
    } catch (err) {
      setPrError(String(err));
    } finally {
      setPrLoading(false);
    }
  };

  const handleContinue = () => {
    if (nextPhase) {
      setShowGoTo(false);
      runPhase(viewingTaskId, nextPhase, contextText.trim() || undefined);
      setContextText("");
      setContextOpen(false);
    }
  };

  const handleGoTo = (phaseId: LgePhaseId) => {
    setShowGoTo(false);
    runPhase(viewingTaskId, phaseId, contextText.trim() || undefined);
    setContextText("");
    setContextOpen(false);
  };

  return (
    <div className="flex h-full flex-col p-6">
      {/* Back button — just closes the view, doesn't kill the process */}
      <button
        onClick={closeView}
        className="mb-6 flex items-center gap-2 text-sm text-text-secondary transition-colors hover:text-text-primary"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M10 2L4 8l6 6" />
        </svg>
        {t("lge.action.backToTask")}
      </button>

      <h2 className="mb-6 text-lg font-bold text-text-primary">
        {t("lge.title")}
      </h2>

      {/* Phase cards */}
      <div className="mb-8 flex flex-col gap-3">
        {PHASE_ORDER.map((phaseId, i) => {
          const phase = phases[phaseId];
          const style = STATUS_STYLES[phase.status];
          const isCurrent = currentPhaseId === phaseId;

          return (
            <div key={phaseId}>
              {i > 0 && <div className="ml-4 h-3 w-px bg-border" />}
              <div
                className={`flex items-center gap-3 rounded-xl border px-4 py-3 transition-colors ${
                  isCurrent ? "border-accent/40 bg-accent/5" : "border-border bg-bg-card"
                }`}
              >
                <div className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-full ${style.bgColor}`}>
                  {phase.status === "running" ? (
                    <div className="h-4 w-4 animate-spin rounded-full border-2 border-accent border-t-transparent" />
                  ) : phase.status === "queued" ? (
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none"
                      stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"
                      className="text-warning">
                      <circle cx="7" cy="7" r="5.5" />
                      <path d="M7 4v3l2 2" />
                    </svg>
                  ) : phase.status === "completed" ? (
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="2" className="text-success">
                      <path d="M3 7l3 3 5-5" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>
                  ) : phase.status === "failed" ? (
                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="2" className="text-error">
                      <path d="M3 3l8 8M11 3l-8 8" strokeLinecap="round" />
                    </svg>
                  ) : (
                    <span className="h-2 w-2 rounded-full bg-text-muted" />
                  )}
                </div>
                <div className="flex-1">
                  <p className="text-sm font-medium text-text-primary">{t(`lge.phase.${phaseId}`)}</p>
                  <p className="text-xs text-text-muted">{MODEL_DISPLAY[phaseModels[phaseId]] ?? phaseModels[phaseId]}</p>
                </div>
                <Badge color={style.color} bgColor={style.bgColor}>
                  {phase.status === "failed" && phase.error === "Interrupted by user"
                    ? t("lge.status.interrupted")
                    : t(`lge.status.${phase.status}`)}
                </Badge>
              </div>
              {phase.error && phase.error !== "Interrupted by user" && (
                <p className="ml-12 mt-1 text-xs text-error">{phase.error}</p>
              )}
            </div>
          );
        })}
      </div>

      {/* Interrupt button — visible while a phase is running */}
      {isRunning && currentPhaseId && (
        <div className="flex flex-col gap-2">
          <Button
            variant="danger"
            onClick={() => cancelPhase(viewingTaskId, currentPhaseId)}
          >
            {t("lge.action.interrupt")} — {t(`lge.phase.${currentPhaseId}`)}
          </Button>
        </div>
      )}

      {/* Action buttons — visible between phases */}
      {waitingForUserAction && !isRunning && (
        <div className="flex flex-col gap-2">
          {/* Context injection */}
          <div className="mb-2 rounded-lg border border-border bg-bg-card">
            <button
              onClick={() => setContextOpen(!contextOpen)}
              className="flex w-full items-center justify-between px-3 py-2 text-sm text-text-secondary hover:text-text-primary transition-colors"
            >
              <span>{t("lge.context.toggle")}</span>
              <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor"
                strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"
                className={`transition-transform ${contextOpen ? "rotate-180" : ""}`}>
                <path d="M3 5l4 4 4-4" />
              </svg>
            </button>
            {contextOpen && (
              <div className="px-3 pb-3">
                <TextArea
                  value={contextText}
                  onChange={(e) => setContextText(e.target.value)}
                  placeholder={t("lge.context.placeholder")}
                  rows={4}
                />
                <p className="mt-1 text-xs text-text-muted">{t("lge.context.hint")}</p>
              </div>
            )}
          </div>

          {/* Retry failed/interrupted phase */}
          {currentPhaseId && phases[currentPhaseId]?.status === "failed" && (
            <Button onClick={() => {
              runPhase(viewingTaskId, currentPhaseId, contextText.trim() || undefined);
              setContextText("");
              setContextOpen(false);
            }}>
              {t("lge.action.retry")} — {t(`lge.phase.${currentPhaseId}`)}
            </Button>
          )}

          {/* Continue to next phase */}
          {nextPhase && (
            <Button
              onClick={handleContinue}
              variant={currentPhaseId && phases[currentPhaseId]?.status === "failed" ? "secondary" : "primary"}
            >
              {t("lge.action.continue")}
            </Button>
          )}

          {/* Go to specific phase */}
          <Button variant="secondary" onClick={() => setShowGoTo(!showGoTo)}>
            {t("lge.action.goToPhase")}
          </Button>
          {showGoTo && (
            <div className="flex flex-col gap-1 rounded-lg border border-border bg-bg-card p-2">
              {PHASE_ORDER.map((phaseId) => (
                <button
                  key={phaseId}
                  onClick={() => handleGoTo(phaseId)}
                  className="rounded-md px-3 py-1.5 text-left text-sm text-text-secondary transition-colors hover:bg-bg-hover hover:text-text-primary"
                >
                  {t(`lge.phase.${phaseId}`)}
                </button>
              ))}
            </div>
          )}

          <Button variant="ghost" onClick={closeView}>
            {t("lge.action.stop")}
          </Button>
          {!nextPhase && !(currentPhaseId && phases[currentPhaseId]?.status === "failed") && (
            <p className="mt-2 text-center text-xs text-success">{t("lge.processComplete")}</p>
          )}
        </div>
      )}

      {/* PR Panel — shown when Guardian is complete and branch was created */}
      {prReady && !nextPhase && waitingForUserAction && !isRunning && (
        <div className="mt-4 rounded-xl border border-accent/30 bg-accent/5 p-4">
          <h3 className="mb-3 text-sm font-semibold text-text-primary">{t("git.pr.panelTitle")}</h3>

          {prUrl ? (
            <div className="flex flex-col gap-3">
              <p className="text-xs text-success">{t("git.pr.success")}</p>
              <a
                href={prUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-1.5 text-sm text-blue-400 transition-colors hover:text-blue-300 break-all"
                onClick={() => { /* Tauri opens external links via shell */ }}
              >
                {prUrl}
                <svg width="12" height="12" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                  <path d="M6 1H1v12h12V8" /><path d="M8 1h5v5" /><path d="M13 1L6 8" />
                </svg>
              </a>
              {currentTask?.worktree_path && !worktreeCleaned && (
                <button
                  onClick={async () => {
                    if (!currentTask) return;
                    setCleaningWorktree(true);
                    try {
                      await api.removeWorktree(currentTask.id);
                      setWorktreeCleaned(true);
                      useRepositoryStore.getState().fetchRepositories();
                    } catch { /* best effort */ }
                    finally { setCleaningWorktree(false); }
                  }}
                  disabled={cleaningWorktree}
                  className="flex items-center gap-1.5 self-start rounded-lg border border-border bg-bg-card px-3 py-2 text-xs text-text-muted transition-colors hover:border-warning/50 hover:text-warning"
                >
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" className="shrink-0">
                    <path d="M2 3h8M3 3v7a1 1 0 001 1h4a1 1 0 001-1V3M5 6v3M7 6v3" />
                  </svg>
                  {cleaningWorktree ? t("git.worktree.cleaning") : t("git.worktree.cleanupAfterPr")}
                </button>
              )}
              {worktreeCleaned && (
                <p className="text-xs text-success">{t("git.worktree.cleaned")}</p>
              )}
            </div>
          ) : (
            <div className="flex flex-col gap-4">

              {/* Step 1 — Commit message */}
              <div className="flex flex-col gap-2">
                <p className="text-xs font-medium text-text-secondary">{t("git.commit.messageLabel")}</p>

                {commitMode === "idle" && (
                  <div className="grid grid-cols-2 gap-2">
                    <button
                      onClick={handleSelectManual}
                      className="flex flex-col items-center gap-1.5 rounded-xl border border-border bg-bg-card px-3 py-3 text-center transition-colors hover:border-accent/50 hover:bg-accent/5"
                    >
                      <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="text-text-secondary">
                        <path d="M3 14h12M3 10h8M3 6h12" />
                      </svg>
                      <span className="text-xs font-medium text-text-primary">{t("git.commit.manual")}</span>
                      <span className="text-xs text-text-muted">{t("git.commit.manualHint")}</span>
                    </button>
                    <button
                      onClick={handleSelectAi}
                      className="flex flex-col items-center gap-1.5 rounded-xl border border-accent/30 bg-accent/5 px-3 py-3 text-center transition-colors hover:border-accent/60 hover:bg-accent/10"
                    >
                      <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="text-accent">
                        <path d="M9 2l1.5 4h4l-3.2 2.4 1.2 4L9 10l-3.5 2.4 1.2-4L3.5 6h4z" />
                      </svg>
                      <span className="text-xs font-medium text-text-primary">{t("git.commit.ai")}</span>
                      <span className="text-xs text-text-muted">{t("git.commit.aiHint")}</span>
                    </button>
                  </div>
                )}

                {commitMode === "ai-loading" && (
                  <div className="flex items-center gap-3 rounded-xl border border-accent/20 bg-accent/5 px-4 py-3">
                    <div className="h-4 w-4 shrink-0 animate-spin rounded-full border-2 border-accent border-t-transparent" />
                    <div className="flex flex-col">
                      <span className="text-sm font-medium text-text-primary">{t("git.commit.aiAnalyzing")}</span>
                      <span className="text-xs text-text-muted">{t("git.commit.aiAnalyzingHint")}</span>
                    </div>
                  </div>
                )}

                {(commitMode === "manual" || commitMode === "ai-ready") && (
                  <div className="flex flex-col gap-1">
                    {commitMode === "ai-ready" && (
                      <div className="mb-1 flex items-center gap-1.5">
                        <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-accent">
                          <path d="M6 1l1 2.8h2.8l-2.3 1.7.9 2.8L6 7l-2.4 1.3.9-2.8L2.2 3.8H5z" />
                        </svg>
                        <span className="text-xs text-accent">{t("git.commit.aiGenerated")}</span>
                        <button onClick={handleSelectAi} className="ml-auto text-xs text-text-muted underline hover:text-text-primary">
                          {t("git.commit.regenerate")}
                        </button>
                      </div>
                    )}
                    <input
                      type="text"
                      value={commitMessage}
                      onChange={(e) => setCommitMessage(e.target.value)}
                      className="rounded-lg border border-border bg-bg-card px-3 py-2 text-sm text-text-primary placeholder:text-text-muted focus:border-accent focus:outline-none"
                      placeholder="feat: ..."
                      autoFocus={commitMode === "manual"}
                    />
                    <button onClick={() => setCommitMode("idle")} className="self-start text-xs text-text-muted underline hover:text-text-primary">
                      {t("git.commit.changeMode")}
                    </button>
                  </div>
                )}
              </div>

              {/* Step 2 — Base branch + action (only after commit message is set) */}
              {(commitMode === "manual" || commitMode === "ai-ready") && (
                <>
                  <div className="flex flex-col gap-1">
                    <label className="text-xs font-medium text-text-secondary">{t("git.branch.baseLabel")}</label>
                    <input
                      type="text"
                      value={prBaseBranch}
                      onChange={(e) => setPrBaseBranch(e.target.value)}
                      className="rounded-lg border border-border bg-bg-card px-3 py-2 text-sm text-text-primary placeholder:text-text-muted focus:border-accent focus:outline-none"
                      placeholder="develop"
                    />
                  </div>
                  {prError && <PrErrorBlock error={prError} />}
                  <Button
                    onClick={handleCommitAndPr}
                    disabled={prLoading || !currentRepo || !currentTask?.git_branch || !commitMessage.trim()}
                  >
                    {prLoading ? t("git.pr.loading") : t("git.pr.commitAndOpen")}
                  </Button>
                </>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
