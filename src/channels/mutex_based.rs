use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

// Notes:
// Didn't have to use atomics, because all types that compose Channel are
// send and sync. The compiler implicitly understands that.
pub struct Channel<T> {
    queue: Mutex<VecDeque<T>>,
    item_ready: Condvar,
}

// Pros: This is very flexible, allowing any number of sending and receiving threads.
// Cons: Not optimal implementation:
// 1. Any send or receive operation will block all other send or receive operations
// 2. If VecDeque::push has to grow the capacity of VecDeque, all other threads have
// to wait for that thread to finish
// 3. The queue could grow without bounds
impl<T> Channel<T> {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            item_ready: Condvar::new(),
        }
    }

    // Locks the mutex to push a new message onto the back of the queue.
    // Notifies one waiting receiver, after unlocking the queue.
    pub fn send(&self, message: T) {
        self.queue.lock().unwrap().push_back(message);
        // For above, Guard out of scope here, so mutex is unlocked.
        self.item_ready.notify_one();
    }

    // Blocks current thread until mutex is acquired and locked, pops message from
    // front of queue, but will use condition variable to wait if no message
    // available yet.
    pub fn receive(&self) -> T {
        let mut b = self.queue.lock().unwrap();
        loop {
            if let Some(message) = b.pop_front() {
                return message
            }
            b = self.item_ready.wait(b).unwrap();
        }
    }
}