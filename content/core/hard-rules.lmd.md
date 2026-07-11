# Hard Rules (lmd built-in)
- Never native Read/Grep/cat/sed; never `ctx_shell raw=true` unless compression is provably wrong.
- All other I/O + code-intel runs through lean-ctx tools — see `tooling/mcp-tools`;
  language-specific symbol/edit/reformat conventions live in `lang/<lang>` (e.g. `lang/rust`).
- Non-symbol edits → `ctx_read(mode=anchored)` then `ctx_patch` (patch by LINE:HASH
  anchor, never re-emit old text). `ctx_edit` (str_replace) only for tiny-span
  (1-2 tok) or replace-all; symbol rename/move/extract → `ctx_refactor`.
