// Keep CURRENT_VERSION in sync with package.json, tauri.conf.json, and Cargo.toml
export const CURRENT_VERSION = "0.9.0";
export const WHATS_NEW_STORAGE_KEY = "lge_cockpit_seen_version";

export type ReleaseFeatureIcon =
  | "folder-tree"
  | "ide"
  | "trash"
  | "badge"
  | "limit"
  | "health"
  | "git-branch"
  | "commit"
  | "pull-request"
  | "tracking"
  | "repo"
  | "task"
  | "jira"
  | "search"
  | "edit"
  | "pipeline"
  | "context"
  | "artifact"
  | "cancel"
  | "notification"
  | "i18n"
  | "settings";

export interface ReleaseFeature {
  icon: ReleaseFeatureIcon;
  labelKey: string;
}

export interface ReleaseNote {
  version: string;
  titleKey: string;
  color: "accent" | "success" | "warning";
  features: ReleaseFeature[];
}

export const RELEASE_NOTES: ReleaseNote[] = [
  {
    version: "0.9.0",
    titleKey: "whatsNew.releases.v090.title",
    color: "accent",
    features: [
      { icon: "i18n", labelKey: "whatsNew.releases.v090.f1" },
      { icon: "artifact", labelKey: "whatsNew.releases.v090.f2" },
      { icon: "settings", labelKey: "whatsNew.releases.v090.f3" },
    ],
  },
  {
    version: "0.8.0",
    titleKey: "whatsNew.releases.v080.title",
    color: "accent",
    features: [
      { icon: "jira", labelKey: "whatsNew.releases.v080.f1" },
      { icon: "settings", labelKey: "whatsNew.releases.v080.f2" },
      { icon: "health", labelKey: "whatsNew.releases.v080.f3" },
    ],
  },
  {
    version: "0.7.1",
    titleKey: "whatsNew.releases.v071.title",
    color: "accent",
    features: [
      { icon: "settings", labelKey: "whatsNew.releases.v071.f1" },
      { icon: "jira", labelKey: "whatsNew.releases.v071.f2" },
    ],
  },
  {
    version: "0.7.0",
    titleKey: "whatsNew.releases.v070.title",
    color: "accent",
    features: [
      { icon: "trash", labelKey: "whatsNew.releases.v070.f1" },
      { icon: "git-branch", labelKey: "whatsNew.releases.v070.f2" },
      { icon: "folder-tree", labelKey: "whatsNew.releases.v070.f3" },
      { icon: "task", labelKey: "whatsNew.releases.v070.f4" },
      { icon: "cancel", labelKey: "whatsNew.releases.v070.f5" },
    ],
  },
  {
    version: "0.6.0",
    titleKey: "whatsNew.releases.v060.title",
    color: "accent",
    features: [
      { icon: "artifact", labelKey: "whatsNew.releases.v060.f1" },
      { icon: "context", labelKey: "whatsNew.releases.v060.f2" },
      { icon: "pipeline", labelKey: "whatsNew.releases.v060.f3" },
      { icon: "health", labelKey: "whatsNew.releases.v060.f4" },
      { icon: "jira", labelKey: "whatsNew.releases.v060.f5" },
      { icon: "settings", labelKey: "whatsNew.releases.v060.f6" },
      { icon: "jira", labelKey: "whatsNew.releases.v060.f7" },
      { icon: "task", labelKey: "whatsNew.releases.v060.f8" },
    ],
  },
  {
    version: "0.5.0",
    titleKey: "whatsNew.releases.v050.title",
    color: "accent",
    features: [
      { icon: "settings", labelKey: "whatsNew.releases.v050.f1" },
      { icon: "folder-tree", labelKey: "whatsNew.releases.v050.f2" },
      { icon: "pull-request", labelKey: "whatsNew.releases.v050.f3" },
      { icon: "artifact", labelKey: "whatsNew.releases.v050.f4" },
      { icon: "git-branch", labelKey: "whatsNew.releases.v050.f5" },
    ],
  },
  {
    version: "0.4.0",
    titleKey: "whatsNew.releases.v040.title",
    color: "accent",
    features: [
      { icon: "artifact", labelKey: "whatsNew.releases.v040.f1" },
      { icon: "pipeline", labelKey: "whatsNew.releases.v040.f2" },
      { icon: "settings", labelKey: "whatsNew.releases.v040.f3" },
    ],
  },
  {
    version: "0.3.0",
    titleKey: "whatsNew.releases.v030.title",
    color: "accent",
    features: [
      { icon: "folder-tree", labelKey: "whatsNew.releases.v030.f1" },
      { icon: "ide", labelKey: "whatsNew.releases.v030.f2" },
      { icon: "trash", labelKey: "whatsNew.releases.v030.f3" },
      { icon: "badge", labelKey: "whatsNew.releases.v030.f4" },
      { icon: "limit", labelKey: "whatsNew.releases.v030.f5" },
      { icon: "health", labelKey: "whatsNew.releases.v030.f6" },
      { icon: "settings", labelKey: "whatsNew.releases.v030.f7" },
    ],
  },
  {
    version: "0.2.0",
    titleKey: "whatsNew.releases.v020.title",
    color: "success",
    features: [
      { icon: "git-branch", labelKey: "whatsNew.releases.v020.f1" },
      { icon: "commit", labelKey: "whatsNew.releases.v020.f2" },
      { icon: "pull-request", labelKey: "whatsNew.releases.v020.f3" },
      { icon: "tracking", labelKey: "whatsNew.releases.v020.f4" },
    ],
  },
  {
    version: "0.1.0",
    titleKey: "whatsNew.releases.v010.title",
    color: "warning",
    features: [
      { icon: "repo", labelKey: "whatsNew.releases.v010.f1" },
      { icon: "task", labelKey: "whatsNew.releases.v010.f2" },
      { icon: "jira", labelKey: "whatsNew.releases.v010.f3" },
      { icon: "search", labelKey: "whatsNew.releases.v010.f4" },
      { icon: "edit", labelKey: "whatsNew.releases.v010.f5" },
      { icon: "pipeline", labelKey: "whatsNew.releases.v010.f6" },
      { icon: "context", labelKey: "whatsNew.releases.v010.f7" },
      { icon: "artifact", labelKey: "whatsNew.releases.v010.f8" },
      { icon: "cancel", labelKey: "whatsNew.releases.v010.f9" },
      { icon: "health", labelKey: "whatsNew.releases.v010.f10" },
      { icon: "notification", labelKey: "whatsNew.releases.v010.f11" },
      { icon: "i18n", labelKey: "whatsNew.releases.v010.f12" },
    ],
  },
];
