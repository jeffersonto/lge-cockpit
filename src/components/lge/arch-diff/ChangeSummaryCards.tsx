import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ChangeSummary } from "../../../types";

interface Props {
  summary: ChangeSummary;
}

function RiskGauge({ score }: { score: number }) {
  const radius = 28;
  const circumference = 2 * Math.PI * radius;
  const filled = (score / 100) * circumference;
  const color =
    score <= 30 ? "var(--color-success)" :
    score <= 60 ? "var(--color-warning)" :
    "var(--color-error)";

  return (
    <svg width="72" height="72" viewBox="0 0 72 72" className="rotate-[-90deg]">
      <circle cx="36" cy="36" r={radius} fill="none" stroke="var(--color-border)" strokeWidth="6" />
      <circle
        cx="36" cy="36" r={radius}
        fill="none"
        stroke={color}
        strokeWidth="6"
        strokeDasharray={`${filled} ${circumference - filled}`}
        strokeLinecap="round"
        style={{ transition: "stroke-dasharray 0.6s ease" }}
      />
    </svg>
  );
}

function riskLabel(score: number, t: (k: string) => string) {
  if (score <= 30) return t("lge.artifacts.archDiff.riskLow");
  if (score <= 60) return t("lge.artifacts.archDiff.riskMedium");
  if (score <= 80) return t("lge.artifacts.archDiff.riskHigh");
  return t("lge.artifacts.archDiff.riskCritical");
}

function riskColor(score: number) {
  if (score <= 30) return "text-success";
  if (score <= 60) return "text-warning";
  return "text-error";
}

export function ChangeSummaryCards({ summary }: Props) {
  const { t } = useTranslation();
  const [depsExpanded, setDepsExpanded] = useState(false);

  return (
    <div className="grid grid-cols-2 gap-3">
      {/* Files card */}
      <div className="rounded-lg border border-border bg-bg-card p-4">
        <p className="mb-2 text-xs font-medium text-text-muted">
          {t("lge.artifacts.archDiff.filesChanged")}
        </p>
        <div className="flex flex-col gap-1">
          <span className="text-sm font-semibold text-success">+{summary.files_added} {t("lge.artifacts.archDiff.added")}</span>
          <span className="text-sm font-semibold text-warning">~{summary.files_modified} {t("lge.artifacts.archDiff.modified")}</span>
          <span className="text-sm font-semibold text-error">-{summary.files_deleted} {t("lge.artifacts.archDiff.removed")}</span>
        </div>
      </div>

      {/* Lines card */}
      <div className="rounded-lg border border-border bg-bg-card p-4">
        <p className="mb-2 text-xs font-medium text-text-muted">
          {t("lge.artifacts.archDiff.linesChanged")}
        </p>
        <div className="flex flex-col gap-1">
          <span className="text-sm font-semibold text-success">+{summary.lines_added}</span>
          <span className="text-sm font-semibold text-error">-{summary.lines_removed}</span>
        </div>
      </div>

      {/* Dependencies card */}
      <div className="rounded-lg border border-border bg-bg-card p-4">
        <p className="mb-2 text-xs font-medium text-text-muted">
          {t("lge.artifacts.archDiff.dependencies")}
        </p>
        {summary.new_dependencies.length === 0 ? (
          <span className="text-sm text-text-muted">—</span>
        ) : (
          <button
            onClick={() => setDepsExpanded(!depsExpanded)}
            className="text-left"
          >
            <span className="text-sm font-semibold text-accent">
              +{summary.new_dependencies.length}
            </span>
            {depsExpanded && (
              <ul className="mt-2 space-y-1">
                {summary.new_dependencies.slice(0, 8).map((dep, i) => (
                  <li key={i} className="text-xs text-text-secondary truncate max-w-[120px]">
                    {dep}
                  </li>
                ))}
                {summary.new_dependencies.length > 8 && (
                  <li className="text-xs text-text-muted">+{summary.new_dependencies.length - 8} more</li>
                )}
              </ul>
            )}
          </button>
        )}
      </div>

      {/* Risk card */}
      <div className="rounded-lg border border-border bg-bg-card p-4">
        <p className="mb-2 text-xs font-medium text-text-muted">
          {t("lge.artifacts.archDiff.riskScore")}
        </p>
        <div className="flex items-center gap-3">
          <div className="relative flex items-center justify-center">
            <RiskGauge score={summary.risk_score} />
            <span className={`absolute text-sm font-bold rotate-[90deg] ${riskColor(summary.risk_score)}`}>
              {summary.risk_score}
            </span>
          </div>
          <div className="flex flex-col gap-0.5 min-w-0">
            <span className={`text-xs font-semibold ${riskColor(summary.risk_score)}`}>
              {riskLabel(summary.risk_score, t)}
            </span>
            {summary.risk_factors.slice(0, 2).map((f, i) => (
              <span key={i} className="text-xs text-text-muted leading-tight truncate">{f}</span>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
