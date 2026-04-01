<p align="center">
  <img width="50" height="50" alt="black-box" src="https://github.com/user-attachments/assets/94d70715-e9fe-4d74-8e3e-7a591e9ab543" />
</p>

<h1 align="center">Black Box</h1>

<p align="center" height="10">
  <img width="830" height="10" alt="network_chart" src="https://github.com/user-attachments/assets/f24206fb-3b7e-4f2f-99ef-bf32dc641a3e" />
</p>

A lightweight, always-on forensics recorder for Linux servers. Captures system metrics, process events, and security events to help with post-incident analysis.

Ideal for tracking malicious activity, monitoring AI agents, and reviewing errors.

<p align="center">
  <img width="520" height="890" alt="Screenshot_20260131_105934" src="https://github.com/user-attachments/assets/78c9329b-ba8b-4ee9-ac66-bb3818661600" />
</p>

## Key Features

- Continuous monitoring with very low overhead  
- Real-time streaming via a built-in web UI  
- Time-travel playback to query historical events by timestamp or range  
- Tamper-resistant modes (append-only or fully immutable logs)  
- Timeline view showing event density and resource usage  
- JSON export for external analysis  
- Optional remote monitoring with health checks and auto-export  
- Fixed disk usage using a ring buffer  
- HTTP Basic Auth support
- Systemd integration  
- Ships as a single static binary  

## What It Captures

- **System metrics** (1s): per-core CPU, memory, swap, disk I/O (per disk + temps), network (throughput, TCP connections, errors/drops), load averages, temperatures, GPU stats, and per-mount filesystem usage  
- **System info**: kernel version, CPU model, and hardware details (captured at startup and hourly)  
- **Process events**: lifecycle tracking (start/exit/stuck), command lines, resource usage, and top consumers  
- **Security events** (5s): user logins, SSH activity, sudo usage, and basic brute-force/port-scan detection  
- **Filesystem events**: file creates, deletes, and modifications with paths and sizes  
- **Anomaly detection**: resource spikes, disk-full warnings, stuck processes, and thread/connection leaks  

## Usage

### Building

```bash
cargo build --release
```

Binary will be at `target/release/black-box`.

### Start Black Box

```bash
./black-box
```

This starts:
- Data recording to `./data/` directory by default (configurable, see Configuration section)
- Web UI at `http://localhost:8080`
- WebSocket/REST API for events, playback, and monitoring

On first run, Black Box will generate a `config.toml` file with default credentials. If using authentication, change the default password immediately. See configuration details below.

**Note:** For production deployments, configure a dedicated data directory like `/var/lib/black-box/` or `~/.local/share/black-box/` in `config.toml`.

**Monitor Mode (Lightweight):**
```bash
./black-box monitor
```

Runs data collection only, without the web UI or API endpoints. Use this for minimal overhead when you only need recording.

#### Command Line Options

```bash
# Run with custom port
./black-box --port 9000

# Run in monitor mode (lightweight, no web UI)
./black-box monitor

# Run with tamper protection (append-only files)
./black-box --protected

# Run with hardened protection (immutable until stop)
./black-box --hardened

# Export events (supports --compress, --format csv, --event-type filter)
./black-box export -o events.json
./black-box export --start "2026-01-15T10:00:00Z" --end "2026-01-15T11:00:00Z" -o range.json

# Check status (supports --format json, --username/--password for auth)
./black-box status

# Watch remote instance (health checks + auto-export on failure)
./black-box watch http://server:8080 --interval 60 --export-dir ./backups

# Generate systemd service
./black-box systemd generate
```

## Configuration

Black Box uses a `config.toml` file for settings. On first run, it generates a default config:

```toml
[auth]
enabled = true
username = "admin"
password_hash = "$2b$12$..."  # bcrypt hash of "admin"

[server]
port = 8080
data_dir = "./data"  # For development. Use "/var/lib/black-box" for production
max_storage_mb = 100  # Maximum storage size in MB (default: 100MB)
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

### Data Directory

**Default:** `./data` (current directory)

For production deployments, use a dedicated system directory:

```toml
[server]
port = 8080
data_dir = "/var/lib/black-box"  # Recommended for production
max_storage_mb = 100
```

**Recommended locations:**
- **Production (systemd service):** `/var/lib/black-box/`
- **User service:** `~/.local/share/black-box/`
- **Development/testing:** `./data` (default)

Create the directory and set permissions:
```bash
sudo mkdir -p /var/lib/black-box
sudo chown black-box:black-box /var/lib/black-box  # If running as dedicated user
```

### Configuring Storage Size

The ring buffer size can be adjusted based on your needs:

```toml
[server]
max_storage_mb = 100  # Default: 100MB

# Examples:
# max_storage_mb = 50   # Low disk usage (50MB)
# max_storage_mb = 500  # Medium retention (500MB)
# max_storage_mb = 1000 # High retention (1GB)
```

Storage is organized into 8MB segments. The system keeps approximately `max_storage_mb / 8` segments in a ring buffer, automatically deleting the oldest when the limit is reached.

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

## Contributions

Contributions, issues and feature requests are welcome.

- Fork it (https://github.com/tom-draper/black-box)
- Create your feature branch (`git checkout -b my-new-feature`)
- Commit your changes (`git commit -am 'Add some feature'`)
- Push to the branch (`git push origin my-new-feature`)
- Create a new Pull Request

----

If you find value in my work, consider supporting me.

Buy Me a Coffee: https://www.buymeacoffee.com/tomdraper<br>
PayPal: https://www.paypal.com/paypalme/tomdraper
