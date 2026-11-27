# ADR-0006: M2 ambient features are deferred until M1 stabilisation

- **Status:** Accepted
- **Date:** 2026-11-27
- **Deciders:** discodesk-lead, systems-architect
- **Gate:** 0 (repository scaffold)

## Context

`ARCHITECTURE.md` Addendum A specifies the Ambient Desktop Experience: a per-monitor
wallpaper observer, content-hash cache reuse, opt-in automatic reaction, a native
Magic Wand, local notifications and creator presets. `docs/DESKTOP_EXPERIENCE_SPEC.md`
refines it. These are fully specified product requirements and **must not be removed or
weakened**.

They are, however, all built on foundations that do not exist yet: a working audio
capture path, a working renderer, a working desktop host, an analysis cache with
correct invalidation, a stable IPC contract and a tested engine lifecycle.

`OPENCODE_MASTER_DIRECTIVE.md` §17 requires M1 stabilisation (Gates 0–6 plus a beta
soak-test report) before M2 work begins, and `AGENTS.md` states that the Magic Wand and
the wallpaper watcher are deferred until the Windows MVP, cache, IPC and host lifecycle
are stable.

The risk this ADR addresses: implementing a desktop observer, a no-activate overlay and
an automatic-reaction policy while the engine's own lifecycle is still unstable
produces a system where every failure has three plausible causes, and where a crash
in the observer can take the wallpaper down.

## Options considered

1. **Implement the observer and reaction policy as soon as the IPC contract exists
   (chosen: deferred).**
2. **Build the observer first, because it is "just" a file-watch.** Rejected: the
   observer is not a file-watch. It needs per-monitor authoritative state, debouncing,
   source stabilisation, content hashing, cache invalidation and a policy engine that
   respects manual pause. Every one of those depends on unproven machinery.
3. **Implement the Magic Wand early as a visible demo.** Rejected: it is a native
   no-activate overlay whose z-order, hit-testing and DPI behaviour must be guaranteed
   against a desktop host that does not exist yet. `ARCHITECTURE.md` A5 explicitly
   allows shipping the tray/keyboard action instead if that cannot be guaranteed.
4. **Remove the M2 requirements to simplify the roadmap.** Rejected outright: the
   requirements are part of the product and are preserved intact in the specifications.
   Deferral is not removal.

## Decision

1. **Gate 0 implements none of the M2 features.** No wallpaper observer, no content-hash
   cache reuse, no automatic-reaction policy, no Magic Wand, no notification service, no
   `.ddpreset` authoring, no Workshop integration.
2. **The requirements are preserved.** `ARCHITECTURE.md` Addendum A,
   `docs/DESKTOP_EXPERIENCE_SPEC.md` and the M2/M3 checklists in
   `docs/IMPLEMENTATION_STATE.md` remain the specification of record. Deferral changes
   *when*, never *what*.
3. **The one M2 concern that Gate 0 must respect is the separation of concerns.** The
   observer's future job is to emit a bounded change hint, not to own analysis,
   rendering or policy. The Gate 0 crate boundaries are chosen so that an
   `dd-desktop-automation` crate can be added later without the engine core changing
   shape. Specifically: the engine owns authoritative state, and a future observer is a
   *hint source* that causes the engine to re-query platform state — it is never the
   authority.
4. **Gate 0 must not create empty placeholder crates** for the deferred features. A
   crate with no implementation and no tests would misrepresent the roadmap.
5. **The M2 implementation order is fixed** by `OPENCODE_MASTER_DIRECTIVE.md` §17 and is
   not re-litigated at each gate: observer, source validation and hashing, cache reuse,
   background analysis, reaction policy, Magic Wand prototype, accessible fallbacks,
   notification policy, then the beta test matrix.

## Consequences

- Easier: Gates 0–6 each have one failure surface, and the soak test has a bounded
  set of things that can go wrong.
- Easier: the desktop host's reliability (Explorer restart, focus, z-order, hit
  testing) is proven against a simple surface before an overlay is added to it.
- Harder: the visible "ambient" part of the product arrives later than a user might
  expect. Accepted deliberately: the wallpaper must not destabilise before it works.
- Harder: the analysis cache design must anticipate content-addressing even though
  Gate 0 does not implement it, so the cache key format is fixed early.
- Now forbidden: shipping a placeholder `dd-desktop-automation` crate marked complete;
  implementing the observer as "just a watcher"; a wand overlay without a tray/keyboard
  alternative; an automatic-reaction policy that can override a manual pause.

## Verification

- Gate 0: the repository contains no M2 implementation. The `docs/IMPLEMENTATION_STATE.md`
  M2/M3 checkboxes remain unchecked and are not marked complete on the basis of a
  specification.
- M1: the soak-test report in `docs/IMPLEMENTATION_STATE.md` lists the required
  scenarios before M2 work is scheduled.

## References

- `AGENTS.md` — Magic Wand and watcher deferral rule.
- `OPENCODE_MASTER_DIRECTIVE.md` §17, §18.
- `ARCHITECTURE.md` Addendum A (A1–A11), §11.
- `docs/DESKTOP_EXPERIENCE_SPEC.md`.
