import { useTranslation } from "react-i18next";
import type { ArchitectureDiff, LgePhaseId } from "../../../types";

interface Props {
  allDiffs: Partial<Record<LgePhaseId, ArchitectureDiff | null>>;
}

const PHASE_LABELS: Record<string, string> = {
  builder: "Builder",
  review: "Review",
  guardian: "Guardian",
};

function riskColor(score: number) {
  if (score <= 30) return "text-success";
  if (score <= 60) return "text-warning";
  return "text-error";
}

function Delta({ prev, curr }: { prev: number; curr: number }) {
  const diff = curr - prev;
  if (diff === 0) return <span className="text-xs text-text-muted">=</span>;
  const isGood = diff < 0;
  return (
    <span className={`text-xs font-medium ${isGood ? "text-success" : "text-error"}`}>
      {isGood ? "↓" : "↑"} {Math.abs(diff)}
    </span>
  );
}

export function PhaseComparisonTimeline({ allDiffs }: Props) {
  const { t } = useTranslation();

  const phases: LgePhaseId[] = ["builder", "review", "guardian"];
  const availablePhases = phases.filter((p) => allDiffs[p] != null);

  if (availablePhases.length < 2) {
    return null; // Only show when 2+ phases have data
  }

  return (
    <div>
      <h3 className="mb-3 text-xs font-semibold uppercase tracking-wider text-text-muted">
        {t("lge.artifacts.archDiff.timeline")}
      </h3>
      <div className="flex items-start gap-0">
        {availablePhases.map((phaseId, idx) => {
          const diff = allDiffs[phaseId]!;
          const prevDiff = idx > 0 ? allDiffs[availablePhases[idx - 1]] : null;

          return (
            <div key={phaseId} className="flex items-start">
              {/* Phase card */}
              <div className="w-36 rounded-lg border border-border bg-bg-card p-3">
                <p className="mb-2 text-xs font-semibold text-accent">
                  {PHASE_LABELS[phaseId] ?? phaseId}
                </p>
                <div className="space-y-1.5">
                  <div className="flex justify-between">
                    <span className="text-xs text-text-muted">Files</span>
                    <span className="text-xs text-text-primary">
                      {diff.summary.files_added + diff.summary.files_modified + diff.summary.files_deleted}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-xs text-text-muted">Lines</span>
                    <span className="text-xs text-text-primary">
                      +{diff.summary.lines_added} -{diff.summary.lines_removed}
                    </span>
                  </div>
                  <div className="flex justify-between items-center">
                    <span className="text-xs text-text-muted">Risk</span>
                    <div className="flex items-center gap-1.5">
                      <span className={`text-xs font-bold ${riskColor(diff.summary.risk_score)}`}>
                        {diff.summary.risk_score}
                      </span>
                      {prevDiff && (
                        <Delta
                          prev={prevDiff.summary.risk_score}
                          curr={diff.summary.risk_score}
                        />
                      )}
                    </div>
                  </div>
                </div>
              </div>

              {/* Arrow connector */}
              {idx < availablePhases.length - 1 && (
                <div className="flex items-center self-center px-2">
                  <div className="h-px w-6 bg-border" />
                  <span className="text-text-muted text-xs">→</span>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
