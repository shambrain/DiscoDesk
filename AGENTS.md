# DiscoDesk — repository instructions

These rules apply to the whole repository and are loaded by OpenCode.

## Start every task

1. Inspect git status, branch, current files and docs/IMPLEMENTATION_STATE.md.
2. Read docs/PROJECT_MAP.md to locate relevant modules and specs.
3. Read only the relevant sections of ARCHITECTURE.md, RESEARCH.md and task specifications. Do not load the entire corpus into every agent context by default.
4. Read OPENCODE_MASTER_DIRECTIVE.md for gate work, architecture-wide changes or before starting a new subsystem.
5. Read the full architecture/research corpus only for a deliberate system-wide audit.

## Product invariants

- DiscoDesk transforms the user's still wallpaper into a restrained, image-aware, music-responsive desktop scene.
- Rust owns the background engine, audio capture, DSP, image analysis, rendering, desktop hosting, automation, cache, notifications and package validation.
- Tauri v2 + React + TypeScript is the on-demand Studio/control UI, never the wallpaper renderer.
- Core playback is local-first and must work without Steam, network, cloud services or generative AI.
- Image inference is import/change-time work; cache outputs; never run depth inference in the playback loop.
- System audio capture is the compatibility baseline; per-app capture requires runtime capability detection.
- Frequency bands are not instrument stems; spectral flux is not a complete beat tracker.
- No strobing, uncontrolled shake, harsh flicker, destructive image edits or motion-only status.
- Magic Wand and automatic wallpaper watcher are deferred until the Windows MVP, cache, IPC and host lifecycle are stable.
- Auto reaction respects manual pause, opt-out, per-wallpaper/per-display settings, exclusions and reduced motion.
- Workshop presets are declarative validated data, not arbitrary executable content.
- No performance/accuracy/compatibility claim without reproducible measurements.

## Research-first engineering

Use current vendor documentation, maintained upstream repositories, exact OS versions and exact licenses before selecting dependencies or native behavior. Record evidence in RESEARCH.md and ADRs. Reddit/forums are operational evidence, not authoritative API contracts. Separate fact, inference, workaround and unknown.

## Workflow and safety

- Implement one coherent acceptance gate at a time; keep the repository buildable.
- Define interfaces before parallel specialist work; no concurrent writes to the same files.
- Use worktrees for independent source edits; reviewers are read-only.
- Keep capture callbacks bounded, allocation-free and non-blocking.
- Validate paths, IPC, dimensions, audio formats, model tensors, archives, shader inputs and OS handles.
- Make background jobs bounded, cancellable and joined on shutdown.
- Test silence, clipping, NaN/infinity, empty buffers, device/process loss, permission denial, display hotplug, Explorer restart, GPU loss, cache corruption, hostile packages and shutdown races where relevant.
- Prefer safe Rust; isolate unsafe FFI and document invariants.
- Never suppress warnings, invent test results or mark a feature complete because it compiles.
- Never log raw PCM, API keys, private paths or user image contents.
- Pin dependencies and review model weights separately from code licenses.

## Native agent model routing

Every native agent under `.opencode/agents/` must have an explicit OpenCode `model:` in YAML frontmatter. Keep assignments aligned with `docs/AI_TOOLING_DECISION.md`. OpenCode otherwise lets a subagent inherit the parent session model. OMO Slim's six managed agents are separate and remain configured in `.opencode/oh-my-opencode-slim.json`. Preserve prompt bodies, permission rules and skill grants when changing model routing.

## Token discipline

Use project map and task-specific reads. Search symbols before opening large files. Research agents return summarized claims and source links. Keep tool output bounded. Create durable checkpoints before long fresh sessions. Do not confuse a large context window with efficient context usage.

## Completion report

List changed paths, acceptance criteria, exact commands, pass/fail/skip counts, OS/hardware, performance evidence, unsupported behavior and unresolved risks. Update IMPLEMENTATION_STATE.md only when evidence supports the status.
