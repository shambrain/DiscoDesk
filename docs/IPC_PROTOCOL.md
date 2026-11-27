# DiscoDesk local IPC protocol

**Version:** 1 (`PROTOCOL_VERSION`)
**Status:** Implemented at Gate 0 for the commands listed in §6. Deferred commands are
present in the contract and refuse with a typed error naming their gate.
**Source of truth:** `crates/dd-ipc/src/{frame,protocol,error,transport/*}.rs`. If this
document and the code disagree, the code is wrong or the document is stale — fix one of
them, never both silently.

---

## 1. Purpose and boundaries

The IPC channel exists so the on-demand Studio can read and control a long-running
engine. It is **not** a data plane.

| Travels over IPC | Never travels over IPC |
|---|---|
| Commands, settings, state snapshots, telemetry summaries | Raw PCM |
| Capability reports | Audio feature frames |
| Diagnostic snapshots | Image data |
| Request/response correlation | Anything high-rate |

`AGENTS.md` and `ARCHITECTURE.md` §4 require this split: audio features go from DSP to
renderer **inside** the engine. The protocol has no frame type capable of carrying PCM,
and adding one requires a new ADR.

---

## 2. Transport

| Property | Windows | macOS / Linux |
|---|---|---|
| Kind | Named pipe | Unix domain socket |
| Default path | `\\.\pipe\discodesk-engine-<segment>` | `$XDG_RUNTIME_DIR/discodesk-engine-<segment>.sock` |
| Server | Engine | Engine |
| Client | Studio | Studio |
| Remote access | Refused (`PIPE_REJECT_REMOTE_CLIENTS`) | Filesystem permissions |
| Authentication | Explicit DACL (see §3) | Socket `0600` in a user-owned directory |

The engine is **always** the server. Studio never binds or listens. There is no TCP
listener for DiscoDesk; no wildcard bind and no HTTP endpoint exists anywhere in the
product.

`<segment>` is derived from the **account identity**, not from an environment variable:

- **Windows:** the account SID from the process token, hashed with FNV-1a to a fixed
  16-hex-character form (`u412f96d8994d1362`).
- **Fallback (and Unix until Gate 8):** a sanitised `USERNAME`/`USER`, or `default`.

This exists so two accounts on one machine do not collide on one path. **Security does not
depend on the name**: the descriptor on the Windows pipe restricts access regardless of
what the pipe is called.

The SID is used rather than `USERNAME` because an environment variable can be empty or
overridden. During this project's own verification, `USERNAME` was empty in the development
shell, which silently collapsed every account onto `discodesk-engine-default`. A SID cannot
be spoofed that way. It is hashed rather than embedded because a raw SID is up to ~45
characters and truncating it would collide for accounts sharing a machine prefix.

`--endpoint <path>` overrides the path. That bypasses the per-user default name but not
the security descriptor.

### 2.1 The endpoint is derived, not discovered

Both processes call `dd_ipc::endpoint::default_endpoint_path()`. There is exactly one
derivation, so either process can be started first:

- the engine listens on it and prints it with `--print-endpoint`;
- the Studio connects to it and never needs to be told.

The name deliberately contains **no process id**, so it is stable within a user session.
The Windows transport also exposes `per_process_endpoint_path()` (SID + pid) for the one
case where a test must avoid colliding with a running engine; that is not the product
default.

An earlier revision had two derivations — `USERNAME` in the engine and SID + pid in the
transport. Both looked correct and named different pipes; the Studio could never have
found the engine. See `docs/ADR/0011`.

---

## 3. Security model

The Windows default DACL for a named pipe grants **read access to `Everyone`**. Relying on
it would be a disclosure bug, so the engine always creates the pipe with an explicit
protected descriptor:

```
D:P(A;;GA;;;OW)(A;;GA;;;SY)
```

| Component | Meaning |
|---|---|
| `D:P` | The DACL is **protected**: no inherited ACEs are added |
| `(A;;GA;;;OW)` | Generic-all to `OWNER RIGHTS` — the principal that owns the object |
| `(A;;GA;;;SY)` | Generic-all to `LOCAL SYSTEM` |

There is deliberately **no** `WD` (`Everyone`), `BU` (`BUILTIN\Users`) or `AN`
(`ANONYMOUS`) ACE. The descriptor is allocated by
`ConvertStringSecurityDescriptorToSecurityDescriptorW` and released exactly once through
an RAII wrapper that calls `LocalFree`, so it cannot leak on an error path.

`PIPE_REJECT_REMOTE_CLIENTS` is set, so a client on another machine cannot connect even
if it somehow authenticates locally.

The tests read the effective descriptor back
(`GetSecurityInfo` → `ConvertSecurityDescriptorToStringSecurityDescriptorW`) and assert
that no world/group/anonymous ACE is present, rather than only asserting that the
constant string looks right.

---

## 4. Framing

Every message is a length-delimited frame. All integers are little-endian.

```
offset  size  field
0       4     magic       = 0x44 0x44 0x46 0x31  ("DDF1")
4       2     version     u16  protocol version
6       4     length      u32  payload byte count
10      N     payload     N = length bytes
```

| Constant | Value | Meaning |
|---|---|---|
| `FRAME_HEADER_BYTES` | 10 | Fixed header size |
| `MAX_FRAME_BYTES` | 1 048 576 (1 MiB) | Maximum payload |

**Bounds rule:** the declared `length` is validated against `MAX_FRAME_BYTES` **before**
any buffer is allocated. A hostile header claiming ~4 GiB is rejected without allocating
anything. `peek_payload_len` exists so a reader can validate the header and size its
buffer without touching the payload.

Errors are distinct, not merged: `BadMagic`, `ProtocolVersionMismatch`, `FrameTooLarge`,
`TruncatedFrame`, `TrailingBytes`.

The codec is hand-rolled (`frame.rs`) rather than derived, so the header never goes
through a serializer and the allocation count stays predictable.

---

## 5. Envelopes

The payload is UTF-8 JSON. The frame layer is transport; the envelope layer is the
contract.

### 5.1 Request

```json
{
  "version": 1,
  "request_id": 42,
  "command": "GetEngineStatus",
  "payload": null
}
```

| Field | Type | Notes |
|---|---|---|
| `version` | u16 | Protocol version. Checked again here, see §7 |
| `request_id` | u64 | Client-chosen, echoed in the response |
| `command` | string | One of §6's `name()` values |
| `payload` | object \| absent | Only for commands that take one |

A payload supplied to a command that takes none is **rejected**, so a malformed client
cannot smuggle data past the handler.

### 5.2 Response

Success:

```json
{ "version": 1, "request_id": 42, "result": { "...": "..." } }
```

Failure:

```json
{ "version": 1, "request_id": 42, "error": { "code": 1015, "message": "GetAudioSources is not implemented at Gate 0; it belongs to Gate 1 (audio capture)" } }
```

Exactly one of `result` / `error` is present. `is_ok()` is `error.is_none()`.

### 5.3 Payload string limits

| Constant | Value | Applies to |
|---|---|---|
| `MAX_STRING_BYTES` | 1024 | Identifiers |
| `MAX_NAME_BYTES` | 256 | Display names |

Paths, where a command accepts one, must be absolute, NUL-free and within the limit. The
IPC layer validates the *shape* only; the owning subsystem resolves and opens the path
under its own permissions.

---

## 6. Commands

Names are part of the protocol and must not change within a version.

| Command | Payload | Gate 0 behaviour |
|---|---|---|
| `GetEngineStatus` | — | **Implemented**: state, engine version, protocol version, uptime, surface count |
| `GetPlatformCapabilities` | — | **Implemented**: returns `probed: false` and `unknown` for every capability |
| `GetDiagnosticsSnapshot` | — | **Implemented**: returns an empty event list plus an explanation |
| `ShutdownEngine` | — | **Implemented**: replies, then stops the listener and exits `0` |
| `GetAudioSources` | — | Refused: Gate 1 |
| `SetAudioSource` | `{mode, source_id?}` | Refused: Gate 1 |
| `Pause` / `Resume` / `TogglePause` | — | Refused: Gate 1 (lifecycle beyond idle) |
| `SetEffectSettings` | `{effect, enabled, intensity}` | Refused: Gate 2 / Gate 4 |
| `SetVibePreset` | `{preset_id, ...}` | Refused: Gate 4 |
| `SetPerformancePolicy` | `{...}` | Refused: Gate 2 / Gate 4 |
| `ImportWallpaper` | `{path, options}` | Refused: Gate 5 |
| `CancelJob` | `{job_id}` | Refused: Gate 5 |
| `ApplyPackage` | `{package_id}` | Refused: M3 |

A refused command returns `ErrorCode::NotImplemented` (1015) with a message that names
the gate. This is deliberate: an honest "not yet" is better than a fake success, and it
lets Studio render an accurate state instead of a generic failure.

### 6.1 Capability reporting is honest

`GetPlatformCapabilities` at Gate 0 reports `probed: false` and `"unknown"` for
wallpaper hosting, system audio capture, per-app audio capture and wallpaper-change
detection.

`unknown` ("not probed") is **not** the same claim as `unsupported` ("probed and
absent"). `docs/ADR/0003` requires the two to stay distinguishable, and
`docs/PLATFORM_SUPPORT.md` records that this host is Windows 10 build 19045, which is
below the build 20348 threshold for process loopback. When Gate 1 adds a real probe, this
host must report system-mix only.

---

## 7. Versioning

- The version appears in **both** the frame header and the envelope. They are checked at
  different layers and therefore fail differently:

| Mismatch | Where detected | Behaviour |
|---|---|---|
| Frame header version | `Transport::recv_frame` | **Connection is closed.** The request was never decoded, so there is no `request_id` to correlate a reply with. The engine logs the mismatch. |
| Envelope `version` | `RequestEnvelope::validate` | **Typed error response** `ProtocolVersionMismatch` (1001) correlated to `request_id`, then the connection is closed. |

This split is deliberate. Pretending a transport-level mismatch can be answered politely
would be false: the peer is not speaking the framing this build understands.

- Bumping `PROTOCOL_VERSION` invalidates every older peer. There is no automated
  fallback and no "best effort" parse.
- Unknown command names are **rejected** (`UnknownCommand`, 1006), never defaulted. This
  is what makes `#[non_exhaustive]` on `Command` safe on the wire.

---

## 8. Error codes

Stable numeric values. A retired value is never reassigned.

| Code | Name | Retryable | Meaning |
|---|---|---|---|
| 1000 | `InvalidArgument` | no | Syntactically valid, failed validation |
| 1001 | `ProtocolVersionMismatch` | no | Peer speaks another version |
| 1002 | `FrameTooLarge` | no | Declared payload exceeds 1 MiB |
| 1003 | `BadMagic` | no | Header does not start `DDF1` |
| 1004 | `TruncatedFrame` | no | Ended before the declared payload |
| 1005 | `TrailingBytes` | no | Bytes after a complete frame |
| 1006 | `UnknownCommand` | no | Not part of the protocol |
| 1007 | `PermissionDenied` | no | Capability or OS permission missing |
| 1008 | `NotFound` | no | Referenced resource absent |
| 1009 | `AlreadyRunning` | no | Conflicts with a running instance |
| 1010 | `PeerClosed` | — | Peer closed cleanly (not an error to report to a user) |
| 1011 | `MalformedPayload` | no | Payload invalid for the command |
| 1012 | `Internal` | maybe | Engine-internal failure |
| 1013 | `EndpointUnavailable` | maybe | Endpoint could not be created/opened |
| 1014 | `Cancelled` | — | Caller cancelled |
| 1015 | `NotImplemented` | no | Command exists; its subsystem is a later gate |

**The wire carries a code and a short message only.** No backtrace, no absolute path, no
internal type name, no raw OS error text that could embed a path. Studio branches on the
code, never on the message text (`docs/ADR/0005`).

---

## 9. Failure behaviour summary

| Condition | Server | Client sees |
|---|---|---|
| Engine not running | Nothing listening | Connect fails (`ENOENT` / `ERROR_FILE_NOT_FOUND`) → `EndpointUnavailable`, retryable |
| Wrong user tries to connect | OS rejects at `CreateFile`/`connect` | `ERROR_ACCESS_DENIED` → `PermissionDenied`; engine unaffected |
| Frame version mismatch | Closes the connection | Connect/read failure; report "update required", do not spin |
| Envelope version mismatch | Typed 1001, then close | Correlated error |
| Oversized frame | Rejects before allocating, closes | 1002 or a closed connection |
| Malformed JSON envelope | Typed 1011 with `request_id: 0`, then close | Error without correlation |
| Unknown command | Typed 1006, **connection stays open** | Correlated error |
| Deferred command | Typed 1015, connection stays open | Correlated error with the gate name |
| Client disconnects | Connection thread ends; engine stays in `IdleReady`. A client process exiting surfaces as `ERROR_BROKEN_PIPE`/`ERROR_PIPE_NOT_CONNECTED`, which maps to `PeerClosed` and is not logged as a warning | — |
| `ShutdownEngine` | Reply, then stop listener and exit `0` | Successful response |

**A failed connection never stops the engine.** Accept and per-connection errors are
logged and the loop continues.

---

## 10. Concurrency and bounds

- One client connection is served at a time at Gate 0, on its own thread.
- Every channel is bounded. There is no `unbounded()` call anywhere in the workspace
  (`docs/ADR/0009`), and `scripts/check.ps1` fails the build if one appears.
- No async runtime is used. Every thread is joined on shutdown with a bounded timeout.
- A slow or stalled client cannot grow engine memory without limit: the frame size is
  capped before allocation.

---

## 11. Not implemented at Gate 0

Stated explicitly so this document is not read as a capability claim:

- No request cancellation delivery (`CancelJob` is refused).
- No server-push events. The protocol is strictly request/response; there is no
  `EngineStateChanged` stream yet, so a client must poll `GetEngineStatus`.
- No diagnostics event buffering behind `GetDiagnosticsSnapshot`.
- No authentication beyond the OS-level endpoint ACL. No token, no nonce.
- Ctrl-C is not intercepted; the graceful stop path is `ShutdownEngine`.
- Named-pipe reads have no timeout, so a wedged engine leaves one blocked OS thread in the
  client. The Studio bounds this to a single in-flight request; recovering needs a Studio
  restart.

## 12. Reference client: the Studio

`apps/discodesk-studio/src-tauri/src/engine_client.rs` is the reference implementation of
this contract. Its behaviour is normative for other clients:

| Property | Behaviour |
|---|---|
| Commands used | `GetEngineStatus` only, at Gate 0 |
| `ShutdownEngine` | **Never sent.** The Studio closing must not stop the engine. A unit test asserts the set of commands actually transmitted |
| Connection reuse | One transport, reused while healthy; dropped on any transport failure and re-established on the next request |
| Concurrency | At most one in-flight request; a single polling interval in the UI |
| Request ids | Start at 1 and increase; a response with a different id is rejected |
| Status caching | None. Every rendered value comes from the most recent response |
| Engine state ownership | None. `dd-core` remains the only writer of engine state |

The Studio's `--probe` mode performs one round-trip and prints the resulting view as JSON,
which is how `scripts/ipc-smoke.ps1` verifies the boundary with the real binaries.
