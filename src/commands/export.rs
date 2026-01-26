use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{self, Write};

use crate::cli::ExportFormat;
use crate::event::Event;
use crate::reader::LogReader;

pub fn run_export(
    output: Option<String>,
    format: ExportFormat,
    compress: bool,
    event_type: Option<String>,
    start: Option<String>,
    end: Option<String>,
    data_dir: Option<String>,
) -> Result<()> {
    let data_dir = data_dir.unwrap_or_else(|| "./data".to_string());

    // Read events from ring buffer
    let reader = LogReader::new(&data_dir);

    let mut events = if start.is_some() || end.is_some() {
        // Parse time range
        let start_ts = start.as_ref().map(|s| parse_timestamp(s)).transpose()?;
        let end_ts = end.as_ref().map(|s| parse_timestamp(s)).transpose()?;
        reader.read_events_range(start_ts, end_ts)?
    } else {
        reader.read_all_events()?
    };

    // Filter by event type if specified
    if let Some(ref filter_type) = event_type {
        events.retain(|e| matches_event_type(e, filter_type));
    }

    eprintln!("Found {} events", events.len());

    // Create output writer
    let writer: Box<dyn Write> = if let Some(path) = output {
        if compress && !path.ends_with(".gz") {
            eprintln!("Warning: compress flag set but output doesn't end with .gz");
        }
        Box::new(File::create(&path).context("Failed to create output file")?)
    } else {
        if compress {
            eprintln!("Warning: compress flag ignored when writing to stdout");
        }
        Box::new(io::stdout())
    };

    // Wrap in gzip if needed
    let mut writer: Box<dyn Write> = if compress {
        Box::new(GzEncoder::new(writer, Compression::default()))
    } else {
        writer
    };

    // Export in requested format
    match format {
        ExportFormat::Json => export_json(&events, &mut writer)?,
        ExportFormat::Jsonl => export_jsonl(&events, &mut writer)?,
        ExportFormat::Csv => export_csv(&events, &mut writer)?,
    }

    // Flush and finish compression if needed
    writer.flush()?;
    drop(writer);

    eprintln!("Export complete");
    Ok(())
}

fn parse_timestamp(s: &str) -> Result<i64> {
    // Try parsing as Unix timestamp first
    if let Ok(ts) = s.parse::<i64>() {
        return Ok(ts);
    }

    // Try parsing as RFC3339
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;

    let dt = OffsetDateTime::parse(s, &Rfc3339)
        .context("Invalid timestamp format. Use Unix timestamp or RFC3339")?;
    Ok(dt.unix_timestamp())
}

fn matches_event_type(event: &Event, filter: &str) -> bool {
    let filter_lower = filter.to_lowercase();
    match event {
        Event::SystemMetrics(_) => filter_lower.contains("system") || filter_lower.contains("metrics"),
        Event::ProcessLifecycle(_) => filter_lower.contains("process") && filter_lower.contains("lifecycle"),
        Event::ProcessSnapshot(_) => filter_lower.contains("process") && filter_lower.contains("snapshot"),
        Event::SecurityEvent(_) => filter_lower.contains("security") || filter_lower.contains("sec"),
        Event::Anomaly(_) => filter_lower.contains("anomaly") || filter_lower.contains("alert"),
    }
}

fn export_json(events: &[Event], writer: &mut dyn Write) -> Result<()> {
    let json = serde_json::to_string_pretty(&events)
        .context("Failed to serialize events to JSON")?;
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

fn export_jsonl(events: &[Event], writer: &mut dyn Write) -> Result<()> {
    for event in events {
        let json = serde_json::to_string(&event)
            .context("Failed to serialize event to JSON")?;
        writer.write_all(json.as_bytes())?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}

fn export_csv(events: &[Event], writer: &mut dyn Write) -> Result<()> {
    // Write CSV header
    writeln!(writer, "timestamp,event_type,details")?;

    for event in events {
        let (ts, event_type, details) = match event {
            Event::SystemMetrics(m) => (
                m.ts.unix_timestamp(),
                "system_metrics",
                format!(
                    "CPU:{:.1}% Mem:{:.1}% Disk:{:.0}% Load:{:.2}",
                    m.cpu_usage_percent,
                    m.mem_usage_percent,
                    m.disk_usage_percent,
                    m.load_avg_1m
                ),
            ),
            Event::ProcessLifecycle(p) => (
                p.ts.unix_timestamp(),
                "process_lifecycle",
                format!("{:?}: {} (pid {})", p.kind, p.name, p.pid),
            ),
            Event::ProcessSnapshot(s) => (
                s.ts.unix_timestamp(),
                "process_snapshot",
                format!("{} processes", s.processes.len()),
            ),
            Event::SecurityEvent(s) => (
                s.ts.unix_timestamp(),
                "security",
                format!("{:?}: {}", s.kind, s.message),
            ),
            Event::Anomaly(a) => (
                a.ts.unix_timestamp(),
                "anomaly",
                format!("{:?} - {:?}: {}", a.severity, a.kind, a.message),
            ),
        };

        // Escape CSV fields
        let details_escaped = details.replace('"', "\"\"");
        writeln!(writer, "{},\"{}\",\"{}\"", ts, event_type, details_escaped)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        // Unix timestamp
        assert_eq!(parse_timestamp("1234567890").unwrap(), 1234567890);

        // RFC3339
        let result = parse_timestamp("2024-01-01T00:00:00Z");
        assert!(result.is_ok());
    }

    #[test]
    fn test_matches_event_type() {
        use crate::event::{GpuInfo, SystemMetrics, TemperatureReadings};
        use time::OffsetDateTime;

        let event = Event::SystemMetrics(SystemMetrics {
            ts: OffsetDateTime::now_utc(),
            kernel_version: "6.0.0-test on x86_64".to_string(),
            cpu_model: "Test CPU".to_string(),
            cpu_mhz: 3000,
            system_uptime_seconds: 0,
            cpu_usage_percent: 50.0,
            per_core_usage: vec![],
            mem_used_bytes: 0,
            mem_total_bytes: 0,
            swap_used_bytes: 0,
            swap_total_bytes: 0,
            load_avg_1m: 0.0,
            load_avg_5m: 0.0,
            load_avg_15m: 0.0,
            disk_read_bytes_per_sec: 0,
            disk_write_bytes_per_sec: 0,
            disk_used_bytes: 0,
            disk_total_bytes: 0,
            per_disk_metrics: vec![],
            filesystems: vec![],
            net_recv_bytes_per_sec: 0,
            net_send_bytes_per_sec: 0,
            tcp_connections: 0,
            tcp_time_wait: 0,
            context_switches_per_sec: 0,
            temps: TemperatureReadings {
                cpu_temp_celsius: None,
                per_core_temps: vec![],
                gpu_temp_celsius: None,
                motherboard_temp_celsius: None,
            },
            fans: vec![],
            gpu: GpuInfo::default(),
            logged_in_users: vec![],
        });

        assert!(matches_event_type(&event, "system"));
        assert!(matches_event_type(&event, "metrics"));
        assert!(!matches_event_type(&event, "security"));
    }
}
