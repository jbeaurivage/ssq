A `#[no_std]`, single-producer, single-consumer, single-slot queue, useful for thread-safe message passing in embedded systems.

# Example

``` rust
use ssq::{Producer, Consumer, SingleSlotQueue};
let mut queue = SingleSlotQueue::<u32>::new();
let (mut cons, mut prod) = queue.split();

// `enqueue` returns `None` when the queue is empty.
let maybe_returned = prod.enqueue(50);
assert!(maybe_returned == None);

// `enqueue` returns `Some(t)` when the queue is already full.
let maybe_returned = prod.enqueue(2);
assert!(maybe_returned == Some(2));

// `enqueue_overwrite` overwrites the old value, at the cost of taking a lock.
prod.enqueue_overwrite(25);
assert!(cons.dequeue() ==  Some(25));

// `dequeue` returns `None` if the queue is empty.
assert!(cons.dequeue() == None);

```