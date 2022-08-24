//! Soundness tests that should be run through Miri
use rand::random;
use ssq::SingleSlotQueue;
use std::thread;

#[test]
fn enqueue() {
    let mut queue = SingleSlotQueue::<u32>::new();
    let (mut cons, mut prod) = queue.split();

    thread::scope(|scope| {
        let feed = scope.spawn(|| {
            for _ in 0..500 {
                prod.enqueue_overwrite(random());
            }
        });

        let consume = scope.spawn(|| {
            for _ in 0..500 {
                let _ = cons.dequeue();
            }
        });

        feed.join().unwrap();
        consume.join().unwrap();
    });
}

/// Soundness test that should be run through Miri
#[test]
fn enqueue_overwrite() {
    let mut queue = SingleSlotQueue::<u32>::new();
    let (mut cons, mut prod) = queue.split();

    thread::scope(|scope| {
        let feed = scope.spawn(|| {
            for _ in 0..500 {
                prod.enqueue_overwrite(random());
            }
        });

        let consume = scope.spawn(|| {
            for _ in 0..500 {
                let _ = cons.dequeue();
            }
        });

        feed.join().unwrap();
        consume.join().unwrap();
    });
}

#[test]
fn peek() {
    let mut queue = SingleSlotQueue::<u32>::new();
    let (mut cons, mut prod) = queue.split();

    // Enqueue *something* to seed the queue
    prod.enqueue(0);
    assert!(cons.peek() == Some(0));
    assert!(cons.peek() == Some(0));

    thread::scope(|scope| {
        let feed = scope.spawn(|| {
            for _ in 0..500 {
                prod.enqueue_overwrite(random());
            }
        });

        let consume = scope.spawn(|| {
            for _ in 0..500 {
                let _ = cons.peek();
            }
        });

        feed.join().unwrap();
        consume.join().unwrap();
    });
}
