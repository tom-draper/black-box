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
    let html = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <title>Black Box</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <script src="https://cdn.tailwindcss.com"></script>
    <style>
        * { line-height: 1.5; font-size: 13px; }
        body { font-family: system-ui, -apple-system, sans-serif; }
        .max-w-42 { max-width: 42rem; }
        .py-5vh { padding-top: 5vh; padding-bottom: 5vh; }
    </style>
</head>
<body class="bg-gray-50 min-h-screen">
<div class="max-w-42 mx-auto px-4 py-5vh">
    <div class="flex justify-between items-center">
        <div class="text-gray-900 font-semibold">Black Box</div>
        <span id="wsStatus" class="text-red-600" style="display:none;">Disconnected</span>
    </div>
    <div class="flex justify-between text-gray-500">
        <span id="datetime"></span>
        <span id="uptime"></span>
    </div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">System</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div id="kernelRow" class="text-gray-500"></div>
    <div id="cpuRow" class="text-gray-500 flex justify-between"></div>
    <div id="cpuCoresContainer" class="grid grid-cols-2 gap-x-4"></div>
    <div class="text-gray-500" id="ramUsed"></div>
    <div class="text-gray-500" id="ramAvail"></div>
    <div class="text-gray-500" id="cpuTemp"></div>
    <div class="text-gray-500" id="moboTemp"></div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">Network</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div class="text-gray-500" id="netRx"></div>
    <div class="text-gray-500" id="netTx"></div>
    <div class="text-gray-500" id="tcpConns"></div>
    <div class="text-gray-500" id="tcpWait"></div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">File Systems</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div id="diskContainer"></div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">Processes</span>
        <span id="procCount" class="text-gray-500 font-normal pr-2"></span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div id="topProcsContainer"></div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">Events</span>
        <div class="flex-1 flex items-center">
            <div class="flex-1 border-b border-gray-200"></div>
            <div class="flex gap-2 items-center font-normal ml-2">
                <input type="text" id="filterInput" placeholder="Search..."
                    class="px-2 py-0 border border-gray-300 rounded text-gray-700 focus:outline-none focus:ring-1 focus:ring-gray-400" />
                <select id="eventType" class="px-2 py-0 border border-gray-300 rounded text-gray-700 focus:outline-none">
                    <option value="">All</option>
                    <option value="process">Process</option>
                    <option value="security">Security</option>
                    <option value="anomaly">Anomaly</option>
                </select>
            </div>
        </div>
    </div>
    <div id="eventsContainer" class="font-mono max-h-96 overflow-y-auto bg-white border border-gray-200"></div>
</div>

<script>
let ws=null;
let eventBuffer=[];
let lastStats=null;
let topProcs=[];
let totalProcs=0;
let runningProcs=0;
let startTime=Date.now();
const MAX_BUFFER=1000;

// Utility functions
function fmt(b){
    if(b===0 || b===undefined)return'0B';
    const k=1024,s=['B','KB','MB','GB','TB'];
    const i=Math.floor(Math.log(b)/Math.log(k));
    return(b/Math.pow(k,i)).toFixed(i>1?1:0)+s[i];
}

function fmtRate(b){
    return fmt(b)+'/s';
}

function formatUptime(s){
    const d=Math.floor(s/86400),h=Math.floor((s%86400)/3600),m=Math.floor((s%3600)/60),sec=Math.floor(s%60);
    if(d>0)return `${d}d ${h}h ${m}m`;
    if(h>0)return `${h}h ${m}m ${sec}s`;
    return `${m}m ${sec}s`;
}

function formatDate(date){
    const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
    const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
    const day = days[date.getDay()];
    const d = String(date.getDate()).padStart(2, '0');
    const month = months[date.getMonth()];
    const year = date.getFullYear();
    const time = date.toTimeString().substring(0, 8);
    return `${day}, ${d} ${month} ${year}, ${time}`;
}

function createBar(pct, width='w-32'){
    const color = pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-yellow-500' : 'bg-green-500';
    return `<span class="inline-block ${width} h-3 bg-gray-200 rounded-sm overflow-hidden align-middle ml-1">
        <span class="${color} block h-full transition-all duration-300" style="width:${Math.min(100,pct)}%"></span>
    </span>`;
}

function render(){
    if(!lastStats)return;
    const e=lastStats;
    const wsOk=ws&&ws.readyState===1;
    const now=new Date();

    // Status line
    document.getElementById('datetime').textContent = formatDate(now);
    if(e.system_uptime_seconds !== undefined && e.system_uptime_seconds !== null){
        document.getElementById('uptime').textContent = `Uptime: ${formatUptime(e.system_uptime_seconds)}`;
    } else {
        document.getElementById('uptime').textContent = '';
    }
    document.getElementById('wsStatus').style.display = wsOk ? 'none' : 'inline';

    // CPU info
    if(e.cpu !== undefined){
        document.getElementById('cpuRow').innerHTML = `<span>CPU ${e.cpu.toFixed(1)}%</span><span>Load ${e.load?.toFixed(2) || '--'} ${e.load5?.toFixed(2) || '--'} ${e.load15?.toFixed(2) || '--'}</span>`;
    }

    // Per-core CPUs in 2 columns
    const perCore = e.per_core_cpu || [];
    if(perCore.length > 0){
        const coresHTML = perCore.map((v,i) => {
            return `<div class="text-gray-500 flex items-center justify-between">
                <span>CPU${i} ${v.toFixed(1)}%</span>
                ${createBar(v, 'w-32')}
            </div>`;
        }).join('');
        document.getElementById('cpuCoresContainer').innerHTML = coresHTML;
    }

    // RAM
    if(e.mem !== undefined){
        document.getElementById('ramUsed').innerHTML = `RAM ${fmt(e.mem_used)}/${fmt(e.mem_total)} ${e.mem.toFixed(1)}% ${createBar(e.mem, 'w-32')}`;
        document.getElementById('ramAvail').innerHTML = `Available RAM ${fmt(e.mem_total - e.mem_used)}`;
    }

    // Temps
    if(e.cpu_temp){
        const tempClass = e.cpu_temp >= 80 ? 'text-red-600' : e.cpu_temp >= 60 ? 'text-yellow-600' : 'text-green-600';
        document.getElementById('cpuTemp').innerHTML = `<span class="${tempClass}">CPU Temp ${Math.round(e.cpu_temp)}°C</span>`;
    } else {
        document.getElementById('cpuTemp').innerHTML = '';
    }

    if(e.mobo_temp){
        document.getElementById('moboTemp').innerHTML = `Motherboard ${Math.round(e.mobo_temp)}°C`;
    } else {
        document.getElementById('moboTemp').innerHTML = '';
    }

    // Network
    const rx = e.net_recv || 0;
    const tx = e.net_send || 0;
    document.getElementById('netRx').innerHTML = `RX ${fmtRate(rx)}`;
    document.getElementById('netTx').innerHTML = `TX ${fmtRate(tx)}`;
    document.getElementById('tcpConns').innerHTML = `TCP ${e.tcp || '--'}`;
    document.getElementById('tcpWait').innerHTML = `TIME_WAIT ${e.tcp_wait || '--'}`;

    // Disks
    const filesystems = e.filesystems || [];
    if(filesystems.length > 0){
        const disksHTML = filesystems.map(fs => {
            const pct = fs.total > 0 ? Math.round((fs.used/fs.total)*100) : 0;
            const mount = fs.mount;
            const usage = `${fmt(fs.used)}/${fmt(fs.total)} ${pct}%`;
            return `<div class="text-gray-500 flex items-center justify-between">
                <span>${mount}</span>
                <span class="flex items-center gap-1">
                    <span>${usage}</span>
                    ${createBar(pct, 'w-32')}
                </span>
            </div>`;
        }).join('');
        document.getElementById('diskContainer').innerHTML = disksHTML;
    }

    // Processes
    if(topProcs.length > 0){
        document.getElementById('procCount').textContent = `${totalProcs} total ${runningProcs} running`;

        const topCpu = topProcs.slice().sort((a,b) => b.cpu_percent - a.cpu_percent).slice(0,5);
        const topMem = topProcs.slice().sort((a,b) => b.mem_bytes - a.mem_bytes).slice(0,5);

        let procsHTML = '<div class="grid grid-cols-2 gap-4">';

        // Top CPU column
        procsHTML += '<div><div class="text-gray-700 font-medium">Top CPU</div>';
        topCpu.forEach(p => {
            const name = p.name.substring(0,18).padEnd(18);
            procsHTML += `<div class="text-gray-500 font-mono">${name} ${p.cpu_percent.toFixed(1)}%</div>`;
        });
        procsHTML += '</div>';

        // Top Memory column
        procsHTML += '<div><div class="text-gray-700 font-medium">Top Memory</div>';
        topMem.forEach(p => {
            const name = p.name.substring(0,16).padEnd(16);
            procsHTML += `<div class="text-gray-500 font-mono">${name} ${fmt(p.mem_bytes)}</div>`;
        });
        procsHTML += '</div>';

        procsHTML += '</div>';

        document.getElementById('topProcsContainer').innerHTML = procsHTML;
    }
}

// WebSocket connection
function connectWebSocket(){
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(protocol + '//' + window.location.host + '/ws');

    ws.onopen = () => render();
    ws.onmessage = (ev) => {
        try {
            const event = JSON.parse(ev.data);
            handleEvent(event);
        } catch(e) {}
    };
    ws.onerror = () => render();
    ws.onclose = () => {
        render();
        setTimeout(connectWebSocket, 5000);
    };
}

function handleEvent(event){
    if(event.type === 'SystemMetrics'){
        lastStats = event;
        render();
    } else if(event.type === 'ProcessSnapshot'){
        topProcs = event.processes || [];
        totalProcs = event.total_processes || 0;
        runningProcs = event.running_processes || 0;
        render();
    } else {
        addEventToLog(event);
    }
}

function addEventToLog(event){
    eventBuffer.push(event);
    if(eventBuffer.length > MAX_BUFFER) eventBuffer.shift();

    const filter = document.getElementById('filterInput').value.toLowerCase();
    const evType = document.getElementById('eventType').value;

    if(matchesFilter(event, filter, evType)){
        const container = document.getElementById('eventsContainer');
        const entry = createEventEntry(event);
        if(entry){
            container.insertBefore(entry, container.firstChild);
            while(container.children.length > 200) container.removeChild(container.lastChild);
        }
    }
}

function matchesFilter(e, filter, evType){
    if(evType){
        const map = {process:'ProcessLifecycle', security:'SecurityEvent', anomaly:'Anomaly'};
        if(e.type !== map[evType]) return false;
    }
    if(filter && !JSON.stringify(e).toLowerCase().includes(filter)) return false;
    return true;
}

function createEventEntry(e){
    if(!e.type || e.type === 'ProcessSnapshot') return null;

    const div = document.createElement('div');
    div.className = 'text-gray-600';
    const time = (e.timestamp || '').substring(11,19);

    if(e.type === 'ProcessLifecycle'){
        const color = e.kind === 'Started' ? 'text-green-600' : e.kind === 'Exited' ? 'text-gray-400' : 'text-yellow-600';
        div.innerHTML = `<span class="text-gray-400">${time}</span> <span class="${color}">[${e.kind}]</span> ${e.name} <span class="text-gray-400">(pid ${e.pid})</span>`;
    } else if(e.type === 'SecurityEvent'){
        const color = e.kind.includes('Success') ? 'text-green-600' : 'text-red-600';
        div.innerHTML = `<span class="text-gray-400">${time}</span> <span class="${color}">[${e.kind}]</span> ${e.user} ${e.source_ip ? 'from ' + e.source_ip : ''}`;
    } else if(e.type === 'Anomaly'){
        const color = e.severity === 'Critical' ? 'text-red-600' : 'text-yellow-600';
        div.innerHTML = `<span class="text-gray-400">${time}</span> <span class="${color}">[${e.severity}]</span> ${e.message}`;
    }

    return div;
}

function reloadEvents(){
    const container = document.getElementById('eventsContainer');
    container.innerHTML = '';
    const filter = document.getElementById('filterInput').value.toLowerCase();
    const evType = document.getElementById('eventType').value;

    eventBuffer.slice().reverse().forEach(event => {
        if(matchesFilter(event, filter, evType)){
            const entry = createEventEntry(event);
            if(entry) container.appendChild(entry);
        }
    });
}

document.getElementById('filterInput').addEventListener('input', reloadEvents);
document.getElementById('eventType').addEventListener('change', reloadEvents);

connectWebSocket();
setInterval(() => {
    if(lastStats){
        document.getElementById('datetime').textContent = formatDate(new Date());
    }
}, 1000);
</script>
</body>
</html>
"#;

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
                "system_uptime_seconds": m.system_uptime_seconds,
                "cpu": m.cpu_usage_percent,
                "per_core_cpu": m.per_core_usage,
                "mem": mem_pct,
                "mem_used": m.mem_used_bytes,
                "mem_total": m.mem_total_bytes,
                "load": m.load_avg_1m,
                "load5": m.load_avg_5m,
                "load15": m.load_avg_15m,
                "disk": disk_pct.round(),
                "disk_used": m.disk_used_bytes,
                "disk_total": m.disk_total_bytes,
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
                "tcp": m.tcp_connections,
                "tcp_wait": m.tcp_time_wait,
                "net_recv": m.net_recv_bytes_per_sec,
                "net_send": m.net_send_bytes_per_sec,
                "cpu_temp": m.temps.cpu_temp_celsius,
                "per_core_temps": m.temps.per_core_temps,
                "gpu_temp": m.temps.gpu_temp_celsius,
                "mobo_temp": m.temps.motherboard_temp_celsius,
                "fans": m.fans.iter().map(|f| serde_json::json!({
                    "label": f.label,
                    "rpm": f.rpm,
                })).collect::<Vec<_>>(),
                "users": m.logged_in_users.iter().map(|u| serde_json::json!({
                    "username": u.username,
                    "terminal": u.terminal,
                    "remote_host": u.remote_host,
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
                "total_processes": p.total_processes,
                "running_processes": p.running_processes,
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
