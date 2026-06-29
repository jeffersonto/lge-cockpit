import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useTaskStore } from "../../stores/taskStore";
import { SettingsDialog } from "../settings/SettingsDialog";
import { WhatsNewDialog } from "./WhatsNewDialog";
import { CURRENT_VERSION, WHATS_NEW_STORAGE_KEY } from "../../data/releaseNotes";

const LANGUAGES = [
  { code: "pt-BR", label: "PT" },
  { code: "en", label: "EN" },
  { code: "es", label: "ES" },
] as const;

export function StatusBar() {
  const { t, i18n } = useTranslation();
  const tasks = useTaskStore((s) => s.tasks);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [whatsNewOpen, setWhatsNewOpen] = useState(false);
  const [isNewVersion, setIsNewVersion] = useState(
    () => localStorage.getItem(WHATS_NEW_STORAGE_KEY) !== CURRENT_VERSION
  );

  return (
    <>
      <div className="flex h-8 items-center justify-between border-t border-border bg-bg-surface px-4 text-xs text-text-muted">
        <div className="flex items-center gap-3">
          <span className="flex items-center gap-1.5">
            <span className="h-1.5 w-1.5 rounded-full bg-success" />
            {t("statusbar.ready")}
          </span>
          {tasks.length > 0 && (
            <span>{t("statusbar.tasksCount", { count: tasks.length })}</span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => {
              localStorage.setItem(WHATS_NEW_STORAGE_KEY, CURRENT_VERSION);
              setIsNewVersion(false);
              setWhatsNewOpen(true);
            }}
            className="relative flex items-center gap-1 rounded px-1.5 py-0.5 font-mono transition-colors hover:bg-bg-hover hover:text-text-secondary"
            title={t("whatsNew.chipTooltip")}
          >
            v{CURRENT_VERSION}
            {isNewVersion && (
              <span className="absolute -right-0.5 -top-0.5 h-1.5 w-1.5 animate-pulse rounded-full bg-accent" />
            )}
          </button>
          <div className="h-3 w-px bg-border" />
          <button
            onClick={() => setSettingsOpen(true)}
            className="rounded p-1 transition-colors hover:bg-bg-hover hover:text-text-secondary"
            title={t("settings.title")}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
              <circle cx="12" cy="12" r="3" />
            </svg>
          </button>
          <div className="h-3 w-px bg-border" />
          <div className="flex items-center gap-1">
            {LANGUAGES.map((lang) => (
              <button
                key={lang.code}
                onClick={() => i18n.changeLanguage(lang.code)}
                className={`rounded px-1.5 py-0.5 transition-colors ${
                  i18n.language === lang.code
                    ? "bg-accent/20 text-accent"
                    : "hover:bg-bg-hover hover:text-text-secondary"
                }`}
              >
                {lang.label}
              </button>
            ))}
          </div>
        </div>
      </div>
      <SettingsDialog
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />
      <WhatsNewDialog
        open={whatsNewOpen}
        onClose={() => setWhatsNewOpen(false)}
      />
    </>
  );
}
