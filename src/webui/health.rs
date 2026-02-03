use actix_web::{web, HttpResponse};
use serde_json::json;
use std::time::Instant;

use crate::config::Config;
use crate::reader::LogReader;

pub async fn health_check(
    reader: web::Data<LogReader>,
    start_time: web::Data<Instant>,
    config: web::Data<Config>,
) -> HttpResponse {
    // Calculate uptime
    let uptime_secs = start_time.elapsed().as_secs();

    // Count events
    let event_count = match reader.read_all_events() {
        Ok(events) => events.len(),
        Err(_) => 0,
    };

    // Calculate storage usage
    let storage_bytes_used = calculate_storage_usage();
    let max_storage_bytes = config.server.max_storage_mb * 1024 * 1024;
    let storage_percent = (storage_bytes_used as f64 / max_storage_bytes as f64) * 100.0;

    let health_status = json!({
        "status": "healthy",
        "uptime_seconds": uptime_secs,
        "event_count": event_count,
        "storage_bytes_used": storage_bytes_used,
        "storage_bytes_max": max_storage_bytes,
        "storage_percent": format!("{:.2}", storage_percent),
        "timestamp": time::OffsetDateTime::now_utc().to_string(),
    });

    HttpResponse::Ok().json(health_status)
}

fn calculate_storage_usage() -> u64 {
    let data_dir = "./data";

    match std::fs::read_dir(data_dir) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.metadata().ok())
            .map(|metadata| metadata.len())
            .sum(),
        Err(_) => 0,
    }
}
