# Hard Rules (lmd built-in)
- I/O only via lean-ctx MCP tools (ctx_read/ctx_search/ctx_tree/ctx_shell).
- Never use native Read/Grep/cat/sed; never `ctx_shell raw=true` unless compression is provably wrong.
- For *.rs prefer symbol-aware tools: navigate & refactor via
  ctx_refactor / ctx_symbol (@symbol) — rename/move/extract over hand edits.
- Plain @edit / ctx_edit only for non-symbol changes; reformat before
  commit via ctx_refactor action=reformat.
