# DiscoDesk Studio

The on-demand control UI. **Not** the wallpaper renderer.

- Separate process from `discodesk-engine` (`docs/ADR/0001`).
- Talks to the engine only over the versioned local IPC contract
  (`docs/IPC_PROTOCOL.md`, `docs/ADR/0002`).
- Owns no engine state, no audio capture, no DSP, no renderer, no wallpaper host.
- **Never** stops the engine. There is no code path that sends `ShutdownEngine`.

## Layout

```text
apps/discodesk-studio/
├── package.json            npm project (frontend toolchain)
├── index.html
├── vite.config.ts          Vite + Vitest configuration
├── tsconfig.json
├── src/                    React 19 + TypeScript UI
│   ├── connection.ts       pure view-model translation (unit tested)
│   ├── api.ts              the only module that imports @tauri-apps/api
│   ├── App.tsx             the only screen: connection state + engine details
│   └── styles.css
└── src-tauri/              Tauri v2 backend
    ├── tauri.conf.json
    ├── capabilities/default.json
    ├── build.rs            fails clearly if the frontend is not built
    ├── icons/              provisional application icon
    └── src/
        ├── engine_client.rs  EngineClient + Connector trait (unit tested)
        ├── commands.rs       engine_status, engine_endpoint, engine_disconnect
        ├── lib.rs            app wiring, --probe support
        └── main.rs           CLI entry
```

## Build order matters

`tauri-build` embeds the frontend at compile time, so the frontend must exist before any
cargo command that touches this crate. `build.rs` fails with instructions rather than
creating a placeholder directory.

```powershell
cd apps/discodesk-studio
npm ci
npm run build        # produces ./dist, which ../src-tauri requires
```

Then, from the repository root:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo build -p discodesk-studio
```

Or use the helpers, which do the ordering for you:

```powershell
powershell -File scripts/vs/Invoke-DiscoDeskVS.ps1 -Action BuildStudio
powershell -File scripts/vs/Invoke-DiscoDeskVS.ps1 -Action BuildStudioSymbols
```

## Run

```powershell
# terminal 1
cargo run -p discodesk-engine

# terminal 2
cd apps/discodesk-studio
npx tauri dev
```

Either process may be started first. If the engine is absent the Studio shows
"Engine not running" and reconnects automatically on the next poll.

## Headless probe

```powershell
target\release\discodesk-studio.exe --probe
target\release\discodesk-studio.exe --probe --endpoint '\\.\pipe\discodesk-engine-<user>'
```

Prints the connection view as JSON and exits `0` only when it reached a healthy engine.
This is how `scripts/ipc-smoke.ps1` verifies the Studio↔engine boundary with the real
binaries.

## Tests

```powershell
npm test                 # 35 frontend tests (Vitest + jsdom + Testing Library)
cargo test -p discodesk-studio   # 18 backend tests
powershell -File scripts/studio-smoke.ps1   # real window + engine survival
```

## Audio panel

Studio renders the engine's live audio telemetry: device and format, six level meters
(each with a printed numeric value, so colour is never the only signal), overall level in
dBFS, an onset indicator, packet/drop counters and DSP timing.

It polls one interval with one in-flight guard, and requests audio separately from engine
status, so a missing audio subsystem cannot make the connection state look broken. The
Studio never captures audio itself — the engine owns the device.

## Deliberate limitations

- **Console subsystem is attached** in windowed mode, so `--probe` can write to stdout and
  diagnostics stay visible. Hiding it is a polish item, and it must not be done by breaking
  `--probe`.
- **No installer bundling.** `bundle.active` is `false`; `tauri build` produces the
  executable only. Installers, signing and icons for distribution are later work.
- **The icon is provisional** and is disclosed as such in `THIRD_PARTY_NOTICES.md`.
- **A wedged engine leaves one blocked thread.** Named-pipe reads have no timeout; the UI
  bounds this to a single in-flight request, but recovery needs a Studio restart.
- **TypeScript is pinned to 5.9.3**, not the 7.x line: ADR-0008 requires a working build to
  prove compatibility before adopting a new major.
