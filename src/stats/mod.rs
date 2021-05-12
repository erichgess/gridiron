use std::time::SystemTime;

pub enum MetricEvent {
    ClockTick(SystemTime),
    Work(SystemTime, SystemTime),
}
