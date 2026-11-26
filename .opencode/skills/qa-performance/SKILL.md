---
name: qa-performance
description: Use when writing or reviewing DiscoDesk tests, failure-path coverage, real-time safety, accessibility, profiling or performance acceptance evidence.
---

# QA and performance workflow

Convert every feature requirement into an observable acceptance criterion. Inspect current code and actual supported targets before selecting tests.

Cover deterministic unit tests, integration tests, malformed-input tests, lifecycle/recovery tests and platform tests. Include silence, noise, clipping, NaN/infinity, empty buffers, device loss, process exit, permission denial, display hotplug, Explorer restart, GPU loss, corrupted packages and cancellation races where relevant.

For real-time audio, verify bounded callbacks, allocation behavior and queue overflow. For graphics, report resolution, FPS cap, frame pacing, adapter/backend, GPU/driver, CPU/GPU time and memory. For startup/import analysis, measure cold and warm paths separately. Record test hardware and OS version.

Review visual comfort: no default strobing, uncontrolled shake, fast global luminance pulsing or aggressive distortion. Check reduced-motion, pause and edge/icon readability. WCAG flash guidance is a minimum policy reference, not a guarantee of comfort for every person.

Never invent benchmark results or claim optimization from code inspection alone. Report exact commands, pass/fail counts, skipped tests and untested platforms.
