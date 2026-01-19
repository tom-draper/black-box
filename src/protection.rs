use anyhow::Result;
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

    /// Print protection mode information
    pub fn print_info(&self) {
        // Print will be handled in main.rs - this is now a no-op
        // Kept for backwards compatibility

        println!();
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
