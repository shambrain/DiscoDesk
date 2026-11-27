# Gate 0 implementation plan — repository and environment baseline

**Status:** In progress. Scaffold and CI implemented; verification evidence recorded in
`docs/IMPLEMENTATION_STATE.md`.
**Authority:** `OPENCODE_MASTER_DIRECTIVE.md` §9 defines Gate 0. `AGENTS.md` binds every
decision here. `ARCHITECTURE.md` §4, §5, §11 and §16 define the contracts.

This document records *what Gate 0 builds, what it deliberately does not build, and how
each acceptance criterion is proven.* It is the plan of record for the gate; it is not a
claim that the work is finished.

---

## 1. Environment baseline

Measured, not assumed. Full table with the exact probe command per row:
[`PLATFORM_SUPPORT.md`](PLATFORM_SUPPORT.md) §1.

| Item | Value |
|---|---|
| OS | Windows 10 build 19045, x64 |
| Rust | stable **1.100.0**, `x86_64-pc-windows-msvc`, `rustfmt` + `clippy` |
| MSVC | Build Tools 2019 (14.29) **and** 2026 (14.51) |
| Windows SDK | 10.0.19041.0 and 10.0.26100.0 |
| Node / npm | 26.12.1 / 11.20.0 |
| WebView2 Runtime | 158.0.4370.33 |
| GPU | AMD RX 6700 XT + Radeon integrated (driver 32.0.21045.29003) |

Two environment facts shape the plan:

1. **Rust is not on the default `PATH`.** `scripts/check.ps1` resolves it explicitly.
2. **The OS build is 19045**, below the documented 20348 threshold for process loopback.
   Per-application capture must report *unavailable* on this host (Gate 1/6 concern, but
   recorded now so it is never mislabelled).

---

## 2. Scope

### 2.1 In scope

| Area | Deliverable |
|---|---|
| Toolchain | `rust-toolchain.toml` pinning 1.100.0, target and components |
| Workspace | Cargo workspace, `[workspace.package]`, `[workspace.dependencies]` pins |
| Engine | `discodesk-engine` binary: lifecycle, IPC server, diagnostics, CLI |
| Core | `dd-core`: authoritative state machine and typed error taxonomy |
| IPC | `dd-ipc`: versioned framing, envelope, command enum, named-pipe transport |
| Diagnostics | `dd-diagnostics`: structured events, path redaction, rotating log |
| Studio | `discodesk-studio` Tauri v2 shell as a **separate process** showing connection state only |
| CI | Format, lint, test, build, dependency licence/advisory audit |
| Docs | `PLATFORM_SUPPORT.md`, `ADR/` 0001–0009, this plan, updated map and state |
| Legal | `LICENSE` (honestly marked undecided), `THIRD_PARTY_NOTICES.md` |

### 2.2 Explicitly out of scope

Gate 0 must not implement, and must not create empty placeholder crates for:

- audio capture or DSP (`dd-audio`, `dd-dsp`) — Gate 1
- the `wgpu` renderer or any shader (`dd-renderer`, `shaders/`) — Gate 2
- Windows wallpaper hosting (`dd-platform/src/windows`) — Gate 3
- image analysis, depth models or the analysis cache (`dd-analysis`, `models/`) — Gate 5
- `.ddpkg` packaging (`dd-package`) — later
- Steam / Workshop (`dd-steam`) — later
- the wallpaper observer, automatic reaction, Magic Wand or notifications
  (`dd-desktop-automation`) — M2, see `ADR/0006`
- Studio feature UI (Library, Vibe Studio, Diagnostics panes) — Gate 4

**Enforcement:** `scripts/check.ps1` and the CI `test` job fail if a deferred crate
directory or `crates/dd-platform/src/windows` exists. A planned module is not evidence
that code exists (`ADR/0006`), and an empty crate that appears to exist is worse than an
absent one.

---

## 3. Crate graph

```text
discodesk-engine (bin) ──┬─> dd-core ──────────> dd-diagnostics
                         ├─> dd-ipc
                         └─> dd-diagnostics

discodesk-studio (Tauri bin, separate process) ──> (IPC client only; no engine crates)
```

Dependency direction is acyclic. `dd-ipc` never depends on `dd-core`: the command enum and
the error codes live in `dd-ipc`, and `dd-core` consumes them. This keeps the wire contract
independent of engine internals, so a protocol change never forces an engine refactor.

| Crate | Responsibility | Owns state? |
|---|---|---|
| `dd-core` | Engine state machine, lifecycle, authoritative status | **Yes** |
| `dd-ipc` | Framing, envelope, command/error contract, transport | No |
| `dd-diagnostics` | Structured events, redaction, rotating sink | No (owns its writer) |
| `dd-platform` *(contract only)* | `DesktopHost` trait + capability reporting | No |
| `dd-errors` *(see note)* | Shared error taxonomy | No |

**Note on `dd-errors`.** The systems architect proposed a dedicated crate so that
`dd-ipc` and a future `dd-platform` share the taxonomy without depending on `dd-core`.
If the implemented layout instead keeps the taxonomy in `dd-core` with the codes
mirrored in `dd-ipc`, that is acceptable provided there is exactly one source of truth
for the numeric codes and a test asserts the mapping is total. The final layout is
recorded in `docs/IMPLEMENTATION_STATE.md`.

---

## 4. Interfaces

### 4.1 IPC transport

| Property | Decision |
|---|---|
| Windows | Named pipe, **explicit security descriptor** granting only the current user and SYSTEM |
| Unix | Domain socket in a user-owned directory, mode `0600` |
| Server | The **engine**. Studio is always a client and never listens |
| Remote access | Refused (`PIPE_REJECT_REMOTE_CLIENTS`) |
| HTTP | Never. No TCP listener, no wildcard bind |
| Framing | Length-delimited, header carries protocol version and payload length |
| Bounds | A maximum frame size checked **before** allocation |
| Correlation | Request ID on every request/response; cancellation ID for long work |

The default Windows named-pipe DACL grants read access to `Everyone` (Microsoft Learn).
Relying on it would violate `AGENTS.md` and `ADR-0002`, so a protected DACL is
mandatory.

### 4.2 Error taxonomy

One enum, stable numeric codes, converted exactly once at the boundary. The wire carries
`{code, message}` and never a path, backtrace or internal type name (`ADR-0005`).

### 4.3 Engine state machine

`Stopped → Starting → IdleReady`, with `PausedByUser`, `ShuttingDown` and `FatalError`.
Transitions are validated by an explicit table, not scattered booleans (`ADR-0009`).
States from `ARCHITECTURE.md` §11 that Gate 0 cannot reach are **not faked**; they are
added by the gate that makes them reachable.

### 4.4 Process model

The engine outlives Studio. Closing Studio leaves the engine in `IdleReady`. Studio holds
a read-only mirror of engine state and re-reads it on every reconnect (`ADR-0001`).

---

## 5. Acceptance matrix

| Directive §9 requirement | How it is proven |
|---|---|
| Check/build/test on the actual Windows target | `scripts/check.ps1`: fmt, clippy `-D warnings`, test, build — all on `x86_64-pc-windows-msvc` with 1.100.0 |
| CI reproduces the same commands | `.github/workflows/ci.yml` runs the same command list on `windows-2025` |
| Engine and Studio are separate processes | Two distinct binaries; an integration test drives the engine over a real pipe and asserts the Studio-side client fails cleanly when the engine is killed |
| IPC ownership documented | `ADR-0002`, `docs/IPC_PROTOCOL.md` |
| IPC versioning documented | Protocol version in every frame; a mismatch returns a typed error and closes the connection |
| IPC permissions documented | Protected DACL on Windows; `0600` on Unix; a test reads the descriptor back and asserts no `Everyone`/`Users`/`Anonymous` ACE |
| IPC error behaviour documented | A test per failure class: version mismatch, oversized frame, unknown command, malformed payload, engine absent |
| No unbounded background task | No async runtime (`ADR-0009`); every thread joined with a bounded timeout; `check.ps1` fails on `unbounded(` |
| No placeholder marked complete | Deferred-crate guard; `IMPLEMENTATION_STATE.md` checkboxes stay unchecked without evidence |
| Format/lint/test/security config | `rustfmt.toml`, `clippy.toml`, `deny.toml`, CI audit job |
| Licence and notices | `LICENSE` marked undecided rather than inventing a grant; `THIRD_PARTY_NOTICES.md` with verified SPDX ids |
| `PLATFORM_SUPPORT.md` | Measured matrix; unimplemented capabilities are `Unknown`/`Unsupported`, never "supported" |
| Fixtures and integration harness | `tests/` conventions documented; no fixture may contain user content or non-redistributable media |

---

## 6. Verification commands

Canonical, run locally and in CI:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/check.ps1
```

which is exactly:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace
```

Dependency policy, separately (not part of the code build):

```powershell
cargo deny check all
```

`cargo-deny` is installed by CI; if it is absent locally, say so rather than claiming the
check passed (`ADR/0004`).

---

## 7. Open decisions carried forward

| # | Decision | Owner | Effect if unresolved |
|---|---|---|---|
| D1 | **Project licence** (MIT vs Apache-2.0) | product owner | `LICENSE` stays "not decided"; no licence claim in any file |
| D2 | Studio UI framework details (vite/TS version pin) | tauri-ui-engineer | Studio skeleton uses the template defaults; pin recorded when a build passes |
| D3 | Whether `dd-errors` is a crate or a module | systems-architect / rust-engineer | Cosmetic; the numeric codes must have one source of truth either way |

D1 is the only one that blocks a *claim*. It does not block the build, the tests or the
technical scaffold, which is why Gate 0 proceeds with the licence honestly marked
undecided rather than inventing a grant.

---

## 8. Exit criteria

Gate 0 is complete when, and only when:

1. `scripts/check.ps1` passes on the Windows target, with the real counts recorded.
2. CI is green on `windows-2025` for the same commit.
3. The engine and the Studio are demonstrably separate processes.
4. The IPC contract, its versioning, its permissions and its error behaviour are
   documented and covered by tests.
5. `docs/IMPLEMENTATION_STATE.md` reflects only what the evidence supports.
6. No deferred crate exists and no M2 feature is implemented (`ADR/0006`).

Gate 1 (audio capture and DSP) does not begin until these hold.
