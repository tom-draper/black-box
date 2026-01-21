use anyhow::{Context, Result};
use std::{collections::HashMap, fs};

// ===== System Uptime =====

pub fn read_system_uptime() -> Result<u64> {
    let content = fs::read_to_string("/proc/uptime")?;
    let uptime_str = content.split_whitespace().next().context("Empty /proc/uptime")?;
    let uptime_secs = uptime_str.parse::<f64>().context("Parse uptime")?;
    Ok(uptime_secs as u64)
}

// ===== Kernel Version =====

pub fn read_kernel_version() -> String {
    let release = fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let arch = std::env::consts::ARCH;
    format!("{} on {}", release, arch)
}

// ===== CPU Info =====

pub struct CpuInfo {
    pub model: String,
    pub mhz: u32,
}

pub fn read_cpu_info() -> CpuInfo {
    let content = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let mut model = String::new();
    let mut mhz: u32 = 0;

    for line in content.lines() {
        if line.starts_with("model name") {
            if let Some(val) = line.split(':').nth(1) {
                model = val.trim().to_string();
            }
        } else if line.starts_with("cpu MHz") {
            if let Some(val) = line.split(':').nth(1) {
                mhz = val.trim().parse::<f64>().unwrap_or(0.0) as u32;
            }
        }
        if !model.is_empty() && mhz > 0 {
            break;
        }
    }

    CpuInfo { model, mhz }
}

// ===== GPU Info =====

use crate::event::GpuInfo;

pub fn read_gpu_info() -> GpuInfo {
    // Try nvidia-smi first
    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=clocks.gr,clocks.mem,temperature.gpu,power.draw", "--format=csv,noheader,nounits"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = stdout.trim().split(", ").collect();
            if parts.len() >= 4 {
                return GpuInfo {
                    gpu_freq_mhz: parts.get(0).and_then(|s| s.trim().parse().ok()),
                    mem_freq_mhz: parts.get(1).and_then(|s| s.trim().parse().ok()),
                    gpu_temp_celsius: parts.get(2).and_then(|s| s.trim().parse().ok()),
                    power_watts: parts.get(3).and_then(|s| s.trim().parse().ok()),
                };
            }
        }
    }
    GpuInfo::default()
}

// ===== CPU Stats =====

#[derive(Debug, Clone)]
pub struct CpuStats {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
}

impl CpuStats {
    pub fn total(&self) -> u64 {
        self.user + self.nice + self.system + self.idle + self.iowait + self.irq + self.softirq + self.steal
    }

    pub fn idle_total(&self) -> u64 {
        self.idle + self.iowait
    }

    pub fn usage_percent(&self, prev: &CpuStats) -> f32 {
        let total_delta = self.total().saturating_sub(prev.total());
        let idle_delta = self.idle_total().saturating_sub(prev.idle_total());

        if total_delta == 0 {
            return 0.0;
        }

        let busy_delta = total_delta.saturating_sub(idle_delta);
        (busy_delta as f32 / total_delta as f32) * 100.0
    }
}

// ===== Per-Core CPU Stats =====

#[derive(Debug, Clone)]
pub struct CpuStatsSnapshot {
    pub aggregate: CpuStats,
    pub per_core: HashMap<u32, CpuStats>,
}

fn parse_cpu_line(parts: &[&str]) -> Result<CpuStats> {
    if parts.len() < 9 {
        anyhow::bail!("Not enough fields in CPU line");
    }

    Ok(CpuStats {
        user: parts[1].parse()?,
        nice: parts[2].parse()?,
        system: parts[3].parse()?,
        idle: parts[4].parse()?,
        iowait: parts[5].parse()?,
        irq: parts[6].parse()?,
        softirq: parts[7].parse()?,
        steal: parts[8].parse()?,
    })
}

pub fn read_all_cpu_stats() -> Result<CpuStatsSnapshot> {
    let content = fs::read_to_string("/proc/stat")?;
    let mut per_core = HashMap::new();
    let mut aggregate = None;

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        if parts[0] == "cpu" {
            aggregate = Some(parse_cpu_line(&parts)?);
        } else if parts[0].starts_with("cpu") {
            if let Some(core_id_str) = parts[0].strip_prefix("cpu") {
                if let Ok(core_id) = core_id_str.parse::<u32>() {
                    per_core.insert(core_id, parse_cpu_line(&parts)?);
                }
            }
        }
    }

    Ok(CpuStatsSnapshot {
        aggregate: aggregate.context("No aggregate CPU line found")?,
        per_core,
    })
}

impl CpuStatsSnapshot {
    pub fn per_core_usage(&self, prev: &CpuStatsSnapshot) -> Vec<f32> {
        let mut cores: Vec<(u32, f32)> = self.per_core
            .iter()
            .filter_map(|(core_id, current_stats)| {
                prev.per_core.get(core_id).map(|prev_stats| {
                    let usage = current_stats.usage_percent(prev_stats);
                    (*core_id, usage)
                })
            })
            .collect();

        cores.sort_by_key(|(core_id, _)| *core_id);
        cores.into_iter().map(|(_, usage)| usage).collect()
    }
}

// ===== Memory Stats =====

#[derive(Debug)]
pub struct MemoryStats {
    pub total_kb: u64,
    pub free_kb: u64,
    pub available_kb: u64,
    pub buffers_kb: u64,
    pub cached_kb: u64,
}

impl MemoryStats {
    pub fn used_kb(&self) -> u64 {
        self.total_kb
            .saturating_sub(self.free_kb + self.buffers_kb + self.cached_kb)
    }

    pub fn usage_percent(&self) -> f32 {
        if self.total_kb == 0 {
            return 0.0;
        }
        (self.used_kb() as f32 / self.total_kb as f32) * 100.0
    }
}

pub fn read_memory_stats() -> Result<MemoryStats> {
    let content = fs::read_to_string("/proc/meminfo").context("Failed to read /proc/meminfo")?;

    let mut stats = MemoryStats {
        total_kb: 0,
        free_kb: 0,
        available_kb: 0,
        buffers_kb: 0,
        cached_kb: 0,
    };

    for line in content.lines() {
        if let Some(value) = line.strip_prefix("MemTotal:") {
            stats.total_kb = parse_meminfo_value(value)?;
        } else if let Some(value) = line.strip_prefix("MemFree:") {
            stats.free_kb = parse_meminfo_value(value)?;
        } else if let Some(value) = line.strip_prefix("MemAvailable:") {
            stats.available_kb = parse_meminfo_value(value)?;
        } else if let Some(value) = line.strip_prefix("Buffers:") {
            stats.buffers_kb = parse_meminfo_value(value)?;
        } else if let Some(value) = line.strip_prefix("Cached:") {
            stats.cached_kb = parse_meminfo_value(value)?;
        }
    }

    Ok(stats)
}

fn parse_meminfo_value(s: &str) -> Result<u64> {
    s.trim()
        .split_whitespace()
        .next()
        .context("Missing value")?
        .parse()
        .context("Parse integer")
}

// ===== Load Average =====

#[derive(Debug, Clone)]
pub struct LoadAvg {
    pub load_1m: f32,
    pub load_5m: f32,
    pub load_15m: f32,
}

pub fn read_load_avg() -> Result<LoadAvg> {
    let content = fs::read_to_string("/proc/loadavg").context("Failed to read /proc/loadavg")?;

    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() < 3 {
        anyhow::bail!("Invalid /proc/loadavg format");
    }

    Ok(LoadAvg {
        load_1m: parts[0].parse().context("Parse 1m load")?,
        load_5m: parts[1].parse().context("Parse 5m load")?,
        load_15m: parts[2].parse().context("Parse 15m load")?,
    })
}

// ===== Swap Stats =====

#[derive(Debug)]
pub struct SwapStats {
    pub total_kb: u64,
    pub free_kb: u64,
}

impl SwapStats {
    pub fn used_kb(&self) -> u64 {
        self.total_kb.saturating_sub(self.free_kb)
    }
}

pub fn read_swap_stats() -> Result<SwapStats> {
    let content = fs::read_to_string("/proc/meminfo").context("Failed to read /proc/meminfo")?;

    let mut stats = SwapStats {
        total_kb: 0,
        free_kb: 0,
    };

    for line in content.lines() {
        if let Some(value) = line.strip_prefix("SwapTotal:") {
            stats.total_kb = parse_meminfo_value(value)?;
        } else if let Some(value) = line.strip_prefix("SwapFree:") {
            stats.free_kb = parse_meminfo_value(value)?;
        }
    }

    Ok(stats)
}

// ===== Disk I/O Stats =====

#[derive(Debug, Clone)]
pub struct DiskStats {
    pub read_bytes: u64,
    pub write_bytes: u64,
}

// Helper: Check if device name represents a physical disk (not partition)
fn is_physical_disk(dev_name: &str) -> bool {
    // SATA/SAS physical disks: sda, sdb, sdc, etc.
    if dev_name.len() == 3 && dev_name.starts_with("sd") {
        if let Some(last_char) = dev_name.chars().nth(2) {
            return last_char.is_ascii_lowercase();
        }
    }

    // NVMe physical disks: nvme0n1, nvme1n1, etc.
    if dev_name.starts_with("nvme") && dev_name.contains("n") && !dev_name.contains("p") {
        return true;
    }

    // VirtIO disks: vda, vdb, vdc, etc.
    if dev_name.len() == 3 && dev_name.starts_with("vd") {
        if let Some(last_char) = dev_name.chars().nth(2) {
            return last_char.is_ascii_lowercase();
        }
    }

    false
}

// Per-disk stats structure (for internal use)
#[derive(Debug, Clone)]
pub struct DiskStatsDetailed {
    #[allow(dead_code)]
    pub device_name: String,
    pub read_bytes: u64,
    pub write_bytes: u64,
}

// Snapshot of all disks
#[derive(Debug, Clone)]
pub struct AllDisksStats {
    pub by_device: HashMap<String, DiskStatsDetailed>,
    pub total: DiskStats,
}

pub fn read_disk_stats_per_device() -> Result<AllDisksStats> {
    let content = fs::read_to_string("/proc/diskstats")?;
    let mut by_device = HashMap::new();
    let mut total_read_sectors = 0u64;
    let mut total_write_sectors = 0u64;

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 14 {
            continue;
        }

        let dev_name = parts[2];

        // Skip loop, ram, sr devices
        if dev_name.starts_with("loop")
            || dev_name.starts_with("ram")
            || dev_name.starts_with("sr") {
            continue;
        }

        // Only include physical disks (exclude partitions)
        if !is_physical_disk(dev_name) {
            continue;
        }

        let read_sectors: u64 = parts[5].parse().unwrap_or(0);
        let write_sectors: u64 = parts[9].parse().unwrap_or(0);

        total_read_sectors += read_sectors;
        total_write_sectors += write_sectors;

        by_device.insert(dev_name.to_string(), DiskStatsDetailed {
            device_name: dev_name.to_string(),
            read_bytes: read_sectors * 512,
            write_bytes: write_sectors * 512,
        });
    }

    Ok(AllDisksStats {
        by_device,
        total: DiskStats {
            read_bytes: total_read_sectors * 512,
            write_bytes: total_write_sectors * 512,
        },
    })
}

impl AllDisksStats {
    pub fn per_disk_throughput(
        &self,
        prev: &AllDisksStats,
        interval_secs: f32,
    ) -> Vec<(String, u64, u64)> {
        let mut results = Vec::new();

        for (dev_name, current) in &self.by_device {
            if let Some(previous) = prev.by_device.get(dev_name) {
                let read_delta = current.read_bytes.saturating_sub(previous.read_bytes);
                let write_delta = current.write_bytes.saturating_sub(previous.write_bytes);

                let read_per_sec = (read_delta as f32 / interval_secs) as u64;
                let write_per_sec = (write_delta as f32 / interval_secs) as u64;

                results.push((dev_name.clone(), read_per_sec, write_per_sec));
            }
        }

        results.sort_by(|a, b| a.0.cmp(&b.0));
        results
    }
}

impl DiskStats {
    pub fn bytes_per_sec(&self, prev: &DiskStats, interval_secs: f32) -> (u64, u64) {
        let read_delta = self.read_bytes.saturating_sub(prev.read_bytes);
        let write_delta = self.write_bytes.saturating_sub(prev.write_bytes);

        let read_per_sec = (read_delta as f32 / interval_secs) as u64;
        let write_per_sec = (write_delta as f32 / interval_secs) as u64;

        (read_per_sec, write_per_sec)
    }
}

// ===== Disk Space Stats =====

#[derive(Debug, Clone)]
pub struct DiskSpaceStats {
    pub total_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct FilesystemStats {
    pub filesystem: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
}

pub fn read_disk_space() -> Result<DiskSpaceStats> {
    // Simple approach: use df for root
    let output = std::process::Command::new("df")
        .arg("-B1") // 1-byte blocks
        .arg("/")
        .output()
        .context("Failed to run df")?;

    let content = String::from_utf8_lossy(&output.stdout);

    for line in content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let total = parts[1].parse().unwrap_or(0);
            let used = parts[2].parse().unwrap_or(0);
            return Ok(DiskSpaceStats {
                total_bytes: total,
                used_bytes: used,
            });
        }
    }

    anyhow::bail!("Failed to parse df output")
}

pub fn read_all_filesystems() -> Result<Vec<FilesystemStats>> {
    let output = std::process::Command::new("df")
        .arg("-B1") // 1-byte blocks
        .arg("-x").arg("tmpfs")
        .arg("-x").arg("devtmpfs")
        .arg("-x").arg("squashfs")
        .arg("-x").arg("overlay")
        .output()
        .context("Failed to run df")?;

    let content = String::from_utf8_lossy(&output.stdout);
    let mut filesystems = Vec::new();

    for line in content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 6 {
            let filesystem = parts[0].to_string();
            let total: u64 = parts[1].parse().unwrap_or(0);
            let used: u64 = parts[2].parse().unwrap_or(0);
            let available: u64 = parts[3].parse().unwrap_or(0);
            let mount_point = parts[5].to_string();

            // Skip if total is 0 or mount point is system-related
            if total == 0 {
                continue;
            }

            filesystems.push(FilesystemStats {
                filesystem,
                mount_point,
                total_bytes: total,
                used_bytes: used,
                available_bytes: available,
            });
        }
    }

    Ok(filesystems)
}

// ===== Network I/O Stats =====

#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub recv_bytes: u64,
    pub send_bytes: u64,
}

pub fn read_network_stats() -> Result<NetworkStats> {
    let content = fs::read_to_string("/proc/net/dev").context("Failed to read /proc/net/dev")?;

    let mut total_recv = 0u64;
    let mut total_send = 0u64;

    for line in content.lines().skip(2) {
        // Skip header lines
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            continue;
        }

        // Skip loopback
        if parts[0].starts_with("lo:") {
            continue;
        }

        // Recv bytes is field 1, send bytes is field 9
        if let (Ok(recv), Ok(send)) = (parts[1].parse::<u64>(), parts[9].parse::<u64>()) {
            total_recv += recv;
            total_send += send;
        }
    }

    Ok(NetworkStats {
        recv_bytes: total_recv,
        send_bytes: total_send,
    })
}

impl NetworkStats {
    pub fn bytes_per_sec(&self, prev: &NetworkStats, interval_secs: f32) -> (u64, u64) {
        let recv_delta = self.recv_bytes.saturating_sub(prev.recv_bytes);
        let send_delta = self.send_bytes.saturating_sub(prev.send_bytes);

        let recv_per_sec = (recv_delta as f32 / interval_secs) as u64;
        let send_per_sec = (send_delta as f32 / interval_secs) as u64;

        (recv_per_sec, send_per_sec)
    }
}

// ===== Context Switch Stats =====

#[derive(Debug, Clone)]
pub struct ContextSwitchStats {
    pub count: u64,
}

pub fn read_context_switches() -> Result<ContextSwitchStats> {
    let content = fs::read_to_string("/proc/stat").context("Failed to read /proc/stat")?;

    for line in content.lines() {
        if let Some(value) = line.strip_prefix("ctxt ") {
            let count = value.parse().context("Parse ctxt")?;
            return Ok(ContextSwitchStats { count });
        }
    }

    anyhow::bail!("ctxt not found in /proc/stat")
}

impl ContextSwitchStats {
    pub fn per_sec(&self, prev: &ContextSwitchStats, interval_secs: f32) -> u64 {
        let delta = self.count.saturating_sub(prev.count);
        (delta as f32 / interval_secs) as u64
    }
}

// ===== TCP Connection Stats =====

#[derive(Debug, Clone)]
pub struct TcpStats {
    pub total_connections: u32,
    pub time_wait: u32,
}

pub fn read_tcp_stats() -> Result<TcpStats> {
    let mut total = 0u32;
    let mut time_wait = 0u32;

    // Read IPv4 connections
    if let Ok(content) = fs::read_to_string("/proc/net/tcp") {
        for line in content.lines().skip(1) {
            // Skip header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                total += 1;
                // State is in field 3, TIME_WAIT = 06
                if parts[3] == "06" {
                    time_wait += 1;
                }
            }
        }
    }

    // Read IPv6 connections
    if let Ok(content) = fs::read_to_string("/proc/net/tcp6") {
        for line in content.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                total += 1;
                if parts[3] == "06" {
                    time_wait += 1;
                }
            }
        }
    }

    Ok(TcpStats {
        total_connections: total,
        time_wait,
    })
}

// ===== Per-Process Details =====

#[derive(Debug, Clone)]
pub struct ProcessDetail {
    pub pid: u32,
    pub name: String,
    pub cmdline: String,
    pub state: String,
    #[allow(dead_code)]
    pub cpu_time_jiffies: u64, // Total CPU time (user + system)
    pub mem_bytes: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub num_fds: u32,
    pub num_threads: u32,
}

pub fn read_process_details(pid: u32) -> Result<ProcessDetail> {
    let name = read_process_name(pid)?;
    let cmdline = read_process_cmdline(pid).unwrap_or_else(|_| String::from("[unknown]"));
    let stat = read_process_stat(pid)?;
    let io = read_process_io(pid).unwrap_or_default();
    let num_fds = count_process_fds(pid).unwrap_or(0);
    let num_threads = stat.num_threads;

    Ok(ProcessDetail {
        pid,
        name,
        cmdline,
        state: stat.state,
        cpu_time_jiffies: stat.utime + stat.stime,
        mem_bytes: stat.rss_bytes,
        read_bytes: io.read_bytes,
        write_bytes: io.write_bytes,
        num_fds,
        num_threads,
    })
}

fn read_process_name(pid: u32) -> Result<String> {
    let comm_path = format!("/proc/{}/comm", pid);
    let name = fs::read_to_string(&comm_path)
        .context("Failed to read comm")?
        .trim()
        .to_string();
    Ok(name)
}

fn read_process_cmdline(pid: u32) -> Result<String> {
    let cmdline_path = format!("/proc/{}/cmdline", pid);
    let content = fs::read_to_string(&cmdline_path).context("Failed to read cmdline")?;

    // cmdline uses null bytes as separators
    let cmdline = content
        .replace('\0', " ")
        .trim()
        .to_string();

    if cmdline.is_empty() {
        anyhow::bail!("Empty cmdline");
    }

    Ok(cmdline)
}

struct ProcessStat {
    state: String,
    utime: u64,
    stime: u64,
    rss_bytes: u64,
    num_threads: u32,
}

fn read_process_stat(pid: u32) -> Result<ProcessStat> {
    let stat_path = format!("/proc/{}/stat", pid);
    let content = fs::read_to_string(&stat_path).context("Failed to read stat")?;

    // Parse /proc/[pid]/stat - format is complex due to comm field containing spaces and parens
    let _start = content.find('(').context("Invalid stat format")?;
    let end = content.rfind(')').context("Invalid stat format")?;
    let after_comm = &content[end + 2..]; // Skip ") "
    let parts: Vec<&str> = after_comm.split_whitespace().collect();

    if parts.len() < 22 {
        anyhow::bail!("Not enough fields in stat");
    }

    Ok(ProcessStat {
        state: parts[0].to_string(),                             // Field 3
        utime: parts[11].parse().unwrap_or(0),                   // Field 14
        stime: parts[12].parse().unwrap_or(0),                   // Field 15
        num_threads: parts[17].parse().unwrap_or(1),             // Field 20
        rss_bytes: parts[21].parse::<u64>().unwrap_or(0) * 4096, // Field 24 (pages to bytes)
    })
}

#[derive(Default)]
struct ProcessIo {
    read_bytes: u64,
    write_bytes: u64,
}

fn read_process_io(pid: u32) -> Result<ProcessIo> {
    let io_path = format!("/proc/{}/io", pid);
    let content = fs::read_to_string(&io_path).context("Failed to read io")?;

    let mut io = ProcessIo::default();

    for line in content.lines() {
        if let Some(value) = line.strip_prefix("read_bytes: ") {
            io.read_bytes = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("write_bytes: ") {
            io.write_bytes = value.parse().unwrap_or(0);
        }
    }

    Ok(io)
}

fn count_process_fds(pid: u32) -> Result<u32> {
    let fd_path = format!("/proc/{}/fd", pid);
    let count = fs::read_dir(&fd_path)
        .context("Failed to read fd dir")?
        .count() as u32;
    Ok(count)
}

// ===== Process Tracking =====

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub state: String,
}

pub type ProcessSnapshot = HashMap<u32, ProcessInfo>;

pub fn read_processes() -> Result<ProcessSnapshot> {
    let mut processes = HashMap::new();

    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if let Ok(pid) = name_str.parse::<u32>() {
            if let Ok(name) = read_process_name(pid) {
                if let Ok(stat) = read_process_stat(pid) {
                    processes.insert(
                        pid,
                        ProcessInfo {
                            pid,
                            name,
                            state: stat.state,
                        },
                    );
                }
            }
        }
    }

    Ok(processes)
}

#[derive(Debug)]
pub struct ProcessDiff {
    pub started: Vec<ProcessInfo>,
    pub exited: Vec<ProcessInfo>,
    pub stuck: Vec<ProcessInfo>,    // D state
    pub zombie: Vec<ProcessInfo>,   // Z state
}

pub fn diff_processes(prev: &ProcessSnapshot, current: &ProcessSnapshot) -> ProcessDiff {
    let mut started = Vec::new();
    let mut exited = Vec::new();
    let mut stuck = Vec::new();
    let mut zombie = Vec::new();

    // Find newly started processes and state changes
    for (pid, info) in current {
        if !prev.contains_key(pid) {
            started.push(info.clone());
        } else if let Some(prev_info) = prev.get(pid) {
            // Check for state transitions (not just current state)
            if info.state == "D" && prev_info.state != "D" {
                stuck.push(info.clone());
            } else if info.state == "Z" && prev_info.state != "Z" {
                zombie.push(info.clone());
            }
        }
    }

    // Find exited processes
    for (pid, info) in prev {
        if !current.contains_key(pid) {
            exited.push(info.clone());
        }
    }

    ProcessDiff {
        started,
        exited,
        stuck,
        zombie,
    }
}

// ===== Security Monitoring =====

#[derive(Debug, Clone)]
pub struct LoggedInUser {
    pub username: String,
    pub terminal: String,
    #[allow(dead_code)]
    pub login_time: String,
    pub remote_host: Option<String>,
}

pub fn read_logged_in_users() -> Result<Vec<LoggedInUser>> {
    // Use 'w' command as it's more reliable than 'who' on some systems
    let output = std::process::Command::new("w")
        .args(["-h"]) // no header
        .output()
        .context("Failed to run w")?;

    let content = String::from_utf8_lossy(&output.stdout);
    let mut users = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // w output: USER TTY FROM LOGIN@ IDLE JCPU PCPU WHAT
        if parts.len() >= 4 {
            let terminal = parts[1].to_string();
            let from = parts[2].to_string();
            let login_time = parts[3].to_string();

            // Get full username via stat on the tty device (w truncates usernames)
            let tty_path = if terminal.starts_with("pts/") {
                format!("/dev/{}", terminal)
            } else {
                format!("/dev/{}", terminal)
            };
            let username = std::process::Command::new("stat")
                .args(["-c", "%U", &tty_path])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| parts[0].to_string());

            let remote_host = if from != "-" && !from.is_empty() {
                Some(from)
            } else {
                None
            };

            users.push(LoggedInUser {
                username,
                terminal,
                login_time,
                remote_host,
            });
        }
    }

    Ok(users)
}

#[derive(Debug, Clone)]
pub struct AuthLogEntry {
    #[allow(dead_code)]
    pub timestamp: String,
    pub event_type: AuthEventType,
    pub user: String,
    pub source_ip: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthEventType {
    SshSuccess,
    SshFailure,
    SudoCommand,
    #[allow(dead_code)]
    FailedPassword,
    InvalidUser,
    #[allow(dead_code)]
    Other,
}

pub fn tail_auth_log(last_position: &mut u64) -> Result<Vec<AuthLogEntry>> {
    use std::io::{Read, Seek, SeekFrom};

    let auth_log_paths = [
        "/var/log/auth.log",      // Debian/Ubuntu
        "/var/log/secure",        // RHEL/CentOS
    ];

    let auth_log = auth_log_paths.iter()
        .find(|path| std::path::Path::new(path).exists())
        .context("No auth log found")?;

    let mut file = std::fs::File::open(auth_log)
        .context("Failed to open auth log")?;

    let file_len = file.metadata()?.len();

    // If file was rotated, start from beginning
    if *last_position > file_len {
        *last_position = 0;
    }

    file.seek(SeekFrom::Start(*last_position))?;

    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;

    *last_position = file_len;

    let mut entries = Vec::new();

    for line in buffer.lines() {
        if let Some(entry) = parse_auth_log_line(line) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn parse_auth_log_line(line: &str) -> Option<AuthLogEntry> {
    // Parse common auth log formats
    let parts: Vec<&str> = line.splitn(4, ' ').collect();
    if parts.len() < 4 {
        return None;
    }

    let timestamp = format!("{} {} {}", parts[0], parts[1], parts[2]);
    let rest = parts[3];

    let (event_type, user, source_ip) = if rest.contains("sshd") {
        if rest.contains("Accepted password") || rest.contains("Accepted publickey") {
            let user = extract_after(rest, "for ")?;
            let ip = extract_after(rest, "from ");
            (AuthEventType::SshSuccess, user, ip)
        } else if rest.contains("Failed password") {
            let user = extract_after(rest, "for ").or_else(|| Some("unknown".to_string()))?;
            let ip = extract_after(rest, "from ");
            (AuthEventType::SshFailure, user, ip)
        } else if rest.contains("Invalid user") {
            let user = extract_after(rest, "Invalid user ").or_else(|| Some("unknown".to_string()))?;
            let ip = extract_after(rest, "from ");
            (AuthEventType::InvalidUser, user, ip)
        } else {
            return None;
        }
    } else if rest.contains("sudo:") && (rest.contains("COMMAND=") || rest.contains("session opened")) {
        // Extract username - format is usually "hostname sudo: username : ..."
        let user = if let Some(pos) = rest.find("sudo:") {
            let after_sudo = &rest[pos + 5..].trim_start();
            after_sudo.split_whitespace()
                .next()
                .unwrap_or("unknown")
                .trim_end_matches(':')
                .to_string()
        } else {
            "unknown".to_string()
        };
        (AuthEventType::SudoCommand, user, None)
    } else {
        return None;
    };

    Some(AuthLogEntry {
        timestamp,
        event_type,
        user,
        source_ip,
        message: rest.to_string(),
    })
}

fn extract_after(text: &str, marker: &str) -> Option<String> {
    text.find(marker).map(|pos| {
        let after = &text[pos + marker.len()..];
        after.split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    })
}

// ===== Port Scan Detection =====

#[derive(Debug)]
pub struct ConnectionTracker {
    // Track connections per source IP to detect scanning
    connections_per_ip: HashMap<String, Vec<u16>>, // IP -> ports attempted
}

impl ConnectionTracker {
    pub fn new() -> Self {
        Self {
            connections_per_ip: HashMap::new(),
        }
    }

    pub fn update(&mut self) -> Result<Vec<String>> {
        // Read current TCP connections
        let mut new_connections: HashMap<String, Vec<u16>> = HashMap::new();

        if let Ok(content) = fs::read_to_string("/proc/net/tcp") {
            for line in content.lines().skip(1) {
                if let Some((src_ip, src_port)) = parse_tcp_line(line) {
                    new_connections.entry(src_ip.clone())
                        .or_insert_with(Vec::new)
                        .push(src_port);
                }
            }
        }

        // Detect potential port scans (many ports from same IP)
        let mut alerts = Vec::new();
        for (ip, ports) in &new_connections {
            if ports.len() > 20 {
                // Same IP connecting to 20+ different ports
                alerts.push(format!("Potential port scan from {}: {} ports", ip, ports.len()));
            }
        }

        self.connections_per_ip = new_connections;
        Ok(alerts)
    }
}

fn parse_tcp_line(line: &str) -> Option<(String, u16)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    // Remote address is in format: hex_ip:hex_port
    let remote_addr = parts[2];
    let addr_parts: Vec<&str> = remote_addr.split(':').collect();
    if addr_parts.len() != 2 {
        return None;
    }

    // Parse hex IP (stored in reverse byte order for IPv4)
    let ip_hex = addr_parts[0];
    if ip_hex.len() == 8 {
        // IPv4
        if let Ok(ip_num) = u32::from_str_radix(ip_hex, 16) {
            let ip = format!(
                "{}.{}.{}.{}",
                ip_num & 0xFF,
                (ip_num >> 8) & 0xFF,
                (ip_num >> 16) & 0xFF,
                (ip_num >> 24) & 0xFF
            );

            let port = u16::from_str_radix(addr_parts[1], 16).ok()?;
            return Some((ip, port));
        }
    }

    None
}

// ===== Top Processes =====

pub fn get_top_processes(n: usize) -> Result<Vec<ProcessDetail>> {
    let mut processes = Vec::new();

    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if let Ok(pid) = name_str.parse::<u32>() {
            if let Ok(detail) = read_process_details(pid) {
                processes.push(detail);
            }
        }
    }

    // Sort by memory usage (descending)
    processes.sort_by(|a, b| b.mem_bytes.cmp(&a.mem_bytes));
    processes.truncate(n);

    Ok(processes)
}

// ===== Temperature Monitoring =====

use std::sync::OnceLock;

// Parse temperature from millidegrees to Celsius
fn parse_temp_millidegrees(path: &std::path::Path) -> Result<f32> {
    let content = fs::read_to_string(path)?;
    let millidegrees: i32 = content.trim().parse()?;
    Ok(millidegrees as f32 / 1000.0)
}

// Execute command with basic error handling
fn execute_command_timeout(cmd: &str, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new(cmd)
        .args(args)
        .output()
        .context("Failed to execute command")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        anyhow::bail!("Command failed")
    }
}

// CPU Temperature
fn read_cpu_temperature() -> Result<Option<f32>> {
    // Try thermal zones first
    let thermal_zone_pattern = "/sys/class/thermal/thermal_zone*/temp";
    let mut max_temp = None;

    if let Ok(paths) = glob::glob(thermal_zone_pattern) {
        for entry in paths.flatten() {
            if let Ok(temp) = parse_temp_millidegrees(&entry) {
                max_temp = Some(max_temp.unwrap_or(0.0_f32).max(temp));
            }
        }
    }

    if max_temp.is_some() {
        return Ok(max_temp);
    }

    // Fallback to hwmon
    let hwmon_pattern = "/sys/class/hwmon/hwmon*/temp*_input";
    if let Ok(paths) = glob::glob(hwmon_pattern) {
        for entry in paths.flatten() {
            if let Ok(temp) = parse_temp_millidegrees(&entry) {
                max_temp = Some(max_temp.unwrap_or(0.0_f32).max(temp));
            }
        }
    }

    Ok(max_temp)
}

// GPU Temperature
#[derive(Debug, Clone, Copy)]
enum GpuCommand {
    NvidiaSmi,
    RocmSmi,
    None,
}

static GPU_COMMAND: OnceLock<GpuCommand> = OnceLock::new();

fn detect_gpu_command() -> GpuCommand {
    if std::process::Command::new("nvidia-smi").arg("--version").output().is_ok() {
        return GpuCommand::NvidiaSmi;
    }
    if std::process::Command::new("rocm-smi").arg("--version").output().is_ok() {
        return GpuCommand::RocmSmi;
    }
    GpuCommand::None
}

fn read_gpu_temperature() -> Result<Option<f32>> {
    let cmd = GPU_COMMAND.get_or_init(detect_gpu_command);

    match cmd {
        GpuCommand::NvidiaSmi => {
            let output = execute_command_timeout(
                "nvidia-smi",
                &["--query-gpu=temperature.gpu", "--format=csv,noheader,nounits"],
            )?;
            let temp: f32 = output.trim().parse()?;
            Ok(Some(temp))
        }
        GpuCommand::RocmSmi => {
            let output = execute_command_timeout("rocm-smi", &["--showtemp"])?;
            // Parse output - format varies, look for temperature value
            for line in output.lines() {
                if line.contains("Temperature") {
                    if let Some(temp_str) = line.split_whitespace().find(|s| s.parse::<f32>().is_ok()) {
                        if let Ok(temp) = temp_str.parse::<f32>() {
                            return Ok(Some(temp));
                        }
                    }
                }
            }
            Ok(None)
        }
        GpuCommand::None => Ok(None),
    }
}

fn try_smartctl(dev_path: &str) -> Result<Option<f32>> {
    let output = execute_command_timeout("smartctl", &["-A", dev_path])?;

    for line in output.lines() {
        if line.contains("Temperature") || line.contains("temperature") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            for part in parts {
                if let Ok(temp) = part.parse::<f32>() {
                    if temp > 0.0 && temp < 100.0 {
                        return Ok(Some(temp));
                    }
                }
            }
        }
    }
    Ok(None)
}

fn try_hddtemp(dev_path: &str) -> Result<Option<f32>> {
    let output = execute_command_timeout("hddtemp", &[dev_path])?;

    // Parse: /dev/sda: DISK_NAME: 42°C
    for part in output.split(':') {
        if part.contains("°C") || part.contains("C") {
            let temp_str = part.trim().replace("°C", "").replace("C", "");
            if let Ok(temp) = temp_str.parse::<f32>() {
                return Ok(Some(temp));
            }
        }
    }
    Ok(None)
}

// Motherboard Temperature
fn read_motherboard_temperature() -> Result<Option<f32>> {
    let hwmon_pattern = "/sys/class/hwmon/hwmon*";

    if let Ok(paths) = glob::glob(hwmon_pattern) {
        for dir in paths.flatten() {
            // Look for temperature inputs
            let temp_pattern = format!("{}/*_input", dir.display());
            if let Ok(temp_paths) = glob::glob(&temp_pattern) {
                for temp_path in temp_paths.flatten() {
                    // Check corresponding label file
                    let label_path = temp_path.to_string_lossy().replace("_input", "_label");
                    if let Ok(label) = fs::read_to_string(&label_path) {
                        let label_lower = label.to_lowercase();
                        if label_lower.contains("motherboard") ||
                           label_lower.contains("chipset") ||
                           label_lower.contains("pch") {
                            if let Ok(temp) = parse_temp_millidegrees(&temp_path) {
                                return Ok(Some(temp));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

// Main wrapper function
pub fn read_temperatures() -> crate::event::TemperatureReadings {
    crate::event::TemperatureReadings {
        cpu_temp_celsius: read_cpu_temperature().ok().flatten(),
        per_core_temps: Vec::new(),  // Will be populated separately in main loop
        gpu_temp_celsius: read_gpu_temperature().ok().flatten(),
        motherboard_temp_celsius: read_motherboard_temperature().ok().flatten(),
    }
}

// ===== Per-Core Temperature =====

pub fn read_per_core_temperatures(num_cores: usize) -> Vec<Option<f32>> {
    let mut core_temps: HashMap<u32, f32> = HashMap::new();

    // Try to map thermal zones to cores
    if let Ok(paths) = glob::glob("/sys/class/thermal/thermal_zone*/") {
        for zone_path in paths.flatten() {
            if let Ok(type_str) = fs::read_to_string(zone_path.join("type")) {
                let type_name = type_str.trim();

                if type_name.contains("coretemp") {
                    if let Some(zone_name) = zone_path.file_name().and_then(|n| n.to_str()) {
                        if let Some(idx_str) = zone_name.strip_prefix("thermal_zone") {
                            if let Ok(zone_idx) = idx_str.parse::<u32>() {
                                if let Ok(temp) = parse_temp_millidegrees(&zone_path.join("temp")) {
                                    core_temps.insert(zone_idx, temp);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Build result vector with proper ordering
    let mut result = Vec::with_capacity(num_cores);
    for core_id in 0..num_cores {
        result.push(core_temps.get(&(core_id as u32)).copied());
    }

    // If no per-core temps found, fall back to aggregate CPU temp
    if core_temps.is_empty() {
        if let Some(aggregate_temp) = read_cpu_temperature().ok().flatten() {
            result = vec![Some(aggregate_temp); num_cores];
        }
    }

    result
}

// ===== Per-Disk Temperature =====

use std::collections::HashMap as StdHashMap;

static DISK_TEMPS_CACHE: OnceLock<std::sync::Mutex<StdHashMap<String, CachedDiskTemp>>> = OnceLock::new();

#[derive(Debug, Clone)]
struct CachedDiskTemp {
    temp: Option<f32>,
    last_update: std::time::Instant,
}

fn get_physical_disks() -> Result<Vec<String>> {
    let content = fs::read_to_string("/proc/diskstats")?;
    let mut disks = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let dev_name = parts[2];
            if is_physical_disk(dev_name) {
                disks.push(dev_name.to_string());
            }
        }
    }

    Ok(disks)
}

pub fn read_disk_temperatures() -> StdHashMap<String, Option<f32>> {
    let mut temps = StdHashMap::new();

    let Ok(disks) = get_physical_disks() else {
        return temps;
    };

    let cache = DISK_TEMPS_CACHE.get_or_init(|| std::sync::Mutex::new(StdHashMap::new()));
    let mut cache_lock = cache.lock().unwrap();

    for disk in disks {
        // Check cache (30-second interval per disk)
        if let Some(cached) = cache_lock.get(&disk) {
            if cached.last_update.elapsed().as_secs() < 30 {
                temps.insert(disk.clone(), cached.temp);
                continue;
            }
        }

        // Read fresh temperature
        let dev_path = format!("/dev/{}", disk);
        let temp = try_smartctl(&dev_path)
            .or_else(|_| try_hddtemp(&dev_path))
            .ok()
            .flatten();

        // Update cache
        cache_lock.insert(disk.clone(), CachedDiskTemp {
            temp,
            last_update: std::time::Instant::now(),
        });

        temps.insert(disk, temp);
    }

    temps
}

// ===== Fan Speed Monitoring =====

pub fn read_fan_speeds() -> Vec<crate::event::FanReading> {
    let mut fans = Vec::new();

    let hwmon_pattern = "/sys/class/hwmon/hwmon*";

    if let Ok(paths) = glob::glob(hwmon_pattern) {
        for dir in paths.flatten() {
            let fan_pattern = format!("{}/*_input", dir.display());
            if let Ok(fan_paths) = glob::glob(&fan_pattern) {
                for fan_path in fan_paths.flatten() {
                    let path_str = fan_path.to_string_lossy();

                    // Only process fan*_input files
                    if !path_str.contains("fan") {
                        continue;
                    }

                    // Read RPM value
                    if let Ok(rpm_str) = fs::read_to_string(&fan_path) {
                        if let Ok(rpm) = rpm_str.trim().parse::<u32>() {
                            // Skip if fan is not spinning or invalid
                            if rpm == 0 || rpm > 50000 {
                                continue;
                            }

                            // Try to read label
                            let label_path = path_str.replace("_input", "_label");
                            let label = fs::read_to_string(&label_path)
                                .ok()
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|| {
                                    if let Some(fan_num) = path_str
                                        .split('/')
                                        .last()
                                        .and_then(|s| s.strip_prefix("fan"))
                                        .and_then(|s| s.chars().next())
                                        .and_then(|c| c.to_digit(10)) {
                                        format!("Fan {}", fan_num)
                                    } else {
                                        "Unknown Fan".to_string()
                                    }
                                });

                            fans.push(crate::event::FanReading { label, rpm });
                        }
                    }
                }
            }
        }
    }

    // Sort by label for consistent ordering
    fans.sort_by(|a, b| a.label.cmp(&b.label));
    fans
}

// ===== User Account Monitoring =====

use std::sync::Mutex;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

static PASSWD_HASH: OnceLock<Mutex<u64>> = OnceLock::new();
static GROUP_HASH: OnceLock<Mutex<u64>> = OnceLock::new();
static SUDOERS_HASH: OnceLock<Mutex<u64>> = OnceLock::new();

fn hash_file(path: &str) -> Result<u64> {
    let content = fs::read_to_string(path)?;
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    Ok(hasher.finish())
}

pub fn check_passwd_changes() -> Result<Option<String>> {
    let current_hash = match hash_file("/etc/passwd") {
        Ok(h) => h,
        Err(_) => return Ok(None), // File not readable, skip check
    };

    let mutex = PASSWD_HASH.get_or_init(|| Mutex::new(current_hash));
    let mut last_hash = mutex.lock().unwrap();

    if *last_hash != current_hash {
        *last_hash = current_hash;
        return Ok(Some("User account file /etc/passwd modified".to_string()));
    }

    Ok(None)
}

pub fn check_group_changes() -> Result<Option<String>> {
    let current_hash = match hash_file("/etc/group") {
        Ok(h) => h,
        Err(_) => return Ok(None), // File not readable, skip check
    };

    let mutex = GROUP_HASH.get_or_init(|| Mutex::new(current_hash));
    let mut last_hash = mutex.lock().unwrap();

    if *last_hash != current_hash {
        *last_hash = current_hash;
        return Ok(Some("Group file /etc/group modified".to_string()));
    }

    Ok(None)
}

pub fn check_sudoers_changes() -> Result<Option<String>> {
    // Check main sudoers file (may not be readable without root)
    let current_hash = hash_file("/etc/sudoers").unwrap_or(0);

    // Also check sudoers.d directory if it exists
    let mut sudoers_d_hash = 0u64;
    if let Ok(entries) = fs::read_dir("/etc/sudoers.d") {
        for entry in entries.flatten() {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                let mut hasher = DefaultHasher::new();
                content.hash(&mut hasher);
                sudoers_d_hash ^= hasher.finish();
            }
        }
    }

    let combined_hash = current_hash ^ sudoers_d_hash;

    // If we couldn't read anything (no permissions), skip this check
    if combined_hash == 0 {
        return Ok(None);
    }

    let mutex = SUDOERS_HASH.get_or_init(|| Mutex::new(combined_hash));
    let mut last_hash = mutex.lock().unwrap();

    if *last_hash != combined_hash && *last_hash != 0 {
        *last_hash = combined_hash;
        return Ok(Some("Sudoers configuration modified".to_string()));
    }

    // Update the hash on first run
    if *last_hash == 0 {
        *last_hash = combined_hash;
    }

    Ok(None)
}

// ===== Listening Port Monitoring =====

static LISTENING_PORTS: OnceLock<Mutex<std::collections::HashSet<(String, u16)>>> = OnceLock::new();

pub fn check_listening_port_changes() -> Result<(Vec<(String, u16)>, Vec<(String, u16)>)> {
    let current_ports = match get_listening_ports() {
        Ok(p) => p,
        Err(_) => return Ok((vec![], vec![])), // Skip if we can't read ports
    };

    let mutex = LISTENING_PORTS.get_or_init(|| Mutex::new(current_ports.clone()));
    let mut last_ports = mutex.lock().unwrap();

    // Find new and closed ports
    let new_ports: Vec<_> = current_ports.difference(&*last_ports).cloned().collect();
    let closed_ports: Vec<_> = last_ports.difference(&current_ports).cloned().collect();

    *last_ports = current_ports;

    Ok((new_ports, closed_ports))
}

fn get_listening_ports() -> Result<std::collections::HashSet<(String, u16)>> {
    let mut ports = std::collections::HashSet::new();

    // Read TCP listening ports
    if let Ok(content) = fs::read_to_string("/proc/net/tcp") {
        for line in content.lines().skip(1) {
            if let Some((addr, port, state)) = parse_tcp_line_with_state(line) {
                // State 0A = TCP_LISTEN
                if state == "0A" {
                    ports.insert((format!("tcp:{}", addr), port));
                }
            }
        }
    }

    // Read TCP6 listening ports
    if let Ok(content) = fs::read_to_string("/proc/net/tcp6") {
        for line in content.lines().skip(1) {
            if let Some((addr, port, state)) = parse_tcp_line_with_state(line) {
                if state == "0A" {
                    ports.insert((format!("tcp6:{}", addr), port));
                }
            }
        }
    }

    // Read UDP listening ports
    if let Ok(content) = fs::read_to_string("/proc/net/udp") {
        for line in content.lines().skip(1) {
            if let Some((addr, port, _)) = parse_tcp_line_with_state(line) {
                ports.insert((format!("udp:{}", addr), port));
            }
        }
    }

    Ok(ports)
}

fn parse_tcp_line_with_state(line: &str) -> Option<(String, u16, String)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }

    // Parse local address
    let local_addr = parts[1];
    let addr_parts: Vec<&str> = local_addr.split(':').collect();
    if addr_parts.len() != 2 {
        return None;
    }

    let ip_hex = addr_parts[0];
    let port_hex = addr_parts[1];

    // Parse IP address (reversed byte order)
    let ip = if ip_hex.len() == 8 {
        let bytes = (0..4)
            .map(|i| u8::from_str_radix(&ip_hex[i*2..(i+1)*2], 16).unwrap_or(0))
            .collect::<Vec<_>>();
        format!("{}.{}.{}.{}", bytes[3], bytes[2], bytes[1], bytes[0])
    } else {
        "::".to_string()
    };

    // Parse port
    let port = u16::from_str_radix(port_hex, 16).ok()?;

    // Get state
    let state = parts.get(3)?.to_string();

    Some((ip, port, state))
}

// ===== Kernel Module Monitoring =====

static KERNEL_MODULES: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();

pub fn check_kernel_module_changes() -> Result<(Vec<String>, Vec<String>)> {
    let current_modules = match get_loaded_modules() {
        Ok(m) => m,
        Err(_) => return Ok((vec![], vec![])), // Skip if we can't read modules
    };

    let mutex = KERNEL_MODULES.get_or_init(|| Mutex::new(current_modules.clone()));
    let mut last_modules = mutex.lock().unwrap();

    // Find loaded and unloaded modules
    let loaded: Vec<_> = current_modules.difference(&*last_modules).cloned().collect();
    let unloaded: Vec<_> = last_modules.difference(&current_modules).cloned().collect();

    *last_modules = current_modules;

    Ok((loaded, unloaded))
}

fn get_loaded_modules() -> Result<std::collections::HashSet<String>> {
    let mut modules = std::collections::HashSet::new();

    let content = fs::read_to_string("/proc/modules")?;
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let Some(module_name) = parts.first() {
            modules.insert(module_name.to_string());
        }
    }

    Ok(modules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_auth_log_line_ssh_success_password() {
        let line = "Jan 15 10:23:45 server sshd[1234]: Accepted password for ubuntu from 192.168.1.100 port 54321 ssh2";
        let entry = parse_auth_log_line(line).unwrap();

        assert_eq!(entry.event_type, AuthEventType::SshSuccess);
        assert_eq!(entry.user, "ubuntu");
        assert_eq!(entry.source_ip, Some("192.168.1.100".to_string()));
        assert!(entry.message.contains("Accepted password"));
    }

    #[test]
    fn test_parse_auth_log_line_ssh_success_publickey() {
        let line = "Jan 15 10:23:45 server sshd[1234]: Accepted publickey for admin from 10.0.0.5 port 22222 ssh2";
        let entry = parse_auth_log_line(line).unwrap();

        assert_eq!(entry.event_type, AuthEventType::SshSuccess);
        assert_eq!(entry.user, "admin");
        assert_eq!(entry.source_ip, Some("10.0.0.5".to_string()));
    }

    #[test]
    fn test_parse_auth_log_line_ssh_failure() {
        let line = "Jan 15 10:23:45 server sshd[1234]: Failed password for testuser from 1.2.3.4 port 12345 ssh2";
        let entry = parse_auth_log_line(line).unwrap();

        assert_eq!(entry.event_type, AuthEventType::SshFailure);
        assert_eq!(entry.user, "testuser");
        assert_eq!(entry.source_ip, Some("1.2.3.4".to_string()));
    }

    #[test]
    fn test_parse_auth_log_line_invalid_user() {
        let line = "Jan 15 10:23:45 server sshd[1234]: Invalid user testuser from 5.6.7.8";
        let entry = parse_auth_log_line(line).unwrap();

        assert_eq!(entry.event_type, AuthEventType::InvalidUser);
        assert_eq!(entry.user, "testuser");
        assert_eq!(entry.source_ip, Some("5.6.7.8".to_string()));
    }

    #[test]
    fn test_parse_auth_log_line_sudo_command() {
        let line = "Jan 15 10:23:45 server sudo: ubuntu : COMMAND=/usr/bin/apt update";
        let entry = parse_auth_log_line(line).unwrap();

        assert_eq!(entry.event_type, AuthEventType::SudoCommand);
        assert_eq!(entry.user, "ubuntu");
        assert_eq!(entry.source_ip, None);
    }

    #[test]
    fn test_parse_auth_log_line_sudo_session() {
        let line = "Jan 15 10:23:45 server sudo: ubuntu : session opened for user root";
        let entry = parse_auth_log_line(line).unwrap();

        assert_eq!(entry.event_type, AuthEventType::SudoCommand);
        assert_eq!(entry.user, "ubuntu");
    }

    #[test]
    fn test_parse_auth_log_line_invalid() {
        let line = "Jan 15 10:23:45 server kernel: some random message";
        let entry = parse_auth_log_line(line);

        assert!(entry.is_none());
    }

    #[test]
    fn test_parse_auth_log_line_malformed() {
        let line = "invalid log line";
        let entry = parse_auth_log_line(line);

        assert!(entry.is_none());
    }

    #[test]
    fn test_parse_tcp_line_valid() {
        // Format: local_address:port remote_address:port state...
        // 0100007F = 127.0.0.1 in hex (reversed bytes)
        // 1F90 = 8080 in hex
        let line = "   1: 0100007F:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 12345 1 0000000000000000 100 0 0 10 0";
        let result = parse_tcp_line(line);

        assert!(result.is_some());
        let (ip, port) = result.unwrap();
        assert_eq!(ip, "0.0.0.0");
        assert_eq!(port, 0);
    }

    #[test]
    fn test_parse_tcp_line_specific_ip() {
        // C0A80164 = 192.168.1.100 in hex (reversed bytes: 100.1.168.192 -> reverse each byte)
        let line = "   1: 0100007F:1F90 C0A80164:01BB 01 00000000:00000000 00:00000000 00000000     0        0 12345 1 0000000000000000 100 0 0 10 0";
        let result = parse_tcp_line(line);

        assert!(result.is_some());
        let (ip, port) = result.unwrap();
        // The function parses in reverse byte order
        assert_eq!(ip, "100.1.168.192");
        assert_eq!(port, 443); // 01BB = 443
    }

    #[test]
    fn test_parse_tcp_line_invalid() {
        let line = "invalid tcp line";
        let result = parse_tcp_line(line);

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_tcp_line_insufficient_fields() {
        let line = "   1: 0100007F:1F90";
        let result = parse_tcp_line(line);

        assert!(result.is_none());
    }

    #[test]
    fn test_extract_after_found() {
        let text = "foo bar baz qux";
        let result = extract_after(text, "bar ");

        assert_eq!(result, Some("baz".to_string()));
    }

    #[test]
    fn test_extract_after_not_found() {
        let text = "foo bar baz";
        let result = extract_after(text, "missing ");

        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_after_at_end() {
        let text = "foo bar";
        let result = extract_after(text, "bar");

        assert_eq!(result, Some("".to_string()));
    }

    #[test]
    fn test_cpu_usage_calculation() {
        let prev = CpuStats {
            user: 1000,
            nice: 0,
            system: 500,
            idle: 8500,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
        };

        let current = CpuStats {
            user: 1500,
            nice: 0,
            system: 600,
            idle: 8900,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
        };

        let usage = current.usage_percent(&prev);
        // Total delta: 11000 - 10000 = 1000
        // Idle delta: 8900 - 8500 = 400
        // Busy delta: 1000 - 400 = 600
        // Usage: 600 / 1000 * 100 = 60%
        assert!((usage - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_disk_stats_bytes_per_sec() {
        let prev = DiskStats {
            read_bytes: 1000000,
            write_bytes: 2000000,
        };

        let current = DiskStats {
            read_bytes: 1500000,
            write_bytes: 2800000,
        };

        let (read_per_sec, write_per_sec) = current.bytes_per_sec(&prev, 1.0);
        assert_eq!(read_per_sec, 500000);
        assert_eq!(write_per_sec, 800000);
    }

    #[test]
    fn test_memory_stats_used_calculation() {
        let stats = MemoryStats {
            total_kb: 16000000,
            free_kb: 2000000,
            available_kb: 10000000,
            buffers_kb: 1000000,
            cached_kb: 3000000,
        };

        // Used = total - (free + buffers + cached)
        // = 16000000 - (2000000 + 1000000 + 3000000) = 10000000
        assert_eq!(stats.used_kb(), 10000000);
    }

    #[test]
    fn test_memory_stats_usage_percent() {
        let stats = MemoryStats {
            total_kb: 10000,
            free_kb: 2000,
            available_kb: 5000,
            buffers_kb: 1000,
            cached_kb: 2000,
        };

        // Used = 10000 - (2000 + 1000 + 2000) = 5000
        // Usage = 5000 / 10000 * 100 = 50%
        let usage = stats.usage_percent();
        assert!((usage - 50.0).abs() < 0.01);
    }
}
