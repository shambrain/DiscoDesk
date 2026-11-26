# DiscoDesk — project instructions

These instructions apply to the entire repository and are loaded by OpenCode.

## Required reading

Before making implementation changes, read these documents completely:

1. ARCHITECTURE.md
2. RESEARCH.md
3. OPENCODE_MASTER_DIRECTIVE.md
4. docs/IMPLEMENTATION_STATE.md

Treat them as the current product and engineering baseline. If a new fact contradicts them, verify it against current primary documentation, update the relevant decision/document, and explain the change. Do not silently change architecture.

## Product invariants

- DiscoDesk transforms a user's own still wallpaper into a restrained music-responsive desktop scene.
- Audio capture, DSP, image analysis, rendering, platform hosting and package validation belong in the native Rust engine.
- Tauri v2 + React + TypeScript is the on-demand Studio/control UI; it is not the wallpaper renderer.
- wgpu/WGSL renders the wallpaper; per-OS adapters host it behind desktop icons and ordinary windows.
- Core playback is local-first and must work without Steam, cloud services, network access or a generative AI model.
- Image inference is an import-time operation. Cache outputs and never run depth inference in the playback loop.
- System audio capture is the initial compatibility baseline. Per-application capture must be gated by runtime capability detection and OS version.
- A frequency band is not an instrument stem, vocal detector or source separator. Spectral flux is not a complete beat tracker.
- No full-screen strobe, uncontrolled shake, harsh flicker, aggressive distortion or destructive source-image edits by default.
- No resource, latency, quality, compatibility or performance claim without reproducible measurement.

## Development workflow

1. Inspect git status, current branch, repository tree, toolchain and target OS before editing.
2. Work in the smallest coherent phase. Do not scaffold every subsystem in one unreviewable change.
3. Write or update docs/IMPLEMENTATION_STATE.md as implementation progresses.
4. Record decisions that affect public interfaces, packages or platform compatibility as ADRs under docs/ADR/.
5. Define interfaces and deterministic tests before introducing OS-specific implementations.
6. Keep real-time audio callbacks bounded and allocation-free. No locks that can block, filesystem access, inference, UI calls or unbounded logging in callbacks.
7. Validate all untrusted inputs, paths, IPC messages, dimensions, buffer lengths, sample rates, model outputs, package entries, shader parameters and OS handles.
8. Test silence, noise, clipping, NaN/infinity, zero-length input, device removal, process exit, permission denial, display hotplug, shell restart, GPU loss and shutdown races.
9. Run formatter, linter, unit/integration tests and relevant benchmarks. Report exact commands and results.
10. Pin dependencies and review every new dependency, model checkpoint, binary, font, image and other distributable asset for license and provenance.

## Code quality

- Prefer safe Rust. Use unsafe only at a narrowly scoped FFI/platform boundary with documented invariants and tests.
- Avoid panic-prone indexing and unchecked assumptions in runtime paths.
- Avoid global mutable state, needless allocations in hot loops, duplicated abstractions and speculative frameworks.
- Keep modules small, names descriptive and errors structured.
- Do not silence compiler or clippy warnings to make a build pass.
- Keep UI state out of the renderer and high-rate audio frames out of Studio IPC.
- Keep credentials, tokens, private paths, user audio and user artwork out of logs and commits.

## Agent collaboration

The discodesk-lead agent coordinates work. Delegate bounded tasks to the specialist agents under .opencode/agents/. Avoid parallel edits to the same files. Require each subagent to report changed paths, tests executed, results, assumptions and unresolved risks. The qa-performance-reviewer and research-licensing-auditor are reviewers and must not modify source files.

## Completion standard

A feature is not complete because it compiles. It is complete only when its acceptance criteria pass on the stated OS/runtime, fallback behavior is verified, tests are recorded, and documentation accurately describes what was tested.
