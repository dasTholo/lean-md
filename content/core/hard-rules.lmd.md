# Hard Rules (lmd built-in)
- Never native Read/Grep/cat/sed; never `ctx_shell raw=true` unless compression is provably wrong.
- All other I/O + code-intel runs through lean-ctx tools — see `tooling/mcp-tools`;
  language-specific symbol/edit/reformat conventions live in `lang/<lang>` (e.g. `lang/rust`).
