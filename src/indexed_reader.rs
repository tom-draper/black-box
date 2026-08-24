use anyhow::{Context, Result};
use memmap2::Mmap;
use std::{
    collections::HashMap,
    fs::File,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{Mutex, RwLock},
};

use crate::event::Event;
use crate::index::{find_relevant_segments, find_start_block, find_start_metrics_offset, IndexBuilder};
use crate::storage::{find_segment_files, RecordHeader, SegmentIndex, MAGIC};

/// Efficient reader using memory-mapped I/O and block indexes
pub struct IndexedReader {
    dir: PathBuf,
    indexes: RwLock<Vec<SegmentIndex>>,
    metadata_cache: Mutex<HashMap<(i64, u16), serde_json::Value>>,
}

impl IndexedReader {
    /// Create a new indexed reader and build indexes for all segments
    pub fn new(dir: impl AsRef<Path>) -> Result<Self> {
        let dir_path = dir.as_ref().to_path_buf();
        let builder = IndexBuilder::new(&dir_path);
        let indexes = builder.build_index()?;

        Ok(Self {
            dir: dir_path,
            indexes: RwLock::new(indexes),
            metadata_cache: Mutex::new(HashMap::new()),
        })
    }

    /// Refresh the index to pick up new segments
    pub fn refresh(&self) -> Result<()> {
        let builder = IndexBuilder::new(&self.dir);
        let new_indexes = builder.build_index()?;
        let mut indexes = self.indexes.write().unwrap();
        *indexes = new_indexes;
        self.metadata_cache.lock().unwrap().clear();
        Ok(())
    }

    /// Refresh only when the newest segment changed on disk. The recorder flushes
    /// periodically, so this avoids rescanning the active segment for every seek.
    pub fn refresh_if_changed(&self) -> Result<bool> {
        let Some((segment_id, path)) = find_segment_files(&self.dir).pop() else {
            return Ok(false);
        };
        let file_size = std::fs::metadata(&path)?.len();
        let indexes = self.indexes.read().unwrap();
        let unchanged = indexes
            .last()
            .is_some_and(|index| index.segment_id == segment_id && index.file_size == file_size);
        drop(indexes);

        if unchanged {
            return Ok(false);
        }

        self.refresh()?;
        Ok(true)
    }

    pub fn cached_metadata(&self, end_ns: i128, missing_fields: u16) -> Option<serde_json::Value> {
        self.metadata_cache
            .lock()
            .unwrap()
            .get(&(metadata_bucket(end_ns), missing_fields))
            .cloned()
    }

    pub fn cache_metadata(&self, end_ns: i128, missing_fields: u16, metadata: serde_json::Value) {
        let mut cache = self.metadata_cache.lock().unwrap();
        // Keep roughly one day of five-minute metadata snapshots.
        if cache.len() >= 288 {
            cache.clear();
        }
        cache.insert((metadata_bucket(end_ns), missing_fields), metadata);
    }

    /// Read events in a time range efficiently using indexes
    pub fn read_time_range(
        &self,
        start_ns: Option<i128>,
        end_ns: Option<i128>,
    ) -> Result<Vec<Event>> {
        let indexes = self.indexes.read().unwrap();
        let relevant_segments = find_relevant_segments(&indexes, start_ns, end_ns);

        let mut events = Vec::new();

        for segment in relevant_segments {
            let segment_events = self.read_segment_range(segment, start_ns, end_ns)?;
            events.extend(segment_events);
        }

        Ok(events)
    }

    /// Read a segment using mmap and block index for fast seeking
    fn read_segment_range(
        &self,
        segment: &SegmentIndex,
        start_ns: Option<i128>,
        end_ns: Option<i128>,
    ) -> Result<Vec<Event>> {
        let file = File::open(&segment.file_path)
            .context("Failed to open segment file")?;

        // Memory-map the file for zero-copy access
        let mmap = unsafe { Mmap::map(&file)? };

        // Verify magic number
        if mmap.len() < 4 {
            anyhow::bail!("Segment file too small");
        }
        let magic = u32::from_le_bytes([mmap[0], mmap[1], mmap[2], mmap[3]]);
        if magic != MAGIC {
            anyhow::bail!("Invalid magic number");
        }

        // Find the starting block using binary search
        let start_block_idx = if let Some(start) = start_ns {
            find_start_block(segment, start)
        } else {
            0
        };

        // Start reading from the beginning of the start block
        let block_offset = if start_block_idx < segment.blocks.len() {
            segment.blocks[start_block_idx].file_offset as usize
        } else {
            4 // Just after magic number
        };
        let start_offset = start_ns
            .and_then(|start| find_start_metrics_offset(segment, start))
            .map(|offset| offset as usize)
            .unwrap_or(block_offset);

        let mut events = Vec::new();
        let mut cursor = Cursor::new(&mmap[start_offset..]);

        loop {
            // Try to read header
            let header = match bincode::deserialize_from::<_, RecordHeader>(&mut cursor) {
                Ok(h) => h,
                Err(_) => break, // End of data
            };

            // Check if we've passed the end time
            if let Some(end) = end_ns {
                if header.timestamp_unix_ns > end {
                    break;
                }
            }

            // Read payload
            let current_pos = cursor.position() as usize;
            let payload_end = current_pos + header.payload_len as usize;

            if payload_end > cursor.get_ref().len() {
                break; // Not enough data
            }

            let payload = &cursor.get_ref()[current_pos..payload_end];
            cursor.set_position(payload_end as u64);

            // Deserialize event
            if let Ok(event) = bincode::deserialize::<Event>(payload) {
                // Filter by start time
                if let Some(start) = start_ns {
                    if header.timestamp_unix_ns < start {
                        continue;
                    }
                }

                events.push(event);
            }
        }

        Ok(events)
    }

    /// Get the number of indexed segments without cloning
    pub fn segment_count(&self) -> usize {
        self.indexes.read().unwrap().len()
    }

    /// Get time range covered by all segments
    pub fn get_time_range(&self) -> Option<(i128, i128)> {
        let indexes = self.indexes.read().unwrap();
        if indexes.is_empty() {
            return None;
        }

        let first = indexes.first()?.first_timestamp_ns;
        let last = indexes.last()?.last_timestamp_ns;

        Some((first, last))
    }

    /// Get total number of events (estimated from block counts)
    pub fn estimate_event_count(&self) -> u64 {
        let indexes = self.indexes.read().unwrap();
        indexes
            .iter()
            .flat_map(|seg| seg.blocks.iter())
            .map(|block| block.event_count as u64)
            .sum()
    }
}

fn metadata_bucket(end_ns: i128) -> i64 {
    (end_ns / (5 * 60 * 1_000_000_000i128)) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    use crate::storage::MAGIC;

    #[test]
    fn test_indexed_reader_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let reader = IndexedReader::new(temp_dir.path()).unwrap();
        assert_eq!(reader.segment_count(), 0);
        assert!(reader.get_time_range().is_none());
    }

    #[test]
    fn refreshes_only_after_the_active_segment_changes() {
        let temp_dir = TempDir::new().unwrap();
        let segment = temp_dir.path().join("segment_00000.dat");
        std::fs::write(&segment, MAGIC.to_le_bytes()).unwrap();
        let reader = IndexedReader::new(temp_dir.path()).unwrap();

        assert!(!reader.refresh_if_changed().unwrap());

        let mut file = std::fs::OpenOptions::new().append(true).open(&segment).unwrap();
        file.write_all(&[0]).unwrap();
        file.flush().unwrap();

        assert!(reader.refresh_if_changed().unwrap());
        assert!(!reader.refresh_if_changed().unwrap());
    }

    #[test]
    fn caches_metadata_by_five_minute_bucket() {
        let temp_dir = TempDir::new().unwrap();
        let reader = IndexedReader::new(temp_dir.path()).unwrap();
        reader.cache_metadata(600_000_000_000, 1, serde_json::json!({"kernel": "test"}));

        assert_eq!(reader.cached_metadata(899_000_000_000, 1), Some(serde_json::json!({"kernel": "test"})));
        assert_eq!(reader.cached_metadata(899_000_000_000, 2), None);
        assert_eq!(reader.cached_metadata(900_000_000_000, 1), None);
    }
}
