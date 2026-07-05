import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { useSettingsStore } from "../../stores/settingsStore";
import type { LgePhaseId } from "../../types";

const PHASES: LgePhaseId[] = ["planning", "builder", "review", "guardian"];
const MODELS = ["opus", "sonnet", "haiku"] as const;

const TABS = [
  { id: "model", labelKey: "settings.tabs.model" },
  { id: "jira", labelKey: "settings.tabs.jira" },
  { id: "environment", labelKey: "settings.tabs.environment" },
] as const;
type TabId = (typeof TABS)[number]["id"];

interface SettingsDialogProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const { t } = useTranslation();
  const {
    phaseModels, shellEnv, jiraConfig, loaded,
    fetchPhaseModels, savePhaseModels,
    fetchShellEnv, saveShellEnv,
    fetchJiraConfig, saveJiraConfig,
    verifyConnection,
  } = useSettingsStore();

  const [draft, setDraft] = useState<Record<LgePhaseId, string>>({
    ...phaseModels,
  });
  const [shellEnvDraft, setShellEnvDraft] = useState(shellEnv);
  const [jiraBaseUrlDraft, setJiraBaseUrlDraft] = useState(jiraConfig.baseUrl);
  const [jiraEmailDraft, setJiraEmailDraft] = useState(jiraConfig.email);
  const [jiraTokenDraft, setJiraTokenDraft] = useState(jiraConfig.apiToken);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<TabId>("model");

  useEffect(() => {
    if (open && !loaded) {
      fetchPhaseModels();
      fetchShellEnv();
      fetchJiraConfig();
    }
  }, [open, loaded, fetchPhaseModels, fetchShellEnv, fetchJiraConfig]);

  useEffect(() => {
    if (open) {
      setDraft({ ...phaseModels });
      setShellEnvDraft(shellEnv);
      setJiraBaseUrlDraft(jiraConfig.baseUrl);
      setJiraEmailDraft(jiraConfig.email);
      setJiraTokenDraft(jiraConfig.apiToken);
      setSaved(false);
      setTestResult(null);
      setError(null);
      setActiveTab("model");
    }
  }, [open, phaseModels, shellEnv, jiraConfig]);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    setTestResult(null);
    try {
      const modelsChanged = PHASES.some((p) => draft[p] !== phaseModels[p]);
      const shellEnvChanged = shellEnvDraft !== shellEnv;
      const jiraDirty =
        jiraBaseUrlDraft.trim() !== jiraConfig.baseUrl ||
        jiraEmailDraft.trim() !== jiraConfig.email ||
        jiraTokenDraft !== jiraConfig.apiToken;

      if (modelsChanged) await savePhaseModels(draft);
      if (shellEnvChanged) await saveShellEnv(shellEnvDraft);
      if (jiraDirty) {
        await saveJiraConfig({
          baseUrl: jiraBaseUrlDraft.trim(),
          email: jiraEmailDraft.trim(),
          apiToken: jiraTokenDraft,
        });
      }

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
      const jiraDirty =
        jiraBaseUrlDraft.trim() !== jiraConfig.baseUrl ||
        jiraEmailDraft.trim() !== jiraConfig.email ||
        jiraTokenDraft !== jiraConfig.apiToken;
      if (jiraDirty) {
        await saveJiraConfig({
          baseUrl: jiraBaseUrlDraft.trim(),
          email: jiraEmailDraft.trim(),
          apiToken: jiraTokenDraft,
        });
      }

      const self = await verifyConnection();
      setTestResult(`✓ ${self.display_name}${self.email ? ` <${self.email}>` : ""}`);
    } catch (e) {
      setTestResult(null);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setTesting(false);
    }
  };

  const modeloDirty = PHASES.some((p) => draft[p] !== phaseModels[p]);
  const jiraDirty =
    jiraBaseUrlDraft.trim() !== jiraConfig.baseUrl ||
    jiraEmailDraft.trim() !== jiraConfig.email ||
    jiraTokenDraft !== jiraConfig.apiToken;
  const ambienteDirty = shellEnvDraft !== shellEnv;
  const hasChanges = modeloDirty || jiraDirty || ambienteDirty;
  const tabDirty: Record<TabId, boolean> = {
    model: modeloDirty,
    jira: jiraDirty,
    environment: ambienteDirty,
  };

  return (
    <Dialog open={open} onClose={onClose} title={t("settings.title")}>
      {/* Tab bar — underline-accent idiom (cf. LgeArtifactPanel) */}
      <div className="flex border-b border-border">
        {TABS.map((tab) => {
          const active = activeTab === tab.id;
          const dirty = tabDirty[tab.id];
          return (
            <button
              key={tab.id}
              type="button"
              onClick={() => setActiveTab(tab.id)}
              className={`relative px-4 py-2 text-sm transition-colors ${
                active
                  ? "border-b-2 border-accent text-accent"
                  : "text-text-secondary hover:text-text-primary"
              }`}
            >
              {t(tab.labelKey)}
              {dirty && (
                <span
                  aria-hidden="true"
                  className="ml-1.5 inline-block h-1.5 w-1.5 rounded-full bg-accent align-middle"
                />
              )}
            </button>
          );
        })}
      </div>

      {/* Tab body — fixed min-height avoids jumping between tabs */}
      <div className="mt-4 min-h-[280px]">
        {activeTab === "model" && (
          <div className="space-y-6">
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
          </div>
        )}

        {activeTab === "jira" && (
          <div className="space-y-4">
            <div>
              <h3 className="text-sm font-medium text-text-primary">
                {t("settings.jira.title")}
              </h3>
              <p className="mt-1 text-xs text-text-muted">
                {t("settings.jira.description")}
              </p>
            </div>

            <div className="space-y-3">
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
        )}

        {activeTab === "environment" && (
          <div className="space-y-4">
            <div>
              <h3 className="text-sm font-medium text-text-primary">
                {t("settings.shellEnv.title")}
              </h3>
              <p className="mt-1 text-xs text-text-muted">
                {t("settings.shellEnv.description")}
              </p>
            </div>
            <textarea
              value={shellEnvDraft}
              onChange={(e) => setShellEnvDraft(e.target.value)}
              rows={4}
              spellCheck={false}
              placeholder={t("settings.shellEnv.placeholder")}
              className="w-full rounded-lg border border-border bg-bg-surface px-3 py-2 font-mono text-xs text-text-primary placeholder:text-text-muted focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
            />
          </div>
        )}
      </div>

      <div className="flex items-center justify-end gap-3 pt-4">
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
    </Dialog>
  );
}