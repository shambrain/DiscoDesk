# DiscoDesk — Technical Architecture

**Document type:** Engineering master specification  
**Status:** Research-backed proposal; not an implementation or performance certification  
**Research date:** 26 November 2026  
**Product:** DiscoDesk — locally analysed, music-reactive desktop wallpapers

---

## 1. Product definition

DiscoDesk turns a user's own still wallpaper into a subtle, depth-aware, music-responsive desktop scene. The application analyses the image locally once, creates reusable visual-analysis assets, listens to an explicitly selected system or application audio source, extracts musical features, and drives a GPU renderer that adds restrained motion to the original artwork.

The product is **not** primarily a spectrum-bar visualizer, a video wallpaper player, a generative-art app, or a full 3D scene reconstruction tool.

### Core user promise

1. Select an image or the current wallpaper.
2. Press **Make It React**.
3. DiscoDesk analyses the image locally and proposes a restrained visual recipe.
4. It follows system audio automatically, or the user locks it to an application where the OS supports that capability.
5. The original image remains recognisable and compositionally stable. Motion is subtle by default and can be disabled or tuned independently.

### Product principles

- **Local-first:** audio and image analysis remain on the user's machine. No account, cloud inference, upload, or telemetry is required for core operation.
- **Image-preserving:** no generative image editing, face replacement, hallucinated geometry, or destructive changes to the source.
- **Low visual fatigue:** no full-screen strobing, constant camera shake, uncontrolled distortion, or harsh chromatic splitting by default.
- **Music-aware, not merely loudness-aware:** sustained energy, spectral change, transients, and optional beat estimates are treated separately.
- **Resource-aware:** pause or greatly reduce work when covered, fullscreen, on battery, or not visible; measure actual resource consumption rather than promise arbitrary numbers.
- **Honest platform support:** audio capture and desktop wallpaper hosting are implemented through native per-OS adapters. Cross-platform product architecture does not mean identical feature parity on every desktop environment.
- **Graceful fallback:** every AI-derived effect has a deterministic fallback. Missing models, unsupported capture APIs, and shell-host failures must not crash the desktop or lose the user's existing wallpaper.

### Non-goals for the first release

- True 3D reconstruction or inpainting of objects hidden behind foreground subjects.
- Real-time generative AI during playback.
- Voice, microphone, or ambient-room capture enabled by default.
- Automatic identification of vocals, individual instruments, or stems from simple FFT bands.
- A user-facing node graph / visual programming system.
- A full wallpaper marketplace or user-generated scripting runtime in the initial version.
- Universal Wayland support across every compositor on day one.

---

## 2. Competitive positioning and differentiation

Audio-reactive wallpapers and still-image shader effects already exist. Lively Wallpaper has audio visualizer integration and a separate depth-wallpaper workflow; KDE projects demonstrate both living-image shader effects and audio-reactive wallpaper shaders; Steam already hosts music-responsive desktop applications. DiscoDesk must not market itself as the first audio-reactive wallpaper engine.

The intended differentiation is the integrated workflow:

- Automatically analyse a user's own still image.
- Infer a conservative 2.5D depth layout, highlight map, palette, and likely low-detail particle regions.
- Generate a coherent visual recipe rather than ask the user to build a scene or configure shader inputs.
- Couple image-aware effects to musical features with tuned smoothing and strict visual limits.
- Deliver a polished, low-friction desktop product with transparent local processing and predictable power behaviour.

The product should be validated against Wallpaper Engine, Lively, Animate Desktop, KOpenWallpaper, Music Visualizer Engine, and other current products before public claims are made.

---

## 3. Recommended technology stack

| Area | Recommended choice | Reason / constraints |
|---|---|---|
| Core engine | Rust, Edition 2024 | Native runtime, memory safety, good system integration, one shared core |
| UI | Tauri v2 + TypeScript + React | Control center only; use OS WebView, load only when user opens Studio |
| UI design | FigCraft | Create and review interface flows, tokens, components and prototypes; it is not the runtime engine |
| GPU rendering | `wgpu` + WGSL | Native D3D12 / Metal / Vulkan paths; use fallback adapter when required |
| Audio capture | Native Rust platform adapters | WASAPI on Windows, Core Audio taps on macOS, PipeWire/PulseAudio on Linux |
| DSP | `realfft` or `rustfft`, custom filter/envelope code | Deterministic small DSP pipeline; benchmark both before pinning |
| Real-time handoff | Preallocated SPSC ring buffer (`ringbuf` or equivalent) | No locks or heap allocations in the capture callback |
| Image decode / processing | `image` + `fast_image_resize` initially | Lightweight, deterministic processing; add a CV library only when it provides a proven benefit |
| Depth inference | Model adapter: DA2 Small ONNX baseline; evaluate DA3 Small via `depth-anything.cpp` | Keep the runtime independent of one model format/provider; check licenses and benchmark target hardware |
| IPC | Named pipe on Windows; Unix domain socket on macOS/Linux | Local-only engine/Studio communication; no localhost HTTP server required |
| Packages | Versioned ZIP container with JSON manifest and hashed assets | Portable, inspectable, upgradeable `.ddpkg` format |
| Steam | Optional Steamworks / `ISteamUGC` adapter | Workshop publishing and subscription are optional integrations, not runtime dependencies |

### Why not Java or an always-running Node process?

A Java service or Electron/Node application is not needed. The audio capture, DSP, renderer, wallpaper host, cache, and power manager belong in the native Rust engine. Tauri's TypeScript/WebView frontend should be opened only for configuration and preview. Node may be used by the frontend build toolchain, but it should not remain running as a separate application service.

### Why `wgpu` instead of rendering in the WebView?

The wallpaper is a native, multi-monitor GPU surface. It needs predictable frame scheduling, resource ownership, platform-specific surface lifetime handling, and efficient shader access to image/depth textures. Tauri is for the user interface; `wgpu` is for the wallpaper renderer. Do not embed the wallpaper renderer inside the Studio WebView.

---

## 4. Runtime process model

Use two main processes and one optional helper boundary:

```text
┌────────────────────────────────────────────────────────────────┐
│ discodesk-studio (on demand)                                    │
│ Tauri v2 + React + TypeScript                                   │
│ Library · Vibe Studio · Preferences · Diagnostics · Workshop    │
└──────────────────────────────┬─────────────────────────────────┘
                               │ local IPC
┌──────────────────────────────▼─────────────────────────────────┐
│ discodesk-engine (background)                                   │
│                                                                  │
│ dd-core state machine                                            │
│ ├── dd-audio capture adapters                                    │
│ ├── dd-dsp feature extraction                                    │
│ ├── dd-analysis cache / import jobs                              │
│ ├── dd-renderer wgpu + WGSL                                      │
│ ├── dd-platform desktop host / power integration                 │
│ └── dd-package serializer                                        │
│                                                                  │
│ Optional: dd-steam integration (only when Steam features used)   │
└────────────────────────────────────────────────────────────────┘
```

`discodesk-engine` starts with the user only when they enable startup. It owns all wallpaper surfaces and native audio capture. Closing Studio must not stop wallpaper playback. The engine must be independently controllable via tray menu / global pause command and shut down cleanly.

### IPC contract

- Use versioned, length-delimited messages over a local named pipe / Unix socket.
- Authenticate the Windows named pipe with current-user ACLs; create Unix socket in a user-owned runtime directory with restrictive permissions.
- Do not bind an HTTP listener to `0.0.0.0` or expose unauthenticated loopback endpoints.
- Support request IDs, cancellation IDs, bounded payload sizes, structured errors, and protocol version negotiation.
- High-rate audio feature frames do **not** go through Studio IPC. They are passed directly inside the engine to the renderer. IPC carries settings, state, telemetry snapshots and user commands only.
- The UI can reconnect after being closed and reopened; the engine state remains authoritative.

Suggested IPC messages:

```text
GetEngineStatus
GetPlatformCapabilities
GetAudioSources
SetAudioSource { mode, source_id? }
SetVibePreset { preset_id }
SetEffectSettings { ... }
ImportWallpaper { path, options }
CancelJob { job_id }
ApplyPackage { package_id }
Pause / Resume / TogglePause
SetPerformancePolicy { ... }
GetDiagnosticsSnapshot
ShutdownEngine
```

Every message must validate its fields, enum values, path access, size limits, and version before acting.

---

## 5. Workspace layout

```text
discodesk/
├── Cargo.toml
├── rust-toolchain.toml
├── README.md
├── ARCHITECTURE.md
├── RESEARCH.md
├── OPENCODE_MASTER_DIRECTIVE.md
├── LICENSE
├── THIRD_PARTY_NOTICES.md
├── .gitignore
├── apps/
│   ├── discodesk-engine/
│   │   ├── Cargo.toml
│   │   └── src/
│   └── discodesk-studio/
│       ├── src-tauri/
│       └── ui/
├── crates/
│   ├── dd-core/
│   ├── dd-audio/
│   ├── dd-dsp/
│   ├── dd-analysis/
│   ├── dd-renderer/
│   ├── dd-platform/
│   ├── dd-package/
│   ├── dd-ipc/
│   ├── dd-diagnostics/
│   └── dd-steam/                 # optional, behind a feature flag
├── shaders/
│   ├── common/
│   ├── image_base.wgsl
│   ├── depth_parallax.wgsl
│   ├── local_glow.wgsl
│   ├── particles.wgsl
│   └── composite.wgsl
├── models/
│   ├── manifests/                # model metadata / hashes; no unlicensed weights
│   └── README.md
├── assets/
│   ├── presets/
│   ├── icons/
│   └── sample_wallpapers/        # only redistributable test artwork
├── docs/
│   ├── ADR/
│   ├── DSP_SPEC.md
│   ├── PACKAGE_FORMAT.md
│   ├── PLATFORM_SUPPORT.md
│   ├── PRIVACY.md
│   └── PERFORMANCE_BUDGET.md
└── tests/
    ├── fixtures/
    ├── audio-corpus/
    ├── image-corpus/
    ├── integration/
    └── benchmarks/
```

Each crate should have one clear responsibility and a public API documented with purpose, parameters, return type, errors, ownership and thread requirements. Avoid a monolithic `main.rs` or large Tauri commands that perform engine work.

---

## 6. Audio capture subsystem (`dd-audio`)

### 6.1 Shared model

Expose an `AudioSourceDescriptor` with:

```rust
struct AudioSourceDescriptor {
    stable_id: String,
    display_name: String,
    process_id: Option<u32>,
    process_name: Option<String>,
    source_kind: AudioSourceKind,
    capability: CaptureCapability,
    is_active: bool,
}

enum AudioSourceKind {
    SystemMix,
    Application,
    OutputDevice,
}

enum CaptureCapability {
    SystemMixOnly,
    PerApplication,
    Unavailable { reason: String },
}
```

Use stable platform IDs internally and human-readable names in the UI. PIDs are transient; refresh them when processes exit/restart. The UI must never label a device-loopback stream as per-app capture.

### 6.2 Windows adapter

- Primary system-mix path: WASAPI loopback capture from the selected render endpoint.
- App-specific path: `ActivateAudioInterfaceAsync` with `AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK` and include/exclude process-tree mode, where supported.
- Enumerate audio sessions with Core Audio session APIs for app names, process IDs, state and UI selection; session enumeration alone does not deliver raw PCM for each app.
- Handle default-device changes, endpoint removal, format renegotiation, audio service restart, process exit, and device sleep/resume.
- Re-open the appropriate capture path and fade visual features to silence while reconnecting; do not hang the desktop host.

**Compatibility gate:** Microsoft's Application Loopback sample states a minimum of Windows 10 build 20348. Standard retail Windows 10 installations commonly run lower than that. For the user's existing Windows 10 20H2-class build, the initial reliable feature should be system/output loopback; app-specific capture must be shown as unavailable unless capability probing confirms support. Windows 11 is the main target for the process-specific path.

### 6.3 macOS adapter

- Use Core Audio process taps (`AudioHardwareCreateProcessTap`) and aggregate devices for process-specific capture where available.
- Treat macOS 14.2+ as the initial minimum for the Core Audio process-tap implementation.
- Declare `NSAudioCaptureUsageDescription` and explain why access is needed before triggering the OS permission prompt.
- Handle permission denial, revocation, process termination, aggregate-device failure, route changes, and sleep/wake.
- Never silently switch to microphone capture. Microphone input is a separate opt-in feature, not a fallback.

### 6.4 Linux adapter

- Primary path: native PipeWire stream capture. Enumerate graph nodes, connect to the default sink monitor for system-mix capture, and support selecting a specific application node where the session manager exposes the route and permissions allow it.
- PulseAudio compatibility path: capture the selected sink's monitor source.
- Track node removal, default-sink changes, channel/sample-format changes and suspend/resume.
- `pw_stream` callbacks must dequeue and return buffers correctly; process callbacks should do only bounded, non-blocking work and hand frames to the DSP worker.
- Provide capability disclosure for app-specific routing. PipeWire graph capture is not automatically a universal per-application PCM API across all desktop policies.
- No GNOME/Wayland claim until a real GNOME test matrix and supported desktop-surface integration exist.

### 6.5 Auto source selection

Do not rely on a single hard-coded `-50 dBFS` test. Use an event-driven policy:

1. Enumerate eligible active sessions / nodes.
2. Score candidates using sustained signal energy, recent activity, source persistence and optional foreground preference.
3. Ignore configured applications and short transient sessions.
4. Require a configurable sustain period before auto-switching.
5. Apply hysteresis so quiet gaps do not bounce between applications.
6. Let the user lock to System Mix or a named application.
7. Show the selected source clearly and provide an immediate mute/stop button.

Auto mode is a convenience heuristic. It must never silently capture an unrelated microphone or claim per-app isolation when only the system mix is available.

### 6.6 Real-time callback rules

- Capture callback: copy or reference the valid PCM frame into a preallocated bounded ring buffer and return promptly.
- No heap allocations, blocking locks, file I/O, logging, model execution, UI calls or shader work in the callback.
- Handle ring-buffer overflow by dropping the oldest or newest complete frame according to a documented policy; increment a diagnostic counter.
- Resample to a stable internal rate on a worker thread when necessary.
- Preserve channel layout metadata; downmix only in the analysis path.

---

## 7. DSP subsystem (`dd-dsp`)

### 7.1 Input and output contract

Normalize captured PCM to `f32`, 48 kHz internally, preserving stereo where helpful. The renderer consumes a compact feature frame, not raw PCM.

```rust
struct AudioFeatureFrame {
    timestamp_ns: u64,
    level_rms: f32,
    loudness_dbfs: f32,
    sub_bass: f32,
    bass: f32,
    low_mid: f32,
    mid: f32,
    high_mid: f32,
    air: f32,
    transient: f32,
    spectral_flux: f32,
    beat_confidence: f32,
    tempo_bpm: Option<f32>,
    is_silent: bool,
}
```

All normalized values must be finite and clamped to `0.0..=1.0`. Tempo is optional and must carry confidence; never invent a reliable beat where no estimate exists.

### 7.2 Multi-resolution analysis

A single 2,048-sample FFT is inadequate for detailed sub-bass analysis at 48 kHz:

- 2,048 samples span about 42.7 ms.
- Bin spacing is about 23.4 Hz.
- 50% overlap advances by 1,024 samples, about 21.3 ms.

Use at least two analysis paths:

| Path | Starting configuration | Purpose |
|---|---|---|
| Low-frequency path | 8,192 samples, Hann window, 50% overlap | Better low-frequency discrimination; 5.86 Hz/bin at 48 kHz |
| Fast path | 1,024–2,048 samples with 50% overlap | Faster transient/high-frequency updates |
| Optional filter path | Low-cost IIR band filters with smoothed energy | Fast per-band envelopes without waiting for a long FFT window |

These are initial engineering values, not sacred constants. Benchmark temporal feel, CPU cost and bass resolution using real music. Apply window normalization and spectral scaling correctly; avoid comparing raw FFT magnitudes across different window sizes without normalization.

### 7.3 Product-facing frequency zones

Use six configurable feature zones:

| Feature | Initial range | Intended effect channel |
|---|---:|---|
| `sub_bass` | 20–60 Hz | Slow mass / depth breathing |
| `bass` | 60–150 Hz | Small transient impulse / bass weight |
| `low_mid` | 150–500 Hz | Warm ambient response |
| `mid` | 500 Hz–2.5 kHz | General presence / image energy |
| `high_mid` | 2.5–6 kHz | Small highlight articulation |
| `air` | 6–20 kHz | Sparse high-detail shimmer / fine particles |

These are custom broad frequency groupings. They are **not six Bark critical bands and are not six instrument stems**. A frequency band cannot identify a vocal, guitar, lead, snare or hi-hat reliably. Use names that describe frequency/energy rather than claiming source separation.

Optionally integrate energy using Bark- or ERB-spaced weighting where it improves perceptual balance, while keeping the user-facing six zones as a simple control surface.

### 7.4 Transients and spectral flux

- Use positive spectral flux (half-wave rectified frame-to-frame magnitude increase) with per-band weighting.
- Normalize against a rolling noise floor / adaptive threshold, and apply a refractory interval to prevent repeated triggers from one attack.
- Distinguish onset/transient strength from sustained energy.
- A transient detector is not a complete beat tracker. Beat/tempo estimation is an optional later module with confidence and a fallback to transient-only behavior.
- If using phase-based or complex-domain onset features, benchmark against the simpler flux detector before adding complexity.

### 7.5 Envelopes and motion ballistics

Use independent envelope followers per feature:

- Fast attack and smoother release for transient channels.
- Slower attack / release for broad ambient energy.
- A critically damped or slightly underdamped spring for visual displacement after the feature frame has been smoothed.

Starting tuning ranges:

- transient envelope attack: 10–25 ms;
- sustained energy attack: 40–120 ms;
- release: 180–350 ms;
- spring settling: tune separately from the DSP envelope.

Do not claim a 5 ms response from a 2,048-sample FFT updated every 21.3 ms. Short attacks require sample/block-level filters or faster feature updates. All timing constants must be configurable and covered by automated impulse-response tests.

### 7.6 Adaptive normalization

- Learn rolling percentiles per band over a bounded window.
- Use a silence floor / gate and prevent quiet room noise from driving visual motion.
- Adapt to different mastered loudness levels without pumping the entire scene.
- Avoid aggressive automatic gain control that makes silence and loud sections look equally active.
- Make sensitivity and minimum threshold user-visible in advanced settings.

### 7.7 Performance and concurrency

- One audio worker owns DSP state.
- Use fixed-size buffers and avoid per-frame allocations.
- Publish compact feature frames through a latest-value slot or bounded ring buffer.
- Renderer reads the newest valid feature frame and interpolates at render cadence.
- A slow renderer must not cause unbounded audio backlog.
- Provide telemetry for dropped capture frames, DSP duration, queue depth and stale-frame count.

---

## 8. Wallpaper analysis (`dd-analysis`)

### 8.1 Import pipeline

```text
image selected
  → validate file / size / format
  → decode + EXIF orientation + color management
  → compute content hash
  → resize a model copy to the model's supported dimensions
  → optional depth inference (once)
  → derive depth confidence / reliability mask
  → extract luminance, edges, highlights and saliency
  → extract dominant palette in perceptual color space
  → infer conservative motion regions and negative-space regions
  → create an editable effect recipe
  → cache analysis by source hash + model + pipeline version
```

Supported initial import formats: PNG, JPEG, WebP. HEIF/AVIF and animated formats can be considered only after decode, color, memory and licensing tests. Handle EXIF orientation and ICC color profiles explicitly; never silently rotate or shift the source on screen.

### 8.2 Depth model strategy

Expose a model-neutral trait:

```rust
trait DepthEstimator {
    fn model_id(&self) -> &str;
    fn model_version(&self) -> &str;
    fn estimate(&self, image: &RgbImage) -> Result<DepthEstimate, AnalysisError>;
}

struct DepthEstimate {
    width: u32,
    height: u32,
    relative_depth: Vec<f32>,
    confidence: Option<Vec<f32>>,
}
```

Evaluate two local candidates:

1. **Depth Anything V2 Small via ONNX Runtime** — established small checkpoint; upstream says the Small model is Apache-2.0, while Base/Large/Giant use CC BY-NC 4.0. This is a practical baseline for a first native inference adapter.
2. **Depth Anything 3 Small via `depth-anything.cpp`** — newer model family with a C API and ggml backends, reported by its maintainers as CPU-first. The adapter's code is MIT, but its weights retain the original model's license. DA3 Small is listed as Apache-2.0; avoid non-commercial large/giant checkpoints in a paid product.

**Do not decide between DA2 and DA3 from model names alone.** Build a test corpus of wallpapers (anime, portraits, landscapes, architecture, cars, abstract art, low-light images, typography), compare depth quality, edge artefacts, confidence, load time, peak memory, and analysis time on actual target machines. Keep `DepthEstimator` swappable. Do not make DA3 a hard dependency until its native port, model packaging, and Windows/macOS/Linux builds have been verified.

Depth is an estimate from one view. It does not reveal hidden surfaces, guarantee object segmentation, or produce a physically accurate scene. Treat it as a soft displacement guide, not a true 3D mesh.

### 8.3 Deterministic fallback

If the model is absent, too slow, unsupported or fails:

- compute palette, luminance, edge and local-contrast maps;
- use a neutral low-amplitude breathing/ambient drift recipe;
- disable depth-dependent displacement and occlusion;
- clearly mark the analysis as **Basic** rather than silently implying AI depth was generated.

Users must be able to skip AI analysis, delete cached model assets, or re-run analysis after replacing the image.

### 8.4 Image masks and recipe

Produce separate maps instead of one opaque mask:

- `depth_map`: normalized relative depth;
- `confidence_map`: per-pixel inference reliability, when available;
- `highlight_map`: luminance/contrast-derived candidate light pixels;
- `edge_map`: boundaries to protect from obvious warping;
- `saliency_map`: a deterministic attention heuristic, not semantic understanding;
- optional `protect_mask`: user-painted regions that should not move or receive particles.

Palette extraction should work in OKLab/OKLCH or another perceptually meaningful space. Use stable color quantization and avoid changing the source image's palette globally. Glow and particle colors should be derived from the original art and remain adjustable.

Particle placement should prefer low-detail / negative-space areas. Do not claim semantic sky or subject detection unless a dedicated segmentation model is added and tested. Provide a simple user brush: **Protect**, **Allow movement**, **Particle zone**.

### 8.5 Analysis cache

Cache key:

```text
SHA-256(source bytes)
+ import pipeline version
+ depth model identifier/version
+ model settings
+ image orientation/color transform version
```

The cache must be content-addressed, atomically written, size-limited and safe to delete. If an input changes, do not reuse stale depth maps. Reopening an identical image with the same analysis version should reuse cached results.

---

## 9. GPU renderer (`dd-renderer`)

### 9.1 Rendering layers

```text
0. Original wallpaper (unaltered source texture)
1. Small depth-aware camera/parallax displacement
2. Optional slow breathing / ambient drift
3. Localized highlight bloom / gentle emissive shimmer
4. Sparse depth-aware particles in allowed regions
5. Optional low-opacity haze / dust
6. Composite with edge protection and luminance limits
```

Keep each effect independently toggleable and scaleable. Preserve the original source image and allow an instant **Original / Reactive** comparison.

### 9.2 Parallax and image protection

- Use relative depth to create a very small virtual-camera displacement.
- Weight displacement by model confidence and edge reliability.
- Clamp UV offsets and overscan the source texture to avoid exposing black borders.
- Use border attenuation; avoid warping desktop edges and icon zones.
- Offer a focal point and protection brush when automatic results are visibly wrong.
- Do not synthesize hidden pixels at runtime in the MVP. If an image cannot tolerate even small displacement, automatically lower or disable parallax.

### 9.3 Audio-to-visual mapping

Do not wire raw FFT bins directly to arbitrary pixel displacement. Separate continuous and transient channels:

| Audio feature | Visual response | Design constraint |
|---|---|---|
| Sub/bass sustained energy | Very small slow depth/scale breath | No repeated full-screen thump |
| Bass transient / onset | Tiny spring impulse to camera or selected region | One impulse per attack; fast decay |
| Low-mid energy | Slight local warmth / environmental intensity | Avoid global color wash |
| Mid energy | Small intensity modulation for selected focal highlights | Frequency band is not vocal detection |
| High-mid energy / flux | Fine local shimmer on highlight map | Avoid high-frequency jitter |
| Air energy | Sparse particles in permitted negative space | Low count, low opacity, slow drift |
| Beat estimate | Optional subtle phase alignment | Only when confidence is adequate |

Mappings are creative presets, not universal psychological laws. Let users invert, remap, disable and tune each response.

### 9.4 Visual comfort defaults

- Default intensity should look nearly static at a glance; movement is discovered over a few seconds.
- No full-screen flashing, rapid alternation, or abrupt strobing.
- Avoid continuous motion in peripheral UI/icon regions.
- Limit parallax, scale and glow amplitude using empirically tuned defaults.
- Use smooth transitions on start, stop, source changes and app focus changes.
- If audio is silent, the scene settles into an almost-static ambient state rather than snapping to a different image.
- Respect OS reduced-motion/accessibility settings where available; provide a direct **Reduce Motion** toggle regardless.
- Add a strict safety preset which disables particles, bloom and transients.

Do not advertise a universal safe percentage of luminance modulation. Accessibility guidance around flashing is a minimum restriction, not a guarantee against discomfort. DiscoDesk should use conservative visual design and let users disable motion completely.

### 9.5 GPU scheduling and power policy

- Default target: 30 FPS, with 24/30/60 options and 60 FPS opt-in. Do not run at 144/240 Hz by default.
- Pause rendering when its monitor is fully covered by a fullscreen app, during selected game mode, or when the display is asleep.
- Reduce to a very low update rate when the image is static and there is no audio or ambient effect requiring motion.
- Respect battery saver and configurable AC/battery policy.
- Avoid rendering hidden monitors or duplicated work where possible.
- Recreate device resources after adapter/device loss and recover without terminating the engine.
- Use fixed-size GPU buffers and reusable textures; no per-frame texture creation.
- Report actual FPS, GPU frame time, CPU worker time and allocated texture size in diagnostics.

No initial promise of `<1% CPU`, a fixed VRAM figure or universal 4K performance is permitted until measured on a documented hardware matrix.

---

## 10. Desktop host adapters (`dd-platform`)

There is no single public wallpaper-surface API that provides identical behavior on Windows, macOS and every Linux compositor. Implement a common trait with native backends and a capability matrix.

```rust
trait DesktopHost {
    fn enumerate_displays(&self) -> Result<Vec<DisplayInfo>, HostError>;
    fn create_surface(&mut self, display: &DisplayInfo) -> Result<SurfaceHandle, HostError>;
    fn set_wallpaper_surface(&mut self, surface: SurfaceHandle) -> Result<(), HostError>;
    fn detach_surface(&mut self, surface: SurfaceHandle) -> Result<(), HostError>;
    fn query_visibility(&self, display_id: &str) -> VisibilityState;
    fn subscribe_events(&mut self, sink: HostEventSink);
}
```

### Windows

- First implementation uses a carefully isolated WorkerW / Progman desktop-host adapter based on proven live-wallpaper techniques, with Explorer restart recovery.
- This shell integration is implementation-sensitive and must be tested on the supported Windows versions; do not treat undocumented shell window hierarchy as a guaranteed public contract.
- Surface must be non-activating, non-focusable and non-intercepting for desktop icon input.
- On host attach failure, retain the OS wallpaper and expose a diagnostic; never place a full-screen topmost overlay over the user's desktop.

### macOS

- Use a native AppKit/SwiftUI window configured at desktop window level, behind normal windows. Apple documents a `desktop` level specifically for windows just above wallpaper and behind everything else.
- Validate behavior with Finder desktop icons, Spaces, Mission Control, multiple displays, display sleep, full-screen apps and login/logout.
- Audio process taps require permission; wallpaper window hosting and audio permission are separate capabilities.

### Linux

- KDE Plasma 6: prefer a native Plasma wallpaper provider/plugin for reliable wallpaper lifecycle and placement.
- Wayland compositors implementing `wlr-layer-shell`: use the background layer only where the protocol is supported and tested.
- X11: use a dedicated desktop-window backend with correct desktop type/hints and no input capture.
- GNOME and other Wayland compositors: declare unsupported until a native host/extension integration is built and tested. Do not market “Linux support” based only on one KDE or wlroots test.

### Multi-monitor requirements

- Enumerate display IDs, resolution, scale factor, HDR capability and refresh rate.
- Create one surface per display, each with its own viewport, render pacing and visibility state.
- Support independent wallpapers initially; spanning a single image across monitors can be a later feature.
- Hotplug, DPI scaling and display rotation must not corrupt source layout.

---

## 11. Engine state machine and lifecycle

Use an explicit state machine rather than scattered booleans:

```text
Stopped
  → Starting
  → IdleReady
  → Listening
  → AudioActive
  → Rendering

Any running state may transition to:
  PausedByUser
  PausedByFullscreen
  PausedByPowerPolicy
  PausedByDisplaySleep
  WaitingForPermission
  RecoveringAudio
  RecoveringGpu
  RecoveringDesktopHost
  AnalysisFallback
  ErrorRecoverable
```

Rules:

- A permission failure is not a crash; report the missing capability and continue with a supported fallback.
- Audio capture failure must not destroy the wallpaper surface; fade to ambient mode.
- GPU device loss must recreate resources and reload cached textures.
- Explorer/desktop-host restart must trigger bounded reattachment retries.
- Cancellation must be honored by all analysis/import jobs.
- All transitions are logged to a rotating local diagnostic log without logging PCM or image contents.
- Closing Studio does not stop the engine unless the user explicitly chooses Exit.

---

## 12. `.ddpkg` package format

Use a ZIP container with versioned JSON manifest and content-addressed assets. Keep cache data separate from an exported/shareable package.

Example layout:

```text
MyWallpaper.ddpkg
├── manifest.json
├── source/image.png
├── analysis/depth.r16
├── analysis/confidence.r8          # optional
├── analysis/highlight.r8
├── analysis/edge.r8                # optional
├── analysis/protect.r8             # optional
├── palette.json
├── recipe/vibe.json
└── checksums.sha256
```

Example manifest:

```json
{
  "format": "ddpkg",
  "format_version": 1,
  "discodesk_min_version": "0.1.0",
  "package_id": "local.content-addressed-id",
  "source": {
    "path": "source/image.png",
    "sha256": "<sha256>",
    "width": 3840,
    "height": 2160,
    "format": "png"
  },
  "analysis": {
    "pipeline_version": 1,
    "depth_model": "depth-anything-v2-small",
    "depth_model_version": "<model-version-or-hash>",
    "relative_depth_path": "analysis/depth.r16",
    "confidence_path": null,
    "highlight_path": "analysis/highlight.r8"
  },
  "recipe": {
    "path": "recipe/vibe.json",
    "preset_id": "soft-pulse"
  },
  "integrity": {
    "checksums_path": "checksums.sha256"
  }
}
```

Security rules:

- Validate archive paths; reject absolute paths, `..`, symlinks and duplicate names.
- Bound decompressed size, image dimensions, entry count and total file count.
- Verify hashes before applying a package.
- Never execute package scripts in the initial format.
- Store original source only when user exports/shares the package; local cache may reference the original path and hash.
- Add schema migration and unknown-field handling before public releases.
- Treat Workshop packages as untrusted user content.

---

## 13. Vibe Studio and user experience

### Quick mode

One screen with:

- Current wallpaper preview / choose image.
- **Make It React** / **Pause**.
- Audio source: Auto / System Mix / selected App (only when supported).
- Vibe: Soft Pulse / Slow Bloom / Air & Dust / Club Pulse / Ambient Drift / Studio Minimal.
- Overall intensity and Reduce Motion.
- FPS / performance preset.

### Advanced mode

**Audio**
- Per-band sensitivity, noise floor, attack, release, transient strength, beat confidence threshold.
- Ignore-list and per-application lock.

**Visual**
- Depth intensity, camera breathing, highlight glow, particle amount, particle size/speed, palette tint.
- Protect / Allow Movement / Particle Zone brush.
- Focal point and edge-safe margin.

**Performance**
- 24 / 30 / 60 FPS cap.
- Pause on fullscreen, game mode, battery saver, covered desktop, display sleep.
- Texture quality / analysis quality.

**Library**
- Imported wallpapers, cached analysis state, packages, presets, favorites, recent applications.

**Diagnostics**
- Audio-source capability matrix, capture state, DSP timing, renderer backend, GPU frame time, memory, host attach state and recoverable errors.

The default flow must not ask the user to understand FFT, Bark, shaders, model names or OS audio internals.

---

## 14. Steam and content sharing

- Core playback must work without Steam running or user login.
- Steam integration belongs in an optional `dd-steam` crate behind a feature flag.
- Use `ISteamUGC` (`CreateItem`, `StartItemUpdate`, item metadata/content/preview setters, `SubmitItemUpdate`) for modern Workshop publishing. Do not build new code on deprecated legacy publish calls.
- Workshop packages need validated content, previews, tags, format versions and clear creator attribution.
- Do not automatically publish a user's personal wallpaper. Require explicit action, rights confirmation and package preview.
- Workshop subscription/download is a later phase; first implement local package import/export and verify package stability.
- Do not assume Rust bindings expose every current Steamworks function. Verify the binding version and use a small audited FFI boundary if required.
- Steam Cloud should sync small settings/library metadata, not blindly upload full 4K wallpaper assets or large depth caches.
- Any user-uploaded image or wallpaper package remains the user's responsibility to share; add reporting/moderation and attribution metadata for community distribution.

---

## 15. Performance and reliability contract

Measure performance before setting promises. Required metrics:

- idle engine CPU and private working set;
- active audio capture + DSP CPU;
- render CPU submission time and GPU frame time;
- allocated GPU texture memory at 1080p, 1440p, 4K and multiple displays;
- import-analysis wall-clock time and peak memory;
- resume time after fullscreen / display wake;
- sustained 8-hour stability and memory growth.

Suggested initial product policy:

- 30 FPS cap by default.
- Rendering paused when not visible / fully covered / fullscreen game policy activates.
- Audio analysis does not queue indefinitely; only latest features are relevant.
- Cache model session and GPU textures; avoid repeated allocation.
- Use a low-resolution analysis map and upsample for shader use when quality permits.
- Use GPU texture formats chosen for precision and size; validate depth precision and banding in 16-bit or normalized formats.
- Avoid full-resolution duplicate buffers unless profiling shows they are needed.

Do not state `0% GPU`, `<0.8% CPU`, `<60 MB VRAM`, or “runs at 4K/120 FPS on every GPU” without measured reproducible results. A single RGBA 4K texture alone is about 31.6 MiB before depth, masks, swapchain buffers and intermediate textures.

### Test matrix

At minimum:

- Windows 10 build 19042 / 19045: system loopback, fallback behavior, desktop host, Explorer restart.
- Windows 11 current supported build: system + process loopback, multiple displays, audio session churn.
- macOS 14.2+ and current release: Core Audio tap permissions, selected process termination, Spaces, sleep/wake.
- Linux KDE Plasma 6 Wayland: native wallpaper plugin + PipeWire.
- Linux X11: desktop host + PulseAudio/PipeWire.
- Additional Wayland compositor only after a dedicated integration test.
- GPU tiers: integrated graphics, mainstream discrete, older DX11/GL fallback if supported, AMD and NVIDIA.
- 1080p, 1440p, 4K, mixed DPI, 60/120/144 Hz, single and multi-monitor.

---

## 16. Test strategy

### Unit tests

- FFT scaling/window normalization and band aggregation.
- Filter coefficients and stable behavior at sample-rate changes.
- Spectral flux positivity and noise-floor adaptation.
- Impulse onset detector false positives and refractory behavior.
- Envelope attack/release response and time constants.
- Spring-damper stability at different frame rates / large delta times.
- Value clamping, NaN/Inf rejection, silence and clipping.
- Depth normalization and confidence blending.
- Palette extraction determinism.
- Mask bounds, edge attenuation and UV clamp.
- `.ddpkg` path traversal / decompression bomb / malformed manifest / hash failures.
- IPC version mismatch, oversized messages, unknown command and permission denied.

### Integration tests

- Mock audio provider produces deterministic PCM test vectors.
- Loopback source disconnect/reconnect and default-device switching.
- Process exits while locked; auto mode selects a valid fallback.
- Desktop host receives pause/resume and monitor hotplug events.
- GPU loss/recreation does not crash engine.
- Analysis cancellation leaves no partial cache presented as valid.
- Restart app and reopen identical image; verify cache reuse.
- Export/import packages across different app versions with migration.

### Golden visual tests

Use a redistributable image corpus: faces/characters, text-heavy art, architecture, foliage, low-contrast photos, neon scenes, sky, abstract gradients. Snapshot shader outputs at known feature frames and compare against approved image tolerances. Add an automated motion-envelope check to detect excessive screen-wide displacement or luminance changes.

### Audio corpus

Include silence, pink noise, kick-only, snare-only, bassline, dense master, lo-fi track, ambient pad, classical, speech, notification-like blips, distorted clipping and quiet mastering. The purpose is robustness, not genre classification.

---

## 17. Delivery roadmap and gates

### Phase 0 — Repository and proof harness

- Create workspace and design tokens.
- Create a CLI to load a still image, generate deterministic palette/masks, and display analysis overlays.
- Create a standalone audio telemetry CLI fed by mock PCM and actual loopback.
- Create shader preview app window, not yet attached to desktop.
- Record CPU/GPU/memory baselines.

**Gate:** reproducible analysis, feature frames and preview rendering; no shell-window injection until the renderer and state model work.

### Phase 1 — Windows functional MVP

- Rust engine background process.
- Windows WASAPI system loopback.
- DSP multiresolution bands, onset flux, envelopes.
- `wgpu` image shader with subtle breathing, localized glow and optional sparse particles.
- Windows desktop-host adapter with Explorer recovery.
- Tauri Studio quick mode and tray commands.

**Gate:** user can choose any supported still image and see it react to system audio; desktop icons remain clickable; fallback works on Windows 10 build 19042 without process-specific capture.

### Phase 2 — Image intelligence

- Model adapter and deterministic fallback.
- Benchmark DA2 Small ONNX against DA3 Small native C API.
- Depth/confidence maps, masks and cache.
- User protect brush and focal point.
- Vibe recipe auto-selection.

**Gate:** automated image corpus passes edge/protection checks; no claim of 3D or semantic understanding.

### Phase 3 — Smart audio sources and polish

- Windows process-specific loopback where supported.
- Auto source scoring, app lock, ignore list.
- Better transient features and optional tempo confidence.
- Game/fullscreen/battery policies.
- Diagnostics and performance-quality settings.

**Gate:** no notification-induced repeated activation; device/process changes recover; DSP does not allocate in callback.

### Phase 4 — macOS

- Core Audio process taps and permissions.
- Native desktop-level wallpaper surfaces.
- Metal through wgpu.
- App notarization, signing and updater.

**Gate:** permission and system lifecycle tests complete on real Macs; only then declare minimum OS and app-specific capture support.

### Phase 5 — Linux

- KDE Plasma 6 native wallpaper integration.
- PipeWire system monitor and app nodes; PulseAudio fallback.
- X11 host where maintained.
- wlr-layer-shell backend for supported compositors only.

**Gate:** publish a desktop/compositor compatibility table; don't claim universal Linux Wayland support.

### Phase 6 — Steam and creator ecosystem

- Steamworks integration, Workshop publish/subscribe.
- `.ddpkg` compatibility and package signing/hash verification.
- Creator metadata, tags, preview generation and moderation tools.
- Optional Steam Cloud for lightweight settings.

**Gate:** Steam API, workshop legal agreement, content rights flow, package migration and update rollback verified.

---

## 18. Architecture decisions to preserve

1. Rust owns runtime engine, capture, DSP, image processing, renderer, platform adapters and IPC.
2. Tauri/React is only the Studio UI; it is not the wallpaper rendering surface.
3. Model inference is offline and one-shot per image analysis; no model inference occurs in the audio render loop.
4. AI depth is optional and fallible. The core product must work with deterministic analysis only.
5. One source image + analysis maps + recipe form a scene; do not generate replacement artwork.
6. Capture capability is reported accurately per OS build / permission / desktop environment.
7. Use native desktop host adapters; do not claim a universal background-window API.
8. Audio callback is allocation-free, lock-free where practical, and bounded.
9. The renderer runs at a capped rate and pauses when not useful.
10. Performance claims are published only from reproducible benchmarks.
11. Model weights and all third-party dependencies must pass commercial-license review before bundling.
12. Keep the system small, modular and testable; do not build a plugin scripting system until the core workflow is proven.

---

# Addendum A — Ambient Desktop Experience

**Status:** Planned milestone; not implemented. This addendum extends the existing architecture without removing or weakening any original requirement. Implementation begins only after the Windows MVP, image analysis/cache, engine IPC and desktop-host lifecycle have passed stabilization.

## A1. Product intent and boundaries

DiscoDesk should become an unobtrusive desktop companion. After initial setup, users should not need to open Studio every time their OS wallpaper changes. The engine observes wallpaper changes, reuses cached image analysis when possible, prepares a safe default visual recipe and continues reacting to the selected audio source.

A small **Magic Wand** quick action should be available on the desktop where the OS permits a safe, native implementation. Clicking it re-evaluates the current wallpaper and applies the best available reaction without requiring the full Studio window.

Keep three distinct responsibilities:

1. Wallpaper observation: discover a possible OS wallpaper change.
2. Image analysis/cache: resolve the source, calculate or reuse image-derived masks/depth/palette.
3. Reaction policy: decide whether auto-reaction is enabled and which saved preset/profile applies.

A wallpaper change must never override manual pause, explicit user opt-out, per-display disable, per-wallpaper disable, excluded applications or reduced-motion settings.

## A2. Platform-neutral contracts and state machine

Add an isolated desktop-automation module or crate. It owns wallpaper-change observation, per-display state, quick-action commands and reaction-policy evaluation. It does not own audio capture, DSP or GPU rendering.

Suggested data contracts:
- DesktopWallpaperSnapshot: display_id, source_uri, wallpaper_kind, observed_at, optional source_revision.
- WallpaperKind: StillImage, Slideshow, Dynamic, SolidColor, Unknown.
- WallpaperChangeHint: optional display_id, reason and timestamp.
- WallpaperChangeReason: OsSettingEvent, RegistryOrConfigWatch, SlideshowTransition, DisplayTopologyChanged, PeriodicReconciliation, UserRequestedRefresh.
- AutoReactionMode: Off, AskOnNewWallpaper, ReactAutomatically.
- WallpaperReactionPolicy: enabled, mode, default preset, respect_manual_pause, per-display overrides and excluded wallpaper hashes.

The watcher emits a bounded **change hint**, not a trusted file path or decoded image. Re-query platform state authoritatively, per monitor, after receiving the hint.

Required state machine:

Watching -> ChangeHint -> DebounceAndCoalesce -> QueryPerDisplaySnapshots -> WaitForStableSource -> ValidateAndHash -> CacheLookup -> LoadCachedRecipe OR DeterministicAnalysisAndQueueDepthJob -> EvaluateUserPolicy -> ApplyPreset -> Active / Disabled / NeedsAttention.

Requirements:
- Coalesce duplicate events for the same display/source. Never queue one analysis job per repeated notification.
- A newly observed image may still be copied or rewritten. Verify that the source has stabilized before decoding.
- If source is missing, corrupt, unsupported, inaccessible, virtual, cloud-only or dynamically generated without a stable image, preserve the OS wallpaper and show a quiet status.
- Never modify, rename, move, re-encode or delete the source wallpaper.
- No network image fetch or cloud inference.
- Auto-reaction is an explicit preference. Explain it during onboarding; do not silently enable it.
- Manual pause and per-display/per-wallpaper opt-out take precedence.
- If depth analysis is pending, use a conservative deterministic 2D recipe. Blend in depth later without a jump.
- All work is cancellable, bounded and joined on shutdown, logout or session teardown.

## A3. Content-addressed analysis cache

Use image content identity, not filename identity. Recommended cache identity combines SHA-256 of image bytes with decoder schema, analysis schema, exact model/checkpoint hash and preprocessing version.

Keep image-derived maps separate from monitor-specific render transforms. The same image can be reused on several displays with different aspect ratios and safe motion margins.

Cache may contain normalized image metadata, optional quantized depth/confidence maps, luminance/highlight/edge/saliency masks, particle-safe regions, palette, user protection masks, analyzer provenance and cache accounting.

Cache requirements:
- Atomic staged writes and validation before final rename.
- Reject partial, malformed, mismatched-version and hash-invalid entries.
- Bounded disk budget and LRU eviction; never evict active or user-pinned entries.
- User controls for pin, reanalyse, clear and remove an analysis.
- Version invalidation must preserve source artwork and saved user presets.
- Cache failure falls back to reanalysis; it never prevents the original wallpaper from displaying.
- Do not store captured PCM or send image information to cloud services.

## A4. Windows wallpaper observer

Windows is the first implementation target.

Use the documented IDesktopWallpaper COM interface to enumerate monitors and query the wallpaper per monitor. GetWallpaper(NULL) can return S_FALSE when monitors use different images or a slideshow is running; do not assume a single global wallpaper string.

Combine multiple signals:
1. WM_SETTINGCHANGE as a best-effort system-setting hint.
2. A narrowly scoped registry change watch as a supplementary hint where justified.
3. Low-frequency authoritative per-monitor reconciliation.
4. Reconciliation after display topology, session, sleep/resume and relevant shell events.

A Microsoft Q&A response dated 11 February 2026 reports that some Windows 11 Settings wallpaper changes may not broadcast WM_SETTINGCHANGE and suggests registry monitoring plus IDesktopWallpaper re-query. That is support-forum evidence, not a documented Windows contract. Event-only observation is therefore insufficient; conservative reconciliation is mandatory.

Test standard image selection, per-monitor image, slideshow, Windows 11 Settings, theme/personalization changes, Spotlight-like source, sleep/resume, Explorer restart and display hotplug.

## A5. Magic Wand desktop surface

The wand must be a separate, tiny native quick-action surface. It is not the wallpaper renderer, a Tauri always-on-top window or a full-screen transparent overlay.

Windows prototype:
- Investigate a small, borderless, layered top-level window with no-activate/tool-window behavior, such as WS_EX_NOACTIVATE and WS_EX_LAYERED.
- Show only when the desktop is the visible active surface; hide behind ordinary applications, secure desktop, lock screen and fullscreen apps.
- The hit-test region must be limited to the visible button. It must not intercept clicks outside the control.
- Never steal focus, cover the taskbar or make the wallpaper surface topmost over ordinary windows.
- Support dragging/repositioning and selectable corners. Default to lower-right safe inset above the taskbar.
- Recover or hide safely on Explorer restart, monitor hotplug, DPI change, rotation and sleep/resume.
- Multi-monitor instances require explicit user choice and must not duplicate one global action.
- Provide tray, keyboard and accessible alternatives. A no-activate overlay must not be the only way to invoke the feature.
- If z-order or click behavior cannot be guaranteed, ship tray/keyboard quick action and defer the visual overlay.

Windows Widgets are not a substitute for this exact UI: the official Widgets Board is a separate flyout opened by taskbar, Win+W or swipe, not a permanently pinned desktop button.

## A6. macOS and Linux capability strategy

macOS: query each screen using NSWorkspace.desktopImageURL(for:) on the main thread. The reviewed public sources establish a per-screen query API but do not establish a universal wallpaper-changed notification for every target macOS release. Use conservative reconciliation unless a documented public event is verified. Prefer a native menu-bar extra as the first always-available quick action. Desktop-level overlay is optional only after Spaces, Finder, focus, accessibility and fullscreen tests.

KDE Plasma: investigate a native Plasmoid or wallpaper-provider integration. KDE documents desktop widgets and wallpaper plugins as native extension points. Keep this as a platform adapter.

GNOME, Xfce and other X11/Wayland desktops require separate capability probes and desktop-specific adapters. A tray/menu action is the fallback. Never claim universal Wayland support based on one compositor protocol.

Every platform reports wallpaper observation, per-monitor identity, quick-action surface, hit testing and notification capabilities. The UI must not expose unsupported controls.

## A7. Wand UX and interaction

The wand is a small, quiet entry point. Clicking it:
1. Re-queries the current wallpaper on that monitor.
2. Resolves its content hash and cache.
3. Applies the saved per-wallpaper profile or user default on cache hit.
4. On cache miss, starts deterministic analysis and optional local depth inference.
5. Uses the 2D fallback while depth analysis runs.
6. Reuses current eligible audio source and fades into the response.
7. Shows a compact state: Reacting, Analysing, Paused, Unsupported, Needs attention.

Popover actions: Make React, Pause/Resume, Change Vibe, Exclude Wallpaper, Open Studio and Hide Wand.

Do not reset manual configuration on wand click. Full Studio opens only when detailed tuning is requested.

## A8. Presets and creator ecosystem

Make saved vibes a first-class capability. Separate:

- **Vibe preset (.ddpreset):** portable audio-feature-to-effect mappings, envelopes/ballistics, effect strengths, comfort profile, palette behavior and metadata. Does not require the creator's wallpaper.
- **Wallpaper package (.ddpkg):** artwork plus optional analysis assets and wallpaper-specific profile. Sharing artwork requires explicit intent and distribution rights.

Preset schema includes stable ID, version, name, creator, description, tags/category, input features and normalization, mapping curves, allowlisted renderer effects, amplitude bounds, attack/release, smoothing, particle limits, motion/luminance comfort metadata, reduced-motion fallback and preview metadata.

Users can create, duplicate, rename, tag, favorite, import, export, migrate and delete presets. Provide simple quick mode and advanced live mapping editor.

Workshop packages are validated declarative data only. Do not permit arbitrary DLLs, executables, native plugins, unrestricted scripts or user-supplied WGSL in the initial format.

Validate archive paths, hashes, decompressed sizes, entry counts, schema, effect IDs, parameter ranges, version compatibility and metadata lengths before applying content. Local preset operation must work with Steam absent/offline. A .ddpreset must not include a creator's wallpaper. A .ddpkg with artwork requires explicit rights confirmation.

Support creator identity, attribution, version history, remixes/forks, tags and optional license metadata. Do not force a particular genre or visual aesthetic.

## A9. Smart local notifications

Add a local notification policy service. Events never need to be sent to a cloud service.

Classes:
- Silent: wallpaper changed and cache hit; update wand/status only.
- Informational: new analysis completed; optional, off by default for slideshow.
- Action required: audio permission denied or selected source unavailable.
- Recoverable: model failed but deterministic fallback is active; notify once.
- Critical: wallpaper host cannot recover; notify once with recovery action.

Deduplicate by event type and stable subject ID. Apply per-event cooldown, quiet hours, global mute, per-monitor controls and a notification budget. Never notify every slideshow transition, cache hit, audio transient or repeated error. Keep a local status history for events suppressed by quiet hours.

Windows: investigate Windows App SDK AppNotificationManager for local desktop notifications. macOS: UserNotifications. Linux: freedesktop Desktop Notifications D-Bus. All actions use validated IPC and cannot execute arbitrary shell/package content.

Smart detection should surface meaningful changes such as source loss, wallpaper changes, fullscreen pause, display/renderer fallback, cache pressure or required user action. Prefer a status indicator to a toast.

## A10. Calm Studio UX

Studio should feel soothing, premium, modern and integrated—not like a constantly animated gaming dashboard.

- Neutral surfaces, comfortable dark/light themes, strong typography hierarchy and one controlled accent family.
- Wallpaper-derived palette may tint previews and small accents only; it must not make text contrast unpredictable.
- Use Mica for appropriate opaque Windows 11 surfaces and Acrylic for transient flyouts where supported; provide solid/blurred fallback elsewhere.
- Avoid excessive glow, noisy gradients, giant all-caps labels and decorative animation.
- Home answers: current wallpaper, audio source, reaction state and pause action.
- Library provides wallpaper/preset browsing, search, tags, favorites and cache status.
- Vibe Studio groups Motion, Depth, Light, Particles, Audio Response, Comfort and Advanced Mapping.
- Audio shows the actual capture capability; Performance explains why rendering is paused/throttled.
- Respect OS theme, high contrast, keyboard navigation, visible focus, screen readers, scaling and reduced motion.
- Status is never conveyed by color or animation alone.
- Permanent Pause/Stop and reduced-motion controls remain easy to locate.

This follows Fluent 2 guidance on purposeful/natural motion and platform materials, Apple HIG motion guidance, and WCAG flash requirements. WCAG thresholds are minimum conformance references, not a universal comfort guarantee.

## A11. Milestones

**M1 — Working Windows MVP prerequisite:** Gates 0–6 pass, real wallpaper reacts to real system audio, image cache works, engine runs independently from Studio, pause/reconnect/recovery pass, and a beta soak-test report records CPU/GPU/RAM/frame-time/crash/recovery evidence.

**M2 — Ambient Desktop Experience:** per-monitor observer, content-hash cache reuse, background analysis, opt-in reaction policy, Windows wand prototype, tray/keyboard fallback, quick popover, native notifications, slideshow/multi-monitor/Explorer/fullscreen/icon tests.

**M3 — Creator ecosystem:** local preset authoring/import/export, .ddpreset migration/validation, Steam Workshop upload/download, attribution/remix/version/license metadata and explicit artwork rights controls.

Wand is deliberately scheduled after M1. It must not destabilize the desktop before the native wallpaper host and cache are proven.

---

# Addendum B — implementation status

Wallpaper observer, automatic reaction, Magic Wand, smart notifications and Workshop preset authoring are specified but NOT implemented. Keep all their acceptance boxes unchecked until actual code and real OS tests exist.
