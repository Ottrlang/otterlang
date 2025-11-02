use otterlang::runtime::task::{runtime, TaskChannel};

#[test]
fn task_spawn_and_join() {
    let scheduler = runtime().scheduler().clone();
    let handle = scheduler.spawn_fn(Some("test".into()), || {});
    handle.join();
}

#[test]
fn task_channel_send_recv() {
    let channel: TaskChannel<i64> = TaskChannel::new();
    channel.send(42);
    assert_eq!(channel.recv(), Some(42));
}
