CREATE TABLE IF NOT EXISTS artists (
  id INTEGER PRIMARY KEY,
  subsonic_id TEXT UNIQUE,
  name TEXT NOT NULL,
  album_count INTEGER DEFAULT 0,
  track_count INTEGER DEFAULT 0,
  image_url TEXT,
  bio_summary TEXT,
  tags TEXT,
  similar_artists_json TEXT,
  play_count INTEGER DEFAULT 0,
  last_played_at TEXT,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
  lastfm_artist_url TEXT,
  lastfm_listeners INTEGER,
  lastfm_playcount INTEGER,
  mbid TEXT
);
CREATE TABLE IF NOT EXISTS albums (id INTEGER PRIMARY KEY, subsonic_id TEXT UNIQUE, title TEXT NOT NULL, artist_id INTEGER, album_artist_id INTEGER, year INTEGER, genre TEXT, song_count INTEGER DEFAULT 0, duration_seconds INTEGER DEFAULT 0, size_bytes INTEGER DEFAULT 0, cover_art_id TEXT, cover_art_url TEXT, play_count INTEGER DEFAULT 0, rating INTEGER, starred INTEGER DEFAULT 0, created_at TEXT DEFAULT CURRENT_TIMESTAMP, updated_at TEXT DEFAULT CURRENT_TIMESTAMP, lastfm_album_url TEXT, lastfm_listeners INTEGER, lastfm_playcount INTEGER, mbid TEXT);
CREATE TABLE IF NOT EXISTS tracks (id INTEGER PRIMARY KEY, subsonic_id TEXT UNIQUE, title TEXT NOT NULL, artist_id INTEGER, album_id INTEGER, album_artist_id INTEGER, duration_seconds INTEGER DEFAULT 0, track_number INTEGER, disc_number INTEGER, year INTEGER, genre TEXT, mood TEXT, file_path TEXT, suffix TEXT, content_type TEXT, size_bytes INTEGER DEFAULT 0, bit_rate INTEGER, bit_depth INTEGER, sampling_rate INTEGER, channel_count INTEGER, play_count INTEGER DEFAULT 0, skip_count INTEGER DEFAULT 0, rating INTEGER, starred INTEGER DEFAULT 0, last_played_at TEXT, created_at TEXT DEFAULT CURRENT_TIMESTAMP, updated_at TEXT DEFAULT CURRENT_TIMESTAMP, lastfm_track_url TEXT, lastfm_listeners INTEGER, lastfm_playcount INTEGER, mbid TEXT);
CREATE TABLE IF NOT EXISTS genres (id INTEGER PRIMARY KEY, name TEXT UNIQUE, track_count INTEGER DEFAULT 0, album_count INTEGER DEFAULT 0, artist_count INTEGER DEFAULT 0);
CREATE TABLE IF NOT EXISTS plays (id INTEGER PRIMARY KEY, track_id INTEGER, played_at TEXT, source TEXT, scrobble_id TEXT);
CREATE TABLE IF NOT EXISTS api_cache (id INTEGER PRIMARY KEY, provider TEXT, cache_key TEXT UNIQUE, response_json TEXT, expires_at TEXT, created_at TEXT DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS discovered_releases (id INTEGER PRIMARY KEY, artist_id INTEGER, title TEXT, release_type TEXT, release_date TEXT, source TEXT, external_url TEXT, cover_url TEXT, already_in_library INTEGER DEFAULT 0, confidence_score REAL DEFAULT 0, created_at TEXT DEFAULT CURRENT_TIMESTAMP, updated_at TEXT DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS sync_state (id INTEGER PRIMARY KEY, source TEXT UNIQUE, last_sync_at TEXT, status TEXT, error_message TEXT);
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS idx_tracks_artist_id ON tracks(artist_id);
CREATE INDEX IF NOT EXISTS idx_tracks_album_id ON tracks(album_id);
CREATE INDEX IF NOT EXISTS idx_tracks_genre ON tracks(genre);
CREATE INDEX IF NOT EXISTS idx_tracks_year ON tracks(year);
CREATE INDEX IF NOT EXISTS idx_tracks_last_played_at ON tracks(last_played_at);
CREATE INDEX IF NOT EXISTS idx_albums_artist_id ON albums(artist_id);
CREATE INDEX IF NOT EXISTS idx_albums_year ON albums(year);
CREATE INDEX IF NOT EXISTS idx_artists_name ON artists(name);
CREATE INDEX IF NOT EXISTS idx_plays_track_id ON plays(track_id);
CREATE INDEX IF NOT EXISTS idx_plays_played_at ON plays(played_at);
CREATE INDEX IF NOT EXISTS idx_discovered_artist_id ON discovered_releases(artist_id);
CREATE INDEX IF NOT EXISTS idx_discovered_release_date ON discovered_releases(release_date);
