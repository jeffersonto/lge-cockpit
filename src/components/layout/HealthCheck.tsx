import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/Button";
import { LgeIcon } from "../ui/LgeIcon";
import type { DependencyStatus } from "../../types";
import * as api from "../../lib/tauri";

interface HealthCheckProps {
  onDismiss: () => void;
}

type CheckState = "checking" | "done";

interface DepCheck {
  dep: DependencyStatus | null;
  state: CheckState;
}

export function HealthCheck({ onDismiss }: HealthCheckProps) {
  const { t } = useTranslation();
  const [checks, setChecks] = useState<DepCheck[]>([
    { dep: null, state: "checking" },
    { dep: null, state: "checking" },
    { dep: null, state: "checking" },
  ]);
  const [allDone, setAllDone] = useState(false);
  const [allOk, setAllOk] = useState(false);
  const [copiedIdx, setCopiedIdx] = useState<number | null>(null);

  useEffect(() => {
    runSequentialCheck();
  }, []);

  const runSequentialCheck = async () => {
    setAllDone(false);
    setChecks([
      { dep: null, state: "checking" },
      { dep: null, state: "checking" },
      { dep: null, state: "checking" },
    ]);

    try {
      const result = await api.checkDependencies();

      // Reveal each dependency one by one with a delay
      for (let i = 0; i < result.dependencies.length; i++) {
        await delay(400);
        setChecks((prev) => {
          const next = [...prev];
          next[i] = { dep: result.dependencies[i], state: "done" };
          return next;
        });
      }

      await delay(300);
      setAllDone(true);
      setAllOk(result.all_ok);

      if (result.all_ok) {
        await delay(1200);
        onDismiss();
      }
    } catch (err) {
      console.error("Health check failed:", err);
      setAllDone(true);
    }
  };

  const copyCommand = (cmd: string, idx: number) => {
    navigator.clipboard.writeText(cmd);
    setCopiedIdx(idx);
    setTimeout(() => setCopiedIdx(null), 2000);
  };

  return (
    <div className="flex h-screen items-center justify-center bg-bg-primary p-8">
      <div className="w-full max-w-lg">
        {/* Header */}
        <div className="mb-8 text-center">
          <div className="relative mx-auto mb-5 w-fit">
            {/* Animated rings behind icon while checking */}
            {!allDone && (
              <>
                <span className="absolute inset-0 -m-3 animate-ping rounded-full bg-accent/20" style={{ animationDuration: "2s" }} />
                <span className="absolute inset-0 -m-1 animate-ping rounded-full bg-accent/10" style={{ animationDuration: "2.6s", animationDelay: "0.4s" }} />
              </>
            )}
            {/* Success halo */}
            {allDone && allOk && (
              <span className="absolute inset-0 -m-2 rounded-full bg-success/15 blur-sm" />
            )}
            <LgeIcon size={80} />
          </div>
          <h1 className="text-xl font-bold text-text-primary">LGE Cockpit</h1>
          <p className="mt-1 text-sm text-text-muted">{t("health.checking")}</p>
        </div>

        {/* Dependency list */}
        <div className="mb-6 rounded-xl border border-border bg-bg-surface p-4">
          <div className="flex flex-col gap-0">
            {checks.map((check, idx) => (
              <div key={idx}>
                {idx > 0 && <div className="ml-3 h-3 border-l border-border" />}
                <CheckRow
                  check={check}
                  idx={idx}
                  copiedIdx={copiedIdx}
                  onCopy={copyCommand}
                />
              </div>
            ))}
          </div>
        </div>

        {/* Actions */}
        {allDone && (
          <div className="flex justify-center gap-3">
            {allOk ? (
              <p className="text-sm text-success">{t("health.allOk")}</p>
            ) : (
              <>
                <Button onClick={runSequentialCheck} variant="secondary" size="sm">
                  {t("health.recheck")}
                </Button>
                <Button onClick={onDismiss} variant="ghost" size="sm">
                  {t("health.continueAnyway")}
                </Button>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function CheckRow({
  check,
  idx,
  copiedIdx,
  onCopy,
}: {
  check: DepCheck;
  idx: number;
  copiedIdx: number | null;
  onCopy: (cmd: string, idx: number) => void;
}) {
  const { t } = useTranslation();
  const { dep, state } = check;

  return (
    <div className="flex items-start gap-3 py-2">
      {/* Status icon */}
      <div className="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center">
        {state === "checking" ? (
          <div className="h-4 w-4 animate-spin rounded-full border-2 border-accent border-t-transparent" />
        ) : dep?.available ? (
          <svg width="18" height="18" viewBox="0 0 18 18" fill="none" className="text-success">
            <circle cx="9" cy="9" r="8" stroke="currentColor" strokeWidth="1.5" />
            <path d="M5.5 9l2.5 2.5 4.5-4.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        ) : (
          <svg width="18" height="18" viewBox="0 0 18 18" fill="none" className="text-error">
            <circle cx="9" cy="9" r="8" stroke="currentColor" strokeWidth="1.5" />
            <path d="M6 6l6 6M12 6l-6 6" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        {state === "checking" ? (
          <div className="flex items-center gap-2">
            <span className="text-sm text-text-muted">{t("health.verifying")}...</span>
          </div>
        ) : dep ? (
          <>
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium text-text-primary">{dep.name}</span>
              {dep.version && (
                <span className="rounded bg-bg-card px-1.5 py-0.5 text-[10px] text-text-muted">
                  {dep.version}
                </span>
              )}
            </div>
            <p className="mt-0.5 text-xs text-text-muted">{dep.description}</p>

            {/* Manual command */}
            {!dep.available && dep.install_command && (
              <div className="mt-2 flex items-center gap-1.5">
                <code className="flex-1 truncate rounded bg-bg-card px-2 py-1 text-[11px] text-text-secondary">
                  {dep.install_command}
                </code>
                <button
                  onClick={() => onCopy(dep.install_command!, idx)}
                  className="shrink-0 rounded bg-bg-card px-2 py-1 text-[11px] text-text-muted transition-colors hover:text-text-primary"
                >
                  {copiedIdx === idx ? t("health.copied") : t("health.copy")}
                </button>
              </div>
            )}
          </>
        ) : null}
      </div>
    </div>
  );
}

function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
