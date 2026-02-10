-- SQLite schema for SMS Tagging Officer
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS messages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  content TEXT NOT NULL,
  received_at TEXT NULL,
  sender TEXT NULL,
  phone TEXT NULL,
  source TEXT NULL,
  has_url INTEGER NOT NULL DEFAULT 0,
  has_amount INTEGER NOT NULL DEFAULT 0,
  has_verification_code INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_messages_has_url ON messages(has_url);
CREATE INDEX IF NOT EXISTS idx_messages_has_amount ON messages(has_amount);
CREATE INDEX IF NOT EXISTS idx_messages_has_verification_code ON messages(has_verification_code);
CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender);
CREATE INDEX IF NOT EXISTS idx_messages_source ON messages(source);

CREATE TABLE IF NOT EXISTS labels (
  message_id INTEGER PRIMARY KEY,
  industry TEXT NOT NULL,
  sms_type TEXT NOT NULL,
  confidence REAL NOT NULL,
  needs_review INTEGER NOT NULL,
  reasons_json TEXT NOT NULL,
  signals_json TEXT NOT NULL,
  rules_version TEXT NOT NULL,
  model_version TEXT NOT NULL,
  schema_version TEXT NOT NULL,
  entities_json TEXT NOT NULL,
  updated_by TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  is_manual INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY(message_id) REFERENCES messages(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_labels_industry ON labels(industry);
CREATE INDEX IF NOT EXISTS idx_labels_sms_type ON labels(sms_type);
CREATE INDEX IF NOT EXISTS idx_labels_needs_review ON labels(needs_review);
CREATE INDEX IF NOT EXISTS idx_labels_confidence ON labels(confidence);

CREATE TABLE IF NOT EXISTS audit_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  message_id INTEGER NOT NULL,
  operator TEXT NOT NULL,
  at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  before_json TEXT NULL,
  after_json TEXT NOT NULL,
  diff_json TEXT NOT NULL,
  FOREIGN KEY(message_id) REFERENCES messages(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_message_id ON audit_logs(message_id);
