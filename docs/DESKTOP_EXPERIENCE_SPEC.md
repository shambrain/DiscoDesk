# DiscoDesk — Desktop Experience and Automation Specification

**Status:** Planned M2/M3 capability; not implemented.  
**Prerequisite:** Stable Windows MVP, native wallpaper host, image analysis/cache, engine IPC and lifecycle recovery.

## 1. Desired user experience

After initial setup, the user changes the OS wallpaper. DiscoDesk notices, identifies the new image, reuses cached analysis if it exists, otherwise performs bounded local analysis in the background, and applies the saved reaction profile when Auto React is enabled.

A tiny Magic Wand quick action is available when the platform can display it safely. It re-queries the current wallpaper and applies the best available recipe without opening Studio.

The engine observes the OS wallpaper. It does not set, replace, move, rename, re-encode or delete the source image.

## 2. Ownership

| Component | Responsibility |
|---|---|
| dd-platform | OS wallpaper query, change hints, monitor identity, shell integration |
| desktop automation | Debounce, reconciliation, auto-reaction policy, wand state |
| dd-analysis | Decode, masks, depth, palette and analysis jobs |
| cache | Content identity, versioning, LRU, invalidation |
| dd-core | Engine lifecycle, pause/overrides/per-monitor policy |
| dd-renderer | Visual recipe and per-monitor crop/transforms |
| notifications | Deduplication, quiet hours and native delivery |
| package | .ddpreset/.ddpkg validation and migration |
| Studio | Settings, preview, diagnostics, creator tools |

## 3. Event-to-reaction flow

1. Native platform adapter sends a bounded change hint.
2. Automation service debounces/coalesces hints.
3. Adapter re-queries authoritative current wallpaper for every display.
4. Source resolver verifies URI/path and waits for file stability.
5. Worker computes content hash.
6. Cache lookup uses source hash and analysis/model schema.
7. Cache hit loads the validated visual recipe.
8. Cache miss runs deterministic analysis and queues optional local depth inference.
9. Reaction policy checks manual pause, global opt-out, display/image override, app exclusions and reduced motion.
10. Renderer applies selected profile.
11. Wand status and notification state update; duplicate events are suppressed.

## 4. Cache identity and limits

Cache identity combines SHA-256 of image bytes, decoder schema, analysis schema, model/checkpoint hash and preprocessing version. Filenames/timestamps are hints only.

Cache contains source metadata, dimensions/orientation, optional depth/confidence, luminance/highlight/edge/particle-safe masks, palette, user protection regions, analyzer provenance and LRU accounting.

Use atomic writes, content-hash validation, bounded disk budget, active-entry pinning, corruption rejection, per-image reanalysis and selective invalidation. Model version change must not erase source art, preset or user brush. Cache errors fall back to reanalysis.

## 5. Magic Wand

A separate, small native quick-action surface:
- Default lower-right safe inset above taskbar/dock/panel.
- User can hide, reposition, resize, set opacity and select monitors.
- Only the visible button is hit-testable.
- Hide behind ordinary apps, fullscreen, lock screen and secure desktop.
- Never steal focus or obscure taskbar/dock/desktop icons.
- Provide tray/menu, keyboard and accessible equivalents.
- Do not create a full-screen transparent click-capturing window.

Windows prototype uses a tiny layered no-activate window with strict z-order and Explorer recovery. Mixed DPI, shell restart, desktop icon hits, app foreground, fullscreen and accessibility must pass before shipping. If unreliable, defer the visual overlay and ship tray/keyboard action.

macOS first uses menu-bar extra; desktop overlay is optional only after Spaces/Finder/focus testing. KDE Plasma can be investigated for native Plasmoid/wallpaper integration. Other Linux environments fall back to tray/menu unless a tested desktop widget exists.

Popover actions: Make React, Pause/Resume, Change Vibe, Exclude Wallpaper, Open Studio, Hide Wand. Status: Reacting, Analysing, Paused, Unsupported or Needs attention.

## 6. Auto reaction policy

Settings:
- Auto React: Off / Ask per new wallpaper / Automatically react.
- Default preset.
- Analyse new images in background.
- Allow local depth inference.
- Per-display enable/disable.
- Per-wallpaper profile.
- Respect manual pause.
- Fullscreen/app exclusions.
- Reduced-motion override.
- Notification preferences.

Precedence:
1. Engine stopped.
2. User manual pause.
3. Per-wallpaper/per-display opt-out.
4. Reduced-motion and comfort policy.
5. Auto reaction policy.
6. Default preset.

## 7. Creator preset formats

### .ddpreset

Portable vibe recipe independent of source wallpaper. Includes schema version, stable ID, name, author, version, description, tags, category, feature inputs, mapping curves, effect allowlist, amplitudes, attack/release, smoothing, comfort metadata, palette strategy and preview metadata.

Users can create, duplicate, rename, tag, favorite, export, import, migrate and delete. Quick mode adjusts overall intensity/depth/glow/particles/response; advanced mode maps sustained and transient features to effect channels.

### .ddpkg

Wallpaper package with artwork, optional analysis assets and wallpaper-specific profile. Artwork inclusion requires explicit distribution rights and user confirmation.

Initial Workshop packages are declarative data only. No arbitrary DLL, executable, native plugin, unrestricted script or user-supplied WGSL.

Validate schema, values, effect IDs, hashes, archive paths, entry counts, decompressed sizes, image dimensions and compatibility before applying. Local preset workflow works without Steam/offline.

## 8. Smart local notifications

Use classes:
- Silent: successful wallpaper/cache transition; update status only.
- Informational: analysis completed; optional, suppress slideshow noise.
- Action required: permission denied or source unavailable.
- Recoverable: model failed but fallback active; notify once.
- Critical: wallpaper host cannot recover; notify once with recovery action.

Deduplicate by event type and stable subject, apply per-event cooldown, quiet hours, global mute, per-monitor preferences and a maximum notification budget. Never toast every slideshow transition, cache hit, source transient or repeated error.

Use Windows App SDK AppNotificationManager for appropriate desktop apps, Apple UserNotifications on macOS and freedesktop notification D-Bus on Linux. No cloud push required. Provide in-app event history for suppressed notifications.

## 9. Studio UX

Calm, modern and comfortable:
- Neutral foundation, restrained accent, dark/light/system themes.
- Wallpaper palette may tint previews/small accents, never critical text contrast.
- Mica/Acrylic only where supported and appropriate; graceful solid fallback.
- Avoid constant animation, excessive glow, noisy gradients and dense jargon.
- Home: wallpaper, source, reaction state, pause.
- Library: images, presets, search, favorites and cache status.
- Vibe Studio: Motion, Depth, Light, Particles, Audio Response, Comfort, Advanced.
- Audio: real platform capture capability, source lock and ignore list.
- Performance: diagnostics and why playback is paused/throttled.
- Support keyboard, focus, screen readers, scaling, high contrast and reduced motion.

## 10. Test matrix

- Wallpaper source unavailable, replaced or mid-copy.
- Same image under different filenames.
- Same image on multiple displays.
- Per-monitor distinct wallpapers.
- Slideshow transitions/repeats.
- Dynamic/Spotlight path unavailable.
- Cache hit/miss/corrupt/invalidation.
- Model missing/failure/cancel.
- Manual pause vs auto reaction.
- Preset change during analysis.
- Display add/remove/rotation/DPI.
- Explorer/engine restart, logoff/shutdown.
- Wand hit-test, icons, taskbar, keyboard, accessibility.
- Fullscreen app/game/DAW.
- Notification permission denied, quiet hours, duplicate event.
- Hostile preset package and migration.

Do not mark M2 complete without real OS tests and recorded evidence.
