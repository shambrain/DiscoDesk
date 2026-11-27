# ADR-0008: Studio frontend toolchain, and the runtime-component licence boundary

- **Status:** Accepted
- **Date:** 2026-11-27
- **Deciders:** discodesk-lead, research-licensing-auditor
- **Gate:** 0 (repository scaffold) — binding on Gate 4

## Context

The Studio is a Tauri v2 + React + TypeScript application (ADR-0001). Three findings from
the Gate 0 licence audit change the obvious defaults:

1. **Bun statically links JavaScriptCore/WebKit, which is LGPL-2.** Bun itself is MIT,
   but the runtime bundles a copyleft component. Redistributing Bun would carry LGPL-2
   obligations. Plan B was "use Bun because it is fast" — that plan is unsafe for a
   commercially distributed desktop product.
2. **The Studio binary is not MIT/Apache end to end.** It depends on the Microsoft Edge
   **WebView2 Runtime** (proprietary Microsoft EULA, present at runtime on the user's
   machine or shipped as a ~250 MB Fixed Version), the **MSVC C/C++ runtime** (statically
   linked by `static_vcruntime`, redistributable under Microsoft terms), and
   **`WebView2Loader.dll`** (an architecture-specific native DLL). All three are outside
   the Rust/npm licence set.
3. **`typescript` on npm is now on the 7.x line** (7.0.2), which is the native/Go port
   line, not a 5.x patch bump. Adopting it is a real behavioural change that must be
   validated, not assumed.

`AGENTS.md` requires dependency pinning, licence review and that model weights and
runtime binaries be audited separately from source-code licences.

## Options considered

1. **npm as the package manager, Node 24 LTS, TypeScript validated before pinning, and an
   explicit written licence boundary for the shipped binary (chosen).**
2. **Bun as the package manager.** Rejected: LGPL-2 static linkage. Even though a build
   tool is not redistributed, adopting a copyleft-bearing toolchain as a project
   dependency creates a distribution trap for anyone who later bundles it, and it is
   avoidable at zero cost.
3. **Describe the Studio as "MIT/Apache licensed".** Rejected as inaccurate: the binary
   includes Microsoft components under Microsoft terms.
4. **Pin TypeScript 7 immediately because it is `latest`.** Rejected without validation:
   Vite 8's React plugin and the Tauri template must be shown to work with it first.

## Decision

1. **Package manager: npm** (pnpm is acceptable). **Bun is not a project dependency** and
   must not appear in `package.json`, CI or the build instructions. It may be used as a
   personal developer convenience, never as part of the product pipeline.
2. **Node: 24 LTS.** The effective floor is Node `^20.19.0 || >=22.12.0`, imposed by Vite
   8, not by Tauri (which names no minimum beyond "LTS").
3. **TypeScript is pinned only after a working build proves compatibility** with the
   chosen Vite/React-plugin/Tauri versions. Until then the Studio skeleton uses the
   version the Tauri template ships, and the pin is recorded in `RESEARCH.md` §16.3.
4. **The licence boundary is documented, not blurred.** `THIRD_PARTY_NOTICES.md` and any
   user-facing licence text must distinguish:
   - permissive source dependencies (MIT / Apache-2.0 / BSD / ISC / Unicode);
   - the **WebView2 Runtime** (Microsoft EULA, a runtime prerequisite);
   - the **MSVC runtime** (Microsoft redistributable terms);
   - **`WebView2Loader.dll`** (native redistributable).
   No statement may claim the shipped application is entirely open source.
5. **Studio MSRV is 1.92**, not the `tauri` crate's declared 1.90, because `wry` and the
   Tauri workspace declare 1.92. The workspace toolchain pin (1.100.0) satisfies it.
6. **The Studio remains a separate process** and is never the wallpaper renderer
   (ADR-0001). None of the above changes the runtime process model.

## Consequences

- Easier: the whole build pipeline is permissive-licensed and the one copyleft hazard is
  removed before it can propagate.
- Easier: the licence section of a future store listing has a factual basis.
- Harder: slightly slower installs than Bun. Accepted; build speed is not worth a
  copyleft trap in a shipped product.
- Harder: the Studio cannot be described with a single licence label. That is the honest
  position and must be maintained in the notices file.
- Now forbidden: adding Bun to the build; claiming the binary is wholly MIT/Apache;
  pinning TypeScript 7 without a passing build; shipping without `WebView2Loader.dll` or
  a documented WebView2 runtime requirement.

## Verification

- Gate 4: `package.json` contains no Bun dependency; the CI Studio build uses npm; the
  built application launches on a machine with the WebView2 Runtime present.
- Gate 4: `THIRD_PARTY_NOTICES.md` lists the Microsoft runtime components separately from
  the permissive source dependencies.
- Gate 0/4: a documented, pinned TypeScript version with a passing `tsc` and Vite build.

## References

- `docs/RESEARCH.md` §16.3 (verified Studio package versions), §16.4 (runtime components
  that are not open source).
- Bun licence and LGPL-2 static-linking note: <https://github.com/oven-sh/bun/blob/main/LICENSE.md>
- WebView2 distribution and runtime requirement:
  <https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution>
- Tauri v2 prerequisites: <https://v2.tauri.app/start/prerequisites/>
- `AGENTS.md` — dependency pinning, licence review, separate model/weights review.
