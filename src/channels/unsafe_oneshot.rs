use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Release};

// Typical use case: sending only one message from one thread to another
// This is a minimal implementation without putting much thought into the
// interface
// Pros: It is usable, and if used right does exactly what it needs to do
// Cons: Many ways to misuse it
// 1. Calling send more than once, could cause a data race
// 2. Calling receive more than once results in two copies of the message,
// even if T does not implement Copy and cannot be safely copied.
// 3. Lack of Drop implementation, MaybeUninit doesn't track if it's been
// initialized or dropped, if a message is sent, but never received, it will
// never be dropped.
pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

// Tell compiler our channel is safe to share between threads, as long as
// T is Send
unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    // A new channel is empty, with message being uninitialized and ready set
    // to false
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    // Safety: Only call this once!
    // We're leaving the call only once up to the caller of this code.
    // Dereference the pointer to the MaybeUninit<T> and call MaybeUninit::write
    // The atomic store releases the message to the receiver, initialization will be
    // finished from the perspective of the receiving thread, if it loads true in
    // acquire ordering
    pub unsafe fn send(&self, message: T) {
        (*self.message.get()).write(message);
        self.ready.store(true, Release);
    }

    // We're not going to make a blocking interface, it'll be up to the user to
    // use something like thread parking if they want to block.

    pub fn is_ready(&self) -> bool {
        self.ready.load(Acquire)
    }

    // Safety: Only call this once,
    // and only after is_ready() returns true!
    // Deref the pointer to the MaybeUninit<T> and
    // call MaybeUninit::assume_init_read on it
    // We unsafely assume that it's been initialized,
    // and that it isn't being used to produce multiple
    // copies of non-Copy objects.
    pub unsafe fn receive(&self) -> T {
        (*self.message.get()).assume_init_read()
    }
}