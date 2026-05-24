# ---------- stage 1: build the React SPA ----------
FROM node:22-bookworm-slim AS web
WORKDIR /web
COPY web/package.json web/package-lock.json* ./
RUN npm install
COPY web/ ./
RUN npm run build

# ---------- stage 2: build the Rust server ----------
FROM rust:slim-bookworm AS server
WORKDIR /src
COPY server/Cargo.toml ./
RUN mkdir src && echo "fn main(){}" > src/main.rs
# cache deps
RUN cargo build --release || true
COPY server/ ./
RUN touch src/main.rs && cargo build --release

# ---------- stage 3: runtime ----------
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=server /src/target/release/getpixii-server /app/getpixii-server
COPY --from=web /web/dist /app/dist
ENV STATIC_DIR=/app/dist
ENV BIND=0.0.0.0:8080
EXPOSE 8080
CMD ["/app/getpixii-server"]
