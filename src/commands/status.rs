use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::time::Duration;

use crate::cli::StatusFormat;

#[derive(Deserialize, serde::Serialize)]
struct HealthResponse {
    uptime_seconds: u64,
    event_count: usize,
    storage_bytes_used: u64,
    storage_bytes_max: u64,
    storage_percent: f32,
    timestamp: String,
}

pub fn run_status(
    url: String,
    username: Option<String>,
    password: Option<String>,
    format: StatusFormat,
) -> Result<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let health_url = format!("{}/health", url.trim_end_matches('/'));

    // Build request with optional auth
    let mut req = client.get(&health_url);
    if let (Some(u), Some(p)) = (&username, &password) {
        req = req.basic_auth(u, Some(p));
    }

    let response = req
        .send()
        .context("Failed to connect to black box server")?;

    if !response.status().is_success() {
        anyhow::bail!("Server returned status: {}", response.status());
    }

    let health: HealthResponse = response
        .json()
        .context("Failed to parse health response")?;

    match format {
        StatusFormat::Human => print_human_status(&health),
        StatusFormat::Json => print_json_status(&health)?,
    }

    Ok(())
}

fn print_human_status(health: &HealthResponse) {
    println!("Black Box Status");
    println!("================");
    println!();
    println!("Uptime:       {}", format_duration(health.uptime_seconds));
    println!("Events:       {}", health.event_count);
    println!("Storage:      {:.1}% ({} / {})",
        health.storage_percent,
        format_bytes(health.storage_bytes_used),
        format_bytes(health.storage_bytes_max)
    );
    println!("Last Update:  {}", health.timestamp);
    println!();

    // Status indicator
    if health.storage_percent > 95.0 {
        println!("⚠ WARNING: Storage nearly full");
    } else if health.storage_percent > 80.0 {
        println!("⚠ Storage usage high");
    } else {
        println!("✓ System healthy");
    }
}

fn print_json_status(health: &HealthResponse) -> Result<()> {
    let json = serde_json::to_string_pretty(health)?;
    println!("{}", json);
    Ok(())
}

fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}MB", bytes as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:.1}GB", bytes as f64 / 1024.0 / 1024.0 / 1024.0)
    }
}
