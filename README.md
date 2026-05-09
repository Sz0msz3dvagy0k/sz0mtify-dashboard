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

## Implemented endpoints
Includes health, settings, sync, library, stats, discovery, recommendations, search, and cover proxy placeholders.

## Seed/mock mode
Set `SEED_MODE=true` in `.env` to allow frontend development without valid API credentials.
