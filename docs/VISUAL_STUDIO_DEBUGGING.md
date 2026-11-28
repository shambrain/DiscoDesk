# Interactive Visual Studio debugging procedure (Studio)

**Status:** prepared and verified as far as is possible without a human at the IDE.
**Gate item:** Gate 0 / B1 — interactive debugger verification.

This is a *human-operated* check. Everything an operator needs is committed and verified:
the binary, a matching PDB containing Rust source paths, a launch profile, and a breakpoint
location that is actually reached at runtime. What has **not** happened is a person
stepping through it, so this item is recorded as **awaiting human verification**, not as
passed.

---

## Verified prerequisites (measured on this host, 2026-11-28)

| Item | Value |
|---|---|
| Executable | `target\debug\discodesk-studio.exe` (12,773,376 B) |
| PDB | `target\debug\discodesk_studio.pdb` (111,562,752 B) |
| Timestamps agree | both `12:12:08` — the PDB matches this build |
| PDB contains Rust source paths | `engine_client.rs`, `commands.rs`, `lib.rs`, `discodesk-studio` all present in the PDB bytes |
| Launch profile | `.vs/launch.vs.json` → `DiscoDesk Studio (Debug)` → `target\debug\discodesk-studio.exe` |
| Breakpoint target exists | `engine_client.rs:258` `pub fn status`, reached every poll |

> **PDB naming:** Rust writes `discodesk_studio.pdb` (underscored crate name), next to
> `discodesk-studio.exe`. Visual Studio finds it by its own rules; do not rename it.

---

## The procedure

### 1. Which launch profile to select

In Visual Studio 2026, open the repository folder (**File > Open > Folder**), then in the
toolbar's launch dropdown select **DiscoDesk Studio (Debug)**.

Start the engine first in a separate terminal if you want to inspect the *connected* path:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo run -p discodesk-engine
```

The Studio also runs without the engine; the breakpoint is hit either way, and the
unavailable path is worth inspecting too.

### 2. Which Rust file and function to open

`apps/discodesk-studio/src-tauri/src/engine_client.rs`, function `EngineClient::status`
(around line 258). It is called by the UI's 2-second poll, so it is reached without any
interaction.

### 3. Exact breakpoint location

Set a **function breakpoint** on:

```
discodesk_studio::engine_client::EngineClient::<discodesk_studio::engine_client::PipeConnector>::status
```

If function breakpoints are awkward in this VS version, instead open `engine_client.rs`,
put the caret on this statement inside `status`, and press F9:

```rust
        match self.request(Command::GetEngineStatus) {
```

A second useful breakpoint, one level deeper, is the first statement of
`fn exchange(...)`:

```rust
    let envelope = RequestEnvelope::new(request_id, command);
```

### 4. How to verify PDB loading

**Debug > Windows > Modules** (only available while debugging). Find
`discodesk-studio.exe` and confirm:

- **Symbol Status** reads `Symbols loaded`;
- **Symbol File** points at `target\debug\discodesk_studio.pdb`.

The **Output** window at debug start should show the module loading. If symbol status says
`Binary was not built with debug information` or `Cannot find or open the PDB file`, the
wrong executable or a stale PDB is selected — rebuild with
`-Action BuildStudio` and check the timestamps in the table above.

### 5. Expected call stack

With the `status` breakpoint hit, the Call Stack window should show frames resembling:

```
discodesk_studio::engine_client::EngineClient<...>::status
discodesk_studio::commands::engine_status::{{closure}}
tokio::runtime::task::core::...        (blocking pool)
std::thread::...
```

With the `exchange` breakpoint hit, `status` and the `commands::engine_status` closure
appear above it. Frame names are mangled by default; right-click the Call Stack and enable
**Show Demangled Names** / disable **Hide External Code** as needed.

### 6. What evidence to record

Record all of the following in `docs/IMPLEMENTATION_STATE.md` §3 (or attach to the PR):

1. A screenshot or text capture of the **Modules** window showing
   `discodesk-studio.exe` → `Symbols loaded` and the PDB path.
2. The breakpoint hit, with the file and line visible in the editor.
3. The **Call Stack** window showing at least `status` plus one caller frame.
4. Whether the Studio was connected or disconnected at the time.
5. The process exit code after closing the window (expected `0`), and confirmation that
   the engine process was still running afterwards.

---

## Honest limitations to expect

- Visual Studio debugs Rust through the **C++ debugger** with MSVC PDBs. Source-level
  stepping generally works, but it is not `rust-analyzer`, and some optimised or generic
  frames will not map cleanly. Use the **Debug** profile for breakpoints, not
  `release-with-symbols` (that profile is for profiling and crash analysis, where symbols
  still resolve but stepping is less reliable).
- Inlined and monomorphised functions can make a frame look like it "skipped" a call.
  Disable **Just My Code** if frames appear collapsed.
- The engine is a separate process. To debug *both*, start the engine outside Visual Studio
  and attach a second debugger instance, or run a second VS instance with the
  **DiscoDesk Engine (Debug)** profile. Do not expect one launch profile to own both.
