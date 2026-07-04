# Rust language pack (lmd)
- Build/test via @query — never a raw shell loop:
  @query "cargo clippy --all-targets" # zero warnings is the bar
- Symbol nav/refactor: @symbol / ctx_refactor
- Reformat before commit: ctx_refactor action=reformat (rustfmt).

## Plan-content rule: symbol changes go through @refactor

- Rust tasks that rename / move / extract a symbol MUST instruct `@refactor <op>
  <symbol>` (ctx_refactor) — **no** hand-edits.
- `@edit` is for non-symbol changes only (text / config / doc lines).
- Reformat before commit via `@reformat` (ctx_refactor action=reformat / rustfmt).
- Anchor the affected callers with `@graph callers <symbol>` before the refactor.
