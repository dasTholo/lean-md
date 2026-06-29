# ctx_symbol → ctx_search Migration + `@symbol body` Op — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `@symbol body name=X` op routing to `ctx_search action=symbol`, and migrate all `ctx_symbol` doc/seed references to `ctx_search`.

**Architecture:** lean-md never calls `ctx_symbol` at runtime — the `@symbol` bridge routes nav/overview to `ctx_refactor`. We add ONE new op (`body`) to the existing `SymbolBridge` that routes outbound to `ctx_search action=symbol` (AST-precise symbol-body-by-name, the replacement for the deprecated `ctx_symbol`). The remaining work is renaming `ctx_symbol` → `ctx_search` in seeds, one test assertion, and three living-doc lines. The old bridge stays fully intact; `ctx_refactor` remains its backing for nav/overview.

**Tech Stack:** Rust (crate `lean_md`), `cargo nextest`, `serde_json`. Bridge pattern: `DirectiveBridge::execute(ctx, args) -> Result<String, BridgeError>`; outbound via `ctx.backend.call(tool, args)`.

## Global Constraints

- Tests: always `cargo nextest run`, never `cargo test`.
- Shell: every command its own invocation; no `&&`/`||`/`;` chaining.
- Before `git add` of any `.rs` file: `cargo fmt`.
- Zero clippy warnings; all tests pass.
- Determinism (#498): seed `.md` files are embedded via `include_str!` and must stay byte-identical built-in == on-disk. The gate `builtin_fragments_match_seed_files_on_disk` enforces this — editing the seed file updates both (compile-time embed).
- Output verbatim: code-intel bridge output is returned unchanged (no local CRP) for the new `body` op, consistent with the nav ops.
- No worktrees — work directly on branch `feat-lmd-v2`.

---

## File Structure

- `src/bridges/symbol.rs` — **modify**: add `body()` fn + `execute()` dispatch branch + ERROR-message update; add tests. (Single responsibility: the `@symbol` bridge. All new code lives here.)
- `content/core/hard-rules.lmd.md` — **modify**: seed line, `ctx_symbol` → `ctx_search:symbol`.
- `content/core/_fragments/tool-quick-ref.lmd.md` — **modify**: seed line, `ctx_symbol` → `ctx_search:symbol`.
- `src/fragments.rs` — **modify**: flip one assertion `ctx_symbol` → `ctx_search:symbol`.
- `AGENTS.md`, `docs/21-lean-md.md`, `docs/appendix-lean-md.md` — **modify**: living-doc references.

---

## Task 1: New `@symbol body` op routing to `ctx_search action=symbol`

**Files:**
- Modify: `src/bridges/symbol.rs` (add `body()` fn ~after `overview()`; add `execute()` branch; update `nav()` ERROR string; update unknown-op message; add tests in the `#[cfg(test)] mod tests`)
- Test: `src/bridges/symbol.rs` (same file, inline test module)

**Interfaces:**
- Consumes: `DirectiveArgs::positional(0) -> Option<&str>` (the op), `DirectiveArgs::get("name"|"file"|"kind") -> Option<&str>`; `ctx.jail_root: PathBuf`; `ctx.backend.call(&str, serde_json::Value) -> Result<String, BackendError>`; `crate::pathx::resolve_tool_path(Some(root), None, path) -> Result<String, _>`; `EngineContext::with_backend(LeanMdHeader, PathBuf, Box<dyn CodeIntelBackend>) -> EngineContext`.
- Produces: a new bridge op `body` reachable as `@symbol body name=X [file=…] [kind=…]`. Payload shape sent to `ctx_search`: `{ "action":"symbol", "name":<str>, "path":<jail_root>, "file"?:<abs>, "kind"?:<str> }`. No new public Rust symbols (all `fn` are private to the module).

- [ ] **Step 1: Write the failing tests**

Add these four tests inside the existing `#[cfg(test)] mod tests { … }` block in `src/bridges/symbol.rs` (the module already imports `super::*`, `LeanMdHeader`, `PathBuf`, and has `ctx_at`):

```rust
    /// Build an EngineContext whose backend records every outbound (tool, args).
    fn recording_ctx(
        root: PathBuf,
    ) -> (
        Rc<EngineContext>,
        std::rc::Rc<std::cell::RefCell<Vec<(String, serde_json::Value)>>>,
    ) {
        use crate::backend::{BackendError, CodeIntelBackend};
        use std::cell::RefCell;
        struct Rec {
            calls: std::rc::Rc<RefCell<Vec<(String, serde_json::Value)>>>,
        }
        impl CodeIntelBackend for Rec {
            fn call(
                &self,
                tool: &str,
                args: serde_json::Value,
            ) -> Result<String, BackendError> {
                self.calls.borrow_mut().push((tool.to_string(), args));
                Ok(String::new())
            }
        }
        let calls = std::rc::Rc::new(RefCell::new(Vec::new()));
        let ctx = Rc::new(EngineContext::with_backend(
            LeanMdHeader::default(),
            root,
            Box::new(Rec {
                calls: calls.clone(),
            }),
        ));
        (ctx, calls)
    }

    #[test]
    fn body_forwards_name_file_and_kind_to_ctx_search() {
        let dir = std::env::temp_dir().join("lmd_symbol_body_fwd");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("s.rs");
        std::fs::write(&f, "pub fn target() {}\n").unwrap();
        let (ctx, calls) = recording_ctx(dir.clone());
        let args = DirectiveArgs::parse(&format!(
            "body name=target file={} kind=fn",
            f.to_str().unwrap()
        ));
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        assert!(out.is_empty(), "recording backend returns empty: {out}");
        let calls = calls.borrow();
        let (tool, payload) = calls.first().expect("one outbound call recorded");
        assert_eq!(tool, "ctx_search", "routes to ctx_search");
        assert_eq!(payload.get("action").and_then(|v| v.as_str()), Some("symbol"));
        assert_eq!(payload.get("name").and_then(|v| v.as_str()), Some("target"));
        assert_eq!(payload.get("kind").and_then(|v| v.as_str()), Some("fn"));
        assert!(payload.get("file").is_some(), "file forwarded: {payload}");
    }

    #[test]
    fn body_missing_name_errors() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = SymbolBridge
            .execute(&ctx, &DirectiveArgs::parse("body"))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg("name")), "got: {err:?}");
    }

    #[test]
    fn body_op_dispatches_not_unknown() {
        // body must NOT hit the unknown-op path even without file=/kind=.
        let dir = std::env::temp_dir().join("lmd_symbol_body_disp");
        std::fs::create_dir_all(&dir).unwrap();
        let (ctx, calls) = recording_ctx(dir.clone());
        let args = DirectiveArgs::parse("body name=Widget");
        let out = SymbolBridge.execute(&ctx, &args).unwrap();
        assert!(!out.contains("unknown @symbol op"), "must dispatch: {out}");
        let calls = calls.borrow();
        let (tool, payload) = calls.first().expect("one call recorded");
        assert_eq!(tool, "ctx_search");
        assert_eq!(payload.get("name").and_then(|v| v.as_str()), Some("Widget"));
        assert!(payload.get("file").is_none(), "no file when omitted");
        assert!(payload.get("kind").is_none(), "no kind when omitted");
    }

    #[test]
    fn unknown_op_message_lists_body() {
        let ctx = ctx_at(PathBuf::from("."));
        let err = SymbolBridge
            .execute(&ctx, &DirectiveArgs::parse("frobnicate x.rs"))
            .unwrap_err();
        match err {
            BridgeError::Resolve(m) => assert!(m.contains("body"), "op list names body: {m}"),
            other => panic!("expected Resolve, got: {other:?}"),
        }
    }
```

Also update the EXISTING test `name_addressing_without_line_returns_clear_error` so it pins the new message (it currently only checks `"ERROR"` + `"line="`). Replace its body's final assertion with:

```rust
        assert!(
            out.contains("ERROR") && out.contains("line=") && out.contains("@symbol body"),
            "must explain name= needs line= and point to @symbol body: {out}"
        );
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run: `cargo nextest run -p lean-md body_forwards_name_file_and_kind_to_ctx_search body_missing_name_errors body_op_dispatches_not_unknown unknown_op_message_lists_body name_addressing_without_line_returns_clear_error`
Expected: FAIL — `body_*` panic with `unknown @symbol op 'body'` (or compile-OK runtime fail); `unknown_op_message_lists_body` fails (message lacks `body`); `name_addressing_*` fails (message lacks `@symbol body`).

- [ ] **Step 3: Add the `body()` fn**

Insert this function in `src/bridges/symbol.rs` immediately after the `overview()` fn (before the `#[cfg(test)]` module):

```rust
/// `@symbol body name=X [file=…] [kind=fn|struct|class|trait|enum]` →
/// `ctx_search action=symbol`. Fetches one symbol's AST-precise body by name —
/// the replacement for the deprecated `ctx_symbol` tool. `path` is scoped to the
/// jail root; `file` (if given) is jail-resolved to narrow the search. Output is
/// returned verbatim from the backend (no local CRP), consistent with the nav ops.
fn body(
    ctx: &Rc<EngineContext>,
    args: &DirectiveArgs,
    root: &str,
) -> Result<String, BridgeError> {
    let name = args.get("name").ok_or(BridgeError::MissingArg("name"))?;

    let mut obj = serde_json::Map::new();
    obj.insert("action".into(), "symbol".into());
    obj.insert("name".into(), name.into());
    obj.insert("path".into(), root.into());
    if let Some(file) = args.get("file") {
        let abs = crate::pathx::resolve_tool_path(Some(root), None, file)
            .map_err(|e| BridgeError::Resolve(format!("path blocked by jail: {e}")))?;
        obj.insert("file".into(), abs.into());
    }
    if let Some(kind) = args.get("kind") {
        obj.insert("kind".into(), kind.into());
    }

    let out = ctx
        .backend
        .call("ctx_search", serde_json::Value::Object(obj))
        .map_err(BridgeError::Backend)?;
    Ok(out)
}
```

- [ ] **Step 4: Wire the dispatch branch + update messages in `execute()`/`nav()`**

In `SymbolBridge::execute`, the body currently reads:

```rust
        let op = args.positional(0).ok_or(BridgeError::MissingArg("op"))?;
        let action = map_op(op).ok_or_else(|| {
            BridgeError::Resolve(format!(
                "unknown @symbol op '{op}'. Use: refs|def|impl|declaration|type-hierarchy|overview"
            ))
        })?;
        let root = ctx.jail_root.to_str().unwrap_or(".");

        if action == "symbols_overview" {
            return overview(ctx, args, root);
        }
        nav(ctx, args, action, root)
```

Replace it with (hoist `root`, add the `body` branch before `map_op`, extend the op list):

```rust
        let op = args.positional(0).ok_or(BridgeError::MissingArg("op"))?;
        let root = ctx.jail_root.to_str().unwrap_or(".");

        if op == "body" {
            return body(ctx, args, root);
        }

        let action = map_op(op).ok_or_else(|| {
            BridgeError::Resolve(format!(
                "unknown @symbol op '{op}'. Use: refs|def|impl|declaration|type-hierarchy|overview|body"
            ))
        })?;

        if action == "symbols_overview" {
            return overview(ctx, args, root);
        }
        nav(ctx, args, action, root)
```

Then in `nav()`, update the `name=`-without-`line=` ERROR string. It currently returns:

```rust
        return Ok(
            "ERROR: name= addressing requires line= (resolve_name_path is not available \
             outbound; provide path= line= column= explicitly)"
                .to_string(),
        );
```

Replace that returned string with:

```rust
        return Ok(
            "ERROR: name= addressing needs line= for nav ops (resolve_name_path is not \
             available outbound). For a symbol body by name use '@symbol body name=…'."
                .to_string(),
        );
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo nextest run -p lean-md body_forwards_name_file_and_kind_to_ctx_search body_missing_name_errors body_op_dispatches_not_unknown unknown_op_message_lists_body name_addressing_without_line_returns_clear_error`
Expected: PASS (5 passed).

- [ ] **Step 6: Run the full symbol-bridge module + clippy**

Run: `cargo nextest run -p lean-md bridges::symbol`
Expected: PASS — all existing nav/overview tests still green (no regression).

Run: `cargo clippy -p lean-md --all-targets`
Expected: no warnings.

- [ ] **Step 7: Format + commit**

Run: `cargo fmt -p lean-md`

```bash
git add src/bridges/symbol.rs
git commit -m "feat(symbol): add @symbol body op routing to ctx_search action=symbol"
```

---

## Task 2: Migrate seed references + fragments test assertion

**Files:**
- Modify: `content/core/hard-rules.lmd.md:4` (seed)
- Modify: `content/core/_fragments/tool-quick-ref.lmd.md:3` (seed)
- Modify: `src/fragments.rs:157-160` (test assertion)

**Interfaces:**
- Consumes: nothing from Task 1 (independent — runtime already migrated).
- Produces: seeds whose `@symbol` backing reads `ctx_search:symbol`; the gate `builtin_fragments_match_seed_files_on_disk` stays green (include_str re-embeds at compile time).

- [ ] **Step 1: Flip the fragments assertion (failing first)**

In `src/fragments.rs`, the assertion at lines 157-160 currently reads:

```rust
        assert!(
            out.contains("ctx_symbol"),
            "hard-rules must name ctx_symbol for *.rs"
        );
```

Replace it with (note: `ctx_search:symbol` is the *specific* new token — a plain `ctx_search` would pass trivially since hard-rules line 1 already names `ctx_search`):

```rust
        assert!(
            out.contains("ctx_search:symbol"),
            "hard-rules must name ctx_search:symbol for *.rs (@symbol backing)"
        );
```

- [ ] **Step 2: Run the fragments tests to verify the assertion fails**

Run: `cargo nextest run -p lean-md fragments`
Expected: FAIL — `hard-rules must name ctx_search:symbol for *.rs` (seed still says `ctx_symbol`). The byte-identical gate `builtin_fragments_match_seed_files_on_disk` still PASSES (built-in == disk, both unchanged).

- [ ] **Step 3: Edit the hard-rules seed**

In `content/core/hard-rules.lmd.md`, replace the line:

```
  ctx_refactor / ctx_symbol (@symbol) — rename/move/extract over hand edits.
```

with:

```
  ctx_refactor / ctx_search:symbol (@symbol) — rename/move/extract over hand edits.
```

Use `mcp__lean-ctx__ctx_edit(path="content/core/hard-rules.lmd.md", old_string="ctx_refactor / ctx_symbol (@symbol)", new_string="ctx_refactor / ctx_search:symbol (@symbol)")`.

- [ ] **Step 4: Edit the tool-quick-ref seed**

In `content/core/_fragments/tool-quick-ref.lmd.md`, replace the token `@symbol=ctx_refactor/ctx_symbol` with `@symbol=ctx_refactor/ctx_search:symbol`.

Use `mcp__lean-ctx__ctx_edit(path="content/core/_fragments/tool-quick-ref.lmd.md", old_string="@symbol=ctx_refactor/ctx_symbol", new_string="@symbol=ctx_refactor/ctx_search:symbol")`.

- [ ] **Step 5: Run the fragments tests to verify they pass**

Run: `cargo nextest run -p lean-md fragments`
Expected: PASS — assertion green AND `builtin_fragments_match_seed_files_on_disk` green (include_str re-embeds the edited seed at compile time, so built-in == disk).

- [ ] **Step 6: Commit**

(No `.rs` formatting needed beyond `fragments.rs`; run fmt to be safe.)

Run: `cargo fmt -p lean-md`

```bash
git add content/core/hard-rules.lmd.md content/core/_fragments/tool-quick-ref.lmd.md src/fragments.rs
git commit -m "refactor(seeds): migrate @symbol backing ctx_symbol → ctx_search:symbol"
```

---

## Task 3: Migrate living-doc references

**Files:**
- Modify: `AGENTS.md:16`
- Modify: `docs/21-lean-md.md:123`
- Modify: `docs/appendix-lean-md.md:20`

**Interfaces:**
- Consumes: nothing (pure docs).
- Produces: living docs consistent with the migrated runtime/seeds. (Historical `docs/lean-md/plans/…` and `docs/lean-md/specs/…` are dated records — left unchanged.)

- [ ] **Step 1: Edit AGENTS.md**

Replace the line (line 16):

```
- **File editing** → `ctx_edit`; symbol nav / refactor / reformat via `ctx_refactor`/`ctx_symbol`
```

with:

```
- **File editing** → `ctx_edit`; symbol nav / refactor / reformat via `ctx_refactor`; symbol body by name via `ctx_search action=symbol`
```

Use `mcp__lean-ctx__ctx_edit(path="AGENTS.md", old_string="symbol nav / refactor / reformat via `ctx_refactor`/`ctx_symbol`", new_string="symbol nav / refactor / reformat via `ctx_refactor`; symbol body by name via `ctx_search action=symbol`")`.

- [ ] **Step 2: Edit docs/21-lean-md.md**

Replace the table row (line 123):

```
| `@symbol` | R | `ctx_refactor` + `ctx_symbol` | 3.2 | read-only |
```

with:

```
| `@symbol` | R | `ctx_refactor` + `ctx_search action=symbol` | 3.2 | read-only |
```

Use `mcp__lean-ctx__ctx_edit(path="docs/21-lean-md.md", old_string="`ctx_refactor` + `ctx_symbol`", new_string="`ctx_refactor` + `ctx_search action=symbol`")`.

- [ ] **Step 3: Edit docs/appendix-lean-md.md**

In the `@symbol` table row (line 20), replace the cell token `` `ctx_refactor`+`ctx_symbol` `` with `` `ctx_refactor`+`ctx_search action=symbol` ``.

Use `mcp__lean-ctx__ctx_edit(path="docs/appendix-lean-md.md", old_string="`ctx_refactor`+`ctx_symbol`", new_string="`ctx_refactor`+`ctx_search action=symbol`")`.

(Column padding may shift slightly; markdown table rendering is unaffected — no realignment required.)

- [ ] **Step 4: Verify no stray `ctx_symbol` remains in living docs/seeds/src**

Run: `mcp__lean-ctx__ctx_search(pattern="ctx_symbol", path="/home/tholo/Scripts/lean-md")`
Expected: matches ONLY under `docs/lean-md/plans/…` and `docs/lean-md/specs/…` (historical, intentionally left). No hits in `AGENTS.md`, `content/`, `src/`, `docs/21-lean-md.md`, `docs/appendix-lean-md.md`.

- [ ] **Step 5: Commit**

```bash
git add AGENTS.md docs/21-lean-md.md docs/appendix-lean-md.md
git commit -m "docs: migrate @symbol backing references ctx_symbol → ctx_search action=symbol"
```

---

## Final Verification

- [ ] **Step 1: Full test suite**

Run: `cargo nextest run`
Expected: PASS — all tests green (no `cargo test`).

- [ ] **Step 2: Clippy clean**

Run: `cargo clippy --all-targets`
Expected: no warnings.

---

## Self-Review Notes

- **Spec coverage:** §Design 1 (new `body` op) → Task 1. §Design 2 (ERROR-path) → Task 1 Step 4. §Design 3 (seeds) → Task 2. §Design 4 (fragments test) → Task 2. §Design 5 (living docs) → Task 3. §Tests table → Task 1 Step 1. §Verification → Final Verification. All covered.
- **Determinism:** seed edits are byte-stable; `builtin_fragments_match_seed_files_on_disk` re-validated in Task 2 Step 5.
- **Type consistency:** `body()` signature `(ctx, args, root)` mirrors `overview()`/`nav()`; payload field names (`action/name/path/file/kind`) match `ctx_search action=symbol` schema; tests reference only existing helpers (`ctx_at`, `EngineContext::with_backend`, `DirectiveArgs::parse`).
- **Open dependency:** Task 1 assumes `ctx_search action="symbol"` is available in the pinned lean-ctx version (`docs/CONTRACT.md`). Tests use a RecordingBackend and never hit a live backend, so they pass regardless; only real `@symbol body` usage needs the live tool.
