use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use ring::{rand, signature};
use ring::signature::KeyPair;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::config::ProtectionConfig;
use crate::storage::{find_segment_files, parse_segment_id};

const DOMAIN_SEPARATOR: &[u8] = b"black-box.segment-manifest.v1\0";

pub struct SegmentSigner {
    key_pair: signature::Ed25519KeyPair,
    public_key: Vec<u8>,
}

pub struct SegmentVerifier {
    public_key: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct SegmentManifest {
    segment_id: u64,
    file_size: u64,
    signature: String,
    public_key: String,
}

#[derive(Clone, Serialize)]
pub struct ArchiveSegment {
    pub segment_id: u64,
    pub file_name: String,
    pub file_size: u64,
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
            .context("protection.signing_key must be a base64-encoded Ed25519 PKCS#8 private key")?;
        let key_pair = signature::Ed25519KeyPair::from_pkcs8(&key)
            .map_err(|_| anyhow::anyhow!("Invalid Ed25519 signing key"))?;
        let public_key = key_pair.public_key().as_ref().to_vec();
        if let Some(verification_key) = config.verification_key.as_deref() {
            if general_purpose::STANDARD.decode(verification_key)? != public_key {
                anyhow::bail!("verification_key does not match signing_key");
            }
        }

        Ok(Some(Self {
            key_pair,
            public_key,
        }))
    }

    pub fn seal_or_verify(&self, path: &Path) -> Result<()> {
        if manifest_path(path).exists() {
            SegmentVerifier {
                public_key: self.public_key.clone(),
            }
            .verify(path)
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
            signature: general_purpose::STANDARD.encode(self.key_pair.sign(&data).as_ref()),
            public_key: general_purpose::STANDARD.encode(&self.public_key),
        };
        let manifest_path = manifest_path(path);
        let temp_path = manifest_path.with_extension("sig.tmp");
        std::fs::write(&temp_path, serde_json::to_vec(&manifest)?)?;
        std::fs::rename(temp_path, manifest_path)?;
        Ok(())
    }

}

impl SegmentVerifier {
    pub fn from_config(config: &ProtectionConfig) -> Result<Option<Self>> {
        let public_key = match config.verification_key.as_deref() {
            Some(key) => general_purpose::STANDARD.decode(key)?,
            None if config.sign_events => SegmentSigner::from_config(config)?.unwrap().public_key,
            None => return Ok(None),
        };
        Ok(Some(Self { public_key }))
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
        let manifest_public_key = general_purpose::STANDARD.decode(&manifest.public_key)?;
        if manifest_public_key != self.public_key {
            anyhow::bail!("Manifest signing key does not match the configured verification key");
        }
        let signature = general_purpose::STANDARD.decode(&manifest.signature)?;
        signature::UnparsedPublicKey::new(&signature::ED25519, &self.public_key)
            .verify(&signed_data(path, segment_id)?, &signature)
            .map_err(|_| anyhow::anyhow!("Integrity verification failed for {}", path.display()))
    }

    /// Verifies all sealed segments. The newest segment may be active and is
    /// intentionally unsigned until rotation.
    pub fn verify_directory(&self, dir: impl AsRef<Path>) -> Result<usize> {
        Ok(self.verified_segments(dir)?.len())
    }

    pub fn verified_segments(&self, dir: impl AsRef<Path>) -> Result<Vec<ArchiveSegment>> {
        let segments = find_segment_files(dir.as_ref());
        let mut verified = Vec::new();
        for (index, (_, path)) in segments.iter().enumerate() {
            if index + 1 == segments.len() && !manifest_path(path).exists() {
                continue;
            }
            self.verify(path)?;
            verified.push(ArchiveSegment {
                segment_id: segment_id(path)?,
                file_name: path.file_name().unwrap().to_string_lossy().into_owned(),
                file_size: std::fs::metadata(path)?.len(),
            });
        }
        Ok(verified)
    }
}

pub fn generate_keypair() -> Result<(String, String)> {
    let rng = rand::SystemRandom::new();
    let private_key = signature::Ed25519KeyPair::generate_pkcs8(&rng)
        .map_err(|_| anyhow::anyhow!("Failed to generate Ed25519 key pair"))?;
    let key_pair = signature::Ed25519KeyPair::from_pkcs8(private_key.as_ref())
        .map_err(|_| anyhow::anyhow!("Failed to load generated Ed25519 key pair"))?;
    Ok((
        general_purpose::STANDARD.encode(private_key.as_ref()),
        general_purpose::STANDARD.encode(key_pair.public_key().as_ref()),
    ))
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
        let (signing_key, verification_key) = generate_keypair().unwrap();
        let config = ProtectionConfig {
            sign_events: true,
            signing_key: Some(signing_key),
            verification_key: Some(verification_key),
            ..ProtectionConfig::default()
        };
        let signer = SegmentSigner::from_config(&config).unwrap().unwrap();
        let verifier = SegmentVerifier::from_config(&config).unwrap().unwrap();

        signer.seal(&segment).unwrap();
        verifier.verify(&segment).unwrap();
        std::fs::write(&segment, b"modified").unwrap();
        assert!(verifier.verify(&segment).is_err());
    }
}
