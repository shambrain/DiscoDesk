---
name: image-intelligence
description: Use when building still-wallpaper import, local image analysis, depth estimation, saliency/highlight maps, palette extraction or the analysis cache.
---

# Image intelligence workflow

Treat analysis as an import-time job. Playback consumes validated cached assets and must never invoke model inference.

Implement safe image decode and deterministic maps first: dimensions/orientation, aspect-preserving resize, palette, luminance, local contrast, edges, focal regions and conservative particle-safe regions. Then add a replaceable DepthEstimator interface.

Model output is uncertain relative depth, not recovered 3D geometry. It cannot reveal hidden surfaces. Check tensor layout, input preprocessing, output size, confidence, NaN/infinity, cancellation, provider availability and memory use. Use deterministic 2D fallback when the model fails or confidence is poor.

Cache by source content hash plus analysis/model version and preprocessing parameters. Record exact model/checkpoint hash, source, license, runtime/provider and attribution. Verify commercial redistribution rights separately for code and weights before bundling. Test portraits, illustrations, text, landscapes, reflective and low-light images, alpha channels and corrupt input.
