use super::Task;
use alloc::collections::VecDeque;

/// A simple executor that doesn't sleep when waiting for tasks
pub struct SimpleExecutor {
    task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
    /// Create a new SimpleExecutor
    pub fn new() -> SimpleExecutor {
        SimpleExecutor {
            task_queue: VecDeque::new(),
        }
    }

    /// Spawn a new task
    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task);
    }
}

use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(core::ptr::null(), vtable)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}

impl SimpleExecutor {
    /// Run the executor
    pub fn run(&mut self) {
        while let Some(mut task) = self.task_queue.pop_front() {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);

            match task.poll(&mut context) {
                Poll::Ready(()) => {}
                Poll::Pending => self.task_queue.push_back(task),
            }
        }
    }
}
