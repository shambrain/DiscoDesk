# DiscoDesk — Research Audit

**Research snapshot:** 26 November 2026  
**Purpose:** Validate the product opportunity, inspect existing solutions, audit the supplied Gemini proposal, and justify the proposed engineering direction.

> This is a research snapshot, not a claim that every product or repository in the market has been exhaustively audited. Current product features, prices, platform support, APIs and model terms should be rechecked before release.

---

## 1. Executive assessment

DiscoDesk is technically feasible and has a credible product opportunity, but the statement that no similar software exists is incorrect.

Existing software already provides combinations of:

- music-reactive wallpaper and spectrum visualization;
- animated shader effects applied to arbitrary images;
- image-to-depth wallpaper conversion;
- local desktop wallpaper hosting;
- app/source selection and game/fullscreen pausing;
- cross-platform music visualization.

The stronger opportunity is a cohesive **automatic still-image-to-audio-reactive-scene workflow**: the user supplies an ordinary picture, DiscoDesk infers conservative motion and light regions, creates a tasteful recipe, and reacts to music without requiring manual shader authoring or a separate visualizer widget.

It is not yet established that this exact integration is unique worldwide. The product must differentiate through execution, ergonomics, image quality, DSP quality, reliable platform support and performance.

### Recommended direction

Build a Rust native engine with a Tauri Studio frontend and a `wgpu` renderer. Implement native audio and wallpaper-host adapters per operating system. Make AI depth analysis optional, one-shot and cached. Ship Windows first, because that is the user's current environment, then expand through macOS and Linux adapters after native host and permission testing.

---

## 2. Market and project audit

| Product / project | What it already proves | Gap relative to DiscoDesk |
|---|---|---|
| [Wallpaper Engine](https://store.steampowered.com/app/431960/Wallpaper_Engine/) | Commercial live-wallpaper ecosystem, scene effects and community content. Its docs include manual depth-parallax effect configuration. | Depth-parallax scene setup is not the same as automatically analysing any single still image and generating a music-aware recipe. |
| [Lively Wallpaper](https://github.com/lively-community/lively) | Open-source Windows wallpaper engine; audio visualizer API and existing WASAPI loopback service; separate image depth-analysis workflow based on MiDaS / ONNX. | The inspected implementation exposes an audio spectrum array for visualizers. The depth-estimation workflow is a separate wallpaper creation flow, not a unified automatic audio-reactive still-image transformation. |
| [Lively audio-visualizer wallpaper](https://github.com/rocksdanister/audio-visualizer-wallpaper) | A visualizer wallpaper can receive frequency values through Lively's `livelyAudioListener` and draw a canvas spectrum. | Conventional overlay/spectrum approach, rather than intelligent motion of original artwork. |
| [Lively depth-map wallpaper](https://github.com/rocksdanister/depthmap-wallpaper) | Open-source example that depth maps can be used as wallpaper assets. | Depth effects and music reaction are separate product capabilities. |
| [KOpenWallpaper](https://github.com/hteariH/kopenwalpaper) | KDE Plasma 6 plugin with a Living Image shader preset (breathing zoom, parallax sway, chromatic aberration, bokeh) and separate system-audio / spectrum inputs exposed to custom shaders. Its README describes a ~30 FPS cap and pausing when windows are maximized/fullscreen. | KDE Plasma-specific; requires selecting/configuring shader presets and audio response. It demonstrates much of the concept, so DiscoDesk cannot claim to invent these effects. |
| [Shader Wallpaper for KDE](https://github.com/y4my4my4m/kde-shader-wallpaper) | 270+ shader presets, PipeWire audio reactivity, multi-pass rendering and performance monitoring. | KDE-specific shader platform; not a cross-platform automatic image-analysis workflow. |
| [DcentWallpapers](https://github.com/dcenhance/dcent-wallpaper-linux) | KDE Plasma 6 provider for local Wallpaper Engine content; PipeWire/PulseAudio audio feed and native scene, web, video and image paths. | KDE-focused, and audio response depends on the underlying wallpaper implementing an audio listener/effect. |
| [Music Visualizer Engine PC Live Wallpaper](https://store.steampowered.com/app/1204920/Music%20Visualizer%20Engine%20PC%20Live%20Wallpaper/) | Steam product released Jan 28, 2020; describes 3D wallpapers reacting to system audio and configurable scenes. | Prebuilt scenes and visualizer-centric experience; not the same as automatic depth-aware movement of any supplied still image. |
| [Animate Desktop](https://store.steampowered.com/app/5255400/Animate_Desktop/) | A recent Steam competitor advertising bass/beat/melody reactivity and automatic pause for fullscreen/game conditions. | Must be tested directly against the final DiscoDesk workflow; product descriptions alone do not establish its image-analysis capabilities. |
| [Open Wallpaper Engine](https://openwallpaperengine.app/) | macOS implementation supporting Wallpaper Engine-style scenes and Core Audio audio-reactive effects on supported macOS versions. | Does not establish universal cross-platform automatic conversion of arbitrary still images. |
| [projectM](https://github.com/projectM-visualizer/projectm) | Mature cross-platform MilkDrop-style audio visualization library; useful research reference for audio feature-to-visual mapping. | Its psychedelic procedural visuals are not the intended restrained still-image animation model. Check LGPL and preset asset licensing before integration. |
| [WebDesk](https://github.com/NAME0x0/WebDesk) | Windows C#/.NET and WebView2 app with image/video/web/shader wallpaper types and audio-reactive shaders. | Useful architecture comparison, not a turnkey general cross-platform image-analysis engine. |
| [OpenDance](https://github.com/WodsonKun/OpenDance) | GameMaker Language recreation of Ubisoft Just Dance gameplay engines, using UbiArt CKD and Bluestar JSON structures, with choreography/video/audio playback. | No OS audio capture, wallpaper hosting, live DSP-to-shader pipeline or image depth analysis. Not a suitable foundation or dependency. |

### What to study, not blindly copy

- **Lively:** Windows desktop hosting, wallpaper lifecycle, audio-source UX, pause behaviour, and its practical MiDaS/ONNX workflow.
- **KOpenWallpaper:** its Living Image shader controls, audio feature uniforms, and low-FPS/pause design.
- **Wallpaper Engine:** scene composition, depth-parallax controls, creator ecosystem, and Workshop workflows.
- **projectM:** DSP and music-reactive artistic parameterization. Do not copy its psychedelic visual language as DiscoDesk's default.
- **Open Wallpaper Engine:** macOS Core Audio tap and desktop-surface integration lessons.

Do not reuse code just because a repository is public. Audit the exact license and all bundled assets/model weights before copying, linking or redistributing anything.

---

## 3. Audit of the supplied Gemini proposal

| Gemini proposal | Assessment | Correction / action |
|---|---|---|
| “Nobody has built anything like this.” | Incorrect / unproven. | Existing products cover several major components. Position DiscoDesk around automatic, image-aware composition and user experience, not a first-ever claim. |
| OpenDance is a Just Dance GML recreation. | Confirmed. | Its README describes a GameMaker recreation of Ubisoft Just Dance gameplay using UbiArt/Bluestar data and clips. Do not use as a foundation. |
| FigCraft is useful for UI design. | Reasonable workflow choice. | Use it to design the Studio interface; do not treat it as runtime architecture or an implementation dependency. |
| Rust + Tauri + wgpu. | Strong recommendation. | Rust native engine; Tauri/React for on-demand UI; wgpu/WGSL for native wallpaper rendering. Separate processes and IPC. |
| Java / Node server should power the app. | Not recommended. | No Java or always-on Node backend required. Node can remain a frontend build-time tool only. |
| Windows process-specific loopback works on all modern Windows 10. | Too broad. | Microsoft's Application Loopback sample requires Windows 10 build 20348 or later. Normal retail Windows 10 builds such as 19042/19045 need system/output loopback fallback. |
| macOS can use ScreenCaptureKit / CoreAudio tap. | Needs a more specific choice. | Prefer Core Audio process taps for process-specific audio on macOS 14.2+; declare capture permission. ScreenCaptureKit is another system capture framework, but should not be presented as a universal silent tap. |
| Linux PipeWire capture. | Correct direction, incomplete platform claim. | PipeWire streams can consume audio and monitor sink output. Per-app capture depends on graph visibility, routing and session policy. Wallpaper surfaces vary by compositor. |
| 2,048-point FFT provides enough sub-bass detail. | Not adequate alone. | At 48 kHz, 2,048 samples span 42.7 ms and bin spacing is 23.4 Hz. Add a long low-frequency analysis path and fast transient path. |
| Six zones are six Bark bands / music stems. | Incorrect. | They are custom broad frequency zones. They cannot isolate instruments or vocals. Bark scale has many more critical bands. |
| FFT spectral flux detects beats. | Incomplete. | Flux is an onset/transient feature. Beat/tempo detection is a separate probabilistic process and should expose confidence. |
| 5–15 ms attack from a 2,048 FFT with 50% overlap. | Timing mismatch. | The hop is ~21.3 ms at 48 kHz; sub-hop attack needs block/sample-level filters, not just that FFT schedule. |
| ±6% full-screen luminance is a universal safe limit. | Unsupported as a universal safety threshold. | Avoid full-screen flashing entirely by design; keep conservative local effects and user-controlled motion. WCAG flashing criteria are not a guarantee of comfort. |
| Depth Anything V2 Small is the latest model. | Outdated by the 2025 DA3 release. | Compare DA2 Small and DA3 Small. DA2 Small has practical ONNX options; DA3 Small is an Apache-licensed candidate and has a 2026 C++/ggml port. Benchmark both. |
| Depth map means 3D understanding. | Overstated. | Monocular depth gives an estimated relative/metric depth field; hidden pixels, topology and true geometry are ambiguous. Keep displacement subtle and confidence-aware. |
| Model must be running to animate. | Incorrect. | Model inference is only needed at import/re-analysis. Playback uses cached textures and shaders. A no-model deterministic path is possible. |
| `<0.8% CPU`, `<60 MB VRAM`, `60 FPS at 4K`. | Unsupported without measurement. | These are benchmark targets at best. 4K RGBA source alone is ~31.6 MiB; depth, masks, swapchain and intermediate textures add more. Publish measured numbers only. |
| WorkerW / Progman is a universal stable wallpaper API. | Incorrect. | It is a Windows shell integration technique. Isolate, test and recover it; keep desktop integration in native adapters. |
| wlr-layer-shell is all Linux Wayland. | Incorrect. | It is an unstable protocol supported by some compositors. KDE native wallpaper plugin is the reliable Plasma route; GNOME and others require separate work. |
| Steam Workshop package support. | Good future feature. | Use modern `ISteamUGC` (`CreateItem`, `StartItemUpdate`, `SubmitItemUpdate`) and validate package format/content. Keep Steam optional. |

---

## 4. Audio capture API findings

### Windows — WASAPI

Microsoft documents application loopback with `AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK`, `ActivateAudioInterfaceAsync`, and include/exclude process-tree capture modes. The official sample states Windows 10 build 20348 or later.

- System output capture: WASAPI loopback on render endpoint.
- Process capture: newer application loopback interface, capability-gated.
- Session enumeration: useful for listing app process IDs and session status, but does not by itself provide PCM samples per application.
- Product policy: always show whether a selected source is truly app-specific or a system output mix.

Sources: [Microsoft Application Loopback sample](https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/) · [Process loopback parameters](https://learn.microsoft.com/en-us/windows/win32/api/audioclientactivationparams/ns-audioclientactivationparams-audioclient_process_loopback_params)

**User hardware relevance:** the previously stated Windows 10 20H2 environment is build 19042, below the documented 20348 threshold. Plan the first Windows version around WASAPI system/output loopback and show app lock as unsupported until the OS's capability probe says otherwise. This does not stop the core DiscoDesk experience.

### macOS — Core Audio process taps

Apple's Core Audio process-tap documentation describes creating a process tap and aggregating it into an audio device. The process-tap path is suitable for per-process capture on macOS 14.2+ and requires the audio-capture usage description / permission flow.

Sources: [Capturing system audio with Core Audio taps](https://developer.apple.com/documentation/coreaudio/capturing-system-audio-with-core-audio-taps) · [Open Wallpaper Engine](https://openwallpaperengine.app/)

### Linux — PipeWire / PulseAudio

PipeWire documents streams that consume audio (`PW_DIRECTION_INPUT`) and has an official audio-capture example using `pw_stream_dequeue_buffer` / `pw_stream_queue_buffer`. It documents sink monitoring via `stream.capture.sink`. PulseAudio compatibility can use sink monitor sources.

Sources: [PipeWire Streams](https://docs.pipewire.org/page_streams.html) · [PipeWire audio capture example](https://docs.pipewire.org/audio-capture_8c-example.html) · [PipeWire stream API](https://docs.pipewire.org/group__pw__stream.html)

**Limitation:** app-targeted stream selection depends on PipeWire node visibility and desktop session policy. Build a user-visible capability table and fallback to system mix rather than asserting every Linux desktop has the same routing behavior.

---

## 5. Desktop hosting and renderer findings

### Native graphics

`wgpu` documents Vulkan, Metal, Direct3D 12 and OpenGL backends. Prefer D3D12 on Windows, Metal on macOS, Vulkan on Linux, with explicit adapter fallback and diagnostics. Backend support depends on OS/driver/feature flags; do not promise that every machine has every API.

Sources: [wgpu Backends](https://docs.rs/wgpu/latest/wgpu/struct.Backends.html) · [wgpu Backend enum](https://docs.rs/wgpu/latest/wgpu/enum.Backend.html)

### Windows

Existing wallpaper software such as Lively uses WorkerW/Progman and shell window management techniques. This is practical but relies on implementation details of the Windows desktop shell. Implement a dedicated adapter with attach/re-attach logic, no focus/input capture, and shell restart tests. Keep normal Windows wallpaper intact as a fallback.

Reference implementation: [Lively WinDesktopCore](https://github.com/lively-community/lively/blob/core-separation/src/Lively/Lively/Core/WinDesktopCore.cs)

### macOS

Apple documents SwiftUI `WindowLevel.desktop` as a window level just above the wallpaper and behind other content. This is a useful native host primitive, but still needs testing with Finder icons, Spaces, Mission Control, multiple displays and fullscreen applications.

Source: [Apple WindowLevel.desktop](https://developer.apple.com/documentation/swiftui/windowlevel/desktop)

### Linux

Wayland has no universal wallpaper API. KDE Plasma can host a native Wallpaper plugin; wlroots-based compositors may implement `wlr-layer-shell`. X11 has its own desktop window conventions. Treat KDE, wlr-layer-shell and X11 as separate implementations with explicit supported versions. Do not claim all Linux/Wayland support from one test.

Sources: [wlr-layer-shell protocol](https://github.com/wlr-protocols/wlr-layer-shell-unstable-v1) · [KOpenWallpaper](https://github.com/hteariH/kopenwalpaper)

---

## 6. Model and AI research

### Depth Anything V2 Small

The official Depth Anything V2 repository identifies the Small model as Apache-2.0. The Base, Large and Giant checkpoints are CC BY-NC 4.0. This makes Small the safe member of that model family to evaluate for commercial distribution, subject to exact checkpoint/card and notice review.

Sources: [Depth Anything V2 repository](https://github.com/DepthAnything/Depth-Anything-V2) · [model license](https://github.com/DepthAnything/Depth-Anything-V2/blob/main/LICENSE)

### Depth Anything 3 Small

The official DA3 repository was released in 2025 and presents a newer model family. The 2026 `depth-anything.cpp` project provides a C++17/ggml port with a flat C API for embedding into C/Rust. Its README says DA3 Small and Base weights are Apache-2.0, while large/giant variants are CC BY-NC 4.0; the C++ port itself is MIT. These terms are not inherited by all checkpoints automatically: verify each model card and hash before redistributing.

Sources: [Official Depth Anything 3](https://github.com/ByteDance-Seed/Depth-Anything-3) · [depth-anything.cpp](https://github.com/localai-org/depth-anything.cpp) · [port's license notes](https://github.com/localai-org/depth-anything.cpp#license)

### Model recommendation

Do not hardcode one model in the product architecture. Add a `DepthEstimator` interface. Benchmark DA2 Small ONNX against DA3 Small through the C API on the target hardware and image corpus. For the first MVP, ship deterministic image analysis and optional depth analysis. Do not hold the first Windows MVP hostage to model packaging or GPU-provider bugs.

No LLM, text model, generative model or cloud API is required. A small vision model is optional for image depth; the rest of the pipeline is deterministic computer vision and shader programming.

### Limits of single-image depth

Single-view depth estimation is not scene reconstruction. It can misjudge reflective surfaces, flat illustrations, typography, occlusion boundaries and low-texture areas. It cannot recover hidden pixels. Use confidence weighting, edge masks, user correction and conservative movement.

---

## 7. Music perception, audio features and visual ergonomics

### Use musical structure rather than only amplitude

A useful audiovisual design should respond to different temporal structures: sustained energy, rapid attacks, rhythmic repetition and spectral change. Crossmodal research indicates that sound–visual associations can be influenced by shared temporal/spatial structure and context, not just fixed universal mappings.

Source: [Spence & Di Stefano, “Mid-Level Audiovisual Crossmodal Correspondences: A Narrative Review” (2025)](https://pubmed.ncbi.nlm.nih.gov/41175881/)

### Onset detection

Spectral flux is an established onset feature: it measures positive changes in spectral magnitude over time. It does not, by itself, identify a stable beat grid, tempo, instrument or source stem.

Source: [Bello et al., “A Tutorial on Onset Detection in Music Signals” (IEEE, 2005)](https://doi.org/10.1109/TSA.2005.851998)

### Frequency mappings

The proposed six zones are reasonable as artistic controls but should not be called Bark bands or stems. Use a perceptual weighting (Bark/ERB/log frequency) only as an internal aggregation option. Do not interpret 500 Hz–2.5 kHz as “vocals”: many sources share those frequencies.

### Visual comfort

WCAG 2.2 contains criteria concerning flashing content. The three-flashes-per-second criterion is a restrictive accessibility rule, not a guarantee that a visual effect below that frequency will be comfortable for everyone. Avoid flashes entirely as a default, clamp visual impulse amplitude, avoid rapid jitter, provide reduced-motion / pause controls and test with users.

Sources: [WCAG 2.2](https://www.w3.org/TR/WCAG22/) · [Understanding Success Criterion 2.3.2](https://www.w3.org/WAI/WCAG22/Understanding/three-flashes.html)

The Gemini proposal's global `±6% luminance` is not a universal evidence-backed safety boundary. DiscoDesk should instead avoid screen-wide flashing and use localized, low-amplitude, slowly smoothed responses.

---

## 8. DSP configuration sanity check

At 48,000 samples/second:

| Window size | Window duration | Bin spacing | 50%-overlap hop |
|---:|---:|---:|---:|
| 1,024 | 21.33 ms | 46.875 Hz | 10.67 ms |
| 2,048 | 42.67 ms | 23.4375 Hz | 21.33 ms |
| 4,096 | 85.33 ms | 11.71875 Hz | 42.67 ms |
| 8,192 | 170.67 ms | 5.859375 Hz | 85.33 ms |

This supports a two-path design: longer window for sub/low bass and short path or filters for transient response. A long analysis window has better low-frequency discrimination but poorer time localization; that trade-off must be managed rather than ignored.

---

## 9. Steam and commercial distribution

Steam's current Workshop API uses `ISteamUGC` methods such as `CreateItem`, `StartItemUpdate`, update metadata/content/preview methods and `SubmitItemUpdate`. Use the official implementation guide and API reference; do not build new implementation around deprecated legacy Workshop upload functions.

Sources: [ISteamUGC API](https://partner.steamgames.com/doc/api/ISteamUGC) · [Steam Workshop implementation guide](https://partner.steamgames.com/doc/features/workshop/implementation)

Commercially important checks:

- Keep Steam optional for local playback.
- Ensure all bundled dependencies and model weights are commercially distributable.
- Publish exact third-party notices and model attribution.
- Require explicit user action and rights confirmation before uploading personal artwork.
- Validate all downloaded Workshop packages as untrusted archives.
- Confirm Rust Steamworks bindings support the required `ISteamUGC` functions; otherwise isolate a small audited FFI module.
- Consider a free demo / capability test before the paid version so users can verify OS capture and wallpaper-host support before purchase.

No price or sales forecast is supportable from this technical research alone. Validate demand with a polished prototype, user tests and competitor reviews before choosing a Steam price model.

---

## 10. Recommended product strategy

### Target user groups

- Music listeners who want a desktop that gently follows their listening.
- Producers who want a subtle, non-obstructive studio ambience while working in a DAW.
- Gamers who want an expressive desktop but need automatic pause in fullscreen games.
- Wallpaper creators who want to turn existing still images into shareable reactive packages.

### Product pillars

1. **One-click transformation:** automatic image analysis and recipe generation.
2. **Tasteful motion:** the artwork breathes instead of being covered by a spectrum widget.
3. **Producer-aware DSP:** low-end weight, transient punch, sustained warmth, high detail and optional tempo confidence.
4. **Comfort and control:** minimal default motion, separate controls for each effect and reliable reduce-motion mode.
5. **Local performance:** cached one-shot model inference, native renderer, pause-on-fullscreen and no cloud dependency.
6. **Creator sharing:** portable `.ddpkg` format and Steam Workshop after the core engine is reliable.

### First release scope

Focus first on Windows with system-output audio capture, one monitor, still image, deterministic analysis, a native shader renderer and a Studio control center. Then add model-based depth, Windows per-app capture where supported, multi-monitor handling and the remaining platforms. This is a better way to achieve reliable quality than trying to ship every OS and every AI feature at once.

---

## 11. Source register

The sources below are the primary references used for this audit. Project READMEs describe their own current implementation; they are not independent performance audits.

### Existing projects

- [Lively Wallpaper](https://github.com/lively-community/lively)
- [Lively audio visualizer wallpaper](https://github.com/rocksdanister/audio-visualizer-wallpaper)
- [Lively depth-map wallpaper](https://github.com/rocksdanister/depthmap-wallpaper)
- [KOpenWallpaper](https://github.com/hteariH/kopenwalpaper)
- [Shader Wallpaper KDE](https://github.com/y4my4my4m/kde-shader-wallpaper)
- [DcentWallpapers](https://github.com/dcenhance/dcent-wallpaper-linux)
- [projectM](https://github.com/projectM-visualizer/projectm)
- [OpenDance](https://github.com/WodsonKun/OpenDance)
- [Open Wallpaper Engine](https://openwallpaperengine.app/)
- [WebDesk](https://github.com/NAME0x0/WebDesk)
- [Wallpaper Engine](https://store.steampowered.com/app/431960/Wallpaper_Engine/)
- [Animate Desktop](https://store.steampowered.com/app/5255400/Animate_Desktop/)
- [Music Visualizer Engine PC Live Wallpaper](https://store.steampowered.com/app/1204920/Music%20Visualizer%20Engine%20PC%20Live%20Wallpaper/)

### Platform and graphics APIs

- [Microsoft Application Loopback Audio sample](https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/)
- [Microsoft process-loopback parameters](https://learn.microsoft.com/en-us/windows/win32/api/audioclientactivationparams/ns-audioclientactivationparams-audioclient_process_loopback_params)
- [Apple Core Audio process taps](https://developer.apple.com/documentation/coreaudio/capturing-system-audio-with-core-audio-taps)
- [Apple WindowLevel.desktop](https://developer.apple.com/documentation/swiftui/windowlevel/desktop)
- [PipeWire streams](https://docs.pipewire.org/page_streams.html)
- [PipeWire audio-capture example](https://docs.pipewire.org/audio-capture_8c-example.html)
- [wlr-layer-shell protocol](https://github.com/wlr-protocols/wlr-layer-shell-unstable-v1)
- [wgpu backend documentation](https://docs.rs/wgpu/latest/wgpu/struct.Backends.html)
- [Tauri architecture](https://v2.tauri.app/concept/architecture/)

### AI, DSP and accessibility

- [Depth Anything V2](https://github.com/DepthAnything/Depth-Anything-V2)
- [Depth Anything 3](https://github.com/ByteDance-Seed/Depth-Anything-3)
- [depth-anything.cpp](https://github.com/localai-org/depth-anything.cpp)
- [Spence & Di Stefano (2025) crossmodal review](https://pubmed.ncbi.nlm.nih.gov/41175881/)
- [Bello et al. (2005) onset detection tutorial](https://doi.org/10.1109/TSA.2005.851998)
- [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- [WCAG 2.3.2 flashing guidance](https://www.w3.org/WAI/WCAG22/Understanding/three-flashes.html)

### Steam

- [ISteamUGC API](https://partner.steamgames.com/doc/api/ISteamUGC)
- [Steam Workshop implementation guide](https://partner.steamgames.com/doc/features/workshop/implementation)

---

## 12. Desktop automation and OS integration research

### Windows wallpaper identity and observation

Microsoft's IDesktopWallpaper COM interface exposes monitor enumeration and per-monitor wallpaper query. GetWallpaper(NULL) can return S_FALSE if different monitors use different wallpapers or a slideshow is running, so query every monitor deliberately.

Sources:
- https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-idesktopwallpaper
- https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-idesktopwallpaper-getwallpaper
- https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-idesktopwallpaper-getmonitordevicepathcount
- https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-settingchange

WM_SETTINGCHANGE is a system-setting broadcast for some changes and does not always tell an application precisely which setting changed. A Microsoft Q&A response dated 11 February 2026 reports that some Windows 11 Settings wallpaper changes did not broadcast WM_SETTINGCHANGE and suggests monitoring HKCU\\Control Panel\\Desktop\\Wallpaper with RegNotifyChangeKeyValue, then querying IDesktopWallpaper. This is support-forum evidence, not a stable API guarantee.

Conclusion: use OS change hints plus low-frequency authoritative per-monitor reconciliation. Do not depend exclusively on WM_SETTINGCHANGE, registry watching or rapid polling.

### Windows wand feasibility

- https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles documents WS_EX_NOACTIVATE; a top-level window with that style does not become foreground on click. It is not in normal keyboard navigation by default, so keyboard/tray/screen-reader alternatives are necessary.
- https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features documents layered windows and alpha composition.
- https://learn.microsoft.com/en-us/windows/apps/design/widgets explains that the Windows Widgets Board is a separate flyout opened from taskbar/Win+W/swipe, not a persistent arbitrary desktop button.
- https://learn.microsoft.com/en-us/windows/apps/develop/widgets/widget-providers documents widget provider capabilities.

A small no-activate layered overlay is a prototype direction, not a verified product implementation. Test z-order, Explorer, click routing, icons, taskbar, fullscreen, DPI and accessibility before shipping. Never use a full-screen click-capturing overlay.

### macOS

- https://developer.apple.com/documentation/appkit/nsworkspace/desktopimageurl%28for%3A%29 returns the desktop image URL for a specified NSScreen and must be called on the main thread.
- https://developer.apple.com/documentation/appkit/nsworkspace documents desktop-image management.
- https://developer.apple.com/design/human-interface-guidelines/the-menu-bar documents the native menu-bar surface for persistent quick actions.

The reviewed public sources establish a per-screen wallpaper query but not a universal wallpaper-change notification for all target versions. Use conservative reconciliation unless a public supported event is independently verified. Menu-bar quick action is the safe first implementation.

### Linux

- https://github.com/KDE/plasma-desktop/blob/master/design/wallpaper describes wallpaper plugins as components painting desktop containments.
- https://develop.kde.org/docs/plasma/scripting/api/ documents per-desktop wallpaper configuration and screen-specific desktop objects.
- https://develop.kde.org/docs/plasma/ distinguishes desktop widgets/Plasmoids from wallpaper plugins.

KDE offers native extension points for a desktop widget/wallpaper provider. Other desktop environments require individual adapters. A generic Wayland feature claim is not justified by one protocol or compositor.

### Native local notifications

- https://learn.microsoft.com/en-us/windows/apps/develop/notifications/ recommends Windows App SDK AppNotificationManager for new WinUI/WPF/WinForms/unpackaged Win32 desktop applications. Local notifications are distinct from cloud push.
- https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/toast-notifications-overview documents actionable local notifications and constraints.
- https://developer.apple.com/documentation/usernotifications is Apple's notification framework.
- https://specifications.freedesktop.org/notification/latest-single/ is the freedesktop Desktop Notifications D-Bus specification, version 1.3.

Core DiscoDesk notifications can be entirely local. They still need user permission, quiet hours, deduplication, severity and rate limits.

## 13. Visual design and comfort research

Primary sources:
- Fluent 2 motion: https://fluent2.microsoft.design/motion
- Fluent 2 materials: https://fluent2.microsoft.design/material
- Fluent 2 color: https://fluent2.microsoft.design/color
- Fluent 2 typography: https://fluent2.microsoft.design/typography
- Apple HIG motion: https://developer.apple.com/design/human-interface-guidelines/motion
- WCAG 2.2: https://www.w3.org/TR/WCAG22/

Design conclusions:
1. Neutral base surfaces, comfortable light/dark themes, readable hierarchy and a restrained accent.
2. Mica for appropriate opaque Windows 11 surfaces; Acrylic for transient surfaces; provide platform fallbacks.
3. Purposeful brief animation, no decorative constant movement.
4. Permanent pause/stop and reduced-motion behavior that reduces zoom, parallax, depth, particles and UI transitions.
5. Keyboard, focus, screen-reader, scaling, contrast and high-contrast support.
6. Wallpaper colors may affect previews and minor accents, not text contrast.
7. Advanced DSP language should remain behind advanced mode.

WCAG flash criteria are minimum web-content conformance references, not proof of universal visual comfort.

## 14. Coding-agent decision: Claude Code vs OpenCode

Research date: 27 November 2026.

### Official capabilities

OpenCode documents project agents, per-agent model settings, skills, MCP, plugins, permissions, worktrees, multiple sessions, background subagents and local runtimes:
- https://opencode.ai/docs/agents
- https://opencode.ai/docs/skills
- https://opencode.ai/docs/permissions
- https://opencode.ai/docs/tools
- https://opencode.ai/docs/providers
- https://opencode.ai/docs/models

Claude Code documents custom subagents, skills, hooks, MCP, plan workflows, settings and parallel work:
- https://code.claude.com/docs/en/overview
- https://code.claude.com/docs/en/sub-agents
- https://code.claude.com/docs/en/skills
- https://code.claude.com/docs/en/hooks
- https://code.claude.com/docs/en/settings

For this repository, OpenCode fits the existing multi-provider/local-model/MCP structure and already has an agent/skill system. Claude Code is strong and integrated for Anthropic-first work, but migrating would add a second repository configuration layer. The decision is specific to DiscoDesk, not a claim that OpenCode is universally superior.

### DeepSeek current official details

Current model IDs and prices are documented at https://api-docs.deepseek.com/quick_start/pricing/ and checked on 27 November 2026:
- deepseek-flash serves DeepSeek-V4.1-Flash-1017.
- deepseek-v4-pro serves DeepSeek-V4-Pro-0813.
- Both advertise 1M context and tool calls; Flash supports vision and Pro does not.
- Off-peak per million tokens: Flash $0.15 input miss, $0.003 cache hit, $0.60 output; Pro $0.66, $0.022, $1.98.
- Peak rates double during designated weekday windows.

Official OpenCode integration:
https://api-docs.deepseek.com/quick_start/agent_integrations/opencode/

Official Claude Code integration:
https://api-docs.deepseek.com/quick_start/agent_integrations/claude_code/

DeepSeek maps Claude model aliases to its own models when using the Anthropic-compatible endpoint. A Claude alias routed to DeepSeek is not Claude Opus.

OpenCode issue #55684 reports tool-call reliability complaints for the built-in DeepSeek OpenAI-compatible route and documents a custom Anthropic-compatible provider workaround:
https://github.com/anomalyco/opencode/issues/55684
Treat it as a concrete smoke-test risk, not proof all requests fail.

### Claude subscription

Current official pricing:
https://claude.com/pricing
https://support.claude.com/en/articles/11049741-what-is-the-max-plan
https://support.claude.com/en/articles/14552983-models-usage-and-limits-in-claude-code

The individual page lists Pro at $20 monthly (annual discount available), Max at $100 for 5x Pro usage, and Ultra at $200 for 10x Pro usage. Actual usage limits vary with model, effort, files and conversation length. Claude Code subscription usage and API-key usage are separate billing routes. Do not assume subscription auth is available to third-party harnesses unless official provider support confirms it.

### Real developer reports (anecdotal)

- A March 2026 same-model refactoring comparison reported OpenCode/Sonnet ahead on one task and OpenCode/Codex ahead on cost in a 10k-line app. One user experiment, not a controlled general benchmark: https://www.reddit.com/r/opencodeCLI/comments/1s3mi6l/opencode_vs_claudecode_as_agentic_harness_test/
- A March 2026 OpenCode user valued control/visibility and fine-tunable subagents but reported weaker default planning and a snapshot storage surprise: https://www.reddit.com/r/opencodeCLI/comments/1ryl1l6/my_brief_and_bad_experience_with_claude_code/
- A November 2026 Claude Code user warned that automatic edits/reviews can make code more complex and harder to understand: https://www.reddit.com/r/ClaudeCode/comments/1xzdi6c/claude_code_is_design_to_make_you_dump/
- A November 2026 plan/execution workflow post describes Claude Code for plan-only and OpenCode for implementation, but comes from a tool builder and is not independent benchmarking: https://www.reddit.com/r/ClaudeCode/comments/1xom28d/claude_code_as_the_planner_opencode_as_the/

These reports are biased and conflict. Their useful message is that workflow visibility, task decomposition, tests and human review matter as much as model choice.

### Recommendation

Keep OpenCode as the primary harness. Assign a frontier reasoning model to the coordinator/architect and independent review; use DeepSeek Flash for bounded implementation and DeepSeek Pro for harder coding/reasoning; use LM Studio for low-risk local assistance. Claude Code can be a separate plan/review environment if the user later buys a subscription, but do not run it and OpenCode concurrently as source writers on the same worktree.

Do not pin unverified model IDs in the repository. Connect providers and choose exact models from OpenCode /models. Measure actual acceptance quality and rework before buying a higher subscription.

### Controlled benchmark

Use the same clean commit, task, context and acceptance criteria in isolated worktrees. Test:
1. Explain a seeded bug without editing.
2. Implement one DSP module plus deterministic tests.
3. Implement one renderer change plus preview evidence.
4. Diagnose a deliberate cross-module regression.
5. Review a native lifecycle/security patch read-only.

Record accepted criteria, actual test results, regressions, manual corrections, retries, time, changed lines, input/cache/output tokens and billed cost. The winner is the configuration that produces verified correct work with less human rework—not the one that emits more code.

## 15. Research source register additions

Platform:
- Microsoft IDesktopWallpaper, WM_SETTINGCHANGE, window styles, layered windows and App Notifications (links in section 12).
- Apple NSWorkspace, UserNotifications and Human Interface Guidelines (links in section 12).
- KDE Plasma wallpaper/widgets and freedesktop notifications (links in section 12).

Design:
- Fluent 2 Motion/Material/Color/Typography.
- Apple HIG Motion.
- WCAG 2.2.

AI coding workflow:
- OpenCode official docs.
- Claude Code official docs.
- DeepSeek official pricing/integration docs.
- OpenCode issue #55684.
- Reddit reports explicitly labeled as anecdotal in section 14.


---

## 16. Gate 0 dependency and toolchain register (verified)

**Verified:** 27 November 2026. **Method:** primary sources only � the crates.io version
API (`https://crates.io/api/v1/crates/<crate>`), the npm registry
(`https://registry.npmjs.org/<package>`), the official Rust release channel manifests
(`https://static.rust-lang.org/dist/channel-rust-*.toml`), the GitHub releases API and
Microsoft Learn. Community posts were not used as evidence.

This register exists because `AGENTS.md` requires a source URL, access date, version and
SPDX licence identifier before a dependency is added, and because a version chosen from
memory can be stale, can have raised its MSRV, or can carry a licence change.

### 16.1 Toolchain

| Item | Verified value | Source |
|---|---|---|
| Latest stable Rust | **1.100.0** (`0ef1bc386 2026-11-09`, channel date 2026-11-12) | `channel-rust-stable.toml` |
| Previous stable | 1.99.1 (point release of 1.99.0, 2026-10-01) | `channel-rust-1.99.1.toml` |
| Two-previous stable | 1.98.0 (2026-08-20), 1.98.1 point release | `channel-rust-1.98.0.toml` |
| Bundled cargo / clippy / rustfmt | cargo 0.101.0, clippy 0.1.100, rustfmt 1.11.0 | `channel-rust-stable.toml` |
| Current edition | 2024 (no 2027 edition exists) | Rust Edition Guide |
| Edition 2024 requires | Rust 1.85.0+ | Rust 1.85.0 release notes |
| Windows MSVC target tier | `x86_64-pc-windows-msvc` is Tier 1 with host tools | `platform-support.html` |
| Minimum Visual Studio for Rust | VS 2017, "not actively tested in CI"; latest VS recommended | `platform-support/windows-msvc.html` |

**Decision:** pin `1.100.0`. It is the current stable and matches what GitHub's
`windows-2025` runner image ships, so local and CI agree without a download.

**Caveat recorded as fact:** Rust's own MSVC page states VS 2017 is the nominal minimum
but is *not actively tested in CI*, and recommends the latest Visual Studio. The 2019
Build Tools toolset (14.29) on this host is therefore not a platform to rely on; the
2026 toolset (14.51) is.

### 16.2 Rust crates (Gate 0 set)

| Crate | Pinned version | SPDX licence | MSRV | Notes |
|---|---|---|---|---|
| `serde` | 1.0.229 | MIT OR Apache-2.0 | 1.56 | |
| `serde_json` | 1.0.154 | MIT OR Apache-2.0 | 1.71 | |
| `thiserror` | 2.0.21 | MIT OR Apache-2.0 | 1.77 | |
| `tracing` | 0.1.44 | **MIT only** | 1.65 | no Apache fallback |
| `tracing-subscriber` | 0.3.23 | **MIT only** | 1.65 | =0.3.20 required by RUSTSEC-2025-0055 |
| `tracing-appender` | 0.2.5 | **MIT only** | 1.63 | rotating file sink |
| `clap` | 4.6.10 | MIT OR Apache-2.0 | 1.74 | derive feature |
| `windows` | 0.62.2 | MIT OR Apache-2.0 | 1.82 | latest *published*; master 0.101.0 is unpublished |
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 | 1.71 | not used at Gate 0 |

Rejected or deferred, with reason:

| Crate | Reason |
|---|---|
| `interprocess` 2.4.10 (0BSD OR Apache-2.0) | Local-socket types are runtime-dispatched enums, not OS primitives; async is Tokio-only; first-party UDS removed in 2.0.0. Hand-roll the named pipe instead. |
| `once_cell` | Superseded by `std::sync::LazyLock` / `OnceLock` (stable since 1.80). |
| `parking_lot` 0.13.0 | MSRV 1.95 and an avoidable dependency; `std::sync::Mutex` suffices at Gate 0. |
| `tokio` | No async runtime is needed at Gate 0. Deferred to the gate that needs it (audio/GPU). |
| `postcard` / `bincode` | The frame codec is hand-rolled; no serialisation framework needed for the framing layer. |
| `uuid` / `sha2` | No content hashing or ID generation at Gate 0. `sha2` 0.11 also raised MSRV to 1.88. |

### 16.3 Studio / frontend (verified, not yet added to the workspace)

| Package | Latest verified | SPDX licence |
|---|---|---|
| `tauri` (crate) | 2.12.7 | Apache-2.0 OR MIT |
| `tauri-build` | 2.7.6 | Apache-2.0 OR MIT |
| `wry` | 0.58.2 | Apache-2.0 OR MIT |
| `@tauri-apps/cli` | 2.12.6 | Apache-2.0 OR MIT |
| `@tauri-apps/api` | 2.12.5 | Apache-2.0 OR MIT |
| `vite` | 8.4.0 | MIT |
| `@vitejs/plugin-react` | 6.3.0 | MIT |
| `react` / `react-dom` | 19.3.1 | MIT |
| `typescript` | 7.0.2 | Apache-2.0 |

**Effective Studio MSRV is 1.92**, not the `tauri` crate's declared 1.90, because `wry`
declares 1.92 and the Tauri workspace declares 1.92. 1.100.0 satisfies it.

**Node:** Vite 8 requires `^20.19.0 || >=22.12.0`. Tauri names no minimum beyond "LTS".
Recommend Node 24 LTS.

**TypeScript is now on the 7.x line** (npm `latest` = 7.0.2). That is a behavioural
change from 5.x, not a patch bump; it must be validated before pinning.

**Bun is not used as the package manager.** Bun is MIT, but it statically links
JavaScriptCore/WebKit, which is **LGPL-2**. Redistributing Bun would carry copyleft
obligations. It is a personal developer tool only. Use npm or pnpm for the Studio build.

### 16.4 Runtime components that are NOT open source

This is a licence-boundary fact, not a code-dependency fact, and must not be glossed over
in any "the app is MIT/Apache" statement:

- **Microsoft Edge WebView2 Runtime** � proprietary Microsoft EULA, required at runtime on
  the user's machine (or shipped as the Fixed Version, which adds roughly 250 MB).
- **MSVC C/C++ runtime** � `static_vcruntime` statically links it; redistributable under
  Microsoft's Visual C++ redistributable terms.
- **`WebView2Loader.dll`** � an architecture-specific native DLL that must ship, or be
  statically linked.

The Rust and npm sources are permissive. The shipped Studio **binary** includes Microsoft
components under separate terms.

### 16.5 Security advisories checked

RustSec advisory database: <https://github.com/RustSec/advisory-db>. Verified as fixed by
the pinned (or minimum) version:

| Advisory | Affected | Fixed in | Our status |
|---|---|---|---|
| RUSTSEC-2025-0055 / CVE-2025-58160 � ANSI-escape injection via logged user input | `tracing-subscriber < 0.3.20` | 0.3.20 | Pinned 0.3.23 |
| RUSTSEC-2026-0384 / CVE-2026-115567 � race in `tokio::time::Interval` | tokio 1.54.0 | 1.54.1 | tokio not used |
| RUSTSEC-2026-0007 / CVE-2026-25541 � integer overflow in `BytesMut::reserve` | bytes < 1.11.1 | 1.11.1 | bytes not used |
| RUSTSEC-2026-0190 � unsoundness in `anyhow::Error::downcast_mut` | anyhow < 1.0.103 | 1.0.103 | anyhow not used |
| CVE-2026-42184 / GHSA-7gmj-67g7-phm9 � Tauri origin confusion allowing remote pages to invoke local-only IPC | tauri = 2.11.0 | 2.11.1 | 2.12.7 when the Studio is added |

`RUSTSEC-2025-0055` is directly relevant to DiscoDesk: it is a log-poisoning class bug,
which is exactly why `docs/ADR/0007` forbids logging unredacted user-controlled strings.
Transitive `rand`/`zerocopy` advisories must be confirmed empirically with
`cargo deny check advisories` once `Cargo.lock` exists; they are not asserted here.

### 16.6 CI actions

| Action | Verified tag | Note |
|---|---|---|
| `actions/checkout` | v7.0.1 | not v4 |
| `dtolnay/rust-toolchain` | `v1` (floating) | pin via the `toolchain:` input |
| `Swatinem/rust-cache` | v2.9.3 | |
| `actions/upload-artifact` | v7.0.2 | |

`codecov/codecov-action` is at v7.1.1 and `actions/setup-node` at v7.1.0; neither is used
at Gate 0 (there is no coverage or Node build yet).

**Runner:** `windows-latest` currently resolves to Windows Server 2025 (build
10.0.26100, Windows SDK 10.0.26100.0). Two distinct Windows Server 2025 images exist �
one with Visual Studio 2022 17.14 and one with VS 2026 18.12 � so the workflow pins
`windows-2025` explicitly rather than relying on `-latest`.

**`EmbarkStudios/cargo-deny-action` cannot be used here:** it is a Docker container
action, and Docker actions do not run on Windows runners. CI installs `cargo-deny
0.22.1` with cargo and invokes it directly.

### 16.7 Unverified items

Stated explicitly so they are not mistaken for facts:

| Item | Status |
|---|---|
| Current WebView2 Evergreen stable version from Microsoft | **Unverified** � `edge.microsoft.com/updates/WebView2RuntimeAvailability.xml` returned HTTP 400 to every request variant attempted. The installed `158.0.4370.33` was read from the local registry, not from Microsoft's feed. |
| Minimum Visual Studio version Tauri v2 officially supports | **Unverified** � Tauri documents only "Microsoft C++ Build Tools" with the Desktop development with C++ workload, naming no version floor. |
| Windows SDK version required by Tauri/wry | **No version is pinned by either.** Recommendation: use 10.0.26100.0 and validate with an actual build. |
