# Third-party notices

**Generated:** 2026-11-27  
**Tool:** `cargo-deny 0.22.1`  
**Source of truth:** `Cargo.lock` (committed) and `deny.toml`.

This file lists the licences of the third-party **Rust crates** currently linked into
the DiscoDesk workspace. It is generated from the dependency graph, not maintained by
hand.

Regenerate with:

```powershell
cargo deny list
```

## Licence policy

The allow list lives in `deny.toml`. Anything not explicitly allowed is refused,
including AGPL/GPL/LGPL, CDDL, EPL, OSL, SSPL and every CC-BY-NC variant.
`cargo deny check all` enforces this in CI.

## Rust crates by licence

### Apache-2.0 (45)

- `anstyle@1.0.14`
- `cfg-if@1.0.5`
- `clap@4.6.10`
- `clap_builder@4.6.10`
- `clap_derive@4.6.9`
- `clap_lex@1.1.2`
- `crossbeam-channel@0.5.17`
- `crossbeam-utils@0.8.23`
- `deranged@0.5.9`
- `heck@0.5.0`
- `itoa@1.0.19`
- `lazy_static@1.5.1`
- `num-conv@0.2.2`
- `once_cell@1.21.4`
- `pin-project-lite@0.2.18`
- `powerfmt@0.2.1`
- `proc-macro2@1.0.107`
- `quote@1.0.47`
- `regex-automata@0.4.19`
- `regex-syntax@0.8.12`
- `serde@1.0.229`
- `serde_core@1.0.229`
- `serde_derive@1.0.229`
- `serde_json@1.0.154`
- `symlink@0.1.0`
- `syn@2.0.119`
- `syn@3.0.10`
- `thiserror@2.0.21`
- `thiserror-impl@2.0.21`
- `thread_local@1.1.10`
- `time@0.3.55`
- `time-core@0.1.9`
- `unicode-ident@1.0.27`
- `windows@0.62.2`
- `windows-collections@0.3.2`
- `windows-core@0.62.2`
- `windows-future@0.3.2`
- `windows-implement@0.60.2`
- `windows-interface@0.59.3`
- `windows-link@0.2.1`
- `windows-numerics@0.3.1`
- `windows-result@0.4.1`
- `windows-strings@0.5.1`
- `windows-sys@0.61.2`
- `windows-threading@0.2.1`

### MIT (54)

- `anstyle@1.0.14`
- `cfg-if@1.0.5`
- `clap@4.6.10`
- `clap_builder@4.6.10`
- `clap_derive@4.6.9`
- `clap_lex@1.1.2`
- `crossbeam-channel@0.5.17`
- `crossbeam-utils@0.8.23`
- `deranged@0.5.9`
- `heck@0.5.0`
- `itoa@1.0.19`
- `lazy_static@1.5.1`
- `matchers@0.2.0`
- `memchr@2.8.3`
- `nu-ansi-term@0.50.3`
- `num-conv@0.2.2`
- `once_cell@1.21.4`
- `pin-project-lite@0.2.18`
- `powerfmt@0.2.1`
- `proc-macro2@1.0.107`
- `quote@1.0.47`
- `regex-automata@0.4.19`
- `regex-syntax@0.8.12`
- `serde@1.0.229`
- `serde_core@1.0.229`
- `serde_derive@1.0.229`
- `serde_json@1.0.154`
- `sharded-slab@0.1.7`
- `symlink@0.1.0`
- `syn@2.0.119`
- `syn@3.0.10`
- `thiserror@2.0.21`
- `thiserror-impl@2.0.21`
- `thread_local@1.1.10`
- `time@0.3.55`
- `time-core@0.1.9`
- `tracing@0.1.44`
- `tracing-appender@0.2.5`
- `tracing-core@0.1.36`
- `tracing-subscriber@0.3.23`
- `unicode-ident@1.0.27`
- `windows@0.62.2`
- `windows-collections@0.3.2`
- `windows-core@0.62.2`
- `windows-future@0.3.2`
- `windows-implement@0.60.2`
- `windows-interface@0.59.3`
- `windows-link@0.2.1`
- `windows-numerics@0.3.1`
- `windows-result@0.4.1`
- `windows-strings@0.5.1`
- `windows-sys@0.61.2`
- `windows-threading@0.2.1`
- `zmij@1.0.23`

### Unicode-3.0 (1)

- `unicode-ident@1.0.27`

### Unlicense (1)

- `memchr@2.8.3`

### Unlicensed (4)

- `dd-core@0.1.0`
- `dd-diagnostics@0.1.0`
- `dd-ipc@0.1.0`
- `discodesk-engine@0.1.0`

## Workspace crates

`dd-core`, `dd-ipc`, `dd-diagnostics` and `discodesk-engine` are first-party and are
reported as unlicensed by `cargo deny list`, which is accurate: the project licence is
**not yet decided**. See `LICENSE` and `docs/GATE0_PLAN.md` D1. They are excluded from
the policy check through `[licenses.private] ignore = true` because they are
`publish = false` local crates, not third-party dependencies.

## Runtime components that are NOT open source

These are not Rust crates and are therefore outside the list above, but they ship
with (or are required by) the product and must be disclosed separately:

| Component | Terms | Status |
|---|---|---|
| Microsoft Edge WebView2 Runtime | Proprietary Microsoft EULA | Required at runtime by the Studio; not yet shipped |
| MSVC C/C++ runtime | Microsoft Visual C++ redistributable terms | Statically linked by MSVC builds |
| `WebView2Loader.dll` | Microsoft redistributable | Ships with a Tauri Windows build |

No statement anywhere may claim the shipped application is entirely open source.
