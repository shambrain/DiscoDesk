---
name: research-licensing
description: Use when verifying current platform APIs, OS build requirements, scientific claims, model checkpoints, third-party licenses or commercial redistribution rights.
---

# Research and licensing workflow

Prefer current primary evidence: official OS/vendor documentation, API references, maintained upstream repositories, model cards and license files. Use papers and standards bodies for scientific claims. Treat search snippets, forums and community discussions as discovery leads, not final evidence.

Record the exact URL, source type, checked date, product/API version or OS build, and the claim it supports. Distinguish facts, inference and unknowns. Check whether a model's code license and weight license differ. Check each third-party binary, runtime, model, font, image and sample separately.

For technical claims, verify API introduction/version gates, permission behavior, lifecycle constraints, deprecation status and fallback. For performance or accessibility claims, find the actual measurement or normative scope; do not turn a guideline into an unsupported safety guarantee.

Before a dependency or model is bundled, record license identifier, attribution/notice requirements, source URL, hash and redistribution decision. Flag ambiguity instead of assuming commercial permission.
