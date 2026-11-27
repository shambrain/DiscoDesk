# ADR-0003: Windows-first, one supported target at Gate 0

- **Status:** Accepted
- **Date:** 2026-11-27
- **Deciders:** discodesk-lead, systems-architect
- **Gate:** 0 (repository scaffold)

## Context

`ARCHITECTURE.md` describes a cross-platform product, but `AGENTS.md` and
`OPENCODE_MASTER_DIRECTIVE.md` both forbid claiming platform support that has not been
tested on that platform. The directive gates macOS (Gate 7) and Linux (Gate 8) behind
the Windows MVP and the M1 stabilisation soak test.

Building three platform backends at once would triple the surface area before a single
user-visible flow works, and would create a large amount of code that cannot be verified
on the available hardware. The only development host available today is Windows.

The development host runs Windows 10 build **19045**. Two consequences follow:

- Microsoft documents `AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK` as requiring
  Windows 10 build **20348** or later. This host is below that threshold, so
  per-application capture must report as unavailable here, and the system/output mix is
  the baseline.
- `AGENTS.md` forbids labelling the system mix as per-app capture. The capability probe
  must be honest on this build.

## Options considered

1. **Windows-only at Gate 0, with the platform boundary designed but not implemented
   (chosen).**
2. **All three platforms at Gate 0.** Rejected: unverifiable on the available hardware,
   and it would front-load the hardest native work before the product flow exists.
3. **Windows-only, with no platform abstraction.** Rejected: it would bake Windows
   assumptions into the crate boundaries and make Gate 7/8 a rewrite rather than an
   implementation.
4. **macOS or Linux first.** Rejected: no host machine exists for either.

## Decision

1. **Gate 0 builds and tests on Windows only.** The primary target is
   `x86_64-pc-windows-msvc`.
2. The `dd-platform` boundary is designed as a **trait plus a capability-reporting
   type** at Gate 0. The trait documents the contract; no Windows wallpaper-host
   implementation is written at Gate 0. A concrete implementation belongs to Gate 3.
3. macOS and Linux adapters are **deferred to Gates 7 and 8**, behind the M1
   stabilisation gate, exactly as `OPENCODE_MASTER_DIRECTIVE.md` §19–20 require.
4. Platform-specific code is isolated behind `cfg` so a Unix-only defect cannot break
   the Windows build, and the Unix path is compiled and tested in CI when a Unix runner
   is added.
5. The capability-reporting type must be able to express "not probed yet" distinctly
   from "probed and unavailable". A platform that has not been tested is never reported
   as supported.

## Consequences

- Easier: one toolchain, one build matrix and one honest capability matrix during
  Gate 0–6.
- Easier: the Windows desktop-host integration (the highest-risk native work) gets
  full attention before other platforms compete for it.
- Harder: the platform trait must be designed carefully at Gate 0 without a
  second implementation to validate it against. Mitigation: the trait is kept minimal
  and modelled directly on the `DesktopHost` shape in `ARCHITECTURE.md` §10.
- Harder: `docs/PLATFORM_SUPPORT.md` must keep macOS and Linux rows explicitly
  `unverified` so the matrix never implies coverage.
- Now forbidden: a "Linux support" or "macOS support" checkbox based on a cross-compile
  that was never run on that OS; a universal Wayland claim; labelling a build below
  20348 as supporting per-application capture.

## Verification

- Gate 0: `cargo build`, `cargo test` and `cargo clippy` pass on the Windows target;
  CI runs the identical commands on `windows-latest`.
- Gate 1: the capability probe reports `SystemMixOnly` or `Unavailable` on build 19045.
- Gate 7/8: acceptance requires an actual macOS or Linux host, recorded in
  `docs/PLATFORM_SUPPORT.md`.

## References

- `ARCHITECTURE.md` §10 (desktop host adapters), §15 (test matrix).
- `OPENCODE_MASTER_DIRECTIVE.md` §9, §19, §20.
- Microsoft Application Loopback sample (Windows 10 build 20348 or later):
  <https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/>
- `docs/PLATFORM_SUPPORT.md` — the measured host baseline.
