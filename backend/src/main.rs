// SPDX-License-Identifier: GPL-3.0-only

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path as FsPath, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tower_http::{cors::CorsLayer, services::ServeDir};

#[derive(Clone)]
struct AppState {
    data_dir: PathBuf,
    rev: Arc<AtomicU64>,
    tx: tokio::sync::broadcast::Sender<u64>,
    last_api_write: Arc<std::sync::RwLock<std::time::Instant>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Area {
    id: String,
    title: String,
    rect: Rect,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct NodeLayout {
    id: String,
    x: f64,
    y: f64,
    #[serde(default)]
    #[serde(rename = "areaId")]
    area_id: Option<String>,
    #[serde(default)]
    #[serde(rename = "parentId")]
    parent_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Edge {
    id: String,
    from: String,
    to: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    #[serde(rename = "sourceHandle")]
    source_handle: Option<String>,
    #[serde(default)]
    #[serde(rename = "targetHandle")]
    target_handle: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GraphFile {
    version: u32,
    #[serde(default)]
    areas: Vec<Area>,
    #[serde(default)]
    nodes: Vec<NodeLayout>,
    #[serde(default)]
    edges: Vec<Edge>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct NodeFull {
    id: String,
    title: String,
    #[serde(rename = "type")]
    ntype: String,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    fields: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    body: String,
    x: f64,
    y: f64,
    #[serde(rename = "areaId")]
    area_id: Option<String>,
    #[serde(rename = "parentId")]
    parent_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StateResponse {
    areas: Vec<Area>,
    nodes: Vec<NodeFull>,
    edges: Vec<Edge>,
    rev: u64,
}

fn data_paths(data_dir: &FsPath) -> (PathBuf, PathBuf, PathBuf) {
    (
        data_dir.join("graph.yaml"),
        data_dir.join("nodes"),
        data_dir.join("assets"),
    )
}

fn split_frontmatter(text: &str) -> (String, String) {
    let mut lines = text.lines();
    if lines.next().unwrap_or("").trim() != "---" {
        return (String::new(), text.to_string());
    }
    let mut fm = Vec::new();
    for line in lines.by_ref() {
        if line.trim() == "---" {
            break;
        }
        fm.push(line);
    }
    let body: Vec<&str> = lines.collect();
    (fm.join("\n"), body.join("\n"))
}

#[derive(Debug, Deserialize)]
struct NodeFrontmatter {
    #[serde(default)]
    title: String,
    #[serde(default, rename = "type")]
    ntype: String,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    fields: serde_json::Map<String, serde_json::Value>,
}

fn load_graph(data_dir: &FsPath) -> Result<GraphFile, String> {
    let (graph_path, _, _) = data_paths(data_dir);
    let text = std::fs::read_to_string(&graph_path).map_err(|e| e.to_string())?;
    serde_yaml::from_str(&text).map_err(|e| e.to_string())
}

fn load_one_node(
    nodes_dir: &FsPath,
    id: &str,
) -> (String, String, Option<String>, serde_json::Map<String, serde_json::Value>, String) {
    let path = nodes_dir.join(format!("{id}.md"));
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let (fm_text, body) = split_frontmatter(&text);
    let fm: NodeFrontmatter = serde_yaml::from_str(&fm_text).unwrap_or(NodeFrontmatter {
        title: id.to_string(),
        ntype: "note".to_string(),
        color: None,
        fields: serde_json::Map::new(),
    });
    let title = if fm.title.is_empty() { id.to_string() } else { fm.title };
    let ntype = if fm.ntype.is_empty() { "note".to_string() } else { fm.ntype };
    (title, ntype, fm.color, fm.fields, body)
}

fn load_state(data_dir: &FsPath, rev: u64) -> Result<StateResponse, String> {
    let graph = load_graph(data_dir)?;
    let (_, nodes_dir, _) = data_paths(data_dir);
    let mut nodes = Vec::new();
    for l in &graph.nodes {
        let (title, ntype, color, fields, body) = load_one_node(&nodes_dir, &l.id);
        nodes.push(NodeFull {
            id: l.id.clone(),
            title,
            ntype,
            color,
            fields,
            body,
            x: l.x,
            y: l.y,
            area_id: l.area_id.clone(),
            parent_id: l.parent_id.clone(),
        });
    }
    Ok(StateResponse { areas: graph.areas, nodes, edges: graph.edges, rev })
}

fn write_atomic(path: &FsPath, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

async fn get_state(State(s): State<AppState>) -> impl IntoResponse {
    let rev = s.rev.load(Ordering::Relaxed);
    match load_state(&s.data_dir, rev) {
        Ok(st) => (StatusCode::OK, Json(st)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

async fn put_state(State(s): State<AppState>, Json(st): Json<StateResponse>) -> impl IntoResponse {
    *s.last_api_write.write().unwrap() = std::time::Instant::now();
    let (graph_path, nodes_dir, _) = data_paths(&s.data_dir);
    let graph = GraphFile {
        version: 1,
        areas: st.areas,
        nodes: st
            .nodes
            .iter()
            .map(|n| NodeLayout {
                id: n.id.clone(),
                x: n.x,
                y: n.y,
                area_id: n.area_id.clone(),
                parent_id: n.parent_id.clone(),
            })
            .collect(),
        edges: st.edges,
    };
    let yaml = match serde_yaml::to_string(&graph) {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    if let Err(e) = write_atomic(&graph_path, &yaml) {
        return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response();
    }
    for n in &st.nodes {
        let mut fm_val = serde_json::json!({
            "id": n.id,
            "title": n.title,
            "type": n.ntype,
            "fields": n.fields,
        });
        if let Some(c) = &n.color {
            fm_val["color"] = serde_json::Value::String(c.clone());
        }
        let fm = serde_yaml::to_string(&fm_val).unwrap_or_default();
        let text = format!("---\n{fm}---\n{}", n.body);
        let path = nodes_dir.join(format!("{}.md", n.id));
        if let Err(e) = write_atomic(&path, &text) {
            return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response();
        }
    }
    *s.last_api_write.write().unwrap() = std::time::Instant::now();
    let rev = s.rev.fetch_add(1, Ordering::Relaxed) + 1;
    let _ = s.tx.send(rev);
    (StatusCode::OK, Json(serde_json::json!({ "rev": rev }))).into_response()
}

async fn put_asset(State(s): State<AppState>, Path(name): Path<String>, body: Bytes) -> impl IntoResponse {
    let safe: String = name.chars().filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_').collect();
    if safe.is_empty() {
        return (StatusCode::BAD_REQUEST, "bad name".to_string()).into_response();
    }
    let (_, _, assets_dir) = data_paths(&s.data_dir);
    *s.last_api_write.write().unwrap() = std::time::Instant::now();
    let path = assets_dir.join(&safe);
    std::fs::create_dir_all(&assets_dir).ok();
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, &body).is_err() || std::fs::rename(&tmp, &path).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "asset write failed".to_string()).into_response();
    }
    (StatusCode::OK, Json(serde_json::json!({ "name": safe }))).into_response()
}

async fn ws_handler(State(s): State<AppState>, ws: axum::extract::ws::WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket| async move {
        let mut rx = s.tx.subscribe();
        let current = s.rev.load(Ordering::Relaxed);
        let _ = socket
            .send(axum::extract::ws::Message::Text(format!("{{\"rev\":{current}}}").into()))
            .await;
        while rx.recv().await.is_ok() {
            let rev = s.rev.load(Ordering::Relaxed);
            if socket
                .send(axum::extract::ws::Message::Text(format!("{{\"rev\":{rev}}}").into()))
                .await
                .is_err()
            {
                break;
            }
        }
    })
}

fn watcher_interesting(ev: &notify::Event) -> bool {
    // Reads must not count. Only real content changes reload the UI.
    match ev.kind {
        notify::EventKind::Modify(_) | notify::EventKind::Create(_) | notify::EventKind::Remove(_) => {}
        _ => return false,
    }
    ev.paths.iter().any(|p| {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with(".tmp") {
            return false;
        }
        matches!(p.extension().and_then(|e| e.to_str()), Some("md") | Some("yaml") | Some("yml"))
    })
}

fn spawn_watcher(
    data_dir: PathBuf,
    rev: Arc<AtomicU64>,
    tx: tokio::sync::broadcast::Sender<u64>,
    last_api_write: Arc<std::sync::RwLock<std::time::Instant>>,
) {
    use notify::{RecursiveMode, Watcher};
    std::thread::spawn(move || {
        let (ftx, frx) = std::sync::mpsc::channel();
        let mut watcher = match notify::recommended_watcher(ftx) {
            Ok(w) => w,
            Err(_) => return,
        };
        match watcher.watch(&data_dir, RecursiveMode::Recursive) {
            Ok(_) => eprintln!("[watch] watching {}", data_dir.display()),
            Err(e) => eprintln!("[watch] WATCH FAILED: {e}"),
        }
        loop {
            let mut interesting = false;
            match frx.recv() {
                Err(_) => break,
                Ok(Ok(ev)) => interesting |= watcher_interesting(&ev),
                Ok(Err(_)) => {}
            }
            // Small delay groups rapid saves.
            std::thread::sleep(std::time::Duration::from_millis(300));
            // Drain extra events.
            while let Ok(msg) = frx.try_recv() {
                if let Ok(ev) = msg {
                    interesting |= watcher_interesting(&ev);
                }
            }
            if !interesting {
                continue;
            }
            // Skip echo of our own API writes.
            if last_api_write.read().unwrap().elapsed() < std::time::Duration::from_millis(1200) {
                continue;
            }
            let next = rev.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = tx.send(next);
        }
    });
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let data_dir = PathBuf::from(data_dir);
    let (graph_path, nodes_dir, assets_dir) = data_paths(&data_dir);
    std::fs::create_dir_all(&nodes_dir).ok();
    std::fs::create_dir_all(&assets_dir).ok();
    if !graph_path.exists() {
        eprintln!("missing {}", graph_path.display());
    }
    let rev = Arc::new(AtomicU64::new(1));
    let (tx, _rx) = tokio::sync::broadcast::channel::<u64>(16);
    let last_api_write = Arc::new(std::sync::RwLock::new(
        std::time::Instant::now() - std::time::Duration::from_secs(3600),
    ));
    spawn_watcher(data_dir.clone(), rev.clone(), tx.clone(), last_api_write.clone());
    let state = AppState { data_dir: data_dir.clone(), rev, tx, last_api_write };

    let assets_service = ServeDir::new(data_dir.join("assets"));
    // Frontend dist in dev and ./public in docker.
    let public_dir = ["public", "/app/public", "frontend/dist", "../frontend/dist"]
        .into_iter()
        .find(|p| FsPath::new(p).exists())
        .unwrap_or("public");
    let app = Router::new()
        .route("/api/state", get(get_state).put(put_state))
        .route("/api/assets/{name}", put(put_asset))
        .route("/ws", get(ws_handler))
        .nest_service("/uploads", assets_service)
        .fallback_service(ServeDir::new(public_dir).not_found_service(get(|| async {
            (StatusCode::NOT_FOUND, "not found")
        })))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port: u16 = std::env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(3000);
    let addr = format!("0.0.0.0:{port}");
    println!("weft on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
