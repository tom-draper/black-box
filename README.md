<p align="center">
  <img width="50" height="50" alt="black-box" src="https://github.com/user-attachments/assets/94d70715-e9fe-4d74-8e3e-7a591e9ab543" />
</p>

<h1 align="center">Black Box</h1>

<p align="center">
  <img width="830" height="10" alt="network_chart" src="https://github.com/user-attachments/assets/bd135275-1da5-4ebf-a40f-29376e92c62a" />
</p>

A lightweight, always-on forensics recorder for Linux servers. Captures system metrics, process events, and security events to help with post-incident analysis.

Ideal for tracking malicious activity, monitoring AI agents, and reviewing errors.

## Key Features

- Always-on monitoring with minimal overhead
- Real-time streaming with a web UI
- Time-travel playback - query historical events by timestamp or time range
- Tamper protection modes (append-only or immutable log files)
- Timeline visualization with event density and resource usage
- Export events to JSON for external analysis
- Remote monitoring with health checks and auto-export
- Fixed disk usage (ring buffer)
- HTTP Basic Authentication for security
- Systemd integration
- Single static binary

<p align="center">
  <img width="520" height="890" alt="Screenshot_20260131_105934" src="https://github.com/user-attachments/assets/ff1de6b3-2961-4464-93e1-b7c485efc7c4" />
</p>

## What It Captures

### System Metrics (1s interval)
- CPU, Memory, Swap usage (overall and per-core)
- Disk I/O and space (per-disk metrics and temperatures)
- Network I/O (bytes sent/received, errors, drops)
- Load average (1m, 5m, 15m)
- TCP connections and time-wait states
- Context switches
- Hardware monitoring (CPU/GPU/motherboard temperatures, fan speeds)
- GPU metrics (frequency, memory frequency, power consumption)
- Filesystem usage (per mount point)
- System uptime

### Process Intelligence
- Lifecycle events (start/exit/stuck/zombie)
- Full command lines and working directories
- Process metadata (PID, PPID, user, UID, exit codes)
- Thread counts and resource usage
- Memory and CPU usage per process
- Top resource consumers snapshot (every 5s)
- Total and running process counts

### Security Events (5s interval)
- Logged-in users
- SSH authentication (success/failure)
- Sudo commands
- Failed login attempts
- Brute force detection (5+ failures in 5 minutes)
- Port scan detection (20+ ports from same IP)

### File System Events
- File modifications, creations, deletions
- File paths and sizes

### Anomaly Detection
- CPU spike (>80%)
- Memory spike (>90%)
- Swap usage (>50%)
- Disk full (>90%)
- Disk I/O spike (>100MB/s)
- Network spike (>500MB/s)
- Context switch spike (>50k/s)
- Process stuck in D state
- Thread/connection leaks

## Usage

### Building

```bash
cargo build --release
```

Binary will be at `target/release/black-box` (single file, ~3.5MB).

### Start Black Box

```bash
./black-box
```

This starts:
- Data recording to `./data/` directory
- Web UI at `http://localhost:8080` with authentication (default: admin/admin)
- Events API at `http://localhost:8080/api/events` with authentication (default: admin/admin)
- Health endpoint at `http://localhost:8080/health`

On first run, Black Box will create a `config.toml` file with default credentials. If using authentication, change the default password immediately. See configuration details below.

#### Command Line Options

```bash
# Run with custom port
./black-box --port 9000

# Run in headless mode (no web UI, data collection only)
./black-box --headless

# Run with tamper protection (append-only files)
./black-box --protected

# Run with hardened protection (immutable until stop)
./black-box --hardened

# Export recorded events to JSON
./black-box export -o events.json

# Export events from a time range
./black-box export --start "2026-01-15T10:00:00Z" --end "2026-01-15T11:00:00Z"

# Check status of running instance
./black-box status

# Monitor health and auto-export on failure
./black-box monitor --interval 60 --export-dir ./backups

# Generate systemd service
./black-box systemd generate
```

## Configuration

Black Box uses a `config.toml` file for settings. On first run, it creates a default config:

```toml
[auth]
enabled = true
username = "admin"
password_hash = "$2b$12$..."  # bcrypt hash of "admin"

[server]
port = 8080
data_dir = "./data"
```

### Changing the Password

**Option 1: Generate hash manually**
```bash
# Use Python with bcrypt
python3 -c "import bcrypt; print(bcrypt.hashpw(b'your-password', bcrypt.gensalt()).decode())"
```

Then update the `password_hash` in `config.toml`.

**Option 2: Edit config and let bcrypt do it**

The password is hashed using bcrypt with cost factor 12 for security. Never store plaintext passwords in the config file.

### Disabling Authentication

To disable authentication (not recommended for production):

```toml
[auth]
enabled = false
```

## Protection Modes

Black Box supports tamper protection to prevent attackers from deleting evidence:

### Default Mode
No special protection. Log files can be modified or deleted.

### Protected Mode (`--protected`)
Uses `chattr +a` to make log files append-only. Files cannot be modified or deleted, only appended to. Useful for preventing evidence tampering while still allowing graceful shutdown.

### Hardened Mode (`--hardened`)
Maximum protection. Log files are made immutable during recording. Cannot be stopped gracefully - requires system reboot or manual intervention to stop. Use this when you need the highest level of tamper resistance.

**Requirements:**
- Root/sudo access (for `chattr` commands)
- ext4 or similar filesystem with attribute support

**Example:**
```bash
# Run with append-only protection
sudo ./black-box --protected

# Run with maximum protection (cannot stop without force)
sudo ./black-box --hardened
```

## Permissions

Most features work as a regular user. For enhanced capabilities:

**Security event monitoring:**
- Add user to `adm` group for auth log access: `sudo usermod -aG adm username`
- Required for SSH login monitoring, sudo command tracking, and failed auth detection

**Tamper protection modes:**
- `--protected` and `--hardened` modes require root/sudo access
- Uses `chattr` filesystem attributes to prevent log tampering
- Requires ext4 or similar filesystem with extended attribute support

## API Endpoints



### `/api/events` - REST API
Get recent events (last 1000) with optional filtering.

```bash
# All events
curl -u admin:admin http://localhost:8080/api/events

# Filter by type
curl -u admin:admin "http://localhost:8080/api/events?type=anomaly"

# Search events
curl -u admin:admin "http://localhost:8080/api/events?filter=ssh"
```

### `/ws` - WebSocket Stream
Real-time event streaming via WebSocket. Requires Basic Auth in the connection request.

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');
ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    console.log('Event:', data);
};
```

### `/api/playback/info` - Playback Time Range
Get the time range of available historical data.

```bash
curl -u admin:admin http://localhost:8080/api/playback/info
```

Response:
```json
{
  "first_timestamp": 1705320000,
  "last_timestamp": 1705323600,
  "first_timestamp_iso": "2026-01-15T10:00:00Z",
  "last_timestamp_iso": "2026-01-15T11:00:00Z",
  "segment_count": 5,
  "estimated_event_count": 3600
}
```

### `/api/playback/events` - Historical Events
Query historical events with two modes:

**Mode 1: Count-based** - Get last N SystemMetrics before a timestamp:
```bash
# Get last 60 SystemMetrics before timestamp
curl -u admin:admin "http://localhost:8080/api/playback/events?timestamp=1705323600&count=60"

# Get events BEFORE timestamp (for progressive loading)
curl -u admin:admin "http://localhost:8080/api/playback/events?timestamp=1705323600&count=60&before=true"
```

### `/health` - Health Check
Returns JSON with system status, uptime, event count, and storage usage.



**Mode 2: Range-based** - Get all events in a time range:
```bash
# Get all events between start and end (up to limit)
curl -u admin:admin "http://localhost:8080/api/playback/events?start=1705320000&end=1705323600&limit=1000"
```

### `/api/initial-state` - Initial State
Get the most recent complete SystemMetrics for page initialization.

```bash
curl -u admin:admin http://localhost:8080/api/initial-state
```

### `/api/timeline` - Event Timeline
Get event density timeline with CPU and memory usage for visualization.

```bash
curl -u admin:admin http://localhost:8080/api/timeline
```

Response includes per-minute buckets with event counts and average CPU/memory usage.

```bash
curl -u admin:admin http://localhost:8080/health
```

Response:
```json
{
  "status": "healthy",
  "uptime_seconds": 3600,
  "event_count": 15000,
  "storage_bytes_used": 52428800,
  "storage_bytes_max": 104857600,
  "storage_percent": "50.00",
  "timestamp": "2026-01-15T10:30:00Z"
}
```

## CLI Commands

### Export Events

Export recorded events to JSON for external analysis or archival:

```bash
# Export all events to JSON file
./black-box export -o events.json

# Export with compression
./black-box export -o events.json.gz --compress

# Export specific time range
./black-box export \
  --start "2026-01-15T10:00:00Z" \
  --end "2026-01-15T11:00:00Z" \
  -o events.json

# Export only specific event type
./black-box export --event-type SystemMetrics -o metrics.json

# Export from custom data directory
./black-box export --data-dir /path/to/data -o events.json
```

### Monitor Health

Monitor a Black Box instance and automatically export data on failure:

```bash
# Monitor with 60 second intervals
./black-box monitor --interval 60 --export-dir ./backups

# Monitor with authentication
./black-box monitor \
  --url http://server:8080 \
  --username admin \
  --password secret \
  --export-dir ./backups

# Continuous backup (export on every check, not just failures)
./black-box monitor --continuous --export-dir ./backups
```

### Check Status

Query the health endpoint and display status:

```bash
# Check local instance
./black-box status

# Check remote instance with authentication
./black-box status \
  --url http://server:8080 \
  --username admin \
  --password secret

# JSON output for scripting
./black-box status --format json
```

## Binary Format

```
Segment file:
[MAGIC: 0xBB10_0001 (4 bytes)]
[Record Header: timestamp_ns (16 bytes) + payload_len (4 bytes)]
[Payload: bincode-serialized Event]
[Record Header...]
[Payload...]
...
```

## Use Cases

- Post-incident forensics ("what happened before the crash?")
- Performance debugging ("why was the server slow at 3am?")
- Security investigation ("who logged in and what did they do?")
- Capacity planning (historical resource usage)
- Detecting stuck database queries, memory leaks, connection leaks

## Example Incident Response

Server crashed at 3am. You have Black Box running.

1. Open web UI: `http://localhost:8080` (login with your credentials)
2. Click the timeline at the top to see event density and resource usage over time
3. Use the time picker or rewind/fast-forward buttons to navigate to 3am
4. Look for anomalies flagged automatically (red highlights)
5. Check what processes were running at that time (Process events)
6. Review security events (SSH logins, sudo usage)
7. See exact resource usage before crash (System metrics with CPU/memory graphs)
8. Export data via `/api/playback/events` for external analysis

All data is timestamped and correlated - you can travel back to any point in time within the retention window.

## Contributions

Contributions, issues and feature requests are welcome.

- Fork it (https://github.com/tom-draper/nginx-analytics)
- Create your feature branch (`git checkout -b my-new-feature`)
- Commit your changes (`git commit -am 'Add some feature'`)
- Push to the branch (`git push origin my-new-feature`)
- Create a new Pull Request

----

If you find value in my work, consider supporting me.

Buy Me a Coffee: https://www.buymeacoffee.com/tomdraper<br>
PayPal: https://www.paypal.com/paypalme/tomdraper
