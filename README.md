# Black Box - Server Forensics Recorder

A lightweight, always-on forensics recorder for Linux servers. Captures system metrics, process events, and security events to help with post-incident analysis.

## Features

- Always-on monitoring with minimal overhead
- Fixed disk usage (100MB ring buffer)
- No configuration required
- Single static binary
- Terminal-like web UI for time-rewind analysis

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

### Start Black Box (Recorder + Web UI)
```bash
./black-box
```

This starts:
- Data recording to `./data/` directory
- Web UI at `http://localhost:8080` (after 2 second startup delay)

### Command Line Options
```bash
# Run with custom port
./black-box --port 9000

# Run without web UI (headless mode)
./black-box --no-ui
./black-box --headless

# Combine options
./black-box --port 9000
```

### Web UI Features
- Real-time event stream with auto-refresh
- Search/filter events
- Filter by event type (System/Process/Security/Anomalies)
- Terminal-like aesthetic with color coding

## Building

```bash
cargo build --release
```

Binary will be at `target/release/black-box` (single file, ~1.4MB).

## Permissions

Most features work as regular user. For full security monitoring:
- Add user to `adm` group for auth log access: `sudo usermod -aG adm username`
- Or run with sudo (not recommended for continuous operation)

## Architecture

- **Recorder**: Collects events and writes to binary log files
- **Storage**: Segmented ring buffer (8MB segments, max 12 segments = ~100MB)
- **Reader**: Deserializes binary log files
- **Web UI**: HTTP server with JSON API and terminal UI

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

Server crashed at 3am. You have the black box running.

1. Open web UI: `./black-box ui`
2. Search for time range around incident
3. Look for anomalies flagged automatically
4. Check what processes were running
5. Review security events (SSH logins, sudo usage)
6. See exact resource usage before crash

All data is timestamped and correlated - you can "rewind" to any point in time.

## License

MIT
