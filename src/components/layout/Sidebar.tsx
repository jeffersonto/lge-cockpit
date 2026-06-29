import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { useRepositoryStore } from "../../stores/repositoryStore";
import { useTaskStore } from "../../stores/taskStore";
import { DeleteProjectDialog } from "../repositories/DeleteProjectDialog";
import type { Repository } from "../../types";

export function Sidebar() {
  const { t } = useTranslation();
  const {
    repositories,
    selectedRepoId,
    fetchRepositories,
    addRepository,
    removeRepository,
    selectRepository,
  } = useRepositoryStore();
  const { fetchTasks, clearTasks } = useTaskStore();
  const [repoToDelete, setRepoToDelete] = useState<Repository | null>(null);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    fetchRepositories();
  }, [fetchRepositories]);

  const handleAddRepo = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      await addRepository(selected as string);
    }
  };

  const handleSelectRepo = (id: string) => {
    selectRepository(id);
    fetchTasks(id);
  };

  const handleRemoveClick = (e: React.MouseEvent, repo: Repository) => {
    e.stopPropagation();
    setRepoToDelete(repo);
  };

  const handleRemoveConfirm = async () => {
    if (!repoToDelete) return;
    setDeleting(true);
    try {
      await removeRepository(repoToDelete.id);
      if (selectedRepoId === repoToDelete.id) {
        clearTasks();
      }
    } catch {
      // error already logged in store
    } finally {
      setDeleting(false);
      setRepoToDelete(null);
    }
  };

  return (
    <>
      <div className="flex h-full w-60 flex-col border-r border-border bg-bg-surface">
        <div className="flex items-center justify-between p-4 pb-2">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-text-muted">
            {t("sidebar.repositories")}
          </h2>
          <button
            onClick={handleAddRepo}
            className="flex h-6 w-6 items-center justify-center rounded-md text-text-muted transition-colors hover:bg-bg-hover hover:text-text-primary"
            title={t("sidebar.addRepo")}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 14 14"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
            >
              <path d="M7 1v12M1 7h12" />
            </svg>
          </button>
        </div>

        <div className="flex-1 overflow-y-auto px-2 py-1">
          {repositories.length === 0 ? (
            <p className="px-2 py-4 text-center text-xs text-text-muted">
              {t("sidebar.noRepos")}
            </p>
          ) : (
            repositories.map((repo) => (
              <div
                key={repo.id}
                onClick={() => handleSelectRepo(repo.id)}
                className={`group flex cursor-pointer items-center justify-between rounded-lg px-3 py-2 text-sm transition-colors ${
                  selectedRepoId === repo.id
                    ? "bg-accent/15 text-accent"
                    : "text-text-secondary hover:bg-bg-hover hover:text-text-primary"
                }`}
              >
                <div className="flex items-center gap-2 overflow-hidden">
                  <svg
                    width="16"
                    height="16"
                    viewBox="0 0 16 16"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    className="shrink-0"
                  >
                    <path d="M2 4l6-2 6 2v8l-6 2-6-2V4z" />
                    <path d="M8 2v12" />
                  </svg>
                  <span className="truncate">{repo.name}</span>
                  {repo.active_worktree_count > 0 && (
                    <span className="ml-auto shrink-0 rounded-full bg-accent/20 px-1.5 py-0.5 text-[10px] font-medium text-accent">
                      {repo.active_worktree_count} wt
                    </span>
                  )}
                </div>
                <button
                  onClick={(e) => handleRemoveClick(e, repo)}
                  className="hidden shrink-0 rounded p-0.5 text-text-muted transition-colors hover:bg-error/20 hover:text-error group-hover:block"
                  title={t("common.remove")}
                >
                  <svg
                    width="12"
                    height="12"
                    viewBox="0 0 12 12"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    strokeLinecap="round"
                  >
                    <path d="M2 2l8 8M10 2l-8 8" />
                  </svg>
                </button>
              </div>
            ))
          )}
        </div>
      </div>

      {repoToDelete && (
        <DeleteProjectDialog
          repository={repoToDelete}
          open={!!repoToDelete}
          onClose={() => setRepoToDelete(null)}
          onConfirm={handleRemoveConfirm}
          loading={deleting}
        />
      )}
    </>
  );
}
