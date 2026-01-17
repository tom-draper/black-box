use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::event::Event;
use crate::reader::LogReader;

#[derive(Deserialize)]
pub struct EventQueryParams {
    filter: Option<String>,
    #[serde(rename = "type")]
    event_type: Option<String>,
}

pub async fn index() -> HttpResponse {
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Black Box - Server Forensics</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            background: #0a0a0a;
            color: #d0d0d0;
            font-family: 'Courier New', Courier, monospace;
            font-size: 13px;
            line-height: 1.4;
            padding: 20px;
        }

        .header {
            border-bottom: 1px solid #333;
            padding-bottom: 15px;
            margin-bottom: 20px;
        }

        h1 {
            color: #00ff00;
            font-size: 18px;
            font-weight: normal;
            margin-bottom: 5px;
        }

        .status {
            color: #888;
            font-size: 12px;
        }

        .controls {
            background: #111;
            border: 1px solid #333;
            padding: 10px;
            margin-bottom: 15px;
        }

        .controls input, .controls select, .controls button {
            background: #1a1a1a;
            border: 1px solid #444;
            color: #d0d0d0;
            padding: 5px 10px;
            font-family: 'Courier New', Courier, monospace;
            font-size: 12px;
            margin-right: 10px;
        }

        .controls button {
            cursor: pointer;
            background: #2a2a2a;
        }

        .controls button:hover {
            background: #3a3a3a;
            border-color: #666;
        }

        .log-container {
            background: #0f0f0f;
            border: 1px solid #333;
            padding: 15px;
            overflow-x: auto;
            max-height: calc(100vh - 650px);
            overflow-y: auto;
        }

        .log-entry {
            margin-bottom: 8px;
            font-size: 12px;
            color: #d0d0d0;
        }

        .log-entry.event-system {
            color: #00ff00;
        }

        .log-entry.event-process {
            color: #ffaa00;
        }

        .log-entry.event-security {
            color: #ff00ff;
        }

        .log-entry.event-anomaly {
            color: #ffff00;
        }

        .log-entry.event-anomaly.critical {
            color: #ff5555;
        }

        .graph-container {
            background: #0f0f0f;
            border: 1px solid #333;
            padding: 10px;
            margin-bottom: 15px;
        }

        .graph-header {
            color: #666;
            font-size: 10px;
            text-transform: uppercase;
            margin-bottom: 8px;
        }

        #metricsCanvas {
            width: 100%;
            height: 120px;
            display: block;
        }

        .footer {
            margin-top: 20px;
            padding-top: 15px;
            border-top: 1px solid #333;
            color: #666;
            font-size: 11px;
            text-align: center;
        }

        .connection-status {
            display: inline-block;
            padding: 2px 8px;
            border-radius: 3px;
            font-size: 11px;
        }

        .connection-status.connected {
            background: #1a5f1a;
            color: #55ff55;
        }

        .connection-status.disconnected {
            background: #5f1a1a;
            color: #ff5555;
        }

        .connection-status.connecting {
            background: #5f5f1a;
            color: #ffff55;
        }

        .stats-panel {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
            gap: 10px;
            margin-bottom: 15px;
        }

        .stat-box {
            background: #111;
            border: 1px solid #333;
            padding: 12px;
            display: flex;
            flex-direction: column;
        }

        .stat-label {
            color: #666;
            font-size: 10px;
            text-transform: uppercase;
            margin-bottom: 4px;
        }

        .stat-value {
            color: #00ff00;
            font-size: 20px;
            font-weight: bold;
        }

        .stat-value.warning {
            color: #ffff00;
        }

        .stat-value.critical {
            color: #ff5555;
        }

        .stat-detail {
            color: #555;
            font-size: 10px;
            margin-top: 4px;
        }

        .stat-bar {
            height: 4px;
            background: #222;
            margin-top: auto;
            padding-top: 6px;
            border-radius: 2px;
            overflow: hidden;
        }

        .stat-bar-fill {
            height: 100%;
            background: #00ff00;
            transition: width 0.3s ease;
        }

        .stat-bar-fill.warning {
            background: #ffff00;
        }

        .stat-bar-fill.critical {
            background: #ff5555;
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>BLACK-BOX :: Server Forensics Recorder</h1>
        <div class="status">
            Status: <span class="success">ACTIVE</span> |
            Connection: <span class="connection-status connecting" id="wsStatus">CONNECTING</span>
        </div>
    </div>

    <div class="stats-panel" id="statsPanel">
        <div class="stat-box">
            <div class="stat-label">CPU Usage</div>
            <div class="stat-value" id="statCpu">--</div>
            <div class="stat-detail" id="statCpuDetail">overall usage</div>
            <div class="stat-bar"><div class="stat-bar-fill" id="statCpuBar" style="width: 0%"></div></div>
        </div>
        <div class="stat-box">
            <div class="stat-label">Memory</div>
            <div class="stat-value" id="statMem">--</div>
            <div class="stat-detail" id="statMemDetail">-- / --</div>
            <div class="stat-bar"><div class="stat-bar-fill" id="statMemBar" style="width: 0%"></div></div>
        </div>
        <div class="stat-box">
            <div class="stat-label">Disk</div>
            <div class="stat-value" id="statDisk">--</div>
            <div class="stat-detail" id="statDiskDetail">-- / --</div>
            <div class="stat-bar"><div class="stat-bar-fill" id="statDiskBar" style="width: 0%"></div></div>
        </div>
        <div class="stat-box">
            <div class="stat-label">Load Average</div>
            <div class="stat-value" id="statLoad">--</div>
            <div class="stat-detail" id="statLoadDetail">1m / 5m / 15m</div>
        </div>
        <div class="stat-box">
            <div class="stat-label">Network I/O</div>
            <div class="stat-value" id="statNet">--</div>
            <div class="stat-detail" id="statNetDetail">recv / send</div>
        </div>
        <div class="stat-box">
            <div class="stat-label">TCP Connections</div>
            <div class="stat-value" id="statTcp">--</div>
            <div class="stat-detail" id="statTcpDetail">-- time_wait</div>
        </div>
        <div class="stat-box">
            <div class="stat-label">CPU Temperature</div>
            <div class="stat-value" id="statTemp">--</div>
            <div class="stat-detail" id="statTempDetail">sensors</div>
        </div>
    </div>

    <div class="graph-container">
        <div class="graph-header">Metrics History (60s)</div>
        <canvas id="metricsCanvas"></canvas>
    </div>

    <div class="controls">
        <label>Filter: <input type="text" id="filterInput" placeholder="search events..."></label>
        <select id="eventType">
            <option value="">All Events</option>
            <option value="system">System Metrics</option>
            <option value="process">Process Events</option>
            <option value="security">Security Events</option>
            <option value="anomaly">Anomalies</option>
        </select>
        <button onclick="clearFilter()">Clear</button>
    </div>

    <div class="log-container" id="logContainer">
        <div class="info">Connecting to event stream...</div>
    </div>

    <div class="footer">
        Black Box Forensics | WebSocket Real-Time Streaming | Ring Buffer: ~100MB
    </div>

    <script>
        let ws = null;
        let reconnectTimeout = null;
        let eventBuffer = [];
        const MAX_BUFFER_SIZE = 500;
        const MAX_DOM_SIZE = 300;
        const filterInput = document.getElementById('filterInput');
        const eventTypeSelect = document.getElementById('eventType');

        // Metrics history for graph
        const metricsHistory = {
            cpu: [],
            mem: [],
            disk: [],
            maxPoints: 60
        };

        // Counter for system metrics - only log every 10th reading
        let systemMetricsCounter = 0;

        const canvas = document.getElementById('metricsCanvas');
        const ctx = canvas.getContext('2d');

        // Set canvas size accounting for device pixel ratio
        function resizeCanvas() {
            const rect = canvas.getBoundingClientRect();
            canvas.width = rect.width * window.devicePixelRatio;
            canvas.height = rect.height * window.devicePixelRatio;
            ctx.scale(window.devicePixelRatio, window.devicePixelRatio);
            drawGraph();
        }

        window.addEventListener('resize', resizeCanvas);
        resizeCanvas();

        function connectWebSocket() {
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/ws`;

            updateConnectionStatus('connecting');
            ws = new WebSocket(wsUrl);

            ws.onopen = () => {
                console.log('WebSocket connected');
                updateConnectionStatus('connected');
                clearReconnectTimeout();
            };

            ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    addEvent(data);
                } catch (err) {
                    console.error('Failed to parse event:', err);
                }
            };

            ws.onerror = (error) => {
                console.error('WebSocket error:', error);
                updateConnectionStatus('disconnected');
            };

            ws.onclose = () => {
                console.warn('WebSocket closed, reconnecting...');
                updateConnectionStatus('disconnected');
                scheduleReconnect();
            };
        }

        function updateConnectionStatus(status) {
            const statusEl = document.getElementById('wsStatus');
            statusEl.className = `connection-status ${status}`;
            statusEl.textContent = status.toUpperCase();
        }

        function scheduleReconnect() {
            clearReconnectTimeout();
            reconnectTimeout = setTimeout(() => {
                console.log('Attempting to reconnect...');
                connectWebSocket();
            }, 5000);
        }

        function clearReconnectTimeout() {
            if (reconnectTimeout) {
                clearTimeout(reconnectTimeout);
                reconnectTimeout = null;
            }
        }

        function formatBytes(bytes) {
            if (bytes === 0) return '0 B';
            const k = 1024;
            const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
            const i = Math.floor(Math.log(bytes) / Math.log(k));
            return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
        }

        function getStatusClass(value, warnThreshold, critThreshold) {
            if (value >= critThreshold) return 'critical';
            if (value >= warnThreshold) return 'warning';
            return '';
        }

        function drawGraph() {
            const rect = canvas.getBoundingClientRect();
            const width = rect.width;
            const height = rect.height;

            // Clear canvas
            ctx.fillStyle = '#0f0f0f';
            ctx.fillRect(0, 0, width, height);

            // Draw grid lines
            ctx.strokeStyle = '#222';
            ctx.lineWidth = 1;
            for (let i = 0; i <= 4; i++) {
                const y = (height / 4) * i;
                ctx.beginPath();
                ctx.moveTo(0, y);
                ctx.lineTo(width, y);
                ctx.stroke();
            }

            // Draw percentage labels
            ctx.fillStyle = '#444';
            ctx.font = '9px monospace';
            ctx.textAlign = 'right';
            for (let i = 0; i <= 4; i++) {
                const pct = 100 - (i * 25);
                const y = (height / 4) * i + 3;
                ctx.fillText(pct + '%', width - 2, y);
            }

            if (metricsHistory.cpu.length < 2) return;

            const pointSpacing = width / (metricsHistory.maxPoints - 1);

            // Draw lines for each metric
            function drawLine(data, color) {
                ctx.strokeStyle = color;
                ctx.lineWidth = 1.5;
                ctx.beginPath();

                const startIdx = Math.max(0, data.length - metricsHistory.maxPoints);
                for (let i = 0; i < data.length - startIdx; i++) {
                    const value = data[startIdx + i];
                    const x = i * pointSpacing;
                    const y = height - (value / 100) * height;

                    if (i === 0) {
                        ctx.moveTo(x, y);
                    } else {
                        ctx.lineTo(x, y);
                    }
                }
                ctx.stroke();
            }

            drawLine(metricsHistory.cpu, '#00ff00');
            drawLine(metricsHistory.mem, '#00aaff');
            drawLine(metricsHistory.disk, '#ffaa00');

            // Draw legend
            ctx.font = '10px monospace';
            ctx.textAlign = 'left';
            ctx.fillStyle = '#00ff00';
            ctx.fillText('CPU', 5, 12);
            ctx.fillStyle = '#00aaff';
            ctx.fillText('MEM', 40, 12);
            ctx.fillStyle = '#ffaa00';
            ctx.fillText('DISK', 80, 12);
        }

        function updateStats(event) {
            if (event.type !== 'SystemMetrics') return;

            // Update metrics history
            metricsHistory.cpu.push(event.cpu);
            metricsHistory.mem.push(event.mem);
            metricsHistory.disk.push(event.disk);

            if (metricsHistory.cpu.length > metricsHistory.maxPoints) {
                metricsHistory.cpu.shift();
                metricsHistory.mem.shift();
                metricsHistory.disk.shift();
            }

            // Draw the graph
            drawGraph();

            // CPU
            const cpuEl = document.getElementById('statCpu');
            const cpuBar = document.getElementById('statCpuBar');
            const cpuClass = getStatusClass(event.cpu, 70, 90);
            cpuEl.textContent = event.cpu.toFixed(1) + '%';
            cpuEl.className = 'stat-value ' + cpuClass;
            cpuBar.style.width = event.cpu + '%';
            cpuBar.className = 'stat-bar-fill ' + cpuClass;

            // Memory
            const memEl = document.getElementById('statMem');
            const memBar = document.getElementById('statMemBar');
            const memDetail = document.getElementById('statMemDetail');
            const memClass = getStatusClass(event.mem, 80, 95);
            memEl.textContent = event.mem.toFixed(1) + '%';
            memEl.className = 'stat-value ' + memClass;
            memBar.style.width = event.mem + '%';
            memBar.className = 'stat-bar-fill ' + memClass;
            memDetail.textContent = formatBytes(event.mem_used) + ' / ' + formatBytes(event.mem_total);

            // Disk
            const diskEl = document.getElementById('statDisk');
            const diskBar = document.getElementById('statDiskBar');
            const diskDetail = document.getElementById('statDiskDetail');
            const diskClass = getStatusClass(event.disk, 80, 95);
            diskEl.textContent = event.disk + '%';
            diskEl.className = 'stat-value ' + diskClass;
            diskBar.style.width = event.disk + '%';
            diskBar.className = 'stat-bar-fill ' + diskClass;
            diskDetail.textContent = formatBytes(event.disk_used) + ' / ' + formatBytes(event.disk_total);

            // Load
            const loadEl = document.getElementById('statLoad');
            const loadDetail = document.getElementById('statLoadDetail');
            loadEl.textContent = event.load.toFixed(2);
            loadDetail.textContent = event.load.toFixed(2) + ' / ' + event.load5.toFixed(2) + ' / ' + event.load15.toFixed(2);

            // Network
            const netEl = document.getElementById('statNet');
            const netDetail = document.getElementById('statNetDetail');
            netEl.textContent = formatBytes(event.net_recv + event.net_send) + '/s';
            netDetail.textContent = formatBytes(event.net_recv) + '/s in / ' + formatBytes(event.net_send) + '/s out';

            // TCP
            const tcpEl = document.getElementById('statTcp');
            const tcpDetail = document.getElementById('statTcpDetail');
            tcpEl.textContent = event.tcp;
            tcpDetail.textContent = event.tcp_wait + ' time_wait';

            // Temperature
            const tempEl = document.getElementById('statTemp');
            const tempDetail = document.getElementById('statTempDetail');
            if (event.cpu_temp !== null && event.cpu_temp !== undefined) {
                const temp = event.cpu_temp;
                tempEl.textContent = temp.toFixed(1) + '°C';
                const tempClass = getStatusClass(temp, 70, 85);
                tempEl.className = 'stat-value ' + tempClass;

                // Build detail string with available temps
                let details = [];
                if (event.cpu_temp) details.push('CPU:' + event.cpu_temp.toFixed(0) + '°C');
                if (event.gpu_temp) details.push('GPU:' + event.gpu_temp.toFixed(0) + '°C');
                tempDetail.textContent = details.length > 0 ? details.join(' / ') : 'no sensors';
            } else {
                tempEl.textContent = '--';
                tempEl.className = 'stat-value';
                tempDetail.textContent = 'no sensors';
            }
        }

        function addEvent(event) {
            // Update stats panel for SystemMetrics
            updateStats(event);

            // For SystemMetrics, only add to log display every 10th reading
            if (event.type === 'SystemMetrics') {
                systemMetricsCounter++;
                if (systemMetricsCounter % 10 !== 0) {
                    // Still add to buffer for filtering purposes, but don't render
                    return;
                }
            }

            // Apply client-side filter
            const filter = filterInput.value.toLowerCase();
            const eventType = eventTypeSelect.value;

            if (!matchesFilter(event, filter, eventType)) {
                return;
            }

            // Add to buffer (ring buffer)
            eventBuffer.push(event);
            if (eventBuffer.length > MAX_BUFFER_SIZE) {
                eventBuffer.shift();
            }

            // Render event
            const container = document.getElementById('logContainer');
            const logEntry = createLogEntry(event);
            if (logEntry) {
                container.appendChild(logEntry);
            }

            // Auto-scroll to bottom
            container.scrollTop = container.scrollHeight;

            // Limit DOM size - keep it lower than buffer for better performance
            while (container.children.length > MAX_DOM_SIZE) {
                container.removeChild(container.firstChild);
            }
        }

        function matchesFilter(event, filter, eventType) {
            // Type filter
            if (eventType) {
                const typeMap = {
                    'system': 'SystemMetrics',
                    'process': 'ProcessLifecycle',
                    'security': 'SecurityEvent',
                    'anomaly': 'Anomaly'
                };
                const allowedTypes = Array.isArray(typeMap[eventType]) ? typeMap[eventType] : [typeMap[eventType]];
                if (!allowedTypes.includes(event.type)) {
                    return false;
                }
            }

            // Text filter
            if (filter) {
                const text = JSON.stringify(event).toLowerCase();
                if (!text.includes(filter)) {
                    return false;
                }
            }

            return true;
        }

        function createLogEntry(event) {
            const ts = event.timestamp || '';
            const type = event.type || 'unknown';

            // Filter out ProcessSnapshot and unknown event types from the log display
            if (type === 'ProcessSnapshot' || type === 'unknown') {
                return null;
            }

            const div = document.createElement('div');
            div.className = 'log-entry';

            let text = '[' + ts.substring(11, 23) + '] ';

            if (type === 'SystemMetrics') {
                text += '[SYSTEM] CPU:' + event.cpu.toFixed(1) + '% ';
                text += 'Mem:' + event.mem.toFixed(1) + '% ';
                text += 'Load:' + event.load.toFixed(2) + ' ';
                text += 'Disk:' + event.disk + '% ';
                text += 'TCP:' + event.tcp;
                div.className += ' event-system';
            } else if (type === 'ProcessLifecycle') {
                let symbol = '';
                if (event.kind === 'Started') symbol = '[+]';
                else if (event.kind === 'Exited') symbol = '[-]';
                else if (event.kind === 'Stuck') symbol = '[D]';
                else if (event.kind === 'Zombie') symbol = '[Z]';

                text += '[PROCESS] ' + symbol + ' ' + event.name + ' (pid ' + event.pid + ')';
                div.className += ' event-process';
            } else if (type === 'SecurityEvent') {
                let label = '';
                if (event.kind === 'SshLoginSuccess') label = '[SSH OK]';
                else if (event.kind === 'SshLoginFailure') label = '[SSH FAIL]';
                else if (event.kind === 'SudoCommand') label = '[SUDO]';

                text += '[SECURITY] ' + label + ' ' + event.user;
                if (event.source_ip) {
                    text += ' from ' + event.source_ip;
                }
                div.className += ' event-security';
            } else if (type === 'Anomaly') {
                let severity = event.severity === 'Critical' ? '[CRITICAL]' : '[WARNING]';
                text += '[ANOMALY] ' + severity + ' ' + event.message;
                div.className += ' event-anomaly';
                if (event.severity === 'Critical') {
                    div.className += ' critical';
                }
            } else {
                return null;
            }

            div.textContent = text;
            return div;
        }

        function clearFilter() {
            filterInput.value = '';
            eventTypeSelect.value = '';
            // Reload from buffer
            reloadEvents();
        }

        function reloadEvents() {
            const container = document.getElementById('logContainer');
            container.innerHTML = '';
            const filter = filterInput.value.toLowerCase();
            const eventType = eventTypeSelect.value;

            // Only render the most recent MAX_DOM_SIZE events for performance
            const startIdx = Math.max(0, eventBuffer.length - MAX_DOM_SIZE);

            for (let i = startIdx; i < eventBuffer.length; i++) {
                const event = eventBuffer[i];
                if (matchesFilter(event, filter, eventType)) {
                    const logEntry = createLogEntry(event);
                    if (logEntry) {
                        container.appendChild(logEntry);
                    }
                }
            }
            container.scrollTop = container.scrollHeight;
        }

        // Event listeners
        filterInput.addEventListener('input', reloadEvents);
        eventTypeSelect.addEventListener('change', reloadEvents);

        // Connect on load
        connectWebSocket();
    </script>
</body>
</html>"#;

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}

pub async fn api_events(
    reader: web::Data<LogReader>,
    query: web::Query<EventQueryParams>,
) -> HttpResponse {
    let filter = query.filter.as_ref().map(|s| s.to_lowercase());
    let event_type = query.event_type.as_deref();

    let events = match reader.read_all_events() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error reading events: {}", e);
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": format!("Failed to read events: {}", e)}));
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

    HttpResponse::Ok().json(json_events)
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
                "per_core_cpu": m.per_core_usage,
                "mem": mem_pct,
                "load": m.load_avg_1m,
                "disk": disk_pct.round(),
                "per_disk": m.per_disk_metrics.iter().map(|d| serde_json::json!({
                    "device": d.device_name,
                    "read": d.read_bytes_per_sec,
                    "write": d.write_bytes_per_sec,
                    "temp": d.temp_celsius,
                })).collect::<Vec<_>>(),
                "tcp": m.tcp_connections,
                "cpu_temp": m.temps.cpu_temp_celsius,
                "per_core_temps": m.temps.per_core_temps,
                "gpu_temp": m.temps.gpu_temp_celsius,
                "mobo_temp": m.temps.motherboard_temp_celsius,
                "fans": m.fans.iter().map(|f| serde_json::json!({
                    "label": f.label,
                    "rpm": f.rpm,
                })).collect::<Vec<_>>(),
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
                "processes": p.processes.iter().map(|proc| serde_json::json!({
                    "pid": proc.pid,
                    "name": proc.name,
                    "cmdline": proc.cmdline,
                    "state": proc.state,
                    "cpu_percent": proc.cpu_percent,
                    "mem_bytes": proc.mem_bytes,
                    "num_threads": proc.num_threads,
                })).collect::<Vec<serde_json::Value>>(),
            }))
        }
    }
}
