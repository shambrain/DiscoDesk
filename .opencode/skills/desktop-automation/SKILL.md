---
name: desktop-automation
description: Use only after M1 Windows MVP stabilization for wallpaper observation, per-monitor auto-reaction, cache reuse, desktop wand or native notifications.
---

# Desktop automation workflow

1. Confirm M1 stabilization in docs/IMPLEMENTATION_STATE.md. If absent, stop implementation and provide a plan only.
2. Read ARCHITECTURE.md Addendum A and docs/DESKTOP_EXPERIENCE_SPEC.md.
3. Define a platform-neutral change-hint/snapshot contract before native code.
4. Use authoritative per-monitor snapshots; events are hints, not trusted paths.
5. Debounce/coalesce changes, wait for stable source, hash content and query versioned cache.
6. Queue one bounded/cancellable analysis job per unique image/version. Use deterministic fallback while inference is pending.
7. Check manual pause, global opt-out, image/display overrides, exclusions and reduced motion before auto-applying.
8. Keep wand tiny and native. Test z-order, hit region, focus, icons, taskbar, fullscreen, DPI and accessibility.
9. Provide tray/keyboard alternative. If overlay is unreliable, defer it.
10. Deduplicate/rate-limit local notifications; use status UI for routine changes.
11. Test slideshow, same image/different name, per-monitor changes, inaccessible paths, Explorer restart, display hotplug, cache corruption and shutdown.
12. Update implementation state only with real host evidence.
