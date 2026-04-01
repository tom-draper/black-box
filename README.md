<p align="center">
  <img width="50" height="50" alt="black-box" src="https://github.com/user-attachments/assets/94d70715-e9fe-4d74-8e3e-7a591e9ab543" />
</p>

<h1 align="center">Black Box</h1>



<p align="center">A small always-on recorder for Linux machines.</p>

<p align="center" height="10">
  <img width="830" height="10" alt="network_chart" src="https://github.com/user-attachments/assets/35499b1c-361d-4d32-8700-95db23476d39" />
</p>

Black Box keeps a rolling history of your system’s activity, so you can quickly answer questions like: what spiked, what started, who logged in, what changed, and how resources looked at any given moment.

It is built for the situations where normal monitoring falls short:

- a process went rogue for a minute and then vanishes
- an AI agent misbehaves and you need a clear timeline
- the server slows down overnight and the cause is a mystery
- you suspect tampering and need an audit trail you can trust

<p align="center">
  <img width="520" height="890" alt="Black Box screenshot" src="https://github.com/user-attachments/assets/78c9329b-ba8b-4ee9-ac66-bb3818661600" />
</p>

## Why It Exists

Most tools are built either for live dashboards or for heavy log pipelines.

Black Box sits in the middle. It records enough to reconstruct what happened, stores it locally in a fixed-size ring buffer, and gives you a built-in UI to scrub backwards through time. You leave it running and only care about it when you need it.

## What It Records

Black Box continuously records:

- system state: CPU, memory, swap, load, temperatures, GPU, disk usage, disk I/O, network activity, TCP connections
- process activity: starts, exits, stuck processes, top CPU and memory users
- security-relevant events: logins, SSH activity, sudo usage, failed auth patterns, basic brute-force and port-scan signals
- filesystem changes: creates, deletes, and modifications
- anomalies: spikes, drops, leaks, and other suspicious changes worth flagging

It also captures static machine details like kernel version and CPU model so old recordings still make sense later.

## Quick Start

Build it:

```bash
cargo build --release
```

Run it:

```bash
./target/release/black-box
```

That starts recording to `./data/` by default and opens the web UI on `http://localhost:8080`.

On first run, Black Box creates a `config.toml` with default credentials. If auth is enabled, change the password before exposing it anywhere.

If you only want recording and do not need the UI:

```bash
./target/release/black-box monitor
```

## Common Commands

```bash
# Run the web UI on a different port
./black-box --port 9000

# Record only, no UI
./black-box monitor

# Append-only recording
sudo ./black-box --protected

# Stronger tamper resistance
sudo ./black-box --hardened

# Export a time range
./black-box export --start "2026-01-15T10:00:00Z" --end "2026-01-15T11:00:00Z" -o range.json

# Check status
./black-box status

# Watch a remote instance and auto-export on failure
./black-box watch http://server:8080 --interval 60 --export-dir ./backups

# Generate a systemd unit
./black-box systemd generate
```

## Configuration

Black Box uses `config.toml`. The generated default is small:

```toml
[auth]
enabled = true
username = "admin"
password_hash = "$2b$12$..."

[server]
port = 8080
data_dir = "./data"
max_storage_mb = 100
```

The main settings most people care about are:

- `data_dir`: where recordings live
- `max_storage_mb`: how much disk to use before old data is overwritten
- `port`: web UI port
- `auth.enabled`: whether the UI/API requires login

For production, use a real data directory such as `/var/lib/black-box` instead of `./data`.

### Passwords

Passwords are stored as bcrypt hashes. If you want to set one manually:

```bash
python3 -c "import bcrypt; print(bcrypt.hashpw(b'your-password', bcrypt.gensalt()).decode())"
```

Put the result into `password_hash`.

If you disable auth, do it deliberately:

```toml
[auth]
enabled = false
```

## Retention

Storage is fixed-size. Black Box writes into a ring buffer and overwrites the oldest segments when the limit is reached.

That means disk usage stays predictable, but retention depends on how busy the machine is and how much space you give it.

## Protection Modes

Black Box can make recordings harder to remove after the fact.

`--protected`
Uses append-only file attributes. Data can be added, but existing evidence cannot be edited or deleted normally.

`--hardened`
Uses immutable protection while recording. This is much harder to tamper with, but it is also much less convenient operationally.

These modes need root and a filesystem that supports the required attributes, such as ext4.

## Permissions

You can run Black Box as a normal user, but some data sources need extra access.

Useful examples:

- add the user to `adm` if you want auth-log-based security events
- use `sudo` for `--protected` or `--hardened`

## When It Fits

Black Box is a good fit when you want:

- local history on a single Linux box
- low overhead and fixed storage
- something you can leave running and inspect later
- a timeline that helps explain short-lived incidents

It is not trying to replace a full observability stack, SIEM, or centralized log platform.

## Contributions

Contributions, issues and feature requests are welcome.

- Fork it (https://github.com/tom-draper/black-box)
- Create your feature branch (`git checkout -b my-new-feature`)
- Commit your changes (`git commit -am 'Add some feature'`)
- Push to the branch (`git push origin my-new-feature`)
- Create a new Pull Request

---

If you find value in my work, consider supporting me.

Buy Me a Coffee: https://www.buymeacoffee.com/tomdraper  
PayPal: https://www.paypal.com/paypalme/tomdraper
