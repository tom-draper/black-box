use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Result;
use time::OffsetDateTime;

use crate::event::Event;
use crate::storage::{RecordHeader, MAGIC};

const SEGMENT_SIZE: u64 = 8 * 1024 * 1024; // 8MB

pub struct Recorder {
    dir: PathBuf,
    current_segment: u64,
    file: File,
    offset: u64,
}

impl Recorder {
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;

        let segment = 0;
        let path = segment_path(dir, segment);

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)?;

        let offset = file.metadata()?.len();

        if offset == 0 {
            file.write_all(&MAGIC.to_le_bytes())?;
        }

        Ok(Self {
            dir: dir.to_path_buf(),
            current_segment: segment,
            file,
            offset,
        })
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
        self.file.flush()?;

        self.offset += record_len as u64;

        Ok(())
    }

    fn rotate_segment(&mut self) -> Result<()> {
        self.current_segment += 1;
        self.offset = 0;

        let path = segment_path(&self.dir, self.current_segment);
        self.file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)?;

        self.file.write_all(&MAGIC.to_le_bytes())?;
        self.offset += 4;

        Ok(())
    }
}

fn segment_path(dir: &Path, id: u64) -> PathBuf {
    dir.join(format!("segment_{:05}.dat", id))
}

