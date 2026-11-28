//! A bounded single-producer / single-consumer ring of `f32` samples.
//!
//! # Why this exists
//!
//! The WASAPI capture thread must hand samples to the DSP thread without
//! blocking, allocating or taking a lock. A lock-free SPSC ring with atomics is
//! the standard answer, and it is small enough to audit and test.
//!
//! # Overflow policy
//!
//! When the ring is full the **newest** samples are dropped and counted. That is
//! safe here because the consumer drains the whole ring on every wake and does
//! far less work than real time; it cannot accumulate a permanent backlog, so
//! "newest" never means "twenty seconds stale". Drops are always reported, never
//! hidden.
//!
//! # Safety argument
//!
//! * Indices are monotonically increasing `usize` values, masked to a
//!   power-of-two capacity.
//! * Only the producer writes `write`, only the consumer writes `read`, both
//!   with `Release`; each reads the other's index with `Acquire`.
//! * The producer writes slots in `[write, write + n)` which, by the availability
//!   check, are not in `[read, write)` that the consumer may be reading.
//! * `UnsafeCell` is used only to permit writing through a shared reference; all
//!   aliasing rules are enforced by the index protocol above, not by the type
//!   system.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// A bounded SPSC ring of mono `f32` samples.
#[derive(Debug)]
pub struct MonoRing {
    slots: Box<[UnsafeCell<f32>]>,
    mask: usize,
    write: AtomicUsize,
    read: AtomicUsize,
    dropped: AtomicU64,
}

// SAFETY: access to the slots is mediated entirely by the SPSC index protocol
// documented on the type. `MonoRing` is shared between exactly one producer and
// one consumer.
unsafe impl Sync for MonoRing {}
// SAFETY: moving the ring does not invalidate any index; slots are owned solely
// by the ring.
unsafe impl Send for MonoRing {}

impl MonoRing {
    /// Creates a ring holding at least `capacity` samples, rounded up to a power
    /// of two (minimum 2).
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let size = capacity.max(2).next_power_of_two();
        let mut slots = Vec::with_capacity(size);
        for _ in 0..size {
            slots.push(UnsafeCell::new(0.0_f32));
        }
        Self {
            slots: slots.into_boxed_slice(),
            mask: size - 1,
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
            dropped: AtomicU64::new(0),
        }
    }

    /// Usable capacity in samples.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Samples currently buffered.
    #[must_use]
    pub fn len(&self) -> usize {
        let write = self.write.load(Ordering::Acquire);
        let read = self.read.load(Ordering::Acquire);
        write.saturating_sub(read).min(self.capacity())
    }

    /// Whether the ring currently holds no samples.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Total samples dropped because the ring was full.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Pushes as many samples as fit; returns how many were accepted.
    ///
    /// Called only from the producer thread.
    pub fn push_slice(&self, samples: &[f32]) -> usize {
        if samples.is_empty() {
            return 0;
        }

        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Acquire);
        let used = write.saturating_sub(read);
        let free = self.capacity().saturating_sub(used.min(self.capacity()));
        let accepted = samples.len().min(free);

        for (offset, &sample) in samples.iter().take(accepted).enumerate() {
            let index = (write.wrapping_add(offset)) & self.mask;
            // SAFETY: this slot is in the producer-owned region `[write, write +
            // accepted)`; the availability check guarantees it is outside the
            // region the consumer may be reading. No other thread writes it.
            unsafe {
                *self.slots[index].get() = sample;
            }
        }

        if accepted > 0 {
            self.write
                .store(write.wrapping_add(accepted), Ordering::Release);
        }
        let rejected = samples.len() - accepted;
        if rejected > 0 {
            self.dropped.fetch_add(rejected as u64, Ordering::Relaxed);
        }
        accepted
    }

    /// Pops up to `out.len()` samples; returns how many were read.
    ///
    /// Called only from the consumer thread.
    pub fn pop_into(&self, out: &mut [f32]) -> usize {
        let read = self.read.load(Ordering::Relaxed);
        let write = self.write.load(Ordering::Acquire);
        let available = write.saturating_sub(read).min(self.capacity());
        let taken = out.len().min(available);

        for (offset, slot) in out.iter_mut().enumerate().take(taken) {
            let index = (read.wrapping_add(offset)) & self.mask;
            // SAFETY: this slot is in the consumer-owned region `[read, read +
            // taken)`; the producer only writes at or beyond `write`, and
            // `taken <= write - read`.
            *slot = unsafe { *self.slots[index].get() };
        }

        if taken > 0 {
            self.read.store(read.wrapping_add(taken), Ordering::Release);
        }
        taken
    }

    /// Discards all buffered samples. Consumer-side only.
    pub fn clear(&self) {
        let write = self.write.load(Ordering::Acquire);
        self.read.store(write, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn capacity_rounds_up_to_a_power_of_two() {
        assert_eq!(MonoRing::with_capacity(1).capacity(), 2);
        assert_eq!(MonoRing::with_capacity(3).capacity(), 4);
        assert_eq!(MonoRing::with_capacity(1000).capacity(), 1024);
        assert_eq!(MonoRing::with_capacity(1024).capacity(), 1024);
    }

    #[test]
    fn pushes_and_pops_in_order() {
        let ring = MonoRing::with_capacity(8);
        assert_eq!(ring.push_slice(&[1.0, 2.0, 3.0]), 3);
        assert_eq!(ring.len(), 3);

        let mut out = [0.0_f32; 8];
        assert_eq!(ring.pop_into(&mut out), 3);
        assert_eq!(&out[..3], &[1.0, 2.0, 3.0]);
        assert!(ring.is_empty());
    }

    #[test]
    fn wraps_around_without_corruption() {
        let ring = MonoRing::with_capacity(4);
        for round in 0..10 {
            let base = round as f32 * 10.0;
            let input = [base + 1.0, base + 2.0, base + 3.0];
            assert_eq!(ring.push_slice(&input), 3);
            let mut out = [0.0_f32; 4];
            let n = ring.pop_into(&mut out);
            assert_eq!(n, 3);
            assert_eq!(&out[..3], &input);
        }
    }

    #[test]
    fn a_full_ring_drops_the_newest_and_counts_it() {
        let ring = MonoRing::with_capacity(4);
        assert_eq!(ring.push_slice(&[1.0, 2.0, 3.0, 4.0]), 4);
        assert_eq!(ring.push_slice(&[5.0, 6.0]), 0);
        assert_eq!(ring.dropped(), 2);

        let mut out = [0.0_f32; 4];
        assert_eq!(ring.pop_into(&mut out), 4);
        assert_eq!(out, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn a_partial_push_reports_the_accepted_count() {
        let ring = MonoRing::with_capacity(4);
        assert_eq!(ring.push_slice(&[1.0, 2.0, 3.0]), 3);
        assert_eq!(ring.push_slice(&[4.0, 5.0, 6.0]), 1);
        assert_eq!(ring.dropped(), 2);
        assert_eq!(ring.len(), 4);
    }

    #[test]
    fn clearing_discards_buffered_samples() {
        let ring = MonoRing::with_capacity(8);
        ring.push_slice(&[1.0, 2.0, 3.0]);
        ring.clear();
        assert!(ring.is_empty());
        let mut out = [0.0_f32; 4];
        assert_eq!(ring.pop_into(&mut out), 0);
    }

    #[test]
    fn empty_and_oversized_pops_are_safe() {
        let ring = MonoRing::with_capacity(4);
        let mut out = [0.0_f32; 8];
        assert_eq!(ring.pop_into(&mut out), 0);
        ring.push_slice(&[1.0, 2.0]);
        assert_eq!(ring.pop_into(&mut out), 2);
        assert_eq!(ring.pop_into(&mut []), 0);
    }

    #[test]
    fn a_producer_and_consumer_on_separate_threads_preserve_every_sample() {
        // The producer sends a monotonically increasing sequence; the consumer
        // verifies it arrives in order with no gaps. Any torn or duplicated slot
        // breaks the sequence and fails the test.
        let ring = Arc::new(MonoRing::with_capacity(1024));
        let total = 200_000_usize;

        let producer = {
            let ring = Arc::clone(&ring);
            thread::spawn(move || {
                let mut sent = 0_usize;
                while sent < total {
                    let chunk: Vec<f32> =
                        (sent..(sent + 64).min(total)).map(|i| i as f32).collect();
                    let accepted = ring.push_slice(&chunk);
                    sent += accepted;
                    if accepted == 0 {
                        thread::yield_now();
                    }
                }
            })
        };

        let mut received = 0_usize;
        let mut buffer = [0.0_f32; 128];
        let mut spin = 0_u32;
        while received < total {
            let n = ring.pop_into(&mut buffer);
            for (offset, &value) in buffer[..n].iter().enumerate() {
                let expected = (received + offset) as f32;
                assert_eq!(value, expected, "sample {} out of order", received + offset);
            }
            received += n;
            if n == 0 {
                spin += 1;
                assert!(spin < 10_000_000, "consumer made no progress");
                thread::yield_now();
            }
        }

        producer.join().expect("producer must not panic");
        assert_eq!(received, total);
        assert!(ring.is_empty());
    }
}
