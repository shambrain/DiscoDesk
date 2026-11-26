---
name: project-architecture
description: Use when changing DiscoDesk architecture, crate boundaries, engine processes, IPC, state machines, project scope or implementation phase plans.
---

# DiscoDesk architecture workflow

1. Read AGENTS.md, ARCHITECTURE.md, RESEARCH.md and docs/IMPLEMENTATION_STATE.md.
2. Inspect the real repository and current phase; never assume a planned module already exists.
3. Translate user-visible requirements into component ownership and explicit contracts.
4. Keep Rust engine, audio/DSP, image analysis, wgpu renderer, native platform hosts and Tauri Studio separated.
5. Define error cases, cancellation, lifecycle, versioning, security and fallback paths before implementation.
6. Record consequential choices as an ADR under docs/ADR/ with context, options, decision and consequences.
7. Update docs/IMPLEMENTATION_STATE.md only when evidence supports a status change.

Do not invent OS API capabilities, universal cross-platform behavior or performance targets. Prefer the smallest architecture that serves the next acceptance gate. Do not scaffold future phases without a concrete interface consumer.
