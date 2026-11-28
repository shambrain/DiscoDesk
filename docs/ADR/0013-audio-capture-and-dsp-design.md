# ADR-0013: Audio capture and DSP design for Gate 1

- **Status:** Accepted
- **Date:** 2026-11-28
- **Deciders:** discodesk-lead, audio-dsp-engineer, rust-engineer, systems-architect
- **Gate:** 1

## Context

Gate 1 is the first gate that touches real audio. Four decisions have long tails and were
not obvious:

1. Where the sample handoff between capture and analysis lives.
2. Whether to resample to a fixed internal rate or make the DSP rate-aware.
3. How band values are normalised so that "0.0" means silence rather than "quiet".
4. Whether `PROTOCOL_VERSION` must be bumped for the new commands.

## Decision

1. **A bounded lock-free SPSC ring, not a channel or a mutex.** The capture thread pushes
   mono `f32`; the DSP thread drains. Overflow drops the newest samples and increments a
   counter. Rejected: `std::sync::mpsc` (unbounded and allocating), a mutex (blocks the
   capture path), and a third-party ring (a dependency for 120 auditable lines).
2. **The DSP is rate-aware; nothing is resampled.** Envelope coefficients, bin
   frequencies and band ranges all derive from the negotiated rate, so 44.1 and 48 kHz
   behave identically in time and correctly in frequency. Rejected: resampling to 48 kHz,
   which would add a filter, latency and a failure mode for no benefit at Gate 1.
3. **Band values are dBFS-mapped with a fixed −60 dBFS floor.** 0.0 is silence, 1.0 is
   0 dBFS. Rejected: per-frame peak normalisation, which makes silence and loud passages
   look equally active — exactly the pumping `ARCHITECTURE.md` §7.6 forbids.
4. **`PROTOCOL_VERSION` stays 1.** `GetAudioTelemetry`, `StartAudio` and `StopAudio` are
   additive: an older client never sends them, and an older engine answers
   `UnknownCommand` (1006) rather than misbehaving. A version bump is reserved for a
   breaking framing or envelope change.
5. **Silence is synthesised when the endpoint stops rendering.** WASAPI loopback delivers
   no packets during silence, which froze the envelopes. The capture loop injects
   zero samples proportional to elapsed time after a 100 ms gap.

## Consequences

- Easier: capture cannot stall the DSP and the DSP cannot stall capture.
- Easier: users on any device format (44.1/48 kHz, 1–8 channels, f32/i16/i32/u8) get
  correct behaviour without a resampler.
- Harder: the band mapping is fixed, so a very quiet master looks uniformly dark. Rolling
  percentile adaptation is recorded as a Gate 3 refinement.
- Harder: synthesised silence is a small fiction; it keeps timing correct but means the
  engine cannot distinguish "rendered silence" from "nothing rendering" beyond the
  `Silent` state.
- Now forbidden: sending PCM over Studio IPC; running inference or file I/O in the capture
  path; an unbounded queue; normalising bands per frame.

## Verification

- `crates/dd-dsp`: 56 tests, including FFT-versus-DFT, per-zone tone placement at both
  44.1 and 48 kHz, silence decay, determinism and hostile input.
- `crates/dd-audio`: 36 tests plus 4 ignored hardware tests (format extremes, ring
  ordering under two threads, overflow accounting, silence synthesis, start/stop).
- `scripts/ipc-smoke.ps1`: audio commands and the telemetry CLI against a live engine.
- `docs/AUDIO_PIPELINE.md` records the measured hardware results.

## References

- `ARCHITECTURE.md` §6, §7; `docs/AUDIO_PIPELINE.md`; `docs/ADR/0009` (no async runtime).
- Microsoft WASAPI loopback and `AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK` documentation.
