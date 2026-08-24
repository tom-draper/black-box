use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use ring::hmac;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::ProtectionConfig;
use crate::storage::{find_segment_files, parse_segment_id};

const DOMAIN_SEPARATOR: &[u8] = b"black-box.segment-manifest.v1\0";

pub struct SegmentSigner {
    key: hmac::Key,
}

#[derive(Serialize, Deserialize)]
struct SegmentManifest {
    segment_id: u64,
    file_size: u64,
    mac: String,
}

impl SegmentSigner {
    pub fn from_config(config: &ProtectionConfig) -> Result<Option<Self>> {
        if !config.sign_events {
            return Ok(None);
        }

        let encoded = config
            .signing_key
            .as_deref()
            .context("protection.signing_key is required when sign_events is enabled")?;
        let key = general_purpose::STANDARD
            .decode(encoded)
            .context("protection.signing_key must be base64-encoded")?;
        if key.len() < 32 {
            anyhow::bail!("protection.signing_key must contain at least 32 random bytes");
        }

        Ok(Some(Self {
            key: hmac::Key::new(hmac::HMAC_SHA256, &key),
        }))
    }

    pub fn seal_or_verify(&self, path: &Path) -> Result<()> {
        if manifest_path(path).exists() {
            self.verify(path)
        } else {
            self.seal(path)
        }
    }

    pub fn seal(&self, path: &Path) -> Result<()> {
        let segment_id = segment_id(path)?;
        let data = signed_data(path, segment_id)?;
        let manifest = SegmentManifest {
            segment_id,
            file_size: std::fs::metadata(path)?.len(),
            mac: general_purpose::STANDARD.encode(hmac::sign(&self.key, &data).as_ref()),
        };
        let manifest_path = manifest_path(path);
        let temp_path = manifest_path.with_extension("sig.tmp");
        std::fs::write(&temp_path, serde_json::to_vec(&manifest)?)?;
        std::fs::rename(temp_path, manifest_path)?;
        Ok(())
    }

    pub fn verify(&self, path: &Path) -> Result<()> {
        let manifest_path = manifest_path(path);
        let manifest: SegmentManifest = serde_json::from_slice(
            &std::fs::read(&manifest_path)
                .with_context(|| format!("Missing manifest for {}", path.display()))?,
        )?;
        let segment_id = segment_id(path)?;
        if manifest.segment_id != segment_id || manifest.file_size != std::fs::metadata(path)?.len() {
            anyhow::bail!("Manifest metadata does not match {}", path.display());
        }
        let expected = general_purpose::STANDARD.decode(&manifest.mac)?;
        hmac::verify(&self.key, &signed_data(path, segment_id)?, &expected)
            .map_err(|_| anyhow::anyhow!("Integrity verification failed for {}", path.display()))
    }

    /// Verifies all sealed segments. The newest segment may be active and is
    /// intentionally unsigned until rotation.
    pub fn verify_directory(&self, dir: impl AsRef<Path>) -> Result<usize> {
        let segments = find_segment_files(dir.as_ref());
        let mut verified = 0;
        for (index, (_, path)) in segments.iter().enumerate() {
            if index + 1 == segments.len() && !manifest_path(path).exists() {
                continue;
            }
            self.verify(path)?;
            verified += 1;
        }
        Ok(verified)
    }
}

pub fn manifest_path(segment_path: &Path) -> PathBuf {
    segment_path.with_extension("sig")
}

fn segment_id(path: &Path) -> Result<u64> {
    path.file_name()
        .and_then(|name| parse_segment_id(&name.to_string_lossy()))
        .context("Invalid segment filename")
}

fn signed_data(path: &Path, segment_id: u64) -> Result<Vec<u8>> {
    let contents = std::fs::read(path)?;
    let mut data = Vec::with_capacity(DOMAIN_SEPARATOR.len() + 8 + contents.len());
    data.extend_from_slice(DOMAIN_SEPARATOR);
    data.extend_from_slice(&segment_id.to_le_bytes());
    data.extend_from_slice(&contents);
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_a_modified_segment() {
        let dir = tempfile::tempdir().unwrap();
        let segment = dir.path().join("segment_00000.dat");
        std::fs::write(&segment, b"recording").unwrap();
        let signer = SegmentSigner::from_config(&ProtectionConfig {
            sign_events: true,
            signing_key: Some(general_purpose::STANDARD.encode([7u8; 32])),
            ..ProtectionConfig::default()
        })
        .unwrap()
        .unwrap();

        signer.seal(&segment).unwrap();
        signer.verify(&segment).unwrap();
        std::fs::write(&segment, b"modified").unwrap();
        assert!(signer.verify(&segment).is_err());
    }
}
