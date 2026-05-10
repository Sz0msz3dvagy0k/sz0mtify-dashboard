ALTER TABLE discovered_releases ADD COLUMN local_artist_id INTEGER;
ALTER TABLE discovered_releases ADD COLUMN local_artist_name TEXT;
ALTER TABLE discovered_releases ADD COLUMN discovered_artist_name TEXT;
ALTER TABLE discovered_releases ADD COLUMN match_status TEXT DEFAULT 'uncertain';
ALTER TABLE discovered_releases ADD COLUMN reason TEXT;
ALTER TABLE discovered_releases ADD COLUMN source_artist_name TEXT;
ALTER TABLE discovered_releases ADD COLUMN source_artist_id INTEGER;
ALTER TABLE discovered_releases ADD COLUMN raw_json TEXT;
ALTER TABLE discovered_releases ADD COLUMN normalized_discovered_artist_name TEXT;
ALTER TABLE discovered_releases ADD COLUMN normalized_title TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_discovered_unique_item
ON discovered_releases(source, release_type, normalized_discovered_artist_name, normalized_title);

CREATE INDEX IF NOT EXISTS idx_discovered_match_status ON discovered_releases(match_status);
CREATE INDEX IF NOT EXISTS idx_discovered_release_type ON discovered_releases(release_type);
