//! Capture statistics.
//!
//! Every counter is reported to telemetry. Sample loss is never hidden: if the
//! ring is full or the endpoint glitches, the counter says so.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

/// Live capture counters, shared between the capture thread and telemetry.
#[derive(Debug, Default)]
pub struct CaptureStats {
    /// Packets successfully read from the capture client.
    pub packets: AtomicU64,
    /// Frames (per-channel sample groups) captured.
    pub frames: AtomicU64,
    /// Mono samples produced after downmix.
    pub samples: AtomicU64,
    /// Packets Windows flagged as silent.
    pub silent_packets: AtomicU64,
    /// Mono samples the ring refused because it was full.
    pub dropped_samples: AtomicU64,
    /// Times the capture was re-established (device change, invalidation, error).
    pub restarts: AtomicU64,
    /// Times the default endpoint changed identity.
    pub device_changes: AtomicU64,
    /// Last error text, already free of paths.
    last_error: Mutex<Option<String>>,
}

impl CaptureStats {
    /// Records an error message (redacted upstream) for telemetry.
    pub fn set_error(&self, message: impl Into<String>) {
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = Some(message.into());
        }
    }

    /// Clears the recorded error.
    pub fn clear_error(&self) {
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = None;
        }
    }

    /// The last recorded error, if any.
    #[must_use]
    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|guard| guard.clone())
    }

    /// Increments a counter and returns the previous value.
    pub fn bump(counter: &AtomicU64, by: u64) -> u64 {
        counter.fetch_add(by, Ordering::Relaxed)
    }

    /// A consistent-enough snapshot for telemetry. Fields are read
    /// independently; a snapshot taken mid-update may combine values from two
    /// adjacent packets, which is acceptable for display.
    #[must_use]
    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            packets: self.packets.load(Ordering::Relaxed),
            frames: self.frames.load(Ordering::Relaxed),
            samples: self.samples.load(Ordering::Relaxed),
            silent_packets: self.silent_packets.load(Ordering::Relaxed),
            dropped_samples: self.dropped_samples.load(Ordering::Relaxed),
            restarts: self.restarts.load(Ordering::Relaxed),
            device_changes: self.device_changes.load(Ordering::Relaxed),
            last_error: self.last_error(),
        }
    }
}

/// A serialisable view of the counters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StatsSnapshot {
    /// Packets read.
    pub packets: u64,
    /// Frames captured.
    pub frames: u64,
    /// Mono samples produced.
    pub samples: u64,
    /// Silent packets.
    pub silent_packets: u64,
    /// Samples dropped by the ring.
    pub dropped_samples: u64,
    /// Capture restarts.
    pub restarts: u64,
    /// Default-endpoint identity changes observed.
    pub device_changes: u64,
    /// Last error, if any.
    pub last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_start_at_zero() {
        let stats = CaptureStats::default();
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.packets, 0);
        assert_eq!(snapshot.dropped_samples, 0);
        assert!(snapshot.last_error.is_none());
    }

    #[test]
    fn counters_accumulate() {
        let stats = CaptureStats::default();
        CaptureStats::bump(&stats.packets, 1);
        CaptureStats::bump(&stats.packets, 2);
        CaptureStats::bump(&stats.dropped_samples, 7);
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.packets, 3);
        assert_eq!(snapshot.dropped_samples, 7);
    }

    #[test]
    fn errors_are_recorded_and_cleared() {
        let stats = CaptureStats::default();
        stats.set_error("endpoint invalidated");
        assert_eq!(stats.last_error().as_deref(), Some("endpoint invalidated"));
        stats.clear_error();
        assert!(stats.last_error().is_none());
    }
}
