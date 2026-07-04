# Hard Rules (lmd built-in)
- Never native Read/Grep/cat/sed; never `ctx_shell raw=true` unless compression is provably wrong.
- All other I/O + code-intel runs through lean-ctx tools — see `tooling/mcp-tools`;
  language-specific symbol/edit/reformat conventions live in `lang/<lang>` (e.g. `lang/rust`).
- A `.lmd.md` is a **rendered artifact, not a source file**. `@read`/ctx_read
  (any mode, `raw` included) and every `render` path RENDER it → `@import` NotFound
  cascade, `@phase` isolation collapse, `@define` macros consumed (the file looks
  "empty"). Access it with lean-md renderer means, per intent:
  - a task/phase brief                                   → `render --phase <p>`
  - the macro API index                                 → `render --signatures`
  - the raw source (copy shape / set exact edit anchors) → `lean-md source <file>`
  Never native cat/Read and never ctx_read for `.lmd.md` source — both render it.
