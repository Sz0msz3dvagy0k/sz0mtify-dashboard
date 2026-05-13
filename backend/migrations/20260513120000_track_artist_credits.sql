ALTER TABLE tracks ADD COLUMN raw_artist TEXT;

CREATE TABLE IF NOT EXISTS track_artists (
  track_id INTEGER NOT NULL,
  artist_id INTEGER NOT NULL,
  role TEXT NOT NULL,
  position INTEGER NOT NULL,
  raw_artist TEXT,
  musicbrainz_artist_mbid TEXT,
  source TEXT NOT NULL DEFAULT 'local_parser',
  created_at TEXT DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (track_id, artist_id, role),
  FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
  FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE
);

INSERT OR IGNORE INTO track_artists(track_id, artist_id, role, position, raw_artist, source)
SELECT t.id, t.artist_id, 'primary', 0, ar.name, 'legacy'
FROM tracks t
JOIN artists ar ON ar.id = t.artist_id
WHERE t.artist_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_track_artists_artist_id ON track_artists(artist_id);
CREATE INDEX IF NOT EXISTS idx_track_artists_track_id ON track_artists(track_id);
CREATE INDEX IF NOT EXISTS idx_track_artists_role ON track_artists(role);
