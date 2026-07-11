# MCP tool pack (lmd) — lean-ctx native only

- Reads → @read (ctx_read); search → @search (ctx_search); list → @list (ctx_tree).
- Shell → @query (ctx_shell, consumer-gated shell=allow).
- Graph/impact → @graph / @impact; semantic → @find (ctx_semantic_search).
- `@find <intent>` — semantic locate via ctx_semantic_search. Use at design/task
  time to find the spot a design or task anchors to; for structural (keyword/path)
  lookup use `@search` instead.
- No external MCP backings (no serena, no JetBrains MCP) — ctx_refactor covers the surface.

## Directive usage reference (for the plan author)

- Non-symbol edits → the anchored loop: `@read <path> mode=anchored` (ctx_read →
  LINE:HASH anchors) then `ctx_patch` (patch by anchor, never re-emit old text).
  This is the default edit path. `@edit` (ctx_edit str_replace) is the exception:
  tiny-span (1-2 tok, anchor ≥ old_string) or replace-all across scattered lines.
- `@refactor <op> <symbol>` — LSP-safe symbol edits via ctx_refactor. Ops:
  `rename` / `move` / `extract`. **Use** for symbol changes in Rust; non-symbol
  edits take the anchored loop above (`@edit` only tiny-span / replace-all).
- `@review diff-review` — fused review verdict (impact + caller + smells) on a diff.
  **Use** as a post-change gate.
- `@smells [scan|summary] <path>` — code-smell findings (ctx_smells). **Use** as a
  quality gate on changed files.
- `@graph <callers|callees|dependents> <symbol>` — call/dep graph
  (ctx_callgraph / ctx_graph). **Use** for task decomposition & as a refactor anchor.
- `@impact <symbol>` — blast radius before edits (ctx_impact). **Use** to justify a
  task's invasiveness.
- `@recall <query>` / `@remember <content>` — read/write durable knowledge
  (ctx_knowledge). **Use** to seed spec decisions / save task gotchas.
