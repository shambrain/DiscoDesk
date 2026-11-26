---
name: tauri-figcraft-ui
description: Use when designing DiscoDesk Studio, integrating FigCraft references, building Tauri v2 commands, React components or accessible control workflows.
---

# Tauri Studio and FigCraft workflow

Use FigCraft to define visual hierarchy, design tokens, component variants, navigation and interaction states. Treat FigCraft as a design source; do not add a runtime dependency on it.

Tauri v2 + React + TypeScript is the control center, not the wallpaper renderer or audio DSP process. Keep the Studio UI on-demand. Closing it must not stop the engine. Communicate through the documented, authenticated local IPC boundary.

Prioritize the one-click flow: choose image, analyse, Make It React, then automatically follow system audio. Put complex audio/visual controls in an advanced area. Show actual platform capabilities, permissions, errors and fallback states.

Design Home, Library, Vibe Studio, Audio and Performance/Diagnostics. Include pause, reduced-motion, intensity, app lock where supported, ignore list, fullscreen behavior, multi-display choices and clear local-processing status. Preserve keyboard navigation, visible focus, contrast, scalable layout and responsive density.

Test engine unavailable/reconnecting, cancelled analysis, unsupported capture, invalid settings and permission denial. Do not put high-rate audio frames or GPU rendering in WebView IPC.
