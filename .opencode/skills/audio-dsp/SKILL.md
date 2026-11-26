---
name: audio-dsp
description: Use when implementing audio capture, active-source detection, PCM transport, FFT analysis, perceptual bands, onset features, envelopes or audio-to-visual mapping.
---

# Audio and DSP workflow

Separate capture, transport and feature extraction. Use deterministic PCM fixtures to test DSP independently of OS capture.

Define sample rate, channels, format, timestamps and discontinuity handling. Keep callbacks bounded, allocation-free and non-blocking; do not log unbounded events or write raw PCM by default. Use a preallocated bounded SPSC queue and measure overflow behavior.

Treat sustained energy, spectral flux/onset evidence, beat/tempo estimates and loudness as different signals. Frequency bands are not instrument stems or vocal detection. If using Bark/ERB/log-frequency aggregation, implement and test the actual mapping rather than relabel arbitrary bands.

Test silence, pink noise, sine sweeps, impulses, clipped samples, NaN/infinity, empty frames, resampling, sample-rate changes, queue overflow and source disconnection. Keep display interpolation separate from analysis cadence. Use smoothed envelopes and clamped spring responses to prevent jitter and overreaction.
