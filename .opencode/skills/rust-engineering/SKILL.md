---
name: rust-engineering
description: Use when implementing DiscoDesk Rust crates, services, IPC, concurrency, error handling, FFI boundaries or Cargo workspace changes.
---

# Rust engineering workflow

- Inspect the installed Rust toolchain, Cargo workspace and target triple before editing.
- Keep crates cohesive and dependencies minimal; pin resolved versions in Cargo.lock.
- Prefer safe Rust. Localize unsafe FFI and document pointer, lifetime, thread and ownership invariants.
- Use typed errors and validate every external value. Avoid panic-based control flow in runtime paths.
- Make queue capacity and backpressure explicit. Avoid unbounded channels.
- Keep real-time and render hot paths free of avoidable heap allocation and blocking operations.
- Implement deterministic unit tests for normal, boundary and malformed inputs.
- Test cancellation, partial initialization, shutdown, process/device removal and failure recovery.
- Run cargo fmt --check, cargo clippy on relevant targets and cargo test. Report exact commands and outputs.
- Never suppress warnings to force a green build; explain platform-gated tests that cannot run on the current host.
