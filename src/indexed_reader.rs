use anyhow::{Context, Result};
use memmap2::Mmap;
use std::{
    fs::File,
    io::Cursor,
    path::Path,
};

use crate::event::Event;
use crate::index::{find_relevant_segments, find_start_block, IndexBuilder};
use crate::storage::{RecordHeader, SegmentIndex, MAGIC};

/// Efficient reader using memory-mapped I/O and block indexes
pub struct IndexedReader {
    indexes: Vec<SegmentIndex>,
}

impl IndexedReader {
    /// Create a new indexed reader and build indexes for all segments
    pub fn new(dir: impl AsRef<Path>) -> Result<Self> {
        let builder = IndexBuilder::new(&dir);
        let indexes = builder.build_index()?;

        Ok(Self { indexes })
    }

    /// Rebuild the index (call this periodically to pick up new segments)
    pub fn refresh(&mut self, dir: impl AsRef<Path>) -> Result<()> {
        let builder = IndexBuilder::new(&dir);
        self.indexes = builder.build_index()?;
        Ok(())
    }

    /// Read events in a time range efficiently using indexes
    pub fn read_time_range(
        &self,
        start_ns: Option<i128>,
        end_ns: Option<i128>,
    ) -> Result<Vec<Event>> {
        let relevant_segments = find_relevant_segments(&self.indexes, start_ns, end_ns);

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
        let start_offset = if start_block_idx < segment.blocks.len() {
            segment.blocks[start_block_idx].file_offset as usize
        } else {
            4 // Just after magic number
        };

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

    /// Get segment metadata (for debugging/UI)
    pub fn get_segments(&self) -> &[SegmentIndex] {
        &self.indexes
    }

    /// Get time range covered by all segments
    pub fn get_time_range(&self) -> Option<(i128, i128)> {
        if self.indexes.is_empty() {
            return None;
        }

        let first = self.indexes.first()?.first_timestamp_ns;
        let last = self.indexes.last()?.last_timestamp_ns;

        Some((first, last))
    }

    /// Get total number of events (estimated from block counts)
    pub fn estimate_event_count(&self) -> u64 {
        self.indexes
            .iter()
            .flat_map(|seg| seg.blocks.iter())
            .map(|block| block.event_count as u64)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_indexed_reader_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let reader = IndexedReader::new(temp_dir.path()).unwrap();
        assert_eq!(reader.get_segments().len(), 0);
        assert!(reader.get_time_range().is_none());
    }
}
