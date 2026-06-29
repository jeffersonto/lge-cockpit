import { useTranslation } from "react-i18next";
import { Dialog } from "../ui/Dialog";
import type { Task } from "../../types";

interface DeleteTaskDialogProps {
  task: Task;
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  loading?: boolean;
}

export function DeleteTaskDialog({ task, open, onClose, onConfirm, loading }: DeleteTaskDialogProps) {
  const { t } = useTranslation();

  return (
    <Dialog open={open} onClose={onClose} title={t("tasks.deleteTitle")}>
      <div className="space-y-4">
        <p className="text-sm text-text-secondary">
          <span className="font-medium text-text-primary">"{task.title}"</span>
        </p>

        {(task.worktree_path || task.git_branch) && (
          <div className="rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 space-y-1">
            {task.worktree_path && (
              <p className="text-xs text-warning">
                {t("tasks.deleteWarningWorktree")}
              </p>
            )}
            {task.git_branch && (
              <p className="text-xs text-warning">
                {t("tasks.deleteWarningBranch", { branch: task.git_branch })}
              </p>
            )}
          </div>
        )}

        <p className="text-xs text-text-muted">{t("common.irreversible")}</p>

        <div className="flex justify-end gap-2 pt-2">
          <button
            onClick={onClose}
            className="rounded-lg border border-border px-3 py-1.5 text-sm text-text-secondary transition-colors hover:bg-bg-hover"
          >
            {t("common.cancel")}
          </button>
          <button
            onClick={onConfirm}
            disabled={loading}
            className="rounded-lg bg-error px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-error/80 disabled:opacity-50"
          >
            {loading ? "..." : t("common.delete")}
          </button>
        </div>
      </div>
    </Dialog>
  );
}
