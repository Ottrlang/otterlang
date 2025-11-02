use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use std::sync::Arc;

use super::metrics::TaskRuntimeMetrics;

#[derive(Debug)]
pub struct TaskChannel<T> {
    inner: Arc<ChannelInner<T>>,
}

#[derive(Debug)]
struct ChannelInner<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
    metrics: Option<Arc<TaskRuntimeMetrics>>,
}

impl<T> TaskChannel<T> {
    pub fn new() -> Self {
        Self::with_metrics(None)
    }

    pub fn with_metrics(metrics: Option<Arc<TaskRuntimeMetrics>>) -> Self {
        let (sender, receiver) = unbounded();
        if let Some(metrics) = &metrics {
            metrics.register_channel();
        }
        Self {
            inner: Arc::new(ChannelInner {
                sender,
                receiver,
                metrics,
            }),
        }
    }

    pub fn send(&self, value: T) {
        if let Some(metrics) = &self.inner.metrics {
            metrics.record_channel_backlog(1);
        }
        // Ignore send errors since receiver may have been dropped.
        let _ = self.inner.sender.send(value);
    }

    pub fn recv(&self) -> Option<T> {
        match self.inner.receiver.recv() {
            Ok(value) => {
                if let Some(metrics) = &self.inner.metrics {
                    metrics.record_channel_backlog(-1);
                }
                Some(value)
            }
            Err(_) => None,
        }
    }

    pub fn try_recv(&self) -> Option<T> {
        match self.inner.receiver.try_recv() {
            Ok(value) => {
                if let Some(metrics) = &self.inner.metrics {
                    metrics.record_channel_backlog(-1);
                }
                Some(value)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }

    pub fn clone_sender(&self) -> Sender<T> {
        self.inner.sender.clone()
    }

    pub fn clone_receiver(&self) -> Receiver<T> {
        self.inner.receiver.clone()
    }
}

impl<T> Clone for TaskChannel<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TaskMailBox<T> {
    channel: TaskChannel<T>,
}

impl<T> TaskMailBox<T> {
    pub fn new(channel: TaskChannel<T>) -> Self {
        Self { channel }
    }

    pub fn channel(&self) -> TaskChannel<T> {
        self.channel.clone()
    }
}
