use actix_web::{middleware, web, App, HttpServer};
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;

use crate::broadcast::EventBroadcaster;
use crate::config::Config;
use crate::reader::LogReader;

use super::{auth, health, routes, websocket};

pub async fn start_server(
    data_dir: String,
    port: u16,
    broadcaster: Arc<EventBroadcaster>,
    config: Config,
) -> Result<()> {
    let reader = web::Data::new(LogReader::new(data_dir));
    let broadcaster_data = web::Data::from(broadcaster);
    let config_data = web::Data::new(config.clone());
    let start_time = web::Data::new(Instant::now());

    println!("Starting web server on 0.0.0.0:{}", port);

    HttpServer::new(move || {
        App::new()
            .app_data(reader.clone())
            .app_data(broadcaster_data.clone())
            .app_data(config_data.clone())
            .app_data(start_time.clone())
            .wrap(middleware::Logger::default())
            .wrap(auth::BasicAuth::new(config.auth.clone()))
            .route("/", web::get().to(routes::index))
            .route("/api/events", web::get().to(routes::api_events))
            .route("/ws", web::get().to(websocket::ws_handler))
            .route("/health", web::get().to(health::health_check))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
    .map_err(|e| anyhow::anyhow!("Server error: {}", e))
}
