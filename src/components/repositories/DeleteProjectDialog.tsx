import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Dialog } from "../ui/Dialog";
import type { Repository, ProjectDeletePreview } from "../../types";
import * as api from "../../lib/tauri";

interface DeleteProjectDialogProps {
  repository: Repository;
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  loading?: boolean;
}

export function DeleteProjectDialog({ repository, open, onClose, onConfirm, loading }: DeleteProjectDialogProps) {
  const { t } = useTranslation();
  const [preview, setPreview] = useState<ProjectDeletePreview | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);

  useEffect(() => {
    if (!open) return;
    setPreviewLoading(true);
    api.getProjectDeletePreview(repository.id)
      .then(setPreview)
      .catch(() => setPreview(null))
      .finally(() => setPreviewLoading(false));
  }, [open, repository.id]);

  return (
    <Dialog open={open} onClose={onClose} title={t("repos.deleteTitle")}>
      <div className="space-y-4">
        <p className="text-sm text-text-secondary">
          <span className="font-medium text-text-primary">"{repository.name}"</span>
        </p>

        {previewLoading ? (
          <div className="rounded-lg border border-border bg-bg-card px-3 py-2">
            <p className="text-xs text-text-muted animate-pulse">{t("common.loading")}</p>
          </div>
        ) : preview ? (
          <div className="rounded-lg border border-border bg-bg-card px-3 py-2 space-y-1">
            <p className="text-xs text-text-secondary">
              {t("repos.deleteStats", {
                tasks: preview.task_count,
                worktrees: preview.worktree_count,
                branches: preview.branch_count,
              })}
            </p>
          </div>
        ) : null}

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
            disabled={loading || previewLoading}
            className="rounded-lg bg-error px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-error/80 disabled:opacity-50"
          >
            {loading ? "..." : t("common.remove")}
          </button>
        </div>
      </div>
    </Dialog>
  );
}
