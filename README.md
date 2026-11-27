# DiscoDesk

**A desktop that moves with your music — using your own wallpaper.**

DiscoDesk is a planned local-first, cross-platform audio-reactive wallpaper engine. It analyses an ordinary still image, creates a reusable visual recipe, and uses real-time audio features to drive restrained depth, light and particle effects while preserving the original artwork.

## Product direction

- One-click still-image transformation: choose a wallpaper, analyse it, and make it react.
- Image-aware movement instead of conventional spectrum bars pasted over the desktop.
- Local audio capture, DSP and image analysis. No cloud account or cloud inference is required for core playback.
- Rust native engine, Tauri v2 Studio UI, and wgpu/WGSL rendering.
- Optional cached depth estimation; no AI inference in the live playback loop.
- Native Windows, macOS and Linux adapters with honest per-platform capability reporting.
- Versioned .ddpkg packages and optional Steam Workshop integration.

## Repository status

**Research and implementation blueprint. The wallpaper engine is not implemented yet.** No CPU, GPU, memory, compatibility or model-accuracy targets are certified until they are measured on supported hardware.

## Start with OpenCode

Open this repository as the workspace in OpenCode. The repository includes a project-specific coordinator agent and engineering subagents. The default project agent is **discodesk-lead**.

- [AGENTS.md](AGENTS.md) — persistent project rules loaded by OpenCode.
- [opencode.jsonc](opencode.jsonc) — project OpenCode configuration.
- [.opencode/agents/](.opencode/agents/) — coordinator and specialist agents.
- [.opencode/skills/](.opencode/skills/) — reusable on-demand engineering skills.
- [OPENCODE_MASTER_DIRECTIVE.md](OPENCODE_MASTER_DIRECTIVE.md) — phase gates and implementation requirements.

Skills can be invoked in a prompt by ID, for example **@audio-dsp**, **@wgpu-wallpaper-rendering**, or **@qa-performance**. OpenCode also advertises permitted skills when relevant.

## Technical source of truth

- [ARCHITECTURE.md](ARCHITECTURE.md) — system design, crate boundaries, runtime contracts and phased architecture.
- [RESEARCH.md](RESEARCH.md) — competitor audit, OS APIs, model choices, scientific constraints and licensing research.
- [docs/IMPLEMENTATION_STATE.md](docs/IMPLEMENTATION_STATE.md) — current implementation status and next acceptance gate.

## Recommended first milestone

Build and verify a Windows vertical slice before expanding platform coverage:

1. Rust workspace and native engine shell.
2. WASAPI system/output loopback and deterministic DSP telemetry.
3. wgpu preview renderer that reacts to an AudioFeatureFrame.
4. Still-image analysis and cached visual recipe.
5. Windows desktop host only after preview rendering and desktop-interaction tests pass.

## Engineering principles

Preserve the user's source image. Never write captured PCM to disk by default. Never infer instrument stems from broad FFT bands. Never run image models during playback. Keep capture callbacks bounded and allocation-free. Provide deterministic fallbacks for unsupported APIs, permissions, failed models and GPU loss. Do not claim a platform is supported until tested on that platform.

## OpenCode documentation

- [Agents](https://opencode.ai/docs/agents/)
- [Skills](https://opencode.ai/docs/skills/)
- [Project instructions](https://opencode.ai/docs/instructions/)
- [Configuration](https://opencode.ai/docs/config/)


## Ambient desktop roadmap

Later M2/M3 work adds per-monitor wallpaper change detection, content-hash cache reuse, opt-in automatic reaction, a desktop Magic Wand quick action, creator presets and quiet local notifications. These are specified but not implemented and are deliberately scheduled after the Windows MVP is stable.

- [DESKTOP_EXPERIENCE_SPEC.md](docs/DESKTOP_EXPERIENCE_SPEC.md)
- [AI_TOOLING_DECISION.md](docs/AI_TOOLING_DECISION.md)
- [PROJECT_MAP.md](docs/PROJECT_MAP.md)
