use anyhow::{Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{ProtectionConfig, ProtectionMode};

pub struct ProtectionManager {
    mode: ProtectionMode,
    config: ProtectionConfig,
    protected_files: Vec<PathBuf>,
}

impl ProtectionManager {
    pub fn new(mode: ProtectionMode, config: ProtectionConfig) -> Self {
        Self {
            mode,
            config,
            protected_files: Vec::new(),
        }
    }

    pub fn mode(&self) -> ProtectionMode {
        self.mode
    }

    /// Apply protection to a log file based on the protection mode
    pub fn protect_file(&mut self, path: &Path) -> Result<()> {
        match self.mode {
            ProtectionMode::Default => {
                // No protection in default mode
                Ok(())
            }
            ProtectionMode::Protected | ProtectionMode::Hardened => {
                if self.config.append_only || self.mode == ProtectionMode::Hardened {
                    self.set_append_only(path)?;
                    self.protected_files.push(path.to_path_buf());
                }
                Ok(())
            }
        }
    }

    /// Set append-only attribute on a file using chattr
    fn set_append_only(&self, path: &Path) -> Result<()> {
        let output = Command::new("chattr")
            .args(["+a", path.to_str().unwrap()])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                println!("✓ Set append-only protection on: {}", path.display());
                Ok(())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("Warning: Failed to set append-only on {}: {}", path.display(), stderr);
                eprintln!("  Protection mode may not work correctly without root privileges");
                Ok(()) // Don't fail, just warn
            }
            Err(e) => {
                eprintln!("Warning: chattr command failed: {}", e);
                eprintln!("  Append-only protection not available (requires Linux + e2fsprogs)");
                Ok(()) // Don't fail, just warn
            }
        }
    }

    /// Remove append-only attribute (for cleanup)
    pub fn unprotect_file(&self, path: &Path) -> Result<()> {
        if self.config.append_only || self.mode == ProtectionMode::Hardened {
            let _ = Command::new("chattr")
                .args(["-a", path.to_str().unwrap()])
                .output();
        }
        Ok(())
    }

    /// Send event to remote syslog if configured
    pub fn send_to_remote(&self, event_json: &str) -> Result<()> {
        if let Some(ref syslog_config) = self.config.remote_syslog {
            if !syslog_config.enabled {
                return Ok(());
            }

            // Only send in Protected or Hardened mode
            if self.mode == ProtectionMode::Default {
                return Ok(());
            }

            // Simple TCP/UDP send (basic implementation)
            use std::net::TcpStream;
            use std::net::UdpSocket;
            use std::time::Duration;

            let addr = format!("{}:{}", syslog_config.host, syslog_config.port);

            match syslog_config.protocol.as_str() {
                "tcp" => {
                    if let Ok(mut stream) = TcpStream::connect_timeout(
                        &addr.parse().context("Invalid syslog address")?,
                        Duration::from_secs(2)
                    ) {
                        let _ = stream.write_all(event_json.as_bytes());
                        let _ = stream.write_all(b"\n");
                    }
                }
                "udp" | _ => {
                    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
                        let _ = socket.send_to(event_json.as_bytes(), &addr);
                    }
                }
            }
        }
        Ok(())
    }

    /// Print protection mode information
    pub fn print_info(&self) {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║            BLACK BOX PROTECTION MODE                       ║");
        println!("╚════════════════════════════════════════════════════════════╝");

        match self.mode {
            ProtectionMode::Default => {
                println!("\nMode: DEFAULT");
                println!("  • Normal operation");
                println!("  • Standard file permissions");
                println!("  • Easy to stop/modify");
                println!("  • Best for development and testing");
            }
            ProtectionMode::Protected => {
                println!("\nMode: PROTECTED");
                println!("  • Append-only log files (chattr +a)");
                if self.config.remote_syslog.as_ref().map(|c| c.enabled).unwrap_or(false) {
                    println!("  • Remote log streaming enabled");
                }
                if self.config.sign_events {
                    println!("  • Cryptographic event signing");
                }
                println!("  • Systemd auto-restart (use 'systemctl stop' to stop)");
                println!("  • Good for production use");
            }
            ProtectionMode::Hardened => {
                println!("\nMode: HARDENED (Maximum Tamper Resistance)");
                println!("  • Append-only log files (chattr +a)");
                println!("  • Aggressive process protection");
                if self.config.remote_syslog.as_ref().map(|c| c.enabled).unwrap_or(false) {
                    println!("  • Remote log streaming enabled");
                }
                if self.config.sign_events {
                    println!("  • Cryptographic event signing");
                }
                println!("  • Difficult to stop without proper authorization");
                println!("  • Best for forensic/compliance scenarios");
                println!("\n  ⚠️  To stop: Run with --force-stop flag");
            }
        }

        if self.mode != ProtectionMode::Default {
            println!("\n💡 Tips:");
            if self.config.append_only {
                println!("  • To clear logs: sudo chattr -a <logfile> && rm <logfile>");
            }
            if self.config.remote_syslog.is_none() {
                println!("  • Consider configuring remote_syslog in config.toml for off-server backup");
            }
        }

        println!();
    }

    /// Check if force-stop is requested
    pub fn should_allow_stop(args: &[String], mode: ProtectionMode) -> bool {
        let force_stop = args.iter().any(|arg| arg == "--force-stop");

        match mode {
            ProtectionMode::Default | ProtectionMode::Protected => true,
            ProtectionMode::Hardened => {
                if force_stop {
                    println!("\n⚠️  Force stop requested for HARDENED mode");
                    println!("   Disabling protections...\n");
                    true
                } else {
                    false
                }
            }
        }
    }
}

impl Drop for ProtectionManager {
    fn drop(&mut self) {
        // Clean up append-only attributes on exit (if we can)
        for path in &self.protected_files {
            let _ = self.unprotect_file(path);
        }
    }
}
