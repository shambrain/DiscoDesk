---
name: steam-workshop-packaging
description: Use when designing .ddpkg manifests, package security, migrations, Steamworks integration, Workshop upload/download or Steam distribution.
---

# Package and Steam workflow

Local playback and local package import/export must work without Steam. Treat Workshop as an optional integration.

Design .ddpkg as a versioned, inspectable archive with a manifest, hashes, asset inventory, analysis metadata, visual settings, creator metadata and migration rules. Use bounded decompression and reject path traversal, absolute paths, symlink escapes, duplicate paths, suspicious entry counts, oversized assets and malformed manifest data. Verify all content hashes before activation. Make install/update atomic with rollback.

Follow current Steamworks documentation and ISteamUGC APIs. Verify the selected Rust binding covers required methods and ownership semantics; isolate FFI if needed. Require explicit user intent and rights confirmation before publishing user artwork.

Test package round-trip, corrupt hashes, hostile archive samples, decompression limits, version migration, interrupted updates, downloaded content validation and Steam-disabled operation. Audit every bundled dependency/model/asset and ship required notices.
