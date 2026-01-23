use actix_web::{web, HttpResponse};
use serde::Deserialize;
use std::sync::Arc;
use time::OffsetDateTime;

use crate::indexed_reader::IndexedReader;
use crate::event::Event;

#[derive(Deserialize)]
pub struct PlaybackQuery {
    #[serde(rename = "start")]
    start_timestamp: Option<i64>,  // Unix seconds
    #[serde(rename = "end")]
    end_timestamp: Option<i64>,    // Unix seconds
    #[serde(rename = "limit")]
    limit: Option<usize>,
}

/// Get time range metadata
pub async fn api_playback_info(
    reader: web::Data<Arc<IndexedReader>>,
) -> HttpResponse {
    if let Some((first_ns, last_ns)) = reader.get_time_range() {
        let first_secs = (first_ns / 1_000_000_000) as i64;
        let last_secs = (last_ns / 1_000_000_000) as i64;

        let first_dt = OffsetDateTime::from_unix_timestamp(first_secs).ok();
        let last_dt = OffsetDateTime::from_unix_timestamp(last_secs).ok();

        HttpResponse::Ok().json(serde_json::json!({
            "first_timestamp": first_secs,
            "last_timestamp": last_secs,
            "first_timestamp_iso": first_dt.map(|dt| dt.to_string()),
            "last_timestamp_iso": last_dt.map(|dt| dt.to_string()),
            "segment_count": reader.get_segments().len(),
            "estimated_event_count": reader.estimate_event_count(),
        }))
    } else {
        HttpResponse::Ok().json(serde_json::json!({
            "first_timestamp": null,
            "last_timestamp": null,
            "segment_count": 0,
            "estimated_event_count": 0,
        }))
    }
}

/// Get events in a time range for playback
pub async fn api_playback_events(
    reader: web::Data<Arc<IndexedReader>>,
    query: web::Query<PlaybackQuery>,
) -> HttpResponse {
    let start_ns = query.start_timestamp.map(|s| (s as i128) * 1_000_000_000);
    let end_ns = query.end_timestamp.map(|s| (s as i128) * 1_000_000_000);

    match reader.read_time_range(start_ns, end_ns) {
        Ok(mut events) => {
            // Apply limit if specified
            if let Some(limit) = query.limit {
                events.truncate(limit);
            }

            // Format events for API response
            let formatted_events: Vec<serde_json::Value> = events
                .iter()
                .map(format_event_for_api)
                .collect();

            HttpResponse::Ok().json(serde_json::json!({
                "count": formatted_events.len(),
                "events": formatted_events,
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to read events: {}", e),
        })),
    }
}

fn format_event_for_api(event: &Event) -> serde_json::Value {
    match event {
        Event::SystemMetrics(m) => serde_json::json!({
            "type": "SystemMetrics",
            "timestamp": m.ts.unix_timestamp(),
            "data": {
                "cpu_usage": m.cpu_usage_percent,
                "mem_used": m.mem_used_bytes,
                "mem_total": m.mem_total_bytes,
                "disk_used": m.disk_used_bytes,
                "disk_total": m.disk_total_bytes,
            }
        }),
        Event::ProcessLifecycle(p) => serde_json::json!({
            "type": "ProcessLifecycle",
            "timestamp": p.ts.unix_timestamp(),
            "data": {
                "pid": p.pid,
                "name": p.name,
                "kind": format!("{:?}", p.kind),
            }
        }),
        Event::ProcessSnapshot(p) => serde_json::json!({
            "type": "ProcessSnapshot",
            "timestamp": p.ts.unix_timestamp(),
            "data": {
                "total_processes": p.total_processes,
                "running_processes": p.running_processes,
                "process_count": p.processes.len(),
            }
        }),
        Event::SecurityEvent(s) => serde_json::json!({
            "type": "SecurityEvent",
            "timestamp": s.ts.unix_timestamp(),
            "data": {
                "kind": format!("{:?}", s.kind),
                "user": s.user,
                "message": s.message,
            }
        }),
        Event::Anomaly(a) => serde_json::json!({
            "type": "Anomaly",
            "timestamp": a.ts.unix_timestamp(),
            "data": {
                "severity": format!("{:?}", a.severity),
                "kind": format!("{:?}", a.kind),
                "message": a.message,
            }
        }),
    }
}
