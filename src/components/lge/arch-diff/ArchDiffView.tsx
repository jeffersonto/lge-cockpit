import { useTranslation } from "react-i18next";
import type { ArchitectureDiff, LgePhaseId } from "../../../types";
import { ChangeSummaryCards } from "./ChangeSummaryCards";

interface Props {
  archDiff: ArchitectureDiff | null;
  isAnalyzing: boolean;
  onAnalyze: () => void;
  allPhasesDiffs?: Partial<Record<LgePhaseId, ArchitectureDiff | null>>;
}


function SkeletonBlock({ className }: { className?: string }) {
  return (
    <div className={`animate-pulse rounded bg-bg-card ${className ?? "h-4 w-full"}`} />
  );
}

function LoadingSkeleton() {
  return (
    <div className="space-y-6 p-4 animate-fadeIn">
      {/* Cards skeleton */}
      <div className="grid grid-cols-4 gap-3">
        {[1, 2, 3, 4].map((i) => (
          <div key={i} className="rounded-lg border border-border bg-bg-card p-4 space-y-2">
            <SkeletonBlock className="h-3 w-16" />
            <SkeletonBlock className="h-5 w-10" />
            <SkeletonBlock className="h-3 w-12" />
          </div>
        ))}
      </div>
      {/* Tree skeleton */}
      <div className="space-y-2">
        <SkeletonBlock className="h-3 w-32" />
        <div className="rounded-lg border border-border bg-bg-surface p-3 space-y-2">
          {[1, 2, 3, 4, 5].map((i) => (
            <SkeletonBlock key={i} className={`h-3 ${i % 2 === 0 ? "w-3/4 ml-4" : "w-1/2"}`} />
          ))}
        </div>
      </div>
      {/* Diagram skeleton */}
      <div className="space-y-2">
        <SkeletonBlock className="h-3 w-40" />
        <SkeletonBlock className="h-32 rounded-lg" />
      </div>
    </div>
  );
}

function EmptyState({ onAnalyze, t }: { onAnalyze: () => void; t: (k: string) => string }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-6 p-8 animate-fadeIn">
      {/* Icon */}
      <div className="flex h-16 w-16 items-center justify-center rounded-2xl border border-accent/30 bg-accent/10">
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-accent">
          <path d="M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3z" strokeLinecap="round" strokeLinejoin="round" />
          <path d="M17.5 14v7M14 17.5h7" strokeLinecap="round" />
          <path d="M10 6.5h4M17.5 10v4M6.5 10v4" strokeLinecap="round" strokeDasharray="2 2" />
        </svg>
      </div>

      {/* Text */}
      <div className="text-center">
        <h3 className="mb-2 text-sm font-semibold text-text-primary">
          {t("lge.artifacts.archDiff.emptyTitle")}
        </h3>
        <p className="max-w-xs text-xs text-text-muted leading-relaxed">
          {t("lge.artifacts.archDiff.emptyDesc")}
        </p>
        <p className="mt-1 text-xs text-text-muted/60">
          {t("lge.artifacts.archDiff.emptyHint")}
        </p>
      </div>

      {/* CTA */}
      <button
        onClick={onAnalyze}
        className="flex items-center gap-2 rounded-lg bg-accent px-5 py-2.5 text-sm font-medium text-white shadow-lg shadow-accent/20 transition-all hover:bg-accent-hover hover:shadow-accent/30 active:scale-95"
      >
        <span>⚡</span>
        {t("lge.artifacts.archDiff.analyzeBtn")}
      </button>

      {/* Badges */}
      <div className="flex items-center gap-3">
        {["Rápido", "Offline", "Git diff"].map((badge) => (
          <span key={badge} className="rounded-full border border-border px-2 py-0.5 text-[10px] text-text-muted">
            {badge}
          </span>
        ))}
      </div>
    </div>
  );
}

export function ArchDiffView({ archDiff, isAnalyzing, onAnalyze }: Props) {
  const { t } = useTranslation();

  // Loading state
  if (isAnalyzing) {
    return <LoadingSkeleton />;
  }

  // Empty state — CTA to generate
  if (!archDiff) {
    return <EmptyState onAnalyze={onAnalyze} t={t} />;
  }

  return (
    <div className="animate-fadeIn flex h-full flex-col overflow-y-auto">
      {/* Reanalyze toolbar */}
      <div className="flex items-center justify-end border-b border-border px-4 py-1.5">
        <button
          onClick={onAnalyze}
          className="flex items-center gap-1.5 rounded px-2 py-1 text-xs text-text-muted transition-colors hover:bg-bg-surface hover:text-text-primary"
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M1 4v6h6M23 20v-6h-6" strokeLinecap="round" strokeLinejoin="round" />
            <path d="M20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4-4.64 4.36A9 9 0 0 1 3.51 15" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
          {t("lge.artifacts.archDiff.reanalyze")}
        </button>
      </div>

      {/* Summary Dashboard only */}
      <div className="p-4">
        <h3 className="mb-3 text-xs font-semibold uppercase tracking-wider text-text-muted">
          {t("lge.artifacts.archDiff.summary")}
        </h3>
        <ChangeSummaryCards summary={archDiff.summary} />
      </div>
    </div>
  );
}
