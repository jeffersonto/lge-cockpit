import { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  CURRENT_VERSION,
  RELEASE_NOTES,
  type ReleaseFeatureIcon,
} from "../../data/releaseNotes";

interface WhatsNewDialogProps {
  open: boolean;
  onClose: () => void;
}

const COLOR_CLASSES = {
  accent: {
    dot: "bg-accent",
    text: "text-accent",
    badge: "bg-accent/10 text-accent border border-accent/30",
  },
  success: {
    dot: "bg-success",
    text: "text-success",
    badge: "bg-success/10 text-success border border-success/30",
  },
  warning: {
    dot: "bg-warning",
    text: "text-warning",
    badge: "bg-warning/10 text-warning border border-warning/30",
  },
} as const;

const ICON_PATHS: Record<ReleaseFeatureIcon, React.ReactNode> = {
  "folder-tree": (
    <>
      <path d="M20 10a1 1 0 0 0 1-1V6a1 1 0 0 0-1-1h-2.5a1 1 0 0 1-.8-.4l-1.9-2.2a1 1 0 0 0-.8-.4H9a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1z" />
      <path d="M20 21a1 1 0 0 0 1-1v-3a1 1 0 0 0-1-1h-2.9a1 1 0 0 1-.88-.55l-.42-.85a1 1 0 0 0-.88-.55H9a1 1 0 0 0-1 1v4a1 1 0 0 0 1 1z" />
      <path d="M3 5a2 2 0 0 0-2 2v3a2 2 0 0 0 2 2h4" />
      <path d="M3 16a2 2 0 0 0-2 2v1a2 2 0 0 0 2 2h4" />
    </>
  ),
  ide: (
    <>
      <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
      <polyline points="15 3 21 3 21 9" />
      <line x1="10" x2="21" y1="14" y2="3" />
    </>
  ),
  trash: (
    <>
      <path d="M3 6h18" />
      <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
      <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
      <line x1="10" x2="10" y1="11" y2="17" />
      <line x1="14" x2="14" y1="11" y2="17" />
    </>
  ),
  badge: (
    <>
      <path d="M3.85 8.62a4 4 0 0 1 4.78-4.77 4 4 0 0 1 6.74 0 4 4 0 0 1 4.78 4.78 4 4 0 0 1 0 6.74 4 4 0 0 1-4.77 4.78 4 4 0 0 1-6.75 0 4 4 0 0 1-4.78-4.77 4 4 0 0 1 0-6.76Z" />
    </>
  ),
  limit: (
    <>
      <circle cx="12" cy="12" r="10" />
      <line x1="4.93" x2="19.07" y1="4.93" y2="19.07" />
    </>
  ),
  health: (
    <>
      <path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z" />
      <path d="M3.22 12H9.5l.5-1 2 4.5 2-7 1.5 3.5h5.27" />
    </>
  ),
  "git-branch": (
    <>
      <line x1="6" x2="6" y1="3" y2="15" />
      <circle cx="18" cy="6" r="3" />
      <circle cx="6" cy="18" r="3" />
      <path d="M18 9a9 9 0 0 1-9 9" />
    </>
  ),
  commit: (
    <>
      <circle cx="12" cy="12" r="3" />
      <line x1="3" x2="9" y1="12" y2="12" />
      <line x1="15" x2="21" y1="12" y2="12" />
    </>
  ),
  "pull-request": (
    <>
      <circle cx="18" cy="18" r="3" />
      <circle cx="6" cy="6" r="3" />
      <path d="M13 6h3a2 2 0 0 1 2 2v7" />
      <line x1="6" x2="6" y1="9" y2="21" />
    </>
  ),
  tracking: (
    <>
      <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
      <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
    </>
  ),
  repo: (
    <>
      <ellipse cx="12" cy="5" rx="9" ry="3" />
      <path d="M3 5v14a9 3 0 0 0 18 0V5" />
      <path d="M3 12a9 3 0 0 0 18 0" />
    </>
  ),
  task: (
    <>
      <path d="M11 11h6" />
      <path d="M11 15h6" />
      <path d="M11 19h6" />
      <rect width="18" height="18" x="3" y="3" rx="2" />
      <path d="m7 7 .5.5L9 6" />
      <path d="m7 11 .5.5L9 10" />
      <path d="m7 15 .5.5L9 14" />
    </>
  ),
  jira: (
    <>
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="7 10 12 15 17 10" />
      <line x1="12" x2="12" y1="15" y2="3" />
    </>
  ),
  search: (
    <>
      <circle cx="11" cy="11" r="8" />
      <path d="m21 21-4.3-4.3" />
    </>
  ),
  edit: (
    <>
      <path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
      <path d="m15 5 4 4" />
    </>
  ),
  pipeline: (
    <>
      <rect width="8" height="4" x="3" y="3" rx="1" />
      <rect width="8" height="4" x="13" y="3" rx="1" />
      <rect width="8" height="4" x="3" y="17" rx="1" />
      <rect width="8" height="4" x="13" y="17" rx="1" />
      <path d="M7 7v3a3 3 0 0 0 3 3h4a3 3 0 0 0 3-3V7" />
      <path d="M7 17v-3a3 3 0 0 1 3-3h4a3 3 0 0 1 3 3v3" />
    </>
  ),
  context: (
    <>
      <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
      <line x1="12" x2="12" y1="7" y2="13" />
      <line x1="9" x2="15" y1="10" y2="10" />
    </>
  ),
  artifact: (
    <>
      <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
      <polyline points="14 2 14 8 20 8" />
      <line x1="16" x2="8" y1="13" y2="13" />
      <line x1="16" x2="8" y1="17" y2="17" />
      <line x1="10" x2="8" y1="9" y2="9" />
    </>
  ),
  cancel: (
    <>
      <circle cx="12" cy="12" r="10" />
      <path d="m15 9-6 6" />
      <path d="m9 9 6 6" />
    </>
  ),
  notification: (
    <>
      <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
      <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
    </>
  ),
  i18n: (
    <>
      <circle cx="12" cy="12" r="10" />
      <path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20" />
      <path d="M2 12h20" />
    </>
  ),
  settings: (
    <>
      <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
      <circle cx="12" cy="12" r="3" />
    </>
  ),
};

function FeatureIcon({ name }: { name: ReleaseFeatureIcon }) {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="mt-0.5 shrink-0 text-text-muted"
    >
      {ICON_PATHS[name]}
    </svg>
  );
}

export function WhatsNewDialog({ open, onClose }: WhatsNewDialogProps) {
  const { t } = useTranslation();
  const overlayRef = useRef<HTMLDivElement>(null);
  const [expandedVersions, setExpandedVersions] = useState<Set<string>>(
    () => new Set([CURRENT_VERSION])
  );

  useEffect(() => {
    if (open) {
      setExpandedVersions(new Set([CURRENT_VERSION]));
    }
  }, [open]);

  useEffect(() => {
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    if (open) {
      document.addEventListener("keydown", handleEsc);
      return () => document.removeEventListener("keydown", handleEsc);
    }
  }, [open, onClose]);

  if (!open) return null;

  const toggleVersion = (version: string) => {
    setExpandedVersions((prev) => {
      const next = new Set(prev);
      if (next.has(version)) {
        next.delete(version);
      } else {
        next.add(version);
      }
      return next;
    });
  };

  return (
    <div
      ref={overlayRef}
      className="animate-fadeIn fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={(e) => {
        if (e.target === overlayRef.current) onClose();
      }}
    >
      <div className="animate-slideUp flex w-full max-w-2xl flex-col rounded-xl border border-border bg-bg-surface shadow-2xl"
        style={{ maxHeight: "80vh" }}
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border px-6 py-4">
          <div>
            <h2 className="text-lg font-semibold text-text-primary">
              {t("whatsNew.title")}
            </h2>
          </div>
          <div className="flex items-center gap-3">
            <span className="rounded-full bg-accent/10 px-2.5 py-0.5 text-xs font-medium text-accent">
              v{CURRENT_VERSION}
            </span>
            <button
              onClick={onClose}
              className="rounded p-1 text-text-muted transition-colors hover:bg-bg-hover hover:text-text-secondary"
            >
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M18 6 6 18" />
                <path d="m6 6 12 12" />
              </svg>
            </button>
          </div>
        </div>

        {/* Scrollable body */}
        <div className="flex-1 select-text overflow-y-auto px-6 py-6">
          <div className="relative pl-10">
            {/* Vertical timeline line */}
            <div className="absolute bottom-0 left-[7px] top-0 w-px bg-border" />

            {RELEASE_NOTES.map((release, index) => {
              const colors = COLOR_CLASSES[release.color];
              const isExpanded = expandedVersions.has(release.version);
              const isLatest = index === 0;

              return (
                <div key={release.version} className={`relative ${index < RELEASE_NOTES.length - 1 ? "mb-6" : ""}`}>
                  {/* Timeline dot — centered on the timeline line at left-[7px] of the pl-10 container */}
                  <div
                    className={`absolute -left-[33px] top-0.5 h-3.5 w-3.5 rounded-full ${colors.dot} ring-4 ring-bg-surface`}
                  />

                  {/* Version header — clickable */}
                  <button
                    onClick={() => toggleVersion(release.version)}
                    className="flex w-full items-center gap-2 text-left"
                  >
                    <span className="font-semibold text-text-primary">
                      {t(release.titleKey)}
                    </span>
                    {isLatest && (
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] font-semibold uppercase ${colors.badge}`}
                      >
                        {t("whatsNew.latest")}
                      </span>
                    )}
                    <svg
                      width="14"
                      height="14"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      className={`ml-auto text-text-muted transition-transform duration-200 ${
                        isExpanded ? "rotate-180" : ""
                      }`}
                    >
                      <path d="m6 9 6 6 6-6" />
                    </svg>
                  </button>

                  {/* Collapsible feature list */}
                  <div
                    className={`overflow-hidden transition-all duration-300 ease-in-out ${
                      isExpanded ? "mt-3 max-h-[500px] opacity-100" : "max-h-0 opacity-0"
                    }`}
                  >
                    <ul className="space-y-2">
                      {release.features.map((feature, fi) => (
                        <li
                          key={fi}
                          className="flex items-start gap-2 text-sm text-text-secondary"
                        >
                          <FeatureIcon name={feature.icon} />
                          <span>{t(feature.labelKey)}</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-end border-t border-border px-6 py-3">
          <button
            onClick={onClose}
            className="rounded-lg bg-bg-card px-4 py-1.5 text-sm text-text-secondary transition-colors hover:bg-bg-hover"
          >
            {t("whatsNew.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
