use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    SystemMetrics(SystemMetrics),
    ProcessLifecycle(ProcessLifecycle),
    ProcessSnapshot(ProcessSnapshot),
    SecurityEvent(SecurityEvent),
    Anomaly(Anomaly),
}

// System-wide metrics collected each interval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub ts: OffsetDateTime,
    pub cpu_usage_percent: f32,
    pub per_core_usage: Vec<f32>,
    pub mem_used_bytes: u64,
    pub mem_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub load_avg_1m: f32,
    pub load_avg_5m: f32,
    pub load_avg_15m: f32,
    pub disk_read_bytes_per_sec: u64,
    pub disk_write_bytes_per_sec: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub per_disk_metrics: Vec<PerDiskMetrics>,
    pub net_recv_bytes_per_sec: u64,
    pub net_send_bytes_per_sec: u64,
    pub tcp_connections: u32,
    pub tcp_time_wait: u32,
    pub context_switches_per_sec: u64,
    pub temps: TemperatureReadings,
    pub fans: Vec<FanReading>,
}

// Temperature readings from various sensors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureReadings {
    pub cpu_temp_celsius: Option<f32>,
    pub per_core_temps: Vec<Option<f32>>,
    pub gpu_temp_celsius: Option<f32>,
    pub motherboard_temp_celsius: Option<f32>,
}

// Fan speed readings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanReading {
    pub label: String,
    pub rpm: u32,
}

// Per-disk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerDiskMetrics {
    pub device_name: String,
    pub read_bytes_per_sec: u64,
    pub write_bytes_per_sec: u64,
    pub temp_celsius: Option<f32>,
}

// Process lifecycle events (start/exit)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessLifecycle {
    pub ts: OffsetDateTime,
    pub pid: u32,
    pub name: String,
    pub kind: ProcessLifecycleKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessLifecycleKind {
    Started,
    Exited,
    Stuck, // D state (uninterruptible sleep)
    Zombie,
}

// Snapshot of interesting processes (top CPU/memory consumers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSnapshot {
    pub ts: OffsetDateTime,
    pub processes: Vec<ProcessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cmdline: String,
    pub state: String,
    pub cpu_percent: f32,
    pub mem_bytes: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub num_fds: u32,
    pub num_threads: u32,
}

// Security events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub ts: OffsetDateTime,
    pub kind: SecurityEventKind,
    pub user: String,
    pub source_ip: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventKind {
    SshLoginSuccess,
    SshLoginFailure,
    UserLogin,
    UserLogout,
    SudoCommand,
    FailedAuth,
    PortScanDetected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub ts: OffsetDateTime,
    pub severity: AnomalySeverity,
    pub kind: AnomalyKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnomalySeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyKind {
    CpuSpike,
    MemorySpike,
    DiskSpike,
    DiskFull,
    SwapUsage,
    NetworkSpike,
    ContextSwitchSpike,
    ProcessStuck,
    ConnectionExhaustion,
    FdExhaustion,
    ThreadLeak,
    BruteForceAttempt,
    PortScanActivity,
    UnauthorizedAccess,
}

