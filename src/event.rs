use serde::{Serialize, Deserialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    Metric(MetricSample),
    Process(ProcessEvent),
    Anomaly(Anomaly),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricSample {
    pub ts: OffsetDateTime,
    pub cpu_usage: f32,
    pub mem_used_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessEvent {
    pub ts: OffsetDateTime,
    pub pid: u32,
    pub name: String,
    pub kind: ProcessEventKind,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ProcessEventKind {
    Started,
    Exited,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Anomaly {
    pub ts: OffsetDateTime,
    pub message: String,
}

