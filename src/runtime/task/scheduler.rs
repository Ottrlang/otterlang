use crossbeam_deque::{Injector, Steal, Stealer, Worker};
use crossbeam_utils::Backoff;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::metrics::TaskRuntimeMetrics;
use super::task::{JoinHandle, Task, TaskFn};

#[derive(Debug, Clone, Copy)]
pub struct SchedulerConfig {
    pub max_workers: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        let workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            max_workers: workers,
        }
    }
}

#[derive(Debug)]
struct SchedulerCore {
    injector: Injector<Task>,
    stealers: Arc<Vec<Stealer<Task>>>,
    metrics: Arc<TaskRuntimeMetrics>,
    shutdown: AtomicBool,
}

#[derive(Debug, Clone)]
pub struct TaskScheduler {
    core: Arc<SchedulerCore>,
}

impl TaskScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        let metrics = TaskRuntimeMetrics::new();
        let injector = Injector::new();
        let mut workers = Vec::with_capacity(config.max_workers);
        let mut stealer_store = Vec::with_capacity(config.max_workers);

        for _ in 0..config.max_workers {
            let worker = Worker::new_fifo();
            stealer_store.push(worker.stealer());
            workers.push(worker);
        }

        let stealers = Arc::new(stealer_store);

        let core = Arc::new(SchedulerCore {
            injector,
            stealers: Arc::clone(&stealers),
            metrics,
            shutdown: AtomicBool::new(false),
        });

        for (index, worker) in workers.into_iter().enumerate() {
            let core = Arc::clone(&core);
            let stealers = Arc::clone(&stealers);
            thread::Builder::new()
                .name(format!("otter-task-worker-{}", index))
                .spawn(move || worker_loop(core, stealers, worker, index))
                .expect("failed to spawn task worker");
        }

        Self { core }
    }

    pub fn metrics(&self) -> Arc<TaskRuntimeMetrics> {
        Arc::clone(&self.core.metrics)
    }

    pub fn spawn_fn<F>(&self, name: Option<String>, func: F) -> JoinHandle
    where
        F: FnOnce() + Send + 'static,
    {
        let task = Task::new(name, Box::new(func) as TaskFn);
        let join = JoinHandle::new(task.id(), task.join_state());
        self.core.metrics.record_spawn();
        self.core.injector.push(task);
        join
    }

    pub fn shutdown(&self) {
        if self
            .core
            .shutdown
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            for _ in 0..self.core.stealers.len() {
                self.core.injector.push(Task::new(None, Box::new(|| {})));
            }
        }
    }
}

fn worker_loop(
    core: Arc<SchedulerCore>,
    stealers: Arc<Vec<Stealer<Task>>>,
    local: Worker<Task>,
    index: usize,
) {
    let stealers: Vec<_> = stealers
        .iter()
        .enumerate()
        .filter_map(|(i, stealer)| {
            if i != index {
                Some(stealer.clone())
            } else {
                None
            }
        })
        .collect();
    let backoff = Backoff::new();

    loop {
        if core.shutdown.load(Ordering::SeqCst) {
            break;
        }

        if let Some(task) = local.pop() {
            backoff.reset();
            task.run();
            core.metrics.record_completion();
            continue;
        }

        match core.injector.steal_batch_and_pop(&local) {
            Steal::Success(task) => {
                backoff.reset();
                task.run();
                core.metrics.record_completion();
                continue;
            }
            Steal::Retry => {
                backoff.spin();
                continue;
            }
            Steal::Empty => {}
        }

        let mut stolen = None;
        for stealer in &stealers {
            match stealer.steal() {
                Steal::Success(task) => {
                    stolen = Some(task);
                    break;
                }
                Steal::Retry => {
                    stolen = None;
                    break;
                }
                Steal::Empty => continue,
            }
        }

        if let Some(task) = stolen {
            backoff.reset();
            task.run();
            core.metrics.record_completion();
            continue;
        }

        // Nothing to do; yield slightly.
        if backoff.is_completed() {
            thread::sleep(Duration::from_micros(100));
        } else {
            backoff.snooze();
        }
    }
}
