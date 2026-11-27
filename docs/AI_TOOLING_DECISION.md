# DiscoDesk — AI Coding Tool and Model Strategy

**Decision:** OpenCode is the primary project harness. The model provider is selected by role; Claude Code is optional, not the primary repository orchestrator.

## 1. Harness versus model

- Harness: OpenCode or Claude Code; owns filesystem/terminal tools, permissions, sessions, MCP, agents, skills and workflow.
- Model: Claude Opus, DeepSeek V4 Pro/Flash or local model; performs reasoning and code generation.
- Provider/authentication: Anthropic API, Claude Code subscription login, DeepSeek API, LM Studio; controls access and billing.

A DeepSeek model called with a Claude model alias is still DeepSeek. Verify effective provider and model.

## 2. Why OpenCode for this repository

DiscoDesk requires a coordinator plus native Rust, audio DSP, GPU, image inference, Windows/macOS/Linux, Tauri UI, QA, licensing, packaging and Workshop specialists.

OpenCode documents project custom agents, per-agent model selection, SKILL.md workflows, MCP, plugins, permissions, sessions/worktrees and local LM Studio runtimes. The repository already contains OpenCode agents and skills. Claude Code has strong custom subagents, skills, hooks, MCP and parallel workflows, but moving primary harness would require maintaining a second configuration layer.

This is a project-specific decision, not a claim that OpenCode is universally better.

## 3. Current DiscoDesk model routing — OpenCode Go baseline

The repository uses two distinct routing layers. The native OpenCode primary agent coordinates the project; OMO Slim supplies its own specialist agents. The project lead and OMO Orchestrator are separate agents, intentionally assigned the same model.

| Agent | Provider/model ID | Variant | Configuration source |
|---|---|---|---|
| `discodesk-lead` | `opencode-go/minimax-m3` | Provider default | `.opencode/agents/discodesk-lead.md` |
| OMO Orchestrator | `opencode-go/minimax-m3` | `thinking` | `.opencode/oh-my-opencode-slim.json` |
| OMO Oracle | `opencode-go/qwen3.8-max` | `xhigh` | `.opencode/oh-my-opencode-slim.json` |
| OMO Explorer | `opencode-go/deepseek-v4.1-flash` | `high` | `.opencode/oh-my-opencode-slim.json` |
| OMO Designer | `opencode-go/kimi-k2.7-code` | Provider default | `.opencode/oh-my-opencode-slim.json` |
| OMO Fixer | `opencode-go/deepseek-v4.1-flash` | `high` | `.opencode/oh-my-opencode-slim.json` |
| OMO Observer | `opencode-go/mimo-v2.5` | Provider default | `.opencode/oh-my-opencode-slim.json` |

The project-local OMO file defines only the `opencode-go` preset's six model assignments.

### 3.1 Native DiscoDesk agent assignments

These are OpenCode project agents in `.opencode/agents/`, separate from OMO Slim's six managed agents. Every native agent now has an explicit model in its YAML frontmatter; this prevents silent inheritance of the parent session model.

| Native DiscoDesk agent | Model | Variant |
|---|---|---|
| `discodesk-lead` | `opencode-go/minimax-m3` | `thinking` |
| `systems-architect` | `opencode-go/qwen3.8-max` | `xhigh` |
| `rust-engineer` | `opencode-go/deepseek-v4.1-flash` | `high` |
| `audio-dsp-engineer` | `opencode-go/qwen3.8-max` | `xhigh` |
| `graphics-engineer` | `opencode-go/kimi-k2.7-code` | Provider default |
| `platform-engineer` | `opencode-go/qwen3.8-max` | `xhigh` |
| `image-intelligence-engineer` | `opencode-go/deepseek-v4.1-flash` | `high` |
| `tauri-ui-engineer` | `opencode-go/kimi-k2.7-code` | Provider default |
| `qa-performance-reviewer` | `opencode-go/qwen3.8-max` | `xhigh` |
| `research-licensing-auditor` | `opencode-go/qwen3.8-max` | `xhigh` |
| `steam-distribution-engineer` | `opencode-go/deepseek-v4.1-flash` | `high` |
| `desktop-experience-engineer` | `opencode-go/qwen3.8-max` | `xhigh` |
| `product-ux-designer` | `opencode-go/kimi-k2.7-code` | Provider default |

Routing rationale:
- MiniMax M3 coordinates the main project.
- Qwen3.8 Max is reserved for architecture, DSP design, native platform decisions, desktop automation, research and independent verification.
- DeepSeek V4.1 Flash handles bounded Rust, image and package implementation.
- Kimi K2.7 Code handles renderer and Studio/UX implementation.
- Kimi is left at its provider default rather than assigning an unverified reasoning variant.

The model IDs are provider-qualified using OpenCode Go's documented `opencode-go/<model-id>` convention. Variant selection uses OpenCode's documented `provider/model#variant` syntax. The current OMO Go preset already uses `thinking`, `xhigh` and `high` for the corresponding models. The catalogue can change; confirm the model still appears in `/models` before running autonomous work.

OpenCode's native subagent behavior is explicit: a subagent uses its configured model, or inherits its parent session model when none is set. This is the reason all 13 project agents now pin their own model.

The native project lead and OMO Orchestrator are distinct agents, although both use MiniMax M3. The six OMO roles remain configured in `.opencode/oh-my-opencode-slim.json`.

### Source references

- OpenCode agent model and variant frontmatter: https://opencode.ai/docs/agents/
- OpenCode Go current model IDs/catalogue: https://opencode.ai/docs/console/go
- OMO Slim v3.0.8 configuration reference: https://github.com/alvinunreal/oh-my-opencode-slim/blob/v3.0.8/docs/configuration.md
- OMO Slim project-local customization and precedence: https://github.com/alvinunreal/oh-my-opencode-slim/blob/v3.0.8/docs/project-local-customization.md
 It deliberately does not set the active preset, duplicate user-level skills/permissions/MCP settings, or contain credentials. The active preset remains a user choice; the current developer environment selected `opencode-go`. OMO Slim merges project configuration with the user configuration, and higher-precedence root agent or native host overrides can still take precedence. Do not add a top-level OMO `agents` override for a value that must vary by preset.

The `discodesk-lead` model is pinned in its native OpenCode agent file. The OMO Orchestrator is not the same session/agent as `discodesk-lead`; matching their model does not merge their identities or automatically route every lead action through OMO.

### Evidence and verification boundary

The local OMO Slim 3.0.8 diagnostic supplied for this project reported a valid configuration, the `opencode-go` preset applied, and matching effective model/variant state for all six OMO agents. The model names are also listed in the OpenCode Go model catalogue. This establishes configuration resolution, not a successful live inference or tool-call test for every agent. After changing models or creating a new session, verify `/models`, ping each agent, and complete the tool-call smoke test in Section 7.

OMO Slim v3.0.8 declares `opencode >=2.3.0` in its package engine requirement. Verify the installed OpenCode host with `opencode --version`; a passing standalone `doctor` invocation does not by itself prove host-version compatibility.

### Sources

- OpenCode custom agent model/frontmatter configuration: https://opencode.ai/docs/agents/
- OpenCode Go model catalogue: https://docs.opencode.ai/docs/go/
- OMO Slim v3.0.8 configuration and precedence: https://github.com/alvinunreal/oh-my-opencode-slim/blob/v3.0.8/docs/configuration.md
- OMO Slim v3.0.8 package engine requirement: https://github.com/alvinunreal/oh-my-opencode-slim/blob/v3.0.8/package.json

## 4. Current official DeepSeek API details

Official pricing checked 27 November 2026:
- deepseek-flash serves V4.1-Flash-1017.
- deepseek-v4-pro serves V4-Pro-0813.
- Both advertise 1M context and tool calls; Flash supports vision, Pro does not.
- Off-peak per million: Flash $0.15 input miss / $0.003 cache hit / $0.60 output. Pro $0.66 / $0.022 / $1.98.
- Peak prices double at DeepSeek's documented weekday windows.

Official sources:
- https://api-docs.deepseek.com/quick_start/pricing/
- https://api-docs.deepseek.com/quick_start/agent_integrations/opencode/
- https://api-docs.deepseek.com/quick_start/agent_integrations/claude_code/

## 5. Claude Code subscription

Official individual pricing page lists Pro at $20 monthly (annual discount available), Max at $100 for 5x Pro usage and Ultra at $200 for 10x Pro usage. Usage varies by model, effort, files and conversation size; it is not a fixed token allotment.

Sources:
- https://claude.com/pricing
- https://support.claude.com/en/articles/11049741-what-is-the-max-plan
- https://support.claude.com/en/articles/14552983-models-usage-and-limits-in-claude-code

Claude Code subscription usage and Anthropic API-key usage are different billing paths. Do not assume a Claude subscription login is reusable inside a third-party harness. Anthropic API credentials used in OpenCode should be budgeted as API usage unless official provider authentication says otherwise.

## 6. Recommended practical configuration

1. Keep OpenCode as the only primary source-writing harness and use the repository's pinned `discodesk-lead` model.
2. Keep the active OMO Slim preset on `opencode-go` for the model lineup in Section 3. The project override intentionally contains model assignments only; user-level authentication, MCP servers, skills and feature settings remain outside Git.
3. After changing the project model files, start a new OpenCode session or explicitly reselect the model for an existing session. OpenCode stores a session's selected model independently from the default agent.
4. Run `/models`, then ping all six OMO agents and the project lead. Complete the disposable-edit, shell/test and consecutive-tool-call smoke test before autonomous implementation.
5. Use LM Studio for local navigation, triage, summaries and simple scaffolding—not as the sole reviewer for production-critical native code.
6. Add Anthropic or direct DeepSeek API providers only as deliberate alternatives with separate authentication/billing. Do not replace the Go baseline without a documented benchmark or provider failure.
7. Optionally use Claude Code as a separate plan/review environment. It should write a plan or report; OpenCode stays the source-changing owner. Never allow both tools to edit the same worktree at once.

## 7. DeepSeek tool-call smoke test

Before a multi-agent run:
- Verify actual provider/model.
- Run directory listing and file read.
- Make a temporary edit in a disposable worktree.
- Run one test command.
- Verify multiple consecutive tool-call turns.
- Test resume/compaction if supported.
- Inspect errors and actual model routing.

If built-in OpenAI-compatible tool calls fail, stop. Capture a minimal reproduction without secrets and test the documented Anthropic-compatible custom-provider route. Do not retry a broken loop repeatedly.

## 8. Token efficiency

- Use docs/PROJECT_MAP.md; read only relevant architecture sections.
- Avoid loading full repository or logs into every context.
- Research agents return key claims, links, version requirements and unresolved issues.
- Implementers return small diffs and concise test evidence.
- Stable global rules go in AGENTS.md; procedural methods belong in skills.
- Keep stable prefixes for provider caching where supported.
- Make durable checkpoints before fresh sessions.
- Measure cost per accepted feature and human rework, not raw output tokens.
- A 1M context is a capability, not a reason to fill the entire window.

## 9. Controlled benchmark

Use the same clean commit, prompt, acceptance criteria and permissions in separate worktrees. Test:
1. Explain a seeded bug without edits.
2. Implement one DSP module and deterministic tests.
3. Implement one renderer change plus preview evidence.
4. Diagnose a seeded cross-module bug.
5. Review a native lifecycle/security patch read-only.

Record accepted criteria, tests passed, regressions, manual corrections, retries, time, diff size, input/cache/output tokens and billed cost. The winner is the configuration that produces verified correct work with less human rework—not more generated code.

## 10. Re-evaluation trigger

Revisit this decision after a controlled benchmark, material provider/subagent changes, material price/usage changes or repeated project-specific tool failures.
