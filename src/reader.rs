use anyhow::{Context, Result};
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

use crate::event::Event;
use crate::storage::{RecordHeader, MAGIC};

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
        let mut segments = Vec::new();

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
                            segments.push((id, entry.path()));
                        }
                    }
                }
            }
        }

        // Sort by segment ID
        segments.sort_by_key(|(id, _)| *id);

        let mut all_events = Vec::new();

        for (_id, path) in segments {
            let events = self.read_segment(&path)?;
            all_events.extend(events);
        }

        Ok(all_events)
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
                let ts = match event {
                    Event::SystemMetrics(m) => m.ts.unix_timestamp(),
                    Event::ProcessLifecycle(p) => p.ts.unix_timestamp(),
                    Event::ProcessSnapshot(p) => p.ts.unix_timestamp(),
                    Event::SecurityEvent(s) => s.ts.unix_timestamp(),
                    Event::Anomaly(a) => a.ts.unix_timestamp(),
                };

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
