# ADR-0010: Visual Studio Open Folder integration and native Windows diagnostics

- **Status:** Accepted
- **Date:** 2026-11-27
- **Deciders:** discodesk-lead, systems-architect
- **Gate:** 0 (developer environment)

## Context

The primary Windows development host has Visual Studio Community 2026 Insiders and Visual Studio Build Tools 2026 installed, as shown in the Visual Studio Installer screenshot supplied on 2026-11-27. PowerShell previously reported Build Tools 2026 18.11.1, MSVC 14.51.36231, Windows SDK 10.0.19041.0 and 10.0.26100.0. The screenshot shows an 18.12.1 Build Tools update available and the Community Insiders update waiting to download; neither update is recorded as installed.

DiscoDesk is a Rust Cargo workspace with a planned Tauri Studio. Cargo must remain the Rust build authority. Creating an artificial C++ project or a duplicate MSBuild graph would misrepresent the architecture and produce two competing build definitions.

## Decision

1. Integrate Visual Studio using its **Open Folder** workflow, tracked .vs/tasks.vs.json build tasks and .vs/launch.vs.json debugger configurations.
2. Provide separate Community and Build Tools .vsconfig files because their C++ workload IDs differ. Component IDs were checked against Microsoft's current Visual Studio 2026 component directories:
   - Community: Microsoft.VisualStudio.Workload.NativeDesktop.
   - Build Tools: Microsoft.VisualStudio.Workload.VCTools.
3. Prefer an installed stable Visual Studio 18.x Build Tools instance with MSVC x64/x86 and Windows SDK 10.0.26100 for the reproducible compiler environment. Community 2026 Insiders is the IDE candidate shown in the supplied Installer screenshot, but an Insiders channel is not the stable build baseline.
4. Import the environment with VsDevCmd.bat -arch=x64 -host_arch=x64; add the existing rustup-managed .cargo\bin path when present. Do not reinstall Rust as part of IDE integration.
5. Keep Cargo as the source of truth. Visual Studio tasks invoke Cargo or the existing scripts/check.ps1; they do not duplicate compilation in MSBuild.
6. Preserve the existing stripped release profile. Add a separate release-with-symbols profile for optimized native debugging/profiling.
7. Track only portable .vs/tasks.vs.json and .vs/launch.vs.json; ignore other .vs user/cache state.
8. Do not fabricate a missing engine entry point or claim F5 works until a real binary is built.

## Consequences

- Developers can use the familiar Visual Studio IDE, native debugger and diagnostics while retaining Rust/Cargo ownership.
- The actual compiler environment is reproducible through .vsconfig and the diagnostic script.
- Visual Studio's Rust editing support is not treated as equivalent to rust-analyzer. OpenCode remains the AI editing interface; the IDE is used for inspection/debugging.
- GPU diagnostic tools depend on the selected wgpu backend: D3D12 captures can use PIX; Vulkan needs Vulkan-compatible tooling. Visual Studio Graphics Diagnostics is not a universal shader debugger.
- The debug launch profiles are prepared but remain unavailable until the real engine entry point is present and the Cargo target builds.

## Verification

- Validate both .vsconfig files as JSON.
- Validate .vs/tasks.vs.json and .vs/launch.vs.json against their documented schema version 0.2.1 structure.
- Run the VS environment Doctor on the developer's Windows machine.
- Run scripts/check.ps1 from the VS-initialized x64 environment.
- Build debug and optimized-with-symbols engine targets after apps/discodesk-engine/src/main.rs is present.
- Verify F5 launch, breakpoints, PDB loading and clean shutdown on the actual Windows host.

## References

- Microsoft Visual Studio Community workload IDs: https://learn.microsoft.com/en-us/visualstudio/install/workload-component-id-vs-community
- Microsoft Visual Studio Build Tools workload IDs: https://learn.microsoft.com/en-us/visualstudio/install/workload-component-id-vs-build-tools
- GitHub Actions Windows 2025 / Visual Studio 2026 runner-label migration: https://github.com/actions/runner-images/issues/14017
- Microsoft .vsconfig import/export: https://learn.microsoft.com/en-us/visualstudio/install/import-export-installation-configurations
- Microsoft Open Folder tasks/debugging: https://learn.microsoft.com/en-us/visualstudio/ide/customize-build-and-debug-tasks-in-visual-studio
- Rust Windows MSVC target: https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html
- Rust debug information: https://doc.rust-lang.org/rustc/codegen-options/index.html#debuginfo
- Tauri prerequisites: https://tauri.app/start/prerequisites/
