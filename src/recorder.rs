use std::{
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use anyhow::Result;
use time::OffsetDateTime;

use crate::broadcast::SyncSender;
use crate::event::Event;
use crate::storage::{RecordHeader, MAGIC};

const SEGMENT_SIZE: u64 = 8 * 1024 * 1024; // 8MB
const FLUSH_INTERVAL_SECONDS: i64 = 30; // Flush every 30 seconds

pub struct Recorder {
    dir: PathBuf,
    current_segment: u64,
    oldest_segment: u64,
    max_segments: usize,
    file: File,
    offset: u64,
    broadcast_tx: Option<SyncSender>,
    last_flush: OffsetDateTime,
}

impl Recorder {
    pub fn open_with_config(
        dir: impl AsRef<Path>,
        max_segments: usize,
        broadcast_tx: Option<SyncSender>,
    ) -> Result<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;

        // Find existing segments to resume from
        let (current_segment, oldest_segment) = Self::find_segment_range(dir)?;

        let path = segment_path(dir, current_segment);

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)?;

        let mut offset = file.metadata()?.len();

        if offset == 0 {
            file.write_all(&MAGIC.to_le_bytes())?;
            file.flush()?;
            offset = 4;
        } else {
            file.seek(SeekFrom::Start(offset))?;
        }

        Ok(Self {
            dir: dir.to_path_buf(),
            current_segment,
            oldest_segment,
            max_segments,
            file,
            offset,
            broadcast_tx,
            last_flush: OffsetDateTime::now_utc(),
        })
    }

    fn find_segment_range(dir: &Path) -> Result<(u64, u64)> {
        let mut segments = Vec::new();

        if dir.exists() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let name = entry.file_name();
                let name = name.to_string_lossy();

                if let Some(id_str) = name.strip_prefix("segment_").and_then(|s| s.strip_suffix(".dat")) {
                    if let Ok(id) = id_str.parse::<u64>() {
                        segments.push(id);
                    }
                }
            }
        }

        if segments.is_empty() {
            Ok((0, 0))
        } else {
            segments.sort_unstable();
            Ok((*segments.last().unwrap(), *segments.first().unwrap()))
        }
    }

    pub fn append(&mut self, event: &Event) -> Result<()> {
        let payload = bincode::serialize(event)?;

        let header = RecordHeader {
            timestamp_unix_ns: OffsetDateTime::now_utc().unix_timestamp_nanos(),
            payload_len: payload.len() as u32,
        };

        let header_bytes = bincode::serialize(&header)?;
        let record_len = header_bytes.len() + payload.len();

        if self.offset + record_len as u64 > SEGMENT_SIZE {
            self.rotate_segment()?;
        }

        self.file.write_all(&header_bytes)?;
        self.file.write_all(&payload)?;

        self.offset += record_len as u64;

        // Periodic flush every 30 seconds to make recent data available for playback
        let now = OffsetDateTime::now_utc();
        if (now - self.last_flush).whole_seconds() >= FLUSH_INTERVAL_SECONDS {
            self.file.flush()?;
            self.last_flush = now;
        }

        // Broadcast event to WebSocket clients (non-blocking)
        if let Some(tx) = &self.broadcast_tx {
            let _ = tx.try_send(event.clone());
        }

        Ok(())
    }

    fn rotate_segment(&mut self) -> Result<()> {
        self.current_segment += 1;
        self.offset = 0;

        // Enforce ring buffer: delete oldest segment if we exceed max
        let segment_count = (self.current_segment - self.oldest_segment + 1) as usize;
        if segment_count > self.max_segments {
            let old_path = segment_path(&self.dir, self.oldest_segment);
            let _ = std::fs::remove_file(old_path); // Ignore errors if file doesn't exist
            self.oldest_segment += 1;
        }

        let path = segment_path(&self.dir, self.current_segment);
        self.file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)?;

        self.file.write_all(&MAGIC.to_le_bytes())?;
        self.file.flush()?;  // Ensure magic number is written to disk
        self.last_flush = OffsetDateTime::now_utc();
        self.offset += 4;

        Ok(())
    }
}

fn segment_path(dir: &Path, id: u64) -> PathBuf {
    dir.join(format!("segment_{:05}.dat", id))
}

