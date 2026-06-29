CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

INSERT OR IGNORE INTO settings (key, value) VALUES ('model_planning', 'opus');
INSERT OR IGNORE INTO settings (key, value) VALUES ('model_builder', 'haiku');
INSERT OR IGNORE INTO settings (key, value) VALUES ('model_review', 'sonnet');
INSERT OR IGNORE INTO settings (key, value) VALUES ('model_guardian', 'opus');
