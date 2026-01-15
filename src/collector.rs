use anyhow::{Context, Result};
use std::{collections::HashMap, fs};

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

pub fn read_cpu_stats() -> Result<CpuStats> {
    let content = fs::read_to_string("/proc/stat").context("Failed to read /proc/stat")?;

    let line = content.lines().next().context("Empty /proc/stat")?;

    if !line.starts_with("cpu ") {
        anyhow::bail!("Unexpected /proc/stat format");
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 9 {
        anyhow::bail!("Not enough fields in /proc/stat cpu line");
    }

    Ok(CpuStats {
        user: parts[1].parse().context("Parse user")?,
        nice: parts[2].parse().context("Parse nice")?,
        system: parts[3].parse().context("Parse system")?,
        idle: parts[4].parse().context("Parse idle")?,
        iowait: parts[5].parse().context("Parse iowait")?,
        irq: parts[6].parse().context("Parse irq")?,
        softirq: parts[7].parse().context("Parse softirq")?,
        steal: parts[8].parse().context("Parse steal")?,
    })
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

pub fn read_disk_stats() -> Result<DiskStats> {
    let content = fs::read_to_string("/proc/diskstats").context("Failed to read /proc/diskstats")?;

    let mut total_read_sectors = 0u64;
    let mut total_write_sectors = 0u64;

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 14 {
            continue;
        }

        // Skip loop devices, ram devices, etc
        let dev_name = parts[2];
        if dev_name.starts_with("loop")
            || dev_name.starts_with("ram")
            || dev_name.starts_with("sr")
        {
            continue;
        }

        // Field 5: sectors read, Field 9: sectors written
        if let (Ok(read), Ok(write)) = (parts[5].parse::<u64>(), parts[9].parse::<u64>()) {
            total_read_sectors += read;
            total_write_sectors += write;
        }
    }

    // Sectors are typically 512 bytes
    Ok(DiskStats {
        read_bytes: total_read_sectors * 512,
        write_bytes: total_write_sectors * 512,
    })
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
    pub login_time: String,
    pub remote_host: Option<String>,
}

pub fn read_logged_in_users() -> Result<Vec<LoggedInUser>> {
    let output = std::process::Command::new("who")
        .output()
        .context("Failed to run who")?;

    let content = String::from_utf8_lossy(&output.stdout);
    let mut users = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let username = parts[0].to_string();
            let terminal = parts[1].to_string();
            let login_time = parts[2..].iter()
                .take_while(|p| !p.starts_with('('))
                .cloned()
                .collect::<Vec<&str>>()
                .join(" ");

            let remote_host = line.find('(').and_then(|start| {
                line[start+1..].find(')').map(|end| {
                    line[start+1..start+1+end].to_string()
                })
            });

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
    FailedPassword,
    InvalidUser,
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
