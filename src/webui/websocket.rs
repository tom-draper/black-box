use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::sync::Arc;
use std::time::{Duration, Instant};
use time::OffsetDateTime;
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

// WebSocket actor that streams events to connected clients
pub struct WsSession {
    hb: Instant,
    broadcaster: Arc<EventBroadcaster>,
    metadata: Arc<std::sync::RwLock<Option<crate::event::Metadata>>>,
}

impl WsSession {
    fn new(broadcaster: Arc<EventBroadcaster>, metadata: Arc<std::sync::RwLock<Option<crate::event::Metadata>>>) -> Self {
        Self {
            hb: Instant::now(),
            broadcaster,
            metadata,
        }
    }

    // Start heartbeat process to detect disconnections
    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // Check client heartbeat
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("{} WebSocket client heartbeat failed, disconnecting", now_timestamp());
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }

    // Start event streaming from broadcaster (event-driven, not polling!)
    fn start_event_stream(&self, ctx: &mut ws::WebsocketContext<Self>) {
        let rx = self.broadcaster.subscribe();

        // Wrap the broadcast receiver in a stream and add it to the actor
        // This is event-driven: we only process when events arrive (no polling!)
        let stream = BroadcastStream::new(rx);

        ctx.add_stream(stream);
    }

}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("{} WebSocket client connected", now_timestamp());

        // Send metadata as first message (just for populating caches, no render)
        if let Ok(guard) = self.metadata.read() {
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
                if let Ok(json_str) = serde_json::to_string(&metadata_msg) {
                    ctx.text(json_str);
                }
            } else {
                eprintln!("[WEBSOCKET] WARNING: No metadata available!");
            }
        } else {
            eprintln!("[WEBSOCKET] ERROR: Failed to read metadata lock!");
        }

        self.start_heartbeat(ctx);
        self.start_event_stream(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        println!("{} WebSocket client disconnected", now_timestamp());
    }
}

// Handle incoming WebSocket protocol messages from the client
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(_text)) => {
                // Ignore text messages from client (we only push events)
            }
            Ok(ws::Message::Binary(_)) => {
                // Ignore binary messages
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

// Handle incoming events from the broadcast stream (event-driven, not polling!)
impl StreamHandler<Result<crate::event::Event, tokio_stream::wrappers::errors::BroadcastStreamRecvError>> for WsSession {
    fn handle(&mut self, msg: Result<crate::event::Event, tokio_stream::wrappers::errors::BroadcastStreamRecvError>, ctx: &mut Self::Context) {
        match msg {
            Ok(event) => {
                // Serialize and send event
                match event_to_json_string(&event) {
                    Ok(json) => ctx.text(json),
                    Err(e) => {
                        eprintln!("Failed to serialize event: {}", e);
                    }
                }
            }
            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(skipped)) => {
                eprintln!("{} WebSocket client lagged, skipped {} events", now_timestamp(), skipped);
                // Continue receiving, don't stop
            }
        }
    }
}

// WebSocket handler endpoint
pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    broadcaster: web::Data<EventBroadcaster>,
    metadata: web::Data<std::sync::RwLock<Option<crate::event::Metadata>>>,
) -> Result<HttpResponse, Error> {
    let metadata_arc = Arc::clone(&metadata.into_inner());
    let session = WsSession::new(Arc::new(broadcaster.get_ref().clone()), metadata_arc);
    ws::start(session, &req, stream)
}

// Optimized: Serialize event directly to JSON string
fn event_to_json_string(event: &crate::event::Event) -> Result<String, serde_json::Error> {
    // Convert to serde_json::Value then serialize (optimized with pre-sized allocations)
    serde_json::to_string(&event_to_json(event))
}

// Convert Event to JSON format (same as API) - kept for large events
fn event_to_json(event: &crate::event::Event) -> serde_json::Value {
    use crate::event::Event;

    match event {
        Event::SystemMetrics(m) => {
            // Percentages are now calculated every second in main.rs using cached totals

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
