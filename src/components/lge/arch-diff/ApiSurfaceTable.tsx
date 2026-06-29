import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ApiChange } from "../../../types";

interface Props {
  changes: ApiChange[];
}

const CHANGE_COLORS = {
  added: "bg-success/10 border-success/30",
  modified: "bg-warning/10 border-warning/30",
  removed: "bg-error/10 border-error/30",
} as const;

const CHANGE_BADGE = {
  added: "bg-success/20 text-success",
  modified: "bg-warning/20 text-warning",
  removed: "bg-error/20 text-error",
} as const;

const KIND_BADGE: Record<string, string> = {
  function: "bg-accent/20 text-accent",
  type: "bg-blue-500/20 text-blue-400",
  interface: "bg-blue-500/20 text-blue-400",
  struct: "bg-purple-500/20 text-purple-400",
  enum: "bg-orange-500/20 text-orange-400",
  trait: "bg-teal-500/20 text-teal-400",
  class: "bg-indigo-500/20 text-indigo-400",
  unknown: "bg-bg-hover text-text-muted",
};

function Badge({ text, className }: { text: string; className: string }) {
  return (
    <span className={`inline-flex items-center rounded px-1.5 py-0.5 text-xs font-medium ${className}`}>
      {text}
    </span>
  );
}

export function ApiSurfaceTable({ changes }: Props) {
  const { t } = useTranslation();

  // Group by file
  const byFile = changes.reduce<Record<string, ApiChange[]>>((acc, c) => {
    const key = c.file || "(unknown)";
    if (!acc[key]) acc[key] = [];
    acc[key].push(c);
    return acc;
  }, {});

  const files = Object.keys(byFile).sort();

  if (files.length === 0) {
    return (
      <p className="text-center text-sm text-text-muted py-4">
        {t("lge.artifacts.archDiff.apiSurfaceEmpty")}
      </p>
    );
  }

  return (
    <div className="space-y-3">
      {files.map((file) => (
        <FileSection key={file} file={file} changes={byFile[file]} />
      ))}
    </div>
  );
}

function FileSection({
  file,
  changes,
}: {
  file: string;
  changes: ApiChange[];
}) {
  const [collapsed, setCollapsed] = useState(false);
  const fileName = file.split("/").pop() ?? file;
  const dirPath = file.includes("/") ? file.substring(0, file.lastIndexOf("/")) : "";

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      {/* Header */}
      <button
        onClick={() => setCollapsed(!collapsed)}
        className="flex w-full items-center gap-2 bg-bg-card px-3 py-2 text-left hover:bg-bg-hover transition-colors"
      >
        <span className={`text-xs text-text-muted transition-transform duration-150 ${collapsed ? "" : "rotate-90"}`}>
          ▶
        </span>
        <span className="text-xs text-text-muted">{dirPath}/</span>
        <span className="text-xs font-medium text-text-primary">{fileName}</span>
        <span className="ml-auto text-xs text-text-muted">{changes.length} change(s)</span>
      </button>

      {/* Rows */}
      {!collapsed && (
        <div className="divide-y divide-border">
          {changes.map((change, i) => (
            <div
              key={i}
              className={`flex items-start gap-3 px-3 py-2 border-l-2 ${
                CHANGE_COLORS[change.change_type as keyof typeof CHANGE_COLORS] ?? ""
              }`}
            >
              <Badge
                text={change.change_type}
                className={CHANGE_BADGE[change.change_type as keyof typeof CHANGE_BADGE] ?? "bg-bg-hover text-text-muted"}
              />
              <Badge
                text={change.kind}
                className={KIND_BADGE[change.kind] ?? KIND_BADGE.unknown}
              />
              <div className="flex-1 min-w-0">
                <span className="text-xs font-mono text-text-primary">{change.symbol}</span>
                {change.signature && (
                  <p className="mt-0.5 text-xs text-text-muted truncate font-mono">
                    {change.signature}
                  </p>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
