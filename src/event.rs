use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    SystemMetrics(SystemMetrics),
    ProcessLifecycle(ProcessLifecycle),
    ProcessSnapshot(ProcessSnapshot),
    SecurityEvent(SecurityEvent),
    Anomaly(Anomaly),
    FileSystemEvent(FileSystemEvent),
}

// System-wide metrics collected each interval
// Fields marked Option<T> are collected less frequently (static/semi-static data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub ts: OffsetDateTime,

    // Static fields (collected hourly or on change) - reduce storage by ~50-70%
    pub kernel_version: Option<String>,  // Changes on kernel upgrade
    pub cpu_model: Option<String>,        // Never changes
    pub cpu_mhz: Option<u32>,             // Mostly static
    pub mem_total_bytes: Option<u64>,     // Changes if RAM added/removed
    pub swap_total_bytes: Option<u64>,    // Changes if swap reconfigured
    pub disk_total_bytes: Option<u64>,    // Changes on disk resize

    // Semi-static fields (collected every 5 minutes or on change)
    pub filesystems: Option<Vec<FilesystemInfo>>,  // Mount points change infrequently
    pub net_interface: Option<String>,             // Rarely changes
    pub net_ip_address: Option<String>,            // Already was Option
    pub net_gateway: Option<String>,               // Already was Option
    pub net_dns: Option<String>,                   // Already was Option
    pub fans: Option<Vec<FanReading>>,             // Fan config rarely changes
    pub logged_in_users: Option<Vec<LoggedInUserInfo>>, // Emit on change

    // Dynamic fields (collected every second)
    pub system_uptime_seconds: u64,
    pub cpu_usage_percent: f32,
    pub per_core_usage: Vec<f32>,
    pub mem_used_bytes: u64,
    pub mem_usage_percent: f32,  // Calculated using cached total
    pub swap_used_bytes: u64,
    pub swap_usage_percent: f32,  // Calculated using cached total
    pub load_avg_1m: f32,
    pub load_avg_5m: f32,
    pub load_avg_15m: f32,
    pub disk_read_bytes_per_sec: u64,
    pub disk_write_bytes_per_sec: u64,
    pub disk_used_bytes: u64,
    pub disk_usage_percent: f32,  // Calculated using cached total
    pub per_disk_metrics: Vec<PerDiskMetrics>,
    pub net_recv_bytes_per_sec: u64,
    pub net_send_bytes_per_sec: u64,
    pub net_recv_errors_per_sec: u64,
    pub net_send_errors_per_sec: u64,
    pub net_recv_drops_per_sec: u64,
    pub net_send_drops_per_sec: u64,
    pub tcp_connections: u32,
    pub tcp_time_wait: u32,
    pub context_switches_per_sec: u64,
    pub temps: TemperatureReadings,
    pub gpu: GpuInfo,
}

// Logged in user info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoggedInUserInfo {
    pub username: String,
    pub terminal: String,
    pub remote_host: Option<String>,
}

// Temperature readings from various sensors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemperatureReadings {
    pub cpu_temp_celsius: Option<f32>,
    pub per_core_temps: Vec<Option<f32>>,
    pub gpu_temp_celsius: Option<f32>,
    pub motherboard_temp_celsius: Option<f32>,
}

// GPU info
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GpuInfo {
    pub gpu_freq_mhz: Option<u32>,
    pub mem_freq_mhz: Option<u32>,
    pub gpu_temp_celsius: Option<f32>,
    pub power_watts: Option<f32>,
}

// Fan speed readings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FanReading {
    pub label: String,
    pub rpm: u32,
}

// Per-disk metrics (I/O stats)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerDiskMetrics {
    pub device_name: String,
    pub read_bytes_per_sec: u64,
    pub write_bytes_per_sec: u64,
    pub temp_celsius: Option<f32>,
}

// Filesystem usage stats (like df output)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilesystemInfo {
    pub filesystem: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
}

// Process lifecycle events (start/exit)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessLifecycle {
    pub ts: OffsetDateTime,
    pub pid: u32,
    pub ppid: Option<u32>,           // Parent process ID
    pub name: String,
    pub cmdline: String,             // Full command line with arguments
    pub working_dir: Option<String>, // Working directory when started
    pub user: Option<String>,        // Username
    pub uid: Option<u32>,            // User ID
    pub kind: ProcessLifecycleKind,
    pub exit_code: Option<i32>,      // Exit code (only for Exited kind)
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
    pub total_processes: u32,
    pub running_processes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cmdline: String,
    pub state: String,
    pub user: String,
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
    // Immediate value security events
    UserAccountModified,
    GroupModified,
    FailedSuAttempt,
    SudoersModified,
    NewListeningPort,
    ListeningPortClosed,
    KernelModuleLoaded,
    KernelModuleUnloaded,
    // Persistence and package management
    CronJobModified,
    SystemdServiceModified,
    PackageInstalled,
    PackageRemoved,
    // Sensitive file access
    SensitiveFileAccessed,
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

// File system events (file created/modified/deleted)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemEvent {
    pub ts: OffsetDateTime,
    pub kind: FileSystemEventKind,
    pub path: String,
    pub size: Option<u64>,  // File size if available
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileSystemEventKind {
    Created,
    Modified,
    Deleted,
    Renamed { from: String, to: String },
}

impl Event {
    /// Get the timestamp from any event variant
    pub fn timestamp(&self) -> OffsetDateTime {
        match self {
            Event::SystemMetrics(e) => e.ts,
            Event::ProcessLifecycle(e) => e.ts,
            Event::ProcessSnapshot(e) => e.ts,
            Event::SecurityEvent(e) => e.ts,
            Event::Anomaly(e) => e.ts,
            Event::FileSystemEvent(e) => e.ts,
        }
    }
}

/// Static/semi-static system metadata
/// Stored separately from time-series events for efficient access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    pub kernel_version: Option<String>,
    pub cpu_model: Option<String>,
    pub cpu_mhz: Option<u32>,
    pub mem_total_bytes: Option<u64>,
    pub swap_total_bytes: Option<u64>,
    pub disk_total_bytes: Option<u64>,
    pub filesystems: Option<Vec<FilesystemInfo>>,
    pub net_interface: Option<String>,
    pub net_ip_address: Option<String>,
    pub net_gateway: Option<String>,
    pub net_dns: Option<String>,
    pub fans: Option<Vec<FanReading>>,
    pub temps: Option<TemperatureReadings>,
    pub gpu: Option<GpuInfo>,
    pub logged_in_users: Option<Vec<LoggedInUserInfo>>,
    pub processes: Option<Vec<ProcessInfo>>,
    pub total_processes: Option<u32>,
    pub running_processes: Option<u32>,
    pub last_updated: OffsetDateTime,
}

impl Metadata {
    /// Create metadata from SystemMetrics
    pub fn from_system_metrics(m: &SystemMetrics) -> Self {
        Self {
            kernel_version: m.kernel_version.clone(),
            cpu_model: m.cpu_model.clone(),
            cpu_mhz: m.cpu_mhz,
            mem_total_bytes: m.mem_total_bytes,
            swap_total_bytes: m.swap_total_bytes,
            disk_total_bytes: m.disk_total_bytes,
            filesystems: m.filesystems.clone(),
            net_interface: m.net_interface.clone(),
            net_ip_address: m.net_ip_address.clone(),
            net_gateway: m.net_gateway.clone(),
            net_dns: m.net_dns.clone(),
            fans: m.fans.clone(),
            temps: Some(m.temps.clone()),
            gpu: Some(m.gpu.clone()),
            logged_in_users: m.logged_in_users.clone(),
            processes: None,
            total_processes: None,
            running_processes: None,
            last_updated: m.ts,
        }
    }

    /// Check if metadata fields have changed (ignoring timestamp)
    pub fn has_changed(&self, other: &Metadata) -> bool {
        self.kernel_version != other.kernel_version
            || self.cpu_model != other.cpu_model
            || self.cpu_mhz != other.cpu_mhz
            || self.mem_total_bytes != other.mem_total_bytes
            || self.swap_total_bytes != other.swap_total_bytes
            || self.disk_total_bytes != other.disk_total_bytes
            || self.filesystems != other.filesystems
            || self.net_interface != other.net_interface
            || self.net_ip_address != other.net_ip_address
            || self.net_gateway != other.net_gateway
            || self.net_dns != other.net_dns
            || self.fans != other.fans
    }
}

