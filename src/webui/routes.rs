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
    <link rel="icon" type="image/svg+xml"
      href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'%3E%3Crect x='10' y='10' width='80' height='80' fill='black'/%3E%3C/svg%3E">
    <style>
        * { line-height: 1.5; }
        body { font-family: system-ui, -apple-system, sans-serif; font-size: 13px; }
        .max-w { max-width: 32rem; }
        .py-5vh { padding-top: 5vh; padding-bottom: 5vh; }
        th, td { padding: 0; }
    </style>
</head>
<body class="bg-gray-50 min-h-screen">
<div class="max-w mx-auto px-4 py-5vh">
    <div class="flex justify-between items-center">
        <div class="text-gray-900 font-semibold">Black Box</div>
        <span id="wsStatus" class="text-red-500 font-semibold" style="display:none;">Disconnected</span>
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
    <div id="cpuDetailsRow" class="text-gray-500"></div>
    <div id="cpuRow" class="text-gray-500 flex justify-between">
        <span id="cpuVal">CPU --%</span>
        <span id="loadVal">Load -- -- --</span>
    </div>
    <div id="cpuCoresContainer" class="grid grid-cols-2 gap-x-4"></div>
    <div class="flex items-center" style="height:19.5px;width:100%;">
        <div id="cpuChart" class="flex items-end justify-end" style="height:10px;width:100%;"></div>
    </div>
    <div class="flex justify-between gap-4">
        <div class="text-gray-500 flex-1" id="ramUsed"></div>
        <div class="text-gray-500 flex-1 text-right" id="cpuTemp"></div>
    </div>
    <div class="flex justify-between gap-4">
        <div class="text-gray-500 flex-1" id="ramAvail"></div>
        <div class="text-gray-500 flex-1 text-right" id="moboTemp"></div>
    </div>
    <div class="flex items-center" style="height:19.5px;width:100%;">
        <div id="memoryChart" class="flex items-end justify-end" style="height:10px;width:100%;"></div>
    </div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold" id="graphicsSection" style="display:none">
        <span class="pr-2">Graphics</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div class="flex justify-between gap-4" id="graphicsRow1" style="display:none">
        <div class="text-gray-500" id="gpuFreq"></div>
        <div class="text-gray-500 text-right" id="gpuTemp"></div>
    </div>
    <div class="flex justify-between gap-4" id="graphicsRow2" style="display:none">
        <div class="text-gray-500" id="memFreq"></div>
        <div class="text-gray-500 text-right" id="imgQuality"></div>
    </div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">Network</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div class="text-gray-500 flex gap-4">
        <span class="flex-1">
            <span id="netName"></span>
            <span id="netSpeedDown"></span>
        </span>
        <span id="netSpeedUp" class="flex-1"></span>
    </div>
    <div class="text-gray-500 flex gap-4">
        <span class="flex-1" id="netRxStats"></span>
        <span class="flex-1" id="netTxStats"></span>
    </div>
    <div class="grid grid-cols-2 gap-x-4 text-gray-500">
        <div id="netAddress"></div>
        <div id="netTcp"></div>
        <div id="netGateway"></div>
        <div id="netDns"></div>
    </div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">Storage</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div id="diskContainer"></div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">Processes</span>
        <span id="procCount" class="text-gray-500 font-normal pr-2"></span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <table class="w-full text-gray-500">
        <thead><tr class="text-left text-gray-400">
            <th class="font-medium text-gray-700">Top CPU</th>
            <th class="font-normal w-16">PID</th>
            <th class="font-normal w-16 text-right">CPU%</th>
            <th class="font-normal w-16 text-right">MEM%</th>
        </tr></thead>
        <tbody id="topCpuTable"></tbody>
    </table>
    <table class="w-full text-gray-500">
        <thead><tr class="text-left text-gray-400">
            <th class="font-medium text-gray-700">Top Memory</th>
            <th class="font-normal w-16">PID</th>
            <th class="font-normal w-16 text-right">CPU%</th>
            <th class="font-normal w-16 text-right">MEM%</th>
        </tr></thead>
        <tbody id="topMemTable"></tbody>
    </table>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold" id="usersSection" style="display:none">
        <span class="pr-2">Users</span>
        <span id="userCount" class="text-gray-500 font-normal pr-2"></span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div id="usersContainer"></div>

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
    <div id="eventsContainer" class="font-mono max-h-96 p-2 overflow-y-auto bg-white border border-gray-200" style="font-size:12px"></div>
</div>

<script>
let ws=null, eventBuffer=[], lastStats=null;
const MAX_BUFFER=1000;
const memoryHistory = []; // Track last 60 seconds of memory usage
const cpuHistory = []; // Track last 60 seconds of CPU usage
const MAX_HISTORY = 60;

const fmt = b => {
    if(!b) return '0B';
    const k=1024, s=['B','KB','MB','GB','TB'], i=Math.floor(Math.log(b)/Math.log(k));
    return (b/Math.pow(k,i)).toFixed(i>1?1:0)+s[i];
};
const fmtRate = b => fmt(b)+'/s';
const formatUptime = s => {
    const d=Math.floor(s/86400),h=Math.floor((s%86400)/3600),m=Math.floor((s%3600)/60),sec=Math.floor(s%60);
    return d>0?`${d}d ${h}h ${m}m`:h>0?`${h}h ${m}m ${sec}s`:`${m}m ${sec}s`;
};
const formatDate = date => {
    const days=['Sun','Mon','Tue','Wed','Thu','Fri','Sat'], mons=['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
    return `${days[date.getDay()]}, ${String(date.getDate()).padStart(2,'0')} ${mons[date.getMonth()]} ${date.getFullYear()}, ${date.toTimeString().substring(0,8)}`;
};

function updateBar(id, pct, container, labelText, rightLabel){
    let el = document.getElementById(id);
    if(!el){
        container.insertAdjacentHTML('beforeend', `<div class="text-gray-500 flex items-center justify-between" id="row_${id}">
            <span id="lbl_${id}">${labelText}</span>
            <span class="flex items-center">
                <span id="rlbl_${id}" class="${rightLabel ? '' : 'hidden'}">${rightLabel || ''}</span>
                <span class="inline-block w-32 h-3 bg-gray-200 rounded-sm overflow-hidden align-middle ml-1">
                    <span id="${id}" class="block h-full transition-all duration-300" style="width:0%"></span>
                </span>
            </span>
        </div>`);
        el = document.getElementById(id);
    }
    const color = pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-yellow-500' : 'bg-green-500';
    el.style.width = Math.min(100, pct) + '%';
    el.className = `block h-full transition-all duration-300 ${color}`;
    const lbl = document.getElementById('lbl_'+id);
    if(lbl) lbl.textContent = labelText;
    const rlbl = document.getElementById('rlbl_'+id);
    if(rlbl && rightLabel !== undefined) { rlbl.textContent = rightLabel; rlbl.className = ''; }
}

function updateCoreBar(id, pct, container, coreNum){
    let el = document.getElementById(id);
    if(!el){
        container.insertAdjacentHTML('beforeend', `<div class="text-gray-500 flex items-center gap-4" id="row_${id}">
            <span class="w-10">CPU${coreNum}</span>
            <span class="relative flex-1 bg-gray-200 rounded-sm" style="height:10px">
                <span id="${id}" class="block h-full transition-all duration-300" style="width:0%"></span>
                <span id="pct_${id}" class="absolute inset-0 flex items-center justify-center text-gray-500/60 overflow-visible"></span>
            </span>
        </div>`);
        el = document.getElementById(id);
    }
    const color = pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-yellow-500' : 'bg-green-500';
    el.style.width = Math.min(100, pct) + '%';
    el.className = `block h-full transition-all duration-300 ${color}`;
    document.getElementById('pct_'+id).textContent = pct.toFixed(1) + '%';
}

function updateRamBar(pct, used, container){
    let el = document.getElementById('ramBar');
    if(!el){
        container.innerHTML = `<div class="text-gray-500 flex items-center gap-4">
            <span id="ramLabel">RAM Used ${fmt(used)}</span>
            <span class="relative flex-1 bg-gray-200 rounded-sm" style="height:10px">
                <span id="ramBar" class="block h-full transition-all duration-300" style="width:0%"></span>
                <span id="ramPct" class="absolute inset-0 flex items-center justify-center text-gray-500/60 overflow-visible"></span>
            </span>
        </div>`;
        el = document.getElementById('ramBar');
    }
    const color = pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-yellow-500' : 'bg-green-500';
    el.style.width = Math.min(100, pct) + '%';
    el.className = `block h-full transition-all duration-300 ${color}`;
    document.getElementById('ramLabel').textContent = `RAM Used ${fmt(used)}`;
    document.getElementById('ramPct').textContent = pct.toFixed(1) + '%';
}

function getUsageColor(pct){
    // Discrete Tailwind colors based on usage thresholds
    if(pct >= 90) return 'rgb(239, 68, 68)';    // red-500
    if(pct >= 80) return 'rgb(248, 113, 113)';  // red-400
    if(pct >= 70) return 'rgb(252, 165, 165)';  // red-300
    if(pct >= 60) return 'rgb(234, 179, 8)';    // yellow-500
    if(pct >= 50) return 'rgb(250, 204, 21)';   // yellow-400
    if(pct >= 40) return 'rgb(253, 224, 71)';   // yellow-300
    if(pct >= 30) return 'rgb(163, 230, 53)';   // lime-400
    if(pct >= 20) return 'rgb(132, 204, 22)';   // lime-500
    if(pct >= 10) return 'rgb(34, 197, 94)';    // green-500
    return 'rgb(74, 222, 128)';                  // green-400
}

function updateMemoryChart(){
    const container = document.getElementById('memoryChart');
    container.innerHTML = '';
    const barWidth = 100 / MAX_HISTORY;
    memoryHistory.forEach(pct => {
        const bar = document.createElement('div');
        bar.style.width = barWidth + '%';
        bar.style.height = pct + '%';
        bar.style.alignSelf = 'flex-end'; // Align to bottom
        bar.style.backgroundColor = getUsageColor(pct);
        container.appendChild(bar);
    });
}

function updateCpuChart(){
    const container = document.getElementById('cpuChart');
    container.innerHTML = '';
    const barWidth = 100 / MAX_HISTORY;
    cpuHistory.forEach(pct => {
        const bar = document.createElement('div');
        bar.style.width = barWidth + '%';
        bar.style.height = pct + '%';
        bar.style.alignSelf = 'flex-end'; // Align to bottom
        bar.style.backgroundColor = getUsageColor(pct);
        container.appendChild(bar);
    });
}

function updateDiskBar(id, pct, container, mount, used, total){
    let el = document.getElementById(id);
    if(!el){
        container.insertAdjacentHTML('beforeend', `<div class="text-gray-500 flex items-center gap-4" id="row_${id}">
            <span id="lbl_${id}" class="flex-1">${mount}</span>
            <span><span id="used_${id}" class="text-gray-400">${fmt(used)}</span>/<span id="total_${id}">${fmt(total)}</span></span>
            <span class="relative bg-gray-200 rounded-sm" style="height:10px;width:128px">
                <span id="${id}" class="block h-full transition-all duration-300" style="width:0%"></span>
                <span id="pct_${id}" class="absolute inset-0 flex items-center justify-center text-gray-500/60 overflow-visible"></span>
            </span>
        </div>`);
        el = document.getElementById(id);
    }
    const color = pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-yellow-500' : 'bg-green-500';
    el.style.width = Math.min(100, pct) + '%';
    el.className = `block h-full transition-all duration-300 ${color}`;
    document.getElementById('lbl_'+id).textContent = mount;
    document.getElementById('pct_'+id).textContent = pct + '%';
    document.getElementById('used_'+id).textContent = fmt(used);
    document.getElementById('total_'+id).textContent = fmt(total);
}

function updateProcTable(tableId, procs, memTotal){
    const tbody = document.getElementById(tableId);
    tbody.innerHTML = '';
    procs.forEach(p => {
        const memPct = memTotal > 0 ? (p.mem_bytes / memTotal) * 100 : 0;
        const tr = document.createElement('tr');
        tr.innerHTML = `<td>${p.name}</td><td>${p.pid}</td><td class="text-right">${p.cpu_percent.toFixed(1)}%</td><td class="text-right">${memPct.toFixed(1)}%</td>`;
        tbody.appendChild(tr);
    });
}

function render(){
    if(!lastStats)return;
    const e=lastStats;
    document.getElementById('datetime').textContent = formatDate(new Date());
    document.getElementById('uptime').textContent = e.system_uptime_seconds ? `Uptime: ${formatUptime(e.system_uptime_seconds)}` : '';
    updateConnectionStatus();

    if(e.kernel) document.getElementById('kernelRow').textContent = `Linux Kernel: ${e.kernel}`;
    if(e.cpu_model) document.getElementById('cpuDetailsRow').textContent = `CPU Details: ${e.cpu_model}${e.cpu_mhz ? `, ${e.cpu_mhz}MHz` : ''}`;

    if(e.cpu !== undefined){
        document.getElementById('cpuVal').textContent = `CPU ${e.cpu.toFixed(1)}%`;
        document.getElementById('loadVal').textContent = `Load ${e.load?.toFixed(2) || '--'} ${e.load5?.toFixed(2) || '--'} ${e.load15?.toFixed(2) || '--'}`;
        // Update CPU history
        cpuHistory.push(e.cpu);
        if(cpuHistory.length > MAX_HISTORY) cpuHistory.shift();
        updateCpuChart();
    }
    (e.per_core_cpu || []).forEach((v, i) => updateCoreBar(`core_${i}`, v, document.getElementById('cpuCoresContainer'), i));
    if(e.mem !== undefined){
        updateRamBar(e.mem, e.mem_used, document.getElementById('ramUsed'));
        document.getElementById('ramAvail').textContent = `Available RAM ${fmt(e.mem_total - e.mem_used)}`;
        // Update memory history
        memoryHistory.push(e.mem);
        if(memoryHistory.length > MAX_HISTORY) memoryHistory.shift();
        updateMemoryChart();
    }
    if(e.cpu_temp){
        const el = document.getElementById('cpuTemp');
        const color = e.cpu_temp >= 80 ? 'text-red-600' : e.cpu_temp >= 60 ? 'text-yellow-600' : 'text-green-600';
        el.innerHTML = `CPU Temp <span class="${color}">${Math.round(e.cpu_temp)}°C</span>`;
    } else {
        document.getElementById('cpuTemp').textContent = '';
    }
    if(e.mobo_temp){
        const el = document.getElementById('moboTemp');
        const color = e.mobo_temp >= 80 ? 'text-red-600' : e.mobo_temp >= 60 ? 'text-yellow-600' : 'text-green-600';
        el.innerHTML = `MB Temp <span class="${color}">${Math.round(e.mobo_temp)}°C</span>`;
    } else if(e.fans && e.fans.length > 0){
        const fan = e.fans[0];
        document.getElementById('moboTemp').textContent = `${fan.label || 'Fan'} ${fan.rpm}RPM`;
    } else {
        document.getElementById('moboTemp').textContent = '';
    }
    // Graphics section - only show if GPU data available
    const hasGpu = e.gpu_freq || e.gpu_temp2 || e.gpu_mem_freq || e.gpu_power;
    document.getElementById('graphicsSection').style.display = hasGpu ? 'flex' : 'none';
    document.getElementById('graphicsRow1').style.display = hasGpu ? 'flex' : 'none';
    document.getElementById('graphicsRow2').style.display = hasGpu ? 'flex' : 'none';
    if(hasGpu){
        document.getElementById('gpuFreq').textContent = e.gpu_freq ? `GPU Freq ${e.gpu_freq}MHz` : '';
        if(e.gpu_temp2){
            const color = e.gpu_temp2 >= 80 ? 'text-red-600' : e.gpu_temp2 >= 60 ? 'text-yellow-600' : 'text-green-600';
            document.getElementById('gpuTemp').innerHTML = `GPU Temp <span class="${color}">${Math.round(e.gpu_temp2)}°C</span>`;
        }
        document.getElementById('memFreq').textContent = e.gpu_mem_freq ? `Mem Freq ${e.gpu_mem_freq}MHz` : '';
        document.getElementById('imgQuality').textContent = e.gpu_power ? `Power ${e.gpu_power.toFixed(0)}W` : '';
    }
    const netInterface = e.net_interface || 'net';

    document.getElementById('netName').textContent = `${netInterface}:`;
    document.getElementById('netSpeedDown').textContent = `Down ${fmtRate(e.net_recv || 0)}`;
    document.getElementById('netSpeedUp').textContent = `Up ${fmtRate(e.net_send || 0)}`;

    // Show RX and TX stats with errors/drops
    const rxErrors = e.net_recv_errors || 0;
    const rxDrops = e.net_recv_drops || 0;
    const txErrors = e.net_send_errors || 0;
    const txDrops = e.net_send_drops || 0;

    const rxText = `RX: ${rxErrors} err/s, ${rxDrops} drop/s`;
    const txText = `TX: ${txErrors} err/s, ${txDrops} drop/s`;
    const rxColor = (rxErrors > 0 || rxDrops > 0) ? 'text-red-600' : 'text-gray-500';
    const txColor = (txErrors > 0 || txDrops > 0) ? 'text-red-600' : 'text-gray-500';

    const rxEl = document.getElementById('netRxStats');
    const txEl = document.getElementById('netTxStats');
    rxEl.textContent = rxText;
    txEl.textContent = txText;
    rxEl.className = `flex-1 ${rxColor}`;
    txEl.className = `flex-1 ${txColor}`;

    document.getElementById('netAddress').textContent = `Address: ${e.net_ip || '--'}`;
    document.getElementById('netTcp').textContent = `TCP Connections: ${e.tcp || '--'}`;
    document.getElementById('netGateway').textContent = `Gateway: ${e.net_gateway || '--'}`;
    document.getElementById('netDns').textContent = `DNS: ${e.net_dns || '--'}`;

    // Storage section
    (e.filesystems || []).forEach((fs, i) => {
        const pct = fs.total > 0 ? Math.round((fs.used/fs.total)*100) : 0;
        updateDiskBar(`disk_${i}`, pct, document.getElementById('diskContainer'), fs.mount, fs.used, fs.total);
    });
    // Users section
    const users = e.users || [];
    document.getElementById('usersSection').style.display = users.length > 0 ? 'flex' : 'none';
    document.getElementById('userCount').textContent = users.length > 0 ? `${users.length} logged in` : '';
    const usersContainer = document.getElementById('usersContainer');
    usersContainer.innerHTML = '';
    users.forEach(u => {
        const isRemote = u.remote_host && u.remote_host !== '';
        const div = document.createElement('div');
        div.className = 'text-gray-500 flex justify-between';
        div.innerHTML = `<span>${u.username} <span class="text-gray-400">(${u.terminal})</span></span>${isRemote ? `<span class="text-gray-400">from ${u.remote_host}</span>` : ''}`;
        usersContainer.appendChild(div);
    });
}

function updateProcs(event){
    document.getElementById('procCount').textContent = `${event.total_processes || 0} total ${event.running_processes || 0} running`;
    const memTotal = lastStats?.mem_total || 0;
    const topCpu = (event.processes || []).slice().sort((a,b) => b.cpu_percent - a.cpu_percent).slice(0,5);
    const topMem = (event.processes || []).slice().sort((a,b) => b.mem_bytes - a.mem_bytes).slice(0,5);
    updateProcTable('topCpuTable', topCpu, memTotal);
    updateProcTable('topMemTable', topMem, memTotal);
}

function updateConnectionStatus(){
    const isConnected = ws && ws.readyState === 1;
    document.getElementById('wsStatus').style.display = isConnected ? 'none' : 'inline';
}

function connectWebSocket(){
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(protocol + '//' + window.location.host + '/ws');
    ws.onopen = () => {
        updateConnectionStatus();
    };
    ws.onmessage = (ev) => {
        try {
            const e = JSON.parse(ev.data);
            if(e.type === 'SystemMetrics') { lastStats = e; render(); }
            else if(e.type === 'ProcessSnapshot') { updateProcs(e); }
            else { addEventToLog(e); }
        } catch(err) {}
    };
    ws.onerror = () => {
        updateConnectionStatus();
    };
    ws.onclose = () => {
        updateConnectionStatus();
        setTimeout(connectWebSocket, 5000);
    };
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
            if(container.children.length > 200) container.removeChild(container.lastChild);
        }
    }
}

function matchesFilter(e, filter, evType){
    if(evType){
        const map = {process:'ProcessLifecycle', security:'SecurityEvent', anomaly:'Anomaly'};
        if(e.type !== map[evType]) return false;
    }
    return !filter || JSON.stringify(e).toLowerCase().includes(filter);
}

function createEventEntry(e){
    if(!e.type || e.type === 'ProcessSnapshot') return null;
    const div = document.createElement('div');
    div.className = 'text-gray-600';
    const time = (e.timestamp || '').substring(11,23);
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
setInterval(() => { if(lastStats) document.getElementById('datetime').textContent = formatDate(new Date()); }, 1000);
</script>
</body>
</html>
"#;
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(html)
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
                "kernel": m.kernel_version,
                "cpu_model": m.cpu_model,
                "cpu_mhz": m.cpu_mhz,
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
                "net_recv_errors": m.net_recv_errors_per_sec,
                "net_send_errors": m.net_send_errors_per_sec,
                "net_recv_drops": m.net_recv_drops_per_sec,
                "net_send_drops": m.net_send_drops_per_sec,
                "net_interface": m.net_interface,
                "net_ip": m.net_ip_address,
                "net_gateway": m.net_gateway,
                "net_dns": m.net_dns,
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
