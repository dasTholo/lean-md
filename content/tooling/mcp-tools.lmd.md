# MCP tool pack (lmd) — lean-ctx native only
- Reads → @read (ctx_read); search → @search (ctx_search); list → @list (ctx_tree).
- Shell → @query (ctx_shell, consumer-gated shell=allow).
- Graph/impact → @graph / @impact; semantic → @find (ctx_semantic_search).
- No external MCP backings (no serena, no JetBrains MCP) — ctx_refactor covers the surface.
