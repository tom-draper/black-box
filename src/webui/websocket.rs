use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::broadcast::EventBroadcaster;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

// WebSocket actor that streams events to connected clients
pub struct WsSession {
    hb: Instant,
    broadcaster: Arc<EventBroadcaster>,
}

impl WsSession {
    fn new(broadcaster: Arc<EventBroadcaster>) -> Self {
        Self {
            hb: Instant::now(),
            broadcaster,
        }
    }

    // Start heartbeat process to detect disconnections
    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // Check client heartbeat
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("WebSocket client heartbeat failed, disconnecting");
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }

    // Start event streaming from broadcaster
    fn start_event_stream(&self, ctx: &mut ws::WebsocketContext<Self>) {
        let mut rx = self.broadcaster.subscribe();

        ctx.run_interval(Duration::from_millis(50), move |_act, ctx| {
            // Try to receive multiple events per iteration (batch processing)
            let mut count = 0;
            while count < 10 {
                match rx.try_recv() {
                    Ok(event) => {
                        // Serialize event to JSON and send to client
                        match serde_json::to_string(&event_to_json(&event)) {
                            Ok(json) => ctx.text(json),
                            Err(e) => {
                                eprintln!("Failed to serialize event: {}", e);
                            }
                        }
                        count += 1;
                    }
                    Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::broadcast::error::TryRecvError::Lagged(skipped)) => {
                        eprintln!("WebSocket client lagged, skipped {} events", skipped);
                        break;
                    }
                    Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                        ctx.stop();
                        break;
                    }
                }
            }
        });
    }
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("WebSocket client connected");
        self.start_heartbeat(ctx);
        self.start_event_stream(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        println!("WebSocket client disconnected");
    }
}

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

// WebSocket handler endpoint
pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    broadcaster: web::Data<Arc<EventBroadcaster>>,
) -> Result<HttpResponse, Error> {
    let session = WsSession::new(broadcaster.get_ref().clone());
    ws::start(session, &req, stream)
}

// Convert Event to JSON format (same as API)
fn event_to_json(event: &crate::event::Event) -> serde_json::Value {
    use crate::event::Event;
    use time::format_description::well_known::Rfc3339;

    match event {
        Event::SystemMetrics(m) => {
            let disk_pct = if m.disk_total_bytes > 0 {
                (m.disk_used_bytes as f64 / m.disk_total_bytes as f64) * 100.0
            } else {
                0.0
            };

            let mem_pct = if m.mem_total_bytes > 0 {
                (m.mem_used_bytes as f64 / m.mem_total_bytes as f64) * 100.0
            } else {
                0.0
            };

            serde_json::json!({
                "type": "SystemMetrics",
                "timestamp": m.ts.format(&Rfc3339).unwrap_or_default(),
                "cpu": m.cpu_usage_percent,
                "mem": mem_pct,
                "load": m.load_avg_1m,
                "disk": disk_pct.round(),
                "tcp": m.tcp_connections,
            })
        }
        Event::ProcessLifecycle(p) => serde_json::json!({
            "type": "ProcessLifecycle",
            "timestamp": p.ts.format(&Rfc3339).unwrap_or_default(),
            "kind": format!("{:?}", p.kind),
            "pid": p.pid,
            "name": p.name,
        }),
        Event::SecurityEvent(s) => serde_json::json!({
            "type": "SecurityEvent",
            "timestamp": s.ts.format(&Rfc3339).unwrap_or_default(),
            "kind": format!("{:?}", s.kind),
            "user": s.user,
            "source_ip": s.source_ip,
            "message": s.message,
        }),
        Event::Anomaly(a) => serde_json::json!({
            "type": "Anomaly",
            "timestamp": a.ts.format(&Rfc3339).unwrap_or_default(),
            "severity": format!("{:?}", a.severity),
            "kind": format!("{:?}", a.kind),
            "message": a.message,
        }),
        Event::ProcessSnapshot(p) => serde_json::json!({
            "type": "ProcessSnapshot",
            "timestamp": p.ts.format(&Rfc3339).unwrap_or_default(),
            "count": p.processes.len(),
        }),
    }
}
