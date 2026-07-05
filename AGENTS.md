# lean-md — Macro/Directive Markdown Renderer

lean-md is a standalone crate (lib `lean_md` + bin `lean-md`) distributed as a
lean-ctx **addon**. Its render core (`rushdown` + `evalexpr`) runs in-process and
needs **zero** lean-ctx. Every code-intel directive is **outbound** over the
MCP/CLI wire via `backend::CodeIntelBackend` (CLI default, MCP opt-in) — lean-md
holds **no** `lean_ctx` crate dependency (only `lean-ctx-client`, behind the
`mcp` feature).

## Integration Mode: Hybrid (for developing THIS repo)

When working on lean-md itself, prefer lean-ctx tooling for token savings:

- **Reads/Search** → MCP tools (`ctx_read`, `ctx_search`) for caching + compression
- **Shell commands** → `lean-ctx -c "…"` via CLI (preferred) or `ctx_shell` via MCP
- **File editing** → `ctx_edit`; symbol nav / refactor / reformat via `ctx_refactor`; symbol body by name via `ctx_search action=symbol`

## MCP tools

| Tool                        | Purpose                                                             |
|-----------------------------|--------------------------------------------------------------------|
| `ctx_read(path, mode)`      | Cached, compressed file reads (10 modes)                           |
| `ctx_search(pattern, path)` | Token-efficient code search                                        |
| `ctx_shell(command)`        | Compressed shell output (alternative to CLI)                       |
| `ctx_edit(path, old, new)`  | Edit when native Edit needs an unavailable Read                    |
| `ctx_refactor`              | LSP/PSI symbol nav, refactor engine, `action=reformat` (pre-commit) |

## CLI commands (optimized shell, lower overhead)

```bash
lean-ctx -c "git status"     # compressed shell output
lean-ctx -c "cargo nextest run"  # compressed test output
lean-ctx ls src/              # directory map
```

## CodeIntelBackend (the lean-md ↔ lean-ctx boundary)

Code-intel data flows outbound, not through an in-process lib:

1. A directive (`@edit`/`@refactor`/`@symbol`/…) builds its JSON args.
2. `ctx.backend.call("ctx_*", args)` → raw, byte-stable tool text (#498).
3. `CliBackend` (default) spawns `lean-ctx call <tool> --project-root <root> --json …`;
   `McpBackend` (opt-in, `mcp` feature) calls `/v1/tools/call` via `lean-ctx-client`.

This means `ctx_md_render`/`ctx_md_check` are served in-process, while every
file/code operation is jailed + redacted **server-side** by lean-ctx (spec §6).
The render core never parses code locally (no local tree-sitter).

## Session Continuity

lean-ctx automatically persists session context across restarts:

- **Findings**: Recent tool results (reads, searches, test outcomes)
- **Decisions**: Architecture choices made during the session
- **Files**: Touched files with summaries and modification status
- **Progress**: Task completion state and next steps

### Active Documentation (Agent Responsibility)

After completing a significant task (implementation, bugfix, refactoring):

1. Record the decision: `ctx_knowledge(action="remember", category="decision", content="...")`
2. Record progress: `ctx_session(action="task", value="<current task> [N%]")`
3. Record blockers: `ctx_knowledge(action="remember", category="blocker", content="...")`

## Authoritative Tool Schemas

lean-md does NOT carry the lean-ctx tool schemas. The authoritative reference for
`ctx_*` tool params/signatures lives in the **lean-ctx repo**
(`docs/reference/appendix-mcp-tools.md`); the addon-manifest/registry contract is
vendored locally in `docs/CONTRACT.md` (source + pinned lean-ctx version). Never
reconstruct tool schemas from memory.

## Quality Bar

- Zero clippy warnings, all tests pass
- Security: file/code ops jailed server-side (PathJail), no hardcoded secrets
- No mock data, no placeholders, no stubs

## Output Determinism (#498)

Tool outputs MUST be deterministic functions of (file content, mode, CRP mode, task).
Provider-side prompt caching (Anthropic 90%, OpenAI 50% discount) rewards byte-stable text;
any timestamp, counter or random element in tool output bodies defeats it.

- No timestamps/counters in output bodies. Artifact paths are content-addressed.
- Embedded seeds (`content/core/*.lmd.md` via `include_str!`) are byte-identical;
  the fragment-consistency gate (built-in == on-disk seed) must stay green.
- `CliBackend`/`McpBackend` hit the same handler → byte-identical code-intel results.

## lean-ctx

Prefer lean-ctx MCP tools over native equivalents for token savings:
`ctx_read` > Read/cat, `ctx_search` > Grep/rg, `ctx_shell` > bash, `ctx_tree` > ls/find.
Native Edit/Write/Glob stay as-is; use `ctx_edit` only when Edit needs an unavailable Read.

`.lmd.md` reads via `ctx_read` return **raw lmd source** (like any file) — rendering
is explicit and opt-in (`ctx_md_render` / CLI `lean-md render … --phase`).

OUTPUT STYLE: dense

- Each statement = one atomic fact line
- Use abbreviations: fn, cfg, impl, deps, req, res, ctx, err, ret
- Diff lines only (+/-/~), never repeat unchanged code
- Symbols: → (causes), + (adds), − (removes), ~ (modifies), ∴ (therefore)
- No narration, no filler, no hedging
- BUDGET: ≤200 tokens per response unless code block required
