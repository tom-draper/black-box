mod collector;
mod event;
mod recorder;
mod storage;

use anyhow::Result;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};
use time::OffsetDateTime;

use collector::{
    diff_processes, get_top_processes, read_context_switches, read_cpu_stats, read_disk_space,
    read_disk_stats, read_load_avg, read_memory_stats, read_network_stats, read_processes,
    read_swap_stats, read_tcp_stats,
};
use event::{
    Anomaly, AnomalyKind, AnomalySeverity, Event, ProcessInfo, ProcessLifecycle,
    ProcessLifecycleKind, ProcessSnapshot as EventProcessSnapshot, SystemMetrics,
};
use recorder::Recorder;

const COLLECTION_INTERVAL_SECS: u64 = 1;
const TOP_PROCESSES_COUNT: usize = 10;
const PROCESS_SNAPSHOT_INTERVAL: u64 = 5; // Snapshot top processes every 5 seconds

fn main() -> Result<()> {
    let mut recorder = Recorder::open("./data")?;

    println!("===============================================================");
    println!("       Black Box - Server Forensics Recorder                  ");
    println!("===============================================================");
    println!();
    println!("Data directory: ./data");
    println!("Max storage: ~100MB (ring buffer)");
    println!("Collection interval: {}s", COLLECTION_INTERVAL_SECS);
    println!("Tracking: CPU, Memory, Swap, Disk, Network, TCP, Load, Processes");
    println!();
    println!("Press Ctrl+C to stop\n");

    // Initialize baseline metrics
    let mut prev_cpu = read_cpu_stats()?;
    let mut prev_disk = read_disk_stats()?;
    let mut prev_network = read_network_stats()?;
    let mut prev_ctxt = read_context_switches()?;
    let mut prev_processes = read_processes()?;

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

        let cpu_stats = read_cpu_stats()?;
        let mem_stats = read_memory_stats()?;
        let swap_stats = read_swap_stats()?;
        let disk_stats = read_disk_stats()?;
        let disk_space = read_disk_space()?;
        let load_avg = read_load_avg()?;
        let network_stats = read_network_stats()?;
        let ctxt_stats = read_context_switches()?;
        let tcp_stats = read_tcp_stats()?;
        let current_processes = read_processes()?;

        let cpu_usage = cpu_stats.usage_percent(&prev_cpu);
        let (disk_read_per_sec, disk_write_per_sec) =
            disk_stats.bytes_per_sec(&prev_disk, COLLECTION_INTERVAL_SECS as f32);
        let (net_recv_per_sec, net_send_per_sec) =
            network_stats.bytes_per_sec(&prev_network, COLLECTION_INTERVAL_SECS as f32);
        let ctxt_per_sec = ctxt_stats.per_sec(&prev_ctxt, COLLECTION_INTERVAL_SECS as f32);

        // Record system metrics
        let system_metrics = SystemMetrics {
            ts: OffsetDateTime::now_utc(),
            cpu_usage_percent: cpu_usage,
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
            net_recv_bytes_per_sec: net_recv_per_sec,
            net_send_bytes_per_sec: net_send_per_sec,
            tcp_connections: tcp_stats.total_connections,
            tcp_time_wait: tcp_stats.time_wait,
            context_switches_per_sec: ctxt_per_sec,
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

        prev_cpu = cpu_stats;
        prev_disk = disk_stats;
        prev_network = network_stats;
        prev_ctxt = ctxt_stats;
        prev_processes = current_processes;

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

