use std::time::SystemTime;

pub enum MetricEvent {
    ClockTick(SystemTime),
    Work(u64, SystemTime, SystemTime),
    Network(SystemTime, SystemTime),
}
