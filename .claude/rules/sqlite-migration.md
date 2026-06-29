---
globs: ["src-tauri/migrations/*.sql", "src-tauri/src/db/schema.rs", "src-tauri/src/db/queries.rs", "src-tauri/src/models/*.rs"]
---

# SQLite migrations

Migrations are applied at startup by `db::schema::run_migrations` using `PRAGMA user_version`. A new `.sql` file with no matching block in `schema.rs` is silently skipped — production databases will run on stale schemas.

## File naming

`src-tauri/migrations/NNN_snake_case.sql` — three-digit zero-padded prefix, monotonically incrementing. Existing range: `001_initial.sql` … `007_attachment_phases_multi.sql`.

## Required edits when adding a migration

1. Create `src-tauri/migrations/00N_what_changed.sql` with the new prefix.
2. Add a matching block in `src-tauri/src/db/schema.rs`:

   ```rust
   if version < N {
       conn.execute_batch(include_str!("../../migrations/00N_what_changed.sql"))?;
       conn.execute_batch("PRAGMA user_version = N")?;
   }
   ```

   The `version < N` check, the `include_str!` filename, and the `user_version = N` MUST all use the same `N`. A mismatch silently skips the migration on existing installs.

3. Update the affected `src-tauri/src/models/*.rs` struct (add fields, derive `Serialize`/`Deserialize`).
4. Update or add functions in `src-tauri/src/db/queries.rs`. Always use `params![]` or `?` placeholders — never string-format SQL with user input.

## Migration content rules

- Wrap multi-statement migrations to be idempotent where possible (`CREATE TABLE IF NOT EXISTS`, `ALTER TABLE ... ADD COLUMN` only when the column is new).
- Do not edit a previously committed migration — write a new one. Released installs already passed the old `user_version`.
- Foreign keys are enabled (`PRAGMA foreign_keys=ON`); CASCADE deletes are expected on `repository_id` and `task_id`.

## Reminder for CLAUDE.md

Whenever a new migration is added, the `## Database` section of root `CLAUDE.md` must list the new file in the migrations array. Reviewers grep this list to know what changed.
