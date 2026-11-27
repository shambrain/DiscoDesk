# CI — DiscoDesk

Gate 0 CI. The workflows here mirror, as exactly as possible, the commands a developer
runs locally. A CI failure must be reproducible locally without guesswork.

## Locally reproducible command set

The canonical local verification sequence is:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"   # Rust is not on the default PATH

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --release
```

CI runs the same commands, in the same order, with the same toolchain channel that
`rust-toolchain.toml` pins.

## Workflows

| Workflow | Trigger | Purpose |
|---|---|---|
| `ci.yml` | push, pull_request, manual | format, lint, test, build, dependency licence/advisory audit |
| `codeql.yml` | push, pull_request, schedule | GitHub CodeQL for Rust |

## What CI deliberately does NOT do at Gate 0

- No Windows desktop-host / wallpaper-injection test. That is Gate 3.
- No audio capture test. That is Gate 1.
- No renderer benchmark or GPU assertion. That is Gate 2.
- No Studio UI end-to-end test. That is Gate 4.

CI at Gate 0 proves only that the repository builds, lints, tests and audits on the
supported Windows target. It is not a product capability claim.
