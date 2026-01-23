use anyhow::{Context, Result};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use crate::event::Event;
use crate::storage::{BlockIndex, EventTypeBloom, RecordHeader, SegmentIndex, BLOCK_SIZE, MAGIC};

/// Builds an in-memory index of all segments
pub struct IndexBuilder {
    dir: PathBuf,
}

impl IndexBuilder {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
        }
    }

    /// Scan all segments and build indexes
    pub fn build_index(&self) -> Result<Vec<SegmentIndex>> {
        eprintln!("IndexBuilder: Scanning directory: {:?}", self.dir);
        let mut segment_files = Vec::new();

        // Find all segment files
        if let Ok(entries) = fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("segment_") && name_str.ends_with(".dat") {
                    if let Some(id_str) = name_str
                        .strip_prefix("segment_")
                        .and_then(|s| s.strip_suffix(".dat"))
                    {
                        if let Ok(id) = id_str.parse::<u64>() {
                            segment_files.push((id, entry.path()));
                        }
                    }
                }
            }
        }

        // Sort by segment ID
        segment_files.sort_by_key(|(id, _)| *id);

        eprintln!("IndexBuilder: Found {} segment files", segment_files.len());

        let mut indexes = Vec::new();
        for (segment_id, path) in segment_files {
            eprintln!("IndexBuilder: Processing segment {} at {:?}", segment_id, path);
            match self.build_segment_index(segment_id, &path) {
                Ok(index) => {
                    eprintln!("  - First timestamp: {} ({:?})", index.first_timestamp_ns, index.first_timestamp_ns);
                    eprintln!("  - Last timestamp: {} ({:?})", index.last_timestamp_ns, index.last_timestamp_ns);
                    eprintln!("  - Blocks: {}, File size: {} bytes", index.blocks.len(), index.file_size);
                    indexes.push(index);
                }
                Err(e) => {
                    eprintln!("  - ERROR: Failed to index segment: {}", e);
                }
            }
        }

        eprintln!("IndexBuilder: Successfully indexed {} segments", indexes.len());
        Ok(indexes)
    }

    /// Build index for a single segment
    fn build_segment_index(&self, segment_id: u64, path: &Path) -> Result<SegmentIndex> {
        let mut file = File::open(path).context("Failed to open segment")?;
        let file_size = file.metadata()?.len();

        // Read and verify magic number
        let mut magic_bytes = [0u8; 4];
        file.read_exact(&mut magic_bytes)?;
        let magic = u32::from_le_bytes(magic_bytes);

        if magic != MAGIC {
            anyhow::bail!("Invalid magic number in segment");
        }

        let mut blocks = Vec::new();
        let mut event_type_bloom = EventTypeBloom::new();
        let mut first_timestamp_ns = None;
        let mut last_timestamp_ns = 0i128;
        let mut current_offset = 4u64; // After magic number
        let mut block_start_offset = current_offset;
        let mut block_event_count = 0u32;
        let mut block_first_timestamp = None;

        loop {
            // Record current position
            let record_offset = current_offset;

            // Try to read header
            let header = match read_record_header(&mut file) {
                Ok(h) => h,
                Err(_) => break, // End of file
            };

            let header_size = bincode::serialized_size(&header)? as u64;

            // Update timestamps
            if first_timestamp_ns.is_none() {
                first_timestamp_ns = Some(header.timestamp_unix_ns);
            }
            last_timestamp_ns = header.timestamp_unix_ns;

            // Read payload to get event type
            let mut payload = vec![0u8; header.payload_len as usize];
            file.read_exact(&mut payload)?;

            if let Ok(event) = bincode::deserialize::<Event>(&payload) {
                let event_type = event_type_id(&event);
                event_type_bloom.insert(event_type);
            }

            block_event_count += 1;
            if block_first_timestamp.is_none() {
                block_first_timestamp = Some(header.timestamp_unix_ns);
            }

            // Update current offset
            current_offset += header_size + header.payload_len as u64;

            // Create block checkpoint every BLOCK_SIZE bytes
            if current_offset - block_start_offset >= BLOCK_SIZE {
                if let Some(ts) = block_first_timestamp {
                    blocks.push(BlockIndex {
                        file_offset: block_start_offset,
                        timestamp_ns: ts,
                        event_count: block_event_count,
                    });
                }

                block_start_offset = record_offset;
                block_event_count = 0;
                block_first_timestamp = None;
            }
        }

        // Add final block if it has events
        if block_event_count > 0 {
            if let Some(ts) = block_first_timestamp {
                blocks.push(BlockIndex {
                    file_offset: block_start_offset,
                    timestamp_ns: ts,
                    event_count: block_event_count,
                });
            }
        }

        Ok(SegmentIndex {
            segment_id,
            file_path: path.to_path_buf(),
            first_timestamp_ns: first_timestamp_ns.unwrap_or(0),
            last_timestamp_ns,
            file_size,
            blocks,
            event_type_bloom,
        })
    }
}

fn read_record_header(file: &mut File) -> Result<RecordHeader> {
    let header: RecordHeader = bincode::deserialize_from(file)
        .context("Failed to deserialize header")?;
    Ok(header)
}

/// Map event to a type ID for bloom filter
fn event_type_id(event: &Event) -> u8 {
    match event {
        Event::SystemMetrics(_) => 0,
        Event::ProcessLifecycle(_) => 1,
        Event::ProcessSnapshot(_) => 2,
        Event::SecurityEvent(_) => 3,
        Event::Anomaly(_) => 4,
    }
}

/// Query helper: find segments that might contain events in time range
pub fn find_relevant_segments(
    indexes: &[SegmentIndex],
    start_ns: Option<i128>,
    end_ns: Option<i128>,
) -> Vec<&SegmentIndex> {
    indexes
        .iter()
        .filter(|idx| {
            let after_start = start_ns.map_or(true, |s| idx.last_timestamp_ns >= s);
            let before_end = end_ns.map_or(true, |e| idx.first_timestamp_ns <= e);
            after_start && before_end
        })
        .collect()
}

/// Query helper: find the best block to start reading from within a segment
pub fn find_start_block(segment: &SegmentIndex, start_ns: i128) -> usize {
    // Binary search for the block containing or just before start_ns
    match segment.blocks.binary_search_by_key(&start_ns, |b| b.timestamp_ns) {
        Ok(idx) => idx,
        Err(idx) => idx.saturating_sub(1), // Start from previous block
    }
}
