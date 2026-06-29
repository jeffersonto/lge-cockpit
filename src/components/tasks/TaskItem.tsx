import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Task, TaskStatus } from "../../types";
import { Badge } from "../ui/Badge";
import { useTaskStore } from "../../stores/taskStore";
import { useLgeStore } from "../../stores/lgeStore";
import { DeleteTaskDialog } from "./DeleteTaskDialog";

interface TaskItemProps {
  task: Task;
}

const STATUS_STYLES: Record<TaskStatus, { color: string; bgColor: string }> = {
  pending: { color: "text-text-muted", bgColor: "bg-text-muted/20" },
  in_progress: { color: "text-warning", bgColor: "bg-warning/20" },
  completed: { color: "text-success", bgColor: "bg-success/20" },
};

export function TaskItem({ task }: TaskItemProps) {
  const { t } = useTranslation();
  const { deleteTask, selectTask } = useTaskStore();
  const { isAnyPhaseRunning, getProcess } = useLgeStore();
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const running = isAnyPhaseRunning(task.id);
  const currentPhaseId = getProcess(task.id)?.currentPhaseId;
  const statusStyle = STATUS_STYLES[task.status];

  const handleDeleteClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setShowDeleteDialog(true);
  };

  const handleDeleteConfirm = async () => {
    setDeleting(true);
    try {
      const errors = await deleteTask(task.id);
      if (errors.length > 0) {
        window.alert(t("tasks.deletionPartialError") + "\n\n" + errors.join("\n"));
      }
    } catch {
      // error already logged in store
    } finally {
      setDeleting(false);
      setShowDeleteDialog(false);
    }
  };

  const handleClick = () => {
    selectTask(task.id);
  };

  return (
    <>
      <div
        onClick={handleClick}
        className="group flex cursor-pointer items-center gap-3 rounded-xl border border-border bg-bg-card px-4 py-3 transition-colors hover:border-accent/30"
      >
        {/* Status indicator (read-only) */}
        {running ? (
          <div className="flex h-5 w-5 shrink-0 items-center justify-center">
            <div className="h-5 w-5 animate-spin rounded-full border-2 border-accent/30 border-t-accent" />
          </div>
        ) : (
          <div
            className={`flex h-5 w-5 shrink-0 items-center justify-center rounded-full border-2 ${
              task.status === "completed"
                ? "border-success bg-success"
                : task.status === "in_progress"
                  ? "border-warning"
                  : "border-text-muted"
            }`}
          >
            {task.status === "completed" && (
              <svg
                width="10"
                height="10"
                viewBox="0 0 10 10"
                fill="none"
                stroke="white"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M2 5l2 2 4-4" />
              </svg>
            )}
            {task.status === "in_progress" && (
              <span className="h-2 w-2 rounded-full bg-warning" />
            )}
          </div>
        )}

        <div className="flex-1 overflow-hidden">
          <p
            className={`text-sm ${
              task.status === "completed"
                ? "text-text-muted line-through"
                : "text-text-primary"
            }`}
          >
            {task.title}
          </p>
          {task.description && (
            <p className="mt-0.5 truncate text-xs text-text-muted">
              {task.description}
            </p>
          )}
        </div>

        <div className="flex items-center gap-2">
          {task.source === "jira" && task.jira_key && (
            <Badge color="text-blue-400" bgColor="bg-blue-400/20">
              {task.jira_key}
            </Badge>
          )}
          {running && currentPhaseId && (
            <Badge color="text-accent" bgColor="bg-accent/20">
              {t(`lge.phase.${currentPhaseId}`)}
            </Badge>
          )}
          <Badge color={statusStyle.color} bgColor={statusStyle.bgColor}>
            {t(`status.${task.status}`)}
          </Badge>
          <button
            onClick={handleDeleteClick}
            disabled={running}
            className="hidden rounded p-1 text-text-muted transition-colors hover:bg-error/20 hover:text-error group-hover:block disabled:cursor-not-allowed disabled:opacity-40"
            title={running ? t("tasks.deleteDisabledRunning") : t("common.delete")}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 14 14"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            >
              <path d="M3 3l8 8M11 3l-8 8" />
            </svg>
          </button>
        </div>
      </div>

      <DeleteTaskDialog
        task={task}
        open={showDeleteDialog}
        onClose={() => setShowDeleteDialog(false)}
        onConfirm={handleDeleteConfirm}
        loading={deleting}
      />
    </>
  );
}
