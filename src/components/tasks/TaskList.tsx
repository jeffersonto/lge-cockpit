import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useRepositoryStore } from "../../stores/repositoryStore";
import { useTaskStore } from "../../stores/taskStore";
import { TaskItem } from "./TaskItem";
import { CreateTaskDialog } from "./CreateTaskDialog";
import { ImportJiraDialog } from "./ImportJiraDialog";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { TaskStatus } from "../../types";

const STATUS_ACTIVE_STYLES: Record<TaskStatus, string> = {
  pending: "bg-text-muted/20 text-text-muted ring-1 ring-text-muted/40",
  in_progress: "bg-warning/20 text-warning ring-1 ring-warning/40",
  completed: "bg-success/20 text-success ring-1 ring-success/40",
};

export function TaskList() {
  const { t } = useTranslation();
  const selectedRepoId = useRepositoryStore((s) => s.selectedRepoId);
  const selectedRepo = useRepositoryStore((s) =>
    s.repositories.find((r) => r.id === s.selectedRepoId)
  );
  const { tasks, loading, fetchTasks } = useTaskStore();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [showJiraDialog, setShowJiraDialog] = useState(false);
  const [filterText, setFilterText] = useState("");
  const [filterStatuses, setFilterStatuses] = useState<TaskStatus[]>([]);

  const filteredTasks = useMemo(() => {
    return tasks.filter((task) => {
      const matchesText =
        filterText === "" ||
        task.title.toLowerCase().includes(filterText.toLowerCase()) ||
        (task.jira_key?.toLowerCase().includes(filterText.toLowerCase()) ?? false);
      const matchesStatus =
        filterStatuses.length === 0 || filterStatuses.includes(task.status);
      return matchesText && matchesStatus;
    });
  }, [tasks, filterText, filterStatuses]);

  const toggleStatus = (status: TaskStatus) => {
    setFilterStatuses((prev) =>
      prev.includes(status) ? prev.filter((s) => s !== status) : [...prev, status]
    );
  };

  useEffect(() => {
    if (selectedRepoId) {
      fetchTasks(selectedRepoId);
    }
  }, [selectedRepoId, fetchTasks]);

  if (!selectedRepoId) {
    return (
      <div className="flex flex-1 items-center justify-center">
        <div className="text-center">
          <svg
            width="48"
            height="48"
            viewBox="0 0 48 48"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            className="mx-auto mb-3 text-text-muted"
          >
            <path d="M6 12l18-6 18 6v24l-18 6-18-6V12z" />
            <path d="M24 6v36" />
          </svg>
          <p className="text-sm text-text-muted">{t("sidebar.noRepos")}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col overflow-hidden p-6">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h1 className="text-lg font-semibold text-text-primary">
            {selectedRepo?.name}
          </h1>
          <p className="text-xs text-text-muted">{selectedRepo?.path}</p>
        </div>
        <div className="flex gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setShowJiraDialog(true)}
          >
            {t("tasks.importJira")}
          </Button>
          <Button size="sm" onClick={() => setShowCreateDialog(true)}>
            {t("tasks.create")}
          </Button>
        </div>
      </div>

      {tasks.length > 0 && (
        <div className="mb-3 flex flex-col gap-2">
          <Input
            placeholder={t("tasks.filter.placeholder")}
            value={filterText}
            onChange={(e) => setFilterText(e.target.value)}
          />
          <div className="flex gap-2">
            {(["pending", "in_progress", "completed"] as TaskStatus[]).map((status) => (
              <button
                key={status}
                onClick={() => toggleStatus(status)}
                className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
                  filterStatuses.includes(status)
                    ? STATUS_ACTIVE_STYLES[status]
                    : "bg-bg-card text-text-secondary hover:bg-bg-hover"
                }`}
              >
                {t(`status.${status}`)}
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="flex-1 overflow-y-auto">
        {loading ? (
          <div className="flex items-center justify-center py-12">
            <div className="h-6 w-6 animate-spin rounded-full border-2 border-accent border-t-transparent" />
          </div>
        ) : tasks.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12">
            <p className="text-sm text-text-muted">{t("tasks.empty")}</p>
            <p className="mt-1 text-xs text-text-muted">
              {t("tasks.emptyHint")}
            </p>
          </div>
        ) : filteredTasks.length === 0 ? (
          <div className="flex items-center justify-center py-12">
            <p className="text-sm text-text-muted">{t("tasks.filter.noResults")}</p>
          </div>
        ) : (
          <div className="flex flex-col gap-2">
            {filteredTasks.map((task) => (
              <TaskItem key={task.id} task={task} />
            ))}
          </div>
        )}
      </div>

      <CreateTaskDialog
        open={showCreateDialog}
        onClose={() => setShowCreateDialog(false)}
        repositoryId={selectedRepoId}
      />
      <ImportJiraDialog
        open={showJiraDialog}
        onClose={() => setShowJiraDialog(false)}
        repositoryId={selectedRepoId}
      />
    </div>
  );
}
