# Music Listening Dashboard (Self-hosted MVP)

Monorepo with:
- `backend/`: Rust (Axum + SQLx + SQLite)
- `frontend/`: SvelteKit + TypeScript monochrome dashboard
- `docker-compose.yml`: local development stack

## Quick start
1. Copy env file:
   ```bash
   cp .env.example .env
   ```
2. Start:
   ```bash
   docker compose up --build
   ```
3. Open frontend: `http://localhost:5173`
4. Health check: `http://localhost:8080/api/health`

## Backend local dev
```bash
cd backend
cargo fmt
cargo clippy -- -D warnings
cargo run
```

## Frontend local dev
```bash
cd frontend
npm install
npm run dev
npm run check
npm run lint
npm run format
```

## iOS local bundle
The frontend is built as a static SvelteKit SPA and wrapped by Capacitor for iOS.

1. Set the public frontend origin for backend CORS when deploying:
   ```bash
   FRONTEND_BASE_URL=https://kaori.szomszed.me
   CAPACITOR=true
   ```
2. Build and sync the native project:
   ```bash
   cd frontend
   npm run build:mobile
   npm run cap:open:ios
   ```

The frontend API base URL is entered on the login screen. The app validates the URL, checks `/api/health`, and stores it locally only for the lifetime of the saved session.

The audio player uses short-lived stream tokens from `POST /api/auth/stream-token` because native media elements cannot attach custom `Authorization` headers to stream URLs.

## Backend behavior

The backend syncs library metadata from a Subsonic/Navidrome server and enriches artists through Last.fm. Configure:

- `SUBSONIC_BASE_URL`
- `SUBSONIC_USERNAME`
- `SUBSONIC_PASSWORD`
- `SUBSONIC_API_VERSION` optional, defaults to `1.16.1`
- `LASTFM_API_KEY`

The app has a single-user login. Configure either `APP_PASSWORD` for a plain password or `APP_PASSWORD_SHA256` for a SHA-256 password hash. `APP_SESSION_TTL_HOURS` controls normal login sessions, and `APP_STREAM_TOKEN_TTL_SECONDS` controls short-lived audio stream URL tokens.

### Storage stats

`GET /api/stats/storage` treats track file size as the source of truth. `total_storage_bytes` and `tracks_size_bytes` both equal `SUM(tracks.size_bytes)`. Album, artist, genre, format, content-type, and extension storage values are aggregations over tracks, so album totals are not added again to total storage.

### Listening stats

`GET /api/stats/listening` prefers timestamped rows in `plays`. If `plays` is empty, it falls back to imported Subsonic/Navidrome `tracks.play_count`. If neither source has data, top lists are empty and `data_source` is `"none"`. Timeline data is only generated from timestamped play events.

### Cover art

`GET /api/cover/:cover_art_id` proxies Subsonic/Navidrome `getCoverArt` and returns image bytes with an image `Content-Type` when available. Credentials stay server-side and are never exposed in response URLs, bodies, or headers. Invalid or missing cover IDs return JSON errors.

### Discovery

`POST /api/discovery/refresh` analyzes favorite local artists and stores Last.fm-powered discovery rows in SQLite. It calls and caches:

- `artist.getInfo`
- `artist.getTopAlbums`
- `artist.getTopTracks`
- `artist.getSimilar`

Last.fm responses are cached in `api_cache`; default TTL is 7 days for artist info/top albums/top tracks and 14 days for similar artists. Cache keys include the Last.fm method and normalized parameters. Individual Last.fm failures are recorded in the refresh response and do not abort the whole refresh.

Discovery read endpoints return stored rows:

- `GET /api/discovery/missing-albums`
- `GET /api/discovery/new-releases`
- `GET /api/discovery/similar-artists`

They support `limit`, `offset`, and `include_owned=true`. Owned items are hidden by default where applicable. Results include match status, confidence score, source URL, cover URL, reason, and generated timestamp.

Known limitations:

- Last.fm release dates may be incomplete or absent.
- Last.fm top albums/tracks are popularity lists, not guaranteed chronological new releases.
- Discovery matching is fuzzy and confidence-based.

## Seed/mock mode
Set `SEED_MODE=true` in `.env` to allow frontend development without valid API credentials.
