CREATE TABLE IF NOT EXISTS navidrome_scan_state (
  source TEXT PRIMARY KEY,
  last_scan TEXT,
  observed_last_scan TEXT,
  last_checked_at TEXT,
  scanning INTEGER DEFAULT 0,
  count INTEGER DEFAULT 0,
  folder_count INTEGER DEFAULT 0,
  scan_type TEXT,
  elapsed_time INTEGER
);
