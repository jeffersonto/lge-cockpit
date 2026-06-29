import { useMemo, useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useLgeStore, PHASE_ORDER } from "../../stores/lgeStore";
import type { LgePhaseId } from "../../types";
import { ArchDiffView } from "./arch-diff/ArchDiffView";
import { fixMarkdownTables } from "../../lib/markdown";

const TAB_KEYS: Record<LgePhaseId, string> = {
  planning: "lge.artifacts.plan",
  builder: "lge.artifacts.builder",
  review: "lge.artifacts.reviewer",
  guardian: "lge.artifacts.guardian",
};

const CODE_PHASES: LgePhaseId[] = ["builder", "review", "guardian"];

export function LgeArtifactPanel() {
  const { t } = useTranslation();
  const { viewingTaskId, processes, setSelectedArtifactTab, updateArtifact, analyzePhaseArchDiff } = useLgeStore();

  const [isEditing, setIsEditing] = useState(false);
  const [editContent, setEditContent] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [activeSubTab, setActiveSubTab] = useState<"artifact" | "architecture">("artifact");

  const process = viewingTaskId ? (processes[viewingTaskId] ?? null) : null;
  const selectedArtifactTab = process?.selectedArtifactTab ?? "planning";
  const rawArtifact = process ? (process.phases[selectedArtifactTab]?.artifact ?? null) : null;
  const currentPhaseState = process?.phases[selectedArtifactTab];
  const archDiff = currentPhaseState?.archDiff ?? null;
  const isAnalyzingArch = currentPhaseState?.isAnalyzingArch ?? false;

  // Show Architecture sub-tab for any completed code-modifying phase
  const showArchTab =
    CODE_PHASES.includes(selectedArtifactTab) &&
    currentPhaseState?.status === "completed";

  const currentArtifact = useMemo(
    () => (rawArtifact ? fixMarkdownTables(rawArtifact) : null),
    [rawArtifact]
  );

  // Reset sub-tab and edit state when phase tab or task changes
  useEffect(() => {
    setIsEditing(false);
    setEditContent("");
    setActiveSubTab("artifact");
  }, [selectedArtifactTab, viewingTaskId]);

  // Collect all arch diffs for the timeline component
  const allPhasesDiffs = process
    ? {
        builder: process.phases.builder.archDiff,
        review: process.phases.review.archDiff,
        guardian: process.phases.guardian.archDiff,
      }
    : {};

  if (!viewingTaskId || !process) return null;

  const { phases } = process;

  const handleStartEdit = () => {
    setEditContent(rawArtifact ?? "");
    setIsEditing(true);
  };

  const handleCancel = () => {
    setIsEditing(false);
    setEditContent("");
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await updateArtifact(viewingTaskId, selectedArtifactTab, editContent);
      setIsEditing(false);
      setEditContent("");
    } catch (err) {
      console.error("Failed to save artifact:", err);
    } finally {
      setIsSaving(false);
    }
  };

  const handleAnalyze = () => {
    analyzePhaseArchDiff(viewingTaskId, selectedArtifactTab);
  };

  return (
    <div className="flex h-full flex-col border-l border-border bg-bg-primary">
      {/* Phase tab bar */}
      <div className="flex border-b border-border bg-bg-surface">
        {PHASE_ORDER.map((phaseId) => {
          const phase = phases[phaseId];
          const hasArtifact = phase.status === "completed" && phase.artifact;
          const isSelected = selectedArtifactTab === phaseId;

          return (
            <button
              key={phaseId}
              onClick={() => hasArtifact && setSelectedArtifactTab(viewingTaskId, phaseId)}
              disabled={!hasArtifact}
              className={`px-3 py-2 text-xs font-medium transition-colors ${
                isSelected
                  ? "border-b-2 border-accent text-accent"
                  : hasArtifact
                    ? "text-text-secondary hover:text-text-primary"
                    : "cursor-not-allowed text-text-muted/40"
              }`}
            >
              {t(TAB_KEYS[phaseId])}
            </button>
          );
        })}
      </div>

      {/* Sub-tabs: Artifact | Architecture (for any completed code-modifying phase) */}
      {showArchTab && (
        <div className="flex border-b border-border bg-bg-surface px-2">
          <button
            onClick={() => setActiveSubTab("artifact")}
            className={`px-3 py-1.5 text-xs transition-colors ${
              activeSubTab === "artifact"
                ? "border-b-2 border-accent text-accent"
                : "text-text-muted hover:text-text-secondary"
            }`}
          >
            {t("lge.artifacts.artifactTab")}
          </button>
          <button
            onClick={() => setActiveSubTab("architecture")}
            className={`flex items-center gap-1.5 px-3 py-1.5 text-xs transition-colors ${
              activeSubTab === "architecture"
                ? "border-b-2 border-accent text-accent"
                : "text-text-muted hover:text-text-secondary"
            }`}
          >
            {t("lge.artifacts.archTab")}
            {!archDiff && !isAnalyzingArch && (
              <span className="rounded bg-accent/20 px-1 py-0.5 text-[10px] font-medium text-accent">
                ⚡
              </span>
            )}
            {isAnalyzingArch && (
              <span className="h-2.5 w-2.5 animate-spin rounded-full border border-accent border-t-transparent" />
            )}
          </button>
        </div>
      )}

      {/* Architecture view */}
      {showArchTab && activeSubTab === "architecture" && (
        <div className="flex-1 min-h-0">
          <ArchDiffView
            archDiff={archDiff}
            isAnalyzing={isAnalyzingArch}
            onAnalyze={handleAnalyze}
            allPhasesDiffs={allPhasesDiffs}
          />
        </div>
      )}

      {/* Artifact view (edit toolbar + content) */}
      {activeSubTab === "artifact" && (
        <>
          {/* Edit toolbar */}
          {rawArtifact && (
            <div className="flex items-center justify-end gap-2 border-b border-border px-4 py-1.5">
              {isEditing ? (
                <>
                  <button
                    onClick={handleCancel}
                    disabled={isSaving}
                    className="rounded px-3 py-1 text-xs text-text-secondary hover:bg-bg-surface"
                  >
                    {t("lge.artifacts.cancel")}
                  </button>
                  <button
                    onClick={handleSave}
                    disabled={isSaving}
                    className="rounded bg-accent px-3 py-1 text-xs text-white hover:bg-accent/80 disabled:opacity-50"
                  >
                    {isSaving ? t("lge.artifacts.saving") : t("lge.artifacts.save")}
                  </button>
                </>
              ) : (
                <button
                  onClick={handleStartEdit}
                  className="rounded px-3 py-1 text-xs text-text-secondary hover:bg-bg-surface"
                >
                  {t("lge.artifacts.edit")}
                </button>
              )}
            </div>
          )}

          {/* Content */}
          <div className={`flex-1 min-h-0 ${isEditing ? "flex flex-col p-4" : "overflow-y-auto p-4"}`}>
            {currentArtifact ? (
              isEditing ? (
                <textarea
                  value={editContent}
                  onChange={(e) => setEditContent(e.target.value)}
                  className="flex-1 w-full resize-none rounded border border-border bg-bg-surface p-3 font-mono text-sm text-text-primary focus:border-accent focus:outline-none"
                />
              ) : (
                <div className="prose prose-sm max-w-none">
                  <Markdown remarkPlugins={[remarkGfm]}>{currentArtifact}</Markdown>
                </div>
              )
            ) : (
              <div className="flex h-full items-center justify-center">
                <p className="text-sm text-text-muted">{t("lge.artifacts.empty")}</p>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
