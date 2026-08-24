use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;
use time::OffsetDateTime;

use crate::integrity::{manifest_path, ArchiveSegment, SegmentVerifier};

#[derive(Serialize)]
struct ArchiveIndex {
    version: u8,
    created_at: String,
    source_directory: String,
    segments: Vec<ArchiveSegment>,
}

pub fn run_archive(source: &str, destination: &str, verifier: &SegmentVerifier) -> Result<()> {
    let source = Path::new(source);
    let destination = Path::new(destination);
    if destination.exists() {
        anyhow::bail!("Archive destination already exists: {}", destination.display());
    }

    let segments = verifier.verified_segments(source)?;
    std::fs::create_dir_all(destination)
        .with_context(|| format!("Failed to create archive directory {}", destination.display()))?;

    for segment in &segments {
        let source_segment = source.join(&segment.file_name);
        let source_manifest = manifest_path(&source_segment);
        std::fs::copy(&source_segment, destination.join(&segment.file_name))?;
        std::fs::copy(&source_manifest, destination.join(source_manifest.file_name().unwrap()))?;
    }

    let index = ArchiveIndex {
        version: 1,
        created_at: OffsetDateTime::now_utc().to_string(),
        source_directory: source.display().to_string(),
        segments,
    };
    std::fs::write(destination.join("archive.json"), serde_json::to_vec_pretty(&index)?)?;
    println!("Archived {} sealed segment(s) to {}", index.segments.len(), destination.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::ProtectionConfig,
        integrity::{generate_keypair, SegmentSigner},
    };

    #[test]
    fn archives_only_verified_sealed_segments() {
        let source = tempfile::tempdir().unwrap();
        let archive_parent = tempfile::tempdir().unwrap();
        let segment = source.path().join("segment_00000.dat");
        std::fs::write(&segment, b"sealed evidence").unwrap();
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

        let destination = archive_parent.path().join("archive");
        run_archive(
            source.path().to_str().unwrap(),
            destination.to_str().unwrap(),
            &verifier,
        )
        .unwrap();

        assert!(destination.join("segment_00000.dat").exists());
        assert!(destination.join("segment_00000.sig").exists());
        assert!(destination.join("archive.json").exists());
    }
}
