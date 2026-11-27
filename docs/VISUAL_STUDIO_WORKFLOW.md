# Visual Studio 2026 workflow for DiscoDesk

**Last updated:** 2026-11-27  
**Scope:** Windows developer tooling, Open Folder integration, Cargo tasks and native debugging.  
**Product status:** Development integration only. It does not mean the engine or Studio is complete.

## 1. Actual development-host inventory

Evidence supplied by the developer on 2026-11-27:

| Installed product | Screenshot / command evidence | Use in DiscoDesk |
|---|---|---|
| Visual Studio Community 2026 Insiders | Listed under Installed; product update waiting to download at 0% | IDE and Open Folder experience after its pending update finishes |
| Visual Studio Build Tools 2026 | Version 18.11.1 (October 2026 feature update); MSVC 14.51.36231 confirmed in PowerShell | Preferred stable 18.x MSVC x64/x86 toolchain |
| Visual Studio Build Tools 2019 | Version 16.11.58; update 16.11.63 available | Retain as legacy installation; not selected by the DiscoDesk helper |
| Windows SDK 10.0.19041.0 | Detected in PowerShell | Existing compatibility SDK |
| Windows SDK 10.0.26100.0 | Detected in PowerShell | Preferred SDK for this Windows 10 development host |

The Installer screenshot shows an available November 2026 update (18.12.1) for Build Tools 2026 and a pending update for Community 2026 Insiders. Neither update is recorded as installed. The helper selects an installed stable Visual Studio 18.x instance with the MSVC x64/x86 component; it does not install or update Visual Studio.

The Insiders IDE is suitable for evaluating the IDE experience. Prefer the stable Build Tools 2026 instance for the compiler toolchain and reproducible builds. Keep Build Tools 2019 installed unless there is a separately justified reason to remove it.

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
3. Runs VsDevCmd.bat -arch=x64 -host_arch=x64 and imports its environment into the helper process.
4. Adds the rustup-managed %USERPROFILE%\.cargo\bin directory to PATH when present. It does not install Rust or change the pinned toolchain.
5. Reports cl.exe, link.exe, MSBuild, CMake, SDK include versions, Rust/Cargo, WebView2, Community IDE and whether the engine entry point is present.

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

Open the repository folder in Visual Studio. Use the Solution Explorer context menu's configured tasks or the Build menu entries. Task output appears in the Visual Studio Output window; errors are surfaced by the custom task integration.

## 6. Debugging and symbols

The checked-in .vs/launch.vs.json provides:
- DiscoDesk Engine (Debug) -> target/debug/discodesk-engine.exe
- DiscoDesk Engine (Optimized + Symbols) -> target/release-with-symbols/discodesk-engine.exe

The current public main snapshot at commit 2912a7e5b356301121c2956b301fc56dd364b23d contains apps/discodesk-engine/Cargo.toml, whose binary manifest declares src/main.rs, but that source file is absent from the public tree. The build helper explicitly refuses to fabricate an entry point. F5 becomes usable after the real Gate 0 engine source is committed and the corresponding Cargo build succeeds.

The existing release profile strips symbols. It remains unchanged for normal release artifacts. A separate release-with-symbols profile inherits release optimisation but retains full debug information; use it for optimized debugging and profiling. Keep generated PDB files out of Git.

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
