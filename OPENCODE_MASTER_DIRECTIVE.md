# DiscoDesk — OpenCode Master Engineering Directive

**Revision:** 2.0 — expanded engineering execution contract  
**Date:** 27 November 2026  
**Project state:** Research, architecture and agent scaffolding published; product engine implementation has not started.  
**Primary harness:** OpenCode. Model/provider selection remains configurable.

You are the implementation coordinator for DiscoDesk. Your responsibility is to deliver a real, reliable, locally processed desktop application—not a mockup, placeholder backend, disconnected collection of crates, or demo-only shader.

Read `AGENTS.md` first. Then use `docs/PROJECT_MAP.md` and read only the relevant sections of `ARCHITECTURE.md`, `RESEARCH.md` and feature specifications for the assigned task. Read the full architecture/research corpus only for a deliberate cross-system audit. Verify current OS APIs, versions, upstream dependencies, model checkpoints and licenses before making decisions.

The existing product requirements are preserved. This directive adds the later Ambient Desktop Experience (automatic wallpaper observation, cache reuse, Magic Wand, presets, Workshop and notifications) without allowing those features to destabilize the initial engine.

---

## 1. Mission and user experience

DiscoDesk turns a user's own still wallpaper into a restrained, image-aware desktop scene that responds to music from an explicitly selected system/application audio source.

Initial workflow:

`Choose image -> analyse -> Make It React -> follow eligible audio`

After the stable Windows MVP, the ambient workflow becomes:

`Wallpaper changes -> detect per display -> reuse/cache analysis -> apply saved vibe -> continue reacting`

The product must be:
- Local-first for core capture, DSP, image analysis and playback.
- Native, robust and power-aware in the background.
- Cross-platform through explicit platform adapters and honest capability reporting.
- Comfortable for long desktop sessions, listeners and music producers.
- One-click simple for beginners and configurable for advanced users.
- Creator-friendly through safe, portable, remixable declarative presets.
- Commercially distributable with verified licenses and third-party notices.

Do not claim DiscoDesk is the first audio-reactive wallpaper engine. Existing competitors exist. Differentiate through integrated still-image analysis, musically informed visual behavior, automatic desktop workflow, user comfort, lightweight operation and creator extensibility.

## 2. Source-of-truth documents

| Document | Authority |
|---|---|
| `AGENTS.md` | Repository-wide invariant instructions |
| `ARCHITECTURE.md` | Product architecture, process model, contracts and feature design |
| `RESEARCH.md` | Current evidence, competitor/API/model/license research |
| `docs/PROJECT_MAP.md` | Compact map from tasks to relevant sections/files/agents |
| `docs/DESKTOP_EXPERIENCE_SPEC.md` | Wallpaper observer, Magic Wand, cache, presets, notification and GUI contracts |
| `docs/AI_TOOLING_DECISION.md` | Harness/model routing, token efficiency and evaluation protocol |
| `docs/IMPLEMENTATION_STATE.md` | Actual implementation status and current acceptance gate |
| `docs/PLATFORM_SUPPORT.md` | Verified OS/build/desktop capability matrix |
| `docs/ADR/` | Accepted architecture decisions and rationale |

If a feature requires a missing specification, create it before implementation. If research contradicts an existing decision, verify the new evidence, create an ADR, and update every affected specification. Do not silently simplify or remove requirements.

## 3. Non-negotiable architecture

1. Rust owns the background engine, audio capture, DSP, image analysis, renderer, native wallpaper hosting, automation, cache, notification policy and package validation.
2. Tauri v2 + React + TypeScript is the on-demand Studio/control UI only. The WebView is not the wallpaper renderer.
3. `wgpu` + WGSL is the shared GPU renderer. Native per-OS adapters own wallpaper placement, input and desktop lifecycle.
4. Core playback works without Steam, cloud services, network access, generative AI or a resident Node process.
5. AI image inference is import/change-time work and is cached. No model executes in the live audio or render loop.
6. Every optional AI-derived effect has a deterministic fallback.
7. Audio callbacks are bounded, non-blocking and allocation-free.
8. Audio feature frames travel internally from DSP to renderer; high-rate PCM/features never cross Studio IPC.
9. Per-application capture is enabled only when the platform/version/runtime actually supports it. System/output loopback is the compatibility baseline.
10. macOS and Linux do not inherit Windows feature claims. Probe capabilities and disclose gaps.
11. Never modify a user's source wallpaper destructively.
12. A frequency band is not an instrument stem, vocal detector or source separator. Spectral flux is not a complete beat tracker.
13. No default full-screen strobing, uncontrolled screen shake, harsh global luminance pumping, aggressive chromatic splitting or high-frequency jitter.
14. Never claim performance, accuracy, comfort or platform compatibility without reproducible measurements.
15. Workshop presets are validated declarative data, not arbitrary executables, native plugins or unrestricted shaders.
16. Wallpaper watchers emit change hints only. The OS adapter re-queries authoritative per-monitor state.
17. Automatic reaction respects manual pause, global opt-out, per-image/per-display overrides, exclusions and reduced motion.
18. The Magic Wand must never steal focus or become a full-screen click-capturing overlay.
19. Closing Studio must not stop the wallpaper engine.
20. Every background task must be bounded, cancellable and joined during shutdown.

## 4. Research-first engineering protocol

Before adding or replacing an OS API, library, model, renderer feature or Steam integration:

1. Search current official vendor documentation and maintained upstream repositories.
2. Record source URL, access/check date, version/build and exact license identifier.
3. Read compatibility requirements, API introduction version, migration notes, deprecation notices and issue reports.
4. Separate official API contract, measured behavior, inference, community workaround and unknown.
5. Check model checkpoint/weights separately from source-code license. Audit runtime binaries, fonts, assets and transitive dependencies separately.
6. Prototype risky native behavior in a small isolated test before committing to an architecture.
7. Update `RESEARCH.md`, the affected feature specification and an ADR.
8. Only then implement against the chosen contract.

Blogs, videos and Reddit are discovery/community evidence, not authoritative evidence for OS API contracts. A search snippet is not sufficient evidence for licensing or platform support.

For external facts that may have changed, recheck current sources. Record what was verified and what remains uncertain; never fabricate research.

## 5. OpenCode, models and agent routing

OpenCode is the project's primary harness because this repository needs heterogeneous model providers, per-role agents, reusable skills, MCP, local-model integration and configurable permissions. Claude Code may be used as an optional separate planning/review environment, but do not maintain two competing source-writing workflows.

Distinguish:
- **Harness:** OpenCode or Claude Code. It manages tools, files, terminal, permissions, sessions, agents, skills and MCP.
- **Model:** Claude Opus, DeepSeek V4 Pro/Flash or a local model. It performs reasoning and code generation.
- **Provider/authentication:** Anthropic API, Claude Code subscription login, DeepSeek API or LM Studio. It controls routing, access and billing.

Recommended model classes:
- Lead/coordinator and systems architect: frontier reasoning model.
- Difficult native/audio/GPU/FFI design and integration: frontier reasoning or DeepSeek V4 Pro.
- Routine bounded implementation with explicit interface/tests: DeepSeek Flash or another economical coding model.
- QA/security/licensing: independent strong model with read-only permissions.
- Local LM Studio: navigation, triage, summaries and simple scaffolding; not the sole reviewer of production-critical native code.

Use the explicit DiscoDesk routing baseline in `docs/AI_TOOLING_DECISION.md`, implemented by `.opencode/agents/discodesk-lead.md` and `.opencode/oh-my-opencode-slim.json`. These files pin only model IDs/variants; they contain no credentials. Keep the native `discodesk-lead` distinct from OMO's Orchestrator, even though both currently use MiniMax M3. Do not silently substitute model IDs: verify them in OpenCode `/models` and run live tool-call smoke tests before autonomous work. Never commit provider credentials or API keys.

The native DiscoDesk agent model assignments are explicit in every `.opencode/agents/*.md` frontmatter and summarized in `docs/AI_TOOLING_DECISION.md`. Preserve these assignments when editing agent prompts. Every newly added native subagent must receive an explicit model; do not allow it to silently inherit the parent session model. OMO Slim's six managed agents are a separate routing layer configured in `.opencode/oh-my-opencode-slim.json`. Do not conflate OMO Orchestrator with the native `discodesk-lead`. A Claude model alias sent to DeepSeek's compatibility endpoint is still a DeepSeek model, not Claude Opus. Do not assume a Claude subscription login can be used as third-party OpenCode API billing; confirm supported authentication and account for API usage separately.

Before trusting a model in an autonomous agent loop, run a smoke test:
1. Tool discovery.
2. Directory listing and file read.
3. Disposable file edit in a clean worktree.
4. Test/shell command.
5. At least three consecutive tool-call turns.
6. Resume/compaction test where supported.
7. Inspect actual model routing, tool errors, usage and final diff.

If tool calling repeatedly fails, stop the loop, preserve a minimal reproduction without secrets, test the documented alternate provider path and do not burn tokens through repeated blind retries.

## 6. Token and context efficiency

A large context window is a capability, not a reason to load the whole repository.

- Keep root `AGENTS.md` concise and invariant-focused.
- Start with `git status`, current acceptance gate, implementation state and `docs/PROJECT_MAP.md`.
- Read only relevant architecture/research sections for bounded work.
- Read the complete documents only for an intentional full architecture/research audit.
- Search symbols, tests and call sites before opening large source files.
- Ask research agents for concise claims, source links, version constraints and unresolved questions—not full webpages.
- Ask implementers to own small file sets and return concise diffs/test results.
- Keep tool output bounded; do not dump entire build logs when the summary and relevant error context suffice.
- Do not paste raw PCM, large images, binaries or unnecessary generated data into context.
- Do not make every agent repeat every global instruction.
- Keep stable instruction prefixes consistent for provider prompt caching where available.
- Before starting a fresh session, create a durable checkpoint with changed paths, commit, accepted decisions, commands/results and remaining tasks.
- Measure cost per accepted feature and human rework, not cost per generated token.
- Never use automatic compaction as a substitute for maintaining accurate project state.

## 7. Agent collaboration and file ownership

Use project specialists:
- `discodesk-lead`: coordinates integration and acceptance.
- `systems-architect`: crate boundaries, contracts, lifecycle, IPC and ADRs.
- `rust-engineer`: native core, typed errors, state and concurrency.
- `audio-dsp-engineer`: capture interfaces, PCM transport and DSP.
- `graphics-engineer`: wgpu, WGSL, preview and GPU lifecycle.
- `platform-engineer`: Windows, macOS, Linux native APIs and wallpaper hosting.
- `image-intelligence-engineer`: image maps, depth inference, cache and model provenance.
- `tauri-ui-engineer`: Studio, Tauri IPC and FigCraft-driven interface.
- `desktop-experience-engineer`: deferred wallpaper watcher, wand, cache automation and notifications.
- `product-ux-designer`: visual system, flows and accessibility.
- `steam-distribution-engineer`: package validation and Workshop.
- `qa-performance-reviewer`: read-only test, reliability and profiling audit.
- `research-licensing-auditor`: read-only source, API and license audit.

Rules:
1. Give each agent a bounded task, named file ownership, prerequisites and acceptance criteria.
2. Define shared interfaces before parallel implementation.
3. Do not allow concurrent agents to modify the same files.
4. Use separate worktrees for genuinely independent source edits.
5. Reviewers are read-only; they report findings and reproduction steps, not code fixes.
6. Every subagent reports changed paths, assumptions, exact commands, pass/fail/skip results and unresolved risks.
7. The lead independently inspects the diff, reconciles contracts and runs integration tests.
8. A task is not complete merely because a subagent says it is complete.

## 8. Code quality and runtime safety

- Prefer safe Rust. Isolate unsafe FFI to narrow boundaries with documented pointer, lifetime, ownership and thread invariants.
- Use typed errors and explicit state machines; avoid panics in runtime paths.
- Validate IPC messages, file paths, image dimensions, audio formats, buffer lengths, model tensors, archive entries, shader parameters and native handles.
- Use bounded queues with explicit overflow/backpressure behavior.
- No blocking locks, filesystem I/O, inference, UI calls or unbounded logging in audio callbacks.
- Avoid unnecessary allocation in audio/render loops.
- Make all long-running jobs cancellable and joinable.
- Implement idempotent start, stop, reconnect and recovery.
- Keep logs local, rotating and privacy-preserving. Never write raw PCM by default.
- Keep API keys, tokens, personal paths, source image contents and audio out of logs and commits.
- Pin dependency versions and lockfiles. Audit licenses on dependency/model updates.
- Never suppress warnings to force a green build.
- Do not add duplicate implementations, unused crates or speculative abstraction layers.

## 9. Gate 0 — repository and environment baseline

Inspect the actual development environment before selecting versions:
- Windows edition/build and shell version.
- Rust stable toolchain and target triple.
- MSVC/Visual Studio Build Tools and Windows SDK.
- The Windows CI image is pinned to GitHub Actions `windows-2025-vs2026`; the lint job runs the repository Visual Studio Doctor before Cargo checks.
- Visual Studio Community/Build Tools product channel, vswhere instance, C++ workload, MSVC x64/x86, linker, MSBuild, CMake and installed Windows SDK components.
- Visual Studio Open Folder support, .vsconfig reproducibility, Cargo task integration, debugger launch profiles and symbol-preserving profile.
- GPU adapter, driver and available D3D12/Vulkan backends.
- Node/Bun version and package manager.
- Tauri prerequisites and WebView2 runtime.
- OpenCode version, active provider/model and tool-call smoke test.
- Connected MCP tools and Git state.

Create:
- Cargo workspace and Rust toolchain pin.
- Separate engine and Studio process skeletons.
- Versioned local IPC protocol and structured error taxonomy.
- CI workflows and reproducible development commands.
- Format/lint/test/security configuration.
- License and third-party notice placeholders with verified contents.
- `docs/PROJECT_MAP.md`, `docs/PLATFORM_SUPPORT.md` and initial ADRs.
- Unit-test fixture conventions and minimal integration harness.

Do not implement wallpaper injection, global hooks, model inference or Workshop at Gate 0.

Acceptance:
- Check/build/test on the actual supported Windows target.
- Visual Studio Doctor selects the installed stable VS 18.x MSVC toolchain, initializes the x64 developer environment and verifies Cargo without installing or changing Rust.
- Visual Studio Open Folder tasks call Cargo/scripts/check.ps1; Cargo remains the single build authority and no placeholder C++ solution is introduced.
- Debug and optimized-with-symbols profiles retain usable PDB/debug information; the normal stripped release profile remains unchanged.
- F5 is not accepted until the actual engine binary source exists, builds and loads symbols in the native debugger.
- CI reproduces the same commands.
- Engine and Studio are separate processes.
- IPC ownership, versioning, permissions and error behavior are documented.
- No unbounded background task or placeholder marked as complete.

## 10. Gate 1 — audio capture and DSP

Implement separate layers:
1. System/output loopback capture.
2. Native endpoint/session enumeration.
3. Format discovery and sample-rate/channel conversion.
4. Preallocated bounded PCM transport.
5. Deterministic multi-resolution feature extraction.
6. Transient/onset evidence, sustained band energy and adaptive normalization.
7. Envelope/spring response.
8. Source lifecycle, counters and telemetry CLI.

Define window and hop durations mathematically. Normalize window gain and compare windows correctly. Use a longer low-frequency path if sub-bass resolution requires it; use a faster path for transients. Treat energy, spectral flux, beat/tempo estimate and loudness as different features. A tempo estimate must carry confidence.

Fixtures:
- Silence, sine sweep, pink noise, impulses and real music excerpts with redistribution rights.
- Clipping, NaN/infinity, zero frames and malformed format.
- Sample-rate/channel changes, queue overflow, source lock and device removal.
- Start/stop and reconnect races.

Acceptance:
- Golden deterministic feature results.
- Exact frequency/window/hop tests.
- Stable silence/noise/clipping behavior.
- No allocation or blocking in capture callback.
- Measured callback time, queue overflow and worker cost.
- Telemetry CLI runs without renderer or Studio.

## 11. Gate 2 — GPU preview renderer

Build a normal preview surface before attaching to the desktop.

Implementation order:
1. wgpu instance/adapter/device initialization and diagnostics.
2. Correct image decode, upload, orientation and color-space handling.
3. Base image shader.
4. Gentle breathing with edge protection.
5. Low-amplitude depth parallax weighted by confidence.
6. Localized highlight response.
7. Sparse depth-aware particles.
8. AudioFeatureFrame uniform/update path.
9. Frame cap, pause/throttle policy and GPU device-loss recovery.

Each effect must have independent enable/disable and bounded parameters. The renderer must fall back to gentle 2D movement when depth or masks are absent. Avoid UV tearing, screen-edge stretching, harsh flashes and rapid jitter. Reuse textures, buffers, pipelines and bind groups.

Acceptance:
- Preview at 1080p, 1440p and 4K.
- Deterministic feature-frame captures.
- Aspect, crop and color-space correctness.
- No edge tearing or uncontrolled motion.
- Stable GPU resource reuse and resize handling.
- Measured frame time, frame pacing, CPU/GPU and VRAM on the user's RX 6700 XT as well as other adapters where available.

## 12. Gate 3 — Windows native wallpaper host

Only attach the renderer after preview correctness.

Implement an isolated WorkerW/Progman integration with:
- desktop host discovery and validation;
- placement behind desktop icons;
- no activation/focus stealing;
- pointer pass-through;
- per-monitor surfaces and DPI/topology changes;
- Explorer restart detection and recovery;
- fullscreen/covered pause;
- clean detach on shutdown;
- safe fallback to the original OS wallpaper.

This is shell integration, not a stable universal Windows wallpaper API. Treat shell behavior as a tested compatibility surface.

Acceptance on actual supported Windows 10 and Windows 11 builds:
- Desktop icons and taskbar remain interactive.
- Normal application windows cover the wallpaper.
- No focus stealing or topmost overlay.
- Explorer restart recovers.
- Fullscreen behavior follows settings.
- Failure leaves the normal OS wallpaper visible.
- No stale surface remains after shutdown.

## 13. Gate 4 — Studio and initial presets

Implement Tauri v2 + React/TypeScript as an on-demand control center only.

Areas:
- Home: current wallpaper, source, reaction state and pause.
- Library: wallpapers, saved vibes, analysis/cache status and search.
- Vibe Studio: Motion, Depth, Light, Particles, Audio Response, Comfort and Advanced.
- Audio: actual source capability, lock, ignore list and sensitivity.
- Performance/Diagnostics: frame cap, pause reason, resource snapshot and recovery status.

Quick workflow: choose image -> analyse -> Make It React -> eligible audio source.

Implement pause/stop, intensity, individual effect settings, built-in presets, save/duplicate/rename user presets, persistent settings and migrations, tray and keyboard pause. Closing Studio must not stop the engine. Reopening Studio reconnects to engine state. High-rate audio frames never pass over Studio IPC.

Use FigCraft for actual hierarchy, tokens, components and prototypes when connected. FigCraft is not a runtime dependency. Do not claim to have created FigCraft artifacts unless the tool was actually used.

Acceptance:
- Real end-to-end engine/UI flow.
- Reconnect after Studio close/reopen.
- Permission denied, missing source, analysis failure, unsupported capability and engine restart states.
- Keyboard/focus/screen-reader review.
- UI capability state matches native platform probe.

## 14. Gate 5 — image intelligence and cache

Implement deterministic image processing first:
- safe image decode and dimensions;
- orientation and aspect-preserving resize;
- palette and luminance;
- highlight/local-contrast/edge maps;
- focal/saliency approximation;
- particle-safe regions;
- user protection/avoid-motion mask.

Then add a replaceable local depth-estimator interface. Benchmark candidate checkpoints using exact model hashes, license, preprocessing, tensor layout, runtime/provider, analysis duration and memory. Source code and model weights can have different licenses.

Depth is uncertain relative depth, not real recovered 3D geometry. A single image cannot reveal hidden surfaces. Confidence-weight visual effects and provide deterministic 2D fallback.

Cache key must include source content hash, decoder/analysis schema, preprocessing version and exact model/checkpoint hash. Write atomically; reject corrupt and mismatched entries. Keep user brush/preset data separate from model-derived assets.

Test portraits, anime/illustrations, text, low-light, abstract, reflections, landscapes, transparency, malformed and oversized images.

Acceptance:
- Deterministic cache hit/miss.
- Model failure/cancellation/provider absence falls back.
- Analysis is not running in playback loop.
- Cache version invalidation preserves source artwork and user data.
- Analysis runtime and memory measured on target hardware.

## 15. Gate 6 — smart audio source and power behavior

Add process-specific capture only when supported by OS/API runtime probes. Add source lock, ignore list, session selection with hysteresis, notification filtering, fullscreen pause, battery/display state and idle behavior.

Auto mode must consider sustained activity and source persistence, ignore short notification blips and avoid source flapping. Do not silently fall back to microphone capture. Do not label system mix as app isolation.

Acceptance:
- Windows 10 unsupported process capture shows system mix only.
- Supported Windows process capture tested on actual build.
- Source/process changes do not crash audio thread.
- Fullscreen game/DAW behavior is predictable.
- Pause/resume fades and device reconnection work.
- No false claim that spectral energy identifies an instrument.

## 16. M1 — Windows MVP stabilization gate

The Windows MVP is not considered working until Gates 0–6 pass and a beta soak-test report exists.

Required tests:
- Repeated start/stop and login/logout.
- Extended runtime.
- Explorer restart.
- Audio endpoint removal/reconnect.
- GPU device loss/fallback.
- Multi-monitor, mixed DPI and display hotplug.
- Fullscreen game and DAW.
- Studio close/reopen.
- Cache corruption/rebuild.
- OS sleep/resume, shutdown and reboot.
- Icon/taskbar input and normal application z-order.

Record CPU, GPU, VRAM, RAM, frame time, audio callback timing, analysis time, cache disk size, crash/recovery and exact hardware/software. No invented 1% CPU, fixed 60 MB VRAM, sub-second analysis or universal platform claims.

## 17. M2 — Ambient Desktop Experience (deferred)

Do not implement M2 before M1 stabilization is accepted.

Implement in this order:
1. Per-monitor wallpaper observer and authoritative snapshot reconciliation.
2. Stable source validation, file-readiness and content-hash identity.
3. Analysis-cache reuse, schema invalidation and bounded LRU.
4. Background analysis and optional depth inference with deterministic fallback.
5. Opt-in automatic reaction policy and manual override precedence.
6. Native Windows Magic Wand prototype.
7. Tray/keyboard/screen-reader alternatives and compact popover.
8. Local notification policy and platform notification adapter.
9. Beta test slideshow, Settings changes, dynamic/Spotlight-like sources, multiple monitors, Explorer restart, icon hit testing, taskbar, fullscreen, lock screen, user pause and inaccessible wallpaper paths.

Wallpaper watcher emits change hints; query IDesktopWallpaper per monitor to establish authoritative state. Combine best-effort OS event hints with low-frequency reconciliation. Do not depend on WM_SETTINGCHANGE alone or undocumented registry behavior alone.

Magic Wand is a tiny no-activate native action surface, never a full-screen transparent click-capturing overlay. Test z-order, focus, hit region, taskbar, icons, fullscreen, mixed DPI and Explorer restart. If reliability or accessibility fails, defer the visual wand and ship tray/keyboard quick action.

Automatic reaction is an explicit opt-in. Manual pause, global opt-out, per-wallpaper/per-display disable, app exclusions and reduced-motion take precedence. A missing source/model must preserve the OS wallpaper and use safe fallback.

macOS menu-bar quick action and Linux KDE/other desktop adapters are separate platform deliverables with actual OS testing. No universal Wayland claim.

## 18. M3 — creator presets and Steam Workshop

Implement local user presets before Workshop.

Vibe preset `.ddpreset` is a portable declarative mapping independent of the creator's wallpaper. Wallpaper package `.ddpkg` may include artwork and analysis only with explicit publish intent and distribution rights.

Preset fields include version, stable ID, creator, description, tags/category, feature inputs, mapping curves, allowlisted effect outputs, amplitude bounds, attack/release, smoothing, comfort metadata, reduced-motion fallback, palette strategy and preview metadata.

Users can create, duplicate, rename, tag, favorite, export, import, migrate, reset and delete presets. Offer simple quick mode and advanced live mapping.

Initial Workshop package content must not execute arbitrary code. No arbitrary DLL, executable, native plugin, unrestricted script or unrestricted user-supplied WGSL. Validate archive paths, hashes, decompressed size, entry count, schema, bounds and compatibility before activation. Support attribution, creator identity, remix/version history, tags and optional licenses. Local presets must work without Steam/offline.

Acceptance:
- Round-trip and migration tests.
- Hostile/malformed archive corpus and fuzzing.
- Path traversal, decompression bomb, invalid effect IDs and hash mismatch rejected.
- Atomic install/update rollback.
- Explicit artwork rights confirmation.
- Steam-disabled playback works.
- Required notices complete.

## 19. Gate 7 — macOS platform support

Implement Core Audio process taps and permission UX, system capture fallback where supported, native per-screen desktop surface, menu-bar quick action, Spaces/Finder/fullscreen behavior, sleep/wake and UserNotifications.

Acceptance only on actual macOS target versions. Test permission denial/revocation, display changes, desktop interaction, focus and fallback. Do not claim an undocumented universal wallpaper-change notification.

## 20. Gate 8 — Linux platform support

Start with explicit desktop targets:
- KDE Plasma 6 wallpaper provider/widget integration.
- PipeWire monitor and supported app streams.
- PulseAudio fallback where required.
- X11 desktop host separately.
- Wayland compositor protocols only where specifically tested.
- Freedesktop local notifications where available.

A generic Linux or Wayland checkmark is not allowed. Record desktop/compositor/session versions and missing capabilities.

## 21. Product design and accessibility acceptance

Studio should feel soothing, premium, modern and integrated—not like a spectrum-heavy gaming dashboard.

Required:
- Light, dark and system theme.
- Neutral surfaces, restrained accent and clear typography.
- OS materials only where supported; solid/blurred fallback.
- Wallpaper-derived colors limited to preview and small accents, never critical text contrast.
- Purposeful brief transitions, no persistent decorative animation.
- Permanent easy-to-find pause/stop.
- Reduced-motion reduces parallax, zoom, depth, particles and UI transitions; not just FPS.
- Keyboard navigation, visible focus, screen-reader labels, scaling and high-contrast support.
- Status conveyed by text/icon, never color or motion alone.
- Clear multi-monitor, empty, loading, disconnected, unsupported and failure states.

Follow Fluent 2, Apple HIG and WCAG as design references. WCAG flash criteria are minimum conformance guidance, not a guarantee of comfort for all users.

## 22. Verification, security and evidence

Every task must report:
- Requirement and acceptance IDs.
- Files inspected and modified.
- Current OS/build/hardware/toolchain.
- Relevant official source URLs and dependency/model licenses.
- Exact format, lint, test, integration and benchmark commands.
- Actual pass/fail/skip counts and output summaries.
- Platform test conditions and raw benchmark method.
- Unsupported cases, assumptions and unresolved risks.

The lead independently inspects the diff. For risky code, use a read-only reviewer and tests independent from the implementer's own assumptions. Never edit tests merely to conceal an implementation failure. Never claim a feature is complete because it compiles.

## 23. Definition of done

A feature is done only when:
1. The end-to-end user flow works.
2. Public interfaces and failure paths are defined.
3. Relevant tests pass.
4. Fallback and recovery paths are tested.
5. Lifecycle, cancellation and shutdown are safe.
6. Performance is measured on the target hardware.
7. Privacy, dependency and license implications are reviewed.
8. Diagnostics and user documentation are updated.
9. Implementation state accurately reflects evidence.
10. Independent review has no unresolved blocking defect.

Compilation alone is never completion. A mock, static demo, placeholder API or untested native adapter is not a working feature.
