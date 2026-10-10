# Weft self-contained image. Svelte SPA plus Rust backend. No database exists.
# Build from the repo root and run with a volume for state.
#   docker build -t weft:0.1.0 .
#   docker run --rm -p 3000:3000 -v weft-data:/data weft:0.1.0

FROM node:22-alpine AS web
WORKDIR /web
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

FROM rust:1-bookworm AS api
WORKDIR /api
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN useradd -m app && mkdir -p /app/public /app/seed /data && chown -R app:app /app /data
COPY --from=web /web/dist /app/public
COPY --from=api /api/target/release/weft /app/weft
COPY seed/graph.yaml /app/seed/graph.yaml
COPY seed/nodes /app/seed/nodes
USER app
WORKDIR /app
ENV PORT=3000 DATA_DIR=/data
EXPOSE 3000
# First start seeds /data from the sample map. Later starts keep the volume.
CMD sh -c 'if [ ! -f /data/graph.yaml ]; then cp /app/seed/graph.yaml /data/graph.yaml; mkdir -p /data/nodes /data/assets; cp /app/seed/nodes/*.md /data/nodes/; fi; exec /app/weft'
