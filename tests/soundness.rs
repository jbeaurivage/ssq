use rand::random;
use ssq::SingleSlotQueue;
use std::thread;

/// Soundness test that should be run through Miri
#[test]
fn enqueue_overwrite() {
    let mut queue = SingleSlotQueue::<u32>::new();
    let (mut cons, mut prod) = queue.split();

    thread::scope(|scope| {
        let feed = scope.spawn(|| {
            for _ in 0..500 {
                prod.enqueue(random());
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
