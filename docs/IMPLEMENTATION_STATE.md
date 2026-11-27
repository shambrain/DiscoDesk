# DiscoDesk implementation state

**Last updated:** 2026-11-27
**Status:** Gate 0 scaffold implemented and verified on the Windows development host.
**One Gate 0 acceptance item is NOT met:** there is no `discodesk-studio` binary yet, so
"engine and Studio are separate processes" cannot be demonstrated. See §4.

Evidence in this document was produced by running the commands in §3 on the host
described in `docs/PLATFORM_SUPPORT.md` §1. Nothing here is inferred from the fact that
a crate compiles.

---

## 1. Repository baseline

- [x] Product concept and scope documented.
- [x] Research and architecture documents added.
- [x] OpenCode project instructions added.
- [x] Coordinator and specialist agent definitions added.
- [x] Reusable project skills added.
- [x] Rust workspace scaffolded and building.
- [x] Gate 0 CI defined (format, lint, test, build, dependency audit).
- [x] Local IPC contract implemented (framing, envelopes, named pipe, error codes).
- [x] Engine state machine and lifecycle implemented.
- [x] Privacy-preserving diagnostics implemented.
- [x] `discodesk-engine` binary implemented and verified over a real pipe.
- [x] Visual Studio 2026 Open Folder integration verified (`-Action Doctor` exits 0).
- [ ] **Tauri Studio scaffolded.** Not created. Gate 0's "separate processes" criterion
      is therefore unmet.
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

## 2. What actually exists

| Path | Contents | Verified by |
|---|---|---|
| `crates/dd-ipc` | 10-byte frame codec (`DDF1` + version + length), 1 MiB bound asserted before allocation, JSON request/response envelopes, 16 stable error codes, command enum, Windows named-pipe transport with a protected DACL, Unix domain socket transport | 49 unit tests |
| `crates/dd-core` | `EngineState` (all 16 states from `ARCHITECTURE.md` §11 declared; 4 reachable at Gate 0), explicit transition table, `Engine` with idempotent `start`/`stop` and poison-recovering state lock | 26 unit tests |
| `crates/dd-diagnostics` | `DiagnosticEvent`, `EventSink`, `BoundedEventBuffer`, path redaction and hashing, opt-in rotating log sink that writes nothing by default | 18 unit tests + 1 doctest |
| `apps/discodesk-engine` | CLI (`--log-level`, `--log-dir`, `--print-endpoint`, `--endpoint`), IPC server, command dispatch, clean shutdown | 5 unit tests + 16-assertion process smoke test |

**Deliberately absent:** `dd-audio`, `dd-dsp`, `dd-analysis`, `dd-renderer`, `dd-package`,
`dd-steam`, `dd-platform` (including `crates/dd-platform/src/windows`), and
`apps/discodesk-studio`. `scripts/check.ps1` and CI fail if a deferred crate directory
appears.

`dd-platform` was **not** created. `ARCHITECTURE.md` calls for a `DesktopHost` trait, but
at Gate 0 it would have no implementation and no caller, which `AGENTS.md` forbids. The
trait belongs with its first adapter at Gate 3.

---

## 3. Verification evidence

Command and result, on Windows 10 build 19045, Rust 1.100.0 `x86_64-pc-windows-msvc`,
MSVC 14.52.37025, Windows SDK 10.0.26100.0.

### 3.1 Canonical check (also what CI runs)

```powershell
powershell -ExecutionPolicy Bypass -File scripts/check.ps1
```

| Step | Result |
|---|---|
| Gate 0 deferred-crate guard | ok: no deferred crate directories present |
| Unbounded-channel guard | ok: no unbounded channels |
| `cargo fmt --all -- --check` | pass (exit 0) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | pass, **0 warnings** |
| `cargo test --workspace --all-features` | **98 passed, 0 failed, 0 ignored** (+ 1 doctest passed) |
| `cargo build --workspace` | pass |
| IPC smoke test | **passed**, 16/16 assertions |

Per-crate test counts: `dd-core` 26, `dd-diagnostics` 18, `dd-ipc` 49, `discodesk-engine`
5 (sum 98), plus 1 doctest in `dd-diagnostics`.

### 3.2 Builds

| Command | Result |
|---|---|
| `cargo build --workspace` | pass |
| `cargo build --workspace --release` | pass (optimized, `strip = "symbols"`) |
| `Invoke-DiscoDeskVS.ps1 -Action BuildDebug` | pass |
| `Invoke-DiscoDeskVS.ps1 -Action BuildSymbols` | pass (`release-with-symbols`, 19.9 s) |

Artifacts and symbols:

| Path | Size |
|---|---|
| `target/debug/discodesk-engine.exe` | 4,816,384 B |
| `target/debug/discodesk_engine.pdb` | 39,563,264 B |
| `target/release-with-symbols/discodesk-engine.exe` | 1,370,624 B |
| `target/release-with-symbols/discodesk_engine.pdb` | 20,631,552 B |

> Rust names the PDB after the **underscored** crate name (`discodesk_engine.pdb`), not the
> binary name. A file-existence check for `discodesk-engine.pdb` fails wrongly. The
> `release` profile strips symbols by design, so it emits no usable PDB.

### 3.3 IPC end-to-end smoke test

`scripts/ipc-smoke.ps1` starts the real `discodesk-engine.exe`, connects over the real
named pipe, and asserts:

| # | Assertion | Result |
|---|---|---|
| 1 | `--version` prints a name and version | ok |
| 2 | Engine reports readiness | ok |
| 3 | Client connects to the named pipe | ok |
| 4 | Response frame version is 1 | ok |
| 5 | `request_id` is echoed | ok |
| 6 | `GetEngineStatus` state is `IdleReady` | ok |
| 7 | `isRunning` is true | ok |
| 8 | `surfaceCount` is 0 | ok |
| 9 | Capabilities marked `probed: false` | ok |
| 10 | `wallpaperHosting` is `unknown`, not `supported` | ok |
| 11 | `GetAudioSources` → code 1015 (`NotImplemented`) | ok |
| 12 | The refusal names Gate 1 | ok |
| 13 | Unknown command → code 1006, connection stays open | ok |
| 14 | `ShutdownEngine` is acknowledged | ok |
| 15 | Engine process exits | ok |
| 16 | Engine exit code is 0 | ok |

### 3.4 Visual Studio environment

```powershell
powershell -ExecutionPolicy Bypass -File scripts/vs/Invoke-DiscoDeskVS.ps1 -Action Doctor
```

Exit code 0. Resolved: Build Tools 2026 `18.12.1`, MSVC `14.52.37025`, `cl`,
`link`, `msbuild`, `cmake`, Windows SDK 10.0.26100.0, Rust 1.100.0 MSVC, WebView2
158.0.4370.33, IDE `C:\Program Files\Microsoft Visual Studio\18\Insiders`.

### 3.5 Dependency licence and advisory gate

```powershell
cargo deny check all
```

`cargo-deny 0.22.1` installed locally; **exit code 0**.

| Result | Detail |
|---|---|
| Errors | none |
| Warnings | 7 × `license-not-encountered` (allow-list entries no crate currently uses: `0BSD`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `MPL-2.0`, `Unicode-DFS-2016`) and 1 × `duplicate` (`syn` 2.0.119 and 3.0.10, expected in the proc-macro ecosystem) |
| Advisories | none outstanding |
| Sources | crates.io only |

The `license-not-encountered` warnings are informational: they confirm that no crate in
the graph uses those licences. They are not suppressed, because the allow list is a
policy statement rather than a description of today's graph.

`THIRD_PARTY_NOTICES.md` is now generated from `cargo deny list` rather than being a
placeholder.

### 3.6 Not verified

| Item | Status |
|---|---|
| CI actually running green | **NOT YET.** CI is defined and every command is locally verified, but no workflow run has been observed against GitHub. |
| Studio/engine separation | **NOT DEMONSTRABLE.** No Studio binary exists yet. |
| Native debugger breakpoints/PDB load in the IDE | **NOT VERIFIED.** PDBs exist and `devenv.exe` is installed, but no interactive debug session was performed (that requires a human at the IDE). |
| CPU/memory profiling | **NOT VERIFIED.** No profiling session was run; no performance numbers are claimed anywhere. |
| Ctrl-C graceful shutdown | **NOT IMPLEMENTED.** The documented stop path is `ShutdownEngine`. |

---

## 4. Gate 0 acceptance status

| Directive §9 criterion | Status |
|---|---|
| Check/build/test on the actual supported Windows target | **Met** — §3.1, §3.2 |
| CI reproduces the same commands | **Defined, not yet observed green** |
| Engine and Studio are separate processes | **NOT MET** — no Studio binary |
| IPC ownership, versioning, permissions, error behaviour documented | **Met** — `docs/IPC_PROTOCOL.md` |
| IPC behaviour covered by tests | **Met for frame/envelope/command/ACL**; version-mismatch at transport level is documented as "close connection", not tested end to end |
| No unbounded background task | **Met** — no async runtime, guards in `check.ps1` |
| No placeholder marked complete | **Met** — deferred crates absent; capability probes report `unknown` |
| Format/lint/test/security configuration | **Met** — `cargo deny check all` exits 0 |
| Licence and third-party notices with verified contents | **Met** — `THIRD_PARTY_NOTICES.md` generated from `cargo deny list`; `LICENSE` honestly marked undecided |
| `PLATFORM_SUPPORT.md` | **Met** — measured matrix |
| Fixture conventions and integration harness | **Met** — `scripts/ipc-smoke.ps1` is the integration harness |

**Conclusion: Gate 0 is not closed.** The remaining blockers are the missing Studio
scaffold and an observed-green CI run. Gate 1 (audio capture) must not begin until both
are resolved.
---

## 5. Defects found and fixed during verification

Recorded because each was a real bug that a compile-only check would have hidden:

1. `dd-ipc/src/frame.rs` had `};` where `});` was required — the crate had never compiled.
2. `transport/mod.rs` did not declare the `windows`/`unix` submodules.
3. `SetEffectSettingsPayload` derived `Eq` while containing `f32`.
4. `windows.rs` was written against a different `windows` crate API: 22 errors. Notably
   `PIPE_ACCESS_DUPLEX` is in `Storage::FileSystem`, `ReadFile`/`WriteFile` take slices,
   `CreateFileW` returns `Result`, and `CreateNamedPipeW` takes `SECURITY_ATTRIBUTES`.
5. `tracing-subscriber` was missing the `env-filter` feature.
6. `dd-diagnostics::build_file_writer` passed a byte count to `max_log_files` and dropped
   the `WorkerGuard` returned by `non_blocking`, which would have silently stopped the
   log writer.
7. `clippy.toml` contained keys that are hard errors (`max-file-size`, `fix`).
8. **Engine bug:** the pipe was created with `nMaxInstances = 1`, so creating the
   replacement listening instance failed and the just-accepted connection was dropped.
9. **Engine bug:** `DisconnectNamedPipe` in `Drop` discards unread data, losing the
   `ShutdownEngine` acknowledgement.
10. **Contract bug:** the engine closed the connection after an unknown command; the
    documented behaviour is to keep it open.
11. Two unit tests encoded wrong assumptions: `EnvFilter` accepts bare target names, and
    a pipe name legitimately starts with `\\.\pipe\`.
12. **`deny.toml` did not match the cargo-deny 0.22.1 schema.** `[licenses] unknown` and
    `[licenses] deny` were removed upstream (the check is deny-by-default), and
    `[advisories] unmaintained`/`unsound` take a scope (`all|workspace|transitive|none`),
    not a lint level. Corrected, with the GPL/AGPL/LGPL/CC-BY-NC refusal preserved by
    their absence from the allow list.
13. **`[licenses.private] ignore` did not apply** because the workspace crates lacked
    `publish = false`. Added it — the crates genuinely are unpublished.
14. **Wildcard dependencies:** the intra-workspace `path` dependencies had no `version`,
    which is an implicit `*`. Added explicit `version = "0.1.0"` instead of weakening the
    `wildcards = "deny"` check.

---

## 6. Newly specified (not implemented)

- [ ] M2 per-monitor wallpaper observer and authoritative reconciliation.
- [ ] M2 content-hash cache reuse on wallpaper change.
- [ ] M2 opt-in automatic reaction policy.
- [ ] M2 native Magic Wand with tray/keyboard fallback.
- [ ] M2 native local notifications with cooldown/deduplication.
- [ ] M3 editable local creator presets and .ddpreset.
- [ ] M3 Steam Workshop preset publishing/subscription.
- [ ] Calm Studio design system and full accessibility review.

These remain unchecked and deferred until M1 Windows MVP stabilization.

---

## 7. Next actions, in order

1. Scaffold `apps/discodesk-studio` as a minimal Tauri v2 shell that connects to the
   engine and displays connection state only. This closes the last Gate 0 technical gap.
2. Observe one green CI run and record the run URL.
3. Run `cargo deny check all` for real and generate `THIRD_PARTY_NOTICES.md`.
4. Decide the project licence (see `docs/GATE0_PLAN.md` D1).
5. Only then begin Gate 1.
