---
name: creator-presets-workshop
description: Use for DiscoDesk user vibe profiles, .ddpreset, safe preset import/export, creator metadata and Steam Workshop.
---

# Presets and Workshop workflow

1. Read ARCHITECTURE.md Addendum A8 and docs/DESKTOP_EXPERIENCE_SPEC.md.
2. Keep .ddpreset separate from .ddpkg. A vibe preset must work without its creator's wallpaper.
3. Use a versioned declarative schema: features, allowlisted effect outputs, curves, envelope parameters, comfort metadata, creator/version/tags and preview data.
4. No arbitrary executable, DLL, native plugin, unrestricted script or user WGSL in initial Workshop content.
5. Validate schema, ranges, effect IDs, hashes, archive paths, entries, decompressed size and compatibility.
6. Implement explicit migrations and round-trip tests. Reject unknown critical fields safely.
7. Require explicit rights confirmation before publishing wallpaper artwork in .ddpkg.
8. Local presets work without Steam/offline.
9. Include creator attribution, version/changelog, tags and remix flow.
10. Test hostile archives, path traversal, decompression bomb, hash mismatch, invalid effects and interrupted updates.
