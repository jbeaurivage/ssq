//! A `#[no_std]` single-producer, single-consumer, single-slot queue, useful for thread-safe message passing in embedded systems.
//!
//! # Example
//!
//! ```
//! use ssq::{Producer, Consumer, SingleSlotQueue};
//! let mut queue = SingleSlotQueue::<u32>::new();
//! let (mut cons, mut prod) = queue.split();
//!
//! // `enqueue` returns `None` when the queue is empty.
//! let maybe_returned = prod.enqueue(50);
//! assert!(maybe_returned == None);
//!
//! // `enqueue` returns `Some(t)` when the queue is already full.
//! let maybe_returned = prod.enqueue(2);
//! assert!(maybe_returned == Some(2));
//!
//! // `enqueue_overwrite` overwrites the old value, at the cost of taking a lock.
//! prod.enqueue_overwrite(25);
//! assert!(cons.dequeue() ==  Some(25));
//!
//! // `dequeue` returns `None` if the queue is empty.
//! assert!(cons.dequeue() == None);
//!
//! ```

#![no_std]

use atomic_polyfill::{AtomicBool, Ordering};
use core::{cell::UnsafeCell, mem::MaybeUninit, ptr};

struct LightLock(AtomicBool);

impl LightLock {
    const fn new() -> Self {
        LightLock(AtomicBool::new(false))
    }

    /// Blocking; busy-wait until the lock is available
    fn lock(&self) -> LightGuard<'_> {
        loop {
            match self.try_lock() {
                None => continue,
                Some(w) => return w,
            }
        }
    }

    fn try_lock(&self) -> Option<LightGuard<'_>> {
        let was_locked = self.0.swap(true, Ordering::Acquire);
        if was_locked {
            None
        } else {
            Some(LightGuard { lock: self })
        }
    }
}

struct LightGuard<'a> {
    lock: &'a LightLock,
}

impl<'a> Drop for LightGuard<'a> {
    fn drop(&mut self) {
        self.lock.0.store(false, Ordering::Release);
    }
}

/// Single slot queue.
pub struct SingleSlotQueue<T> {
    full: AtomicBool,
    writing: LightLock,
    val: UnsafeCell<MaybeUninit<T>>,
}

impl<T> SingleSlotQueue<T> {
    pub const fn new() -> Self {
        SingleSlotQueue {
            full: AtomicBool::new(false),
            writing: LightLock::new(),
            val: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    pub fn split(&mut self) -> (Consumer<'_, T>, Producer<'_, T>) {
        (Consumer { ssq: self }, Producer { ssq: self })
    }
}

impl<T> Drop for SingleSlotQueue<T> {
    fn drop(&mut self) {
        if self.full.load(Ordering::Relaxed) {
            unsafe {
                ptr::drop_in_place(self.val.get() as *mut T);
            }
        }
    }
}

/// Read handle to a single slot queue.
pub struct Consumer<'a, T> {
    ssq: &'a SingleSlotQueue<T>,
}

impl<'a, T> Consumer<'a, T> {
    /// Try reading a value from the queue.
    ///
    /// # Blocking
    ///
    /// This method blocks if the corresponding [`Producer`] is currently [`enqueue_overwrite`](Producer::enqueue_overwrite)ing
    #[inline]
    pub fn dequeue(&mut self) -> Option<T> {
        if self.ssq.full.load(Ordering::Acquire) {
            // SAFETY: locking and holding onto the guard is important for enqueue_overwrite to be sound.
            let _guard = self.ssq.writing.lock();
            let r = Some(unsafe { ptr::read(self.ssq.val.get().cast()) });
            self.ssq.full.store(false, Ordering::Release);
            r
        } else {
            None
        }
    }

    /// Check if there is a value in the queue.
    #[inline]
    pub fn is_empty(&self) -> bool {
        !self.ssq.full.load(Ordering::Relaxed)
    }
}

impl<'a, T: Copy> Consumer<'a, T> {
    /// Try reading a value without dequeuing.
    ///
    /// # Blocking
    ///
    /// This method blocks if the corresponding [`Producer`] is currently [`enqueue_overwrite`](Producer::enqueue_overwrite)ing
    pub fn peek(&mut self) -> Option<T> {
        if self.ssq.full.load(Ordering::Acquire) {
            // SAFETY: locking and holding onto the guard is important for enqueue_overwrite to be sound.
            let _guard = self.ssq.writing.lock();
            Some(unsafe { ptr::read(self.ssq.val.get().cast()) })
        } else {
            None
        }
    }
}

/// Safety: We gurarantee the safety using an `AtomicBool` to gate the read of the `UnsafeCell`.
unsafe impl<'a, T> Send for Consumer<'a, T> {}

/// Write handle to a single slot queue.
pub struct Producer<'a, T> {
    ssq: &'a SingleSlotQueue<T>,
}

impl<'a, T> Producer<'a, T> {
    /// Write a value into the queue. If there is a value already in the queue this will
    /// return the value given to this method.
    #[inline]
    pub fn enqueue(&mut self, val: T) -> Option<T> {
        if !self.ssq.full.load(Ordering::Acquire) {
            unsafe { ptr::write(self.ssq.val.get().cast(), val) };
            self.ssq.full.store(true, Ordering::Release);
            None
        } else {
            Some(val)
        }
    }

    /// Write a value into the queue, overwriting the old value if it exists.
    ///
    /// # Blocking
    ///
    /// This method blocks if the corresponding [`Consumer`] is currently [`dequeue`](Consumer::dequeue)ing.
    pub fn enqueue_overwrite(&mut self, val: T) {
        // SAFETY: locking and holding onto the guard is important
        let _guard = self.ssq.writing.lock();
        self.ssq.full.store(false, Ordering::Release);
        unsafe { ptr::write(self.ssq.val.get().cast(), val) };
        self.ssq.full.store(true, Ordering::Release);
    }

    /// Check if there is a value in the queue.
    #[inline]
    pub fn is_empty(&self) -> bool {
        !self.ssq.full.load(Ordering::Relaxed)
    }
}

/// Safety: We gurarantee the safety using an `AtomicBool` to gate the write of the
/// `UnsafeCell`.
unsafe impl<'a, T> Send for Producer<'a, T> {}
