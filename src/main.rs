mod channels;

use std::thread;
use channels::safer_oneshot;

fn main() {
    let channel = safer_oneshot::Channel::new();
    let t = thread::current();
    thread::scope(|s| {
        s.spawn(|| {
            channel.send("Hello World!");
            t.unpark();
        });

        while !channel.is_ready() {
            thread::park();
        }

        assert_eq!(channel.receive(), "Hello World!");
    });
}
