<p align="center">
  <img width="50" height="50" alt="black-box" src="https://github.com/user-attachments/assets/94d70715-e9fe-4d74-8e3e-7a591e9ab543" />
</p>

<h1 align="center">Black Box</h1>

A lightweight, always-on forensics recorder for Linux servers. Captures system metrics, process events, and security events to help with post-incident analysis.

## Features

- Always-on monitoring with minimal overhead
- Fixed disk usage (100MB ring buffer)
- Configuration via `config.toml`
- Single static binary
- Real-time WebSocket streaming with a web UI
- HTTP Basic Authentication for security
- Health monitoring endpoint

## What It Captures

### System Metrics (1s interval)
- CPU, Memory, Swap usage
- Disk I/O and space
- Network I/O
- Load average
- TCP connections
- Context switches

### Process Intelligence
- Lifecycle events (start/exit/stuck/zombie)
- Full command lines
- Thread and file descriptor counts
- Memory and disk I/O per process
- Top 10 resource consumers (every 5s)

### Security Events (5s interval)
- Logged-in users
- SSH authentication (success/failure)
- Sudo commands
- Failed login attempts
- Brute force detection (5+ failures in 5 minutes)
- Port scan detection (20+ ports from same IP)

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

### Command Line Options
```bash
# Run with custom port
./black-box --port 9000

# Run without web UI (headless mode)
./black-box --headless
```

### Web UI Features
- Real-time WebSocket streaming - Events pushed to browser instantly
- HTTP Basic Authentication - Secure access with username/password
- Search/filter events
- Filter by event type (System/Process/Security/Anomalies)
- Terminal-like aesthetic with color coding
- Auto-reconnect on disconnect

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

## Building

```bash
cargo build --release
```

Binary will be at `target/release/black-box` (single file, ~3.5MB).

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

## Permissions

Most features work as regular user. For full security monitoring:
- Add user to `adm` group for auth log access: `sudo usermod -aG adm username`
- Or run with sudo (not recommended for continuous operation)

## API Endpoints

### `/health` - Health Check
Returns JSON with system status, uptime, event count, and storage usage.

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

## Architecture

- **Recorder**: Collects events and writes to binary log files (synchronous loop)
- **Storage**: Segmented ring buffer (8MB segments, max 12 segments = ~100MB)
- **Reader**: Deserializes binary log files
- **Broadcaster**: Bridges sync collector to async WebSocket clients
- **Web Server**: Actix-web async HTTP server with WebSocket support and Basic Auth

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
2. WebSocket streams events in real-time
3. Use filters to search time range around incident
4. Look for anomalies flagged automatically (red highlights)
5. Check what processes were running (Process events)
6. Review security events (SSH logins, sudo usage)
7. See exact resource usage before crash (System metrics)
8. Export data via `/api/events` endpoint for external analysis

All data is timestamped and correlated - you can "rewind" to any point in time.

## Security Considerations

- **Authentication**: HTTP Basic Auth protects all endpoints including WebSocket
- **Passwords**: Stored as bcrypt hashes (cost factor 12)
- **Production**: Use HTTPS via reverse proxy (nginx, Caddy) for TLS encryption
- **Network**: Bind to localhost only if running on same machine
- **Credentials**: Never commit `config.toml` with real passwords to version control
