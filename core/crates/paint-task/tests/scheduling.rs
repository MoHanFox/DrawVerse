use paint_task::{
    CancellationToken, Priority, SendError, SerialWorker, TaskError, TaskPool, TaskTicket,
    WorkerState,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};
fn take<T>(ticket: &TaskTicket<T>) -> Result<T, TaskError> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(result) = ticket.try_take() {
            return result;
        }
        assert!(Instant::now() < deadline, "task timed out");
        thread::yield_now();
    }
}
#[test]
fn priorities_and_equal_priority_fifo_with_bounded_queue() {
    let pool = TaskPool::new(1, 4).unwrap();
    let (ready, wait) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let block = pool
        .submit(Priority::Normal, move |_| {
            ready.send(()).unwrap();
            gate.recv().unwrap();
            Ok(())
        })
        .unwrap();
    wait.recv_timeout(Duration::from_secs(2)).unwrap();
    let (trace, events) = mpsc::channel();
    let mut tickets = Vec::new();
    for (priority, id) in [
        (Priority::Background, 1),
        (Priority::Interactive, 2),
        (Priority::Normal, 3),
        (Priority::Interactive, 4),
    ] {
        let trace = trace.clone();
        tickets.push(
            pool.submit(priority, move |_| {
                trace.send(id).unwrap();
                Ok(id)
            })
            .unwrap(),
        );
    }
    assert!(matches!(
        pool.submit(Priority::Interactive, |_| Ok(())),
        Err(TaskError::QueueFull)
    ));
    release.send(()).unwrap();
    take(&block).unwrap();
    for ticket in &tickets {
        take(ticket).unwrap();
    }
    assert_eq!(events.try_iter().collect::<Vec<_>>(), vec![2, 4, 3, 1]);
    pool.shutdown();
    assert!(matches!(
        pool.submit(Priority::Normal, |_| Ok(())),
        Err(TaskError::Stopped)
    ));
}
#[test]
fn queued_cancel_releases_captured_snapshot_without_execution() {
    struct Probe(Arc<AtomicBool>);
    impl Drop for Probe {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }
    let pool = TaskPool::new(1, 2).unwrap();
    let (ready, wait) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let block = pool
        .submit(Priority::Normal, move |_| {
            ready.send(()).unwrap();
            gate.recv().unwrap();
            Ok(())
        })
        .unwrap();
    wait.recv_timeout(Duration::from_secs(2)).unwrap();
    let dropped = Arc::new(AtomicBool::new(false));
    let probe = Probe(dropped.clone());
    let pending = pool
        .submit(Priority::Normal, move |_| {
            drop(probe);
            panic!("cancelled closure ran")
        })
        .unwrap();
    pool.cancel(&pending);
    assert_eq!(take::<()>(&pending), Err(TaskError::Cancelled));
    assert!(dropped.load(Ordering::Acquire));
    release.send(()).unwrap();
    take(&block).unwrap();
}
#[test]
fn running_cancel_and_panic_do_not_kill_pool() {
    let pool = TaskPool::new(1, 2).unwrap();
    let (ready, wait) = mpsc::channel();
    let active = pool
        .submit(Priority::Normal, move |token| {
            ready.send(()).unwrap();
            while !token.is_cancelled() {
                thread::yield_now();
            }
            token.checkpoint()
        })
        .unwrap();
    wait.recv_timeout(Duration::from_secs(2)).unwrap();
    pool.cancel(&active);
    assert_eq!(take(&active), Err(TaskError::Cancelled));
    let panic = pool
        .submit(Priority::Normal, |_| -> Result<(), TaskError> {
            panic!("isolated job")
        })
        .unwrap();
    assert_eq!(take(&panic), Err(TaskError::Panicked));
    let next = pool.submit(Priority::Normal, |_| Ok(42)).unwrap();
    assert_eq!(take(&next), Ok(42));
}
#[test]
fn serial_fifo_and_cancel_slot_preserve_preceding_commands() {
    let (ready, wait) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let (trace, events) = mpsc::channel();
    let mut worker = SerialWorker::spawn(2, Duration::from_millis(1), move |command, _| {
        if let Some(value) = command {
            if value == 0 {
                ready.send(()).unwrap();
                gate.recv().unwrap();
            }
            trace.send(value).unwrap();
        }
    })
    .unwrap();
    let sender = worker.sender();
    sender.try_send(0).unwrap();
    wait.recv_timeout(Duration::from_secs(2)).unwrap();
    sender.try_send(1).unwrap();
    sender.try_send(2).unwrap();
    assert_eq!(sender.try_send(3), Err(SendError::Full));
    sender.send_cancel(4, |v| *v == 2).unwrap();
    release.send(()).unwrap();
    assert_eq!(
        (0..3)
            .map(|_| events.recv_timeout(Duration::from_secs(2)).unwrap())
            .collect::<Vec<_>>(),
        vec![0, 1, 4]
    );
    worker.shutdown();
    assert_eq!(sender.try_send(5), Err(SendError::Stopped));
}
#[test]
fn serial_panic_poisoning_and_shutdown_cancellation() {
    let mut worker = SerialWorker::spawn(1, Duration::from_millis(1), |command: Option<u8>, _| {
        if command.is_some() {
            panic!("state corrupted");
        }
    })
    .unwrap();
    let sender = worker.sender();
    sender.try_send(1).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while sender.state() == WorkerState::Running {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    assert_eq!(sender.state(), WorkerState::Panicked);
    assert_eq!(sender.try_send(2), Err(SendError::Panicked));
    worker.shutdown();
    let (ready, wait) = mpsc::channel();
    let mut worker = SerialWorker::spawn(
        1,
        Duration::from_millis(1),
        move |command: Option<u8>, token| {
            if command.is_some() {
                ready.send(()).unwrap();
                while !token.is_cancelled() {
                    thread::yield_now();
                }
            }
        },
    )
    .unwrap();
    worker.sender().try_send(1).unwrap();
    wait.recv_timeout(Duration::from_secs(2)).unwrap();
    worker.shutdown();
    let token = CancellationToken::default();
    token.cancel();
    assert_eq!(token.checkpoint(), Err(TaskError::Cancelled));
}
