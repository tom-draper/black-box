use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "black-box")]
#[command(about = "Linux server black box - tamper-resistant event recorder", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Protection mode (default, protected, or hardened)
    #[arg(long, global = true)]
    pub protected: bool,

    /// Enable hardened protection mode
    #[arg(long, global = true)]
    pub hardened: bool,

    /// Disable web UI
    #[arg(long, global = true)]
    pub no_ui: bool,

    /// Disable web UI (alias)
    #[arg(long, global = true)]
    pub headless: bool,

    /// Override server port
    #[arg(long, global = true)]
    pub port: Option<u16>,

    /// Config file path
    #[arg(long, global = true, default_value = "./config.toml")]
    pub config: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the black box recorder (default if no command specified)
    Run {
        /// Force stop even in hardened mode
        #[arg(long)]
        force_stop: bool,
    },

    /// Export recorded events
    Export {
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "json")]
        format: ExportFormat,

        /// Compress output (gzip)
        #[arg(short, long)]
        compress: bool,

        /// Filter by event type
        #[arg(long)]
        event_type: Option<String>,

        /// Start time (RFC3339 or Unix timestamp)
        #[arg(long)]
        start: Option<String>,

        /// End time (RFC3339 or Unix timestamp)
        #[arg(long)]
        end: Option<String>,

        /// Data directory to read from
        #[arg(short, long)]
        data_dir: Option<String>,
    },

    /// Monitor black box health and auto-export on failure
    Monitor {
        /// Black box server URL
        #[arg(default_value = "http://localhost:8080")]
        url: String,

        /// Username for authentication
        #[arg(short, long)]
        username: Option<String>,

        /// Password for authentication
        #[arg(short, long)]
        password: Option<String>,

        /// Check interval in seconds
        #[arg(long, default_value = "60")]
        interval: u64,

        /// Export directory for automatic backups
        #[arg(long, default_value = "./backups")]
        export_dir: String,

        /// Auto-export on every check (not just on failure)
        #[arg(long)]
        continuous: bool,
    },

    /// Generate systemd service files
    Systemd {
        /// Command to generate
        #[command(subcommand)]
        command: SystemdCommands,
    },

    /// Check status of running black box
    Status {
        /// Black box server URL
        #[arg(default_value = "http://localhost:8080")]
        url: String,

        /// Username for authentication
        #[arg(short, long)]
        username: Option<String>,

        /// Password for authentication
        #[arg(short, long)]
        password: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "human")]
        format: StatusFormat,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
pub enum SystemdCommands {
    /// Generate systemd service unit
    Generate {
        /// Binary path
        #[arg(long, default_value = "/usr/local/bin/black-box")]
        binary_path: String,

        /// Working directory
        #[arg(long, default_value = "/var/lib/black-box")]
        working_dir: String,

        /// Data directory
        #[arg(long, default_value = "/var/lib/black-box/data")]
        data_dir: String,

        /// Enable auto-export on service stop
        #[arg(long)]
        export_on_stop: bool,

        /// Export directory for auto-export
        #[arg(long, default_value = "/var/backups/black-box")]
        export_dir: String,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Install systemd service
    Install {
        /// Binary path
        #[arg(long, default_value = "/usr/local/bin/black-box")]
        binary_path: String,

        /// Working directory
        #[arg(long, default_value = "/var/lib/black-box")]
        working_dir: String,

        /// Enable auto-export on service stop
        #[arg(long)]
        export_on_stop: bool,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Validate configuration file
    Validate,

    /// Generate default configuration
    Init {
        /// Force overwrite existing config
        #[arg(long)]
        force: bool,
    },

    /// Set up remote syslog streaming
    SetupRemote {
        /// Remote syslog server host
        #[arg(long)]
        host: String,

        /// Remote syslog server port
        #[arg(long, default_value = "514")]
        port: u16,

        /// Protocol (tcp or udp)
        #[arg(long, default_value = "tcp")]
        protocol: String,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ExportFormat {
    /// Pretty-printed JSON
    Json,
    /// Newline-delimited JSON (JSONL)
    Jsonl,
    /// CSV format
    Csv,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum StatusFormat {
    /// Human-readable output
    Human,
    /// JSON output
    Json,
}

impl Cli {
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
