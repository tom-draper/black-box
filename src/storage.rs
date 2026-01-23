use serde::{Serialize, Deserialize};
use std::path::PathBuf;

pub const MAGIC: u32 = 0xBB10_0001;
pub const BLOCK_SIZE: u64 = 512 * 1024; // 512KB blocks for sparse index

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
    pub event_type_bloom: EventTypeBloom,
}

/// Simple bloom filter for event types (256 bits)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EventTypeBloom {
    bits: [u64; 4],
}

impl EventTypeBloom {
    pub fn new() -> Self {
        Self { bits: [0; 4] }
    }

    pub fn insert(&mut self, event_type: u8) {
        let hash1 = event_type as usize;
        let hash2 = (event_type.wrapping_mul(31)) as usize;
        let hash3 = (event_type.wrapping_mul(37)) as usize;

        self.set_bit(hash1 % 256);
        self.set_bit(hash2 % 256);
        self.set_bit(hash3 % 256);
    }

    pub fn might_contain(&self, event_type: u8) -> bool {
        let hash1 = event_type as usize;
        let hash2 = (event_type.wrapping_mul(31)) as usize;
        let hash3 = (event_type.wrapping_mul(37)) as usize;

        self.check_bit(hash1 % 256)
            && self.check_bit(hash2 % 256)
            && self.check_bit(hash3 % 256)
    }

    fn set_bit(&mut self, bit_index: usize) {
        let word = bit_index / 64;
        let bit = bit_index % 64;
        self.bits[word] |= 1u64 << bit;
    }

    fn check_bit(&self, bit_index: usize) -> bool {
        let word = bit_index / 64;
        let bit = bit_index % 64;
        (self.bits[word] & (1u64 << bit)) != 0
    }
}

