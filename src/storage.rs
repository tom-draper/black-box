use serde::{Serialize, Deserialize};

pub const MAGIC: u32 = 0xBB10_0001;

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordHeader {
    pub timestamp_unix_ns: i128,
    pub payload_len: u32,
}

