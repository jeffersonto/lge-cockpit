import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Dialog } from "../ui/Dialog";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { useTaskStore } from "../../stores/taskStore";

interface ImportJiraDialogProps {
  open: boolean;
  onClose: () => void;
  repositoryId: string;
}

export function ImportJiraDialog({
  open,
  onClose,
  repositoryId,
}: ImportJiraDialogProps) {
  const { t } = useTranslation();
  const importJiraTask = useTaskStore((s) => s.importJiraTask);
  const [jiraKey, setJiraKey] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const runImport = async () => {
    setLoading(true);
    setError(null);
    try {
      await importJiraTask(repositoryId, jiraKey.trim().toUpperCase());
      setJiraKey("");
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!jiraKey.trim()) return;
    void runImport();
  };

  return (
    <Dialog open={open} onClose={onClose} title={t("importJira.title")}>
      <form onSubmit={handleSubmit} className="flex flex-col gap-4">
        <Input
          id="jira-key"
          label={t("importJira.key")}
          placeholder={t("importJira.keyPlaceholder")}
          value={jiraKey}
          onChange={(e) => setJiraKey(e.target.value)}
          autoFocus
          required
        />
        {error && (
          <p className="text-xs text-error whitespace-pre-line">{error}</p>
        )}
        <div className="flex justify-end gap-2 pt-2">
          <Button variant="ghost" type="button" onClick={onClose}>
            {t("importJira.cancel")}
          </Button>
          <Button type="submit" disabled={!jiraKey.trim() || loading}>
            {loading ? t("importJira.importing") : t("importJira.import")}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}