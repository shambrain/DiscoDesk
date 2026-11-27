# ADR-0001: Rust owns the engine, Tauri owns Studio

- **Status:** Accepted
- **Date:** 2026-11-27
- **Deciders:** discodesk-lead, systems-architect
- **Gate:** 0 (repository scaffold)

## Context

DiscoDesk must render an animated, audio-reactive wallpaper behind desktop icons while
also providing a calm, accessible Studio UI for configuration. The wallpaper surface
needs predictable frame scheduling, direct access to image/depth textures, per-monitor
surface lifetime and low-latency response to audio features.

A Studio UI built on a system WebView (Tauri, Electron, or a resident Node process)
cannot satisfy those requirements: the WebView compositor is not a wallpaper surface,
high-rate audio frames must not traverse a UI IPC boundary, and the product must keep
working with no network, no Steam and no cloud service.

The user-visible requirement is that closing Studio never stops the wallpaper, and that
the engine continues reacting to audio while the user is in another application.

## Options considered

1. **Rust native engine + Tauri v2 Studio (chosen).** The engine owns capture, DSP,
   analysis, rendering, native desktop hosting and the cache. Studio is a separate
   on-demand process that talks to it over a versioned local IPC.
2. **Single Node/Electron application.** Rejected: the wallpaper surface, the
   allocation-free audio callback and the native Windows shell integration do not fit a
   WebView/Node runtime, and it would require a resident Node process.
3. **Java service.** Rejected: no advantage for native shell integration or GPU surface
   ownership, and an extra runtime to ship and secure.
4. **Rust engine with the Studio embedded in the engine process.** Rejected: it breaks the
   requirement that closing Studio must not stop playback, and it couples a UI framework
   into a long-lived background process.

## Decision

1. The Rust workspace is the product engine. It owns audio capture, DSP, image analysis
   and caching, the `wgpu` renderer, native desktop hosting, the power/pause policy, the
   package validator and the diagnostics log.
2. Tauri v2 with React and TypeScript is the **on-demand Studio control UI only**. The
   Studio WebView is never the wallpaper rendering surface.
3. The engine and the Studio are **separate OS processes** with separate lifecycles. The
   engine's state is authoritative; Studio is a view over it and can be closed and
   reopened at any time.
4. High-rate audio feature frames travel **inside the engine**, from DSP to renderer.
   They never cross the Studio IPC boundary. Studio IPC carries settings, commands,
   state snapshots and telemetry only.
5. Core playback must work with Steam absent, offline, and with no generative-AI or
   network dependency.

## Consequences

- Easier: the engine can be tested headlessly (no WebView, no Studio); the renderer and
  the audio callback keep native performance characteristics; the product runs without a
  JavaScript runtime resident.
- Harder: two processes mean an IPC contract, a version negotiation, an endpoint
  discovery mechanism and an authentication story (see ADR-0002). The Studio must handle
  the engine being absent, restarting, or running a different protocol version.
- Harder: UI capability indicators must be driven by an engine-side capability probe,
  never hard-coded in the frontend.
- Now forbidden: rendering the wallpaper inside the Studio WebView; shipping the Studio
  as a resident background service; sending audio feature frames over Studio IPC;
  making Steam, the network or a cloud service a runtime dependency for core playback.

## Verification

- Gate 0: `discodesk-engine` and `discodesk-studio` exist as separate binaries and
  separate processes; closing Studio leaves the engine process running; the IPC
  protocol carries no high-rate audio frame type.
- Gate 1: the telemetry CLI runs with the renderer and Studio absent.
- Gate 4: reopening Studio reconnects to the live engine state.

## References

- `ARCHITECTURE.md` §3 (recommended technology stack), §4 (runtime process model and IPC
  contract), §18 (architecture decisions to preserve).
- `OPENCODE_MASTER_DIRECTIVE.md` §3 items 1, 2, 4, 8, 19.
- `AGENTS.md` product invariants.
