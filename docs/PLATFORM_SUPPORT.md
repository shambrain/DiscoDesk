# DiscoDesk platform support matrix

**Last updated:** 2026-11-27
**Scope:** Verified development environment and platform capability baseline.
**Status:** Development-host baseline only. No shipping platform is claimed as supported until
the corresponding gate acceptance evidence exists (see `docs/IMPLEMENTATION_STATE.md`).

Rules for this file:

1. A row is only marked *verified* when it was observed on the machine described in
   [§1](#1-verified-development-host) or on a named test machine, with the exact command
   and date recorded.
2. A row marked *unverified* means **not tested**. It is not a negative claim; it means no
   evidence exists yet.
3. Product capability rows (audio capture mode, wallpaper hosting) stay *not implemented*
   until the implementing gate passes. See the *Status key*.

---

## 1. Verified development host

Captured 2026-11-27 on the primary development machine.

| Property | Value | How verified |
|---|---|---|
| OS edition | Windows 10 Enterprise (multi-edition) | `Get-CimInstance Win32_OperatingSystem` → `Caption` |
| OS version / build | 10.0.19045 | `Get-CimInstance Win32_OperatingSystem` → `Version` / `BuildNumber` |
| Architecture | x64 | `OSArchitecture` = `64-bit` |
| PowerShell | Windows PowerShell 5.1 (desktop) | `$PSVersionTable.PSVersion` |
| Rust (stable, pinned) | `rustc 1.100.0 (0ef1bc386 2026-11-09)` | `rustup run 1.100.0 rustc --version --verbose` |
| Rust (host default) | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` | `rustc --version --verbose` |
| Rust host triple | `x86_64-pc-windows-msvc` | `rustup show active-toolchain` |
| Rust components | `cargo`, `rustc`, `rust-std`, `rustfmt`, `clippy`, `rust-docs` | `rustup component list --installed` |
| Extra toolchain | `nightly-x86_64-pc-windows-msvc` (installed, not used by the project pin) | `rustup show` |
| Cargo (pinned) | `cargo 1.100.0 (3acc79c0c 2026-11-05)` | `rustup run 1.100.0 cargo --version` |
| LLVM in rustc | 23.1.1 (1.99.1 host); see `rustc --version --verbose` for 1.100.0 | `rustc --version --verbose` |
| MSVC toolset A | Visual Studio Build Tools 2019, 16.11.37507.1, MSVC 14.29.30133 | `vswhere -products * -format json` |
| MSVC toolset B | Visual Studio Build Tools 2026, 18.11.12315.225, MSVC 14.51.36231 | `vswhere -products * -format json` |
| Windows SDK | 10.0.19041.0 and 10.0.26100.0 | `C:\Program Files (x86)\Windows Kits\10\Lib` |
| `link.exe` | Present in the 2026 Build Tools, Hostx64\x64 | filesystem check |
| MSBuild | Present in VS Build Tools 2026 | filesystem check |
| Node.js | `v26.12.1` | `node --version` |
| npm | `11.20.0` | `npm --version` |
| Bun | `1.4.4` | `bun --version` |
| pnpm | `12.16.1` | `pnpm --version` |
| Git | `2.55.0.windows.5` | `git --version` |
| WebView2 Runtime | `158.0.4370.33` (Evergreen, machine-wide) | `HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-…}` → `pv` |
| Microsoft Edge | Installed at `C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe` | `App Paths\msedge.exe` |
| GPU 1 | AMD Radeon RX 6700 XT, driver `32.0.21045.29003` | `Get-CimInstance Win32_VideoController` |
| GPU 2 | AMD Radeon(TM) Graphics (integrated), driver `32.0.21045.29003` | `Get-CimInstance Win32_VideoController` |
| GPU 3 | Virtual Display Driver `11.30.4.434` (software/virtual adapter) | `Get-CimInstance Win32_VideoController` |
| Network egress | crates.io, npm registry and github.com all reachable (HTTP 200) | `Invoke-WebRequest -Method Head` |

### Environment caveats that affect the build

- **Rust is not on the machine's default `PATH`.** It is installed at
  `C:\Users\Administrator\.cargo\bin`. Every command must be run with
  `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"` first. A fresh shell, a CI
  runner and an IDE task all need the same treatment. This is a real developer-experience
  hazard and is documented in the project README and CI workflow.
- **Two MSVC toolsets are installed.** The 2019 toolset (14.29) is old enough that
  some current Rust crates require a newer toolset. Prefer the Build Tools 2026
  toolset (14.51) and Windows SDK 10.0.26100.0 when a crate reports a missing-symbol or
  SDK-version error. `cargo` selects the newest installed toolset automatically.
- **The OS build is 19045.** This is below the documented Windows 10 build 20348
  threshold for `AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK`. See
  [§3](#3-audio-capture-capability-baseline) and `RESEARCH.md` §4.
- A **Virtual Display Driver** adapter is present. It must not be mistaken for a real
  GPU when reporting renderer backend/performance evidence.
- `opencode` is not on the default `PATH` in the probe shell. OpenCode host version was
  therefore **not** verified here; treat it as unverified rather than guessing.

---

## 2. Status key

| Status | Meaning |
|---|---|
| `verified` | Observed on the named machine with the recorded command on the recorded date. |
| `not implemented` | No code exists yet. The gate that will implement it is named. |
| `unverified` | No test has been performed on any machine. No claim either way. |
| `not planned` | Out of scope for the current roadmap. |

---

## 3. Audio capture capability baseline

No capture code exists yet. The rows below record the *documented OS precondition* and
the *verified host build*, so the capability probe has a known starting point. They are
not capability claims about DiscoDesk.

| Capability | Documented OS precondition | This host | Status |
|---|---|---|---|
| WASAPI output/system loopback | Windows 7+ | Build 19045 | `not implemented` (Gate 1) |
| `AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK` | Windows 10 build **20348** or later | Build **19045** — below threshold | `not implemented` (Gate 6); must be reported **unavailable** by the runtime probe on this build |
| Core Audio session enumeration (app names/PIDs) | Windows 7+ | Build 19045 | `not implemented` (Gate 1) |

**Rule:** on this development host, per-application capture must report
`CaptureCapability::SystemMixOnly` or `Unavailable`. Labelling the system mix as
per-app capture is forbidden by `AGENTS.md`.

---

## 4. Desktop wallpaper hosting

| Platform | Mechanism | Status |
|---|---|---|
| Windows | WorkerW / Progman shell integration | `not implemented` (Gate 3) |
| macOS | AppKit window at `desktop` level | `not implemented` (Gate 7) |
| Linux / KDE Plasma 6 | Native wallpaper plugin | `not implemented` (Gate 8) |
| Linux / X11 | Dedicated desktop-window backend | `not implemented` (Gate 8) |
| Linux / GNOME, Wayland | — | `not planned` until a native integration exists |

---

## 5. Renderer backends

`wgpu` backend availability is a runtime adapter enumeration result, not a static claim.
No renderer code exists at Gate 0, so no backend has been observed.

| Backend | Status |
|---|---|
| D3D12 | `unverified` — to be enumerated at Gate 2 |
| Vulkan | `unverified` — to be enumerated at Gate 2 |
| Metal | `unverified` — no macOS host available |

The host's AMD RX 6700 XT is expected to expose D3D12 and Vulkan, but "expected" is not
evidence. Gate 2 must record the enumerated adapter list.

---

## 6. Studio prerequisites

| Requirement | This host | Status |
|---|---|---|
| WebView2 Runtime | 158.0.4370.33 installed | `verified` |
| Microsoft Edge (WebView2 dev fallback) | Installed | `verified` |
| Node.js | v26.12.1 | `verified` |
| MSVC build tools + Windows SDK | Build Tools 2019 + 2026, SDK 19041 + 26100 | `verified` |

Tauri v2's exact MSRV and Build Tools requirement are tracked in `RESEARCH.md` and must
be re-verified before the Studio crate is added.

---

## 7. Test machines still required

The following are **required before the corresponding acceptance gate can pass** and are
currently `unverified`:

| Target | Needed for | Status |
|---|---|---|
| Windows 11 (current supported build) | Process loopback, multi-display, audio session churn | `unverified` — no Windows 11 host available |
| Windows 10 build 19042 | Loopback + fallback behaviour on the oldest supported build | `unverified` — this host is 19045 |
| Second GPU vendor (NVIDIA / Intel iGPU path) | Renderer coverage across GPU tiers | `unverified` |
| macOS 14.2+ | Core Audio taps, Spaces, sleep/wake | `unverified` — no macOS host available |
| Linux KDE Plasma 6 Wayland | Native wallpaper plugin + PipeWire | `unverified` — no Linux host available |
| Linux X11 | Desktop host + PulseAudio/PipeWire | `unverified` — no Linux host available |

**No cross-platform or multi-vendor claim may be published from the single-host data in
this file.**


## 8. Visual Studio development environment and integration

### 8.1 Installed product inventory from screenshot

The Visual Studio Installer screenshot supplied on 2026-11-27 shows:

| Product | Screenshot state | Interpretation |
|---|---|---|
| Visual Studio Community 2026 Insiders | Installed listing; product update waiting to download at 0% | IDE product is registered; the displayed pending update is not installed evidence |
| Visual Studio Build Tools 2026 | October 2026 Feature Update 18.11.1; update available for November 2026 Feature Update 18.12.1 | Installed stable 18.x native toolchain; keep 18.11.1 until an update is deliberately applied |
| Visual Studio Build Tools 2019 | 16.11.58; update available for 16.11.63 | Legacy toolchain remains installed; it is not the preferred DiscoDesk compiler |

The developer's PowerShell probe separately reported MSVC 14.51.36231 and Windows SDK versions 10.0.19041.0 and 10.0.26100.0. The previous probe did not establish the path to devenv.exe or verify the Community IDE's Open Folder/debugger launch; run the repository Doctor for that.

### 8.2 Project integration

| Capability | Repository integration | Status |
|---|---|---|
| Community workload configuration | Root .vsconfig | Added; component import not yet run |
| Build Tools workload configuration | vsconfig/BuildTools-2026.vsconfig | Added; component import not yet run |
| Open Folder build tasks | .vs/tasks.vs.json | Added; not yet run in Visual Studio |
| Native debug launch profiles | .vs/launch.vs.json | Added; binary/debugger launch not yet verified |
| Environment inspection | scripts/vs/Invoke-DiscoDeskVS.ps1 -Action Doctor | Added; local Windows execution pending |
| Optimized build with symbols | Cargo profile release-with-symbols | Added; not built yet |
| Explicit-path local commit helper | scripts/git/Commit-DiscoDesk.ps1 | Added; Windows execution pending |

### 8.3 Toolchain selection

The helper prefers a stable Visual Studio 18.x installation with the MSVC x64/x86 component and selects Build Tools when it is available. It imports the x64 developer environment from VsDevCmd.bat and adds the existing rustup Cargo directory when present. It does not install Rust, upgrade Visual Studio or alter the pinned rust-toolchain.toml.

Community 2026 Insiders is treated as an IDE candidate, not the release compiler baseline. Updates shown as pending in the screenshot are not claimed as installed.

### 8.4 Known public-main build prerequisite

At public main commit 2912a7e5b356301121c2956b301fc56dd364b23d, apps/discodesk-engine/Cargo.toml declares src/main.rs as the binary path, but apps/discodesk-engine/src/main.rs is absent from the public tree. The VS build helper checks for the real source and stops with an explicit message rather than fabricating a program. The F5 launch profiles are prepared for the eventual executable, but engine build/debug acceptance remains pending until the real entry point is committed and builds successfully.

The user may have uncommitted local engine files not present in public main; check the local checkout before drawing conclusions about local work.


### 8.4 GitHub Actions runner baseline

The CI workflow now uses the explicit `windows-2025-vs2026` runner label. GitHub's runner-images announcement states the `windows-2025` and `windows-latest` labels migrated to Visual Studio 2026 in June 2026; the explicit label pins the intended Visual Studio 2026 image. The lint job runs the same repository Visual Studio Doctor to require the MSVC compiler, linker, MSBuild, Windows SDK 10.0.26100.0, and Rust MSVC host before Cargo linting.

Source: https://github.com/actions/runner-images/issues/14017
