---
name: native-platform-integration
description: Use when implementing or reviewing Windows, macOS or Linux audio capture, desktop wallpaper hosting, permissions, power state or display lifecycle adapters.
---

# Native platform integration workflow

Probe capabilities at runtime and return explicit supported/unsupported reasons. Keep each OS implementation behind a platform adapter and test its contract with mocks before native integration.

Windows: distinguish WASAPI system loopback from process-specific loopback and gate process capture by the documented OS build/API availability. Treat WorkerW/Progman attachment as shell integration that needs Explorer restart recovery and a safe fallback.

macOS: handle Core Audio process-tap availability, permission UX, desktop window level, Spaces and fullscreen transitions on real supported macOS versions.

Linux: separate PipeWire monitor capture, per-app stream capture, PulseAudio compatibility, KDE wallpaper hosting, X11 and compositor-specific Wayland paths. Never equate one compositor protocol with universal Wayland support.

Do not steal focus or mouse input. Handle multi-monitor changes, sleep/resume, display removal, audio device removal, permission denial and shell/session restarts. Do not mark platform support complete from a compile-only test.
