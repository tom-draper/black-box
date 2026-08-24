use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::config::{ProtectionConfig, ProtectionMode};

#[derive(Clone)]
pub struct ProtectionManager {
    append_only: bool,
}

impl ProtectionManager {
    pub fn new(mode: ProtectionMode, config: ProtectionConfig) -> Self {
        Self {
            append_only: config.append_only || mode != ProtectionMode::Default,
        }
    }

    /// Whether recordings are kept as append-only evidence rather than a ring buffer.
    pub fn evidence_mode(&self) -> bool {
        self.append_only
    }

    /// Apply append-only protection. Failure is fatal: continuing would falsely
    /// advertise tamper resistance without actually providing it.
    pub fn protect_file(&self, path: &Path) -> Result<()> {
        if self.append_only {
            self.set_append_only(path)?;
        }
        Ok(())
    }

    /// Set append-only attribute on a file using chattr
    fn set_append_only(&self, path: &Path) -> Result<()> {
        let output = Command::new("chattr")
            .arg("+a")
            .arg(path)
            .output()
            .with_context(|| format!("Failed to run chattr for {}", path.display()))?;

        if output.status.success() {
            println!("✓ Set append-only protection on: {}", path.display());
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to set append-only protection on {}: {}", path.display(), stderr.trim());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_mode_enables_evidence_storage() {
        assert!(
            ProtectionManager::new(ProtectionMode::Protected, ProtectionConfig::default())
                .evidence_mode()
        );
        assert!(
            !ProtectionManager::new(ProtectionMode::Default, ProtectionConfig::default())
                .evidence_mode()
        );
    }
}
