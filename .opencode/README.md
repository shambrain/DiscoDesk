# OpenCode project setup

This repository uses the current OpenCode project-local file conventions.

## Components

- agents/ — Markdown agent profiles discovered by OpenCode.
- skills/<skill-id>/SKILL.md — on-demand reusable workflows.
- Root AGENTS.md — repository-wide instructions.
- Root opencode.jsonc — sets the project coordinator as the default agent.

## How to use

1. Open the repository root in OpenCode.
2. Let OpenCode load the project configuration, or restart the session if it was already open before these files were added.
3. Select discodesk-lead if the client did not select it automatically.
4. Ask it to inspect the repository and plan Gate 0 before implementing.
5. Mention a skill by ID, such as @audio-dsp, to load its workflow explicitly.

Agent files intentionally do not pin a model. They inherit the active session model so this project works with the user's configured local LM Studio models and can also be used with other providers.

Official references: https://opencode.ai/docs/agents/ and https://opencode.ai/docs/skills/
