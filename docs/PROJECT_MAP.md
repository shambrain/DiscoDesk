# DiscoDesk project map

Use this map to navigate without loading the entire architecture and research corpus for every bounded task.

## Start here
- docs/VISUAL_STUDIO_WORKFLOW.md - Visual Studio Open Folder, MSVC/SDK setup, debugging and commit helpers.
- docs/VISUAL_STUDIO_DEBUGGING.md - exact procedure for an interactive Studio debug session.
- docs/GATE0_PLAN.md - what Gate 0 builds, what it refuses to build, and how each item is proven.
- docs/IPC_PROTOCOL.md - the engine/Studio wire contract: framing, envelopes, commands, error codes, security.
- docs/ADR/ - accepted architecture decisions with rationale and verification.
- docs/LICENSING_DECISION.md - licence options and obligations; awaiting an owner decision.
- docs/PLATFORM_SUPPORT.md - measured host baseline and per-platform capability status.

## Crate map (Gate 0)

| Path | Responsibility | Status |
|---|---|---|
| `crates/dd-ipc` | Framing, envelopes, command/error contract, named-pipe + UDS transports, **shared endpoint derivation** | Implemented, tested |
| `crates/dd-core` | Authoritative state machine, lifecycle, typed engine error | Implemented, tested |
| `crates/dd-diagnostics` | Privacy-preserving structured events, path redaction, rotating log | Implemented, tested |
| `apps/discodesk-engine` | Background engine binary: IPC server, CLI | Implemented, tested |
| `apps/discodesk-studio/src-tauri` | Tauri v2 shell, `EngineClient`, three IPC commands, `--probe` mode | Implemented, tested |
| `apps/discodesk-studio/src` | React 19 + TypeScript control UI (connection states only) | Implemented, tested |
| `dd-audio`, `dd-dsp`, `dd-analysis`, `dd-renderer`, `dd-package`, `dd-steam` | Deferred to their gates | **Must not exist** (enforced by `scripts/check.ps1`) |

## Task-to-section map

| Concern | Architecture reference | Specialist |
|---|---|---|
| Product/stack/runtime/processes | Sections 1–5 | systems-architect |
| Local IPC contract | Section 4 and docs/IPC_PROTOCOL.md | systems-architect, rust-engineer |
| Audio capture | Section 6 | audio-dsp-engineer, platform-engineer |
| DSP/features | Section 7 | audio-dsp-engineer |
| Image analysis/cache | Section 8 | image-intelligence-engineer |
| GPU renderer | Section 9 | graphics-engineer |
| Desktop host adapters | Section 10 | platform-engineer |
| Engine state/lifecycle | Section 11 | systems-architect, rust-engineer |
| .ddpkg/Workshop | Sections 12, 14 | steam-distribution-engineer |
| Studio UX | Section 13 and Addendum A9 | tauri-ui-engineer, product-ux-designer |
| Performance/testing | Sections 15–16 | qa-performance-reviewer |
| Roadmap | Section 17 | discodesk-lead |
| Wand/watcher/presets/notifications | Addendum A and docs/DESKTOP_EXPERIENCE_SPEC.md | desktop-experience-engineer |
| Harness/model routing | docs/AI_TOOLING_DECISION.md | discodesk-lead, research-licensing-auditor |
| Windows build/debug tooling | docs/VISUAL_STUDIO_WORKFLOW.md | platform-engineer, rust-engineer |
| Studio process and IPC client | docs/ADR/0011 and docs/IPC_PROTOCOL.md | tauri-ui-engineer, systems-architect |
| Dependency versions and licences | RESEARCH.md §16, docs/ADR/0012 | research-licensing-auditor |

## Work rules
1. Inspect current files and state before editing.
2. Read only the relevant sections for bounded tasks.
3. Read the full architecture/research corpus only for deliberate system-wide review.
4. Search symbols/tests before opening large source files.
5. Update this map when crate/spec ownership changes.
6. A planned module is not evidence that code exists.
7. Verify a crate's real status in `docs/IMPLEMENTATION_STATE.md`, not in this table's intent.
