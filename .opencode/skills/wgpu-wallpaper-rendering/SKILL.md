---
name: wgpu-wallpaper-rendering
description: Use when building the wgpu renderer, WGSL shaders, image/depth texture pipeline, parallax, glow, particles, frame pacing or GPU resource management.
---

# Wallpaper rendering workflow

Implement a normal preview window before attaching to the desktop. Keep the renderer independent of Tauri WebView and native window-host policy.

Start with a stable base-image texture and deterministic uniform data. Add effects incrementally: breathing, low-amplitude parallax, localized highlight response and sparse particles. Every effect needs an enable/disable switch, bounded amplitude and safe fallback when its mask or depth data is absent.

Preserve aspect ratio and color correctness. Clamp UVs and protect screen edges; avoid stretching, edge tearing, rapid luminance flashes and high-frequency jitter. Use depth confidence and edge masks conservatively. A single depth map is not a true 3D scene.

Reuse buffers, textures, bind groups and pipelines. Avoid per-frame CPU allocations and needless uploads. Test resize, multiple displays, adapter fallback and GPU device loss. Measure render time, frame pacing, CPU, GPU and VRAM at multiple resolutions; document hardware/backend/driver.
