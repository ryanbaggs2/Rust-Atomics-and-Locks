use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;

/// For this implementation we will have the user be responsible for the
/// shared channel object, they will create the Channel in a local variable,
/// and Sender/Receiver will borrow it. This avoids the overhead of allocating
/// memory.
///
/// Pros: No memory allocation with
pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

pub struct Sender<'a, T> {
    channel: &'a Channel<T>,
}

pub struct Receiver<'a, T> {
    channel: &'a Channel<T>,
}

