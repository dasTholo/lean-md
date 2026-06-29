# Journey 21 — lean-md (lmd) Directives

> Full reference for `@directive` calls inside `.lmd` / `.lean-md` files.
> Condensed agent reference: [`appendix-lean-md.md`](appendix-lean-md.md).
> MCP tool schemas: [`appendix-mcp-tools.md`](appendix-mcp-tools.md) and
> `rust/src/tools/registered/<tool>.rs`.
> Design spec: `docs/lean-md/specs/2026-05-31-lmd-lean-ctx-native-design.mdai.md`.

---

## 1. Architecture (parser → bridge registry → render)

The lmd render pipeline is a rushdown Markdown-to-HTML renderer wired with two
extensions — a block/inline parser and a renderer hook — both sharing one
`EngineContext` per render pass (spec §4.1).

```
@lean-md header   → parse_header (LeanMdHeader)
                           ↓
body              → lmd_parser_extension  (rushdown block + inline parser)
                           ↓
rushdown AST      → lmd_renderer_extension (renderer hook)
                           ↓
@<directive> node → BridgeRegistry::dispatch → DirectiveBridge::execute
                           ↓
rendered HTML / plain text output
```

Every `@<name>` at the start of a line is claimed by the block parser and
dispatched to the registry. Unknown names render as an HTML comment
`<!-- lmd: unknown directive @<name> -->`.

### 1.1 The R/E split: bridges (R) vs. engine primitives (E) (spec §4.6)

**E-primitives** are built into the engine itself and do not go through the
bridge registry:

| Primitive | What it does |
|---|---|
| Block-directive parser | Captures `@<name>` lines; dispatches to registry |
| Inline parser | `{{ expr }}` and inline `@recall` / `@on complete` |
| Container transformer | `@if`/`@elseif`/`@else`/`@if-end`, `@consumer=ai/human` |
| Macro engine | `@define`/`@call` with param substitution; `@include`/`@import` |
| Pipe + `@render` | Postfix AST transformer (`… | @render type=table`) — Phase 4 |
| TDD render hook | Renderer output gated on `tdd_schema` header flag |

**R-bridges** are registered `DirectiveBridge` implementations that route to an
existing `ctx_*` core tool (no new algorithms — pure routing):

```rust
pub trait DirectiveBridge {
    fn name(&self) -> &'static str;
    fn execute(&self, ctx: &EngineContext, args: &DirectiveArgs)
        -> Result<String, BridgeError>;
}
```

### 1.2 EngineContext, jail_root & the shared file cache

`EngineContext` is constructed once per render and shared (via `Rc`) with every
bridge invoked during that render. It holds:

- `header: LeanMdHeader` — parsed `@lean-md` header flags (shell permission,
  consumer, TDD mode, …).
- `jail_root: PathBuf` — the project root; all FS-path args are resolved through
  `resolve_tool_path` against this root (path jail). Route prefixes and raw diff
  text are **not** jail-resolved.
- `registry: BridgeRegistry` — the set of registered R-bridges
  (`default_registry()` at construction).
- `cache: RefCell<SessionCache>` — one session cache shared by all bridges in
  this render (the Read→Delta guarantee, §1.3).
- `max_chain_depth` / `depth` — `@include` recursion guard (default 16,
  `BridgeError::DepthExceeded` on overflow).
- Lazy memos: `graph_index` and `call_graph` — built at most once per render,
  shared by all `@graph` ops.

### 1.3 Cache coherence: which directives clear the cache, which never do (spec §3.4)

**Read→Delta guarantee:** the shared `SessionCache` means the second `@read` of
the same path within one render is a ~13-token cache-hit auto-delta, never a
full re-dump. `fresh`/`raw` are explicit escape hatches; they are **forbidden**
in the bridge path (spec §4.2a).

| Directive class | Cache effect |
|---|---|
| All read-only R-bridges (`@search`, `@find`, `@repomap`, `@impact`, `@architecture`, `@outline`, `@smells`, `@review`, `@routes`, `@graph`, `@symbol`, `@inspect`, `@refactor *_preview`) | Never clears — read-only |
| `@edit` (and `@refactor *_apply`) | Clears `EngineContext.cache` on success; post-edit reads see the new bytes |
| `@reformat` | Clears on success (file mutated); BACKEND_REQUIRED/ERROR leaves cache warm |

### 1.4 Headless vs. IDE-only classes (spec §3.3)

| Class | Meaning | Directives |
|---|---|---|
| **Headless** | Works without a running IDE; CI-safe | All R-bridges except `@refactor`, `@reformat`, `@inspect` |
| **IDE-only** | Requires a running JetBrains plugin (lean-ctx plugin via HTTP/PSI); degrades to `BACKEND_REQUIRED` envelope headless | `@refactor`, `@reformat`, `@inspect` |

---

## 2. Directive Catalog

### 2.1 Base R-bridges — @search / @list / @env / @date / @count / @query / @graph

| Directive | Class | Routes to | Phase | Cache |
|---|---|---|---|---|
| `@read` | R | `ctx_read` (session-cached) | 1 | read-only |
| `@search` | R | `ctx_search` | 1 | read-only |
| `@list` | R | `ctx_tree` | 1 | read-only |
| `@env` | R | env var lookup | 1 | read-only |
| `@date` | R | system clock | 1 | read-only |
| `@count` | R | `ctx_search` (count mode) | 1 | read-only |
| `@query` | R (shell-gated) | `ctx_shell` | 1 | read-only |
| `@graph` | R | `graph_index` / `call_graph` | 3 | read-only |

`@query` requires `shell: allow` in the `@lean-md` header; denied otherwise
(`ShellDenied`). Shell commands must additionally pass the allowlist
(`ShellRejected`).

### 2.2 @edit (Phase 3.1) · @symbol (Phase 3.2)

| Directive | Class | Routes to | Phase | Cache |
|---|---|---|---|---|
| `@edit` | R (write) | `ctx_edit` (search-and-replace) | 3.1 | clears on apply |
| `@symbol` | R | `ctx_refactor` + `ctx_search action=symbol` | 3.2 | read-only |

`@edit` is the **only write directive**; it routes exclusively to `ctx_edit`,
never to native `Edit` or Serena (spec §4.5). Cache is cleared on every
successful apply so subsequent `@read`/`@graph` ops see the post-edit bytes.

`@symbol` ops: `refs` / `def` / `impl` / `find` / `overview`. LSP-backed
(`ctx_refactor` starts rust-analyzer); headless/CI-safe by default (no JetBrains
required for the rust-analyzer path).

### 2.3 Refactor & format — @refactor / @reformat / @inspect (Phase 3.3–3.4, IDE-only)

### `@refactor` (lmd Phase 3.3, two-phase structural refactoring — **IDE-only**)

Two-phase structural refactoring via `ctx_refactor` (spec §4.2). Four ops:
`rename` / `move` / `safe-delete` / `inline`. **Phase switch:** without `plan_hash=` the call
is a `*_preview` (compute the change plan); with `plan_hash=<hash>` it is the matching `*_apply`
(execute the previewed plan — the hash guards against a stale plan). IDE-only (spec §3.3):
headless, the backend returns a `BACKEND_REQUIRED` envelope, passed through verbatim.

| Op (first positional) | Required args | Routes to |
|---|---|---|
| `rename` | `new=<name>` | `rename_preview` / `rename_apply` |
| `move` | `target=<path>` **or** `parent=<symbol>` (exactly one) | `move_preview` / `move_apply` |
| `safe-delete` | — | `safe_delete_preview` / `safe_delete_apply` |
| `inline` | — | `inline_preview` / `inline_apply` |

Addressing (shared with `@reformat` via `bridges::addressing`): `name=Class/method` resolves
through `ctx_refactor`'s symbol index; `path=<P> line=N` targets a cursor position. For `@refactor`,
path addressing **requires** `line=` (`build_target`, `require_line=true`) — a position op needs a cursor.

Bare flags: `force` (override blocking conflicts — `rename`/`move`/`safe-delete` only; `inline` is not
forceable), `search-comments` / `search-text` (`rename`), `propagate` (`safe-delete`, also delete
newly-unreferenced deps), `keep-definition` (`inline`, keep the declaration).

**Cache coherence (spec §3.4):** the `*_apply` path mutates files, so on success the shared
`EngineContext` cache is cleared; a `*_preview` (or any `BACKEND_REQUIRED`/`ERROR` envelope) leaves
the files untouched → the warm cache is kept.

### `@reformat` (lmd Phase 3.4, single-phase IDE reformat — **IDE-only**)

Idempotent code-style reformat of one file via `ctx_refactor reformat`. IDE-only (spec §3.3):
without a running IDE the backend returns a `BACKEND_REQUIRED` envelope, passed through verbatim.
Convention (not enforced): lmd plans run `@reformat` before `git add` — never a hard render gate.

| Form | Routes to | Backing |
|---|---|---|
| `@reformat path=<P> [line=N] [optimize-imports]` | `ctx_refactor reformat` | **IDE-only** |
| `@reformat name=Class/method [optimize-imports]` | `ctx_refactor reformat` | **IDE-only** |

Addressing: `path=<P>` alone reformats the whole file; `path=<P> line=N` targets a region;
`name=Class/method` is resolved by `ctx_refactor`'s symbol index (no `path=` needed).
`optimize-imports` (bare flag) → also removes unused imports.

**Cache coherence (spec §3.4):** `@reformat` mutates the file, so on success the shared
`EngineContext` cache is cleared (the next `@read`/`@symbol`/`@graph` sees the reformatted bytes).
On the `BACKEND_REQUIRED`/`ERROR` envelope the file is untouched → the warm cache is kept.

### `@inspect` (lmd Phase 3.4, IDE inspections — **IDE-only**, read-only)

Diagnostics / profile inspections via `ctx_refactor inspections`. Read-only — never clears the
cache. IDE-only (spec §3.3): degrades to `BACKEND_REQUIRED` headless.

| Form | Routes to | Backing |
|---|---|---|
| `@inspect run <path>` | `ctx_refactor inspections mode=run` | **IDE-only** |
| `@inspect list` | `ctx_refactor inspections mode=list` | **IDE-only** |

`mode` is the first positional and defaults to `run`. `run` reports diagnostics for the given
file (path required). `list` reports the enabled inspections of the current project profile
(project-wide — no path).

### 2.4 Find & code-intel — @find / @repomap / @impact / @architecture / @outline (Phase 3.5, headless)

### `@find` (lmd Phase 3.5, semantic/hybrid search — headless)

Semantic / hybrid code search via `ctx_semantic_search`. Complements the regex `@search`.

| Form | Routes to |
|---|---|
| `@find query="…" [mode=bm25\|dense\|hybrid] [top_k=N] [path=<dir>]` | `ctx_semantic_search` |

`mode` without a value defaults to `bm25` (instant, no model-load) — the lmd headless default (a deliberate
lmd choice; the `ctx_semantic_search` backend default is `hybrid`); `mode=dense\|hybrid` loads the embedding
model. `path` defaults to the project root.

### `@repomap` (lmd Phase 3.5, PageRank repo map — headless)

PageRank-ranked symbol map for orientation in large plans, via `ctx_repomap`.

| Form | Routes to |
|---|---|
| `@repomap [focus=a.rs,b.rs] [max_tokens=N]` | `ctx_repomap` |

`focus` (comma-separated, surrounding `[]` tolerated) biases the ranking toward those files;
`max_tokens` fits the output to a budget.

### `@impact` (lmd Phase 3.5, blast-radius — headless)

Dependency blast-radius / chain via `ctx_impact` — the risk-gate before edits.

| Form | Routes to |
|---|---|
| `@impact analyze path=<P> [depth=N]` | `ctx_impact analyze` |
| `@impact chain path=<P>` | `ctx_impact chain` |

`action` is positional-0 (default `analyze`); `path` is required.

### `@architecture` (lmd Phase 3.5, structure views — headless)

Project-structure views via `ctx_architecture`.

| Form | Routes to |
|---|---|
| `@architecture [overview\|clusters\|layers\|cycles\|hotspots] [path=<sub>]` | `ctx_architecture` |

The view is positional-0 (default `overview`); optional `path` narrows the scope.

### `@outline` (lmd Phase 3.5, symbols + signatures — headless)

Symbols + signatures of one file via `ctx_outline`.

| Form | Routes to |
|---|---|
| `@outline path=<P> [kind=<filter>]` | `ctx_outline` |

`path` is required (positional-0 or `path=`); optional `kind` narrows the symbol kinds.

### 2.5 Quality — @smells / @review / @routes (Phase 3.6, headless)

### `@smells` (lmd Phase 3.6, code-smell detection — headless)

Code-smell detection over the property graph via `ctx_smells` (8 rules). Read-only — never clears the cache.

| Form | Routes to |
|---|---|
| `@smells [scan\|summary\|rules\|file] [rule=<name>] [path=<P>]` | `ctx_smells` |

`action` is positional-0 (default `scan` — a **deliberate lmd default**; the `ctx_smells` MCP wrapper defaults to `summary`, but a directive surfaces findings, not just counts, mirroring `@find`'s bm25 choice). `rule=` filters to one rule name; `path=` (an FS path, **jail-resolved**) narrows to one file (the natural arg for the `file` action). No `format=` is exposed — the backend default (`text`) applies (§5 "erben, nicht neu erfinden").

### `@review` (lmd Phase 3.6, automated code review — headless)

Automated review via `ctx_review`: the `review` action is **composite** — the backend fuses impact analysis, caller tracking, `@smells`, and test-discovery into a single verdict (one `@review` instead of four directives — the HOW-seam for `requesting-code-review`-style skills). Read-only — never clears the cache.

| Form | Routes to |
|---|---|
| `@review [review] path=<P> [depth=N]` | `ctx_review review` |
| `@review checklist [path=<P>] [depth=N]` | `ctx_review checklist` |
| `@review diff-review diff='<raw git diff>'` | `ctx_review diff-review` |

`action` is positional-0 (default `review`; brackets in the table mark the optional default action). **Path semantics branch per action:** `review`/`checklist` → `path=` is an FS path (**jail-resolved**); `diff-review` → `diff=` (or positional-1) is **raw diff text**, passed verbatim — **no git call**, **not jail-resolved** (the backend parses `+++ b/` / `diff --git a/`). `depth=` (default 3) bounds the impact analysis. A multi-line diff cannot ride a single-line directive arg, so standalone `diff-review` is intended for the Phase-4 pipe (`@query git diff | @review diff-review`); in Phase 3.6 it is a faithful pass-through.

### `@routes` (lmd Phase 3.6, HTTP route extraction — headless)

HTTP route/endpoint extraction via `ctx_routes` (Express / Flask / FastAPI / Actix / Spring / Rails / Next.js + axum). Read-only — never clears the cache.

| Form | Routes to |
|---|---|
| `@routes [method=GET\|POST\|…] [path=<route-prefix>]` | `ctx_routes` |

No `action` concept — `method=` and `path=` are **filters**. `path=` is an HTTP route **prefix** (e.g. `/api/users`), **not** a filesystem path — it is **not** jail-resolved. Without filters, all routes are listed. Tested headless via a temp-fixture match-router indexed on the fly (`ctx_impact build`), since `ctx_routes` needs an indexed file list and the crate root is not indexed under test.

#### Token framing (benchmark-calibrated, `mdai-benchmark.md`)

This family is **output/tool compression**: a dense Rust-computed verdict instead of the agent reading raw `clippy` / `grep` / `git diff` and reasoning. Per the benchmark that is the **marginal** (~10–15 %) lever, **not** the structural phase-isolation lever (the non-marginal win, which belongs to Phase 7 `@dispatch`). `@review diff-review` is the densest single case (it replaces agent-side raw-diff reading) but remains output compression, not a category change.

---

## 3. Argument Grammar (DirectiveArgs)

Directives are line-oriented: `@<name> [positional-0] [positional-1] [key=value] [bare-flag]`.

- **Positional args** are whitespace-delimited tokens before the first `key=` or bare flag.
- **Key-value args**: `key=value` — value extends to the next whitespace boundary (unquoted)
  or to the closing quote (quoted).
- **Double-quote strings** `"…"`: decode `\n`, `\t`, `\r`, `\"`, `\\`. Newlines within
  double-quoted values are embedded as `\n` characters (the directive is still one logical line).
- **Single-quote strings** `'…'`: verbatim — real embedded newlines are preserved as-is.
  Used for `diff=` in `@review diff-review`.
- **Bare flags** (no `=`): present/absent boolean. Examples: `force`, `optimize-imports`,
  `search-comments`.
- **`raw()`**: `DirectiveArgs::raw()` returns the unparsed tail — used by `@query` to pass
  the shell command verbatim.

Path args that name filesystem locations are jail-resolved via `resolve_tool_path`
against the engine's `jail_root`. Route prefixes (`@routes path=`) and raw diff
text (`@review diff-review diff=`) are **not** jail-resolved.

---

## 4. Test & Golden-Parity Strategy (spec §8)

Each bridge is covered at three levels:

| Level | What is tested | Location |
|---|---|---|
| **registered** | Bridge name appears in `default_registry()` | `bridges/mod.rs::default_registry_has_all_core_bridges` |
| **unknown-action** | Wrong op → `BridgeError::Resolve` with a `Use: …` list (not `unknown directive`) | per-bridge `mod tests` in `bridges/<name>.rs` |
| **headless smoke** | Full `render_body` round-trip: directive dispatches, output non-empty, no `unknown directive` | `engine.rs` gate tests |

**Self-repo goldens:** `@routes` is tested via a temp-fixture match-router indexed on the fly
(`ctx_impact build`) — positive: route surfaced with `route(s):`, negative: non-matching prefix
→ `No routes matching`. `@find` uses `mode=bm25` so the gate is instant (no model load).
`@outline` asserts the specific symbol name (`gate_outline_fn`).

Golden-parity tests run under `cargo nextest` (never `cargo test`) in cwd `rust/`.

---

## 5. Error Catalog (BridgeError)

| Variant | Meaning | Typical trigger |
|---|---|---|
| `MissingArg(&'static str)` | Required arg absent | `@impact` without `path=`, `@refactor rename` without `new=` |
| `Resolve(String)` | Arg value invalid or action unknown | Bad jail path, unknown op name (with `Use: …` hint) |
| `Io(String)` | File system error | Write failed, path not found |
| `DepthExceeded` | `@include` recursion past `max_chain_depth` (default 16) | Circular `@include` chain |
| `ShellDenied` | `@query` without `shell: allow` in header | Default header has `shell=deny` |
| `ShellRejected` | `@query` command not in the shell allowlist | Allowlist configured, command absent |

All `BridgeError` variants render as an HTML comment
`<!-- lmd: <error description> -->` in the output — never a panic.

---

## 6. Cross-references & Sources

- **Condensed agent reference:** [`appendix-lean-md.md`](appendix-lean-md.md)
- **MCP tool catalog:** [`appendix-mcp-tools.md`](appendix-mcp-tools.md)
- **MCP tool schemas (authoritative):** `rust/src/tools/registered/<tool>.rs`
- **Design spec:** `docs/lean-md/specs/2026-05-31-lmd-lean-ctx-native-design.mdai.md`
- **Phase plan:** `docs/lean-md/plans/2026-06-20-lmd-quality-phase-3-6.md`
- **Engine source:** `rust/src/lmd/engine.rs`, `rust/src/lmd/bridges/`
- **Journey 19 (JetBrains plugin):** [`19-jetbrains-plugin.md`](19-jetbrains-plugin.md) — parallel reference shape

## 7. E-constructs (Phase 4 — Macro-Engine, Container, Pipe)

### 7.1 Two-space model (spec §2.1)
The consumer sees ONE of two spaces. **Definition space** (`@define`, `@import`)
is consumed by a line-based pre-scan and emits nothing. **Output space**
(`@call`, surviving `@if`/`@consumer` branches, Pipe, leaf directives) renders.
A whole macro library can sit in a `.lmd` and never reach the agent — only the
expanded `@call` result does.

### 7.2 Pass-pipeline (spec §2.3)
`render_body` runs, in order: (1) `extract_definitions` — strip `@define`/`@import`
into `EngineContext.macros`; (3) `prune_containers` — evaluate `@if`/`@consumer`
to the winning branch (raw); then rushdown renders the result, where (2) `@call`
expansion + (4) leaf dispatch happen — `@call` re-enters `render_body`, so a
macro body's own `@define`/`@if` resolve recursively. Re-entrancy is depth-
guarded (`max_chain_depth = 16`).

### 7.3 `@define` / `@call` / `{{ param }}` (E-#4, Phase 4A)
`@define name(p1, p2)` … `@define-end` registers a macro (forward references are
free — every define is registered before any call renders). `@call name(args) /`
looks it up, substitutes `{{ p }}` textually into the body, binds params as
evalexpr vars, and re-renders the body. A **passive** macro body is markdown;
an **active** (workflow) body is directives (`@reformat`/`@query`/…) — only the
dense result is emitted, the interna stay in the definition space.

### 7.4 `@if` / `@consumer` + evalexpr (E-#3, Phase 4B)
Branches evaluate top-to-bottom; first true wins, else `@else`, else empty.
`@consumer X` is sugar for `@if consumer == "X"`. Variables: `consumer`,
`version`, `shell`, `env.NAME`, bound params. No I/O — deterministic goldens.
An eval error skips the container (visible comment); an unterminated block emits
a comment and the render continues (spec §7).

### 7.5 Pipe / `@render` (E-#5, Phase 4C)
A single ` | ` per line: `@A args | @B args`. The left runs, its output is
injected as the right's `piped_input`, the right runs — **only the right is
visible** (the raw intermediate is suppressed; that is the lever). The right
bridge must `accepts_pipe()` (else a visible error). Built-in sinks: `@render
type=table|list` (deterministic formatters) and `@review diff-review` (reads the
piped diff) — making `@query git diff | @review diff-review` (v1 §4.4) real.
Pipe chains, custom `@render` types → out of scope (spec §10/§9).

### 7.6 Forward seams (spec §9)
The three resolution chains each reserve an additive Phase-5 extension tier
(`@call`→plugin tools, `@render type=<custom>`→WASM transform, `@import`→macro
pack). The `orient` built-in macro + render-call-overridable `consumer` are
Phase 6. None are built in Phase 4.
