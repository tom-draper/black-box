mod event;
mod recorder;
mod storage;

use anyhow::Result;
use time::OffsetDateTime;

use event::{Event, MetricSample};
use recorder::Recorder;

fn main() -> Result<()> {
    let mut recorder = Recorder::open("./data")?;

    let metric = MetricSample {
        ts: OffsetDateTime::now_utc(),
        cpu_usage: 12.3,
        mem_used_bytes: 512 * 1024 * 1024,
    };

    recorder.append(&Event::Metric(metric))?;

    println!("event recorded");

    Ok(())
}

