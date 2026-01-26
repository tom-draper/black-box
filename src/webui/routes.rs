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
    <div class="fixed z-10 top-0 right-0">
        <div class="flex gap-3 px-5 py-4 text-gray-400 items-center">
            <svg id="rewindBtn" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="size-4 hover:text-gray-600 transition duration-100 cursor-pointer" title="Rewind 1 minute">
                <path d="M7.712 4.818A1.5 1.5 0 0 1 10 6.095v2.972c.104-.13.234-.248.389-.343l6.323-3.906A1.5 1.5 0 0 1 19 6.095v7.81a1.5 1.5 0 0 1-2.288 1.276l-6.323-3.905a1.505 1.505 0 0 1-.389-.344v2.973a1.5 1.5 0 0 1-2.288 1.276l-6.323-3.905a1.5 1.5 0 0 1 0-2.552l6.323-3.906Z" />
            </svg>
            <svg id="fastForwardBtn" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="size-4 hover:text-gray-600 transition duration-100 cursor-pointer" title="Fast forward 1 minute">
                <path d="M3.288 4.818A1.5 1.5 0 0 0 1 6.095v7.81a1.5 1.5 0 0 0 2.288 1.276l6.323-3.905c.155-.096.285-.213.389-.344v2.973a1.5 1.5 0 0 0 2.288 1.276l6.323-3.905a1.5 1.5 0 0 0 0-2.552l-6.323-3.906A1.5 1.5 0 0 0 10 6.095v2.972a1.506 1.506 0 0 0-.389-.343L3.288 4.818Z" />
            </svg>
            <svg id="pauseBtn" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="size-4 hover:text-gray-600 transition duration-100 cursor-pointer" title="Pause (enable time-travel)">
                <path d="M5.75 3a.75.75 0 0 0-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 0 0 .75-.75V3.75A.75.75 0 0 0 7.25 3h-1.5ZM12.75 3a.75.75 0 0 0-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 0 0 .75-.75V3.75a.75.75 0 0 0-.75-.75h-1.5Z" />
            </svg>
            <svg id="playBtn" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="size-4 text-gray-800 hover:text-gray-600 transition duration-100 cursor-pointer" style="display:none" title="Resume live view">
                <path d="M6.3 2.84A1.5 1.5 0 0 0 4 4.11v11.78a1.5 1.5 0 0 0 2.3 1.27l9.344-5.891a1.5 1.5 0 0 0 0-2.538L6.3 2.841Z" />
            </svg>
            <div class="border-l border-gray-300 h-4"></div>
            <div class="flex flex-col text-xs">
                <input type="datetime-local" id="timePicker" class="px-1 py-0.5 border border-gray-300 rounded text-gray-700 text-xs" style="display:none" title="Select a specific date and time to view" />
                <span id="timeDisplay" class="text-gray-500 cursor-pointer hover:text-gray-700" title="Click to select time, Shift+Click to go Live">Live</span>
                <span id="timeRange" class="text-gray-400 text-xs" title="Total time range of available historical data"></span>
            </div>
        </div>
        <div class="px-5 pb-3">
            <canvas id="timelineChart" width="280" height="48" class="cursor-pointer bg-gray-50 rounded" style="display:none;" title="Event density timeline - Click to jump to any time. Blue line shows current playback position."></canvas>
        </div>
    </div>
    <div id="mainContent" style="display:none;">
    <div class="flex justify-between items-center">
        <div class="text-gray-900 font-semibold" title="Black Box - Linux System Monitor">Black Box</div>
        <span id="wsStatus" class="text-red-500 font-semibold" style="display:none;" title="WebSocket connection to server lost">Disconnected</span>
    </div>
    <div class="flex justify-between text-gray-500">
        <span id="datetime" title="Current system date and time"></span>
        <span id="uptime" title="Time since system boot"></span>
    </div>
    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">System</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div id="kernelRow" class="text-gray-500" title="Linux kernel version"></div>
    <div id="cpuDetailsRow" class="text-gray-500" title="CPU model and frequency"></div>
    <div class="text-gray-500 flex items-center gap-4">
        <div class="flex-1 flex items-center gap-4">
            <span class="w-10" title="Overall CPU usage across all cores">CPU</span>
            <span class="relative flex-1 bg-gray-200" style="height:10px;border-radius:1px">
                <span id="cpuBar" class="block h-full transition-all duration-300" style="width:0%;border-radius:1px"></span>
                <span id="cpuPct" class="absolute inset-0 flex items-center justify-center text-gray-500/60 overflow-visible"></span>
            </span>
        </div>
        <span id="loadVal" class="flex-1 text-right text-gray-500" title="System load average over 1, 5, and 15 minutes">Load average: --% --% --%</span>
    </div>
    <div id="cpuCoresContainer" class="grid grid-cols-2 gap-x-4" title="Per-core CPU usage"></div>
    <div class="flex items-center" style="height:19.5px;width:100%;">
        <canvas id="cpuChart" style="height:10px;width:100%;" title="CPU usage over last 60 seconds"></canvas>
    </div>
    <div class="flex justify-between gap-4">
        <div class="text-gray-500 flex-1" id="ramUsed" title="Amount of RAM currently in use"></div>
        <div class="text-gray-500 flex-1 text-right" id="cpuTemp" title="CPU temperature sensor reading"></div>
    </div>
    <div class="flex justify-between gap-4">
        <div class="text-gray-500 flex-1" id="ramAvail" title="Amount of RAM available for use"></div>
        <div class="text-gray-500 flex-1 text-right" id="moboTemp" title="Motherboard temperature or fan speed"></div>
    </div>
    <div class="flex items-center" style="height:19.5px;width:100%;">
        <canvas id="memoryChart" style="height:10px;width:100%;" title="Memory usage over last 60 seconds"></canvas>
    </div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold" id="graphicsSection" style="display:none" title="GPU metrics (only shown if GPU detected)">
        <span class="pr-2">Graphics</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div class="flex justify-between gap-4" id="graphicsRow1" style="display:none">
        <div class="text-gray-500" id="gpuFreq" title="GPU core frequency"></div>
        <div class="text-gray-500 text-right" id="gpuTemp" title="GPU temperature"></div>
    </div>
    <div class="flex justify-between gap-4" id="graphicsRow2" style="display:none">
        <div class="text-gray-500" id="memFreq" title="GPU memory frequency"></div>
        <div class="text-gray-500 text-right" id="imgQuality" title="GPU power consumption"></div>
    </div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">Network</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div class="text-gray-500 flex gap-4">
        <div class="flex-1">
            <div>
                <span id="netName" title="Primary network interface"></span>
                <span id="netSpeedDown" title="Download speed in bytes per second"></span>
            </div>
            <div class="flex items-center" style="height:19.5px;width:100%;">
                <canvas id="netDownChart" style="height:10px;width:100%;" title="Download speed over last 60 seconds"></canvas>
            </div>
        </div>
        <div class="flex-1">
            <div id="netSpeedUp" title="Upload speed in bytes per second"></div>
            <div class="flex items-center" style="height:19.5px;width:100%;">
                <canvas id="netUpChart" style="height:10px;width:100%;" title="Upload speed over last 60 seconds"></canvas>
            </div>
        </div>
    </div>
    <div class="text-gray-500 flex gap-4">
        <span class="flex-1" id="netRxStats" title="Receive errors and dropped packets per second"></span>
        <span class="flex-1" id="netTxStats" title="Transmit errors and dropped packets per second"></span>
    </div>
    <div class="grid grid-cols-2 gap-x-4 text-gray-500">
        <div id="netAddress" title="IP address of primary interface"></div>
        <div id="netTcp" title="Number of active TCP connections"></div>
        <div id="netGateway" title="Default gateway IP address"></div>
        <div id="netDns" title="DNS server IP address"></div>
    </div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2" title="Mounted filesystems and their usage">Storage</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div id="diskContainer" title="Filesystem mount points with usage bars"></div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold" id="diskIoSection" style="display:none" title="Per-device disk I/O statistics">
        <span class="pr-2">Disk IO</span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <table class="w-full text-gray-500" id="diskIoTable" style="display:none">
        <thead><tr class="text-left text-gray-400">
            <th class="font-normal" style="width:60px" title="Block device name">Device</th>
            <th class="font-normal text-right" style="width:80px" title="Read speed in bytes per second">Read</th>
            <th class="font-normal text-right" style="width:80px" title="Write speed in bytes per second">Write</th>
            <th class="font-normal text-right" style="width:50px" title="Disk temperature in Celsius">Temp</th>
            <th style="width:128px" title="I/O activity over last 60 seconds"></th>
        </tr></thead>
        <tbody id="diskIoTableBody"></tbody>
    </table>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2">Processes</span>
        <span id="procCount" class="text-gray-500 font-normal pr-2" title="Total process count and running count"></span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <table class="w-full text-gray-500" title="Top 5 processes by CPU usage">
        <thead><tr class="text-left text-gray-400">
            <th class="font-medium text-gray-700">Top CPU</th>
            <th class="font-normal w-16" title="Process ID">PID</th>
            <th class="font-normal w-16 text-right" title="CPU usage percentage">CPU%</th>
            <th class="font-normal w-16 text-right" title="Memory usage percentage">MEM%</th>
        </tr></thead>
        <tbody id="topCpuTable"></tbody>
    </table>
    <table class="w-full text-gray-500" title="Top 5 processes by memory usage">
        <thead><tr class="text-left text-gray-400">
            <th class="font-medium text-gray-700">Top Memory</th>
            <th class="font-normal w-16" title="Process ID">PID</th>
            <th class="font-normal w-16 text-right" title="CPU usage percentage">CPU%</th>
            <th class="font-normal w-16 text-right" title="Memory usage percentage">MEM%</th>
        </tr></thead>
        <tbody id="topMemTable"></tbody>
    </table>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold" id="usersSection" style="display:none" title="Currently logged in users">
        <span class="pr-2">Users</span>
        <span id="userCount" class="text-gray-500 font-normal pr-2" title="Number of logged in users"></span>
        <div class="flex-1 border-b border-gray-200"></div>
    </div>
    <div id="usersContainer"></div>

    <div></div>
    <div class="flex items-center text-gray-900 font-semibold">
        <span class="pr-2" title="Real-time event log for processes, security events, and anomalies">Events</span>
        <div class="flex-1 flex items-center">
            <div class="flex-1 border-b border-gray-200"></div>
            <div class="flex gap-1 items-center font-normal ml-2">
                <input type="text" id="filterInput" placeholder="Search..." title="Filter events by text search"
                    class="px-2 py-0 border border-gray-300 rounded text-gray-700 focus:outline-none focus:ring-1 focus:ring-gray-400" />
                <select id="eventType" class="px-2 py-0 border border-gray-300 rounded text-gray-700 focus:outline-none" title="Filter by event type">
                    <option value="">All</option>
                    <option value="process">Process</option>
                    <option value="security">Security</option>
                    <option value="anomaly">Anomaly</option>
                </select>
            </div>
        </div>
    </div>
    <div id="eventsContainer" class="font-mono max-h-96 p-2 overflow-y-auto bg-white border border-gray-200" style="font-size:12px" title="Scrollable event log (last 1000 events)"></div>
    </div>
</div>

<script>
let ws=null, eventBuffer=[], lastStats=null, isPaused=false;
const MAX_BUFFER=1000;
const memoryHistory = []; // Track last 60 seconds of memory usage
const cpuHistory = []; // Track last 60 seconds of CPU usage
const netDownHistory = []; // Track last 60 seconds of download speed
const netUpHistory = []; // Track last 60 seconds of upload speed
const diskIoHistoryMap = {}; // Track last 60 seconds per disk
const MAX_HISTORY = 60;

// Cache for static/semi-static fields (these may not be in every event)
let cachedMemTotal = null;
let cachedSwapTotal = null;
let cachedDiskTotal = null;
let cachedFilesystems = [];
let cachedNetIp = null;
let cachedNetGateway = null;
let cachedNetDns = null;
let cachedKernel = null;
let cachedCpuModel = null;
let cachedCpuMhz = null;

// Previous values cache for change detection (optimization to avoid unnecessary DOM updates)
const prevValues = {};

// Helper function to update DOM element only if value changed
function updateIfChanged(id, value, updateFn) {
    if (prevValues[id] !== value) {
        prevValues[id] = value;
        updateFn(value);
    }
}

// Helper function to update text content only if changed
function updateTextIfChanged(id, text) {
    const key = `${id}_text`;
    if (prevValues[key] !== text) {
        prevValues[key] = text;
        document.getElementById(id).textContent = text;
    }
}

// Helper function to update innerHTML only if changed
function updateHtmlIfChanged(id, html) {
    const key = `${id}_html`;
    if (prevValues[key] !== html) {
        prevValues[key] = html;
        document.getElementById(id).innerHTML = html;
    }
}

// Helper function to update style only if changed
function updateStyleIfChanged(id, prop, value) {
    const key = `${id}_style_${prop}`;
    if (prevValues[key] !== value) {
        prevValues[key] = value;
        document.getElementById(id).style[prop] = value;
    }
}

// Time-travel state
let playbackMode = false; // false = live, true = historical playback
let currentTimestamp = null; // Current playback timestamp (seconds)
let firstTimestamp = null; // Earliest available data
let lastTimestamp = null; // Latest available data
const REWIND_STEP = 60; // 1 minute
let playbackInterval = null; // Auto-playback timer

// Fetch the most recent complete system state on load to initialize caches
async function fetchInitialState() {
    try {
        const resp = await fetch('/api/initial-state');
        const data = await resp.json();

        console.log('initial state', data);

        if(data.type === 'SystemMetrics') {
            // Populate caches with static/semi-static fields
            if(data.mem_total != null) cachedMemTotal = data.mem_total;
            if(data.swap_total != null) cachedSwapTotal = data.swap_total;
            if(data.disk_total != null) cachedDiskTotal = data.disk_total;
            if(data.net_ip != null) cachedNetIp = data.net_ip;
            if(data.net_gateway != null) cachedNetGateway = data.net_gateway;
            if(data.net_dns != null) cachedNetDns = data.net_dns;
            if(data.kernel != null) cachedKernel = data.kernel;
            if(data.cpu_model != null) cachedCpuModel = data.cpu_model;
            if(data.cpu_mhz != null) cachedCpuMhz = data.cpu_mhz;

            if(data.filesystems && data.filesystems.length > 0) {
                cachedFilesystems = data.filesystems;
                // Render filesystems immediately
                const filesystems = data.filesystems;
                filesystems.forEach((fs, i) => {
                    const pct = fs.total > 0 ? Math.round((fs.used/fs.total)*100) : 0;
                    updateDiskBar(`disk_${i}`, pct, document.getElementById('diskContainer'), fs.mount, fs.used, fs.total);
                });
            }

            // Render network info immediately
            if(cachedNetIp) document.getElementById('netAddress').textContent = `Address: ${cachedNetIp}`;
            if(cachedNetGateway) document.getElementById('netGateway').textContent = `Gateway: ${cachedNetGateway}`;
            if(cachedNetDns) document.getElementById('netDns').textContent = `DNS: ${cachedNetDns}`;

            // Render kernel and CPU info immediately
            if(cachedKernel) document.getElementById('kernelRow').textContent = `Linux Kernel: ${cachedKernel}`;
            if(cachedCpuModel) document.getElementById('cpuDetailsRow').textContent = `CPU Details: ${cachedCpuModel}${cachedCpuMhz ? `, ${cachedCpuMhz}MHz` : ''}`;
        }
    } catch(e) {
        console.error('Failed to load initial state:', e);
    }
}

// Timeline visualization
let timelineData = null;

async function fetchTimeline() {
    try {
        const resp = await fetch('/api/timeline');
        const data = await resp.json();
        timelineData = data;

        if(data.timeline && data.timeline.length > 0) {
            document.getElementById('timelineChart').style.display = 'block';
            drawTimeline();
        }
    } catch(e) {
        console.error('Failed to load timeline:', e);
    }
}

function drawTimeline() {
    if(!timelineData || !timelineData.timeline || timelineData.timeline.length === 0) return;

    const canvas = document.getElementById('timelineChart');
    const ctx = canvas.getContext('2d');
    const width = canvas.width;
    const height = canvas.height;

    // Clear canvas and draw background
    ctx.clearRect(0, 0, width, height);
    ctx.fillStyle = 'rgba(249, 250, 251, 1)'; // gray-50
    ctx.fillRect(0, 0, width, height);

    const timeline = timelineData.timeline;
    const firstTs = timelineData.first_timestamp;
    const lastTs = timelineData.last_timestamp;
    const timeRange = lastTs - firstTs;

    if(timeRange <= 0 || timeline.length === 0) return;

    // Find max count for scaling
    const maxCount = Math.max(...timeline.map(p => p.count), 1);

    // Map data points to canvas coordinates
    const points = timeline.map(p => {
        const x = ((p.timestamp - firstTs) / timeRange) * width;
        const y = height - ((p.count / maxCount) * (height - 8)) - 4; // Leave 4px padding at top/bottom
        return { x, y, timestamp: p.timestamp };
    });

    // Draw smooth curve using cubic Bezier curves
    ctx.beginPath();
    ctx.strokeStyle = 'rgba(156, 163, 175, 0.8)'; // gray-400
    ctx.lineWidth = 1.5;

    if(points.length > 0) {
        ctx.moveTo(points[0].x, points[0].y);

        if(points.length === 2) {
            // Just draw a line for 2 points
            ctx.lineTo(points[1].x, points[1].y);
        } else if(points.length > 2) {
            // Use cubic Bezier curves for smooth interpolation
            for(let i = 0; i < points.length - 1; i++) {
                const curr = points[i];
                const next = points[i + 1];

                // Calculate control points for smooth curve
                // Use neighboring points to determine tangent direction
                const prev = i > 0 ? points[i - 1] : curr;
                const after = i < points.length - 2 ? points[i + 2] : next;

                const cp1x = curr.x + (next.x - prev.x) / 6;
                const cp1y = curr.y + (next.y - prev.y) / 6;
                const cp2x = next.x - (after.x - curr.x) / 6;
                const cp2y = next.y - (after.y - curr.y) / 6;

                ctx.bezierCurveTo(cp1x, cp1y, cp2x, cp2y, next.x, next.y);
            }
        }
    }

    ctx.stroke();

    // Draw vertical line for current playback position
    if(playbackMode && currentTimestamp) {
        const currentX = ((currentTimestamp - firstTs) / timeRange) * width;
        if(currentX >= 0 && currentX <= width) {
            ctx.beginPath();
            ctx.strokeStyle = 'rgba(59, 130, 246, 0.8)'; // blue-500
            ctx.lineWidth = 1.5;
            ctx.moveTo(currentX, 0);
            ctx.lineTo(currentX, height);
            ctx.stroke();
        }
    }
}

// Handle timeline click to jump to timestamp
document.getElementById('timelineChart').addEventListener('click', (e) => {
    if(!timelineData || !timelineData.timeline || timelineData.timeline.length === 0) return;

    const canvas = document.getElementById('timelineChart');
    const rect = canvas.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const width = canvas.width;

    const firstTs = timelineData.first_timestamp;
    const lastTs = timelineData.last_timestamp;
    const timeRange = lastTs - firstTs;

    // Calculate timestamp from click position
    const clickRatio = clickX / width;
    const targetTimestamp = firstTs + (clickRatio * timeRange);

    jumpToTimestamp(Math.floor(targetTimestamp));
});

// Handle timeline hover to show timestamp
document.getElementById('timelineChart').addEventListener('mousemove', (e) => {
    if(!timelineData || !timelineData.timeline || timelineData.timeline.length === 0) return;

    const canvas = document.getElementById('timelineChart');
    const rect = canvas.getBoundingClientRect();
    const hoverX = e.clientX - rect.left;
    const width = canvas.width;

    const firstTs = timelineData.first_timestamp;
    const lastTs = timelineData.last_timestamp;
    const timeRange = lastTs - firstTs;

    const hoverRatio = hoverX / width;
    const hoverTimestamp = firstTs + (hoverRatio * timeRange);

    const date = new Date(hoverTimestamp * 1000);
    canvas.title = `Jump to ${date.toLocaleString()}`;
});

// Fetch available time range on load
async function fetchPlaybackInfo() {
    try {
        const resp = await fetch('/api/playback/info');
        const data = await resp.json();
        firstTimestamp = data.first_timestamp;
        lastTimestamp = data.last_timestamp;

        if(firstTimestamp && lastTimestamp) {
            const duration = lastTimestamp - firstTimestamp;
            const hours = Math.floor(duration / 3600);
            const mins = Math.floor((duration % 3600) / 60);

            // Show when the data is from
            const lastDate = new Date(lastTimestamp * 1000);
            const ageSeconds = Math.floor(Date.now() / 1000) - lastTimestamp;
            const ageHours = Math.floor(ageSeconds / 3600);
            const ageMins = Math.floor((ageSeconds % 3600) / 60);

            if(ageSeconds < 60) {
                document.getElementById('timeRange').textContent =
                    `${hours}h ${mins}m (current)`;
            } else {
                document.getElementById('timeRange').textContent =
                    `${hours}h ${mins}m (${ageHours}h ${ageMins}m old)`;
            }
        }
    } catch(e) {
        console.error('Failed to fetch playback info:', e);
    }
}

// Jump to a specific timestamp and load data
async function jumpToTimestamp(timestamp) {
    if(!timestamp) return;

    currentTimestamp = timestamp;
    playbackMode = true;

    // Update time display - add visual indicator for playback mode
    const dt = new Date(timestamp * 1000);
    document.getElementById('timeDisplay').textContent =
        '⏱ ' + dt.toLocaleTimeString();
    document.getElementById('timeDisplay').style.color = '#f59e0b'; // amber color

    // Clear history buffers when entering playback mode
    cpuHistory.length = 0;
    memoryHistory.length = 0;
    netDownHistory.length = 0;
    netUpHistory.length = 0;
    Object.keys(diskIoHistoryMap).forEach(k => delete diskIoHistoryMap[k]);

    // Fetch historical data for this time point (load 60 seconds for smooth charts)
    try {
        const url = `/api/playback/events?start=${timestamp - 60}&end=${timestamp + 1}&limit=200`;

        const resp = await fetch(url);
        const data = await resp.json();

        if(data.events && data.events.length > 0) {
            // If we're using fallback data, show a visual indicator
            const timeDisplay = document.getElementById('timeDisplay');
            if(data.fallback) {
                timeDisplay.title = 'No data at this time, showing most recent available data';
            } else {
                timeDisplay.title = 'Click to select time, Shift+Click to go Live';
            }

            // Process events in order
            let latestSystemMetrics = null;
            let latestProcessSnapshot = null;

            data.events.forEach(event => {
                if(event.type === 'SystemMetrics') {
                    latestSystemMetrics = event;

                    // Build history for charts - collect all events first
                    cpuHistory.push(event.cpu || 0);
                    memoryHistory.push(event.mem || 0);
                    netDownHistory.push(event.net_recv || 0);
                    netUpHistory.push(event.net_send || 0);
                } else if(event.type === 'ProcessSnapshot') {
                    latestProcessSnapshot = event;
                } else {
                    addEventToLog(event);
                }
            });

            // Trim history arrays to keep only the most recent MAX_HISTORY items
            if(cpuHistory.length > MAX_HISTORY) {
                cpuHistory.splice(0, cpuHistory.length - MAX_HISTORY);
                memoryHistory.splice(0, memoryHistory.length - MAX_HISTORY);
                netDownHistory.splice(0, netDownHistory.length - MAX_HISTORY);
                netUpHistory.splice(0, netUpHistory.length - MAX_HISTORY);
            }

            // Render the latest state
            if(latestSystemMetrics) {
                // Merge metadata (missing static/semi-static fields) into the latest metrics
                if(data.metadata) {
                    for(const key in data.metadata) {
                        if(!latestSystemMetrics[key] || latestSystemMetrics[key] === null) {
                            latestSystemMetrics[key] = data.metadata[key];
                        }
                    }

                    // Update caches from metadata
                    if(data.metadata.mem_total_bytes) cachedMemTotal = data.metadata.mem_total_bytes;
                    if(data.metadata.swap_total_bytes) cachedSwapTotal = data.metadata.swap_total_bytes;
                    if(data.metadata.disk_total_bytes) cachedDiskTotal = data.metadata.disk_total_bytes;
                    if(data.metadata.filesystems) cachedFilesystems = data.metadata.filesystems;
                    if(data.metadata.net_ip_address) cachedNetIp = data.metadata.net_ip_address;
                    if(data.metadata.net_gateway) cachedNetGateway = data.metadata.net_gateway;
                    if(data.metadata.net_dns) cachedNetDns = data.metadata.net_dns;
                    if(data.metadata.kernel_version) cachedKernel = data.metadata.kernel_version;
                    if(data.metadata.cpu_model) cachedCpuModel = data.metadata.cpu_model;
                    if(data.metadata.cpu_mhz) cachedCpuMhz = data.metadata.cpu_mhz;
                }

                lastStats = latestSystemMetrics;
                render();
            } else {
                console.log('NO latestSystemMetrics - skipping render!');
            }
            if(latestProcessSnapshot) {
                updateProcs(latestProcessSnapshot);
            }
        } else {
            // No data found at all
            console.log('No data available');
            document.getElementById('timeDisplay').title = 'No historical data available';
        }
    } catch(e) {
        console.error('Failed to load historical data:', e);
    }

    // Update timeline visualization
    drawTimeline();
}

// Rewind button
document.getElementById('rewindBtn').addEventListener('click', () => {
    // Stop auto-playback
    if(playbackInterval) {
        clearTimeout(playbackInterval);
        playbackInterval = null;
    }

    if(!playbackMode) {
        // First click: pause and go back 1 minute from now
        const now = Math.floor(Date.now() / 1000);
        jumpToTimestamp(now - REWIND_STEP);
        isPaused = true;
        document.getElementById('pauseBtn').style.display = 'none';
        document.getElementById('playBtn').style.display = 'block';
    } else {
        // Go back 1 minute from current position
        const newTime = Math.max(firstTimestamp || 0, currentTimestamp - REWIND_STEP);
        jumpToTimestamp(newTime);
        // Show pause button after seeking
        isPaused = true;
        document.getElementById('pauseBtn').style.display = 'none';
        document.getElementById('playBtn').style.display = 'block';
    }
});

// Fast-forward button
document.getElementById('fastForwardBtn').addEventListener('click', () => {
    if(!playbackMode) return; // Only works in playback mode

    // Stop auto-playback
    if(playbackInterval) {
        clearTimeout(playbackInterval);
        playbackInterval = null;
    }

    // Advance forward by REWIND_STEP, but don't go past the latest available timestamp (or now)
    const target = currentTimestamp + REWIND_STEP;
    const maxTime = lastTimestamp || Math.floor(Date.now() / 1000);
    const newTime = Math.min(target, maxTime);
    jumpToTimestamp(newTime);
    // Show pause button after seeking
    isPaused = true;
    document.getElementById('pauseBtn').style.display = 'none';
    document.getElementById('playBtn').style.display = 'block';
});

// Pause button
document.getElementById('pauseBtn').addEventListener('click', () => {
    isPaused = true;
    document.getElementById('pauseBtn').style.display = 'none';
    document.getElementById('playBtn').style.display = 'block';

    // Stop auto-playback if running
    if(playbackInterval) {
        clearTimeout(playbackInterval);
        playbackInterval = null;
    }

    // Enter playback mode at current time
    if(!playbackMode) {
        const now = Math.floor(Date.now() / 1000);
        currentTimestamp = now;
        playbackMode = true;
    }
});

// Play button - either resume playback or return to live
document.getElementById('playBtn').addEventListener('click', async () => {
    if(playbackMode && currentTimestamp) {
        console.log('Starting auto-playback from', new Date(currentTimestamp * 1000));
        // Resume playback: auto-advance through history
        isPaused = false;
        document.getElementById('playBtn').style.display = 'none';
        document.getElementById('pauseBtn').style.display = 'block';

        // Calculate a reasonable "live" threshold - within 10 seconds of now
        const liveThreshold = Math.floor(Date.now() / 1000) - 10;

        // Auto-advance recursively (waits for each fetch to complete)
        const autoAdvance = async () => {
            // Check if still in playback mode
            if(!playbackMode) {
                console.log('Playback stopped');
                return;
            }

            if(currentTimestamp >= liveThreshold) {
                // Reached live time, switch to live mode
                goLive();
            } else {
                currentTimestamp += 1;
                await jumpToTimestamp(currentTimestamp);

                // Schedule next tick
                playbackInterval = setTimeout(autoAdvance, 1000);
            }
        };

        // Start first advance immediately
        await autoAdvance();
    } else {
        console.log('Not in playback mode, going straight to live');
        // Not in playback mode, just unpause
        goLive();
    }
});

// Return to live mode
function goLive() {
    isPaused = false;
    playbackMode = false;
    currentTimestamp = null;

    if(playbackInterval) {
        clearTimeout(playbackInterval);
        playbackInterval = null;
    }

    document.getElementById('playBtn').style.display = 'none';
    document.getElementById('pauseBtn').style.display = 'block';
    document.getElementById('timeDisplay').textContent = 'Live';
    document.getElementById('timeDisplay').style.color = '';
    document.getElementById('timeDisplay').title = 'Click to select time, Shift+Click to go Live';

    // Clear history buffers so they rebuild from live data
    cpuHistory.length = 0;
    memoryHistory.length = 0;
    netDownHistory.length = 0;
    netUpHistory.length = 0;
    Object.keys(diskIoHistoryMap).forEach(k => delete diskIoHistoryMap[k]);

    // Update timeline visualization (clears vertical line)
    drawTimeline();
}

// Time display click - either go live or open picker
document.getElementById('timeDisplay').addEventListener('click', (e) => {
    if(e.shiftKey && playbackMode) {
        // Shift+click: Go live
        goLive();
        return;
    }

    const picker = document.getElementById('timePicker');

    if(firstTimestamp && lastTimestamp) {
        // Set picker range
        const firstDate = new Date(firstTimestamp * 1000);
        const lastDate = new Date(lastTimestamp * 1000);

        picker.min = firstDate.toISOString().slice(0, 16);
        picker.max = lastDate.toISOString().slice(0, 16);

        // Set current value
        const current = currentTimestamp || Math.floor(Date.now() / 1000);
        picker.value = new Date(current * 1000).toISOString().slice(0, 16);

        picker.style.display = 'block';
        picker.focus();
    }
});

document.getElementById('timePicker').addEventListener('change', (e) => {
    const selectedDate = new Date(e.target.value);
    const timestamp = Math.floor(selectedDate.getTime() / 1000);

    jumpToTimestamp(timestamp);
    e.target.style.display = 'none';

    // Enable pause mode
    isPaused = true;
    document.getElementById('pauseBtn').style.display = 'none';
    document.getElementById('playBtn').style.display = 'block';
});

document.getElementById('timePicker').addEventListener('blur', (e) => {
    setTimeout(() => e.target.style.display = 'none', 200);
});

// Fetch playback info and initial state on startup
fetchPlaybackInfo();
fetchInitialState();
fetchTimeline();

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
                <span class="inline-block w-32 h-3 bg-gray-200 overflow-hidden align-middle ml-1" style="border-radius:1px">
                    <span id="${id}" class="block h-full transition-all duration-300" style="width:0%;border-radius:1px"></span>
                </span>
            </span>
        </div>`);
        el = document.getElementById(id);
    }
    const color = pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-yellow-500' : 'bg-green-500';
    el.style.width = Math.min(100, pct) + '%';
    el.className = `block h-full transition-all duration-300 ${color}`;
    el.style.borderRadius = '1px';
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
            <span class="relative flex-1 bg-gray-200" style="height:10px;border-radius:1px">
                <span id="${id}" class="block h-full transition-all duration-300" style="width:0%;border-radius:1px"></span>
                <span id="pct_${id}" class="absolute inset-0 flex items-center justify-center text-gray-500/60 overflow-visible"></span>
            </span>
        </div>`);
        el = document.getElementById(id);
    }
    const color = pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-yellow-500' : 'bg-green-500';
    const widthValue = Math.min(100, pct) + '%';
    updateStyleIfChanged(id, 'width', widthValue);
    updateIfChanged(`${id}_class`, color, () => {
        el.className = `block h-full transition-all duration-300 ${color}`;
    });
    updateTextIfChanged(`pct_${id}`, pct.toFixed(1) + '%');
}

function updateRamBar(pct, used, container){
    let el = document.getElementById('ramBar');
    if(!el){
        container.innerHTML = `<div class="text-gray-500 flex items-center gap-4">
            <span id="ramLabel">RAM Used: ${fmt(used)}</span>
            <span class="relative flex-1 bg-gray-200" style="height:10px;border-radius:1px">
                <span id="ramBar" class="block h-full transition-all duration-300" style="width:0%;border-radius:1px"></span>
                <span id="ramPct" class="absolute inset-0 flex items-center justify-center text-gray-500/60 overflow-visible"></span>
            </span>
        </div>`;
        el = document.getElementById('ramBar');
    }
    const color = pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-yellow-500' : 'bg-green-500';
    const widthValue = Math.min(100, pct) + '%';
    updateStyleIfChanged('ramBar', 'width', widthValue);
    updateIfChanged('ramBar_class', color, () => {
        el.className = `block h-full transition-all duration-300 ${color}`;
    });
    updateTextIfChanged('ramLabel', `RAM Used: ${fmt(used)}`);
    updateTextIfChanged('ramPct', pct.toFixed(1) + '%');
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

function drawChart(canvasId, history){
    const canvas = document.getElementById(canvasId);
    const ctx = canvas.getContext('2d');
    const dpr = window.devicePixelRatio || 1;

    // Set canvas size accounting for device pixel ratio
    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);

    const width = rect.width;
    const height = rect.height;
    const barWidth = width / MAX_HISTORY;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Draw bars from right to left (newest on right)
    history.forEach((pct, i) => {
        const x = (MAX_HISTORY - history.length + i) * barWidth;
        const barHeight = (pct / 100) * height;
        const y = height - barHeight;

        ctx.fillStyle = getUsageColor(pct);
        ctx.fillRect(x, y, barWidth, barHeight);
    });
}

function drawNetworkChart(canvasId, history){
    const canvas = document.getElementById(canvasId);
    const ctx = canvas.getContext('2d');
    const dpr = window.devicePixelRatio || 1;

    // Set canvas size accounting for device pixel ratio
    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);

    const width = rect.width;
    const height = rect.height;
    const barWidth = width / MAX_HISTORY;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Find max value for scaling
    const maxVal = Math.max(...history, 1); // At least 1 to avoid division by zero

    // Draw bars from right to left (newest on right)
    history.forEach((val, i) => {
        const x = (MAX_HISTORY - history.length + i) * barWidth;
        const pct = (val / maxVal) * 100;
        const barHeight = (val / maxVal) * height;
        const y = height - barHeight;

        ctx.fillStyle = getUsageColor(pct);
        ctx.fillRect(x, y, barWidth, barHeight);
    });
}

function updateMemoryChart(){
    drawChart('memoryChart', memoryHistory);
}

function updateCpuChart(){
    drawChart('cpuChart', cpuHistory);
}

function updateNetDownChart(){
    drawNetworkChart('netDownChart', netDownHistory);
}

function updateNetUpChart(){
    drawNetworkChart('netUpChart', netUpHistory);
}

function updateDiskBar(id, pct, container, mount, used, total){
    let el = document.getElementById(id);
    if(!el){
        container.insertAdjacentHTML('beforeend', `<div class="text-gray-500 flex items-center gap-4" id="row_${id}">
            <span id="lbl_${id}" class="flex-1">${mount}</span>
            <span><span id="used_${id}" class="text-gray-400">${fmt(used)}</span>/<span id="total_${id}">${fmt(total)}</span></span>
            <span class="relative bg-gray-200" style="height:10px;width:128px;border-radius:1px">
                <span id="${id}" class="block h-full transition-all duration-300" style="width:0%;border-radius:1px"></span>
                <span id="pct_${id}" class="absolute inset-0 flex items-center justify-center text-gray-500/60 overflow-visible"></span>
            </span>
        </div>`);
        el = document.getElementById(id);
    }
    const color = pct >= 90 ? 'bg-red-500' : pct >= 70 ? 'bg-yellow-500' : 'bg-green-500';
    const widthValue = Math.min(100, pct) + '%';
    updateStyleIfChanged(id, 'width', widthValue);
    updateIfChanged(`${id}_class`, color, () => {
        el.className = `block h-full transition-all duration-300 ${color}`;
    });
    updateTextIfChanged(`lbl_${id}`, mount);
    updateTextIfChanged(`pct_${id}`, pct + '%');
    updateTextIfChanged(`used_${id}`, fmt(used));
    updateTextIfChanged(`total_${id}`, fmt(total));
}

function updateDiskIo(disks){
    const section = document.getElementById('diskIoSection');
    const table = document.getElementById('diskIoTable');
    const tbody = document.getElementById('diskIoTableBody');

    if(!disks || disks.length === 0){
        updateStyleIfChanged('diskIoSection', 'display', 'none');
        updateStyleIfChanged('diskIoTable', 'display', 'none');
        if(prevValues['diskIoTableBody_cleared'] !== true) {
            prevValues['diskIoTableBody_cleared'] = true;
            tbody.innerHTML = '';
        }
        return;
    }

    updateStyleIfChanged('diskIoSection', 'display', 'flex');
    updateStyleIfChanged('diskIoTable', 'display', 'table');
    prevValues['diskIoTableBody_cleared'] = false;

    // Max throughput for scaling (100 MB/s = 100%)
    const maxThroughput = 100 * 1024 * 1024;

    // Update or create rows for each disk
    disks.forEach((disk, i) => {
        const deviceKey = disk.device;

        // Initialize history for this disk if needed
        if(!diskIoHistoryMap[deviceKey]){
            diskIoHistoryMap[deviceKey] = [];
        }

        // Calculate throughput percentage
        const totalThroughput = disk.read + disk.write;
        const throughputPct = Math.min(100, (totalThroughput / maxThroughput) * 100);

        // Add to history
        diskIoHistoryMap[deviceKey].push(throughputPct);
        if(diskIoHistoryMap[deviceKey].length > MAX_HISTORY){
            diskIoHistoryMap[deviceKey].shift();
        }

        // Check if row exists
        let row = document.getElementById(`diskio_row_${i}`);
        if(!row){
            const tr = document.createElement('tr');
            tr.id = `diskio_row_${i}`;
            const tempText = disk.temp ? disk.temp.toFixed(0) + '°C' : '--';
            tr.innerHTML = `
                <td style="width:60px">${disk.device}</td>
                <td class="text-right" style="width:80px"><span id="diskio_read_${i}">${fmt(disk.read)}/s</span></td>
                <td class="text-right" style="width:80px"><span id="diskio_write_${i}">${fmt(disk.write)}/s</span></td>
                <td class="text-right text-gray-400" style="width:50px"><span id="diskio_temp_${i}">${tempText}</span></td>
                <td style="width:128px;text-align:right;vertical-align:middle"><canvas id="diskio_chart_${i}" style="height:10px;width:128px;" class="ml-auto"></canvas></td>
            `;
            tbody.appendChild(tr);
        } else {
            // Update existing row (only if changed)
            const readText = fmt(disk.read) + '/s';
            const writeText = fmt(disk.write) + '/s';
            const tempText = disk.temp ? disk.temp.toFixed(0) + '°C' : '--';
            updateTextIfChanged(`diskio_read_${i}`, readText);
            updateTextIfChanged(`diskio_write_${i}`, writeText);
            updateTextIfChanged(`diskio_temp_${i}`, tempText);
        }

        // Draw chart for this disk
        drawChart(`diskio_chart_${i}`, diskIoHistoryMap[deviceKey]);
    });
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

    // Show content on first data load
    const mainContent = document.getElementById('mainContent');
    if(mainContent.style.display === 'none'){
        mainContent.style.display = 'block';
    }

    // Always show the timestamp from the event data (whether live or historical)
    if(e.timestamp) {
        const eventDate = new Date(e.timestamp);
        if(!isNaN(eventDate.getTime())) {
            updateTextIfChanged('datetime', formatDate(eventDate));
        } else {
            // Fallback if timestamp parsing fails
            console.log('Timestamp parsing failed!');
            updateTextIfChanged('datetime', formatDate(new Date()));
        }
    } else {
        console.log('No e.timestamp found');
        updateTextIfChanged('datetime', formatDate(new Date()));
    }
    const uptimeText = e.system_uptime_seconds ? `Uptime: ${formatUptime(e.system_uptime_seconds)}` : '';
    updateTextIfChanged('uptime', uptimeText);
    updateConnectionStatus();

    const kernel = e.kernel ?? cachedKernel;
    const cpuModel = e.cpu_model ?? cachedCpuModel;
    const cpuMhz = e.cpu_mhz ?? cachedCpuMhz;

    if(kernel) updateTextIfChanged('kernelRow', `Linux Kernel: ${kernel}`);
    if(cpuModel) updateTextIfChanged('cpuDetailsRow', `CPU Details: ${cpuModel}${cpuMhz ? `, ${cpuMhz}MHz` : ''}`);

    if(e.cpu !== undefined){
        // Update CPU bar
        const cpuBar = document.getElementById('cpuBar');
        const cpuPct = document.getElementById('cpuPct');
        const color = e.cpu >= 90 ? 'bg-red-500' : e.cpu >= 70 ? 'bg-yellow-500' : 'bg-green-500';
        const widthValue = Math.min(100, e.cpu) + '%';
        updateStyleIfChanged('cpuBar', 'width', widthValue);
        updateIfChanged('cpuBar_class', color, () => {
            cpuBar.className = `block h-full transition-all duration-300 ${color}`;
        });
        updateTextIfChanged('cpuPct', e.cpu.toFixed(1) + '%');

        const loadText = `Load average: ${e.load?.toFixed(2) || '--'}% ${e.load5?.toFixed(2) || '--'}% ${e.load15?.toFixed(2) || '--'}%`;
        updateTextIfChanged('loadVal', loadText);

        // Update CPU history
        cpuHistory.push(e.cpu);
        if(cpuHistory.length > MAX_HISTORY) cpuHistory.shift();
        updateCpuChart();
    }
    (e.per_core_cpu || []).forEach((v, i) => updateCoreBar(`core_${i}`, v, document.getElementById('cpuCoresContainer'), i));

    // Update cached total values when present
    if(e.mem_total != null) cachedMemTotal = e.mem_total;
    if(e.swap_total != null) cachedSwapTotal = e.swap_total;
    if(e.disk_total != null) cachedDiskTotal = e.disk_total;
    if(e.filesystems && e.filesystems.length > 0) cachedFilesystems = e.filesystems;
    if(e.net_ip != null) cachedNetIp = e.net_ip;
    if(e.net_gateway != null) cachedNetGateway = e.net_gateway;
    if(e.net_dns != null) cachedNetDns = e.net_dns;

    // Memory display - percentage is always calculated by backend
    if(e.mem !== undefined && e.mem_used !== undefined){
        const memTotal = e.mem_total ?? cachedMemTotal ?? 0;
        updateRamBar(e.mem, e.mem_used, document.getElementById('ramUsed'));
        if(memTotal > 0) {
            const availText = `Available RAM: ${fmt(memTotal - e.mem_used)}`;
            updateTextIfChanged('ramAvail', availText);
        }
        // Update memory history
        memoryHistory.push(e.mem);
        if(memoryHistory.length > MAX_HISTORY) memoryHistory.shift();
        updateMemoryChart();
    }
    if(e.cpu_temp){
        const color = e.cpu_temp >= 80 ? 'text-red-600' : e.cpu_temp >= 60 ? 'text-yellow-600' : 'text-green-600';
        const cpuTempHtml = `CPU Temp <span class="${color}">${Math.round(e.cpu_temp)}°C</span>`;
        updateHtmlIfChanged('cpuTemp', cpuTempHtml);
    } else {
        updateTextIfChanged('cpuTemp', '');
    }
    if(e.mobo_temp){
        const color = e.mobo_temp >= 80 ? 'text-red-600' : e.mobo_temp >= 60 ? 'text-yellow-600' : 'text-green-600';
        const moboTempHtml = `MB Temp <span class="${color}">${Math.round(e.mobo_temp)}°C</span>`;
        updateHtmlIfChanged('moboTemp', moboTempHtml);
    } else if(e.fans && e.fans.length > 0){
        const fan = e.fans[0];
        const fanText = `${fan.label || 'Fan'} ${fan.rpm}RPM`;
        updateTextIfChanged('moboTemp', fanText);
    } else {
        updateTextIfChanged('moboTemp', '');
    }
    // Graphics section - only show if GPU data available
    const hasGpu = e.gpu_freq || e.gpu_temp2 || e.gpu_mem_freq || e.gpu_power;
    const gpuDisplay = hasGpu ? 'flex' : 'none';
    updateStyleIfChanged('graphicsSection', 'display', gpuDisplay);
    updateStyleIfChanged('graphicsRow1', 'display', gpuDisplay);
    updateStyleIfChanged('graphicsRow2', 'display', gpuDisplay);
    if(hasGpu){
        const gpuFreqText = e.gpu_freq ? `GPU Freq ${e.gpu_freq}MHz` : '';
        updateTextIfChanged('gpuFreq', gpuFreqText);
        if(e.gpu_temp2){
            const color = e.gpu_temp2 >= 80 ? 'text-red-600' : e.gpu_temp2 >= 60 ? 'text-yellow-600' : 'text-green-600';
            const gpuTempHtml = `GPU Temp <span class="${color}">${Math.round(e.gpu_temp2)}°C</span>`;
            updateHtmlIfChanged('gpuTemp', gpuTempHtml);
        }
        const memFreqText = e.gpu_mem_freq ? `Mem Freq ${e.gpu_mem_freq}MHz` : '';
        updateTextIfChanged('memFreq', memFreqText);
        const powerText = e.gpu_power ? `Power ${e.gpu_power.toFixed(0)}W` : '';
        updateTextIfChanged('imgQuality', powerText);
    }
    const netInterface = e.net_interface || 'net';

    updateTextIfChanged('netName', `${netInterface}:`);
    updateTextIfChanged('netSpeedDown', `Down: ${fmtRate(e.net_recv || 0)}`);
    updateTextIfChanged('netSpeedUp', `Up: ${fmtRate(e.net_send || 0)}`);

    // Update network history
    netDownHistory.push(e.net_recv || 0);
    if(netDownHistory.length > MAX_HISTORY) netDownHistory.shift();
    updateNetDownChart();

    netUpHistory.push(e.net_send || 0);
    if(netUpHistory.length > MAX_HISTORY) netUpHistory.shift();
    updateNetUpChart();

    // Show RX and TX stats with errors/drops
    const rxErrors = e.net_recv_errors || 0;
    const rxDrops = e.net_recv_drops || 0;
    const txErrors = e.net_send_errors || 0;
    const txDrops = e.net_send_drops || 0;

    const rxText = `RX: ${rxErrors} err/s, ${rxDrops} drop/s`;
    const txText = `TX: ${txErrors} err/s, ${txDrops} drop/s`;
    const rxColor = (rxErrors > 0 || rxDrops > 0) ? 'text-red-600' : 'text-gray-500';
    const txColor = (txErrors > 0 || txDrops > 0) ? 'text-red-600' : 'text-gray-500';

    updateTextIfChanged('netRxStats', rxText);
    updateTextIfChanged('netTxStats', txText);
    updateIfChanged('netRxStats_class', rxColor, () => {
        document.getElementById('netRxStats').className = `flex-1 ${rxColor}`;
    });
    updateIfChanged('netTxStats_class', txColor, () => {
        document.getElementById('netTxStats').className = `flex-1 ${txColor}`;
    });

    updateTextIfChanged('netAddress', `Address: ${e.net_ip ?? cachedNetIp ?? '--'}`);
    updateTextIfChanged('netTcp', `TCP Connections: ${e.tcp || '--'}`);
    updateTextIfChanged('netGateway', `Gateway: ${e.net_gateway ?? cachedNetGateway ?? '--'}`);
    updateTextIfChanged('netDns', `DNS: ${e.net_dns ?? cachedNetDns ?? '--'}`);

    // Storage section - use cached filesystems if not in current event
    const filesystems = e.filesystems || cachedFilesystems;
    filesystems.forEach((fs, i) => {
        const pct = fs.total > 0 ? Math.round((fs.used/fs.total)*100) : 0;
        updateDiskBar(`disk_${i}`, pct, document.getElementById('diskContainer'), fs.mount, fs.used, fs.total);
    });

    // Disk IO section
    updateDiskIo(e.per_disk || []);

    // Users section
    const users = e.users || [];
    const usersDisplay = users.length > 0 ? 'flex' : 'none';
    updateStyleIfChanged('usersSection', 'display', usersDisplay);
    const userCountText = users.length > 0 ? `${users.length} logged in` : '';
    updateTextIfChanged('userCount', userCountText);

    // Only update users container if the list actually changed
    const usersKey = JSON.stringify(users);
    if(prevValues['usersContainer_data'] !== usersKey) {
        prevValues['usersContainer_data'] = usersKey;
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
}

function updateProcs(event){
    const procCountText = `${event.total_processes || 0} total ${event.running_processes || 0} running`;
    updateTextIfChanged('procCount', procCountText);

    const memTotal = lastStats?.mem_total || 0;
    const topCpu = (event.processes || []).slice().sort((a,b) => b.cpu_percent - a.cpu_percent).slice(0,5);
    const topMem = (event.processes || []).slice().sort((a,b) => b.mem_bytes - a.mem_bytes).slice(0,5);

    // Only update tables if process lists actually changed
    const topCpuKey = JSON.stringify(topCpu.map(p => `${p.pid}_${p.cpu_percent}`));
    const topMemKey = JSON.stringify(topMem.map(p => `${p.pid}_${p.mem_bytes}`));

    if(prevValues['topCpuTable_data'] !== topCpuKey) {
        prevValues['topCpuTable_data'] = topCpuKey;
        updateProcTable('topCpuTable', topCpu, memTotal);
    }

    if(prevValues['topMemTable_data'] !== topMemKey) {
        prevValues['topMemTable_data'] = topMemKey;
        updateProcTable('topMemTable', topMem, memTotal);
    }
}

function updateConnectionStatus(){
    const isConnected = ws && ws.readyState === 1;
    // Hide connection status in playback mode
    if(playbackMode) {
        document.getElementById('wsStatus').style.display = 'none';
    } else {
        document.getElementById('wsStatus').style.display = isConnected ? 'none' : 'inline';
    }
}

function connectWebSocket(){
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    ws = new WebSocket(protocol + '//' + window.location.host + '/ws');
    ws.onopen = () => {
        updateConnectionStatus();
    };
    ws.onmessage = (ev) => {
        if(isPaused) {
            console.log('WebSocket message ignored (paused)');
            return;
        }
        if(playbackMode) {
            console.log('WebSocket message ignored (playback mode)');
            return;
        }
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
    // Format timestamp (now in milliseconds) to HH:MM:SS.mmm
    const time = e.timestamp ? new Date(e.timestamp).toISOString().substring(11,23) : '--:--:--';
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
// Only update clock in live mode (when not in playback and we have live data)
setInterval(() => {
    if(!playbackMode && lastStats && lastStats.timestamp) {
        // In live mode, update the display using the live timestamp
        const eventDate = new Date(lastStats.timestamp);
        if(!isNaN(eventDate.getTime())) {
            document.getElementById('datetime').textContent = formatDate(eventDate);
        } else {
            document.getElementById('datetime').textContent = formatDate(new Date());
        }
    }
}, 1000);
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

            // Percentages are now calculated every second in main.rs using cached totals

            Some(serde_json::json!({
                "type": "SystemMetrics",
                "timestamp": m.ts.format(&Rfc3339).ok()?,
                "kernel": m.kernel_version,
                "cpu_model": m.cpu_model,
                "cpu_mhz": m.cpu_mhz,
                "system_uptime_seconds": m.system_uptime_seconds,
                "cpu": m.cpu_usage_percent,
                "per_core_cpu": m.per_core_usage,
                "mem": m.mem_usage_percent,
                "mem_used": m.mem_used_bytes,
                "mem_total": m.mem_total_bytes,
                "load": m.load_avg_1m,
                "load5": m.load_avg_5m,
                "load15": m.load_avg_15m,
                "disk": m.disk_usage_percent.round(),
                "disk_used": m.disk_used_bytes,
                "disk_total": m.disk_total_bytes,
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
                "fans": m.fans.as_ref().map(|fan_list| fan_list.iter().map(|f| serde_json::json!({
                    "label": f.label,
                    "rpm": f.rpm,
                })).collect::<Vec<_>>()).unwrap_or_default(),
                "users": m.logged_in_users.as_ref().map(|user_list| user_list.iter().map(|u| serde_json::json!({
                    "username": u.username,
                    "terminal": u.terminal,
                    "remote_host": u.remote_host,
                })).collect::<Vec<_>>()).unwrap_or_default(),
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
