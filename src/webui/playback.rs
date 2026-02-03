// Playback API - provides two query modes for retrieving historical events:
//
// Mode 1: Count-based (?timestamp=T&count=N)
//   - Returns the last N SystemMetrics events before timestamp T
//   - Useful for playback UI where you always want exactly N data points
//   - Server automatically searches backward to find enough events
//
// Mode 2: Range-based (?start=S&end=E&limit=L)
//   - Returns all events between timestamps S and E (up to L total events)
//   - Useful for export, analysis, or when you need events in a specific timeframe
//   - Returns whatever events exist in that range (may be less than limit)

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use std::sync::Arc;
use time::OffsetDateTime;

use crate::indexed_reader::IndexedReader;
use crate::reader::LogReader;
use crate::event::Event;

#[derive(Deserialize)]
pub struct PlaybackQuery {
    // Mode 1: Get last N SystemMetrics before a timestamp
    // Usage: ?timestamp=1234567890&count=60
    #[serde(rename = "timestamp")]
    timestamp: Option<i64>,  // Unix seconds - the playback position
    #[serde(rename = "count")]
    count: Option<usize>,    // Number of SystemMetrics to return (default: 60)
    #[serde(rename = "before")]
    before: Option<bool>,    // If true, fetch events BEFORE timestamp (for progressive loading)

    // Mode 2: Get all events in a time range
    // Usage: ?start=1234567890&end=1234567950&limit=200
    #[serde(rename = "start")]
    start_timestamp: Option<i64>,  // Unix seconds - range start
    #[serde(rename = "end")]
    end_timestamp: Option<i64>,    // Unix seconds - range end
    #[serde(rename = "limit")]
    limit: Option<usize>,          // Max total events to return
}

/// Get the most recent complete SystemMetrics (with static/semi-static fields) for page initialization
/// Uses LogReader to read the most recent segment file (avoids old incompatible segments)
pub async fn api_initial_state(
    reader: web::Data<LogReader>,
) -> HttpResponse {
    match reader.read_recent_segment() {
        Ok(events) => {
            // Try to find the most recent SystemMetrics with filesystems first
            for event in events.iter().rev() {
                if let Event::SystemMetrics(m) = event {
                    if m.filesystems.is_some() {
                        return HttpResponse::Ok().json(format_event_for_api(event));
                    }
                }
            }

            // If no event with filesystems, return the most recent SystemMetrics anyway
            for event in events.iter().rev() {
                if let Event::SystemMetrics(_) = event {
                    return HttpResponse::Ok().json(format_event_for_api(event));
                }
            }

            HttpResponse::Ok().json(serde_json::json!({}))
        }
        Err(_) => {
            HttpResponse::Ok().json(serde_json::json!({}))
        }
    }
}

/// Mode 1: Fetch last N SystemMetrics before a timestamp
/// If `before` is true, fetch events strictly BEFORE the timestamp (for progressive loading)
async fn fetch_events_by_count(
    _log_reader: &LogReader,
    indexed_reader: &Arc<IndexedReader>,
    timestamp: i64,
    target_count: usize,
    before: bool,
) -> HttpResponse {
    // For normal mode: include events AT the requested timestamp
    // For before mode: get events strictly before the timestamp
    let end_ns = if before {
        (timestamp as i128) * 1_000_000_000  // Strictly before
    } else {
        ((timestamp + 1) as i128) * 1_000_000_000  // Include events at timestamp
    };

    // Progressive search: start with 90 seconds, expand if needed
    let search_durations = vec![90, 120, 180, 300]; // seconds to search back

    let mut all_events = Vec::new();
    let mut search_start_ns = end_ns;

    for duration in search_durations {
        search_start_ns = end_ns - ((duration as i128) * 1_000_000_000);

        // Always use IndexedReader - it efficiently reads only relevant segments
        let events_result = indexed_reader.read_time_range(Some(search_start_ns), Some(end_ns));

        match events_result {
            Ok(events) => {
                let sm_count = events.iter().filter(|e| matches!(e, Event::SystemMetrics(_))).count();

                if sm_count >= target_count {
                    all_events = events;
                    break;
                }

                // Keep searching if we don't have enough yet
                all_events = events;
            }
            Err(e) => {
                eprintln!("ERROR in fetch_events_by_count: Failed to read events: {}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to read events: {}", e),
                }));
            }
        }
    }

    // Extract SystemMetrics and take the last target_count
    let mut system_metrics: Vec<Event> = all_events.iter()
        .filter(|e| matches!(e, Event::SystemMetrics(_)))
        .cloned()
        .collect();

    let selected_metrics: Vec<Event> = if system_metrics.len() > target_count {
        system_metrics.split_off(system_metrics.len() - target_count)
    } else {
        system_metrics
    };

    // Get time range of selected metrics
    let (metrics_start_ns, metrics_end_ns) = if !selected_metrics.is_empty() {
        let first_ts = selected_metrics.first().unwrap().timestamp().unix_timestamp_nanos();
        let last_ts = selected_metrics.last().unwrap().timestamp().unix_timestamp_nanos();
        (first_ts, last_ts)
    } else {
        (search_start_ns, end_ns)
    };

    // Include all other events within the selected metrics timespan
    let other_events: Vec<Event> = all_events.into_iter()
        .filter(|e| {
            if matches!(e, Event::SystemMetrics(_)) {
                return false;
            }
            let ts = e.timestamp().unix_timestamp_nanos();
            ts >= metrics_start_ns && ts <= metrics_end_ns
        })
        .collect();

    // Combine and sort
    let mut final_events = selected_metrics;
    final_events.extend(other_events);
    final_events.sort_by_key(|e| e.timestamp().unix_timestamp_nanos());

    // Look back for metadata
    let metadata = find_missing_metadata(&indexed_reader, &final_events, end_ns);

    // Format for API
    let formatted_events: Vec<serde_json::Value> = final_events
        .iter()
        .map(format_event_for_api)
        .collect();

    HttpResponse::Ok().json(serde_json::json!({
        "count": formatted_events.len(),
        "events": formatted_events,
        "metadata": metadata,
    }))
}

/// Get time range metadata
pub async fn api_playback_info(
    reader: web::Data<Arc<IndexedReader>>,
) -> HttpResponse {
    // Refresh index to pick up any new segments written since server start
    let _ = reader.refresh();

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

/// Get event density timeline (events per minute) for visualization
pub async fn api_timeline(
    reader: web::Data<Arc<IndexedReader>>,
) -> HttpResponse {
    // Refresh index to pick up any new segments written since server start
    let _ = reader.refresh();

    if let Some((first_ns, last_ns)) = reader.get_time_range() {
        // Read all events (this might be expensive for very large datasets)
        match reader.read_time_range(Some(first_ns), Some(last_ns)) {
            Ok(events) => {
                // Bucket events by minute
                let first_minute = (first_ns / 60_000_000_000) as i64; // Convert ns to minutes
                let last_minute = (last_ns / 60_000_000_000) as i64;

                let mut buckets = std::collections::HashMap::new();
                let mut cpu_buckets: std::collections::HashMap<i64, Vec<f32>> = std::collections::HashMap::new();
                let mut mem_buckets: std::collections::HashMap<i64, Vec<f32>> = std::collections::HashMap::new();

                // Count events per minute and collect CPU/memory metrics
                for event in events.iter() {
                    let ts_ns = event.timestamp().unix_timestamp_nanos();
                    let minute = (ts_ns / 60_000_000_000) as i64;
                    *buckets.entry(minute).or_insert(0u32) += 1;

                    // Collect CPU and memory usage from SystemMetrics events
                    if let Event::SystemMetrics(m) = event {
                        cpu_buckets.entry(minute).or_insert_with(Vec::new).push(m.cpu_usage_percent);
                        mem_buckets.entry(minute).or_insert_with(Vec::new).push(m.mem_usage_percent);
                    }
                }

                // Build timeline array with all minutes (including empty ones for smooth visualization)
                let mut timeline = Vec::new();

                // Exclude the current incomplete minute to avoid misleading drop-off at the end
                let now_minute = (OffsetDateTime::now_utc().unix_timestamp() / 60) as i64;
                let effective_last_minute = if last_minute >= now_minute {
                    // Exclude current minute if it's incomplete
                    now_minute - 1
                } else {
                    last_minute
                };

                let total_minutes = (effective_last_minute - first_minute + 1) as usize;

                // If we have too many minutes (>500), downsample to keep response size reasonable
                let step = if total_minutes > 500 {
                    (total_minutes / 500).max(1)
                } else {
                    1
                };

                for minute in (first_minute..=effective_last_minute).step_by(step) {
                    // When downsampling, aggregate counts for the step range
                    let mut count = 0u32;
                    let mut cpu_values = Vec::new();
                    let mut mem_values = Vec::new();

                    for m in minute..(minute + step as i64).min(last_minute + 1) {
                        count += buckets.get(&m).copied().unwrap_or(0);
                        if let Some(cpus) = cpu_buckets.get(&m) {
                            cpu_values.extend_from_slice(cpus);
                        }
                        if let Some(mems) = mem_buckets.get(&m) {
                            mem_values.extend_from_slice(mems);
                        }
                    }

                    // Calculate averages
                    let cpu_avg = if !cpu_values.is_empty() {
                        Some(cpu_values.iter().sum::<f32>() / cpu_values.len() as f32)
                    } else {
                        None
                    };
                    let mem_avg = if !mem_values.is_empty() {
                        Some(mem_values.iter().sum::<f32>() / mem_values.len() as f32)
                    } else {
                        None
                    };

                    timeline.push(serde_json::json!({
                        "timestamp": minute * 60, // Convert back to seconds
                        "count": count,
                        "cpu": cpu_avg,
                        "mem": mem_avg,
                    }));
                }

                HttpResponse::Ok().json(serde_json::json!({
                    "timeline": timeline,
                    "first_timestamp": (first_ns / 1_000_000_000) as i64,
                    "last_timestamp": effective_last_minute * 60, // Use effective last minute (excluding incomplete)
                }))
            }
            Err(e) => {
                eprintln!("Failed to read timeline: {}", e);
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to read timeline"
                }))
            }
        }
    } else {
        HttpResponse::Ok().json(serde_json::json!({
            "timeline": [],
            "first_timestamp": null,
            "last_timestamp": null,
        }))
    }
}

/// Get events for playback
///
/// Two modes supported:
/// 1. Count mode: ?timestamp=T&count=N - Get last N SystemMetrics before timestamp T
///    Add &before=true to get events BEFORE timestamp (for progressive loading)
/// 2. Range mode: ?start=S&end=E&limit=L - Get all events between S and E (up to L events)
pub async fn api_playback_events(
    log_reader: web::Data<LogReader>,
    indexed_reader: web::Data<Arc<IndexedReader>>,
    query: web::Query<PlaybackQuery>,
) -> HttpResponse {
    // Refresh index to pick up any new segments written since server start
    let _ = indexed_reader.refresh();

    // Mode 1: Count-based query (timestamp + count)
    if let Some(timestamp) = query.timestamp {
        let target_count = query.count.unwrap_or(60);
        let before = query.before.unwrap_or(false);
        return fetch_events_by_count(&log_reader, &indexed_reader, timestamp, target_count, before).await;
    }

    // Mode 2: Range-based query (start + end)
    fetch_events_by_range(&log_reader, &indexed_reader, &query).await
}

/// Mode 2: Fetch all events in a time range (start to end)
async fn fetch_events_by_range(
    _log_reader: &LogReader,
    indexed_reader: &Arc<IndexedReader>,
    query: &PlaybackQuery,
) -> HttpResponse {
    let start_ns = query.start_timestamp.map(|s| (s as i128) * 1_000_000_000);
    let end_ns = query.end_timestamp.map(|s| (s as i128) * 1_000_000_000);

    // Always use IndexedReader - it efficiently reads only relevant segments
    let events_result = indexed_reader.read_time_range(start_ns, end_ns);

    match events_result {
        Ok(mut events) => {
            let mut used_fallback = false;

            // If no SystemMetrics found in the requested range, try to fall back to earlier data
            if events.is_empty() && end_ns.is_some() {
                // Try to find the most recent data before the requested time
                // Go back up to 7 days (604800 seconds)
                let end_time = end_ns.unwrap();
                let fallback_start = end_time - (7 * 24 * 3600 * 1_000_000_000i128);

                // Read from fallback_start to requested end time (use IndexedReader for historical)
                if let Ok(fallback_events) = indexed_reader.read_time_range(Some(fallback_start), Some(end_time)) {
                    // Take the most recent events before the requested time
                    events = fallback_events;
                    used_fallback = !events.is_empty();
                }
            }

            // Look back to find missing static/semi-static fields (metadata)
            let metadata = if let Some(end_time) = end_ns {
                find_missing_metadata(&indexed_reader, &events, end_time)
            } else {
                serde_json::json!({})
            };

            // Apply limit if specified
            if let Some(limit) = query.limit {
                if events.len() > limit {
                    events = events.into_iter().rev().take(limit).rev().collect();
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
        Err(e) => {
            eprintln!("ERROR in fetch_events_by_range: Failed to read events: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to read events: {}", e),
            }))
        }
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
    let mut has_net_ip = false;
    let mut has_net_gateway = false;
    let mut has_net_dns = false;
    let mut has_fans = false;
    let mut has_processes = false;

    for event in events {
        if let Event::ProcessSnapshot(_) = event {
            has_processes = true;
        }
        if let Event::SystemMetrics(m) = event {
            if m.kernel_version.is_some() { has_kernel = true; }
            if m.cpu_model.is_some() { has_cpu_model = true; }
            if m.cpu_mhz.is_some() { has_cpu_mhz = true; }
            if m.mem_total_bytes.is_some() { has_mem_total = true; }
            if m.swap_total_bytes.is_some() { has_swap_total = true; }
            if m.disk_total_bytes.is_some() { has_disk_total = true; }
            if m.filesystems.is_some() { has_filesystems = true; }
            if m.net_interface.is_some() { has_net_interface = true; }
            if m.net_ip_address.is_some() { has_net_ip = true; }
            if m.net_gateway.is_some() { has_net_gateway = true; }
            if m.net_dns.is_some() { has_net_dns = true; }
            if m.fans.is_some() { has_fans = true; }
        }
    }

    // If all fields are present, no need to look back
    if has_kernel && has_cpu_model && has_cpu_mhz && has_mem_total && has_swap_total &&
       has_disk_total && has_filesystems && has_net_interface && has_net_ip &&
       has_net_gateway && has_net_dns && has_fans && has_processes {
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
                    "mount_point": fs.mount_point,
                    "total_bytes": fs.total_bytes,
                    "used_bytes": fs.used_bytes,
                    "available_bytes": fs.available_bytes,
                })).collect();
                metadata["filesystems"] = serde_json::json!(filesystems);
                has_filesystems = true;
            }
            if !has_net_interface && m.net_interface.is_some() {
                metadata["net_interface"] = serde_json::json!(m.net_interface);
                has_net_interface = true;
            }
            if !has_net_ip && m.net_ip_address.is_some() {
                metadata["net_ip"] = serde_json::json!(m.net_ip_address);
                has_net_ip = true;
            }
            if !has_net_gateway && m.net_gateway.is_some() {
                metadata["net_gateway"] = serde_json::json!(m.net_gateway);
                has_net_gateway = true;
            }
            if !has_net_dns && m.net_dns.is_some() {
                metadata["net_dns"] = serde_json::json!(m.net_dns);
                has_net_dns = true;
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
               has_disk_total && has_filesystems && has_net_interface && has_net_ip &&
               has_net_gateway && has_net_dns && has_fans && has_processes {
                break;
            }
        }
        if !has_processes {
            if let Event::ProcessSnapshot(p) = event {
                let processes: Vec<_> = p.processes.iter().map(|proc| serde_json::json!({
                    "pid": proc.pid,
                    "name": proc.name,
                    "cmdline": proc.cmdline,
                    "state": proc.state,
                    "user": proc.user,
                    "cpu_percent": proc.cpu_percent,
                    "mem_bytes": proc.mem_bytes,
                    "num_threads": proc.num_threads,
                })).collect();
                metadata["processes"] = serde_json::json!(processes);
                metadata["total_processes"] = serde_json::json!(p.total_processes);
                metadata["running_processes"] = serde_json::json!(p.running_processes);
                has_processes = true;

                // Stop early if all fields found
                if has_kernel && has_cpu_model && has_cpu_mhz && has_mem_total && has_swap_total &&
                   has_disk_total && has_filesystems && has_net_interface && has_net_ip &&
                   has_net_gateway && has_net_dns && has_fans && has_processes {
                    break;
                }
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
                    "mount_point": fs.mount_point,
                    "total_bytes": fs.total_bytes,
                    "used_bytes": fs.used_bytes,
                    "available_bytes": fs.available_bytes,
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
            "ppid": p.ppid,
            "name": p.name,
            "cmdline": p.cmdline,
            "working_dir": p.working_dir,
            "user": p.user,
            "uid": p.uid,
            "exit_code": p.exit_code,
        }),
        Event::ProcessSnapshot(p) => serde_json::json!({
            "type": "ProcessSnapshot",
            "timestamp": p.ts.unix_timestamp_nanos() / 1_000_000,  // Convert to milliseconds
            "count": p.processes.len(),
            "total_processes": p.total_processes,
            "running_processes": p.running_processes,
            "processes": p.processes.iter().map(|proc| serde_json::json!({
                "pid": proc.pid,
                "name": proc.name,
                "cmdline": proc.cmdline,
                "state": proc.state,
                "user": proc.user,
                "cpu_percent": proc.cpu_percent,
                "mem_bytes": proc.mem_bytes,
                "num_threads": proc.num_threads,
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
        Event::FileSystemEvent(fse) => serde_json::json!({
            "type": "FileSystemEvent",
            "timestamp": fse.ts.unix_timestamp_nanos() / 1_000_000, // ms
            "kind": format!("{:?}", fse.kind),
            "path": fse.path,
            "size": fse.size,
        }),
    }
}
