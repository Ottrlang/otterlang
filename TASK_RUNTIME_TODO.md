# Otter Task Runtime: Implementation Roadmap

This document captures the staged rollout plan for introducing the task-based concurrency model inspired by the user proposal. The scope goes well beyond the existing rayon-backed task shim and therefore must be split into deliverables that keep the repository in a working (compilable + testable) state at all times.

## Current Baseline
- `src/runtime/stdlib/task.rs` provides the public FFI surface for spawning tasks, joining, sleeping, and channel operations.
- Runtime metrics in `src/runtime/stdlib/runtime.rs` track active tasks with simple atomic counters.
- There is no first-class task scheduler abstraction inside `src/runtime` today; OTTER programs call FFI shims (`task.spawn`, `task.channel`, etc.).

## Target Features (from requirements)
1. Lightweight tasks managed entirely by the Otter runtime.
2. Adaptive, lock-free M:N scheduler with worker-stealing queues.
3. Typed channels that integrate with the scheduler (blocking tasks yield instead of blocking OS threads).
4. Auto-scaling worker pool based on CPU load.
5. Task affinity to keep hot tasks on the same worker.
6. Suspend/resume semantics for blocking APIs (I/O, sleep, await).

## Multi-Phase Delivery Plan

### Phase 0 – Planning & bootstrap (this PR)
- [x] Document the scope and risks (this file).
- [ ] Add runtime scaffolding modules (`src/runtime/task/*`) behind `#[cfg(feature = "task-runtime")]` to avoid breaking the build.
- [ ] Provide no-op API shims that keep existing `sync` exports working.

### Phase 1 – Core task abstraction
- Define `TaskId`, `TaskState`, and `Task` structs that wrap `Pin<Box<dyn Future<Output=()> + Send>>`.
- Implement a minimal executor loop that can poll tasks on a single thread (no parallelism yet).
- Bridge `task.spawn` to the executor instead of relying on ad-hoc thread pools when the feature flag is enabled.

### Phase 2 – Scheduler & workers
- Introduce worker threads with work-stealing queues (`crossbeam-deque`).
- Add central `Scheduler` that owns workers and drives the executor event loop.
- Ensure graceful shutdown and panic propagation semantics.

### Phase 3 – Channels & parking integration
- Replace any remaining legacy helpers with the task-aware runtime in `task.rs`.
- Blocked receivers/senders register wakers so tasks suspend without parking OS threads.
- Provide select-style API (initially limited to two-way select).

### Phase 4 – Timers & otter:time integration
- Build timer wheel or binary heap for delayed wakeups.
- Integrate with `time.sleep`, `time.after`, `time.tick` so they interact with the task scheduler.

### Phase 5 – Auto-scaling & metrics
- Track worker utilization and resize thread pool dynamically.
- Update runtime metrics to expose task counts, worker states, and queue depths.
- Provide diagnostic hooks (e.g., `runtime.tasks()` dumping JSON state).

### Phase 6 – Language-level affordances
- Extend OtterLang syntax/stdlib with `task.spawn`, `task.await`, etc., once runtime is stable.
- Update codegen to emit proper FFI calls and ensure type checking for typed channels.

## Open Questions / Risks
- **Future-based vs. stackful coroutines:** A future-first design keeps us in stable Rust but complicates exposing synchronous-looking OtterLang APIs. Alternative would require userland stack switching or async rewriting.
- **Integration with existing JIT:** Need to audit how the JIT currently runs programs; executor must be initialized before user code executes.
- **FFI boundary safety:** All exports must remain `extern "C"`; wakers need to be handled carefully to avoid UB when invoked from foreign contexts.

## Next Steps
1. Create module skeletons (`task/mod.rs`, `task/task.rs`, `task/scheduler.rs`, `task/worker.rs`, `task/channel.rs`, `task/metrics.rs`).
2. Wire compilation toggles and ensure CI still passes.
3. Iterate per phase, merging when each set of tests is green.

---
This roadmap can evolve as we learn more during implementation. Each phase should land behind a feature gate until production-ready.
