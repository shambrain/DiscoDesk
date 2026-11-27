# DiscoDesk project map

Use this map to navigate without loading the entire architecture and research corpus for every bounded task.

## Start here
- docs/VISUAL_STUDIO_WORKFLOW.md — Visual Studio Open Folder, MSVC/SDK setup, debugging and local commit helper.
- AGENTS.md — global invariants.
- docs/IMPLEMENTATION_STATE.md — actual implementation and current acceptance gate.
- OPENCODE_MASTER_DIRECTIVE.md — execution policy and acceptance gates.
- ARCHITECTURE.md — system design.
- RESEARCH.md — evidence and source register.

## Task-to-section map

| Concern | Architecture reference | Specialist |
|---|---|---|
| Product/stack/runtime/processes | Sections 1–5 | systems-architect |
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

## Work rules
1. Inspect current files and state before editing.
2. Read only the relevant sections for bounded tasks.
3. Read the full architecture/research corpus only for deliberate system-wide review.
4. Search symbols/tests before opening large source files.
5. Update this map when crate/spec ownership changes.
6. A planned module is not evidence that code exists.
