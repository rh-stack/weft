# Homelab Map

Small single user web app. Canvas of nodes plus notes pages. All state is text files. No database exists. No login exists in v1, so keep the app inside your tailnet.

## Layout

- `frontend/` is Svelte 5 plus Svelte Flow plus TypeScript. It builds a static SPA.
- `backend/` is Rust plus Axum. It serves the SPA and a JSON API. It reads and writes `data/`.
- `seed/` holds sample data. Copy it to `data/` on first setup.
- `data/` holds live state and stays out of git. `data/graph.yaml` holds areas, node positions, parents, edges. `data/nodes/*.md` holds one node each. Top block holds id, title, type, fields. Body is Markdown notes. `data/assets/` holds uploaded images, served under `/uploads/`.

## First run on any machine

You need Node 22 and stable Rust.

```
cp -r seed data
cd frontend && npm install && npm run build
cd ../backend && DATA_DIR=../data PORT=3000 cargo run --release
```

Open `http://127.0.0.1:3000`.

Two env vars control the backend. `PORT` picks the listen port. Default is 3000. `DATA_DIR` picks the state folder. Default is `data`.

## Dev with hot reload

```
# shell 1
cd backend && DATA_DIR=../data PORT=3000 cargo run
# shell 2
cd frontend && npm run dev
```

Open the vite URL. API calls proxy to `:3000`.

## Production run

Build the frontend once and run the release binary. Keep `data/` on a backed up disk. It holds everything.

```
cd frontend && npm install && npm run build
cd ../backend && cargo build --release
mkdir -p /srv/homelab-map && cp -r ../frontend/dist /srv/homelab-map/public
cp -r ../seed /srv/homelab-map/data
DATA_DIR=/srv/homelab-map/data PORT=3000 /srv/homelab-map/homelab-map-backend
```

The backend finds the SPA in `./public`, `/app/public`, `frontend/dist`, or `../frontend/dist`. Place the built files in one of those spots relative to your start folder. A simple systemd unit works for autostart. Point `ExecStart` at the binary and set `DATA_DIR` and `PORT` in `Environment`.

Put a reverse proxy in front for TLS. There is no auth in v1. Bind to localhost or serve only inside your tailnet.

## Containerize it yourself

The repo ships no Docker files. Recreate them with this recipe when you need an image.

Minimal multi stage Dockerfile. Build the SPA, build the release binary, copy both into a slim runtime with a seed copy.

```
FROM node:22-alpine AS web
WORKDIR /web
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm install
COPY frontend/ ./
RUN npm run build

FROM rust:1-bookworm AS api
WORKDIR /api
COPY backend/Cargo.toml backend/Cargo.lock* ./
COPY backend/src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN useradd -m app && mkdir -p /app/public /app/seed /data && chown -R app:app /app /data
COPY --from=web /web/dist /app/public
COPY --from=api /api/target/release/homelab-map-backend /app/homelab-map
COPY seed/graph.yaml /app/seed/graph.yaml
COPY seed/nodes /app/seed/nodes
USER app
WORKDIR /app
ENV PORT=3000 DATA_DIR=/data
EXPOSE 3000
CMD sh -c 'if [ ! -f /data/graph.yaml ]; then cp /app/seed/graph.yaml /data/graph.yaml; mkdir -p /data/nodes /data/assets; cp /app/seed/nodes/*.md /data/nodes/; fi; exec /app/homelab-map'
```

Run it with a volume. First start seeds `/data` from the sample. Later starts keep the volume.

```
docker build -t homelab-map:0.1.0 .
docker run --rm -p 3000:3000 -v homelab-map-data:/data homelab-map:0.1.0
```

For k3s push the image to a registry your cluster can read, then create a 1 Gi PVC, a single replica Deployment with `DATA_DIR=/data` and the PVC mounted at `/data`, plus a ClusterIP Service on port 80 to target 3000. Keep the Service inside the tailnet.

## Use

- Drag cards to move them. Drag between the dots on cards to link them.
- Right click empty canvas to add a node or area at that spot.
- Double click a card, or its notes button, to open notes in a popover.
- Delete key removes the picked node, area, or edge.
- Press `?` for all keyboard shortcuts. `Ctrl+Z` undoes a delete.
- Areas move by the top left grip and resize by the bottom right grip only.
- Search lives on the island at the top left of the canvas. Slash focuses it. Enter with one match opens its notes.
- Notes use a Markdown editor with autosave. Paste images straight in.
- Nodes take one of five pastel colors from the dots on top of the notes popover.

## AI file edits

- Move a node: edit `x` and `y` under `data/graph.yaml`.
- Rename a node: edit `title` in `data/nodes/<id>.md`.
- Change fields: edit `fields` in the same file.
- Add a node: add one entry in `graph.yaml` plus one file in `nodes/`.
- Add an edge: add one entry under `edges` in `graph.yaml`.
- The backend picks up disk changes and pushes a new `rev` over `/ws`.
