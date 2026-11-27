# ADR-0009: A threaded engine with no async runtime at Gate 0

- **Status:** Accepted
- **Date:** 2026-11-27
- **Deciders:** discodesk-lead, systems-architect, rust-engineer
- **Gate:** 0 (repository scaffold)

## Context

A Rust desktop engine that owns an audio callback, a GPU render loop, a cache and an IPC
server is a natural candidate for an async runtime. `tokio` is the default choice, and the
IP C crate is an obvious place to reach for it.

But `OPENCODE_MASTER_DIRECTIVE.md` §8 forbids speculative abstraction and unused crates,
and §3 item 20 requires every background task to be bounded, cancellable and joined. An
async runtime makes "joined on shutdown" harder to demonstrate — a cancelled task may
still be running, and the runtime owns threads the engine did not spawn and must still
drain.

At Gate 0 there is exactly one workload: serve a handful of request/response messages over
a local pipe. There is no audio, no GPU, no concurrency pressure and no I/O fan-out. An
async runtime would be pure overhead and an extra dependency with its own advisory
surface (for example RUSTSEC-2026-0384, a race in `tokio::time::Interval`, fixed only at
1.54.1).

The audio callback, when it arrives, must be allocation-free and lock-free — which means
it will not be driven by an async executor in any case.

## Options considered

1. **OS threads plus bounded channels, with an explicit join on shutdown (chosen).**
2. **Tokio from the start.** Rejected at Gate 0: a large dependency, an advisory surface,
   and a harder shutdown story, for a workload that is one connection and a dozen message
   types. It also encourages `tokio::spawn` without a join handle, which is exactly the
   unbounded-task failure mode the directive forbids.
3. **`async-std` or `smol`.** Rejected: same objection, plus a smaller ecosystem.
4. **No concurrency at all (single-threaded, blocking accept loop).** Rejected: a client
   that stalls mid-frame would block the engine. A per-connection thread with a bounded
   queue is the minimum honest design.

## Decision

1. **Gate 0 uses OS threads, `std::sync` primitives and explicitly bounded channels.**
   No async runtime is a dependency of any Gate 0 crate.
2. **Every spawned thread is owned and joined.** Shutdown has a bounded timeout; if a
   worker does not join within it, the engine records that fact and transitions to a
   terminal error state rather than exiting silently.
3. **IPC accepts one client connection at a time at Gate 0**, handled on its own thread,
   with a **bounded** inbound queue. A request that would overflow the queue is rejected
   with a retryable "overloaded" error rather than blocking or growing without limit.
4. **No `unbounded()` channel** may be introduced. This is enforceable by review and by a
   grep in `scripts/check.ps1`.
5. **An async runtime may be introduced later, but only by a superseding ADR**, and only
   for a component that genuinely needs it. The audio callback will never be async; the
   render loop's threading model is decided at Gate 2.
6. **The Studio's own async runtime is its concern.** Tauri has its own runtime; that does
   not leak into the engine.

## Consequences

- Easier: shutdown is provable. Every thread is joined, so "no unbounded background task"
  is an inspectable property rather than a claim.
- Easier: the Gate 0 dependency set stays small, with a correspondingly small licence and
  advisory surface.
- Harder: a future audio pipeline with device callbacks, a resampler and a DSP worker
  needs a hand-built pipeline rather than `tokio::spawn`. That is the correct shape for a
  real-time path regardless.
- Harder: if a later gate wants async file I/O or HTTP for Steam, that gate adds a runtime
  with its own ADR. Deliberate.
- Now forbidden: adding `tokio` (or another runtime) to a Gate 0 crate; an unbounded
  channel; a spawned thread whose handle is dropped without a join path.

## Verification

- Gate 0: `cargo tree` shows no async runtime in the engine's dependency graph.
- Gate 0: a test asserts the engine process exits within the shutdown timeout after a
  shutdown request, and that no child process or thread outlives it.
- Gate 0: `scripts/check.ps1` fails if `unbounded(` appears anywhere in `crates/` or
  `apps/`.

## References

- `OPENCODE_MASTER_DIRECTIVE.md` §3 items 7, 20; §8.
- `AGENTS.md` — bounded, cancellable, joined background jobs; no blocking or unbounded
  work in callbacks.
- `ARCHITECTURE.md` §7.7 (concurrency and performance), §11 (lifecycle).
- `docs/ADR/0002` (the IPC contract this serves).
