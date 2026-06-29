use rusqlite::Connection;

pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

    if version < 1 {
        conn.execute_batch(include_str!("../../migrations/001_initial.sql"))?;
        conn.execute_batch("PRAGMA user_version = 1")?;
    }
    if version < 2 {
        conn.execute_batch(include_str!("../../migrations/002_git_branch.sql"))?;
        conn.execute_batch("PRAGMA user_version = 2")?;
    }
    if version < 3 {
        conn.execute_batch(include_str!("../../migrations/003_settings.sql"))?;
        conn.execute_batch("PRAGMA user_version = 3")?;
    }
    if version < 4 {
        conn.execute_batch(include_str!("../../migrations/004_worktrees.sql"))?;
        conn.execute_batch("PRAGMA user_version = 4")?;
    }
    if version < 5 {
        conn.execute_batch(include_str!("../../migrations/005_shell_env.sql"))?;
        conn.execute_batch("PRAGMA user_version = 5")?;
    }
    if version < 6 {
        conn.execute_batch(include_str!("../../migrations/006_task_attachments.sql"))?;
        conn.execute_batch("PRAGMA user_version = 6")?;
    }
    if version < 7 {
        conn.execute_batch(include_str!("../../migrations/007_attachment_phases_multi.sql"))?;
        conn.execute_batch("PRAGMA user_version = 7")?;
    }
    if version < 8 {
        conn.execute_batch(include_str!("../../migrations/008_jira_base_url.sql"))?;
        conn.execute_batch("PRAGMA user_version = 8")?;
    }

    Ok(())
}
