import { useTranslation } from "react-i18next";
import { useRepositoryStore } from "../../stores/repositoryStore";
import { useTaskStore } from "../../stores/taskStore";
import { useLgeStore, PHASE_ORDER } from "../../stores/lgeStore";
import { LgeIcon } from "../ui/LgeIcon";
import type { LgePhaseId } from "../../types";

const PHASE_SHORT: Record<LgePhaseId, string> = {
  planning: "P",
  builder: "B",
  review: "R",
  guardian: "G",
};

export function TopBar() {
  const { t } = useTranslation();
  const repos = useRepositoryStore((s) => s.repositories);
  const tasks = useTaskStore((s) => s.tasks);
  const { processes, viewingTaskId, openView } = useLgeStore();

  const activeProcesses = Object.entries(processes).filter(([_, proc]) =>
    Object.values(proc.phases).some((p) => p.status === "running")
  );

  const completedProcesses = Object.entries(processes).filter(
    ([_, proc]) =>
      !Object.values(proc.phases).some((p) => p.status === "running") &&
      Object.values(proc.phases).some((p) => p.status === "completed")
  );

  const totalTasks = tasks.length;
  const completedTasks = tasks.filter((t) => t.status === "completed").length;
  const inProgressTasks = tasks.filter((t) => t.status === "in_progress").length;

  return (
    <div className="flex h-11 items-center border-b border-border bg-bg-surface px-4">
      {/* Left: Logo */}
      <div className="flex items-center gap-2">
        <LgeIcon size={24} />
        <span className="text-sm font-semibold text-text-primary">LGE</span>
      </div>

      {/* Center: Live LGE processes */}
      <div className="ml-6 flex flex-1 items-center gap-3 overflow-x-auto">
        {activeProcesses.map(([taskId, proc]) => (
          <button
            key={taskId}
            onClick={() => openView(taskId)}
            className={`flex items-center gap-2 rounded-full border px-3 py-1 text-xs transition-colors ${
              viewingTaskId === taskId
                ? "border-accent/50 bg-accent/10"
                : "border-border bg-bg-card hover:border-accent/30"
            }`}
          >
            <div className="h-2 w-2 animate-pulse rounded-full bg-accent" />
            <span className="max-w-[120px] truncate text-text-primary">
              {proc.taskTitle.length > 20
                ? proc.taskTitle.slice(0, 20) + "..."
                : proc.taskTitle}
            </span>
            {/* Mini phase progress */}
            <div className="flex gap-0.5">
              {PHASE_ORDER.map((phaseId) => {
                const phase = proc.phases[phaseId];
                return (
                  <span
                    key={phaseId}
                    title={t(`lge.phase.${phaseId}`)}
                    className={`flex h-4 w-4 items-center justify-center rounded text-[9px] font-bold ${
                      phase.status === "completed"
                        ? "bg-success/30 text-success"
                        : phase.status === "running"
                          ? "animate-pulse bg-accent/30 text-accent"
                          : phase.status === "failed"
                            ? "bg-error/30 text-error"
                            : "bg-bg-hover text-text-muted"
                    }`}
                  >
                    {PHASE_SHORT[phaseId]}
                  </span>
                );
              })}
            </div>
          </button>
        ))}

        {completedProcesses.map(([taskId, proc]) => (
          <button
            key={taskId}
            onClick={() => openView(taskId)}
            className={`flex items-center gap-2 rounded-full border px-3 py-1 text-xs transition-colors ${
              viewingTaskId === taskId
                ? "border-success/50 bg-success/10"
                : "border-border bg-bg-card hover:border-success/30"
            }`}
          >
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="2" className="text-success">
              <path d="M2 5l2 2 4-4" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            <span className="max-w-[120px] truncate text-text-secondary">
              {proc.taskTitle.length > 20
                ? proc.taskTitle.slice(0, 20) + "..."
                : proc.taskTitle}
            </span>
            <div className="flex gap-0.5">
              {PHASE_ORDER.map((phaseId) => {
                const phase = proc.phases[phaseId];
                return (
                  <span
                    key={phaseId}
                    className={`flex h-4 w-4 items-center justify-center rounded text-[9px] font-bold ${
                      phase.status === "completed"
                        ? "bg-success/30 text-success"
                        : "bg-bg-hover text-text-muted"
                    }`}
                  >
                    {PHASE_SHORT[phaseId]}
                  </span>
                );
              })}
            </div>
          </button>
        ))}

        {activeProcesses.length === 0 && completedProcesses.length === 0 && (
          <span className="text-xs text-text-muted">
            {t("topbar.noProcesses")}
          </span>
        )}
      </div>

      {/* Right: Quick stats */}
      <div className="flex items-center gap-4 text-xs text-text-muted">
        {repos.length > 0 && (
          <span className="flex items-center gap-1.5">
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M1.5 3l4.5-1.5L10.5 3v6l-4.5 1.5L1.5 9V3z" />
              <path d="M6 1.5v9" />
            </svg>
            {repos.length}
          </span>
        )}
        {totalTasks > 0 && (
          <>
            {completedTasks > 0 && (
              <span className="flex items-center gap-1">
                <span className="h-1.5 w-1.5 rounded-full bg-success" />
                {completedTasks}
              </span>
            )}
            {inProgressTasks > 0 && (
              <span className="flex items-center gap-1">
                <span className="h-1.5 w-1.5 rounded-full bg-warning" />
                {inProgressTasks}
              </span>
            )}
            {totalTasks - completedTasks - inProgressTasks > 0 && (
              <span className="flex items-center gap-1">
                <span className="h-1.5 w-1.5 rounded-full bg-text-muted" />
                {totalTasks - completedTasks - inProgressTasks}
              </span>
            )}
          </>
        )}
      </div>
    </div>
  );
}
