# Rust language pack (lmd)
- Build/test via @query — never a raw shell loop:
  @query "cargo clippy --all-targets" # zero warnings is the bar
- Symbol navigation (read/locate): @symbol — for symbol EDITS see the refactor rule below.

## Plan-content rule: symbol changes go through @refactor

- Rust tasks that rename / move / extract a symbol MUST instruct `@refactor <op>
  <symbol>` (ctx_refactor) — **no** hand-edits.
- Non-symbol changes (text / config / doc lines) → `ctx_read mode=anchored` →
  `ctx_patch` (anchored, no old-text recall). `@edit` (ctx_edit) only as the
  tiny-span (1-2 tok) / replace-all exception.
- Reformat before commit via `@reformat` (ctx_refactor action=reformat / rustfmt).
- Anchor the affected callers with `@graph callers <symbol>` before the refactor.
