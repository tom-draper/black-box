use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::time::{interval, Duration};
use tokio_stream::wrappers::BroadcastStream;

use crate::broadcast::EventBroadcaster;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

// Format current time as HH:MM:SS.mmm
fn now_timestamp() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        now.hour(),
        now.minute(),
        now.second(),
        now.millisecond()
    )
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<super::routes::AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state.broadcaster, state.metadata))
}

async fn handle_socket(
    socket: WebSocket,
    broadcaster: Arc<EventBroadcaster>,
    metadata: Arc<std::sync::RwLock<Option<crate::event::Metadata>>>,
) {
    let (mut sender, mut receiver) = socket.split();

    println!("{} WebSocket client connected", now_timestamp());

    // Send metadata as first message
    // Serialize inside the lock scope, but send after dropping the lock
    let metadata_json = if let Ok(guard) = metadata.read() {
        if let Some(ref metadata) = *guard {
            eprintln!("[WEBSOCKET] Sending metadata: filesystems={}, processes={}, net_interface={:?}, net_ip={:?}",
                metadata.filesystems.as_ref().map(|fs| fs.len()).unwrap_or(0),
                metadata.processes.as_ref().map(|p| p.len()).unwrap_or(0),
                metadata.net_interface,
                metadata.net_ip_address);

            let metadata_msg = serde_json::json!({
                "type": "Metadata",
                "kernel": metadata.kernel_version,
                "cpu_model": metadata.cpu_model,
                "cpu_mhz": metadata.cpu_mhz,
                "mem_total": metadata.mem_total_bytes,
                "swap_total": metadata.swap_total_bytes,
                "disk_total": metadata.disk_total_bytes,
                "filesystems": metadata.filesystems,
                "net_interface": metadata.net_interface,
                "net_ip": metadata.net_ip_address,
                "net_gateway": metadata.net_gateway,
                "net_dns": metadata.net_dns,
                "fans": metadata.fans,
                "cpu_temp": metadata.temps.as_ref().and_then(|t| t.cpu_temp_celsius),
                "per_core_temps": metadata.temps.as_ref().map(|t| &t.per_core_temps),
                "gpu_temp": metadata.temps.as_ref().and_then(|t| t.gpu_temp_celsius),
                "mobo_temp": metadata.temps.as_ref().and_then(|t| t.motherboard_temp_celsius),
                "gpu_freq": metadata.gpu.as_ref().and_then(|g| g.gpu_freq_mhz),
                "gpu_mem_freq": metadata.gpu.as_ref().and_then(|g| g.mem_freq_mhz),
                "gpu_temp2": metadata.gpu.as_ref().and_then(|g| g.gpu_temp_celsius),
                "gpu_power": metadata.gpu.as_ref().and_then(|g| g.power_watts),
                "users": metadata.logged_in_users,
                "processes": metadata.processes,
                "total_processes": metadata.total_processes,
                "running_processes": metadata.running_processes,
            });
            serde_json::to_string(&metadata_msg).ok()
        } else {
            eprintln!("[WEBSOCKET] WARNING: No metadata available!");
            None
        }
    } else {
        eprintln!("[WEBSOCKET] ERROR: Failed to read metadata lock!");
        None
    };

    // Send after dropping the lock
    if let Some(json_str) = metadata_json {
        let _ = sender.send(Message::Text(json_str)).await;
    }

    // Subscribe to event broadcast
    let rx = broadcaster.subscribe();
    let mut event_stream = BroadcastStream::new(rx);

    // Set up heartbeat interval
    let mut heartbeat = interval(HEARTBEAT_INTERVAL);
    let mut last_pong = tokio::time::Instant::now();

    loop {
        tokio::select! {
            // Prioritize events over other branches with biased selection
            biased;

            // Handle events from broadcaster FIRST (highest priority)
            event = event_stream.next() => {
                match event {
                    Some(Ok(event)) => {
                        match event_to_json_string(&event) {
                            Ok(json) => {
                                if sender.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                                // Flush immediately for low latency
                                if sender.flush().await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to serialize event: {}", e);
                            }
                        }
                    }
                    Some(Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(skipped))) => {
                        eprintln!("{} WebSocket client lagged, skipped {} events", now_timestamp(), skipped);
                    }
                    None => break,
                }
            }

            // Handle incoming messages from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => {
                        last_pong = tokio::time::Instant::now();
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        last_pong = tokio::time::Instant::now();
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    _ => {}
                }
            }

            // Send heartbeat pings (lowest priority)
            _ = heartbeat.tick() => {
                // Check if client is still responding
                if last_pong.elapsed() > CLIENT_TIMEOUT {
                    println!("{} WebSocket client heartbeat failed, disconnecting", now_timestamp());
                    break;
                }

                if sender.send(Message::Ping(vec![])).await.is_err() {
                    break;
                }
            }
        }
    }

    println!("{} WebSocket client disconnected", now_timestamp());
}

// Optimized: Serialize event directly to JSON string
fn event_to_json_string(event: &crate::event::Event) -> Result<String, serde_json::Error> {
    serde_json::to_string(&event_to_json(event))
}

// Convert Event to JSON format (same as API)
fn event_to_json(event: &crate::event::Event) -> serde_json::Value {
    use crate::event::Event;

    match event {
        Event::SystemMetrics(m) => {
            // Pre-compute nested arrays outside json! macro
            let mut disks = Vec::with_capacity(m.per_disk_metrics.len());
            for d in &m.per_disk_metrics {
                disks.push(serde_json::json!({
                    "device": &d.device_name,
                    "read": d.read_bytes_per_sec,
                    "write": d.write_bytes_per_sec,
                    "temp": d.temp_celsius,
                }));
            }

            let filesystems = match &m.filesystems {
                Some(fs_list) => {
                    let mut filesystems = Vec::with_capacity(fs_list.len());
                    for fs in fs_list {
                        filesystems.push(serde_json::json!({
                            "filesystem": &fs.filesystem,
                            "mount_point": &fs.mount_point,
                            "total_bytes": fs.total_bytes,
                            "used_bytes": fs.used_bytes,
                            "available_bytes": fs.available_bytes,
                        }));
                    }
                    filesystems
                },
                None => Vec::new()
            };

            let users = match &m.logged_in_users {
                Some(user_list) => {
                    let mut users = Vec::with_capacity(user_list.len());
                    for u in user_list {
                        users.push(serde_json::json!({
                            "username": &u.username,
                            "terminal": &u.terminal,
                            "remote_host": &u.remote_host,
                        }));
                    }
                    users
                },
                None => Vec::new()
            };

            let fans = match &m.fans {
                Some(fan_list) => {
                    let mut fans = Vec::with_capacity(fan_list.len());
                    for f in fan_list {
                        fans.push(serde_json::json!({
                            "label": &f.label,
                            "rpm": f.rpm,
                        }));
                    }
                    fans
                },
                None => Vec::new()
            };

            let json_value = serde_json::json!({
                "type": "SystemMetrics",
                "timestamp": m.ts.unix_timestamp_nanos() / 1_000_000,
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
                "per_disk": disks,
                "filesystems": filesystems,
                "users": users,
                "net_interface": m.net_interface,
                "net_ip": m.net_ip_address,
                "net_gateway": m.net_gateway,
                "net_dns": m.net_dns,
                "net_recv": m.net_recv_bytes_per_sec,
                "net_send": m.net_send_bytes_per_sec,
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
                "fans": fans,
            });

            json_value
        }
        Event::ProcessLifecycle(p) => serde_json::json!({
            "type": "ProcessLifecycle",
            "timestamp": p.ts.unix_timestamp_nanos() / 1_000_000,
            "kind": format!("{:?}", p.kind),
            "pid": p.pid,
            "name": p.name,
            "cmdline": p.cmdline,
        }),
        Event::SecurityEvent(s) => serde_json::json!({
            "type": "SecurityEvent",
            "timestamp": s.ts.unix_timestamp_nanos() / 1_000_000,
            "kind": format!("{:?}", s.kind),
            "user": s.user,
            "source_ip": s.source_ip,
            "message": s.message,
        }),
        Event::Anomaly(a) => serde_json::json!({
            "type": "Anomaly",
            "timestamp": a.ts.unix_timestamp_nanos() / 1_000_000,
            "severity": format!("{:?}", a.severity),
            "kind": format!("{:?}", a.kind),
            "message": a.message,
        }),
        Event::ProcessSnapshot(p) => {
            let mut processes = Vec::with_capacity(p.processes.len());
            for proc in &p.processes {
                processes.push(serde_json::json!({
                    "pid": proc.pid,
                    "name": &proc.name,
                    "cmdline": &proc.cmdline,
                    "state": &proc.state,
                    "user": &proc.user,
                    "cpu_percent": proc.cpu_percent,
                    "mem_bytes": proc.mem_bytes,
                    "num_threads": proc.num_threads,
                }));
            }
            serde_json::json!({
                "type": "ProcessSnapshot",
                "timestamp": p.ts.unix_timestamp_nanos() / 1_000_000,
                "count": p.processes.len(),
                "total_processes": p.total_processes,
                "running_processes": p.running_processes,
                "processes": processes,
            })
        },
        Event::FileSystemEvent(f) => serde_json::json!({
            "type": "FileSystemEvent",
            "timestamp": f.ts.unix_timestamp_nanos() / 1_000_000,
            "kind": format!("{:?}", f.kind),
            "path": f.path
        }),
    }
}
