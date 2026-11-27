# Architecture Decision Records

This directory records the consequential design decisions for DiscoDesk. An ADR is
required when implementation work begins on a decision that has more than one viable
option, or when research contradicts an existing decision.

Each ADR is immutable once accepted. A changed decision is a **new** ADR that supersedes
the old one; the superseded ADR is marked, not rewritten or deleted.

## Format

```
docs/ADR/NNNN-short-title.md
```

with this structure:

1. **Status** — `Proposed` | `Accepted` | `Superseded by ADR-NNNN` | `Rejected`
2. **Date** and **deciders**
3. **Context** — the user-visible requirement and the constraints that bind the decision
4. **Options considered** — including the option not taken and why
5. **Decision** — what was chosen, stated unambiguously
6. **Consequences** — what becomes easier, what becomes harder, what is now forbidden
7. **Verification** — the test or measurement that proves the decision was implemented
   correctly, and the gate at which it will be checked
8. **References** — primary sources, with access dates

## Index

| ADR | Title | Status | Gate |
|---|---|---|---|
| [0001](0001-rust-owns-the-engine-tauri-owns-studio.md) | Rust owns the engine, Tauri owns Studio | Accepted | 0 |
| [0002](0002-versioned-local-ipc-over-named-pipe-and-uds.md) | Versioned local IPC over named pipe and UDS | Accepted | 0 |
| [0003](0003-windows-first-single-target.md) | Windows-first, single supported target at Gate 0 | Accepted | 0 |
| [0004](0004-pin-toolchain-and-dependency-versions.md) | Pin the toolchain and dependency versions | Accepted | 0 |
| [0005](0005-structured-error-taxonomy-across-ipc.md) | Structured error taxonomy across the IPC boundary | Accepted | 0 |
| [0006](0006-defer-m2-ambient-features-until-m1.md) | Defer M2 ambient features until M1 stabilisation | Accepted | 0 |
| [0007](0007-diagnostics-are-privacy-preserving-by-default.md) | Diagnostics are privacy-preserving by default | Accepted | 0 |
| [0008](0008-studio-frontend-toolchain-and-licence-boundary.md) | Studio frontend toolchain and the runtime-component licence boundary | Accepted | 0 (binding on 4) |
| [0009](0009-threaded-engine-no-async-runtime.md) | A threaded engine with no async runtime at Gate 0 | Accepted | 0 |
| [0010](0010-visual-studio-open-folder-native-debugging.md) | Visual Studio Open Folder integration and native Windows diagnostics | Accepted | 0 |
| [0011](0011-studio-is-a-thin-ipc-client.md) | The Studio is a thin client of the shared endpoint contract | Accepted | 0 |
| [0012](0012-accepting-mpl-2-dependencies.md) | Accepting MPL-2.0 dependencies from the Tauri toolchain | Accepted | 0 |

## Related documents

- `ARCHITECTURE.md` — the design these decisions implement.
- `OPENCODE_MASTER_DIRECTIVE.md` — the gates these decisions are checked against.
- `docs/PLATFORM_SUPPORT.md` — what has actually been verified, and on what hardware.
- `docs/IMPLEMENTATION_STATE.md` — the current acceptance gate.
