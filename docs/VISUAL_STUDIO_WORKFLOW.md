# Visual Studio 2026 workflow for DiscoDesk

**Last updated:** 2026-11-27  
**Scope:** Windows developer tooling, Open Folder integration, Cargo tasks and native debugging.  
**Product status:** Development integration only. It does not mean the engine or Studio is complete.

## 1. Actual development-host inventory

Verified on 2026-11-27 with `vswhere.exe -all -prerelease -products * -format json` and
by reading `VC\Tools\MSVC` on disk. These are measurements, not expectations.

| Installed product | Verified value | Use in DiscoDesk |
|---|---|---|
| Visual Studio Community 2026 (Insiders) | `18.13.0-insiders+12412.241`, prerelease, `C:\Program Files\Microsoft Visual Studio\18\Insiders` | IDE, Open Folder, native debugger |
| Visual Studio Build Tools 2026 | `18.12.1+12412.261`, stable, `C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools` | **Selected** compiler toolchain for reproducible builds |
| MSVC toolset | **`14.52.37025`** (present in both 18.x installs) | x64/x86 native toolchain |
| Visual Studio Build Tools 2019 | `16.11.58`, MSVC `14.29.30133` | Legacy only; not selected by the helper |
| Windows SDK | `10.0.19041.0` and **`10.0.26100.0`** | `10.0.26100.0` is the required SDK for this workflow |
| WebView2 Runtime | `158.0.4370.33` (Evergreen) | Tauri Studio prerequisite |
| Rust | `1.100.0`, host `x86_64-pc-windows-msvc` | Pinned by `rust-toolchain.toml` |

> **Version drift is expected and must be re-measured, not assumed.** The Build Tools
> instance was at `18.11.1` / MSVC `14.51` when this document was first drafted and is now
> `18.12.1` / MSVC `14.52`. The helper discovers the toolset at run time, so it does not
> hard-code a version. Re-run `-Action Doctor` after any Visual Studio update.

`Get-StableVs18Path` selects a **stable** 18.x installation with the MSVC x64/x86
component and prefers `BuildTools`. That is why the Insiders IDE is used for the debugger
experience while the stable Build Tools instance supplies the compiler. The helper does
not install or update Visual Studio, and no script in this repository will.


## 2. Division of responsibility

- **Cargo is authoritative** for Rust dependency resolution, formatting, linting, testing and building.
- **Visual Studio Community** is the IDE/debugger UI. Open the repository with **File > Open > Folder**.
- **Visual Studio Build Tools 2026** supplies MSVC, linker, Windows SDK integration, MSBuild and native diagnostics.
- **OpenCode + OMO** remains the AI engineering orchestrator. Agents edit the repository; they do not delegate build authority to Visual Studio.
- No C++ application, solution or duplicate build system is created simply to make the repository appear to be a Visual Studio project.

Microsoft documents Open Folder build tasks in the tracked .vs/tasks.vs.json file and debug configurations in .vs/launch.vs.json. These files are tracked for the shared developer workflow; machine-local Visual Studio cache/state remains ignored.

## 3. Reproducible installer configurations

Two configurations are provided because Visual Studio Community and Build Tools use different workload IDs:

- Root .vsconfig: Community 2026 / Community 2026 Insiders. Workload Microsoft.VisualStudio.Workload.NativeDesktop.
- vsconfig/BuildTools-2026.vsconfig: Build Tools 2026. Workload Microsoft.VisualStudio.Workload.VCTools.

Both request documented MSVC x64/x86, Windows SDK 10.0.26100 and CMake components. The Community configuration additionally requests the documented Windows Performance Toolkit and C++ Diagnostic Tools. The Build Tools manifest omits the Windows Performance Toolkit component because it is not listed in Microsoft's current Build Tools component catalogue; install Windows ADK/Windows Performance Toolkit separately when ETW tooling is required.

To import a configuration: open Visual Studio Installer, choose **More > Import configuration**, select the corresponding file, review the selected components, and apply. Review the Installer's changes before accepting; the configuration files do not silently update products.

Microsoft also documents command-line modification with setup.exe modify --installPath <path> --config <file>. Use modify, not update, when applying a component configuration to an existing installation.

## 4. Initialize and inspect the native build environment

From a repository-root PowerShell:

**powershell -ExecutionPolicy Bypass -File .\scripts\vs\Invoke-DiscoDeskVS.ps1 -Action Doctor**

The helper:
1. Locates vswhere.exe.
2. Selects a stable 18.x Visual Studio instance with MSVC x64/x86; prefers Build Tools when present.
3. Runs `VsDevCmd.bat -arch=x64 -host_arch=x64` and imports its environment into the helper process.
4. Adds the rustup-managed %USERPROFILE%\.cargo\bin directory to PATH when present. It does not install Rust or change the pinned toolchain.
5. Reports cl.exe, link.exe, MSBuild, CMake, SDK include versions, Rust/Cargo, WebView2, Community IDE and whether the engine entry point is present.

### 4.1 Defect fixed: VsDevCmd environment import
The first implementation passed `cmd /s /c "..."` to PowerShell as a single native
argument. PowerShell re-quotes native arguments, so the nested quotes were corrupted and
the Doctor failed with `'""C:\Program' is not recognized as an internal or external
command`. A second defect read `ExitCode` after `Dispose()`, discarding it.

`Import-VsEnvironment` now writes the `call` + `set` sequence to a temporary batch file
and invokes it as a single space-free argument, which removes the quoting problem
entirely, and captures the exit code before disposal. The batch prints only `set` output
(VsDevCmd's banner is suppressed) and the parser accepts only `NAME=value` lines, so a
banner line can never become an environment variable.

Verified result on this host (`-Action Doctor`, exit code 0):

```text
Selected stable VS 18.x toolchain: C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools
FOUND  cl.exe:     ...\VC\Tools\MSVC\14.52.37025\bin\HostX64\x64\cl.exe
FOUND  link.exe:   ...\VC\Tools\MSVC\14.52.37025\bin\HostX64\x64\link.exe
FOUND  msbuild.exe:...\MSBuild\Current\Bin\amd64\MSBuild.exe
FOUND  cmake.exe:  ...\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe
FOUND Windows SDK 10.0.26100.0 headers and x64 libraries.
Rust: rustc 1.100.0 (0ef1bc386 2026-11-09)   (target x86_64-pc-windows-msvc)
Cargo: cargo 1.100.0 (3acc79c0c 2026-11-05)
WebView2 Runtime: 158.0.4370.33
Visual Studio IDE: C:\Program Files\Microsoft Visual Studio\18\Insiders\Common7\IDE\devenv.exe
```


The Rust project continues to use rust-toolchain.toml and x86_64-pc-windows-msvc. A shell that cannot find rustc by default is a PATH issue if the pinned Rust toolchain exists under .cargo\bin; it is not evidence that Rust must be reinstalled.

## 5. Visual Studio Open Folder tasks

The checked-in .vs/tasks.vs.json exposes:

- Environment Doctor
- Canonical Gate 0 Check (scripts/check.ps1)
- Debug Build
- Debug Rebuild
- Optimized Build with Symbols
- Workspace Tests
- Developer PowerShell
- **Build Studio (frontend + debug)**
- **Build Studio (optimized + symbols)**
- **Probe Studio to engine**
- **Studio launch smoke (real window + engine survival)**

The Studio tasks exist because the Studio crate cannot compile until its frontend is
built: `tauri-build` embeds `frontendDist` at compile time. `-Action BuildStudio` runs
`npm ci` (when `node_modules` is absent) and `npm run build` before invoking cargo, which
is the same ordering `scripts/check.ps1` and CI use. `-Action ProbeStudio` runs the built
binary's `--probe` against a **running** engine; it does not start the engine.

Open the repository folder in Visual Studio. Use the Solution Explorer context menu's configured tasks or the Build menu entries. Task output appears in the Visual Studio Output window; errors are surfaced by the custom task integration.

## 6. Debugging and symbols

The checked-in .vs/launch.vs.json provides:
- DiscoDesk Engine (Debug) -> target/debug/discodesk-engine.exe
- DiscoDesk Engine (Optimized + Symbols) -> target/release-with-symbols/discodesk-engine.exe
- **DiscoDesk Studio (Debug) -> target/debug/discodesk-studio.exe**
- **DiscoDesk Studio (Optimized + Symbols) -> target/release-with-symbols/discodesk-studio.exe**

Debugging the Studio requires a running engine if you want the connected state; launch the
engine profile first, then the Studio profile. The reverse order is also valid and is what
proves independence: the Studio starts, shows "Engine not running", and switches to
"Engine connected" on its next poll once the engine appears.

### 6.1 Current debug readiness (verified 2026-11-27)

`apps/discodesk-engine/src/main.rs` now exists, so the reason the previous revision of
this document gave for F5 being unavailable no longer applies. The helper's
`Assert-EngineEntryPoint` guard remains: if that file is ever removed, the build tasks
refuse rather than fabricate an entry point.

Both profiles build and emit symbols:

| Artifact | Size |
|---|---|
| `target/debug/discodesk-engine.exe` | 4,816,384 B |
| `target/debug/discodesk_engine.pdb` | 39,563,264 B |
| `target/release-with-symbols/discodesk-engine.exe` | 1,370,624 B |
| `target/release-with-symbols/discodesk_engine.pdb` | 20,631,552 B |
| `target/debug/discodesk-studio.exe` | 12,758,528 B |
| `target/debug/discodesk_studio.pdb` | 111,489,024 B |
| `target/release-with-symbols/discodesk-studio.exe` | 4,535,296 B |
| `target/release-with-symbols/discodesk_studio.pdb` | 66,236,416 B |

> **PDB naming:** Rust names the PDB after the **underscored** crate name
> (`discodesk_engine.pdb`), not after the binary (`discodesk-engine.exe`). A check for
> `discodesk-engine.pdb` reports a false negative. VS launch profiles point at the
> `.exe`; the debugger locates the sibling PDB by its own naming rules.

**Not yet verified:** an interactive debug session (breakpoint hit, native call stack
walk, module/symbol load). The binaries, profiles and PDBs are in place, but stepping
through code in the IDE requires a human at the machine and has not been recorded as
evidence. Do not claim "F5 debugging works" until that is done.

The existing release profile strips symbols and remains correct for distribution
artifacts. A separate `release-with-symbols` profile inherits release optimisation but
retains full debug information; use it for optimized debugging and profiling. Keep
generated PDB files out of Git.

## 6.2 Interactive debugging procedure

A step-by-step procedure for a human-operated debugger session against
discodesk-studio.exe - launch profile, breakpoint location, how to confirm PDB
loading, the expected call stack and the evidence to record - is in
[VISUAL_STUDIO_DEBUGGING.md](VISUAL_STUDIO_DEBUGGING.md). The binaries, PDBs (including
embedded Rust source paths) and launch profiles are verified; the interactive session
itself is recorded as awaiting human verification.

## 7. Profiling and diagnostics

- Use Visual Studio native debugger, call stacks and CPU Usage tooling for supported native debugging scenarios.
- Use Windows Performance Toolkit / Windows Performance Recorder for ETW-based system scheduling and CPU investigations when needed.
- Use PIX for Direct3D 12 GPU capture and analysis where the active wgpu backend is D3D12.
- If wgpu selects Vulkan, use Vulkan-compatible tooling such as RenderDoc where supported. Visual Studio Graphics Diagnostics is not a universal Vulkan/WGSL debugger.
- Record exact backend, GPU, driver, build profile and capture method with every performance result.

## 8. Local commit helper

The repository includes scripts/git/Commit-DiscoDesk.ps1. It stages only explicit paths, refuses pre-staged unrelated changes and obvious secret-file paths, runs git diff --cached --check, shows the staged diff, and requires typing COMMIT. It does not run git add ., pull, rebase or push unless -Push is explicitly supplied.

Example:

powershell -ExecutionPolicy Bypass -File .\scripts\git\Commit-DiscoDesk.ps1 -Message "chore: integrate Visual Studio workflow" -Paths ".vsconfig", "vsconfig/BuildTools-2026.vsconfig", ".vs/tasks.vs.json", ".vs/launch.vs.json", "Cargo.toml", ".gitignore", "scripts/vs/Invoke-DiscoDeskVS.ps1", "scripts/git/Commit-DiscoDesk.ps1", "docs/VISUAL_STUDIO_WORKFLOW.md", "docs/ADR/0010-visual-studio-open-folder-native-debugging.md", "docs/PLATFORM_SUPPORT.md", "docs/GATE0_PLAN.md", "OPENCODE_MASTER_DIRECTIVE.md", "AGENTS.md", "README.md", ".github/README.md"

Add -Push only when you explicitly want the helper to publish the commit to origin.

## 9. Upstream documentation

- Visual Studio Community workload/component IDs: https://learn.microsoft.com/en-us/visualstudio/install/workload-component-id-vs-community
- Visual Studio Build Tools workload/component IDs: https://learn.microsoft.com/en-us/visualstudio/install/workload-component-id-vs-build-tools
- Import/export .vsconfig: https://learn.microsoft.com/en-us/visualstudio/install/import-export-installation-configurations
- Open Folder custom build/debug tasks: https://learn.microsoft.com/en-us/visualstudio/ide/customize-build-and-debug-tasks-in-visual-studio
- Rust Windows MSVC target: https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html
- Rust debug information options: https://doc.rust-lang.org/rustc/codegen-options/index.html#debuginfo
- Tauri Windows prerequisites: https://tauri.app/start/prerequisites/


### GitHub Actions Visual Studio verification

The Windows jobs use `windows-2025-vs2026`, which GitHub documents as the explicit Windows Server 2025 image with Visual Studio 2026. The lint job runs the same Visual Studio Doctor. This verifies the compiler, linker, MSBuild, pinned Windows SDK and Rust MSVC host in CI before the Cargo checks.
