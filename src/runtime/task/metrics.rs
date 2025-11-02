use std::cmp::max;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct TaskRuntimeMetrics {
    spawned: AtomicU64,
    completed: AtomicU64,
    waiting: AtomicI64,
    channels: AtomicU64,
    channel_waiters: AtomicI64,
    channel_backlog: AtomicI64,
}

impl TaskRuntimeMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn record_spawn(&self) {
        self.spawned.fetch_add(1, Ordering::Relaxed);
        self.waiting.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_completion(&self) {
        self.completed.fetch_add(1, Ordering::Relaxed);
        self.waiting.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_waiting_delta(&self, delta: i64) {
        if delta != 0 {
            self.waiting.fetch_add(delta, Ordering::Relaxed);
        }
    }

    pub fn register_channel(&self) {
        self.channels.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_channel_waiters(&self, delta: i64) {
        if delta != 0 {
            self.channel_waiters.fetch_add(delta, Ordering::Relaxed);
        }
    }

    pub fn record_channel_backlog(&self, delta: i64) {
        if delta != 0 {
            self.channel_backlog.fetch_add(delta, Ordering::Relaxed);
        }
    }

    pub fn snapshot(&self) -> TaskMetricsSnapshot {
        TaskMetricsSnapshot {
            tasks_spawned: self.spawned.load(Ordering::Relaxed),
            tasks_completed: self.completed.load(Ordering::Relaxed),
            tasks_waiting: max(self.waiting.load(Ordering::Relaxed), 0) as u64,
            channels_registered: self.channels.load(Ordering::Relaxed),
            channel_waiters: max(self.channel_waiters.load(Ordering::Relaxed), 0) as u64,
            channel_backlog: max(self.channel_backlog.load(Ordering::Relaxed), 0) as u64,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TaskMetricsSnapshot {
    pub tasks_spawned: u64,
    pub tasks_completed: u64,
    pub tasks_waiting: u64,
    pub channels_registered: u64,
    pub channel_waiters: u64,
    pub channel_backlog: u64,
}
