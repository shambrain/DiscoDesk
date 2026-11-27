# Third-party notices

**Generated:** 2026-11-27  
**Tool:** `cargo-deny 0.22.1`  
**Scope:** `x86_64-pc-windows-msvc` (the only shipped target; see `deny.toml`).
**Source of truth:** `Cargo.lock` (committed) and `deny.toml`.

Licences of the third-party **Rust crates** linked into the DiscoDesk workspace,
generated from the dependency graph. This file is not maintained by hand.

Regenerate with:

```powershell
cargo deny list
```

## Licence policy

The allow list lives in `deny.toml`. Everything not explicitly allowed is refused,
including AGPL/GPL/LGPL, CDDL, EPL, OSL, SSPL and every CC-BY-NC variant.
`cargo deny check all` enforces this in CI.

## Rust crates by licence

### 0BSD (1)

- `adler2@2.0.1`

### Apache-2.0 (158)

- `adler2@2.0.1`
- `anstyle@1.0.14`
- `anyhow@1.0.105`
- `autocfg@1.5.1`
- `base64@0.23.1`
- `bitflags@2.13.2`
- `bit-set@0.8.0`
- `bit-vec@0.8.0`
- `block-buffer@0.12.1`
- `camino@1.2.6`
- `cargo_toml@1.0.1`
- `cargo-platform@0.1.9`
- `cc@1.12.0`
- `cfg-if@1.0.5`
- `clap@4.6.10`
- `clap_builder@4.6.10`
- `clap_derive@4.6.9`
- `clap_lex@1.1.2`
- `const-oid@0.10.3`
- `cookie@0.18.3`
- `cpufeatures@0.3.1`
- `crc32fast@1.5.6`
- `crossbeam-channel@0.5.17`
- `crossbeam-utils@0.8.23`
- `crypto-common@0.2.2`
- `ctor@1.0.14`
- `deranged@0.5.9`
- `digest@0.11.3`
- `dirs@7.0.0`
- `dirs-sys@0.5.0`
- `displaydoc@0.2.7`
- `dpi@0.1.2`
- `dtoa@1.0.11`
- `dunce@1.0.5`
- `dyn-clone@1.0.20`
- `equivalent@1.0.3`
- `erased-serde@0.4.11`
- `fastrand@2.5.0`
- `fdeflate@0.3.7`
- `find-msvc-tools@0.1.16`
- `flate2@1.1.11`
- `fnv@1.0.7`
- `form_urlencoded@1.2.3`
- `getrandom@0.3.4`
- `getrandom@0.4.3`
- `glob@0.3.4`
- `hashbrown@0.12.3`
- `hashbrown@0.17.2`
- `heck@0.5.0`
- `html5ever@0.39.0`
- `http@1.5.0`
- `hybrid-array@0.4.15`
- `ident_case@1.0.1`
- `idna@1.1.0`
- `idna_adapter@1.2.2`
- `indexmap@1.9.3`
- `indexmap@2.14.2`
- `itoa@1.0.19`
- `json-patch@4.2.0`
- `jsonptr@0.7.1`
- `keyboard-types@0.8.3`
- `lazy_static@1.5.1`
- `libc@0.2.191`
- `lock_api@0.4.14`
- `log@0.4.35`
- `markup5ever@0.39.0`
- `mime@0.3.18`
- `miniz_oxide@0.8.9`
- `miniz_oxide@0.9.1`
- `muda@0.20.0`
- `num-conv@0.2.2`
- `once_cell@1.21.4`
- `parking_lot@0.12.5`
- `parking_lot_core@0.9.12`
- `percent-encoding@2.3.2`
- `pin-project-lite@0.2.18`
- `png@0.18.1`
- `powerfmt@0.2.1`
- `proc-macro2@1.0.107`
- `quote@1.0.47`
- `raw-window-handle@0.6.2`
- `regex@1.13.2`
- `regex-automata@0.4.19`
- `regex-syntax@0.8.12`
- `rustc_version@0.4.1`
- `rustc-hash@2.1.3`
- `scopeguard@1.2.0`
- `semver@1.0.28`
- `serde@1.0.229`
- `serde_core@1.0.229`
- `serde_derive@1.0.229`
- `serde_derive_internals@0.29.1`
- `serde_json@1.0.154`
- `serde_repr@0.1.21`
- `serde_spanned@1.1.3`
- `serde_with@3.25.0`
- `serde_with_macros@3.25.0`
- `serde-untagged@0.1.9`
- `serialize-to-javascript@0.1.2`
- `serialize-to-javascript-impl@0.1.2`
- `servo_arc@0.4.3`
- `sha2@0.11.2`
- `shlex@2.0.1`
- `siphasher@1.0.4`
- `smallvec@1.16.6`
- `softbuffer@0.4.8`
- `stable_deref_trait@1.2.1`
- `static_vcruntime@3.0.0`
- `string_cache@0.9.0`
- `string_cache_codegen@0.6.1`
- `symlink@0.1.0`
- `syn@2.0.119`
- `syn@3.0.10`
- `tao@0.37.3`
- `tauri@2.12.7`
- `tauri-build@2.7.6`
- `tauri-codegen@2.7.6`
- `tauri-macros@2.7.6`
- `tauri-runtime@2.12.4`
- `tauri-runtime-wry@2.12.5`
- `tauri-utils@2.10.4`
- `tendril@0.5.1`
- `thiserror@2.0.21`
- `thiserror-impl@2.0.21`
- `thread_local@1.1.10`
- `time@0.3.55`
- `time-core@0.1.9`
- `time-macros@0.2.32`
- `toml@1.2.3+spec-1.1.0`
- `toml_datetime@1.1.3+spec-1.1.0`
- `toml_parser@1.1.7+spec-1.1.0`
- `toml_writer@1.2.0+spec-1.1.0`
- `typeid@1.0.3`
- `typenum@1.20.1`
- `unicode-ident@1.0.27`
- `unicode-segmentation@1.13.3`
- `url@2.5.9`
- `utf_types@1.0.0`
- `utf8_iter@1.1.0`
- `uuid@1.29.3`
- `version_check@0.9.5`
- `web_atoms@0.2.6`
- `web-time@1.1.0`
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
- `windows-version@0.1.7`
- `window-vibrancy@0.8.1`
- `wry@0.57.4`

### BSD-3-Clause (4)

- `alloc-no-stdlib@3.0.0`
- `alloc-stdlib@0.3.0`
- `brotli@9.1.3`
- `brotli-decompressor@6.4.0`

### CC0-1.0 (1)

- `dunce@1.0.5`

### MIT (208)

- `adler2@2.0.1`
- `aho-corasick@1.1.5`
- `anstyle@1.0.14`
- `anyhow@1.0.105`
- `autocfg@1.5.1`
- `base64@0.23.1`
- `bitflags@2.13.2`
- `bit-set@0.8.0`
- `bit-vec@0.8.0`
- `block-buffer@0.12.1`
- `brotli@9.1.3`
- `brotli-decompressor@6.4.0`
- `byteorder@1.5.0`
- `bytes@1.12.2`
- `camino@1.2.6`
- `cargo_metadata@0.19.2`
- `cargo_toml@1.0.1`
- `cargo-platform@0.1.9`
- `cc@1.12.0`
- `cfb@0.14.0`
- `cfg-if@1.0.5`
- `clap@4.6.10`
- `clap_builder@4.6.10`
- `clap_derive@4.6.9`
- `clap_lex@1.1.2`
- `const-oid@0.10.3`
- `cookie@0.18.3`
- `cpufeatures@0.3.1`
- `crc32fast@1.5.6`
- `crossbeam-channel@0.5.17`
- `crossbeam-utils@0.8.23`
- `crypto-common@0.2.2`
- `ctor@1.0.14`
- `darling@0.24.1`
- `darling_core@0.24.1`
- `darling_macro@0.24.1`
- `deranged@0.5.9`
- `derive_more@2.1.1`
- `derive_more-impl@2.1.1`
- `digest@0.11.3`
- `dirs@7.0.0`
- `dirs-sys@0.5.0`
- `displaydoc@0.2.7`
- `dom_query@0.28.0`
- `dpi@0.1.2`
- `dtoa@1.0.11`
- `dyn-clone@1.0.20`
- `embed-resource@3.0.12`
- `equivalent@1.0.3`
- `erased-serde@0.4.11`
- `fastrand@2.5.0`
- `fdeflate@0.3.7`
- `find-msvc-tools@0.1.16`
- `flate2@1.1.11`
- `fnv@1.0.7`
- `form_urlencoded@1.2.3`
- `getrandom@0.3.4`
- `getrandom@0.4.3`
- `glob@0.3.4`
- `hashbrown@0.12.3`
- `hashbrown@0.17.2`
- `heck@0.5.0`
- `html5ever@0.39.0`
- `http@1.5.0`
- `hybrid-array@0.4.15`
- `ico@0.6.0`
- `ident_case@1.0.1`
- `idna@1.1.0`
- `idna_adapter@1.2.2`
- `indexmap@1.9.3`
- `indexmap@2.14.2`
- `infer@0.22.0`
- `itoa@1.0.19`
- `json-patch@4.2.0`
- `jsonptr@0.7.1`
- `keyboard-types@0.8.3`
- `lazy_static@1.5.1`
- `libc@0.2.191`
- `lock_api@0.4.14`
- `log@0.4.35`
- `markup5ever@0.39.0`
- `matchers@0.2.0`
- `memchr@2.8.3`
- `mime@0.3.18`
- `miniz_oxide@0.8.9`
- `miniz_oxide@0.9.1`
- `muda@0.20.0`
- `new_debug_unreachable@1.0.6`
- `nu-ansi-term@0.50.3`
- `num-conv@0.2.2`
- `once_cell@1.21.4`
- `parking_lot@0.12.5`
- `parking_lot_core@0.9.12`
- `percent-encoding@2.3.2`
- `phf@0.13.1`
- `phf_codegen@0.13.1`
- `phf_generator@0.13.1`
- `phf_macros@0.13.1`
- `phf_shared@0.13.1`
- `pin-project-lite@0.2.18`
- `plist@1.10.5`
- `png@0.18.1`
- `powerfmt@0.2.1`
- `precomputed-hash@0.1.1`
- `proc-macro2@1.0.107`
- `quick-xml@0.42.0`
- `quote@1.0.47`
- `raw-window-handle@0.6.2`
- `regex@1.13.2`
- `regex-automata@0.4.19`
- `regex-syntax@0.8.12`
- `rustc_version@0.4.1`
- `rustc-hash@2.1.3`
- `same-file@1.0.6`
- `schemars@0.8.22`
- `schemars_derive@0.8.22`
- `scopeguard@1.2.0`
- `semver@1.0.28`
- `serde@1.0.229`
- `serde_core@1.0.229`
- `serde_derive@1.0.229`
- `serde_derive_internals@0.29.1`
- `serde_json@1.0.154`
- `serde_repr@0.1.21`
- `serde_spanned@1.1.3`
- `serde_with@3.25.0`
- `serde_with_macros@3.25.0`
- `serde-untagged@0.1.9`
- `serialize-to-javascript@0.1.2`
- `serialize-to-javascript-impl@0.1.2`
- `servo_arc@0.4.3`
- `sha2@0.11.2`
- `sharded-slab@0.1.7`
- `shlex@2.0.1`
- `simd-adler32@0.3.10`
- `siphasher@1.0.4`
- `smallvec@1.16.6`
- `softbuffer@0.4.8`
- `stable_deref_trait@1.2.1`
- `static_vcruntime@3.0.0`
- `string_cache@0.9.0`
- `string_cache_codegen@0.6.1`
- `strsim@0.11.1`
- `symlink@0.1.0`
- `syn@2.0.119`
- `syn@3.0.10`
- `synstructure@0.14.0`
- `tauri@2.12.7`
- `tauri-build@2.7.6`
- `tauri-codegen@2.7.6`
- `tauri-macros@2.7.6`
- `tauri-runtime@2.12.4`
- `tauri-runtime-wry@2.12.5`
- `tauri-utils@2.10.4`
- `tauri-winres@0.3.7`
- `tendril@0.5.1`
- `thiserror@2.0.21`
- `thiserror-impl@2.0.21`
- `thread_local@1.1.10`
- `time@0.3.55`
- `time-core@0.1.9`
- `time-macros@0.2.32`
- `tokio@1.54.1`
- `toml@1.2.3+spec-1.1.0`
- `toml_datetime@1.1.3+spec-1.1.0`
- `toml_parser@1.1.7+spec-1.1.0`
- `toml_writer@1.2.0+spec-1.1.0`
- `tracing@0.1.44`
- `tracing-appender@0.2.5`
- `tracing-core@0.1.36`
- `tracing-subscriber@0.3.23`
- `typeid@1.0.3`
- `typenum@1.20.1`
- `unicode-ident@1.0.27`
- `unicode-segmentation@1.13.3`
- `url@2.5.9`
- `urlpattern@0.6.0`
- `utf_types@1.0.0`
- `utf8_iter@1.1.0`
- `uuid@1.29.3`
- `version_check@0.9.5`
- `vswhom@0.1.0`
- `vswhom-sys@0.1.3`
- `walkdir@2.5.1`
- `web_atoms@0.2.6`
- `web-time@1.1.0`
- `webview2-com@0.39.1`
- `webview2-com-macros@0.8.1`
- `webview2-com-sys@0.39.1`
- `winapi-util@0.1.11`
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
- `windows-version@0.1.7`
- `window-vibrancy@0.8.1`
- `winnow@1.0.5`
- `winreg@0.56.0`
- `wry@0.57.4`
- `zmij@1.0.23`

### MIT-0 (1)

- `dunce@1.0.5`

### MPL-2.0 (5)

- `cssparser@0.37.0`
- `cssparser-macros@0.7.1`
- `dtoa-short@0.3.5`
- `option-ext@0.2.0`
- `selectors@0.38.0`

### Unicode-3.0 (19)

- `icu_collections@2.4.0`
- `icu_locale_core@2.4.0`
- `icu_normalizer@2.4.0`
- `icu_normalizer_data@2.4.0`
- `icu_properties@2.4.0`
- `icu_properties_data@2.4.0`
- `icu_provider@2.4.0`
- `litemap@0.8.3`
- `potential_utf@0.1.6`
- `tinystr@0.8.5`
- `unicode-ident@1.0.27`
- `writeable@0.6.4`
- `yoke@0.8.4`
- `yoke-derive@0.8.4`
- `zerofrom@0.1.8`
- `zerofrom-derive@0.1.8`
- `zerotrie@0.2.5`
- `zerovec@0.11.9`
- `zerovec-derive@0.11.7`

### Unlicense (6)

- `aho-corasick@1.1.5`
- `byteorder@1.5.0`
- `memchr@2.8.3`
- `same-file@1.0.6`
- `walkdir@2.5.1`
- `winapi-util@0.1.11`

### Zlib (5)

- `foldhash@0.2.0`
- `miniz_oxide@0.8.9`
- `miniz_oxide@0.9.1`
- `raw-window-handle@0.6.2`
- `static_vcruntime@3.0.0`

### Unlicensed (5)

- `dd-core@0.1.0`
- `dd-diagnostics@0.1.0`
- `dd-ipc@0.1.0`
- `discodesk-engine@0.1.0`
- `discodesk-studio@0.1.0`

## Workspace crates

`dd-core`, `dd-ipc`, `dd-diagnostics`, `discodesk-engine` and `discodesk-studio` are
first-party and are reported as unlicensed, which is accurate: the project licence
is **not yet decided**. See `LICENSE` and `docs/GATE0_PLAN.md` D1. They are excluded
from the policy check through `[licenses.private] ignore = true` because they are
`publish = false` local crates, not third-party dependencies.

## npm packages (Studio frontend)

The Studio frontend is built with npm; `apps/discodesk-studio/package-lock.json` is
the authoritative resolved set. Every direct dependency and devDependency is
MIT or Apache-2.0 OR MIT:

| Package | Licence |
|---|---|
| `@tauri-apps/api` | Apache-2.0 OR MIT |
| `react` | MIT |
| `react-dom` | MIT |
| `vite` | MIT |
| `@vitejs/plugin-react` | MIT |
| `typescript` | Apache-2.0 |
| `vitest` | MIT |
| `jsdom` | MIT |
| `@testing-library/react` | MIT |
| `@testing-library/jest-dom` | MIT |
| `@types/react` | MIT |
| `@types/react-dom` | MIT |
| `@types/node` | MIT |
| `@tauri-apps/cli` | Apache-2.0 OR MIT |

Bun is deliberately **not** used: it statically links JavaScriptCore/WebKit under
LGPL-2. See `docs/ADR/0008`.

## Runtime components that are NOT open source

These are not Rust crates or npm packages, but they ship with (or are required by)
the product and are disclosed separately:

| Component | Terms | Status |
|---|---|---|
| Microsoft Edge WebView2 Runtime | Proprietary Microsoft EULA | Required at runtime by the Studio |
| MSVC C/C++ runtime | Microsoft Visual C++ redistributable terms | Statically linked by MSVC builds (`static_vcruntime`) |
| `WebView2Loader.dll` | Microsoft redistributable | Ships with a Tauri Windows build |
| DiscoDesk Studio app icon | First-party, provisional | Generated for this repository; replace before release |

No statement anywhere may claim the shipped application is entirely open source.
