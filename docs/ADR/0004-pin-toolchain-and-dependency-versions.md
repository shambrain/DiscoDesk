# ADR-0004: Pin the toolchain and dependency versions, and verify each against a primary source

- **Status:** Accepted
- **Date:** 2026-11-27
- **Deciders:** discodesk-lead, research-licensing-auditor
- **Gate:** 0 (repository scaffold)

## Context

`AGENTS.md` requires pinned dependencies and lockfiles, and requires that model weights
and third-party dependencies pass commercial-licence review. `OPENCODE_MASTER_DIRECTIVE.md`
§4 requires recording a source URL, access date, version and exact licence identifier
before adding a dependency, and §22 requires exact commands and real pass/fail counts.

The risks at Gate 0 are concrete:

- The development host has **two** MSVC toolsets (Build Tools 2019 with MSVC 14.29, and
  Build Tools 2026 with MSVC 14.51) and two Windows SDKs (19041 and 26100). A build
  that fails with missing symbols is usually a toolset-selection problem, and the
  failure mode must be documented rather than rediscovered.
- The host's Rust is installed but **not on the default `PATH`**. A CI runner and a
  developer's shell therefore differ unless this is handled explicitly.
- Dependency versions chosen from memory can be stale, can have moved their MSRV
  upward, or can carry a licence change between releases.
- Floating version requirements (`"1"`) make builds non-reproducible in practice even
  when a `Cargo.lock` is committed, because `cargo update` silently moves them.

## Options considered

1. **Exact-pinned versions + committed `Cargo.lock` + a versioned `rust-toolchain.toml`
   (chosen).**
2. **Caret requirements only.** Rejected: `cargo update` produces a different build
   with no repository change, which contradicts the reproducibility requirement.
3. **Floating versions.** Rejected outright: non-reproducible, and defeats the audit.
4. **A vendored dependency tree.** Rejected at Gate 0: heavy maintenance cost for a
   project whose dependency set is still small.

## Decision

1. **`rust-toolchain.toml` pins an exact channel** (`1.99.1`), the
   `x86_64-pc-windows-msvc` target, the `rustfmt` and `clippy` components, and
   `profile = "minimal"`. The toolchain is a build input, not a developer preference.
2. **Every dependency is pinned to an exact version** in the workspace manifests, and
   `Cargo.lock` is committed. `Cargo.lock` is the authoritative resolved set.
3. **Before a version is written into a manifest, it is verified against a primary
   source** — the crates.io API, the npm registry, or the upstream repository — and the
   check is recorded in `RESEARCH.md` with the source URL, the access date and the
   SPDX licence identifier. A version that cannot be verified is not pinned to an exact
   number; a conservative constraint is used and the gap is recorded.
4. **Licences are gated in CI.** `deny.toml` restricts the dependency set to permissive
   licences (MIT, Apache-2.0 and its LLVM exception, BSD-2/3-Clause, ISC, Unicode
   licences, and MPL-2.0 only where explicitly justified). `cargo deny check all` runs
   as a required CI job, separate from the compile job so a policy failure is
   distinguishable from a code failure.
5. **The advisory database is checked** by the same `cargo deny` job, against RustSec.
6. **The exact verification command set is shared between local development and CI**:
   `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features
   -- -D warnings`, `cargo test --workspace --all-features`,
   `cargo build --workspace --release`.
7. **The PATH caveat is documented** in the README and in `docs/PLATFORM_SUPPORT.md`,
   and CI installs the toolchain explicitly rather than relying on ambient state.
8. **Warnings are never suppressed** to force a green build. `RUSTFLAGS: "-D warnings"`
   is set in CI and `-D warnings` is passed to clippy locally.

## Consequences

- Easier: a build from a clean checkout is reproducible, and a dependency bump is a
  deliberate, reviewable diff that re-runs the licence and advisory gates.
- Easier: a compiler warning is a CI failure, so it cannot accumulate silently.
- Harder: dependency updates are deliberate work, not a background task. This is the
  intended cost.
- Harder: an exact pin can age out of support. The mitigation is a scheduled review
  and a documented, verified bump procedure.
- Now forbidden: a floating version requirement in a manifest; a version pinned from
  memory without a primary-source check; a licence allowlist edit without re-running
  `cargo deny`; `--cap-lints` or similar warning suppression.

## Verification

- Gate 0: `rust-toolchain.toml` exists and pins the channel; `Cargo.lock` is committed;
  `deny.toml` exists; `cargo deny check all` passes in CI; the four canonical commands
  pass locally and in CI.
- Each dependency entry in `RESEARCH.md` carries a source URL, an access date and an
  SPDX licence identifier.

## References

- `AGENTS.md` — research-first engineering, dependency pinning, licence review.
- `OPENCODE_MASTER_DIRECTIVE.md` §4, §8, §9, §22.
- RustSec advisory database: <https://github.com/rustsec/advisory-db>
- cargo-deny: <https://github.com/EmbarkStudios/cargo-deny>
- crates.io API: <https://crates.io/api/v1/crates/{crate}>
- npm registry API: <https://registry.npmjs.org/{package}>
