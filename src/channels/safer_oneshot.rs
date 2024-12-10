use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

// Typical use case: sending only one message from one thread to another
pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    in_use: AtomicBool,
    ready: AtomicBool,
}

// Tell compiler our channel is safe to share between threads, as long as
// T is Send
unsafe impl<T> Sync for Channel<T> where T: Send {}

/// No need for atomic operations here, because an object can only be
/// dropped if it's fully owned by the thread dropping it with no
/// outstanding borrows. Getting the value mutably takes an exclusive
/// reference, meeting that requirement
impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}

impl<T> Channel<T> {
    // A new channel is empty, with message being uninitialized and ready set
    // to false
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            in_use: AtomicBool::new(false),
            ready: AtomicBool::new(false),
        }
    }

    /// Panics when trying to send more than one message
    pub fn send(&self, message: T) {
        if self.in_use.swap(true, Relaxed) {
            panic!("Can't send more than one message!");
        }
        // Safety: We've checked and reset the in_use flag with swap
        // Basically once we do that swap, we panic if it's called again
        // from anywhere, whether that be another thread or not, this ensures
        // that once this send starts another cannot occur, because only a
        // single swap can occur, as in_use flag is never reset to false,
        // we can use relaxed ordering above.
        unsafe { (*self.message.get()).write(message); }
        self.ready.store(true, Release);
    }

    // We're not going to make a blocking interface, it'll be up to the user to
    // use something like thread parking if they want to block.

    /// We can lower the memory ordering of load to Relaxed, since it's now
    /// only used for indicative purposes.
    pub fn is_ready(&self) -> bool {
        self.ready.load(Relaxed)
    }

    /// Panics if no message is available yet,
    /// or if message was already consumed
    /// Addresses issue with receive being called more
    /// than once
    ///
    /// Tip: Use 'is_ready' to check first
    ///
    /// Safety: Only call this once!
    /// To fix calling receive before a message is ready, we check if it's ready,
    /// no longer results in undefined behavior
    /// If using load, we can't ensure that a race condition isn't occurring
    /// Still unsafe, as user still responsible for not calling this more than
    /// once
    pub fn receive(&self) -> T {
        if !self.ready.swap(false, Acquire) {
            panic!("No message available!");
        }
        // Safety: We've just checked (and reset) the ready flag with swap call
        unsafe { (*self.message.get()).assume_init_read() }
    }
}