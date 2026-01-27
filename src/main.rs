#![recursion_limit = "256"]

mod broadcast;
mod cli;
mod collector;
mod commands;
mod config;
mod event;
mod file_watcher;
mod index;
mod indexed_reader;
mod protection;
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
use cli::Cli;
use config::{Config, ProtectionMode, RemoteSyslogConfig};
use protection::ProtectionManager;

use collector::{
    check_group_changes, check_kernel_module_changes, check_listening_port_changes,
    check_passwd_changes, check_sudoers_changes, diff_processes, get_default_gateway,
    get_dns_server, get_primary_ip_address, get_top_processes, read_all_cpu_stats,
    read_all_filesystems, read_context_switches, read_disk_space, read_disk_stats_per_device,
    read_disk_temperatures, read_fan_speeds, read_load_avg, read_logged_in_users,
    read_memory_stats, read_network_stats, read_per_core_temperatures, read_processes,
    read_swap_stats, read_tcp_stats, read_temperatures, tail_auth_log, AuthEventType,
    ConnectionTracker,
};
use event::{
    Anomaly, AnomalyKind, AnomalySeverity, Event, FilesystemInfo, LoggedInUserInfo,
    PerDiskMetrics, ProcessInfo, ProcessLifecycle, ProcessLifecycleKind,
    ProcessSnapshot as EventProcessSnapshot, SecurityEvent, SecurityEventKind, SystemMetrics,
    TemperatureReadings,
};
use recorder::Recorder;

const COLLECTION_INTERVAL_SECS: u64 = 1;
const TOP_PROCESSES_COUNT: usize = 10;
const PROCESS_SNAPSHOT_INTERVAL: u64 = 5; // Snapshot top processes every 5 seconds
const SECURITY_CHECK_INTERVAL: u64 = 5; // Check security events every 5 seconds
const TEMPERATURE_CHECK_INTERVAL: u64 = 10; // Check temperatures every 10 seconds
const FILESYSTEM_CHECK_INTERVAL: u64 = 30; // Check filesystems every 30 seconds
const NETWORK_CONFIG_CHECK_INTERVAL: u64 = 30; // Check network config every 30 seconds

/// Format current time as HH:MM:SS.mmm
fn now_timestamp() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        now.hour(),
        now.minute(),
        now.second(),
        now.millisecond()
    )
}

fn main() -> Result<()> {
    use cli::{Cli, Commands, ConfigCommands, SystemdCommands};

    let cli = Cli::parse_args();

    // Handle subcommands
    match cli.command {
        Some(Commands::Export {
            output,
            format,
            compress,
            event_type,
            start,
            end,
            data_dir,
        }) => {
            return commands::export::run_export(
                output, format, compress, event_type, start, end, data_dir,
            );
        }
        Some(Commands::Monitor {
            url,
            username,
            password,
            interval,
            export_dir,
            continuous,
        }) => {
            return commands::monitor::run_monitor(
                url, username, password, interval, export_dir, continuous,
            );
        }
        Some(Commands::Status {
            url,
            username,
            password,
            format,
        }) => {
            return commands::status::run_status(url, username, password, format);
        }
        Some(Commands::Systemd { command }) => match command {
            SystemdCommands::Generate {
                binary_path,
                working_dir,
                data_dir,
                export_on_stop,
                export_dir,
                output,
            } => {
                return commands::systemd::generate_service(
                    binary_path,
                    working_dir,
                    data_dir,
                    export_on_stop,
                    export_dir,
                    output,
                );
            }
            SystemdCommands::Install {
                binary_path,
                working_dir,
                export_on_stop,
            } => {
                return commands::systemd::install_service(
                    binary_path,
                    working_dir,
                    export_on_stop,
                );
            }
        },
        Some(Commands::Config { command }) => match command {
            ConfigCommands::Show => {
                return commands::config::show_config();
            }
            ConfigCommands::Validate => {
                return commands::config::validate_config();
            }
            ConfigCommands::Init { force } => {
                return commands::config::init_config(force);
            }
            ConfigCommands::SetupRemote { host, port, protocol } => {
                return commands::config::setup_remote_syslog(host, port, protocol);
            }
        },
        Some(Commands::Run { force_stop: _ }) | None => {
            // Fall through to run the recorder (default behavior)
        }
    }

    // Run the black box recorder (default behavior)
    run_recorder(cli)
}

fn run_recorder(cli: Cli) -> Result<()> {
    // Parse protection mode from CLI flags
    let protection_mode = if cli.hardened {
        ProtectionMode::Hardened
    } else if cli.protected {
        ProtectionMode::Protected
    } else {
        ProtectionMode::Default
    };

    // Check for headless mode
    let disable_ui = cli.headless;

    // Load configuration
    let config = Config::load()?;

    // Create protection manager
    let mut protection_manager = ProtectionManager::new(protection_mode, config.protection.clone());
    protection_manager.print_info();

    // Parse port (command line overrides config)
    let port = cli.port.unwrap_or(config.server.port);

    let data_dir = config.server.data_dir.clone();

    // Create broadcast channel for event streaming
    let (broadcast_tx, broadcaster) = EventBroadcaster::new();

    // Start async services (web server and remote streaming)
    if !disable_ui || config.protection.remote_syslog.as_ref().map(|c| c.enabled).unwrap_or(false) {
        let data_dir_clone = data_dir.clone();
        let config_clone = config.clone();
        let broadcaster = Arc::new(broadcaster);
        let protection_config = config.protection.clone();

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

            // Start async services in background
            rt.block_on(async {
                // Start remote streaming if configured
                if let Some(ref syslog_config) = protection_config.remote_syslog {
                    if syslog_config.enabled && protection_mode != ProtectionMode::Default {
                        let broadcaster_clone = broadcaster.clone();
                        let syslog_config = syslog_config.clone();
                        tokio::spawn(async move {
                            start_remote_streaming(broadcaster_clone, syslog_config).await;
                        });
                    }
                }

                // Start web server if not disabled
                if !disable_ui {
                    if let Err(e) =
                        webui::start_server(data_dir_clone, port, broadcaster, config_clone).await
                    {
                        eprintln!("Web UI failed to start: {}", e);
                    }
                } else {
                    // Keep runtime alive for remote streaming
                    tokio::signal::ctrl_c().await.ok();
                }
            });
        });
    }

    // Clone broadcast_tx for file watcher before moving into recorder
    let file_watcher_tx = broadcast_tx.clone();

    // Run recorder in main thread with broadcasting
    let mut recorder = Recorder::open_with_broadcast(&data_dir, broadcast_tx)?;

    // Start file watcher if configured
    if config.file_watch.enabled && !config.file_watch.watch_dirs.is_empty() {
        let watch_dirs = config.file_watch.watch_dirs.clone();
        file_watcher::spawn_file_watcher(watch_dirs, file_watcher_tx)?;
    }

    // Protect existing segment files
    if let Ok(entries) = std::fs::read_dir(&data_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("seg") {
                let _ = protection_manager.protect_file(&entry.path());
            }
        }
    }

    println!("┌─────────────┐");
    println!("│  Black Box  │");
    println!("└─────────────┘");
    println!();
    println!("Mode: {}", match protection_mode {
        ProtectionMode::Default => "DEFAULT",
        ProtectionMode::Protected => "PROTECTED",
        ProtectionMode::Hardened => "HARDENED",
    });
    println!("Data directory: {}", data_dir);
    println!("Max storage: ~100MB (ring buffer)");
    println!("Collection interval: {}s", COLLECTION_INTERVAL_SECS);
    println!("Tracking: CPU, Memory, Swap, Disk, Network, TCP, Load, Temperature, Processes");
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

    // Track process CPU times for per-process CPU percentage calculation
    let mut prev_process_cpu: std::collections::HashMap<u32, (u64, std::time::Instant)> =
        std::collections::HashMap::new();

    // Cached values for less frequent checks
    let mut cached_temps = read_temperatures();
    let mut cached_per_core_temps = Vec::new();
    let mut cached_disk_temps = std::collections::HashMap::new();
    let mut cached_fans = Vec::new();
    let mut cached_filesystems = read_all_filesystems().unwrap_or_default();
    let mut cached_net_ip = get_primary_ip_address();
    let mut cached_net_gateway = get_default_gateway();
    let mut cached_net_dns = get_dns_server();

    // Track static/semi-static field values for change detection
    let mut last_kernel_version = String::new();
    let mut last_cpu_model = String::new();
    let mut last_cpu_mhz = 0u32;
    let mut last_mem_total = 0u64;
    let mut last_swap_total = 0u64;
    let mut last_disk_total = 0u64;
    let mut last_net_interface = String::new();
    let mut last_logged_in_users: Vec<String> = Vec::new();

    // Cache for calculating percentages every second (even when totals aren't sent)
    let mut cached_mem_total_for_pct = 0u64;
    let mut cached_swap_total_for_pct = 0u64;
    let mut cached_disk_total_for_pct = 0u64;

    // Collection interval counters
    let mut tick_count = 0u64;
    const STATIC_FIELDS_INTERVAL: u64 = 60;       // 1 minute for static fields (ensures clients get them quickly)
    const SEMI_STATIC_FIELDS_INTERVAL: u64 = 60;  // 1 minute for semi-static fields

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
        tick_count += 1;

        // CPU stats
        let cpu_snapshot = read_all_cpu_stats()?;
        let per_core_usage = cpu_snapshot.per_core_usage(&prev_cpu_snapshot);
        let num_cpus = per_core_usage.len() as f32;
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

        // Update temperatures and fans periodically (less frequent)
        static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
        let temp_count = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
        if temp_count % TEMPERATURE_CHECK_INTERVAL == 0 {
            cached_temps = read_temperatures();
            cached_per_core_temps = read_per_core_temperatures(per_core_usage.len());
            cached_disk_temps = read_disk_temperatures();
            cached_fans = read_fan_speeds();
        }

        // Calculate throughput
        let (net_recv_per_sec, net_send_per_sec) =
            network_stats.bytes_per_sec(&prev_network, COLLECTION_INTERVAL_SECS as f32);
        let (net_recv_errors_per_sec, net_send_errors_per_sec) =
            network_stats.errors_per_sec(&prev_network, COLLECTION_INTERVAL_SECS as f32);
        let (net_recv_drops_per_sec, net_send_drops_per_sec) =
            network_stats.drops_per_sec(&prev_network, COLLECTION_INTERVAL_SECS as f32);
        let net_interface = network_stats.primary_interface.clone();

        // Update network config periodically (less frequent)
        static NET_CONFIG_COUNTER: AtomicU64 = AtomicU64::new(0);
        let net_config_count = NET_CONFIG_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
        if net_config_count % NETWORK_CONFIG_CHECK_INTERVAL == 0 {
            cached_net_ip = get_primary_ip_address();
            cached_net_gateway = get_default_gateway();
            cached_net_dns = get_dns_server();
        }

        let ctxt_per_sec = ctxt_stats.per_sec(&prev_ctxt, COLLECTION_INTERVAL_SECS as f32);

        // Update filesystems periodically (less frequent)
        static FS_COUNTER: AtomicU64 = AtomicU64::new(0);
        let fs_count = FS_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
        if fs_count % FILESYSTEM_CHECK_INTERVAL == 0 {
            cached_filesystems = read_all_filesystems().unwrap_or_default();
        }

        // Build per-disk metrics with temperatures
        let per_disk_metrics: Vec<PerDiskMetrics> = per_disk_throughput
            .into_iter()
            .map(|(dev_name, read_ps, write_ps)| {
                PerDiskMetrics {
                    device_name: dev_name.clone(),
                    read_bytes_per_sec: read_ps,
                    write_bytes_per_sec: write_ps,
                    temp_celsius: cached_disk_temps.get(&dev_name).and_then(|t| *t),
                }
            })
            .collect();

        // Determine which static/semi-static fields to include
        // Always include in first 30 seconds to ensure any client connecting gets them
        let include_static = tick_count <= 30 || tick_count % STATIC_FIELDS_INTERVAL == 0;
        let include_semi_static = tick_count <= 30 || tick_count % SEMI_STATIC_FIELDS_INTERVAL == 0;

        // Collect static fields (hourly or on change)
        let cpu_info = collector::read_cpu_info();
        let kernel_version = collector::read_kernel_version();
        let mem_total = mem_stats.total_kb * 1024;
        let swap_total = swap_stats.total_kb * 1024;
        let disk_total = disk_space.total_bytes;

        // Always update cached values for percentage calculations
        cached_mem_total_for_pct = mem_total;
        cached_swap_total_for_pct = swap_total;
        cached_disk_total_for_pct = disk_total;

        let kernel_changed = kernel_version != last_kernel_version;
        let cpu_model_changed = cpu_info.model != last_cpu_model;
        let cpu_mhz_changed = cpu_info.mhz != last_cpu_mhz;
        let mem_total_changed = mem_total != last_mem_total;
        let swap_total_changed = swap_total != last_swap_total;
        let disk_total_changed = disk_total != last_disk_total;

        let opt_kernel_version = if include_static || kernel_changed {
            last_kernel_version = kernel_version.clone();
            Some(kernel_version)
        } else {
            None
        };

        let opt_cpu_model = if include_static || cpu_model_changed {
            last_cpu_model = cpu_info.model.clone();
            Some(cpu_info.model.clone())
        } else {
            None
        };

        let opt_cpu_mhz = if include_static || cpu_mhz_changed {
            last_cpu_mhz = cpu_info.mhz;
            Some(cpu_info.mhz)
        } else {
            None
        };

        let opt_mem_total = if include_static || mem_total_changed {
            last_mem_total = mem_total;
            Some(mem_total)
        } else {
            None
        };

        let opt_swap_total = if include_static || swap_total_changed {
            last_swap_total = swap_total;
            Some(swap_total)
        } else {
            None
        };

        let opt_disk_total = if include_static || disk_total_changed {
            last_disk_total = disk_total;
            Some(disk_total)
        } else {
            None
        };

        // Collect semi-static fields (every 5 minutes or on change)
        let net_interface_changed = net_interface != last_net_interface;

        let opt_filesystems = if include_semi_static {
            Some(cached_filesystems
                .iter()
                .map(|fs| FilesystemInfo {
                    filesystem: fs.filesystem.clone(),
                    mount_point: fs.mount_point.clone(),
                    total_bytes: fs.total_bytes,
                    used_bytes: fs.used_bytes,
                    available_bytes: fs.available_bytes,
                })
                .collect())
        } else {
            None
        };

        let opt_net_interface = if include_semi_static || net_interface_changed {
            last_net_interface = net_interface.clone();
            Some(net_interface.clone())
        } else {
            None
        };

        let opt_fans = if include_semi_static {
            Some(cached_fans.clone())
        } else {
            None
        };

        // Logged in users - only include on change
        let current_user_list: Vec<String> = read_logged_in_users()
            .unwrap_or_default()
            .iter()
            .map(|u| format!("{}@{}", u.username, u.terminal))
            .collect();
        let users_changed = current_user_list != last_logged_in_users;

        let opt_logged_in_users = if users_changed || include_semi_static {
            last_logged_in_users = current_user_list;
            Some(read_logged_in_users()
                .unwrap_or_default()
                .into_iter()
                .map(|u| LoggedInUserInfo {
                    username: u.username,
                    terminal: u.terminal,
                    remote_host: u.remote_host,
                })
                .collect())
        } else {
            None
        };

        // Record system metrics
        let system_metrics = SystemMetrics {
            ts: OffsetDateTime::now_utc(),

            // Static fields (Optional - only included hourly or on change)
            kernel_version: opt_kernel_version,
            cpu_model: opt_cpu_model,
            cpu_mhz: opt_cpu_mhz,
            mem_total_bytes: opt_mem_total,
            swap_total_bytes: opt_swap_total,
            disk_total_bytes: opt_disk_total,

            // Semi-static fields (Optional - every 5 min or on change)
            filesystems: opt_filesystems,
            net_interface: opt_net_interface,
            net_ip_address: if include_semi_static { cached_net_ip.clone() } else { None },
            net_gateway: if include_semi_static { cached_net_gateway.clone() } else { None },
            net_dns: if include_semi_static { cached_net_dns.clone() } else { None },
            fans: opt_fans,
            logged_in_users: opt_logged_in_users,

            // Dynamic fields (always included)
            system_uptime_seconds: collector::read_system_uptime().unwrap_or(0),
            cpu_usage_percent: cpu_usage,
            per_core_usage,
            mem_used_bytes: mem_stats.used_kb() * 1024,
            mem_usage_percent: if cached_mem_total_for_pct > 0 {
                ((mem_stats.used_kb() * 1024) as f64 / cached_mem_total_for_pct as f64 * 100.0) as f32
            } else {
                0.0
            },
            swap_used_bytes: swap_stats.used_kb() * 1024,
            swap_usage_percent: if cached_swap_total_for_pct > 0 {
                ((swap_stats.used_kb() * 1024) as f64 / cached_swap_total_for_pct as f64 * 100.0) as f32
            } else {
                0.0
            },
            load_avg_1m: load_avg.load_1m,
            load_avg_5m: load_avg.load_5m,
            load_avg_15m: load_avg.load_15m,
            disk_read_bytes_per_sec: disk_read_per_sec,
            disk_write_bytes_per_sec: disk_write_per_sec,
            disk_used_bytes: disk_space.used_bytes,
            disk_usage_percent: if cached_disk_total_for_pct > 0 {
                (disk_space.used_bytes as f64 / cached_disk_total_for_pct as f64 * 100.0) as f32
            } else {
                0.0
            },
            per_disk_metrics,
            net_recv_bytes_per_sec: net_recv_per_sec,
            net_send_bytes_per_sec: net_send_per_sec,
            net_recv_errors_per_sec,
            net_send_errors_per_sec,
            net_recv_drops_per_sec,
            net_send_drops_per_sec,
            tcp_connections: tcp_stats.total_connections,
            tcp_time_wait: tcp_stats.time_wait,
            context_switches_per_sec: ctxt_per_sec,
            temps: TemperatureReadings {
                cpu_temp_celsius: cached_temps.cpu_temp_celsius,
                per_core_temps: cached_per_core_temps.clone(),
                gpu_temp_celsius: cached_temps.gpu_temp_celsius,
                motherboard_temp_celsius: cached_temps.motherboard_temp_celsius,
            },
            gpu: collector::read_gpu_info(),
        };

        recorder.append(&Event::SystemMetrics(system_metrics))?;

        // Track process lifecycle changes
        let proc_diff = diff_processes(&prev_processes, &current_processes);

        for proc in &proc_diff.started {
            let event = ProcessLifecycle {
                ts: OffsetDateTime::now_utc(),
                pid: proc.pid,
                name: proc.name.clone(),
                cmdline: proc.cmdline.clone(),
                kind: ProcessLifecycleKind::Started,
            };
            recorder.append(&Event::ProcessLifecycle(event))?;
        }

        for proc in &proc_diff.exited {
            let event = ProcessLifecycle {
                ts: OffsetDateTime::now_utc(),
                pid: proc.pid,
                name: proc.name.clone(),
                cmdline: proc.cmdline.clone(),
                kind: ProcessLifecycleKind::Exited,
            };
            recorder.append(&Event::ProcessLifecycle(event))?;
        }

        for proc in &proc_diff.stuck {
            let event = ProcessLifecycle {
                ts: OffsetDateTime::now_utc(),
                pid: proc.pid,
                name: proc.name.clone(),
                cmdline: proc.cmdline.clone(),
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
                cmdline: proc.cmdline.clone(),
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

        // Network errors/drops detection
        if net_recv_errors_per_sec > 0 || net_send_errors_per_sec > 0 {
            let anomaly = Anomaly {
                ts: OffsetDateTime::now_utc(),
                severity: AnomalySeverity::Warning,
                kind: AnomalyKind::NetworkSpike,
                message: format!(
                    "Network errors detected: RX={}/s TX={}/s",
                    net_recv_errors_per_sec, net_send_errors_per_sec
                ),
            };
            recorder.append(&Event::Anomaly(anomaly))?;
        }

        if net_recv_drops_per_sec > 0 || net_send_drops_per_sec > 0 {
            let anomaly = Anomaly {
                ts: OffsetDateTime::now_utc(),
                severity: AnomalySeverity::Warning,
                kind: AnomalyKind::NetworkSpike,
                message: format!(
                    "Network packet drops detected: RX={}/s TX={}/s",
                    net_recv_drops_per_sec, net_send_drops_per_sec
                ),
            };
            recorder.append(&Event::Anomaly(anomaly))?;
        }

        // Calculate process counts before current_processes is moved
        let total_process_count = current_processes.len() as u32;
        let running_process_count = current_processes.values().filter(|p| p.state == "R").count() as u32;

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
                            "[{}] [SEC] User login: {} on {} from {}",
                            now_timestamp(),
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
                                            "[{}] [!] Brute force detected from {}: {} attempts",
                                            now_timestamp(),
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
                                "[{}] [SEC] SSH login: {} from {}",
                                now_timestamp(),
                                entry.user,
                                entry.source_ip.as_deref().unwrap_or("unknown")
                            );
                        }
                        AuthEventType::SshFailure | AuthEventType::InvalidUser => {
                            if severity == AnomalySeverity::Warning {
                                println!(
                                    "[{}] [SEC] SSH failure: {} from {}",
                                    now_timestamp(),
                                    entry.user,
                                    entry.source_ip.as_deref().unwrap_or("unknown")
                                );
                            }
                        }
                        AuthEventType::SudoCommand => {
                            println!("[{}] [SEC] [SUDO] {}", now_timestamp(), entry.user);
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
                    println!("[{}] [!] Port scan: {}", now_timestamp(), alert);
                }
            }

            // Check for user account changes
            if let Ok(Some(msg)) = check_passwd_changes() {
                let event = SecurityEvent {
                    ts: OffsetDateTime::now_utc(),
                    kind: SecurityEventKind::UserAccountModified,
                    user: "root".to_string(),
                    source_ip: None,
                    message: msg.clone(),
                };
                recorder.append(&Event::SecurityEvent(event))?;
                println!("[{}] [SEC] {}", now_timestamp(), msg);
            }

            // Check for group changes
            if let Ok(Some(msg)) = check_group_changes() {
                let event = SecurityEvent {
                    ts: OffsetDateTime::now_utc(),
                    kind: SecurityEventKind::GroupModified,
                    user: "root".to_string(),
                    source_ip: None,
                    message: msg.clone(),
                };
                recorder.append(&Event::SecurityEvent(event))?;
                println!("[{}] [SEC] {}", now_timestamp(), msg);
            }

            // Check for sudoers changes
            if let Ok(Some(msg)) = check_sudoers_changes() {
                let event = SecurityEvent {
                    ts: OffsetDateTime::now_utc(),
                    kind: SecurityEventKind::SudoersModified,
                    user: "root".to_string(),
                    source_ip: None,
                    message: msg.clone(),
                };
                recorder.append(&Event::SecurityEvent(event))?;
                println!("[{}] [SEC] {}", now_timestamp(), msg);
            }

            // Check for new/closed listening ports
            if let Ok((new_ports, closed_ports)) = check_listening_port_changes() {
                for (proto_addr, port) in new_ports {
                    let event = SecurityEvent {
                        ts: OffsetDateTime::now_utc(),
                        kind: SecurityEventKind::NewListeningPort,
                        user: "system".to_string(),
                        source_ip: None,
                        message: format!("New listening port: {} port {}", proto_addr, port),
                    };
                    recorder.append(&Event::SecurityEvent(event))?;
                    println!("[{}] [SEC] New listening port: {} port {}", now_timestamp(), proto_addr, port);
                }

                for (proto_addr, port) in closed_ports {
                    let event = SecurityEvent {
                        ts: OffsetDateTime::now_utc(),
                        kind: SecurityEventKind::ListeningPortClosed,
                        user: "system".to_string(),
                        source_ip: None,
                        message: format!("Listening port closed: {} port {}", proto_addr, port),
                    };
                    recorder.append(&Event::SecurityEvent(event))?;
                    println!("[{}] [SEC] Listening port closed: {} port {}", now_timestamp(), proto_addr, port);
                }
            }

            // Check for kernel module changes
            if let Ok((loaded, unloaded)) = check_kernel_module_changes() {
                for module in loaded {
                    let event = SecurityEvent {
                        ts: OffsetDateTime::now_utc(),
                        kind: SecurityEventKind::KernelModuleLoaded,
                        user: "kernel".to_string(),
                        source_ip: None,
                        message: format!("Kernel module loaded: {}", module),
                    };
                    recorder.append(&Event::SecurityEvent(event))?;
                    println!("[{}] [SEC] Kernel module loaded: {}", now_timestamp(), module);
                }

                for module in unloaded {
                    let event = SecurityEvent {
                        ts: OffsetDateTime::now_utc(),
                        kind: SecurityEventKind::KernelModuleUnloaded,
                        user: "kernel".to_string(),
                        source_ip: None,
                        message: format!("Kernel module unloaded: {}", module),
                    };
                    recorder.append(&Event::SecurityEvent(event))?;
                    println!("[{}] [SEC] Kernel module unloaded: {}", now_timestamp(), module);
                }
            }
        }

        // Periodically snapshot top processes
        static SNAPSHOT_COUNTER: AtomicU64 = AtomicU64::new(0);
        let snapshot_count = SNAPSHOT_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;

        if snapshot_count % PROCESS_SNAPSHOT_INTERVAL == 0 {
            if let Ok(top_procs) = get_top_processes(TOP_PROCESSES_COUNT) {
                let now = std::time::Instant::now();

                // Calculate CPU percentages and build process infos
                let mut proc_infos: Vec<ProcessInfo> = Vec::new();
                let mut new_process_cpu: std::collections::HashMap<u32, (u64, std::time::Instant)> =
                    std::collections::HashMap::new();

                for p in &top_procs {
                    // Calculate CPU percentage based on previous measurement
                    let cpu_percent = if let Some((prev_cpu, prev_time)) = prev_process_cpu.get(&p.pid) {
                        let elapsed_secs = now.duration_since(*prev_time).as_secs_f32();
                        if elapsed_secs > 0.0 {
                            let delta_cpu = p.cpu_time_jiffies.saturating_sub(*prev_cpu) as f32;
                            // USER_HZ is typically 100 on Linux (clock ticks per second)
                            let delta_cpu_secs = delta_cpu / 100.0;
                            // Divide by elapsed time and normalize by number of CPUs
                            ((delta_cpu_secs / elapsed_secs) * 100.0).min(100.0 * num_cpus)
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };

                    // Track for next iteration
                    new_process_cpu.insert(p.pid, (p.cpu_time_jiffies, now));

                    proc_infos.push(ProcessInfo {
                        pid: p.pid,
                        name: p.name.clone(),
                        cmdline: p.cmdline.clone(),
                        state: p.state.clone(),
                        user: p.user.clone(),
                        cpu_percent,
                        mem_bytes: p.mem_bytes,
                        read_bytes: p.read_bytes,
                        write_bytes: p.write_bytes,
                        num_fds: p.num_fds,
                        num_threads: p.num_threads,
                    });
                }

                // Update tracking map
                prev_process_cpu = new_process_cpu;

                let snapshot = EventProcessSnapshot {
                    ts: OffsetDateTime::now_utc(),
                    processes: proc_infos,
                    total_processes: total_process_count,
                    running_processes: running_process_count,
                };
                recorder.append(&Event::ProcessSnapshot(snapshot))?;
            }
        }

        // Print status updates
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let count = COUNTER.fetch_add(1, Ordering::Relaxed) + 1;

        if count % 10 == 0 {
            let disk_usage_percent = (disk_space.used_bytes as f32 / disk_space.total_bytes as f32) * 100.0;

            // Format temperature string if available
            let temp_str = if let Some(cpu_temp) = cached_temps.cpu_temp_celsius {
                format!("  Temp:{:.0}°C", cpu_temp)
            } else {
                String::new()
            };

            println!(
                "[{}] CPU:{:.1}%  Mem:{:.1}%  Disk:{:.0}%  Load:{:.2}  Net:R={}/s,T={}/s  TCP:{}  Ctxt:{}/s{}",
                now_timestamp(),
                cpu_usage,
                mem_usage_percent,
                disk_usage_percent,
                load_avg.load_1m,
                format_bytes(net_recv_per_sec),
                format_bytes(net_send_per_sec),
                tcp_stats.total_connections,
                ctxt_per_sec,
                temp_str
            );
        }

        // Report interesting events
        if !proc_diff.started.is_empty() {
            for proc in &proc_diff.started {
                println!("[{}] [+] Process started: {} (pid {}) - {}", now_timestamp(), proc.name, proc.pid, proc.cmdline);
            }
        }

        if !proc_diff.exited.is_empty() {
            for proc in &proc_diff.exited {
                println!("[{}] [-] Process exited: {} (pid {}) - {}", now_timestamp(), proc.name, proc.pid, proc.cmdline);
            }
        }

        if !proc_diff.stuck.is_empty() {
            for proc in &proc_diff.stuck {
                println!("[{}] [!] Process STUCK (D state): {} (pid {})", now_timestamp(), proc.name, proc.pid);
            }
        }

        if !proc_diff.zombie.is_empty() {
            for proc in &proc_diff.zombie {
                println!("[{}] [Z] Zombie process: {} (pid {})", now_timestamp(), proc.name, proc.pid);
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

// Remote streaming task - sends events to remote syslog
async fn start_remote_streaming(broadcaster: Arc<EventBroadcaster>, config: RemoteSyslogConfig) {
    use tokio::net::TcpStream;
    use tokio::net::UdpSocket;
    use tokio::io::AsyncWriteExt;

    println!("✓ Remote log streaming enabled: {}:{} ({})", config.host, config.port, config.protocol);

    let mut rx = broadcaster.subscribe();
    let addr = format!("{}:{}", config.host, config.port);

    // Try to establish connection for TCP
    let mut tcp_stream: Option<TcpStream> = None;
    if config.protocol == "tcp" {
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                println!("✓ Connected to remote syslog via TCP");
                tcp_stream = Some(stream);
            }
            Err(e) => {
                eprintln!("⚠ Failed to connect to remote syslog: {}", e);
                eprintln!("  Events will be buffered and retried");
            }
        }
    }

    // For UDP, create socket once
    let udp_socket = if config.protocol == "udp" {
        match UdpSocket::bind("0.0.0.0:0").await {
            Ok(socket) => {
                println!("✓ Remote syslog via UDP ready");
                Some(socket)
            }
            Err(e) => {
                eprintln!("⚠ Failed to create UDP socket: {}", e);
                None
            }
        }
    } else {
        None
    };

    loop {
        match rx.recv().await {
            Ok(event) => {
                // Serialize event to JSON
                let json = match serde_json::to_string(&event) {
                    Ok(j) => j,
                    Err(_) => continue,
                };

                // Send based on protocol
                if config.protocol == "tcp" {
                    if let Some(ref mut stream) = tcp_stream {
                        let msg = format!("{}\n", json);
                        if stream.write_all(msg.as_bytes()).await.is_err() {
                            // Connection lost, try to reconnect
                            eprintln!("⚠ Lost connection to remote syslog, reconnecting...");
                            tcp_stream = TcpStream::connect(&addr).await.ok();
                        }
                    } else {
                        // Try to reconnect periodically
                        tcp_stream = TcpStream::connect(&addr).await.ok();
                        if tcp_stream.is_some() {
                            println!("✓ Reconnected to remote syslog");
                        }
                    }
                } else if let Some(ref socket) = udp_socket {
                    let _ = socket.send_to(json.as_bytes(), &addr).await;
                }
            }
            Err(_) => {
                // Channel closed
                break;
            }
        }
    }
}

