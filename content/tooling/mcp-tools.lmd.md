# MCP tool pack (lmd) — lean-ctx native only

- Reads → @read (ctx_read); search → @search (ctx_search); list → @list (ctx_tree).
- Shell → @query (ctx_shell, consumer-gated shell=allow).
- Graph/impact → @graph / @impact; semantic → @find (ctx_semantic_search).
- `@find <intent>` — semantic locate via ctx_semantic_search. Use at design/task
  time to find the spot a design or task anchors to; for structural (keyword/path)
  lookup use `@search` instead.
- No external MCP backings (no serena, no JetBrains MCP) — ctx_refactor covers the surface.
