use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::Manager;
use tokio::sync::Semaphore;

mod claude_invocation;
mod commands;
mod db;
mod models;
mod phase_runner;

pub struct AppState {
    pub db: Mutex<Connection>,
    /// Track running LGE phase PIDs: key = "taskId:phase"
    pub running_pids: Mutex<HashMap<String, u32>>,
    /// Ensures only one planning phase runs at a time (queue serialization)
    pub planning_semaphore: Arc<Semaphore>,
    /// Task IDs whose planning was cancelled while waiting in the queue
    pub planning_cancelled: Mutex<HashSet<String>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Initialize SQLite database
            let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
            let db_path = app_data_dir.join("lge-cockpit.db");

            let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
            db::schema::run_migrations(&conn).map_err(|e| e.to_string())?;

            app.manage(AppState {
                db: Mutex::new(conn),
                running_pids: Mutex::new(HashMap::new()),
                planning_semaphore: Arc::new(Semaphore::new(1)),
                planning_cancelled: Mutex::new(HashSet::new()),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::repositories::add_repository,
            commands::repositories::list_repositories,
            commands::repositories::remove_repository,
            commands::repositories::get_project_delete_preview,
            commands::tasks::list_tasks,
            commands::tasks::create_task,
            commands::tasks::update_task_status,
            commands::tasks::update_task,
            commands::tasks::delete_task,
            commands::jira::import_jira_task,
            commands::jira::run_jira_diagnostic,
            commands::lge::run_lge_phase,
            commands::lge::load_lge_artifacts,
            commands::health::check_dependencies,
            commands::lge::cancel_lge_phase,
            commands::lge::save_lge_artifact,
            commands::git::get_current_git_branch,
            commands::git::get_head_commit,
            commands::git::create_git_branch,
            commands::git::commit_and_push,
            commands::git::create_pull_request,
            commands::git::generate_commit_message,
            commands::git::remove_worktree,
            commands::git::remove_completed_worktrees,
            commands::git::check_stale_worktrees,
            commands::git::open_in_editor,
            commands::arch_diff::analyze_architecture_diff,
            commands::arch_diff::analyze_working_tree_diff,
            commands::settings::get_phase_models,
            commands::settings::save_phase_models,
            commands::settings::get_shell_env,
            commands::settings::save_shell_env,
            commands::settings::get_jira_base_url,
            commands::settings::save_jira_base_url,
            commands::attachments::add_task_attachment,
            commands::attachments::list_task_attachments,
            commands::attachments::remove_task_attachment,
            commands::attachments::set_attachment_phases,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
