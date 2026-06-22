# Rust language pack (lmd)
- Build/test via @query — never a raw shell loop:
  @query "cargo nextest run"          # never `cargo test`
  @query "cargo clippy --all-targets" # zero warnings is the bar
- Symbol nav/refactor: @symbol / ctx_refactor (rust-analyzer headless backing).
- Reformat before commit: ctx_refactor action=reformat (rustfmt).
