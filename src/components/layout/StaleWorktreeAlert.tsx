import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/Button";
import type { StaleWorktreeInfo } from "../../types";
import * as api from "../../lib/tauri";
import { useRepositoryStore } from "../../stores/repositoryStore";

export function StaleWorktreeAlert() {
  const { t } = useTranslation();
  const [staleItems, setStaleItems] = useState<StaleWorktreeInfo[]>([]);
  const [dismissed, setDismissed] = useState(false);
  const [cleaning, setCleaning] = useState(false);

  useEffect(() => {
    api.checkStaleWorktrees().then((items) => {
      setStaleItems(items);
    }).catch(() => {});
  }, []);

  if (dismissed || staleItems.length === 0) return null;

  // Group by repository
  const byRepo = staleItems.reduce<Record<string, { name: string; id: string; count: number }>>((acc, item) => {
    if (!acc[item.repository_id]) {
      acc[item.repository_id] = { name: item.repository_name, id: item.repository_id, count: 0 };
    }
    acc[item.repository_id].count++;
    return acc;
  }, {});

  const handleCleanAll = async () => {
    setCleaning(true);
    try {
      const repoIds = [...new Set(staleItems.map((s) => s.repository_id))];
      for (const repoId of repoIds) {
        await api.removeCompletedWorktrees(repoId);
      }
      setStaleItems([]);
      useRepositoryStore.getState().fetchRepositories();
    } catch {
      // best effort
    } finally {
      setCleaning(false);
    }
  };

  return (
    <div className="mx-4 mt-2 rounded-lg border border-warning/30 bg-warning/5 px-4 py-3">
      <div className="flex items-start gap-3">
        <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" className="mt-0.5 shrink-0 text-warning">
          <path d="M9 2l7 13H2L9 2z" /><path d="M9 7v3" /><circle cx="9" cy="13" r="0.5" fill="currentColor" />
        </svg>
        <div className="flex-1">
          <p className="text-sm font-medium text-text-primary">
            {t("git.worktree.staleAlert", { count: staleItems.length })}
          </p>
          <p className="mt-0.5 text-xs text-text-muted">
            {Object.values(byRepo).map((r) => `${r.name} (${r.count})`).join(", ")}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Button size="sm" onClick={handleCleanAll} disabled={cleaning}>
            {cleaning ? t("git.worktree.cleaning") : t("git.worktree.cleanAll")}
          </Button>
          <button
            onClick={() => setDismissed(true)}
            className="rounded p-1 text-text-muted transition-colors hover:bg-bg-hover hover:text-text-primary"
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
              <path d="M3 3l8 8M11 3l-8 8" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}
