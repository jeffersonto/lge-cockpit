import { useState } from "react";
import { useTranslation } from "react-i18next";
import { open as openExternal } from "@tauri-apps/plugin-shell";
import { Dialog } from "../ui/Dialog";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { useTaskStore } from "../../stores/taskStore";
import { useRepositoryStore } from "../../stores/repositoryStore";

interface ImportJiraDialogProps {
  open: boolean;
  onClose: () => void;
  repositoryId: string;
}

const AUTH_REQUIRED_PREFIX = "ATLASSIAN_AUTH_REQUIRED:";

function extractAuthUrl(err: unknown): string | null {
  const msg = String(err);
  const idx = msg.indexOf(AUTH_REQUIRED_PREFIX);
  if (idx === -1) return null;
  return msg.slice(idx + AUTH_REQUIRED_PREFIX.length).trim();
}

export function ImportJiraDialog({
  open,
  onClose,
  repositoryId,
}: ImportJiraDialogProps) {
  const { t } = useTranslation();
  const importJiraTask = useTaskStore((s) => s.importJiraTask);
  const runJiraDiagnostic = useTaskStore((s) => s.runJiraDiagnostic);
  const selectedRepoId = useRepositoryStore((s) => s.selectedRepoId);
  const [jiraKey, setJiraKey] = useState("");
  const [loading, setLoading] = useState(false);
  const [diagLoading, setDiagLoading] = useState(false);
  const [diagCopied, setDiagCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [authUrl, setAuthUrl] = useState<string | null>(null);

  const runImport = async () => {
    setLoading(true);
    setError(null);
    setAuthUrl(null);
    try {
      await importJiraTask(repositoryId, jiraKey.trim().toUpperCase());
      setJiraKey("");
      onClose();
    } catch (err) {
      const url = extractAuthUrl(err);
      if (url) {
        setAuthUrl(url);
      } else {
        setError(String(err));
      }
    } finally {
      setLoading(false);
    }
  };

  const handleCopyDiagnostic = async () => {
    if (!jiraKey.trim() || !selectedRepoId) return;
    setDiagLoading(true);
    setDiagCopied(false);
    try {
      const report = await runJiraDiagnostic(selectedRepoId, jiraKey.trim().toUpperCase());
      await navigator.clipboard.writeText(report);
      setDiagCopied(true);
      setTimeout(() => setDiagCopied(false), 3000);
    } catch (err) {
      setError(String(err));
    } finally {
      setDiagLoading(false);
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!jiraKey.trim()) return;
    void runImport();
  };

  const handleAuthenticate = async () => {
    if (!authUrl) return;
    try {
      await openExternal(authUrl);
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <Dialog open={open} onClose={onClose} title={t("importJira.title")}>
      {authUrl ? (
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-2 text-sm">
            <p className="font-medium text-warning">
              {t("importJira.authRequiredTitle")}
            </p>
            <p className="text-text-muted">
              {t("importJira.authRequiredBody")}
            </p>
          </div>
          <div className="flex justify-between gap-2 pt-2">
            <Button
              variant="ghost"
              type="button"
              onClick={() => void handleCopyDiagnostic()}
              disabled={diagLoading || !jiraKey.trim()}
              className="text-xs text-text-muted"
            >
              {diagCopied
                ? t("importJira.diagCopied")
                : diagLoading
                  ? t("importJira.diagLoading")
                  : t("importJira.copyDiag")}
            </Button>
            <div className="flex gap-2">
              <Button variant="ghost" type="button" onClick={onClose}>
                {t("importJira.cancel")}
              </Button>
              <Button type="button" variant="ghost" onClick={handleAuthenticate}>
                {t("importJira.authenticate")}
              </Button>
              <Button type="button" onClick={() => void runImport()} disabled={loading}>
                {loading ? t("importJira.importing") : t("importJira.retry")}
              </Button>
            </div>
          </div>
        </div>
      ) : (
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
            <p className="text-xs text-error">{error}</p>
          )}
          <div className="flex justify-between gap-2 pt-2">
            {error && (
              <Button
                variant="ghost"
                type="button"
                onClick={() => void handleCopyDiagnostic()}
                disabled={diagLoading || !jiraKey.trim()}
                className="text-xs text-text-muted"
              >
                {diagCopied
                  ? t("importJira.diagCopied")
                  : diagLoading
                    ? t("importJira.diagLoading")
                    : t("importJira.copyDiag")}
              </Button>
            )}
            <div className={`flex gap-2 ${error ? "" : "ml-auto"}`}>
              <Button variant="ghost" type="button" onClick={onClose}>
                {t("importJira.cancel")}
              </Button>
              <Button type="submit" disabled={!jiraKey.trim() || loading}>
                {loading ? t("importJira.importing") : t("importJira.import")}
              </Button>
            </div>
          </div>
        </form>
      )}
    </Dialog>
  );
}
