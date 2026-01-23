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
    // NOTE: Index is built at startup, so this reflects segments at that time
    // TODO: Implement automatic index refresh for new segments
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
            // If no events found in the requested range, try to fall back to earlier data
            if events.is_empty() && start_ns.is_some() {
                // Try to find the most recent data before the requested time
                // Go back up to 7 days (604800 seconds)
                let start_time = start_ns.unwrap();
                let fallback_start = start_time - (7 * 24 * 3600 * 1_000_000_000i128);

                // Read from fallback_start to requested start time
                if let Ok(fallback_events) = reader.read_time_range(Some(fallback_start), Some(start_time)) {
                    // Take the most recent events before the requested time
                    events = fallback_events;
                }
            }

            // Apply limit with prioritization for SystemMetrics
            if let Some(limit) = query.limit {
                if events.len() > limit {
                    // Separate SystemMetrics from other events
                    let mut system_metrics = Vec::new();
                    let mut other_events = Vec::new();

                    for event in events {
                        if matches!(event, crate::event::Event::SystemMetrics(_)) {
                            system_metrics.push(event);
                        } else {
                            other_events.push(event);
                        }
                    }

                    // Take the most recent SystemMetrics (up to limit)
                    let sm_count = system_metrics.len().min(limit);
                    let sm_to_take = if system_metrics.len() > sm_count {
                        system_metrics.split_off(system_metrics.len() - sm_count)
                    } else {
                        system_metrics
                    };

                    // Fill remaining space with other events
                    let remaining = limit.saturating_sub(sm_to_take.len());
                    let other_to_take = if other_events.len() > remaining {
                        other_events.split_off(other_events.len() - remaining)
                    } else {
                        other_events
                    };

                    // Combine and sort by timestamp
                    events = sm_to_take;
                    events.extend(other_to_take);
                    events.sort_by_key(|e| match e {
                        crate::event::Event::SystemMetrics(m) => m.ts.unix_timestamp_nanos(),
                        crate::event::Event::ProcessLifecycle(p) => p.ts.unix_timestamp_nanos(),
                        crate::event::Event::ProcessSnapshot(p) => p.ts.unix_timestamp_nanos(),
                        crate::event::Event::SecurityEvent(s) => s.ts.unix_timestamp_nanos(),
                        crate::event::Event::Anomaly(a) => a.ts.unix_timestamp_nanos(),
                    });
                }
            }

            // Format events for API response
            let formatted_events: Vec<serde_json::Value> = events
                .iter()
                .map(format_event_for_api)
                .collect();

            HttpResponse::Ok().json(serde_json::json!({
                "count": formatted_events.len(),
                "events": formatted_events,
                "fallback": formatted_events.len() > 0 && start_ns.is_some(),
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to read events: {}", e),
        })),
    }
}

fn format_event_for_api(event: &Event) -> serde_json::Value {
    match event {
        Event::SystemMetrics(m) => {
            let mem_pct = if m.mem_total_bytes > 0 {
                (m.mem_used_bytes as f64 / m.mem_total_bytes as f64) * 100.0
            } else {
                0.0
            };

            let swap_pct = if m.swap_total_bytes > 0 {
                (m.swap_used_bytes as f64 / m.swap_total_bytes as f64) * 100.0
            } else {
                0.0
            };

            let disk_pct = if m.disk_total_bytes > 0 {
                (m.disk_used_bytes as f64 / m.disk_total_bytes as f64) * 100.0
            } else {
                0.0
            };

            serde_json::json!({
                "type": "SystemMetrics",
                "timestamp": m.ts.to_string(),
                "kernel": m.kernel_version,
                "cpu_model": m.cpu_model,
                "cpu_mhz": m.cpu_mhz,
                "system_uptime_seconds": m.system_uptime_seconds,
                "cpu": m.cpu_usage_percent,
                "per_core_cpu": m.per_core_usage,
                "mem": mem_pct,
                "mem_used": m.mem_used_bytes,
                "mem_total": m.mem_total_bytes,
                "swap": swap_pct,
                "swap_used": m.swap_used_bytes,
                "swap_total": m.swap_total_bytes,
                "load": m.load_avg_1m,
                "load5": m.load_avg_5m,
                "load15": m.load_avg_15m,
                "disk": disk_pct.round(),
                "disk_used": m.disk_used_bytes,
                "disk_total": m.disk_total_bytes,
                "disk_read": m.disk_read_bytes_per_sec,
                "disk_write": m.disk_write_bytes_per_sec,
                "per_disk": m.per_disk_metrics.iter().map(|d| serde_json::json!({
                    "device": d.device_name,
                    "read": d.read_bytes_per_sec,
                    "write": d.write_bytes_per_sec,
                    "temp": d.temp_celsius,
                })).collect::<Vec<_>>(),
                "filesystems": m.filesystems.iter().map(|fs| serde_json::json!({
                    "filesystem": fs.filesystem,
                    "mount": fs.mount_point,
                    "total": fs.total_bytes,
                    "used": fs.used_bytes,
                    "available": fs.available_bytes,
                })).collect::<Vec<_>>(),
                "users": m.logged_in_users.iter().map(|u| serde_json::json!({
                    "username": u.username,
                    "terminal": u.terminal,
                    "remote_host": u.remote_host,
                })).collect::<Vec<_>>(),
                "net_recv": m.net_recv_bytes_per_sec,
                "net_send": m.net_send_bytes_per_sec,
                "net_recv_errors": m.net_recv_errors_per_sec,
                "net_send_errors": m.net_send_errors_per_sec,
                "net_recv_drops": m.net_recv_drops_per_sec,
                "net_send_drops": m.net_send_drops_per_sec,
                "net_interface": m.net_interface,
                "net_ip": m.net_ip_address,
                "net_gateway": m.net_gateway,
                "net_dns": m.net_dns,
                "tcp": m.tcp_connections,
                "tcp_wait": m.tcp_time_wait,
                "ctxt": m.context_switches_per_sec,
                "cpu_temp": m.temps.cpu_temp_celsius,
                "per_core_temps": m.temps.per_core_temps,
                "gpu_temp": m.temps.gpu_temp_celsius,
                "mobo_temp": m.temps.motherboard_temp_celsius,
                "gpu_freq": m.gpu.gpu_freq_mhz,
                "gpu_mem_freq": m.gpu.mem_freq_mhz,
                "gpu_temp2": m.gpu.gpu_temp_celsius,
                "gpu_power": m.gpu.power_watts,
                "fans": m.fans.iter().map(|f| serde_json::json!({
                    "label": f.label,
                    "rpm": f.rpm,
                })).collect::<Vec<_>>(),
            })
        }
        Event::ProcessLifecycle(p) => serde_json::json!({
            "type": "ProcessLifecycle",
            "timestamp": p.ts.to_string(),
            "kind": format!("{:?}", p.kind),
            "pid": p.pid,
            "name": p.name,
        }),
        Event::ProcessSnapshot(p) => serde_json::json!({
            "type": "ProcessSnapshot",
            "timestamp": p.ts.to_string(),
            "total_processes": p.total_processes,
            "running_processes": p.running_processes,
            "processes": p.processes.iter().map(|proc| serde_json::json!({
                "pid": proc.pid,
                "name": proc.name,
                "cpu_percent": proc.cpu_percent,
                "mem_bytes": proc.mem_bytes,
            })).collect::<Vec<_>>(),
        }),
        Event::SecurityEvent(s) => serde_json::json!({
            "type": "SecurityEvent",
            "timestamp": s.ts.to_string(),
            "kind": format!("{:?}", s.kind),
            "user": s.user,
            "source_ip": s.source_ip,
            "message": s.message,
        }),
        Event::Anomaly(a) => serde_json::json!({
            "type": "Anomaly",
            "timestamp": a.ts.to_string(),
            "severity": format!("{:?}", a.severity),
            "kind": format!("{:?}", a.kind),
            "message": a.message,
        }),
    }
}
