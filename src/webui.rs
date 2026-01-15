use anyhow::Result;
use std::sync::Arc;
use tiny_http::{Response, Server};

use crate::event::*;
use crate::reader::LogReader;

pub fn start_server(data_dir: String, port: u16) -> Result<()> {
    let server = Server::http(format!("0.0.0.0:{}", port))
        .map_err(|e| anyhow::anyhow!("Failed to start server: {}", e))?;
    let reader = Arc::new(LogReader::new(data_dir));

    for request in server.incoming_requests() {
        let reader = Arc::clone(&reader);
        let url = request.url().to_string();

        let response = if url == "/" || url.starts_with("/?") {
            handle_index(&reader, &url)
        } else if url.starts_with("/api/events") {
            handle_api_events(&reader, &url)
        } else {
            Response::from_string("404 Not Found").with_status_code(404)
        };

        let _ = request.respond(response);
    }

    Ok(())
}

fn handle_index(_reader: &LogReader, url: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    // Parse query params for time filtering
    let query_params = parse_query_string(url);
    let filter = query_params.get("filter").map(|s| s.as_str()).unwrap_or("");

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Black Box - Server Forensics</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            background: #0a0a0a;
            color: #d0d0d0;
            font-family: 'Courier New', Courier, monospace;
            font-size: 13px;
            line-height: 1.4;
            padding: 20px;
        }}

        .header {{
            border-bottom: 1px solid #333;
            padding-bottom: 15px;
            margin-bottom: 20px;
        }}

        h1 {{
            color: #00ff00;
            font-size: 18px;
            font-weight: normal;
            margin-bottom: 5px;
        }}

        .status {{
            color: #888;
            font-size: 12px;
        }}

        .controls {{
            background: #111;
            border: 1px solid #333;
            padding: 10px;
            margin-bottom: 15px;
        }}

        .controls input, .controls select, .controls button {{
            background: #1a1a1a;
            border: 1px solid #444;
            color: #d0d0d0;
            padding: 5px 10px;
            font-family: 'Courier New', Courier, monospace;
            font-size: 12px;
            margin-right: 10px;
        }}

        .controls button {{
            cursor: pointer;
            background: #2a2a2a;
        }}

        .controls button:hover {{
            background: #3a3a3a;
            border-color: #666;
        }}

        .log-container {{
            background: #0f0f0f;
            border: 1px solid #333;
            padding: 15px;
            overflow-x: auto;
        }}

        .log-entry {{
            margin-bottom: 8px;
            font-size: 12px;
        }}

        .timestamp {{
            color: #666;
        }}

        .event-system {{
            color: #00aaff;
        }}

        .event-process {{
            color: #ffaa00;
        }}

        .event-security {{
            color: #ff00ff;
        }}

        .event-anomaly {{
            color: #ff0000;
        }}

        .metric {{
            color: #00ff00;
        }}

        .warning {{
            color: #ffff00;
        }}

        .error {{
            color: #ff5555;
        }}

        .info {{
            color: #55aaff;
        }}

        .success {{
            color: #55ff55;
        }}

        .separator {{
            color: #333;
            margin: 5px 0;
        }}

        .loading {{
            color: #888;
            text-align: center;
            padding: 20px;
        }}

        .footer {{
            margin-top: 20px;
            padding-top: 15px;
            border-top: 1px solid #333;
            color: #666;
            font-size: 11px;
            text-align: center;
        }}
    </style>
</head>
<body>
    <div class="header">
        <h1>BLACK-BOX :: Server Forensics Recorder</h1>
        <div class="status">Status: <span class="success">ACTIVE</span> | Auto-refresh: ON</div>
    </div>

    <div class="controls">
        <label>Filter: <input type="text" id="filterInput" value="{filter}" placeholder="search events..."></label>
        <select id="eventType">
            <option value="">All Events</option>
            <option value="system">System Metrics</option>
            <option value="process">Process Events</option>
            <option value="security">Security Events</option>
            <option value="anomaly">Anomalies</option>
        </select>
        <button onclick="loadEvents()">Refresh</button>
        <button onclick="clearFilter()">Clear</button>
        <label style="margin-left: 20px;">
            <input type="checkbox" id="autoRefresh" checked> Auto-refresh (5s)
        </label>
    </div>

    <div class="log-container" id="logContainer">
        <div class="loading">Loading events...</div>
    </div>

    <div class="footer">
        Black Box Forensics | Data Directory: ./data | Ring Buffer: ~100MB
    </div>

    <script>
        let autoRefreshInterval = null;

        async function loadEvents() {{
            const filter = document.getElementById('filterInput').value;
            const eventType = document.getElementById('eventType').value;

            try {{
                const params = new URLSearchParams();
                if (filter) params.append('filter', filter);
                if (eventType) params.append('type', eventType);

                const response = await fetch('/api/events?' + params.toString());
                const events = await response.json();

                renderEvents(events);
            }} catch (err) {{
                document.getElementById('logContainer').innerHTML =
                    '<div class="error">Error loading events: ' + err.message + '</div>';
            }}
        }}

        function renderEvents(events) {{
            const container = document.getElementById('logContainer');

            if (events.length === 0) {{
                container.innerHTML = '<div class="info">No events found</div>';
                return;
            }}

            let html = '';

            for (const event of events) {{
                html += formatEvent(event);
            }}

            container.innerHTML = html;
            container.scrollTop = container.scrollHeight;
        }}

        function formatEvent(event) {{
            const ts = event.timestamp || '';
            const type = event.type || 'unknown';

            let line = '<div class="log-entry">';
            line += '<span class="timestamp">[' + ts + ']</span> ';

            if (type === 'SystemMetrics') {{
                line += '<span class="event-system">[SYSTEM]</span> ';
                line += '<span class="metric">';
                line += 'CPU:' + event.cpu.toFixed(1) + '% ';
                line += 'Mem:' + event.mem.toFixed(1) + '% ';
                line += 'Load:' + event.load.toFixed(2) + ' ';
                line += 'Disk:' + event.disk + '% ';
                line += 'TCP:' + event.tcp;
                line += '</span>';
            }} else if (type === 'ProcessLifecycle') {{
                line += '<span class="event-process">[PROCESS]</span> ';
                if (event.kind === 'Started') {{
                    line += '<span class="success">[+]</span> ';
                }} else if (event.kind === 'Exited') {{
                    line += '<span class="info">[-]</span> ';
                }} else if (event.kind === 'Stuck') {{
                    line += '<span class="warning">[D]</span> ';
                }} else if (event.kind === 'Zombie') {{
                    line += '<span class="warning">[Z]</span> ';
                }}
                line += event.name + ' (pid ' + event.pid + ')';
            }} else if (type === 'SecurityEvent') {{
                line += '<span class="event-security">[SECURITY]</span> ';
                if (event.kind === 'SshLoginSuccess') {{
                    line += '<span class="success">[SSH OK]</span> ';
                }} else if (event.kind === 'SshLoginFailure') {{
                    line += '<span class="error">[SSH FAIL]</span> ';
                }} else if (event.kind === 'SudoCommand') {{
                    line += '<span class="warning">[SUDO]</span> ';
                }}
                line += event.user;
                if (event.source_ip) {{
                    line += ' from ' + event.source_ip;
                }}
            }} else if (type === 'Anomaly') {{
                line += '<span class="event-anomaly">[ANOMALY]</span> ';
                if (event.severity === 'Critical') {{
                    line += '<span class="error">[CRITICAL]</span> ';
                }} else if (event.severity === 'Warning') {{
                    line += '<span class="warning">[WARNING]</span> ';
                }}
                line += event.message;
            }} else if (type === 'ProcessSnapshot') {{
                line += '<span class="event-process">[SNAPSHOT]</span> ';
                line += 'Top ' + event.count + ' processes recorded';
            }}

            line += '</div>';
            return line;
        }}

        function clearFilter() {{
            document.getElementById('filterInput').value = '';
            document.getElementById('eventType').value = '';
            loadEvents();
        }}

        function setupAutoRefresh() {{
            const checkbox = document.getElementById('autoRefresh');

            if (checkbox.checked) {{
                autoRefreshInterval = setInterval(loadEvents, 5000);
            }} else {{
                if (autoRefreshInterval) {{
                    clearInterval(autoRefreshInterval);
                    autoRefreshInterval = null;
                }}
            }}
        }}

        document.getElementById('autoRefresh').addEventListener('change', setupAutoRefresh);
        document.getElementById('filterInput').addEventListener('keyup', (e) => {{
            if (e.key === 'Enter') loadEvents();
        }});

        // Initial load
        loadEvents();
        setupAutoRefresh();
    </script>
</body>
</html>"#,
        filter = filter
    );

    Response::from_string(html)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                .unwrap(),
        )
}

fn handle_api_events(reader: &LogReader, url: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let query_params = parse_query_string(url);
    let filter = query_params.get("filter").map(|s| s.to_lowercase());
    let event_type = query_params.get("type").map(|s| s.as_str());

    let events = match reader.read_all_events() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error reading events: {}", e);
            return Response::from_string(format!(r#"{{"error": "Failed to read events: {}"}}"#, e))
                .with_status_code(500)
                .with_header(
                    tiny_http::Header::from_bytes(
                        &b"Content-Type"[..],
                        &b"application/json"[..],
                    )
                    .unwrap(),
                );
        }
    };

    // Convert to JSON-serializable format
    let mut json_events = Vec::new();

    for event in events.iter().rev().take(1000) {
        if let Some(json_event) = event_to_json(event, &filter, event_type) {
            json_events.push(json_event);
        }
    }

    json_events.reverse();

    let json = serde_json::to_string(&json_events).unwrap_or_else(|_| "[]".to_string());

    Response::from_string(json).with_header(
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
    )
}

fn event_to_json(
    event: &Event,
    filter: &Option<String>,
    event_type_filter: Option<&str>,
) -> Option<serde_json::Value> {
    use time::format_description::well_known::Rfc3339;

    match event {
        Event::SystemMetrics(m) => {
            if event_type_filter.is_some() && event_type_filter != Some("system") {
                return None;
            }

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

            Some(serde_json::json!({
                "type": "SystemMetrics",
                "timestamp": m.ts.format(&Rfc3339).ok()?,
                "cpu": m.cpu_usage_percent,
                "mem": mem_pct,
                "load": m.load_avg_1m,
                "disk": disk_pct.round(),
                "tcp": m.tcp_connections,
            }))
        }
        Event::ProcessLifecycle(p) => {
            if event_type_filter.is_some() && event_type_filter != Some("process") {
                return None;
            }

            let text = format!("{:?} {} {}", p.kind, p.name, p.pid);
            if let Some(f) = filter {
                if !text.to_lowercase().contains(f) {
                    return None;
                }
            }

            Some(serde_json::json!({
                "type": "ProcessLifecycle",
                "timestamp": p.ts.format(&Rfc3339).ok()?,
                "kind": format!("{:?}", p.kind),
                "pid": p.pid,
                "name": p.name,
            }))
        }
        Event::SecurityEvent(s) => {
            if event_type_filter.is_some() && event_type_filter != Some("security") {
                return None;
            }

            let text = format!("{} {} {:?}", s.user, s.message, s.kind);
            if let Some(f) = filter {
                if !text.to_lowercase().contains(f) {
                    return None;
                }
            }

            Some(serde_json::json!({
                "type": "SecurityEvent",
                "timestamp": s.ts.format(&Rfc3339).ok()?,
                "kind": format!("{:?}", s.kind),
                "user": s.user,
                "source_ip": s.source_ip,
                "message": s.message,
            }))
        }
        Event::Anomaly(a) => {
            if event_type_filter.is_some() && event_type_filter != Some("anomaly") {
                return None;
            }

            let text = format!("{:?} {}", a.kind, a.message);
            if let Some(f) = filter {
                if !text.to_lowercase().contains(f) {
                    return None;
                }
            }

            Some(serde_json::json!({
                "type": "Anomaly",
                "timestamp": a.ts.format(&Rfc3339).ok()?,
                "severity": format!("{:?}", a.severity),
                "kind": format!("{:?}", a.kind),
                "message": a.message,
            }))
        }
        Event::ProcessSnapshot(p) => {
            if event_type_filter.is_some() && event_type_filter != Some("process") {
                return None;
            }

            Some(serde_json::json!({
                "type": "ProcessSnapshot",
                "timestamp": p.ts.format(&Rfc3339).ok()?,
                "count": p.processes.len(),
            }))
        }
    }
}

fn parse_query_string(url: &str) -> std::collections::HashMap<String, String> {
    let mut params = std::collections::HashMap::new();

    if let Some(query) = url.split('?').nth(1) {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(
                    urlencoding::decode(key).unwrap_or_default().to_string(),
                    urlencoding::decode(value).unwrap_or_default().to_string(),
                );
            }
        }
    }

    params
}

// Simple URL decoding since we don't have a dependency
mod urlencoding {
    pub fn decode(s: &str) -> Option<String> {
        let mut result = String::new();
        let mut chars = s.chars();

        while let Some(c) = chars.next() {
            match c {
                '%' => {
                    let hex: String = chars.by_ref().take(2).collect();
                    if hex.len() == 2 {
                        if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                            result.push(byte as char);
                        }
                    }
                }
                '+' => result.push(' '),
                _ => result.push(c),
            }
        }

        Some(result)
    }
}
