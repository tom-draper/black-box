use actix_web::{middleware, web, App, HttpServer};
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;

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
) -> Result<()> {
    let reader = web::Data::new(LogReader::new(&data_dir));

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
    let indexed_reader_data = web::Data::new(indexed_reader);

    let broadcaster_clone = (*broadcaster).clone();
    let broadcaster_data = web::Data::from(broadcaster);
    let config_data = web::Data::new(config.clone());
    let start_time = web::Data::new(Instant::now());
    let data_dir_data = web::Data::new(data_dir.clone());

    // Spawn the broadcaster bridge (crossbeam -> tokio broadcast)
    tokio::spawn(async move {
        broadcaster_clone.run().await;
    });

    println!("Server listening on http://localhost:{}", port);

    HttpServer::new(move || {
        App::new()
            .app_data(reader.clone())
            .app_data(indexed_reader_data.clone())
            .app_data(broadcaster_data.clone())
            .app_data(config_data.clone())
            .app_data(start_time.clone())
            .app_data(data_dir_data.clone())
            .wrap(middleware::Logger::default())
            .wrap(auth::BasicAuth::new(config.auth.clone()))
            .route("/", web::get().to(routes::index))
            .route("/api/events", web::get().to(routes::api_events))
            .route("/api/playback/info", web::get().to(playback::api_playback_info))
            .route("/api/playback/events", web::get().to(playback::api_playback_events))
            .route("/api/initial-state", web::get().to(playback::api_initial_state))
            .route("/api/timeline", web::get().to(playback::api_timeline))
            .route("/ws", web::get().to(websocket::ws_handler))
            .route("/health", web::get().to(health::health_check))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
    .map_err(|e| anyhow::anyhow!("Server error: {}", e))
}
