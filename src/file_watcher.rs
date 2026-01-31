use anyhow::Result;
use crossbeam_channel::Sender;
use inotify::{Inotify, WatchMask};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use time::OffsetDateTime;

use crate::event::{Event, FileSystemEvent, FileSystemEventKind, SecurityEvent, SecurityEventKind};
use crate::collector::is_sensitive_file_path;

/// Spawn a file watcher in a background thread
pub fn spawn_file_watcher(watch_dirs: Vec<String>, event_sender: Sender<Event>) -> Result<()> {
    thread::spawn(move || {
        if let Err(e) = run_file_watcher(watch_dirs, event_sender) {
            eprintln!("File watcher error: {}", e);
        }
    });

    Ok(())
}

fn run_file_watcher(watch_dirs: Vec<String>, event_sender: Sender<Event>) -> Result<()> {
    let mut watcher = FileWatcher::new(event_sender)?;

    // Add all configured directories
    for dir in &watch_dirs {
        if let Err(e) = watcher.watch_directory(dir) {
            eprintln!("Failed to watch directory {}: {}", dir, e);
        }
    }

    if watcher.watch_descriptors.is_empty() {
        eprintln!("Warning: No directories being watched");
        return Ok(());
    }

    println!("File watcher started, monitoring {} directories", watcher.watch_descriptors.len());

    // Main loop: process events every 100ms
    loop {
        match watcher.process_events() {
            Ok(count) => {
                if count > 0 {
                    // Only log if we actually processed events
                    // (reduces console spam)
                }
            }
            Err(e) => {
                eprintln!("Error processing file events: {}", e);
            }
        }

        // Small sleep to avoid busy-waiting
        thread::sleep(Duration::from_millis(100));
    }
}

pub struct FileWatcher {
    inotify: Inotify,
    watch_descriptors: HashMap<i32, PathBuf>,
    event_sender: Sender<Event>,
}

impl FileWatcher {
    pub fn new(event_sender: Sender<Event>) -> Result<Self> {
        let inotify = Inotify::init()?;

        Ok(FileWatcher {
            inotify,
            watch_descriptors: HashMap::new(),
            event_sender,
        })
    }

    /// Add a directory to watch (non-recursive)
    pub fn watch_directory(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        let mask = WatchMask::CREATE
            | WatchMask::MODIFY
            | WatchMask::DELETE
            | WatchMask::MOVED_FROM
            | WatchMask::MOVED_TO;

        let wd = self.inotify.watches().add(path, mask)?;
        self.watch_descriptors.insert(wd.get_watch_descriptor_id(), path.to_path_buf());
        Ok(())
    }

    /// Process file system events (non-blocking)
    pub fn process_events(&mut self) -> Result<usize> {
        let mut buffer = [0u8; 4096];
        let mut event_count = 0;

        // Read events (non-blocking)
        let events = self.inotify.read_events(&mut buffer)?;

        let mut pending_moves: HashMap<u32, (PathBuf, OffsetDateTime)> = HashMap::new();

        for event in events {
            let wd_id = event.wd.get_watch_descriptor_id();
            let base_path = self.watch_descriptors.get(&wd_id).cloned()
                .unwrap_or_else(|| PathBuf::from("<unknown>"));

            let full_path = if let Some(name) = event.name {
                base_path.join(name)
            } else {
                base_path
            };

            let path_str = full_path.to_string_lossy().to_string();
            let ts = OffsetDateTime::now_utc();

            // Get file size if possible
            let size = std::fs::metadata(&full_path).ok().map(|m| m.len());

            if event.mask.contains(inotify::EventMask::CREATE) {
                let fs_event = FileSystemEvent {
                    ts,
                    kind: FileSystemEventKind::Created,
                    path: path_str.clone(),
                    size,
                };
                let _ = self.event_sender.send(Event::FileSystemEvent(fs_event));
                event_count += 1;

                // Check for sensitive file creation
                if is_sensitive_file_path(&path_str) {
                    let sec_event = SecurityEvent {
                        ts,
                        kind: SecurityEventKind::SensitiveFileAccessed,
                        user: "unknown".to_string(),
                        source_ip: None,
                        message: format!("Sensitive file created: {}", path_str),
                    };
                    let _ = self.event_sender.send(Event::SecurityEvent(sec_event));
                }
            }

            if event.mask.contains(inotify::EventMask::MODIFY) {
                let fs_event = FileSystemEvent {
                    ts,
                    kind: FileSystemEventKind::Modified,
                    path: path_str.clone(),
                    size,
                };
                let _ = self.event_sender.send(Event::FileSystemEvent(fs_event));
                event_count += 1;

                // Check for sensitive file modification
                if is_sensitive_file_path(&path_str) {
                    let sec_event = SecurityEvent {
                        ts,
                        kind: SecurityEventKind::SensitiveFileAccessed,
                        user: "unknown".to_string(),
                        source_ip: None,
                        message: format!("Sensitive file modified: {}", path_str),
                    };
                    let _ = self.event_sender.send(Event::SecurityEvent(sec_event));
                }
            }

            if event.mask.contains(inotify::EventMask::DELETE) {
                let fs_event = FileSystemEvent {
                    ts,
                    kind: FileSystemEventKind::Deleted,
                    path: path_str.clone(),
                    size: None,
                };
                let _ = self.event_sender.send(Event::FileSystemEvent(fs_event));
                event_count += 1;
            }

            // Handle renames (MOVED_FROM + MOVED_TO with same cookie)
            if event.mask.contains(inotify::EventMask::MOVED_FROM) {
                let cookie = event.cookie;
                pending_moves.insert(cookie, (full_path.clone(), ts));
            }

            if event.mask.contains(inotify::EventMask::MOVED_TO) {
                let cookie = event.cookie;
                if let Some((from_path, _)) = pending_moves.remove(&cookie) {
                    let fs_event = FileSystemEvent {
                        ts,
                        kind: FileSystemEventKind::Renamed {
                            from: from_path.to_string_lossy().to_string(),
                            to: path_str.clone(),
                        },
                        path: path_str.clone(),
                        size,
                    };
                    let _ = self.event_sender.send(Event::FileSystemEvent(fs_event));
                    event_count += 1;
                }
            }
        }

        // Handle orphaned MOVED_FROM events (file moved out of watched directory)
        for (from_path, ts) in pending_moves.values() {
            let fs_event = FileSystemEvent {
                ts: *ts,
                kind: FileSystemEventKind::Deleted,
                path: from_path.to_string_lossy().to_string(),
                size: None,
            };
            let _ = self.event_sender.send(Event::FileSystemEvent(fs_event));
            event_count += 1;
        }

        Ok(event_count)
    }
}
