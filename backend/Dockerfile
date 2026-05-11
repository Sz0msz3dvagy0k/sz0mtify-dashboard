FROM rust:1.88 AS builder
WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock* ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs && cargo build --release || true
RUN rm -rf src target/release/music-dashboard-backend target/release/deps/music_dashboard_backend-*
COPY backend .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/music-dashboard-backend /usr/local/bin/music-dashboard-backend
COPY backend/migrations /app/migrations
CMD ["music-dashboard-backend"]
