# Audio pipeline — Gate 1

**Status:** implemented and measured. This document records the actual signal path,
the maths behind it, and what was observed on real hardware.

Code: `crates/dd-audio` (capture) and `crates/dd-dsp` (analysis).
Protocol: `docs/IPC_PROTOCOL.md` §6.2.

---

## 1. Signal path

```text
default render endpoint
  -> WASAPI shared-mode loopback (event driven)
  -> decode to f32, downmix to mono                [dd-audio::format]
  -> bounded SPSC ring, drop counted               [dd-audio::ring]
  -> 2048-sample window, 1024 hop (50% overlap)    [dd-dsp::engine]
  -> Hann window, coherent-gain normalised         [dd-dsp::window]
  -> radix-2 FFT -> magnitude spectrum             [dd-dsp::fft]
  -> six zone amplitudes -> dBFS -> 0..1           [dd-dsp::bands]
  -> attack/release envelopes                      [dd-dsp::envelope]
  -> positive spectral flux -> onset events        [dd-dsp::flux]
  -> RMS/peak -> critically damped smoothing       [dd-dsp::spring]
  -> FeatureFrame
```

## 2. Why these parameters

| Parameter | Value | Reason |
|---|---|---|
| FFT size | 2048 | 42.7 ms window at 48 kHz; 23.4 Hz bins. Documented in `ARCHITECTURE.md` §7.2. |
| Hop | 1024 | 50% overlap; 46.9 frames/s at 48 kHz, 43.1 at 44.1 kHz. |
| Window | periodic Hann | Verified to have exactly 0.5 coherent gain, which is what makes the dBFS mapping meaningful. |
| Band normalisation | `sqrt(sum(magnitude^2))` per zone | Energy-equivalent amplitude. A wide zone is not penalised for covering more bins than a narrow one. |
| Unit mapping | `(dBFS - floor) / -floor`, floor −60 dBFS | 0.0 means silence, 1.0 means 0 dBFS. No adaptive gain that would make silence look active. |
| Envelope | one-pole, `alpha = 1 - exp(-hop/tau)` | Coefficients derived from the **actual** hop, so 44.1 and 48 kHz behave the same in time. |
| Attack / release | 10 ms / 220 ms | `ARCHITECTURE.md` §7.5 starting ranges. |
| Flux | `sum(max(0, Δm)) / sum(m)` | Relative change. Dividing by bin count diluted a real onset to ~0.02. |
| Onset | flux ≥ 0.35 plus 120 ms refractory | Stops one attack producing a burst, and stops steady loudness firing. |
| Level smoothing | analytical critically damped spring, ~120 ms | Explicit Euler diverges at this hop/omega ratio (~4.1); the closed form cannot. |

### The 23.4 Hz resolution limit is real

At 48 kHz with 2048 samples, bin spacing is 23.4375 Hz. A 40 Hz tone sits in bins 1–2, so
"sub-bass" (20–60 Hz) has very coarse resolution there. At 44.1 kHz the spacing is
21.5 Hz. This is why `BandMap` reports **0** for a zone with no bins rather than borrowing
a neighbour: the limit is made visible instead of hidden. A longer low-frequency path is
listed in `ARCHITECTURE.md` §7.2 and is not implemented at Gate 1.

## 3. Real-time contract

* The capture path never logs, never performs file or network I/O, and never allocates
  once its buffers are warm (`Vec` reused, ring preallocated).
* The ring is a bounded SPSC with atomic indices; the capture thread never waits for DSP.
* Overflow drops the **newest** samples and increments `droppedSamples`. Combined with the
  DSP draining the ring fully on every wake, a permanent backlog cannot accumulate.
* Shutdown joins both threads; measured 4–15 ms.

## 4. Device lifecycle

* Default endpoint identity is re-queried every 2 s. A change ends the session and the
  capture thread opens a new one.
* `AUDCLNT_E_DEVICE_INVALIDATED` ends the session and is retried with a 1 s backoff that
  still observes the stop flag.
* A silent packet (`AUDCLNT_BUFFERFLAGS_SILENT`) carries undefined contents, so zeros are
  appended rather than decoding garbage.

## 5. Silence handling (found by running it)

WASAPI loopback delivers **no packets at all** while nothing is rendered. Without
intervention the DSP stops receiving hops and the envelopes freeze at their last value —
the wallpaper would keep reacting to audio that had stopped.

After a 100 ms gap the capture loop synthesises silence proportional to elapsed time
(capped at 2 s), so frames keep their cadence and decay:

```text
tone ends -> presence 0.86 -> 0.07 -> 0.00, state Silent
```

## 6. Measurements

Host: Windows 10 19045, AMD RX 6700 XT machine, output device
`OUT 1-2 (4- BEHRINGER UMC 202HD 192k)` at **44.1 kHz, 2 channels, f32**.

| Scenario | Result |
|---|---|
| Rendered silence | bands exactly `[0,0,0,0,0,0]`, loudness −120 dBFS |
| Pure 1 kHz, amplitude 0.5 | presence **0.86**, all other bands **0.00** |
| 100 Hz + 1 kHz + 8 kHz mix | punch 0.79, presence 0.76, air 0.68 |
| 4 s capture | 417 packets, 183,897 samples, **0 dropped**, 0 restarts |
| DSP per hop, release | 24–30 µs typical, **55 µs worst** |
| DSP per hop, debug | 300–600 µs typical, 1.47 ms worst |
| Hop budget | 21.3 ms at 48 kHz, 23.2 ms at 44.1 kHz |
| Runtime stop (join both threads) | 4–15 ms |

Release DSP therefore uses well under 1% of one core for analysis. No claim is made for
other hardware, sample rates or channel counts.

## 7. What is deliberately not implemented

* **Per-application capture.** Needs `AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK` and
  Windows 10 build 20348+; this host is 19045. `GetAudioSources`/`SetAudioSource` refuse
  with `NotImplemented` naming Gate 6.
* **Beat/tempo tracking.** Spectral flux is an onset feature, not a beat tracker.
  `tempo_bpm`/`beat_confidence` from `ARCHITECTURE.md` §7.1 are not produced.
* **Long low-frequency FFT path.** Sub-bass resolution is the 2048-point limit.
* **Adaptive normalisation.** Mapping is fixed dBFS; rolling percentile adaptation is a
  later refinement.
* **Channel weighting.** Downmix is a plain average.
