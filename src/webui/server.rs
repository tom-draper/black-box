use anyhow::Result;
use axum::{
    Router,
    routing::get,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tower_http::trace::TraceLayer;

use crate::broadcast::EventBroadcaster;
use crate::config::Config;
use crate::indexed_reader::IndexedReader;
use crate::reader::LogReader;

use super::{auth, health, playback, routes, websocket};

pub async fn start_server(
    data_dir: String,
    port: u16,
    broadcaster: Arc<EventBroadcaster>,
    config: Config,
    metadata: Arc<std::sync::RwLock<Option<crate::event::Metadata>>>,
) -> Result<()> {
    let reader = Arc::new(LogReader::new(&data_dir));

    // Build indexed reader for time-travel queries
    let indexed_reader = match IndexedReader::new(&data_dir) {
        Ok(r) => Arc::new(r),
        Err(e) => {
            eprintln!("Warning: Failed to build index: {}. Time-travel features disabled.", e);
            Arc::new(IndexedReader::new(&data_dir).unwrap_or_else(|_| {
                // Create empty reader as fallback
                IndexedReader::new(std::env::temp_dir()).unwrap()
            }))
        }
    };

    let start_time = Arc::new(Instant::now());

    // Spawn the broadcaster bridge (crossbeam -> tokio broadcast)
    let broadcaster_clone = (*broadcaster).clone();
    tokio::spawn(async move {
        broadcaster_clone.run().await;
    });

    // Build app state
    let state = routes::AppState {
        reader,
        indexed_reader,
        broadcaster,
        config: Arc::new(config.clone()),
        start_time,
        data_dir: Arc::new(data_dir),
        metadata,
    };

    // Build router with auth middleware
    let app = Router::new()
        .route("/", get(routes::index))
        .route("/api/events", get(routes::api_events))
        .route("/api/playback/info", get(playback::api_playback_info))
        .route("/api/playback/events", get(playback::api_playback_events))
        .route("/api/initial-state", get(playback::api_initial_state))
        .route("/api/timeline", get(playback::api_timeline))
        .route("/ws", get(websocket::ws_handler))
        .route("/health", get(health::health_check))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::basic_auth_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Server listening on http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Configure TCP socket for low latency (disable Nagle's algorithm)
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>()
    )
    .tcp_nodelay(true)
    .await
    .map_err(|e| anyhow::anyhow!("Server error: {}", e))
}
