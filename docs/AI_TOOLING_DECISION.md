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

## 3. Role routing

| Role | Recommended model class | Economical option |
|---|---|---|
| Lead/coordinator | Frontier reasoning | DeepSeek V4 Pro |
| Systems architect | Frontier reasoning | DeepSeek V4 Pro |
| Rust engine | Strong coding/reasoning for core contracts | Flash for bounded code + tests |
| Audio DSP | Strong reasoning for DSP math/real-time design | Flash for deterministic fixtures |
| Graphics | Strong coding/reasoning for GPU lifecycle | Flash for small shader changes with preview tests |
| Platform integration | Frontier reasoning for OS/FFI/lifecycle | Flash for well-bounded adapter changes |
| Image intelligence | Strong model for checkpoint/inference integration | Flash for preprocessing |
| Tauri UI | Capable coding model | Flash/local for isolated components |
| Desktop automation | Strong systems/platform reasoning | Flash after interfaces are fixed |
| QA/licensing | Independent read-only model | Second independent model |
| Steam/package | Strong security/API model | Flash for bounded schema work |

Do not pin model IDs in agent files until providers are connected and exact IDs are verified with OpenCode /models.

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

1. Keep OpenCode and current repository agents/skills.
2. Connect DeepSeek via OpenCode's official /connect provider flow and select the current model from /models.
3. Test read/search, one disposable edit, shell/test calls, three consecutive tool-call turns and resume/compaction.
4. Add Anthropic API as a separate provider only if the user accepts separate API billing and wants Opus in OpenCode.
5. Assign a strong model to coordinator/architect and independent review; use cheaper models for bounded implementation.
6. Use LM Studio for local navigation, triage, simple scaffolding and repetitive tasks—not as sole reviewer for production-critical native code.
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
