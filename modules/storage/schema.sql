BEGIN IMMEDIATE;
CREATE TABLE settings (
  key TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL
);
CREATE TABLE history (
  id TEXT PRIMARY KEY NOT NULL,
  kind TEXT NOT NULL CHECK(kind IN ('text', 'image')),
  preview TEXT NOT NULL,
  search_text TEXT NOT NULL,
  payload BLOB NOT NULL,
  source_app TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  pinned INTEGER NOT NULL DEFAULT 0 CHECK(pinned IN (0, 1)),
  byte_size INTEGER NOT NULL,
  width INTEGER,
  height INTEGER
);
CREATE INDEX history_order ON history(pinned DESC, updated_at DESC, id ASC);
PRAGMA user_version = 1;
COMMIT;
