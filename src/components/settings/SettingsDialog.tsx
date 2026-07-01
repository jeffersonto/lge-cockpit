import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { useSettingsStore } from "../../stores/settingsStore";
import type { LgePhaseId } from "../../types";

const PHASES: LgePhaseId[] = ["planning", "builder", "review", "guardian"];
const MODELS = ["opus", "sonnet", "haiku"] as const;

interface SettingsDialogProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const { t } = useTranslation();
  const {
    phaseModels, shellEnv, jiraBaseUrl, jiraEmail, jiraApiToken, loaded,
    fetchPhaseModels, savePhaseModels,
    fetchShellEnv, saveShellEnv,
    fetchJiraBaseUrl, saveJiraBaseUrl,
    fetchJiraEmail, saveJiraEmail,
    fetchJiraApiToken, saveJiraApiToken,
    verifyConnection,
  } = useSettingsStore();

  const [draft, setDraft] = useState<Record<LgePhaseId, string>>({
    ...phaseModels,
  });
  const [shellEnvDraft, setShellEnvDraft] = useState(shellEnv);
  const [jiraBaseUrlDraft, setJiraBaseUrlDraft] = useState(jiraBaseUrl);
  const [jiraEmailDraft, setJiraEmailDraft] = useState(jiraEmail);
  const [jiraTokenDraft, setJiraTokenDraft] = useState(jiraApiToken);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open && !loaded) {
      fetchPhaseModels();
      fetchShellEnv();
      fetchJiraBaseUrl();
      fetchJiraEmail();
      fetchJiraApiToken();
    }
  }, [open, loaded, fetchPhaseModels, fetchShellEnv, fetchJiraBaseUrl, fetchJiraEmail, fetchJiraApiToken]);

  useEffect(() => {
    if (open) {
      setDraft({ ...phaseModels });
      setShellEnvDraft(shellEnv);
      setJiraBaseUrlDraft(jiraBaseUrl);
      setJiraEmailDraft(jiraEmail);
      setJiraTokenDraft(jiraApiToken);
      setSaved(false);
      setTestResult(null);
      setError(null);
    }
  }, [open, phaseModels, shellEnv, jiraBaseUrl, jiraEmail, jiraApiToken]);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    setTestResult(null);
    try {
      const modelsChanged = PHASES.some((p) => draft[p] !== phaseModels[p]);
      const shellEnvChanged = shellEnvDraft !== shellEnv;
      const jiraBaseUrlChanged = jiraBaseUrlDraft.trim() !== jiraBaseUrl;
      const jiraEmailChanged = jiraEmailDraft.trim() !== jiraEmail;
      const jiraTokenChanged = jiraTokenDraft !== jiraApiToken;

      if (modelsChanged) await savePhaseModels(draft);
      if (shellEnvChanged) await saveShellEnv(shellEnvDraft);
      if (jiraBaseUrlChanged) await saveJiraBaseUrl(jiraBaseUrlDraft.trim());
      if (jiraEmailChanged) await saveJiraEmail(jiraEmailDraft.trim());
      if (jiraTokenChanged) await saveJiraApiToken(jiraTokenDraft);

      setSaved(true);
      setTimeout(() => onClose(), 800);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleTestConnection = async () => {
    setTesting(true);
    setTestResult(null);
    setError(null);
    try {
      // Persist credentials first so the backend reads what the user typed.
      if (jiraBaseUrlDraft.trim() !== jiraBaseUrl) await saveJiraBaseUrl(jiraBaseUrlDraft.trim());
      if (jiraEmailDraft.trim() !== jiraEmail) await saveJiraEmail(jiraEmailDraft.trim());
      if (jiraTokenDraft !== jiraApiToken) await saveJiraApiToken(jiraTokenDraft);

      const self = await verifyConnection();
      setTestResult(`✓ ${self.display_name}${self.email ? ` <${self.email}>` : ""}`);
    } catch (e) {
      setTestResult(null);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setTesting(false);
    }
  };

  const hasChanges =
    PHASES.some((p) => draft[p] !== phaseModels[p]) ||
    shellEnvDraft !== shellEnv ||
    jiraBaseUrlDraft.trim() !== jiraBaseUrl ||
    jiraEmailDraft.trim() !== jiraEmail ||
    jiraTokenDraft !== jiraApiToken;

  return (
    <Dialog open={open} onClose={onClose} title={t("settings.title")}>
      <div className="space-y-6">
        {/* Model per Phase */}
        <div>
          <h3 className="text-sm font-medium text-text-primary">
            {t("settings.models.title")}
          </h3>
          <p className="mt-1 text-xs text-text-muted">
            {t("settings.models.description")}
          </p>
        </div>

        <div className="space-y-3">
          {PHASES.map((phase) => (
            <div key={phase} className="flex items-center justify-between gap-4">
              <label className="text-sm text-text-secondary min-w-[80px]">
                {t(`settings.models.${phase}`)}
              </label>
              <select
                value={draft[phase]}
                disabled={!loaded}
                onChange={(e) =>
                  setDraft((prev) => ({ ...prev, [phase]: e.target.value }))
                }
                className="flex-1 rounded-lg border border-border bg-bg-surface px-3 py-2 text-sm text-text-primary focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50 disabled:opacity-50"
              >
                {MODELS.map((model) => (
                  <option key={model} value={model}>
                    {t(`settings.models.${model}`)} - {t(`settings.models.${model}Desc`)}
                  </option>
                ))}
              </select>
            </div>
          ))}
        </div>

        {/* Shell Environment */}
        <div className="border-t border-border pt-4">
          <h3 className="text-sm font-medium text-text-primary">
            {t("settings.shellEnv.title")}
          </h3>
          <p className="mt-1 text-xs text-text-muted">
            {t("settings.shellEnv.description")}
          </p>
          <textarea
            value={shellEnvDraft}
            onChange={(e) => setShellEnvDraft(e.target.value)}
            rows={4}
            spellCheck={false}
            placeholder={t("settings.shellEnv.placeholder")}
            className="mt-2 w-full rounded-lg border border-border bg-bg-surface px-3 py-2 font-mono text-xs text-text-primary placeholder:text-text-muted focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
          />
        </div>

        {/* Jira Connection */}
        <div className="border-t border-border pt-4">
          <h3 className="text-sm font-medium text-text-primary">
            {t("settings.jira.title")}
          </h3>
          <p className="mt-1 text-xs text-text-muted">
            {t("settings.jira.description")}
          </p>

          <div className="mt-2 space-y-3">
            <div>
              <label className="text-xs text-text-secondary">
                {t("settings.jira.baseUrl")}
              </label>
              <input
                type="url"
                value={jiraBaseUrlDraft}
                onChange={(e) => setJiraBaseUrlDraft(e.target.value)}
                spellCheck={false}
                placeholder={t("settings.jira.baseUrlPlaceholder")}
                className="mt-1 w-full rounded-lg border border-border bg-bg-surface px-3 py-2 text-sm text-text-primary placeholder:text-text-muted focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
              />
            </div>
            <div>
              <label className="text-xs text-text-secondary">
                {t("settings.jira.email")}
              </label>
              <input
                type="email"
                value={jiraEmailDraft}
                onChange={(e) => setJiraEmailDraft(e.target.value)}
                spellCheck={false}
                autoComplete="off"
                placeholder={t("settings.jira.emailPlaceholder")}
                className="mt-1 w-full rounded-lg border border-border bg-bg-surface px-3 py-2 text-sm text-text-primary placeholder:text-text-muted focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
              />
            </div>
            <div>
              <label className="text-xs text-text-secondary">
                {t("settings.jira.apiToken")}
              </label>
              <input
                type="password"
                value={jiraTokenDraft}
                onChange={(e) => setJiraTokenDraft(e.target.value)}
                spellCheck={false}
                autoComplete="off"
                placeholder={t("settings.jira.apiTokenPlaceholder")}
                className="mt-1 w-full rounded-lg border border-border bg-bg-surface px-3 py-2 text-sm text-text-primary placeholder:text-text-muted focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
              />
              <p className="mt-1 text-[11px] text-text-muted">
                {t("settings.jira.apiTokenHint")}
              </p>
            </div>
            <div className="flex items-center gap-3">
              <Button
                variant="ghost"
                type="button"
                onClick={() => void handleTestConnection()}
                disabled={testing}
                className="text-xs"
              >
                {testing ? t("settings.jira.testing") : t("settings.jira.testConnection")}
              </Button>
              {testResult && (
                <span className="text-xs text-success">{testResult}</span>
              )}
            </div>
          </div>
        </div>

        <div className="flex items-center justify-end gap-3 pt-2">
          {error && (
            <span className="text-xs text-error">{error}</span>
          )}
          {saved && (
            <span className="text-xs text-success">{t("settings.saved")}</span>
          )}
          <Button variant="ghost" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button
            onClick={handleSave}
            disabled={!hasChanges || saving}
          >
            {saving ? "..." : t("settings.save")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
