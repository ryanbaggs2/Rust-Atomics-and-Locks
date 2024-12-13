mod channels;

use std::thread;
use channels::safer_oneshot;
use channels::compile_time_oneshot;

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

    thread::scope(|s| {
        let (sender, receiver) = compile_time_oneshot::channel();

        s.spawn(|| {
            sender.send("Hello World!");
            t.unpark()
        });

        while !receiver.is_ready() {
            thread::park();
        }
        
        assert_eq!(receiver.receive(), "Hello World!");
    });
}
