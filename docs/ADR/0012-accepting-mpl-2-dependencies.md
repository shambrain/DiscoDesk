# ADR-0012: Accepting MPL-2.0 dependencies from the Tauri toolchain

- **Status:** Accepted
- **Date:** 2026-11-27
- **Deciders:** discodesk-lead, research-licensing-auditor
- **Gate:** 0 (Studio scaffold) — binding on all later distribution work

## Context

The Gate 0 dependency audit previously recorded "no MPL crate is in the graph".
Adding the Tauri v2 Studio invalidated that. The resolved graph now contains five
**MPL-2.0-only** crates (not dual-licensed):

| Crate | Version | Role |
|---|---|---|
| `cssparser` | 0.37.0 | CSS tokenizer/parser |
| `cssparser-macros` | 0.7.1 | its derive macros |
| `selectors` | 0.38.0 | CSS selector matching |
| `dtoa-short` | 0.3.5 | float formatting |
| `option-ext` | 0.2.0 | Option helpers |

They arrive through Tauri's HTML/CSS processing (`tauri-utils` / `tauri-codegen`
use them while generating the embedded asset manifest). They are transitive
build/runtime dependencies of the Studio, not of the engine — `dd-ipc`,
`dd-core` and `dd-diagnostics` remain free of MPL.

MPL-2.0 is **weak, file-level copyleft**: it does not reach our source files and
does not require our code to be MPL. It does impose obligations on the MPL files
themselves.

`AGENTS.md` requires that dependency licences be reviewed before bundling and
that commercial redistribution be verified. This ADR records the decision and
its obligations rather than leaving an allow-list entry unexplained.

## Options considered

1. **Accept MPL-2.0 with the obligations satisfied and disclosed (chosen).**
2. **Refuse MPL-2.0 and drop Tauri.** Rejected: MPL-2.0 is OSI-approved,
   commercially usable, and compatible with a closed-source application. Dropping
   the Studio framework over file-level copyleft in the asset pipeline would be a
   disproportionate response.
3. **Patch or vendor the MPL crates to remove the dependency.** Rejected: it
   would add maintenance burden and, if modified, *increase* the obligations.
4. **Allow MPL silently by adding it to the allow list without disclosure.**
   Rejected: that is exactly the kind of unexplained waiver this repository
   forbids.

## Decision

1. **`MPL-2.0` stays in `deny.toml`'s allow list**, with the crates named in the
   comment so the entry is never speculative.
2. **The obligations are met as follows:**
   - *Source availability*: the MPL-covered files are unmodified upstream crates
     published on crates.io at the exact versions pinned in `Cargo.lock`.
   - *Disclosure*: `THIRD_PARTY_NOTICES.md` lists them under their licence.
   - *No additional restrictions*: the project places no extra terms on those
     files.
   - *Modifications*: there are none. **If an MPL-covered file is ever patched in
     this repository, the patch must be published and recorded in this ADR before
     the change ships.**
3. **MPL must not spread.** `dd-core`, `dd-ipc` and `dd-diagnostics` must not
   acquire an MPL dependency. The audit job verifies the graph; a reviewer must
   notice if MPL appears outside the Tauri chain.
4. **`cargo deny check all` remains the enforcement point**, so this stays true
   on every dependency update rather than only at the time of this decision.
5. **Re-review before any public release or store submission**, including the
   WebView2, MSVC runtime and `WebView2Loader.dll` terms recorded in
   `THIRD_PARTY_NOTICES.md`.

## Consequences

- Easier: the Studio can use the maintained Tauri stack without a licence
  exception hidden in an allow list.
- Easier: the obligation is documented, so a future release checklist can verify
  it instead of rediscovering it.
- Harder: the project now carries a weak-copyleft disclosure obligation that a
  purely permissive dependency set would not. It is bounded to five unmodified
  files-sets in the asset pipeline.
- Harder: patching an MPL crate becomes a published Modification, which is a real
  cost and should be avoided.
- Now forbidden: adding MPL to the engine crates; modifying an MPL crate without
  publishing the change; describing the product as wholly permissive-licensed.

## Verification

- `cargo deny check all` passes with MPL-2.0 allowed (CI `audit` job).
- `cargo deny list` shows the five MPL crates, and they are disclosed in
  `THIRD_PARTY_NOTICES.md`.
- A review check: `cargo tree -p dd-ipc` and `cargo tree -p dd-core` contain no
  MPL crate.

## References

- Mozilla Public License 2.0: <https://www.mozilla.org/en-US/MPL/2.0/>
- MPL 2.0 FAQ, "Am I required to make my proprietary software available?":
  <https://www.mozilla.org/en-US/MPL/2.0/FAQ/>
- `deny.toml`, `THIRD_PARTY_NOTICES.md`, `docs/ADR/0008`, `docs/ADR/0004`.
- `AGENTS.md` — licence review before bundling.
