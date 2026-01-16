use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Deserialize)]
struct HealthResponse {
    uptime_seconds: u64,
    event_count: usize,
    storage_bytes_used: u64,
    storage_bytes_max: u64,
    storage_percent: f32,
}

pub fn run_monitor(
    url: String,
    username: Option<String>,
    password: Option<String>,
    interval: u64,
    export_dir: String,
    continuous: bool,
) -> Result<()> {
    println!("Black Box Monitor");
    println!("Target: {}", url);
    println!("Check interval: {}s", interval);
    println!("Export directory: {}", export_dir);
    println!("Mode: {}", if continuous { "continuous" } else { "failure-only" });
    println!();

    // Create export directory if it doesn't exist
    fs::create_dir_all(&export_dir).context("Failed to create export directory")?;

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let health_url = format!("{}/health", url.trim_end_matches('/'));
    let api_url = format!("{}/api/events", url.trim_end_matches('/'));

    let mut last_event_count = 0;
    let mut consecutive_failures = 0;

    loop {
        let check_time = chrono::Utc::now();

        // Build request with optional auth
        let mut req = client.get(&health_url);
        if let (Some(u), Some(p)) = (&username, &password) {
            req = req.basic_auth(u, Some(p));
        }

        // Check health
        match req.send() {
            Ok(response) if response.status().is_success() => {
                match response.json::<HealthResponse>() {
                    Ok(health) => {
                        consecutive_failures = 0;

                        println!(
                            "[{}] OK - Uptime: {}s, Events: {}, Storage: {:.1}%",
                            check_time.format("%Y-%m-%d %H:%M:%S"),
                            health.uptime_seconds,
                            health.event_count,
                            health.storage_percent
                        );

                        // Check for event count decrease (potential data loss)
                        if health.event_count < last_event_count {
                            eprintln!(
                                "  WARNING: Event count decreased from {} to {} (possible data loss or rotation)",
                                last_event_count, health.event_count
                            );
                            perform_export(&client, &api_url, &export_dir, &username, &password, "event-count-decrease")?;
                        }

                        last_event_count = health.event_count;

                        // Export if in continuous mode
                        if continuous {
                            perform_export(&client, &api_url, &export_dir, &username, &password, "scheduled")?;
                        }
                    }
                    Err(e) => {
                        eprintln!("[{}] ERROR: Failed to parse health response: {}",
                            check_time.format("%Y-%m-%d %H:%M:%S"), e);
                        consecutive_failures += 1;
                    }
                }
            }
            Ok(response) => {
                eprintln!(
                    "[{}] ERROR: Server returned status {} - performing emergency export",
                    check_time.format("%Y-%m-%d %H:%M:%S"),
                    response.status()
                );
                consecutive_failures += 1;
                perform_export(&client, &api_url, &export_dir, &username, &password, "error")?;
            }
            Err(e) => {
                eprintln!(
                    "[{}] FAILURE: Cannot reach server: {} - performing emergency export",
                    check_time.format("%Y-%m-%d %H:%M:%S"),
                    e
                );
                consecutive_failures += 1;

                // Try to export via direct file access if on same machine
                if url.contains("localhost") || url.contains("127.0.0.1") {
                    eprintln!("  Attempting direct file access for local server...");
                    if let Err(e) = perform_direct_export(&export_dir) {
                        eprintln!("  Direct export failed: {}", e);
                    }
                }
            }
        }

        // Alert on prolonged failures
        if consecutive_failures >= 3 {
            eprintln!("\n!!! ALERT: {} consecutive health check failures !!!\n", consecutive_failures);
        }

        thread::sleep(Duration::from_secs(interval));
    }
}

fn perform_export(
    client: &Client,
    api_url: &str,
    export_dir: &str,
    username: &Option<String>,
    password: &Option<String>,
    reason: &str,
) -> Result<()> {
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("blackbox-export-{}-{}.json", reason, timestamp);
    let filepath = Path::new(export_dir).join(&filename);

    eprintln!("  Exporting to: {}", filepath.display());

    // Build request with optional auth
    let mut req = client.get(api_url);
    if let (Some(u), Some(p)) = (username, password) {
        req = req.basic_auth(u, Some(p));
    }

    let response = req.send().context("Failed to fetch events from API")?;

    if !response.status().is_success() {
        anyhow::bail!("API returned status {}", response.status());
    }

    let events: serde_json::Value = response.json().context("Failed to parse events JSON")?;

    let json_content = serde_json::to_string_pretty(&events)?;
    fs::write(&filepath, json_content).context("Failed to write export file")?;

    eprintln!("  Export complete: {}", filepath.display());

    // Clean up old exports (keep last 100)
    cleanup_old_exports(export_dir, 100)?;

    Ok(())
}

fn perform_direct_export(export_dir: &str) -> Result<()> {
    use crate::reader::LogReader;

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let filename = format!("blackbox-export-direct-{}.json", timestamp);
    let filepath = Path::new(export_dir).join(&filename);

    eprintln!("  Reading directly from ./data directory...");

    let reader = LogReader::new("./data");
    let events = reader.read_all_events()?;

    let json_content = serde_json::to_string_pretty(&events)?;
    fs::write(&filepath, json_content)?;

    eprintln!("  Direct export complete: {} events written", events.len());
    Ok(())
}

fn cleanup_old_exports(export_dir: &str, keep_count: usize) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(export_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("blackbox-export-"))
                .unwrap_or(false)
        })
        .collect();

    if entries.len() <= keep_count {
        return Ok(());
    }

    // Sort by modification time (oldest first)
    entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());

    let to_remove = entries.len() - keep_count;
    for entry in entries.iter().take(to_remove) {
        if let Err(e) = fs::remove_file(entry.path()) {
            eprintln!("  Warning: Failed to remove old export {}: {}", entry.path().display(), e);
        }
    }

    Ok(())
}
