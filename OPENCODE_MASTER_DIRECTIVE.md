# DiscoDesk — OpenCode Master Directive

You are the implementation agent for DiscoDesk, a commercial-quality, local-first, cross-platform audio-reactive wallpaper application. Read `ARCHITECTURE.md` and `RESEARCH.md` completely before modifying code. They are the product and technical source of truth. Do not silently simplify platform behavior, invent API capabilities, or claim a feature is supported until it is implemented and tested.

## Mission

Transform a user's ordinary still image into a subtle, depth-aware wallpaper that reacts musically to selected system/application audio. The result must preserve the image, keep desktop icons interactive, minimize visual fatigue, and pause intelligently to protect system performance.

## Non-negotiable architecture

1. Rust owns the background engine, audio capture, DSP, image analysis, renderer, desktop host adapters, package format and diagnostics.
2. Tauri v2 + React + TypeScript is the on-demand Studio UI only. The WebView is not the wallpaper renderer.
3. `wgpu` + WGSL is the shared rendering layer; native platform adapters own wallpaper placement.
4. Core playback works without Steam, cloud services, AI models, Node background process or network access.
5. AI inference is import-time only and cached. No AI model executes in the live audio/render loop.
6. Provide deterministic fallback for missing/failed model, audio capture, GPU features and platform desktop hosting.
7. Windows 10 builds below 20348 must use system/output WASAPI loopback; never expose app-specific audio lock unless runtime capability probing confirms it.
8. macOS Core Audio process taps are permission-gated and version-gated. Linux app-specific capture is capability-gated by PipeWire graph and session policy.
9. A frequency band is not an instrument stem or vocal detector. A transient detector is not a complete beat tracker.
10. No full-screen strobe, aggressive screen shake, uncontrolled chromatic split, destructive image warp, or topmost window over desktop icons.
11. Never promise resource numbers before reproducible benchmarks.
12. All models, binaries, source snippets and assets must pass license review before bundling or distributing.

## Engineering workflow

- Inspect repository state, branch, existing code, toolchain and installed SDK versions first.
- Write an implementation plan in `docs/IMPLEMENTATION_STATE.md` before major work; update it after every completed phase.
- Create architecture decision records (`docs/ADR/`) for decisions that affect public APIs or platform behavior.
- Use small, independently testable crates. Avoid a giant event handler or monolithic application entry point.
- Implement interfaces before platform backends; test each backend with a mock provider.
- Keep capture callbacks allocation-free and bounded. No mutex waits, file I/O, model inference, UI calls or logging inside real-time callbacks.
- Validate every file path, package, IPC payload, sample rate, channel count, buffer length, shader input, model output, state transition and OS handle.
- Handle `NaN`, infinities, zero-length buffers, silence, clipped input, missing devices, process exit, display hotplug, Explorer restart, GPU loss and permission denial.
- Keep logs local, rotating and privacy-preserving. Never write captured PCM to disk by default.
- Run formatting, linting, unit tests, integration tests, package tests and benchmarks after each meaningful change. Report exact commands and results.
- Do not suppress warnings or mark a phase complete when tests are skipped without explaining why.
- Do not add unused dependencies, dead code or speculative abstractions.
- Pin dependencies in Cargo.lock and frontend lockfile. Re-audit licenses on dependency updates.

## Phase gates

### Gate 0 — Repository scaffold

Create Cargo workspace, Rust toolchain pin, crate skeletons, Tauri shell, CI, formatting/lint configuration, tests and license notices. No wallpaper injection yet.

Acceptance:
- clean build on CI for the supported host toolchains;
- no unbounded background loops;
- engine and Studio are separate binaries/processes;
- IPC protocol version and error model documented.

### Gate 1 — DSP correctness

Implement deterministic PCM source, sample-rate normalization, multi-resolution feature extraction, spectral flux/onset, adaptive floor, envelopes and spring-damper.

Acceptance:
- tests for exact window durations/bin spacing;
- deterministic golden features for test vectors;
- stable silence/noise/clipping behavior;
- no allocations in the hot capture callback;
- audio-to-visual feature telemetry CLI works without any wallpaper host.

### Gate 2 — Renderer preview

Implement image upload/decode, `wgpu` adapter selection, base texture, subtle breathing, low-amplitude parallax fallback, local highlight response and optional sparse particles. Render in an ordinary preview window first.

Acceptance:
- 1080p/1440p/4K shader preview;
- correct color and aspect handling;
- clamped UVs and edge protection;
- stable GPU resource reuse;
- FPS cap works and GPU device-loss path is tested.

### Gate 3 — Windows desktop host

Implement WorkerW/Progman adapter behind `dd-platform`. Keep the host code isolated and instrumented. Add attach/re-attach, Explorer restart detection, non-activation and pointer pass-through.

Acceptance:
- desktop icons remain clickable;
- normal windows cover the wallpaper;
- no focus stealing;
- Explorer restart recovers;
- failure leaves normal OS wallpaper visible;
- test Windows 10 19042/19045 and supported Windows 11 builds.

### Gate 4 — Tauri Studio / quick workflow

Implement one-click import, auto analysis status, source selector with capability disclosure, presets, intensity, pause/resume, tray controls, reconnectable IPC and diagnostics.

Acceptance:
- closing Studio does not stop engine;
- starting Studio reconnects to existing engine state;
- no WebView wallpaper surface;
- all displayed capabilities match platform probes.

### Gate 5 — Image intelligence

Implement deterministic maps and content-addressed cache first. Then evaluate Depth Anything V2 Small ONNX vs Depth Anything 3 Small through a model adapter. Use model hashes and notices.

Acceptance:
- image corpus includes portraits, anime, text, low-light, landscape and abstract images;
- depth errors degrade gracefully;
- confidence weights parallax;
- user protect brush persists with package;
- no inference during playback;
- model analysis runtime/memory measured on target hardware.

### Gate 6 — Smart audio sources and power behavior

Implement Windows process loopback on qualifying builds, Auto/System/App source policy, ignore list, fullscreen pause and battery/display state.

Acceptance:
- Windows 10 fallback correctly shows system mix only;
- session changes do not crash audio thread;
- notification-like blips do not repeatedly wake the image;
- full-screen game stops wallpaper rendering according to settings;
- pause/resume fades are smooth.

### Gate 7 — macOS and Linux native adapters

Implement Core Audio process tap + permission UX and native desktop-level surface for macOS. Implement KDE Plasma 6 provider, PipeWire monitor capture and separately tested X11/wlr backends for Linux.

Acceptance:
- test on real target OS, not just compile;
- document per-desktop compatibility;
- no universal Linux/Wayland claim without compositor-specific validation.

### Gate 8 — Packages and Steam Workshop

Implement `.ddpkg` v1 with path traversal and decompression-bomb protection, checksums, migrations and preview generation. Add optional `ISteamUGC` upload/download adapter after local package validation.

Acceptance:
- package round-trip across app versions;
- malformed/untrusted package corpus rejected safely;
- Steam-disabled playback works;
- explicit publish rights confirmation;
- third-party notices and model license manifest complete.

## Initial task order

Do not implement every platform simultaneously. Start with the user's actual development environment:

1. Inspect available Rust, Tauri, Node toolchain, Windows SDK, GPU driver and Windows build.
2. Scaffold workspace and CI.
3. Build DSP telemetry CLI with mock PCM and Windows system loopback.
4. Build `wgpu` preview renderer reacting to `AudioFeatureFrame`.
5. Add deterministic image maps and package/cache types.
6. Attach the Windows wallpaper host only after preview correctness and desktop interaction tests.
7. Create the Tauri Studio quick workflow with FigCraft design reference.
8. Benchmark and choose depth model.

## Required deliverables for every phase

- Source changes with clear crate boundaries.
- Tests and exact results.
- Updated `docs/IMPLEMENTATION_STATE.md`.
- Updated platform support matrix.
- Performance measurements where relevant.
- License/source audit for new dependencies, models or assets.
- A concise list of remaining risks and known unsupported behavior.

Never state “complete” based only on successful compilation. A platform feature is complete only after runtime tests on that platform and its documented fallback paths.