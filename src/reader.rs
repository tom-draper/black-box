use anyhow::{Context, Result};
use std::{
    fs::File,
    io::Read,
    path::Path,
};

use crate::event::Event;
use crate::storage::{find_segment_files, RecordHeader, MAGIC};

pub struct LogReader {
    dir: String,
}

impl LogReader {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_string_lossy().to_string(),
        }
    }

    pub fn read_all_events(&self) -> Result<Vec<Event>> {
        let segments = find_segment_files(self.dir.as_ref());
        let mut all_events = Vec::new();

        for (_id, path) in segments {
            // Skip segments that fail to deserialize (e.g., corrupted or old format)
            // This prevents one bad segment from breaking all playback
            match self.read_segment(&path) {
                Ok(events) => all_events.extend(events),
                Err(e) => {
                    eprintln!("Warning: Skipping segment {:?} due to error: {}", path, e);
                    continue;
                }
            }
        }

        Ok(all_events)
    }

    /// Read only the most recent segment file (for initial state loading)
    /// More robust as it avoids old segments with incompatible formats
    pub fn read_recent_segment(&self) -> Result<Vec<Event>> {
        let segments = find_segment_files(self.dir.as_ref());

        if segments.is_empty() {
            return Ok(Vec::new());
        }

        let (_id, path) = segments.last().unwrap();

        // Try to read the segment, but if it fails (e.g., old format), return empty
        match self.read_segment(path) {
            Ok(events) => Ok(events),
            Err(e) => {
                eprintln!("Warning: Failed to read recent segment: {}", e);
                Ok(Vec::new())
            }
        }
    }

    fn read_segment(&self, path: &Path) -> Result<Vec<Event>> {
        let mut file = File::open(path).context("Failed to open segment")?;

        // Read and verify magic number
        let mut magic_bytes = [0u8; 4];
        file.read_exact(&mut magic_bytes)?;
        let magic = u32::from_le_bytes(magic_bytes);

        if magic != MAGIC {
            anyhow::bail!("Invalid magic number in segment");
        }

        let mut events = Vec::new();

        loop {
            // Try to read header
            let header = match read_record_header(&mut file) {
                Ok(h) => h,
                Err(_) => break, // End of file
            };

            // Read payload
            let mut payload = vec![0u8; header.payload_len as usize];
            file.read_exact(&mut payload)?;

            // Deserialize event
            let event: Event = bincode::deserialize(&payload)
                .context("Failed to deserialize event")?;

            events.push(event);
        }

        Ok(events)
    }

    pub fn read_events_range(
        &self,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> Result<Vec<Event>> {
        let all_events = self.read_all_events()?;

        let filtered: Vec<Event> = all_events
            .into_iter()
            .filter(|event| {
                let ts = event.timestamp().unix_timestamp();

                let after_start = start_time.map_or(true, |s| ts >= s);
                let before_end = end_time.map_or(true, |e| ts <= e);

                after_start && before_end
            })
            .collect();

        Ok(filtered)
    }
}

fn read_record_header(file: &mut File) -> Result<RecordHeader> {
    // bincode will read exactly as many bytes as needed
    let header: RecordHeader = bincode::deserialize_from(file)
        .context("Failed to deserialize header")?;

    Ok(header)
}
