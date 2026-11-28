# Licensing decision — options and implications

**Status:** DECISION REQUIRED. No licence has been chosen, and this document does **not**
choose one. It exists so the owner can decide with the facts in front of them.
**Prepared:** 2026-11-28.
**Gate item:** Gate 0 / B3.

An undecided licence does **not** block private engineering, testing or Gate 1
implementation. It does block shipping, and it must not be described as cleared for
distribution.

---

## 1. Current legal state

- `LICENSE` contains no grant. Under default copyright that means **all rights reserved**:
  nobody may copy, modify or redistribute the source. This repository is currently
  "source-available", not open source.
- Every workspace crate declares `license-file = "LICENSE"` rather than an SPDX `license`
  identifier, so no licence is implied by the manifests.
- The dependency audit passes because the workspace crates are `publish = false` and are
  excluded from the policy check via `[licenses.private] ignore = true` in `deny.toml`.
  That is a build-configuration fact, not a licence grant.
- `THIRD_PARTY_NOTICES.md` is generated from `cargo deny list` and is accurate today.

---

## 2. Third-party obligations already present (verified 2026-11-28)

From `cargo deny list` on the shipped target (`x86_64-pc-windows-msvc`):

| Licence | Crates | Obligation if you distribute a binary |
|---|---|---|
| MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2/3-Clause, ISC, 0BSD, Zlib | the large majority | Retain copyright and licence notices. No source disclosure. |
| Unicode-3.0, Unicode-DFS-2016 | 19 ICU / unicode crates | Notice only. |
| **MPL-2.0** | `cssparser`, `cssparser-macros`, `selectors`, `dtoa-short`, `option-ext` | **File-level** copyleft: disclose that these are MPL-2.0, and publish modifications *to those files* if you make any. Unmodified, so publishing the upstream crates satisfies it. See ADR-0012. |
| Unlicense (dual with MIT in practice) | `aho-corasick`, `byteorder`, `memchr`, `same-file`, `walkdir`, `winapi-util` | None. |
| CC0-1.0 / MIT-0 (dual) | `dunce` | None. |

Microsoft components, disclosed separately in `THIRD_PARTY_NOTICES.md`:

| Component | Terms | Consequence |
|---|---|---|
| Microsoft Edge WebView2 Runtime | Proprietary Microsoft EULA | A runtime prerequisite on the user's machine, or ship the Fixed Version (~250 MB) under Microsoft's terms. |
| MSVC C/C++ runtime | Microsoft redistributable terms | Statically linked by `static_vcruntime`. |
| `WebView2Loader.dll` | Microsoft redistributable | Ships with a Tauri Windows build. |

**Net effect:** nothing in the current dependency graph forces DiscoDesk's *own* source to
be disclosed. The project may be distributed under any permissive licence, or as
closed-source proprietary software, provided the notices are retained and the MPL files
stay unmodified or their changes are published.

---

## 3. Options and their implications

### Option A — MIT

| | |
|---|---|
| Modification / redistribution | Permitted, including commercially and closed-source. |
| Source disclosure | None. |
| Patent | No express patent grant. |
| Contribution model | Anyone may contribute; contributions arrive under MIT implicitly. |
| Fit for this product | Simple, universally understood, minimal compliance. |

### Option B — Apache-2.0

| | |
|---|---|
| Modification / redistribution | Permitted, including commercially and closed-source. |
| Source disclosure | None. |
| Patent | **Express patent grant** plus patent-termination if you sue over patents. |
| Extras | Requires a `NOTICE` file; includes a trademark clause (no implied brand rights). |
| Fit for this product | Stronger legal footing for a commercial desktop app; slightly more paperwork. |

### Option C — MIT OR Apache-2.0 (dual)

| | |
|---|---|
| Effect | Downstream picks either. This is the Rust ecosystem convention and matches most of the dependency graph. |
| Patent | Available via the Apache branch for those who want it. |
| Fit for this product | Best compatibility if any crate or component may later be reused; the usual choice for Rust projects. |

### Option D — Proprietary, all rights reserved + EULA

| | |
|---|---|
| Modification / redistribution | Not permitted except as the EULA allows. |
| Source disclosure | None; source stays private. |
| Fit for this product | Viable for a paid closed product. Costs community contribution, most package-manager distribution channels, and any "open source" positioning. |

### Option E — Copyleft (GPL-3.0 / AGPL-3.0)

| | |
|---|---|
| Effect | Distributing a binary obliges you to offer the corresponding source under the same licence. AGPL extends this to network use. |
| Conflict risk | Statically linking the Steamworks SDK and Steam's DRM wrapper is legally contested territory; store policies and the Workshop agreement assume you can license your own binary. |
| Fit for this product | Not recommended for a commercial desktop product that intends Steam distribution. Listed for completeness, not as a suggestion. |

**No option above conflicts with the current dependencies**, with one caveat: choosing a
copyleft licence would make the MPL-2.0 crates *compatible* but would also pull the whole
application into copyleft obligations, which is a much larger commitment than the MPL files
themselves impose.

---

## 4. Copyright ownership

A licence grants rights; it does not establish who owns the copyright. Before publishing,
the owner should confirm:

- **Who holds copyright** in the work — an individual, or a company (in which case an
  employment/IP-assignment agreement is what actually matters).
- **Contribution policy**, if outside contributions will be accepted:
  - *DCO* (`Signed-off-by`): simple, keeps ownership with contributors, licence stays the
    project licence.
  - *CLA*: assigns or licenses contributions to the owner; needed if a future relicensing
    (for example to a proprietary licence) must remain possible.
  - Without either, contributions are implicitly licensed under whatever the project
    licence is at the time, and relicensing later becomes impractical.
- **Assets**: the Studio application icon in `apps/discodesk-studio/src-tauri/icons/` is
  provisional and was generated inside this repository. It should be replaced with owned
  artwork, or explicitly declared as owned, before release.

---

## 5. Forward-looking items (not yet in the build)

These are recorded so the decision is not made blind to Gate 5.

| Item | Status of the licence fact |
|---|---|
| **ONNX Runtime** (planned depth inference runtime) | **UNVERIFIED in this pass.** The upstream repository could not be reached from this environment (HTTP 404 on every path, including README), so no claim is made here. Commonly reported as MIT; **must be verified from the primary source before bundling**, and its native binary redistribution terms reviewed separately from the Rust wrapper crate's licence. |
| Depth Anything **V2 Small** weights | Recorded in `RESEARCH.md` as Apache-2.0 (Base/Large/Giant are CC BY-NC 4.0 and must not be used in a paid product). |
| Depth Anything **V3 Small** weights | Recorded in `RESEARCH.md` as Apache-2.0; the converter's code licence is separate from the weights. |
| Model weights generally | Licences are separate from source code. Gate 5 must record the exact checkpoint hash and licence per model, and `THIRD_PARTY_NOTICES.md` must list them. |
| Steamworks SDK | Requires accepting Valve's SDK licence; only the redistributable binaries may be shipped, and Workshop publishing requires the Workshop legal agreement. The exact current terms were **not re-read in this pass** and must be verified before Gate 6. |

---

## 6. What the owner must decide

Pick one and record it as an ADR; nothing else should change before that.

- [ ] **MIT** — simplest permissive.
- [ ] **Apache-2.0** — permissive with an explicit patent grant and a `NOTICE` obligation.
- [ ] **MIT OR Apache-2.0** — the Rust convention; recommended if adoption matters.
- [ ] **Proprietary / all rights reserved + EULA** — closed source.
- [ ] Something else (record it).

Then, in one change:

1. Add `docs/ADR/0013-project-licence.md` recording the decision, rationale and date.
2. Replace `license-file` in `[workspace.package]` with the chosen SPDX `license`
   (keep `LICENSE` as the human-readable text, and add `NOTICE` if Apache-2.0).
3. Leave `[licenses.private] ignore = true` only if the workspace crates remain
   unpublished; otherwise they become licence-checked like dependencies.
4. Update `README.md` and `THIRD_PARTY_NOTICES.md` with the project licence and a
   statement that the Microsoft runtime components are separately licensed.
5. Re-run `cargo deny check all`.

Until then, the honest statement is: **the project licence is undecided; the dependency
graph is compatible with a permissive or closed-source distribution; the product is not
cleared for distribution.**
