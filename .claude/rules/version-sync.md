---
globs: ["package.json", "src-tauri/tauri.conf.json", "src-tauri/Cargo.toml", "src/data/releaseNotes.ts"]
---

# Version bump synchronization

Four files must agree on the app version. The "What's New" dialog matches the StatusBar version against `releaseNotes.ts` entries — a stale or missing entry means users never see the dialog.

## Files to update together

1. `package.json` → `"version": "X.Y.Z"`
2. `src-tauri/tauri.conf.json` → `"version": "X.Y.Z"`
3. `src-tauri/Cargo.toml` → `version = "X.Y.Z"` under `[package]`
4. `src/data/releaseNotes.ts` → prepend a new entry with matching `version: "X.Y.Z"`, a `date`, and bilingual title/items where applicable

## When to bump

Only on user-visible changes that should appear in the timeline: new features, major bug fixes, UI overhauls. Refactors, doc-only changes, and dependency bumps do not get a version bump.

## Verification

Before committing:

```bash
grep -E '"version"|^version' package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
grep -m1 "version:" src/data/releaseNotes.ts
```

All four must print the same `X.Y.Z`. A mismatch at runtime causes the StatusBar `NEW` indicator to either never show or show stale content.

## Don't

- Bump `package.json` and `tauri.conf.json` but forget `Cargo.toml` — historically the most common mistake. The Rust binary keeps reporting the old version in `cargo` output.
- Rewrite an existing `releaseNotes.ts` entry for a published version. Append a new one.
- Use a non-semver string. `tauri.conf.json` validates against the schema.
