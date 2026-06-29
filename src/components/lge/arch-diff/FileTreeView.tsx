import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { FileNode } from "../../../types";

interface Props {
  nodes: FileNode[];
}

const CHANGE_COLORS = {
  added: "bg-success",
  modified: "bg-warning",
  deleted: "bg-error",
} as const;

const CHANGE_TEXT = {
  added: "text-success",
  modified: "text-warning",
  deleted: "text-error",
} as const;

function TreeNode({ node, depth }: { node: FileNode; depth: number }) {
  const [expanded, setExpanded] = useState(depth < 2);
  const indent = depth * 16;

  const dotColor = CHANGE_COLORS[node.change_type as keyof typeof CHANGE_COLORS] ?? "bg-text-muted";
  const nameColor = CHANGE_TEXT[node.change_type as keyof typeof CHANGE_TEXT] ?? "text-text-secondary";
  const totalLines = node.additions + node.deletions;

  return (
    <>
      <div
        className="flex items-center gap-2 rounded px-2 py-1 hover:bg-bg-hover cursor-default group"
        style={{ paddingLeft: `${8 + indent}px` }}
        onClick={() => node.is_directory && setExpanded(!expanded)}
      >
        {/* Chevron for directories */}
        {node.is_directory ? (
          <span className={`text-text-muted transition-transform duration-150 text-xs ${expanded ? "rotate-90" : ""}`}>
            ▶
          </span>
        ) : (
          <span className="w-3" />
        )}

        {/* Change dot */}
        <span className={`w-2 h-2 rounded-full flex-shrink-0 ${dotColor}`} />

        {/* File/dir name */}
        <span className={`text-xs truncate flex-1 ${node.is_directory ? "text-text-secondary font-medium" : nameColor}`}>
          {node.path.split("/").pop()}
        </span>

        {/* Line change bar */}
        {!node.is_directory && totalLines > 0 && (
          <div className="flex items-center gap-1 flex-shrink-0">
            <div className="flex h-1.5 w-16 overflow-hidden rounded-full bg-bg-surface">
              <div
                className="h-full bg-success"
                style={{ width: `${(node.additions / totalLines) * 100}%` }}
              />
              <div
                className="h-full bg-error"
                style={{ width: `${(node.deletions / totalLines) * 100}%` }}
              />
            </div>
            <span className="text-xs text-text-muted w-12 text-right">
              +{node.additions} -{node.deletions}
            </span>
          </div>
        )}
      </div>

      {/* Children */}
      {node.is_directory && expanded && node.children.map((child, i) => (
        <TreeNode key={i} node={child} depth={depth + 1} />
      ))}
    </>
  );
}

export function FileTreeView({ nodes }: Props) {
  const { t } = useTranslation();
  const [allExpanded, setAllExpanded] = useState<boolean | null>(null);

  const totalFiles = countFiles(nodes);

  if (totalFiles === 0) {
    return (
      <p className="text-sm text-text-muted text-center py-4">
        {t("lge.artifacts.archDiff.depGraphEmpty")}
      </p>
    );
  }

  return (
    <div>
      <div className="mb-2 flex items-center justify-between">
        <span className="text-xs text-text-muted">{totalFiles} file(s)</span>
        <div className="flex gap-2">
          <button
            onClick={() => setAllExpanded(true)}
            className="text-xs text-text-muted hover:text-text-primary"
          >
            {t("lge.artifacts.archDiff.expandAll")}
          </button>
          <span className="text-text-muted">·</span>
          <button
            onClick={() => setAllExpanded(false)}
            className="text-xs text-text-muted hover:text-text-primary"
          >
            {t("lge.artifacts.archDiff.collapseAll")}
          </button>
        </div>
      </div>
      <div
        key={allExpanded === null ? "default" : String(allExpanded)}
        className="rounded-lg border border-border bg-bg-surface overflow-hidden"
      >
        {nodes.map((node, i) => (
          <TreeNode key={i} node={node} depth={0} />
        ))}
      </div>
    </div>
  );
}

function countFiles(nodes: FileNode[]): number {
  return nodes.reduce((acc, n) => {
    if (n.is_directory) return acc + countFiles(n.children);
    return acc + 1;
  }, 0);
}
