use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};

pub const MAGIC: u32 = 0xBB10_0001;
pub const BLOCK_SIZE: u64 = 512 * 1024; // 512KB blocks for sparse index
pub const SEGMENT_SIZE: u64 = 8 * 1024 * 1024; // 8MB per segment
pub const FLUSH_INTERVAL_SECONDS: i64 = 30; // Flush to disk every 30 seconds

pub fn parse_segment_id(name: &str) -> Option<u64> {
    name.strip_prefix("segment_")
        .and_then(|s| s.strip_suffix(".dat"))
        .and_then(|s| s.parse().ok())
}

pub fn find_segment_files(dir: &Path) -> Vec<(u64, PathBuf)> {
    let mut segments = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if let Some(id) = parse_segment_id(&name.to_string_lossy()) {
                segments.push((id, entry.path()));
            }
        }
    }
    segments.sort_by_key(|(id, _)| *id);
    segments
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordHeader {
    pub timestamp_unix_ns: i128,
    pub payload_len: u32,
}

/// Block-level checkpoint within a segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockIndex {
    pub file_offset: u64,
    pub timestamp_ns: i128,
    pub event_count: u32,
}

/// Segment metadata with sparse block index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentIndex {
    pub segment_id: u64,
    pub file_path: PathBuf,
    pub first_timestamp_ns: i128,
    pub last_timestamp_ns: i128,
    pub file_size: u64,
    pub blocks: Vec<BlockIndex>,
}

