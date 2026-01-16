mod broadcast;
mod collector;
mod config;
mod event;
mod reader;
mod recorder;
mod storage;
mod webui;

use anyhow::Result;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use time::OffsetDateTime;

use broadcast::EventBroadcaster;
use config::Config;

use collector::{
    diff_processes, get_top_processes, read_all_cpu_stats, read_context_switches, read_disk_space,
    read_disk_stats_per_device, read_disk_temperatures, read_fan_speeds, read_load_avg,
    read_logged_in_users, read_memory_stats, read_network_stats, read_per_core_temperatures,
    read_processes, read_swap_stats, read_tcp_stats, read_temperatures, tail_auth_log,
    AuthEventType, ConnectionTracker,
};
use event::{
    Anomaly, AnomalyKind, AnomalySeverity, Event, PerDiskMetrics, ProcessInfo, ProcessLifecycle,
    ProcessLifecycleKind, ProcessSnapshot as EventProcessSnapshot, SecurityEvent,
    SecurityEventKind, SystemMetrics, TemperatureReadings,
};
use recorder::Recorder;

const COLLECTION_INTERVAL_SECS: u64 = 1;
const TOP_PROCESSES_COUNT: usize = 10;
const PROCESS_SNAPSHOT_INTERVAL: u64 = 5; // Snapshot top processes every 5 seconds
const SECURITY_CHECK_INTERVAL: u64 = 5; // Check security events every 5 seconds

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Check for --no-ui or --headless flag
    let disable_ui = args.iter().any(|arg| arg == "--no-ui" || arg == "--headless");

    // Load configuration
    let config = Config::load()?;

    // Parse port (command line overrides config)
    let port = args
        .iter()
        .position(|arg| arg == "--port")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|p| p.parse().ok())
        .unwrap_or(config.server.port);

    let data_dir = config.server.data_dir.clone();

    // Create broadcast channel for event streaming
    let (broadcast_tx, broadcaster) = EventBroadcaster::new();

    // Start async web server (unless disabled)
    if !disable_ui {
        let data_dir_clone = data_dir.clone();
        let config_clone = config.clone();
        let broadcaster = Arc::new(broadcaster);

        // Spawn Tokio runtime in background thread
        std::thread::spawn(move || {
            // Give recorder a moment to start
            std::thread::sleep(std::time::Duration::from_secs(2));

            // Create Tokio runtime
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create Tokio runtime: {}", e);
                    return;
                }
            };

            // Run async web server
            rt.block_on(async {
                if let Err(e) =
                    webui::start_server(data_dir_clone, port, broadcaster, config_clone).await
                {
                    eprintln!("Web UI failed to start: {}", e);
                }
            });
        });
    }

    // Run recorder in main thread with broadcasting
    let mut recorder = Recorder::open_with_broadcast(&data_dir, broadcast_tx)?;

    println!("Black Box");
    println!();
    println!("Data directory: {}", data_dir);
    println!("Max storage: ~100MB (ring buffer)");
    println!("Collection interval: {}s", COLLECTION_INTERVAL_SECS);
    println!("Tracking: CPU, Memory, Swap, Disk, Network, TCP, Load, Processes");
    if !disable_ui {
        println!("Web UI: http://localhost:{}", port);
        if config.auth.enabled {
            println!("Auth: Enabled (username: {})", config.auth.username);
        } else {
            println!("Auth: Disabled");
        }
    } else {
        println!("Web UI: Disabled");
    }
    println!();
    println!("Press Ctrl+C to stop\n");

    // Initialize baseline metrics
    let mut prev_cpu_snapshot = read_all_cpu_stats()?;
    let mut prev_disk_snapshot = read_disk_stats_per_device()?;
    let mut prev_network = read_network_stats()?;
    let mut prev_ctxt = read_context_switches()?;
    let mut prev_processes = read_processes()?;

    // Initialize security monitoring
    let mut auth_log_position = 0u64;
    let mut connection_tracker = ConnectionTracker::new();
    let mut prev_logged_in_users: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    // Track failed login attempts for brute force detection
    let mut failed_logins: std::collections::HashMap<String, Vec<std::time::Instant>> =
        std::collections::HashMap::new();

    // Thresholds for anomaly detection
    let cpu_spike_threshold = 80.0;
    let mem_spike_threshold = 90.0;
    let swap_usage_threshold = 50.0; // Start warning if swap is used
    let disk_full_threshold = 90.0;
    let disk_spike_threshold = 100 * 1024 * 1024; // 100 MB/s
    let network_spike_threshold = 500 * 1024 * 1024; // 500 MB/s
    let ctxt_spike_threshold = 50000; // 50k context switches per second

    loop {
        thread::sleep(Duration::from_secs(COLLECTION_INTERVAL_SECS));

        // CPU stats
        let cpu_snapshot = read_all_cpu_stats()?;
        let per_core_usage = cpu_snapshot.per_core_usage(&prev_cpu_snapshot);
        let cpu_usage = cpu_snapshot.aggregate.usage_percent(&prev_cpu_snapshot.aggregate);

        // Disk stats
        let disk_snapshot = read_disk_stats_per_device()?;
        let per_disk_throughput = disk_snapshot.per_disk_throughput(
            &prev_disk_snapshot,
            COLLECTION_INTERVAL_SECS as f32,
        );
        let (disk_read_per_sec, disk_write_per_sec) =
            disk_snapshot.total.bytes_per_sec(&prev_disk_snapshot.total, COLLECTION_INTERVAL_SECS as f32);

        // Other existing stats
        let mem_stats = read_memory_stats()?;
        let swap_stats = read_swap_stats()?;
        let disk_space = read_disk_space()?;
        let load_avg = read_load_avg()?;
        let network_stats = read_network_stats()?;
        let ctxt_stats = read_context_switches()?;
        let tcp_stats = read_tcp_stats()?;
        let current_processes = read_processes()?;

        // Temperature and fans
        let temps = read_temperatures();
        let per_core_temps = read_per_core_temperatures(per_core_usage.len());
        let disk_temps = read_disk_temperatures();
        let fans = read_fan_speeds();

        // Calculate throughput
        let (net_recv_per_sec, net_send_per_sec) =
            network_stats.bytes_per_sec(&prev_network, COLLECTION_INTERVAL_SECS as f32);
        let ctxt_per_sec = ctxt_stats.per_sec(&prev_ctxt, COLLECTION_INTERVAL_SECS as f32);

        // Build per-disk metrics with temperatures
        let per_disk_metrics: Vec<PerDiskMetrics> = per_disk_throughput
            .into_iter()
            .map(|(dev_name, read_ps, write_ps)| {
                PerDiskMetrics {
                    device_name: dev_name.clone(),
                    read_bytes_per_sec: read_ps,
                    write_bytes_per_sec: write_ps,
                    temp_celsius: disk_temps.get(&dev_name).and_then(|t| *t),
                }
            })
            .collect();

        // Record system metrics
        let system_metrics = SystemMetrics {
            ts: OffsetDateTime::now_utc(),
            cpu_usage_percent: cpu_usage,
            per_core_usage,
            mem_used_bytes: mem_stats.used_kb() * 1024,
            mem_total_bytes: mem_stats.total_kb * 1024,
            swap_used_bytes: swap_stats.used_kb() * 1024,
            swap_total_bytes: swap_stats.total_kb * 1024,
            load_avg_1m: load_avg.load_1m,
            load_avg_5m: load_avg.load_5m,
            load_avg_15m: load_avg.load_15m,
            disk_read_bytes_per_sec: disk_read_per_sec,
            disk_write_bytes_per_sec: disk_write_per_sec,
            disk_used_bytes: disk_space.used_bytes,
            disk_total_bytes: disk_space.total_bytes,
            per_disk_metrics,
            net_recv_bytes_per_sec: net_recv_per_sec,
            net_send_bytes_per_sec: net_send_per_sec,
            tcp_connections: tcp_stats.total_connections,
            tcp_time_wait: tcp_stats.time_wait,
            context_switches_per_sec: ctxt_per_sec,
            temps: TemperatureReadings {
                cpu_temp_celsius: temps.cpu_temp_celsius,
                per_core_temps,
                gpu_temp_celsius: temps.gpu_temp_celsius,
                motherboard_temp_celsius: temps.motherboard_temp_celsius,
            },
            fans,
        };
        recorder.append(&Event::SystemMetrics(system_metrics))?;

        // Track process lifecycle changes
        let proc_diff = diff_processes(&prev_processes, &current_processes);

        for proc in &proc_diff.started {
            let event = ProcessLifecycle {
                ts: OffsetDateTime::now_utc(),
                pid: proc.pid,
                name: proc.name.clone(),
                kind: ProcessLifecycleKind::Started,
            };
            recorder.append(&Event::ProcessLifecycle(event))?;
        }

        for proc in &proc_diff.exited {
            let event = ProcessLifecycle {
                ts: OffsetDateTime::now_utc(),
                pid: proc.pid,
                name: proc.name.clone(),
                kind: ProcessLifecycleKind::Exited,
            };
            recorder.append(&Event::ProcessLifecycle(event))?;
        }

        for proc in &proc_diff.stuck {
            let event = ProcessLifecycle {
                ts: OffsetDateTime::now_utc(),
                pid: proc.pid,
                name: proc.name.clone(),
                kind: ProcessLifecycleKind::Stuck,
            };
            recorder.append(&Event::ProcessLifecycle(event))?;

            // Record anomaly for stuck process
            let anomaly = Anomaly {
                ts: OffsetDateTime::now_utc(),
                severity: AnomalySeverity::Warning,
                kind: AnomalyKind::ProcessStuck,
                message: format!("Process stuck in D state: {} (pid {})", proc.name, proc.pid),
            };
            recorder.append(&Event::Anomaly(anomaly))?;
        }

        for proc in &proc_diff.zombie {
            let event = ProcessLifecycle {
                ts: OffsetDateTime::now_utc(),
                pid: proc.pid,
                name: proc.name.clone(),
                kind: ProcessLifecycleKind::Zombie,
            };
            recorder.append(&Event::ProcessLifecycle(event))?;
        }

        // Anomaly detection
        if cpu_usage > cpu_spike_threshold {
            let anomaly = Anomaly {
                ts: OffsetDateTime::now_utc(),
                severity: AnomalySeverity::Warning,
                kind: AnomalyKind::CpuSpike,
                message: format!("CPU spike: {:.1}%", cpu_usage),
            };
            recorder.append(&Event::Anomaly(anomaly))?;
        }

        let mem_usage_percent = mem_stats.usage_percent();
        if mem_usage_percent > mem_spike_threshold {
            let anomaly = Anomaly {
                ts: OffsetDateTime::now_utc(),
                severity: AnomalySeverity::Critical,
                kind: AnomalyKind::MemorySpike,
                message: format!("Memory spike: {:.1}%", mem_usage_percent),
            };
            recorder.append(&Event::Anomaly(anomaly))?;
        }

        if swap_stats.total_kb > 0 {
            let swap_usage_percent = (swap_stats.used_kb() as f32 / swap_stats.total_kb as f32) * 100.0;
            if swap_usage_percent > swap_usage_threshold {
                let anomaly = Anomaly {
                    ts: OffsetDateTime::now_utc(),
                    severity: AnomalySeverity::Warning,
                    kind: AnomalyKind::SwapUsage,
                    message: format!("Swap usage: {:.1}%", swap_usage_percent),
                };
                recorder.append(&Event::Anomaly(anomaly))?;
            }
        }

        let disk_usage_percent = (disk_space.used_bytes as f32 / disk_space.total_bytes as f32) * 100.0;
        if disk_usage_percent > disk_full_threshold {
            let anomaly = Anomaly {
                ts: OffsetDateTime::now_utc(),
                severity: AnomalySeverity::Critical,
                kind: AnomalyKind::DiskFull,
                message: format!("Disk usage: {:.1}%", disk_usage_percent),
            };
            recorder.append(&Event::Anomaly(anomaly))?;
        }

        if disk_write_per_sec > disk_spike_threshold {
            let anomaly = Anomaly {
                ts: OffsetDateTime::now_utc(),
                severity: AnomalySeverity::Warning,
                kind: AnomalyKind::DiskSpike,
                message: format!("Disk write spike: {}/s", format_bytes(disk_write_per_sec)),
            };
            recorder.append(&Event::Anomaly(anomaly))?;
        }

        if net_send_per_sec > network_spike_threshold || net_recv_per_sec > network_spike_threshold {
            let anomaly = Anomaly {
                ts: OffsetDateTime::now_utc(),
                severity: AnomalySeverity::Warning,
                kind: AnomalyKind::NetworkSpike,
                message: format!(
                    "Network spike: RX={}/s TX={}/s",
                    format_bytes(net_recv_per_sec),
                    format_bytes(net_send_per_sec)
                ),
            };
            recorder.append(&Event::Anomaly(anomaly))?;
        }

        if ctxt_per_sec > ctxt_spike_threshold {
            let anomaly = Anomaly {
                ts: OffsetDateTime::now_utc(),
                severity: AnomalySeverity::Warning,
                kind: AnomalyKind::ContextSwitchSpike,
                message: format!("Context switch spike: {}/s", ctxt_per_sec),
            };
            recorder.append(&Event::Anomaly(anomaly))?;
        }

        prev_cpu_snapshot = cpu_snapshot;
        prev_disk_snapshot = disk_snapshot;
        prev_network = network_stats;
        prev_ctxt = ctxt_stats;
        prev_processes = current_processes;

        // Security monitoring (every N seconds to reduce overhead)
        static SECURITY_COUNTER: AtomicU64 = AtomicU64::new(0);
        let security_count = SECURITY_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;

        if security_count % SECURITY_CHECK_INTERVAL == 0 {
            // Check logged-in users
            if let Ok(current_users) = read_logged_in_users() {
                let mut current_user_map = std::collections::HashMap::new();
                for user in &current_users {
                    let key = format!("{}@{}", user.username, user.terminal);
                    let value = user
                        .remote_host
                        .clone()
                        .unwrap_or_else(|| "local".to_string());
                    current_user_map.insert(key.clone(), value.clone());

                    // Check for new logins
                    if !prev_logged_in_users.contains_key(&key) {
                        let event = SecurityEvent {
                            ts: OffsetDateTime::now_utc(),
                            kind: SecurityEventKind::UserLogin,
                            user: user.username.clone(),
                            source_ip: user.remote_host.clone(),
                            message: format!(
                                "User {} logged in on {} from {}",
                                user.username,
                                user.terminal,
                                user.remote_host.as_deref().unwrap_or("local")
                            ),
                        };
                        recorder.append(&Event::SecurityEvent(event))?;
                        println!(
                            "  [SEC] User login: {} on {} from {}",
                            user.username,
                            user.terminal,
                            user.remote_host.as_deref().unwrap_or("local")
                        );
                    }
                }

                // Check for logouts
                for (key, host) in &prev_logged_in_users {
                    if !current_user_map.contains_key(key) {
                        let username = key.split('@').next().unwrap_or("unknown");
                        let event = SecurityEvent {
                            ts: OffsetDateTime::now_utc(),
                            kind: SecurityEventKind::UserLogout,
                            user: username.to_string(),
                            source_ip: Some(host.clone()),
                            message: format!("User {} logged out from {}", username, host),
                        };
                        recorder.append(&Event::SecurityEvent(event))?;
                    }
                }

                prev_logged_in_users = current_user_map;
            }

            // Check auth log for SSH/sudo events
            if let Ok(auth_entries) = tail_auth_log(&mut auth_log_position) {
                for entry in auth_entries {
                    let (kind, severity) = match entry.event_type {
                        AuthEventType::SshSuccess => {
                            (SecurityEventKind::SshLoginSuccess, AnomalySeverity::Info)
                        }
                        AuthEventType::SshFailure | AuthEventType::InvalidUser => {
                            // Track failed attempts for brute force detection
                            if let Some(ip) = &entry.source_ip {
                                failed_logins
                                    .entry(ip.clone())
                                    .or_insert_with(Vec::new)
                                    .push(std::time::Instant::now());

                                // Clean old entries (>5 minutes)
                                if let Some(attempts) = failed_logins.get_mut(ip) {
                                    attempts.retain(|t| t.elapsed().as_secs() < 300);

                                    // Alert if 5+ failures in 5 minutes
                                    if attempts.len() >= 5 {
                                        let anomaly = Anomaly {
                                            ts: OffsetDateTime::now_utc(),
                                            severity: AnomalySeverity::Warning,
                                            kind: AnomalyKind::BruteForceAttempt,
                                            message: format!(
                                                "Brute force attempt from {}: {} failures",
                                                ip,
                                                attempts.len()
                                            ),
                                        };
                                        recorder.append(&Event::Anomaly(anomaly))?;
                                        println!(
                                            "  [!] Brute force detected from {}: {} attempts",
                                            ip,
                                            attempts.len()
                                        );
                                    }
                                }
                            }

                            (
                                SecurityEventKind::SshLoginFailure,
                                AnomalySeverity::Warning,
                            )
                        }
                        AuthEventType::SudoCommand => {
                            (SecurityEventKind::SudoCommand, AnomalySeverity::Info)
                        }
                        _ => (SecurityEventKind::FailedAuth, AnomalySeverity::Warning),
                    };

                    let event = SecurityEvent {
                        ts: OffsetDateTime::now_utc(),
                        kind,
                        user: entry.user.clone(),
                        source_ip: entry.source_ip.clone(),
                        message: entry.message.clone(),
                    };
                    recorder.append(&Event::SecurityEvent(event))?;

                    // Print interesting security events
                    match entry.event_type {
                        AuthEventType::SshSuccess => {
                            println!(
                                "  [SEC] SSH login: {} from {}",
                                entry.user,
                                entry.source_ip.as_deref().unwrap_or("unknown")
                            );
                        }
                        AuthEventType::SshFailure | AuthEventType::InvalidUser => {
                            if severity == AnomalySeverity::Warning {
                                println!(
                                    "  [SEC] SSH failure: {} from {}",
                                    entry.user,
                                    entry.source_ip.as_deref().unwrap_or("unknown")
                                );
                            }
                        }
                        AuthEventType::SudoCommand => {
                            println!("  [SEC] Sudo: {}", entry.user);
                        }
                        _ => {}
                    }
                }
            }

            // Check for port scans
            if let Ok(scan_alerts) = connection_tracker.update() {
                for alert in scan_alerts {
                    let anomaly = Anomaly {
                        ts: OffsetDateTime::now_utc(),
                        severity: AnomalySeverity::Warning,
                        kind: AnomalyKind::PortScanActivity,
                        message: alert.clone(),
                    };
                    recorder.append(&Event::Anomaly(anomaly))?;
                    println!("  [!] Port scan: {}", alert);
                }
            }
        }

        // Periodically snapshot top processes
        static SNAPSHOT_COUNTER: AtomicU64 = AtomicU64::new(0);
        let snapshot_count = SNAPSHOT_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;

        if snapshot_count % PROCESS_SNAPSHOT_INTERVAL == 0 {
            if let Ok(top_procs) = get_top_processes(TOP_PROCESSES_COUNT) {
                let proc_infos: Vec<ProcessInfo> = top_procs
                    .iter()
                    .map(|p| ProcessInfo {
                        pid: p.pid,
                        name: p.name.clone(),
                        cmdline: p.cmdline.clone(),
                        state: p.state.clone(),
                        cpu_percent: 0.0, // TODO: Calculate per-process CPU %
                        mem_bytes: p.mem_bytes,
                        read_bytes: p.read_bytes,
                        write_bytes: p.write_bytes,
                        num_fds: p.num_fds,
                        num_threads: p.num_threads,
                    })
                    .collect();

                let snapshot = EventProcessSnapshot {
                    ts: OffsetDateTime::now_utc(),
                    processes: proc_infos,
                };
                recorder.append(&Event::ProcessSnapshot(snapshot))?;
            }
        }

        // Print status updates
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let count = COUNTER.fetch_add(1, Ordering::Relaxed) + 1;

        if count % 10 == 0 {
            let disk_usage_percent = (disk_space.used_bytes as f32 / disk_space.total_bytes as f32) * 100.0;
            println!(
                "[{}] CPU:{:.1}%  Mem:{:.1}%  Disk:{:.0}%  Load:{:.2}  Net:R={}/s,T={}/s  TCP:{}  Ctxt:{}/s",
                count,
                cpu_usage,
                mem_usage_percent,
                disk_usage_percent,
                load_avg.load_1m,
                format_bytes(net_recv_per_sec),
                format_bytes(net_send_per_sec),
                tcp_stats.total_connections,
                ctxt_per_sec
            );
        }

        // Report interesting events
        if !proc_diff.started.is_empty() {
            for proc in &proc_diff.started {
                println!("  [+] Process started: {} (pid {})", proc.name, proc.pid);
            }
        }

        if !proc_diff.exited.is_empty() {
            for proc in &proc_diff.exited {
                println!("  [-] Process exited: {} (pid {})", proc.name, proc.pid);
            }
        }

        if !proc_diff.stuck.is_empty() {
            for proc in &proc_diff.stuck {
                println!("  [!] Process STUCK (D state): {} (pid {})", proc.name, proc.pid);
            }
        }

        if !proc_diff.zombie.is_empty() {
            for proc in &proc_diff.zombie {
                println!("  [Z] Zombie process: {} (pid {})", proc.name, proc.pid);
            }
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}MB", bytes as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:.1}GB", bytes as f64 / 1024.0 / 1024.0 / 1024.0)
    }
}

