# ADR-0007: Diagnostics are privacy-preserving by default

- **Status:** Accepted
- **Date:** 2026-11-27
- **Deciders:** discodesk-lead, systems-architect
- **Gate:** 0 (repository scaffold)

## Context

DiscoDesk captures the user's system audio and analyses the user's personal wallpaper.
Both are private by nature: a system mix can contain conversations, and a wallpaper can
contain faces, text, or a screenshot of something confidential.

`AGENTS.md` states plainly: never log raw PCM, API keys, private paths or user image
contents. `OPENCODE_MASTER_DIRECTIVE.md` §8 requires rotating local logs that are
privacy-preserving. `ARCHITECTURE.md` §11 requires that all state transitions are logged
to a rotating local diagnostic log **without logging PCM or image contents**.

The tension is that a diagnostic log is the primary tool for diagnosing the exact
failures this product will produce — device loss, Explorer restart, permission denial,
GPU device loss — so it must be genuinely useful, not empty.

A second tension: the developer experience wants logs on by default during development,
while a shipped product must not write files the user did not ask for.

## Options considered

1. **Structured events, path redaction, no raw media, and file logging disabled unless
   explicitly enabled (chosen).**
2. **Log everything, including PCM, for debugging.** Rejected outright: it writes the
   user's audio to disk by default, which `AGENTS.md` forbids and which is a privacy
   incident, not just a bad default.
3. **Log human-readable strings only.** Rejected: string logs cannot be counted,
   deduplicated or asserted on in tests, and they invite embedding raw values.
4. **Always write a rotating file.** Rejected: writes to the user's disk without
   consent, and makes CI and tests dirty.

## Decision

1. **Diagnostics are structured events, not strings.** Each event is a typed enum
   variant with typed fields, so it can be counted, deduplicated and asserted on in
   tests.
2. **Raw PCM and raw image bytes are never logged**, at any log level, in any build.
   There is no code path that can emit them.
3. **Paths are redacted.** A filesystem path is never logged verbatim. What is logged is
   a non-identifying descriptor: the file name and extension without the directory, or a
   hash of the path. The redaction helper is unit-tested against absolute paths,
   UNC paths, user profile paths and paths containing non-ASCII characters.
4. **File logging is off unless explicitly enabled.** The diagnostics sink writes to the
   console by default. A rotating local file sink exists and is opt-in via an explicit
   flag or setting, so a user who wants a support bundle can produce one and a user who
   does not want one gets no files. Tests and CI never write log files.
5. **The file sink is bounded and rotating** — a maximum size per file and a bounded
   number of retained files, so a long-running engine cannot fill a disk.
6. **Events carry no user-content payload.** Where a user-visible object must be
   identified (a wallpaper, a preset), a stable non-reversible identifier is used
   instead of the content.
7. **A capability or permission failure is logged as an event**, not as an error
   containing a path or an OS message that embeds one.

## Consequences

- Easier: tests can assert on event counts and kinds; a crash report can be attached to
  an issue without leaking personal content.
- Easier: the log can be shipped as a support bundle without a redaction pass.
- Harder: diagnosing a problem that genuinely depends on a specific file path requires
  the user to opt into file logging and share it deliberately.
- Harder: every call site must choose an event kind rather than writing a string, which
  is more work up front.
- Now forbidden: logging audio samples or image bytes; logging an absolute path
  verbatim; enabling file logging by default; an unbounded log directory.

## Verification

- Gate 0, automated: a unit test asserts that the path-redaction helper removes the
  directory component from absolute paths, UNC paths and paths under the user profile.
- Gate 0, automated: a test asserts that serialised diagnostic events contain no path
  separator and no absolute-path-shaped substring.
- Gate 0, automated: a test asserts that no file is created by the default
  configuration when events are emitted.
- Gate 0, automated: a test asserts the rotating sink keeps the file count within its
  configured bound.

## References

- `AGENTS.md` — logging rules.
- `OPENCODE_MASTER_DIRECTIVE.md` §8.
- `ARCHITECTURE.md` §11 — state transitions are logged, without PCM or image contents.
