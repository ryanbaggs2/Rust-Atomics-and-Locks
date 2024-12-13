use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

/// Here we'll be taking an argument by value, which for non-Copy types
/// will consume the object, preventing reuse of the functions
/// The channel fn gives a Sender and a Receiver which we can pass around
/// to any thread, but we cannot make copies of
/// User can create a Channel by calling this fn
/// This catches send and/or receive being called twice at compile time!
///
/// Cons: You have to allocate memory, so it costs some performance with
/// the Arc based implementation
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    // Similar to channel::new fn, except it wraps Channel in an Arc,
    // and that arc and a clone of it are wrapped in Sender and Receiver
    let a = Arc::new(Channel {
        message: UnsafeCell::new(MaybeUninit::uninit()),
        ready: AtomicBool::new(false),
    });
    (Sender { channel: a.clone() }, Receiver {channel: a })
}

pub struct Sender<T> {
    channel: Arc<Channel<T>>,
}

pub struct Receiver<T> {
    channel: Arc<Channel<T>>,
}

// Inner implementation not relevant to user, so we keep private
// We don't need the in_use atomic boolean like in the safer_oneshot
// implementation, as send is now statically guaranteed to only be
// called once through the type system.
struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

// Now that we've specified Channel is Sync, Sender and Receiver are also Sync.
// As long as type T is Send, Sync is implemented for Channel of type T.
// I think this means that type T can be something that isn't Sync, but the way that
// the channel works, and that it's specified here, as long as it's wrapped with Channel,
// it's Sync. Also, it is sync due to our implementation of Channel.
unsafe impl<T> Sync for Channel<T> where T: Send {}

// Arc<Channel<T>> will decrement reference counter of allocation when
// either Sender<T> or Receiver<T> is dropped, on the second drop, counter
// reaches zero and Channel<T> is dropped
impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        // if ready, a message is waiting, and drop is called, we drop the message
        // Again, this thread will own the single value of ready and message as
        // they are mutable
        if *self.ready.get_mut() {
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}

impl<T> Sender<T> {
    // Once this is called the Sender object is consumed, and we can no
    // longer call this fn
    // send can no longer panic, as it's precondition (only being called
    // once) is now statically guaranteed
    pub fn send(self, message: T) {
        unsafe { (*self.channel.message.get()).write(message) };
        self.channel.ready.store(true, Release);
    }
}

impl<T> Receiver<T> {
    // This does not consume the Receiver, as it takes the argument
    // by reference
    pub fn is_ready(&self) -> bool {
        self.channel.ready.load(Relaxed)
    }

    // Once this is called, the Receiver object is consumed, as we
    // take the argument by value, rather than reference
    // This can still panic, because the user might still call it
    // before is_ready returns true
    pub fn receive(self) -> T {
        // swap used so drop knows whether there is an unread message
        // that needs to be dropped
        if !self.channel.ready.swap(false, Acquire) {
            panic!("No message available!");
        }
        unsafe { (*self.channel.message.get()).assume_init_read() }
    }
}