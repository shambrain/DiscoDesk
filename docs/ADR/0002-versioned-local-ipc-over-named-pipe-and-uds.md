# ADR-0002: Versioned local IPC over a Windows named pipe and a Unix domain socket

- **Status:** Accepted
- **Date:** 2026-11-27
- **Deciders:** discodesk-lead, systems-architect
- **Gate:** 0 (repository scaffold)

## Context

The engine and the Studio are separate processes (ADR-0001). They must exchange
commands, settings and state snapshots over a local-only channel.

Requirements from `AGENTS.md`, `ARCHITECTURE.md` §4 and `OPENCODE_MASTER_DIRECTIVE.md` §3:

- Local only. Never bind an HTTP listener to `0.0.0.0`; never expose an unauthenticated
  loopback endpoint.
- Authenticated to the current OS user (Windows named pipe with current-user ACLs; a
  Unix socket in a user-owned directory with restrictive permissions).
- Versioned, with explicit protocol version negotiation.
- Length-delimited framing with bounded payload sizes.
- Request IDs and cancellation IDs so the UI can correlate and cancel.
- Structured errors with a stable taxonomy, not free-text strings.
- High-rate audio feature frames must **not** travel over this channel.

Gate 0 requires the contract to exist and be tested, with no subsystems behind it.

## Options considered

1. **Length-delimited binary framing over a named pipe / UDS (chosen).** One connection
   per Studio client; the engine is the server. Small, explicit, no HTTP surface.
2. **HTTP/JSON over loopback.** Rejected: requires binding a TCP port, which is a
   materially larger attack surface, needs its own auth scheme, and invites "just expose
   it on the LAN" mistakes. An explicit prohibition on `0.0.0.0` does not remove the
   risk.
3. **gRPC / protobuf over HTTP2.** Rejected at Gate 0: a heavier dependency and a
   code-generation toolchain, for a command set of roughly a dozen message types.
   Reconsider only if the message surface grows well beyond the ARCHITECTURE.md §4 list.
4. **Windows-only transport.** Rejected: it would make the macOS and Linux ports
   re-implement IPC later. The transport is abstracted behind a trait from day one.

## Decision

1. **Transport**: a Windows named pipe on Windows, a Unix domain socket on macOS and
   Linux, behind one `Transport` trait. The engine is the server; the Studio is the
   client.
2. **Authentication**: the pipe is created with a current-user-only ACL on Windows. The
   Unix socket lives in a user-owned runtime directory with owner-only permissions. The
   endpoint path embeds a per-user identifier so a different user cannot collide with or
   connect to it.
3. **Framing**: length-delimited frames with a magic value, a protocol version, a
   payload length and the payload. The declared length is validated **before** any
   allocation of that size. A frame above the maximum is rejected without allocating it.
4. **Versioning**: a single `PROTOCOL_VERSION` constant plus explicit negotiation in
   the handshake. A mismatch produces a `ProtocolVersionMismatch` error carrying the
   expected and offered versions — it is never silently ignored and never triggers a
   "best-effort" parse of an unknown payload.
5. **Envelope**: every request carries a request ID; every response carries the same ID
   so the client can correlate. Long-running work carries a cancellation ID.
6. **Errors**: a stable numeric `ErrorCode` plus a human-readable message. Backtraces,
   absolute paths and internal type names are not part of the wire format.
7. **Payload discipline**: the command set is the ARCHITECTURE.md §4 list. There is no
   high-rate audio feature frame type in the protocol, and none may be added without a
   new ADR.

## Consequences

- Easier: the Studio can be closed and reopened; the engine state stays authoritative;
  the channel is authenticated by construction rather than by an application-level
  token.
- Easier: the IPC crate can be tested on Windows today, and the Unix path is compiled
  behind `cfg` so a Unix bug cannot break the Windows build.
- Harder: a hand-written framing codec is more code than using an existing library, and
  it must be correct. Mitigated by unit tests for every malformed-frame case.
- Harder: binary framing means the Studio needs a matching codec. The protocol version
  exists precisely to allow both sides to change independently over time.
- Now forbidden: an HTTP listener; a wildcard bind; unauthenticated access; parsing a
  frame whose declared length exceeds the maximum; accepting an unknown protocol
  version; carrying raw PCM or high-rate feature frames over Studio IPC.

## Verification

Gate 0, automated:

- Round-trip: a real named pipe on Windows (a Unix socket on Unix) carries a request and
  returns the matching response with the same request ID.
- Version mismatch: a client offering a wrong protocol version receives
  `ProtocolVersionMismatch` and the connection is closed cleanly.
- Oversized frame: a frame declaring more than the maximum is rejected, and the test
  asserts no large allocation occurred.
- Truncated frame, bad magic and trailing garbage are each rejected with a distinct
  error.
- Unknown command IDs are rejected with `UnknownCommand`.
- The pipe ACL is verified to exclude other users (Windows).

## References

- `ARCHITECTURE.md` §4 (IPC contract).
- `OPENCODE_MASTER_DIRECTIVE.md` §3 item 8, §9 (Gate 0 IPC requirements).
- Microsoft named-pipe security documentation:
  <https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights>
- Microsoft `CreateNamedPipeW`:
  <https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createnamedpipew>
