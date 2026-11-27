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

## Build environment

Verified Windows development baseline (2026-11-27). Full matrix: [docs/PLATFORM_SUPPORT.md](docs/PLATFORM_SUPPORT.md).

| Tool | Version | Notes |
|---|---|---|
| Rust | `1.100.0` stable, `x86_64-pc-windows-msvc` | Pinned in `rust-toolchain.toml`; `rustfmt` + `clippy` required |
| MSVC | Build Tools 2019 (14.29) **and** Build Tools 2026 (14.51) | Prefer 2026; the 2019 toolset is below what Tauri/wry expect |
| Windows SDK | `10.0.19041.0`, `10.0.26100.0` | Use `10.0.26100.0` for the Studio |
| Node.js | `v26.12.1` (npm `11.20.0`) | Studio build toolchain only; use **npm**, not Bun (see below) |
| Bun / pnpm | `1.4.4` / `12.16.1` | Not used for the Studio build |
| WebView2 Runtime | `158.0.4370.33` | Required by the Studio shell |
| GPU | AMD Radeon RX 6700 XT + Radeon integrated | A Virtual Display Driver adapter is also present |

> **PATH caveat:** Rust is installed at `%USERPROFILE%\.cargo\bin` but is **not** on the
> default `PATH` on this machine. `scripts/check.ps1` resolves cargo itself; a manual
> shell needs:
> ```powershell
> $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
> ```

> **Bun is not the Studio package manager.** Bun statically links JavaScriptCore/WebKit
> under LGPL-2. Bun is MIT itself, but a redistributed Bun would carry copyleft
> obligations. It is fine as a personal dev tool, never as a build or distribution
> dependency. See `docs/ADR/0004`.

### Reproducible commands

`scripts/check.ps1` is the single source of truth. CI runs the same commands.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/check.ps1
```

which runs, in order:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace
```

## Visual Studio 2026 development

DiscoDesk supports Visual Studio Open Folder for Windows-native debugging and diagnostics while Cargo remains the authoritative Rust build system. The repository includes Community and Build Tools component manifests, tracked Open Folder tasks/debug profiles, and PowerShell environment/commit helpers.

- [Visual Studio workflow](docs/VISUAL_STUDIO_WORKFLOW.md)
- [Community 2026 component configuration](.vsconfig)
- [Build Tools 2026 component configuration](vsconfig/BuildTools-2026.vsconfig)
- [Visual Studio environment Doctor](scripts/vs/Invoke-DiscoDeskVS.ps1)
- [Safe explicit-path Git commit helper](scripts/git/Commit-DiscoDesk.ps1)

Open the repository with **Visual Studio > File > Open > Folder**. Do not create a C++ solution for the Rust workspace. F5 debugging becomes available when the actual engine binary source is present and the Cargo build succeeds.

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
