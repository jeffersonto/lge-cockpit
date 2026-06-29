ALTER TABLE tasks ADD COLUMN worktree_path TEXT;
ALTER TABLE repositories ADD COLUMN max_worktrees INTEGER NOT NULL DEFAULT 5;
CREATE INDEX IF NOT EXISTS idx_tasks_worktree ON tasks(repository_id) WHERE worktree_path IS NOT NULL;
