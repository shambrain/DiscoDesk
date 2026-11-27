# ADR-0011: The Studio is a thin client of the shared endpoint contract

- **Status:** Accepted
- **Date:** 2026-11-27
- **Deciders:** discodesk-lead, systems-architect, tauri-ui-engineer
- **Gate:** 0 (Studio scaffold)

## Context

Gate 0 requires that "engine and Studio are separate processes". That had been
documented (`ADR-0001`) but not implemented: no Studio binary existed, so the
criterion could not be demonstrated.

Three concrete risks had to be resolved before writing the Studio:

1. **Endpoint derivation was duplicated.** `apps/discodesk-engine` computed the
   default endpoint from `USERNAME`, while `dd_ipc::transport::windows` exposed a
   *different* derivation based on the user SID plus the process id. Two
   derivations can both look correct and still name different pipes. Worse, the
   SID+pid form is unguessable to another process, so a Studio using it would
   always report "engine not running".
2. **The Studio could accidentally acquire engine responsibilities.** A control
   UI that polls state invites caching that state, or spawning/killing the
   engine, or growing a reconnect loop per poll.
3. **"Closing Studio must not stop the engine" is untestable by inspection.**
   It needs an executable check against the real binaries.

## Options considered

1. **A shared endpoint module in `dd-ipc`, a client with an injectable connector,
   and a GUI launch smoke test (chosen).**
2. **Let the Studio read the endpoint from the engine's stdout or a discovery
   file.** Rejected: it introduces a second startup-ordering dependency and a
   file format to secure, for a name that is already derivable from the user.
3. **Keep the pid-based endpoint and pass the path on the Studio command line.**
   Rejected as the default: it makes the two processes only able to meet if
   something else launches both, which contradicts "the Studio starts
   independently of the engine".
4. **Have the Studio own a polling task that spawns a reconnecting thread.**
   Rejected: an unbounded per-poll worker is exactly the failure mode
   `OPENCODE_MASTER_DIRECTIVE.md` §3.20 forbids.

## Decision

1. **`dd_ipc::endpoint::default_endpoint_path()` is the single derivation.** The
   engine and the Studio both call it. It is stable within a user session and
   contains no process id, so either process can find the other. The Windows
   transport keeps `per_process_endpoint_path()` for the separate case where a
   test must avoid colliding with a running engine, and it is documented as such.
2. **The Studio owns no engine state.** `EngineClient` sends a documented command
   and returns a parsed response. It caches a *connection*, never a status.
   `EngineView` is derived only from the bytes the engine sent.
3. **The connector is a trait.** Production uses the named pipe; tests inject a
   scripted transport so every branch — connected, absent, protocol mismatch,
   permission denied, transport failure — is exercised through the real envelope
   and frame code.
4. **One connection, one in-flight request.** The client holds at most one
   transport; the frontend keeps a single in-flight guard and a single interval.
   There is no retry loop in Rust and no per-poll task.
5. **The Studio never sends `ShutdownEngine`.** There is no code path for it. A
   unit test asserts the set of commands actually sent, so adding one without
   reconsidering this ADR fails the build.
6. **The Studio has no audio, DSP, renderer or package code**, and does not
   depend on any future engine crate. Its only engine dependency is `dd-ipc`.
7. **Lifecycle is proven by `scripts/studio-smoke.ps1`**, which launches the real
   GUI, closes it, and asserts the engine is still running and still answering.

## Consequences

- Easier: either process can be started first; the Studio reports "not running"
  honestly instead of failing to find a pipe it could have derived.
- Easier: the IPC boundary is testable without an engine, and the engine is
  testable without a UI.
- Easier: "closing Studio does not stop the engine" is a scripted assertion, not
  a promise.
- Harder: the endpoint name is derivable by any local process of the same user —
  which is why the security model is the pipe ACL, not the name. That is already
  the case (`ADR-0002`).
- Harder: a request that never returns would leave one blocked OS thread, because
  named-pipe reads have no timeout. The UI stays bounded (one in-flight call) but
  a wedged engine requires restarting the Studio. Documented as a limitation.
- Now forbidden: a second endpoint derivation; the Studio sending engine-control
  commands that stop the engine; a Studio-side reconnect thread; duplicating
  engine status into Studio-owned state.

## Verification

- Unit tests (`apps/discodesk-studio/src-tauri/src/engine_client.rs`): status
  parsing, absent engine, protocol mismatch at frame and envelope level,
  permission detail preservation, engine-side error codes, malformed status,
  reconnect after a failed connect, connection reuse, transport failure and
  recovery, wrong request id rejection, monotonic request ids, concurrency, and
  the "never asks the engine to shut down" invariant.
- Frontend tests: loading, connected, unavailable, mismatch, bridge failure and
  disconnect-on-unmount.
- `scripts/ipc-smoke.ps1`: the real Studio binary's `--probe` against the real
  engine binary.
- `scripts/studio-smoke.ps1`: real window appears, Studio exits `0`, the engine
  is still alive afterwards, a fresh probe still reaches it, and the engine then
  shuts down cleanly.

## References

- `docs/ADR/0001` (separate processes), `docs/ADR/0002` (IPC contract),
  `docs/ADR/0008` (frontend toolchain), `docs/ADR/0009` (no async runtime).
- `docs/IPC_PROTOCOL.md`.
- `OPENCODE_MASTER_DIRECTIVE.md` §3 items 1, 2, 8, 19, 20.
