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

/// Get the most recent complete SystemMetrics (with static/semi-static fields) for page initialization
pub async fn api_initial_state(
    reader: web::Data<Arc<IndexedReader>>,
) -> HttpResponse {
    // Look back up to 5 minutes to find a complete SystemMetrics event
    let now_ns = OffsetDateTime::now_utc().unix_timestamp_nanos();
    let lookback_ns = now_ns - (5 * 60 * 1_000_000_000i128); // 5 minutes

    match reader.read_time_range(Some(lookback_ns), Some(now_ns)) {
        Ok(events) => {
            // Find the most recent SystemMetrics with filesystems
            for event in events.iter().rev() {
                if let Event::SystemMetrics(m) = event {
                    if m.filesystems.is_some() {
                        // Found a complete event, return it formatted
                        return HttpResponse::Ok().json(format_event_for_api(event));
                    }
                }
            }
            // No complete event found
            HttpResponse::Ok().json(serde_json::json!({}))
        }
        Err(_) => HttpResponse::Ok().json(serde_json::json!({})),
    }
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
            let mut used_fallback = false;

            // If no events found in the requested range, try to fall back to earlier data
            if events.is_empty() && end_ns.is_some() {
                // Try to find the most recent data before the requested time
                // Go back up to 7 days (604800 seconds)
                let end_time = end_ns.unwrap();
                let fallback_start = end_time - (7 * 24 * 3600 * 1_000_000_000i128);

                // Read from fallback_start to requested end time
                if let Ok(fallback_events) = reader.read_time_range(Some(fallback_start), Some(end_time)) {
                    // Take the most recent events before the requested time
                    events = fallback_events;
                    used_fallback = !events.is_empty();
                }
            }

            // Look back to find missing static/semi-static fields (metadata)
            let metadata = if let Some(end_time) = end_ns {
                find_missing_metadata(&reader, &events, end_time)
            } else {
                serde_json::json!({})
            };

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
                "fallback": used_fallback,
                "metadata": metadata,
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to read events: {}", e),
        })),
    }
}

/// Look back up to 24 hours to find the most recent values for missing static/semi-static fields
fn find_missing_metadata(reader: &IndexedReader, events: &[Event], end_time_ns: i128) -> serde_json::Value {
    // Check which fields are missing from the events in the requested range
    let mut has_kernel = false;
    let mut has_cpu_model = false;
    let mut has_cpu_mhz = false;
    let mut has_mem_total = false;
    let mut has_swap_total = false;
    let mut has_disk_total = false;
    let mut has_filesystems = false;
    let mut has_net_interface = false;
    let mut has_fans = false;

    for event in events {
        if let Event::SystemMetrics(m) = event {
            if m.kernel_version.is_some() { has_kernel = true; }
            if m.cpu_model.is_some() { has_cpu_model = true; }
            if m.cpu_mhz.is_some() { has_cpu_mhz = true; }
            if m.mem_total_bytes.is_some() { has_mem_total = true; }
            if m.swap_total_bytes.is_some() { has_swap_total = true; }
            if m.disk_total_bytes.is_some() { has_disk_total = true; }
            if m.filesystems.is_some() { has_filesystems = true; }
            if m.net_interface.is_some() { has_net_interface = true; }
            if m.fans.is_some() { has_fans = true; }
        }
    }

    // If all fields are present, no need to look back
    if has_kernel && has_cpu_model && has_cpu_mhz && has_mem_total && has_swap_total &&
       has_disk_total && has_filesystems && has_net_interface && has_fans {
        return serde_json::json!({});
    }

    // Look back up to 24 hours to find missing fields
    let lookback_start = end_time_ns - (24 * 3600 * 1_000_000_000i128);
    let lookback_events = reader.read_time_range(Some(lookback_start), Some(end_time_ns))
        .unwrap_or_default();

    // Scan backwards (most recent first) to find missing fields
    let mut metadata = serde_json::json!({});
    for event in lookback_events.iter().rev() {
        if let Event::SystemMetrics(m) = event {
            if !has_kernel && m.kernel_version.is_some() {
                metadata["kernel_version"] = serde_json::json!(m.kernel_version);
                has_kernel = true;
            }
            if !has_cpu_model && m.cpu_model.is_some() {
                metadata["cpu_model"] = serde_json::json!(m.cpu_model);
                has_cpu_model = true;
            }
            if !has_cpu_mhz && m.cpu_mhz.is_some() {
                metadata["cpu_mhz"] = serde_json::json!(m.cpu_mhz);
                has_cpu_mhz = true;
            }
            if !has_mem_total && m.mem_total_bytes.is_some() {
                metadata["mem_total_bytes"] = serde_json::json!(m.mem_total_bytes);
                has_mem_total = true;
            }
            if !has_swap_total && m.swap_total_bytes.is_some() {
                metadata["swap_total_bytes"] = serde_json::json!(m.swap_total_bytes);
                has_swap_total = true;
            }
            if !has_disk_total && m.disk_total_bytes.is_some() {
                metadata["disk_total_bytes"] = serde_json::json!(m.disk_total_bytes);
                has_disk_total = true;
            }
            if !has_filesystems && m.filesystems.is_some() {
                let filesystems: Vec<_> = m.filesystems.as_ref().unwrap().iter().map(|fs| serde_json::json!({
                    "filesystem": fs.filesystem,
                    "mount": fs.mount_point,
                    "total": fs.total_bytes,
                    "used": fs.used_bytes,
                    "available": fs.available_bytes,
                })).collect();
                metadata["filesystems"] = serde_json::json!(filesystems);
                has_filesystems = true;
            }
            if !has_net_interface && m.net_interface.is_some() {
                metadata["net_interface"] = serde_json::json!(m.net_interface);
                has_net_interface = true;
            }
            if !has_fans && m.fans.is_some() {
                let fans: Vec<_> = m.fans.as_ref().unwrap().iter().map(|f| serde_json::json!({
                    "label": f.label,
                    "rpm": f.rpm,
                })).collect();
                metadata["fans"] = serde_json::json!(fans);
                has_fans = true;
            }

            // Stop early if all fields found
            if has_kernel && has_cpu_model && has_cpu_mhz && has_mem_total && has_swap_total &&
               has_disk_total && has_filesystems && has_net_interface && has_fans {
                break;
            }
        }
    }

    metadata
}

fn format_event_for_api(event: &Event) -> serde_json::Value {
    match event {
        Event::SystemMetrics(m) => {
            // Percentages are now calculated every second in main.rs using cached totals

            serde_json::json!({
                "type": "SystemMetrics",
                "timestamp": m.ts.unix_timestamp_nanos() / 1_000_000,  // Convert to milliseconds
                "kernel": m.kernel_version,
                "cpu_model": m.cpu_model,
                "cpu_mhz": m.cpu_mhz,
                "system_uptime_seconds": m.system_uptime_seconds,
                "cpu": m.cpu_usage_percent,
                "per_core_cpu": m.per_core_usage,
                "mem": m.mem_usage_percent,
                "mem_used": m.mem_used_bytes,
                "mem_total": m.mem_total_bytes,
                "swap": m.swap_usage_percent,
                "swap_used": m.swap_used_bytes,
                "swap_total": m.swap_total_bytes,
                "load": m.load_avg_1m,
                "load5": m.load_avg_5m,
                "load15": m.load_avg_15m,
                "disk": m.disk_usage_percent.round(),
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
                "filesystems": m.filesystems.as_ref().map(|fs_list| fs_list.iter().map(|fs| serde_json::json!({
                    "filesystem": fs.filesystem,
                    "mount": fs.mount_point,
                    "total": fs.total_bytes,
                    "used": fs.used_bytes,
                    "available": fs.available_bytes,
                })).collect::<Vec<_>>()).unwrap_or_default(),
                "users": m.logged_in_users.as_ref().map(|user_list| user_list.iter().map(|u| serde_json::json!({
                    "username": u.username,
                    "terminal": u.terminal,
                    "remote_host": u.remote_host,
                })).collect::<Vec<_>>()).unwrap_or_default(),
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
                "fans": m.fans.as_ref().map(|fan_list| fan_list.iter().map(|f| serde_json::json!({
                    "label": f.label,
                    "rpm": f.rpm,
                })).collect::<Vec<_>>()).unwrap_or_default(),
            })
        }
        Event::ProcessLifecycle(p) => serde_json::json!({
            "type": "ProcessLifecycle",
            "timestamp": p.ts.unix_timestamp_nanos() / 1_000_000,  // Convert to milliseconds
            "kind": format!("{:?}", p.kind),
            "pid": p.pid,
            "name": p.name,
        }),
        Event::ProcessSnapshot(p) => serde_json::json!({
            "type": "ProcessSnapshot",
            "timestamp": p.ts.unix_timestamp_nanos() / 1_000_000,  // Convert to milliseconds
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
            "timestamp": s.ts.unix_timestamp_nanos() / 1_000_000,  // Convert to milliseconds
            "kind": format!("{:?}", s.kind),
            "user": s.user,
            "source_ip": s.source_ip,
            "message": s.message,
        }),
        Event::Anomaly(a) => serde_json::json!({
            "type": "Anomaly",
            "timestamp": a.ts.unix_timestamp_nanos() / 1_000_000,  // Convert to milliseconds
            "severity": format!("{:?}", a.severity),
            "kind": format!("{:?}", a.kind),
            "message": a.message,
        }),
    }
}
