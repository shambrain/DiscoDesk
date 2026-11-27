# ADR-0005: A stable error taxonomy crosses the IPC boundary

- **Status:** Accepted
- **Date:** 2026-11-27
- **Deciders:** discodesk-lead, systems-architect
- **Gate:** 0 (repository scaffold)

## Context

The Studio is a separate process from the engine (ADR-0001). It therefore cannot see
Rust error types. `AGENTS.md` requires typed errors and explicit state machines, and
`OPENCODE_MASTER_DIRECTIVE.md` §22 requires structured errors.

Without a shared taxonomy the Studio would have to parse human-readable error strings,
which breaks silently when a message is reworded, and it would encourage the engine to
leak internal detail (absolute paths, backtraces, type names) to a local but still
separate process.

The taxonomy also has to serve the engine internally: the state machine, the IPC layer
and the future platform/audio/analysis subsystems must agree on what an error *means*,
so that the Studio can distinguish "the user must grant a permission" from "the engine
is broken" and show the right thing.

## Options considered

1. **A shared `ErrorCode` enum with a stable numeric representation, mapped from
   crate-local typed errors (chosen).**
2. **Serialise the Rust error enum directly.** Rejected: it couples the wire format to
   Rust type layout, so adding a variant is a breaking protocol change. It also makes
   it easy to accidentally expose a `Debug` representation containing paths.
3. **Free-text error strings only.** Rejected: not machine-readable, not stable, and
   impossible to branch on safely in the UI.
4. **HTTP-style status codes.** Rejected: implies an HTTP model the transport does not
   use, and the numeric space would not map to the product's actual error semantics.

## Decision

1. **One shared error taxonomy** lives in the core crate and is the only error
   vocabulary that crosses the process boundary.
2. **Each error variant maps to a stable `ErrorCode` with an explicit numeric value.**
   The numeric value is part of the protocol contract: it is never reused for a
   different meaning, and a retired value is not reassigned.
3. **The wire payload carries the code and a human-readable message.** It never carries
   a backtrace, an absolute path, a raw OS error string containing a path, or an
   internal Rust type name.
4. **Engine-internal errors are typed and specific** (for example, an illegal state
   transition is distinct from an already-running engine), and are converted to the
   shared code exactly once, at the boundary.
5. **The Studio branches on the code, never on the message text.** The message is for
   display only.
6. **A permission or capability failure is an error code, not a crash**, and the engine
   continues in a supported fallback state where one exists.

## Consequences

- Easier: the Studio can render a correct, specific state for each failure class
  ("permission denied", "engine not running", "protocol mismatch") without string
  matching.
- Easier: adding an internal error variant does not change the protocol, so internal
  refactors are safe.
- Harder: the taxonomy must be maintained. A new public error class requires adding a
  code, which is a deliberate, reviewable act.
- Harder: the mapping from crate-local errors to codes must be total and tested, or
  some internal failure will surface as an unhelpful "internal error" code.
- Now forbidden: putting a path, a backtrace or a type name in an error message that
  crosses the boundary; parsing an error message in the Studio; reusing a retired
  numeric code.

## Verification

- Gate 0, automated: a unit test asserts the complete mapping from every internal
  error variant to its `ErrorCode`, so a new variant that fails to compile against the
  mapping forces the author to choose a code.
- Gate 0, automated: a test asserts that serialised error payloads contain no path
  separator and no absolute-path-shaped string.
- Gate 0, automated: an IPC integration test drives a command into each reachable
  error condition and asserts the returned code.

## References

- `AGENTS.md` — typed errors, no private paths in logs, validate IPC messages.
- `OPENCODE_MASTER_DIRECTIVE.md` §3 item 8, §8, §9.
- `ARCHITECTURE.md` §4 — "structured errors, and protocol version negotiation".
