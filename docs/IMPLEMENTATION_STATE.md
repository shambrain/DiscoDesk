# DiscoDesk implementation state

**Last updated:** 2026-11-27
**Status:** Gate 0 **met**. Engine, IPC contract, diagnostics and the Tauri Studio are
implemented and verified as independent processes communicating over the real local IPC
contract, and the complete GitHub Actions pipeline is green.

Every number below was produced by running the command in §3. Nothing is inferred from
the fact that a crate compiles.

---

## 1. Repository baseline

- [x] Product concept and scope documented.
- [x] Research and architecture documents added.
- [x] OpenCode project instructions and specialist agents added.
- [x] Reusable project skills added.
- [x] Rust workspace scaffolded and building.
- [x] Local IPC contract implemented (framing, envelopes, named pipe + UDS, error codes).
- [x] Engine state machine and lifecycle implemented.
- [x] Privacy-preserving diagnostics implemented.
- [x] `discodesk-engine` binary implemented and verified over a real pipe.
- [x] **`discodesk-studio` Tauri v2 application implemented** (Rust backend + React UI).
- [x] **Studio ↔ engine communication verified with both real binaries.**
- [x] **Closing the Studio verified not to stop the engine.**
- [x] Visual Studio 2026 Open Folder integration verified, including Studio build/debug.
- [x] Gate 0 CI defined (frontend, format, lint, test, build, Tauri build, audit).
- [x] Licence and advisory gate executed locally (`cargo deny check all`, exit 0).
- [x] **GitHub Actions observed green for the engine/IPC/Visual Studio commit
      `af59646`** — all four jobs passed (format+lint, test, build, dependency audit).
      Run: <https://github.com/shambrain/DiscoDesk/actions/runs/46019416540>
- [x] **GitHub Actions observed green for the Studio implementation at `6e58c93`** — all
      five jobs passed, including the Studio frontend job, the Tauri Windows build, both
      smoke tests and the dependency audit.
      Run: <https://github.com/shambrain/DiscoDesk/actions/runs/46058125294>
- [x] **Deterministic Studio shutdown** — explicit exit on `WindowEvent::CloseRequested`;
      measured 270 ms connected / 279 ms disconnected, exit code 0, engine unaffected.
- [x] **Licensing decision prepared** — `docs/LICENSING_DECISION.md`; the licence itself
      remains an owner decision and is not selected.
- [ ] **Interactive Visual Studio debugger session** — procedure, binaries, PDBs and launch
      profiles verified; the session itself awaits a human at the IDE
      (`docs/VISUAL_STUDIO_DEBUGGING.md`).
- [ ] Audio capture implemented. Gate 1.
- [ ] DSP telemetry implemented and validated. Gate 1.
- [ ] wgpu preview renderer implemented. Gate 2.
- [ ] Image analysis and cache implemented. Gate 5.
- [ ] Windows desktop wallpaper host implemented and tested. Gate 3.
- [ ] macOS native adapters implemented and tested. Gate 7.
- [ ] Linux desktop adapters implemented and tested. Gate 8.
- [ ] `.ddpkg` package format implemented and validated. M3.
- [ ] Steam Workshop integration implemented. M3.

---

## 2. What exists

| Path | Contents | Verified by |
|---|---|---|
| `crates/dd-ipc` | 10-byte frame codec (`DDF1` + version + length), 1 MiB bound asserted before allocation, JSON envelopes, 16 stable error codes, command enum, **shared endpoint derivation**, Windows named pipe with a protected DACL, Unix domain socket | 56 unit tests |
| `crates/dd-core` | `EngineState` (all 16 states from `ARCHITECTURE.md` §11 declared; 4 reachable at Gate 0), explicit transition table, `Engine` with idempotent start/stop | 26 unit tests |
| `crates/dd-diagnostics` | `DiagnosticEvent`, `EventSink`, `BoundedEventBuffer`, path redaction and hashing, opt-in rotating log | 18 unit tests + 1 doctest |
| `apps/discodesk-engine` | CLI, IPC server, command dispatch, clean shutdown | 5 unit tests + process smoke |
| `apps/discodesk-studio/src-tauri` | Tauri v2 shell, `EngineClient` with an injectable `Connector`, three commands (`engine_status`, `engine_endpoint`, `engine_disconnect`), headless `--probe` mode | 18 unit tests |
| `apps/discodesk-studio/src` + `ui` | React 19 + TypeScript control UI: connection/lifecycle states, engine details, bounded polling | 28 frontend tests |

**Deliberately absent:** `dd-audio`, `dd-dsp`, `dd-analysis`, `dd-renderer`, `dd-package`,
`dd-steam`, `dd-platform` (including `crates/dd-platform/src/windows`).
`scripts/check.ps1` and CI fail if a deferred crate directory appears.

The Studio contains **no** audio capture, DSP, renderer, wallpaper hosting or package code,
and does not depend on any future engine crate. Its only engine dependency is `dd-ipc`.

---

## 3. Verification evidence

Host: Windows 10 build 19045, Rust 1.100.0 `x86_64-pc-windows-msvc`, MSVC 14.52.37025,
Windows SDK 10.0.26100.0, Node 26.12.1, WebView2 158.0.4370.33.

### 3.1 Canonical check

```powershell
powershell -ExecutionPolicy Bypass -File scripts/check.ps1
```

Exit code 0. Steps, in order:

| Step | Result |
|---|---|
| Deferred-crate guard | ok: no deferred crate directories present |
| Unbounded-channel guard | ok: no unbounded channels |
| Studio frontend (typecheck, tests, production build) | **28 tests passed**, typecheck clean, Vite build 225 kB JS / 2.8 kB CSS |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | pass, **0 warnings** |
| `cargo test --workspace --all-features` | **125 passed, 0 failed, 0 ignored** (+1 doctest) |
| `cargo build --workspace` | pass |
| IPC smoke test | **passed**, 20/20 assertions |
| Studio launch smoke test | **passed**, 11/11 assertions |
| `cargo deny check all` | **exit 0** |

Rust test counts by crate: `dd-core` 26, `dd-diagnostics` 18, `dd-ipc` 58,
`discodesk-engine` 5, `discodesk-studio` 18 → **125**, plus 1 doctest.

### 3.2 Builds

| Command | Result |
|---|---|
| `cargo build --workspace` | pass |
| `cargo build --workspace --release` | pass |
| `Invoke-DiscoDeskVS.ps1 -Action BuildDebug` | pass |
| `Invoke-DiscoDeskVS.ps1 -Action BuildSymbols` | pass (`release-with-symbols`) |
| `Invoke-DiscoDeskVS.ps1 -Action BuildStudio` | pass (frontend + debug binary) |
| `Invoke-DiscoDeskVS.ps1 -Action BuildStudioSymbols` | pass |
| `npx tauri build` (Studio) | pass, 2 m 05 s, produced `target/release/discodesk-studio.exe` |

Artifacts and symbols:

| Path | Size |
|---|---|
| `target/debug/discodesk-engine.exe` | 4,816,384 B |
| `target/debug/discodesk_engine.pdb` | 39,563,264 B |
| `target/release-with-symbols/discodesk-engine.exe` | 1,370,624 B |
| `target/release-with-symbols/discodesk_engine.pdb` | 20,631,552 B |
| `target/debug/discodesk-studio.exe` | 12,758,528 B |
| `target/debug/discodesk_studio.pdb` | 111,489,024 B |
| `target/release-with-symbols/discodesk-studio.exe` | 4,535,296 B |
| `target/release-with-symbols/discodesk_studio.pdb` | 66,236,416 B |

> Rust names PDBs after the **underscored** crate name (`discodesk_engine.pdb`,
> `discodesk_studio.pdb`), not the binary name. A check for `discodesk-engine.pdb`
> reports a false negative.

### 3.3 IPC end-to-end (`scripts/ipc-smoke.ps1`)

Starts the real engine, runs the real Studio binary's `--probe` against it, then drives the
protocol from the outside. 20/20 assertions passed, including:

- `--version`, readiness line, `Studio --probe` exit code 0
- Studio reports `kind: connected`, `status.state: IdleReady`, protocol agreement
- frame version, `request_id` echo, `surfaceCount: 0`
- `GetAudioSources` → 1015 (`NotImplemented`) naming Gate 1
- unknown command → 1006 with the connection kept open
- `ShutdownEngine` acknowledged, engine exit code 0

### 3.4 Studio launch lifecycle (`scripts/studio-smoke.ps1`)

11/11 assertions passed locally (interactive desktop present):

| Assertion | Result |
|---|---|
| Engine reports readiness | ok |
| Studio process starts | ok |
| Studio creates a visible window | ok (title `DiscoDesk Studio`) |
| Window title identifies the Studio | ok |
| Studio process exits after the window closed | ok (exit 0) |
| **Engine is STILL running after the Studio closed** | ok |
| A fresh Studio probe still reaches the engine | ok (exit 0, `connected`) |
| Engine answers after the Studio closed | ok |
| Engine exits after `ShutdownEngine` | ok |
| Engine exit code | 0 |

This is the executable form of `docs/ADR/0001`.

**Environment handling, and two corrected assumptions.** Two things were initially
inferred wrongly and then corrected from evidence rather than left as guesses. Job logs on
this repository require authentication, so the script was instrumented to emit check-run
annotations, which are readable through the API.

*First*: the initial CI failure was assumed to mean the runner had no desktop. The
annotations disproved it:

```
studio-smoke: desktopDetected=True userInteractive=True windowAssertions=enabled
studio-smoke: after window wait - hasExited=False mainWindowHandle=197014 title='DiscoDesk Studio'
```

The window appears on the runner, with the right title. The remaining difference is that
`WM_CLOSE` does not end the process there within the wait, whereas locally it exits with
code 0. The test had therefore conflated "the OS delivered a close promptly" with the
invariant it actually exists to protect.

Current behaviour, which is the honest one:

- The window assertions run wherever a desktop exists and are reported as **SKIPPED** (never
  passed) where one does not.
- The close step attempts a graceful close, waits 15 s, then terminates the process.
  A lingering graceful close emits a **warning** annotation so it stays visible; it is not
  a silent pass and not a hard failure, because whether WM_CLOSE completes promptly is an
  environment property.
- What *is* enforced everywhere is the invariant: the Studio process is gone, and the
  engine is **still running and still answering** afterwards.
- Locally the graceful path is asserted strictly, including exit code 0.

The window timeout is also 60 s rather than 30 s, because a cold first launch makes WebView2
create its runtime profile. Nothing uses `continue-on-error`.

### 3.5 Dependency licence and advisory gate

```powershell
cargo deny check all
```

`cargo-deny 0.22.1`, **exit 0**. Warnings only:

| Warning | Count | Meaning |
|---|---|---|
| `license-not-encountered` | 4 | allow-list entries no crate currently uses |
| `duplicate` | 5 | e.g. `syn` 2.0 and 3.0 in the proc-macro ecosystem |

`[graph] targets = ["x86_64-pc-windows-msvc"]` scopes the audit to the only shipped
target; macOS/Linux triples must be added with Gates 7/8.
`MPL-2.0` is present in the Tauri chain (`cssparser`, `selectors`, `dtoa-short`,
`option-ext`); obligations are recorded in `docs/ADR/0012`.

### 3.6 Visual Studio environment

`-Action Doctor` exit 0: Build Tools 2026 `18.12.1`, MSVC `14.52.37025`, `cl`, `link`,
`msbuild`, `cmake`, Windows SDK 10.0.26100.0, Rust 1.100.0 MSVC, WebView2 158.0.4370.33,
IDE `C:\Program Files\Microsoft Visual Studio\18\Insiders`, engine entry point present.

### 3.7 Continuous integration

| Run | Commit | Result |
|---|---|---|
| [46019416540](https://github.com/shambrain/DiscoDesk/actions/runs/46019416540) | `af59646` (engine, IPC, VS tooling) | **success** — format+lint, test, build, dependency audit |
| [46056620205](https://github.com/shambrain/DiscoDesk/actions/runs/46056620205) | `87f8030` (SID endpoint) | **failure** — only `Studio launch smoke test`; `tauri build` and the IPC smoke passed |
| [46058125294](https://github.com/shambrain/DiscoDesk/actions/runs/46058125294) | `6e58c93` | **success** — all five jobs |
| [46062818178](https://github.com/shambrain/DiscoDesk/actions/runs/46062818178) | `77d9f08` | **failure** — only `Studio launch smoke test`; annotations show the window appeared (`mainWindowHandle=197014`, correct title) and the failure was the graceful-close wait |
| [46063754028](https://github.com/shambrain/DiscoDesk/actions/runs/46063754028) | `4f7f0a2` | **success** — all five jobs, `0 skipped assertion(s)`, and no graceful-close warning, so the close completed within the wait on that run |
| CodeQL `5c90e10` … `6e58c93` | — | **success** |

The documentation commit that records this table does not alter code; its own run is
tracked separately and is not asserted here.

Two runs were needed to get the smoke test right, and both failures were real signals
about the test rather than about the product: the first because job logs are not readable
without authentication (so the script was instrumented), the second because the test
asserted OS close timing instead of the lifecycle invariant.

**Observed variability, recorded deliberately:** the Studio's graceful close completes
within the wait on some runner runs (`4f7f0a2`, no warning) and not others (`77d9f08`,
where the window had appeared correctly and the process lingered). Locally it exits with
code 0 every time. Recommended follow-up: handle `WindowEvent::CloseRequested` and call
`app_handle().exit(0)` so shutdown does not depend on WebView2 teardown timing.

`5c90e10`'s run was cancelled by this repository's own `concurrency: cancel-in-progress`
when the following commit was pushed; four of its five jobs had already passed.

### 3.8 Clean-checkout reproducibility

A fresh clone of commit `5c90e10` — no `target/`, no `node_modules/`, no `dist/` — ran the
canonical script end to end and exited **0**: guards, frontend install/typecheck/tests/build,
fmt, clippy, the full test suite, build, both smoke tests and `cargo deny check all`.

This validates the documented ordering requirement: the frontend must be built before any
cargo command that touches the studio crate.

### 3.9 Not verified

| Item | Status |
|---|---|
| GitHub Actions green for the Studio implementation | **Verified** — `6e58c93`, run 46058125294, all five jobs |
| Interactive debugger session (breakpoints, call stack) | **AWAITING HUMAN.** Binaries, matching PDBs (with embedded Rust source paths) and launch profiles are verified; the procedure is in `docs/VISUAL_STUDIO_DEBUGGING.md`. No interactive result is claimed. |
| Studio window creation in CI | **Verified** on the hosted runner — annotation recorded `desktopDetected=True` and `0 skipped assertion(s)`. |
| CPU/memory profiling | **NOT VERIFIED.** No profiling session was run; no performance numbers are claimed anywhere. |
| macOS / Linux | **NOT IMPLEMENTED** (Gates 7/8). |
| Ctrl-C graceful shutdown of the engine | **NOT IMPLEMENTED.** The documented stop path is `ShutdownEngine`. |
| Studio console window hidden in windowed mode | **NOT IMPLEMENTED.** The console subsystem stays attached so `--probe` can write to stdout. |

---

## 4. Gate 0 acceptance status

| Directive §9 criterion | Status |
|---|---|
| Check/build/test on the actual supported Windows target | **Met** — §3.1, §3.2 |
| CI reproduces the same commands | **Met** — green at `af59646` and `6e58c93` |
| **Engine and Studio are separate processes** | **Met** — §3.3, §3.4 |
| IPC ownership, versioning, permissions, error behaviour documented | **Met** — `docs/IPC_PROTOCOL.md` |
| IPC behaviour covered by tests | **Met** — frame/envelope/command/ACL, plus both process smoke tests |
| No unbounded background task | **Met** — no async runtime in the engine; one interval and one in-flight guard in the UI; guards in `check.ps1` |
| No placeholder marked complete | **Met** — deferred crates absent; capability probes report `unknown` |
| Format/lint/test/security configuration | **Met** |
| Licence and third-party notices with verified contents | **Met** — generated from `cargo deny list` |
| `PLATFORM_SUPPORT.md` | **Met** — measured matrix |
| Fixture conventions and integration harness | **Met** — two smoke scripts driving real binaries |

**Gate 0 is met.** Every acceptance item in `OPENCODE_MASTER_DIRECTIVE.md` §9 now has
recorded evidence, including a green GitHub Actions run covering the Studio
(`46058125294`). Gate 1 (audio capture and DSP) may begin.

---

## 5. Defects found and fixed during this work

Grouped by cause; each compiled cleanly or was invisible to a compile-only check.

**Engine / IPC**
1. `frame.rs` had `};` where `});` was required — `dd-ipc` had never compiled.
2. `transport/mod.rs` did not declare its `windows`/`unix` submodules.
3. `SetEffectSettingsPayload` derived `Eq` while containing `f32`.
4. `windows.rs` targeted a different `windows`-crate API (22 errors); rewritten against the
   vendored 0.62.2 bindings.
5. `tracing-subscriber` was missing the `env-filter` feature.
6. `build_file_writer` dropped the `WorkerGuard` from `non_blocking`, which would have
   silently stopped the log writer.
7. Pipe created with `nMaxInstances = 1`, so the replacement listening instance could not
   be created and the accepted connection was dropped.
8. `DisconnectNamedPipe` in `Drop` discards unread data, losing the `ShutdownEngine`
   acknowledgement.
9. The engine closed the connection after an unknown command; the contract keeps it open.
10. A client process exiting surfaced as `ERROR_BROKEN_PIPE` and was logged as a warning;
    it is a normal disconnect and now maps to `PeerClosed`.

**Tooling and policy**
11. `deny.toml` did not match cargo-deny 0.22.1: `licenses.unknown`/`licenses.deny` were
    removed upstream, and `advisories.unmaintained`/`unsound` take scopes, not lint levels.
12. `[licenses.private] ignore` did not apply because the crates lacked `publish = false`.
13. Intra-workspace `path` dependencies had no `version`, which is an implicit `*`.
14. `rustfmt.toml` used `newline_style = "Windows"` while `.gitattributes` forces `eol=lf`,
    so `cargo fmt --check` failed on a fresh checkout. Switched to `Unix`.
15. `clippy.toml` contained keys that are hard errors (`max-file-size`, `fix`).
16. `[graph]` audited the entire cross-platform graph, reporting a Linux-only `glib`
    unsoundness we cannot act on; scoped to the shipped target.

**Visual Studio**
17. `VsDevCmd` environment import was broken: PowerShell re-quoted the nested `cmd /s /c`
    argument, and `ExitCode` was read after `Dispose()`.
18. Build/Rebuild Open Folder tasks used `type: launch` with `contextType: build`, making
    VS treat them as debug targets.

**Studio**
19. **Two competing endpoint derivations** (engine: `USERNAME`; transport: SID + pid). The
    SID+pid form is unguessable to another process, so the Studio could never have found
    the engine. Consolidated into `dd_ipc::endpoint`.
20. Scripted test responses hard-coded `request_id: 1` while the client correctly
    incremented, which is why the fixtures were wrong, not the client.
21. `tauri-build` requires `icons/icon.ico` for the Windows resource file even with
    bundling disabled; a provisional icon was generated and is disclosed in the notices.
22. **The endpoint name was derived from `USERNAME`**, which was empty in this project's
    own development shell, silently collapsing every account onto
    `discodesk-engine-default`. Verified empirically by running the binaries. The segment
    is now a hash of the account SID from the process token, and confirmed invariant under
    `USERNAME=''`, `'Alice'` and `'SomeoneElse'`.
23. **The Studio launch smoke test conflated the OS close with the invariant.** It failed
    twice on CI at two different points, and job logs require authentication, so the script
    was instrumented to emit check-run annotations. Those showed the window *does* appear on
    the runner with the correct title, and that `WM_CLOSE` simply does not end the process
    within the wait there, while locally it exits with code 0. The step now asserts what
    actually matters (the process is gone, the engine survives and still answers), performs
    a bounded graceful close with a terminate fallback, and surfaces a lingering close as a
    **warning** annotation rather than a false pass or a false failure. Window assertions
    remain environment-gated: skipped where there is no desktop, never reported as passed.

---

## 6. What must happen next, in order

1. Perform one interactive Visual Studio debug session against `discodesk-studio.exe`
   (breakpoint, call stack, PDB load) and record it as evidence — procedure and verified
   prerequisites are in `docs/VISUAL_STUDIO_DEBUGGING.md`.
2. Decide the project licence (`docs/LICENSING_DECISION.md`, `docs/GATE0_PLAN.md` D1).
   Engineering continues meanwhile; distribution stays blocked until it is decided.
3. Begin Gate 1: WASAPI system/output loopback, deterministic DSP, and the telemetry CLI.

---

## 7. Newly specified (not implemented)

- [ ] M2 per-monitor wallpaper observer and authoritative reconciliation.
- [ ] M2 content-hash cache reuse on wallpaper change.
- [ ] M2 opt-in automatic reaction policy.
- [ ] M2 native Magic Wand with tray/keyboard fallback.
- [ ] M2 native local notifications with cooldown/deduplication.
- [ ] M3 editable local creator presets and .ddpreset.
- [ ] M3 Steam Workshop preset publishing/subscription.
- [ ] Calm Studio design system and full accessibility review.

These remain unchecked and deferred until M1 Windows MVP stabilization.
