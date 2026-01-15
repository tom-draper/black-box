use actix_web::{web, HttpResponse};
use serde_json::json;
use std::time::Instant;

use crate::reader::LogReader;

const MAX_STORAGE_BYTES: u64 = 100 * 1024 * 1024; // 100MB

pub async fn health_check(
    reader: web::Data<LogReader>,
    start_time: web::Data<Instant>,
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
    let storage_percent = (storage_bytes_used as f64 / MAX_STORAGE_BYTES as f64) * 100.0;

    let health_status = json!({
        "status": "healthy",
        "uptime_seconds": uptime_secs,
        "event_count": event_count,
        "storage_bytes_used": storage_bytes_used,
        "storage_bytes_max": MAX_STORAGE_BYTES,
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
