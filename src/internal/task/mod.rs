use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// serves the Executor struct
pub mod executor;
/// serves the SimpleExecutor struct (doesn't sleep when waiting for tasks)
pub mod simple_executor;

/// TaskId struct
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(u64);

impl TaskId {
    fn new() -> TaskId {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Task struct
pub struct Task {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    /// Create a new task
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task {
            id: TaskId::new(),
            future: Box::pin(future),
        }
    }

    /// Poll the task to see if it has completed
    pub fn poll(&mut self, cx: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(cx)
    }
}
