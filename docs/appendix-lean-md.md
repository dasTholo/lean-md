# Appendix — lean-md (lmd) Directives (Agent Reference)

> Condensed companion to `21-lean-md.md`. `@directive` calls inside `.lmd` /
> `.lean-md` files route through the lmd bridge layer to a `ctx_*` core tool.

## Coordinates & invocation

- Directives are line-oriented: `@<name> [positional] [key=value] [bare-flag]`.
- Args: positional + `key=value`; double-quote `"…"` decodes `\n \t \r \" \\`;
  single-quote `'…'` is verbatim (real newlines preserved).
- Project root = the engine's `jail_root`. FS-path args are jail-resolved;
  route prefixes and raw diff text are not.

## Directive catalog

| Directive                                                               | Action / form                                                 | Routes to                   | Class        | Cache               |
|-------------------------------------------------------------------------|---------------------------------------------------------------|-----------------------------|--------------|---------------------|
| `@search` / `@list` / `@env` / `@date` / `@count` / `@query` / `@graph` | (base R-bridges)                                              | various `ctx_*`             | R            | read-only¹          |
| `@edit`                                                                 | text / symbolic body edits                                    | `ctx_edit` / `ctx_refactor` | R            | clears on apply     |
| `@symbol`                                                               | refs/def/impl/overview/…                                      | `ctx_refactor`+`ctx_search action=symbol` | R            | read-only           |
| `@refactor`                                                             | rename/move/safe-delete/inline (2-phase)                      | `ctx_refactor`              | R (IDE-only) | clears on `*_apply` |
| `@reformat`                                                             | reformat one file                                             | `ctx_refactor reformat`     | R (IDE-only) | clears on success   |
| `@inspect`                                                              | run/list inspections                                          | `ctx_refactor inspections`  | R (IDE-only) | read-only           |
| `@find`                                                                 | `query=` `[mode=bm25\|dense\|hybrid]` `[top_k=]` `[path=]`    | `ctx_semantic_search`       | R            | read-only           |
| `@repomap`                                                              | `[focus=]` `[max_tokens=]`                                    | `ctx_repomap`               | R            | read-only           |
| `@impact`                                                               | `analyze\|chain path= [depth=]`                               | `ctx_impact`                | R            | read-only           |
| `@architecture`                                                         | `overview\|clusters\|layers\|cycles\|hotspots [path=]`        | `ctx_architecture`          | R            | read-only           |
| `@outline`                                                              | `path= [kind=]`                                               | `ctx_outline`               | R            | read-only           |
| `@smells`                                                               | `scan\|summary\|rules\|file [rule=] [path=]` (default `scan`) | `ctx_smells`                | R            | read-only           |
| `@review`                                                               | `review\|diff-review\|checklist` (default `review`)           | `ctx_review`                | R            | read-only           |
| `@routes`                                                               | `[method=] [path=<route-prefix>]` (filters)                   | `ctx_routes`                | R            | read-only           |

¹ `@query` is shell-gated (`shell: allow` in `@lean-md` header); see §21.
² `@call <plugin_tool>` is extension-gated (`extensions: allow` in `@lean-md`),
deny-by-default — disabled tools surface a visible message, never a subprocess.
Sandbox/trust are inherited from the plugin's `[trust]` manifest.

## E-constructs (Macro-Engine / Container / Pipe — Phase 4)

These are rushdown **engine primitives** (class **E**), not `ctx_*` bridges. The
multi-line forms resolve in line-based pre-passes inside `render_body` BEFORE
rushdown (spec §2.2/§2.3); `@call`/`@render`/Pipe are render-time.

| Construct | Form | Class | Visibility |
|---|---|---|---|
| `@define` | `@define name(p1, p2)` … `@define-end` | E | **invisible** (definition space) |
| `@import` | `@import <lib> /` | E | **invisible** (loads jailed `<lib>.lmd.md` macros) |
| `@call` | `@call name(arg1, arg2) /` | E | visible (macro body, or gated plugin `[[tools]]` output²) |
| `{{ param }}` | inside a macro body | E | visible (textual substitution) |
| `@if`/`@elseif`/`@else` | `@if <expr>` … `@if-end` | E | visible (winning branch only) |
| `@consumer` | `@consumer <ai\|human>` … `@consumer-end` | E | sugar for `@if consumer == "<v>"` |
| `{{ expr }}` | inline `{{ consumer }}`, `{{ env.CI == "true" }}` | E | visible (param → header var → evalexpr) |
| Pipe | `@A args \| @B args` (single pipe) | E | visible (**right only**; left consumed) |
| `@render` | `@render type=table\|list` (pipe sink) | E | visible (formats `piped_input`) |

### Condition variables (`@if` / `{{ expr }}`)

`consumer` (ai/human), `version`, `shell` (allow/deny) — from the `@lean-md`
header; `env.NAME` — process env; bound **macro-params** — in the current
`@call` scope. **No I/O** in conditions (spec §4/§10): existence/structure are
output directives (`@search`/`@list`/…), never booleans.

### Resolution chains (spec §6)

- `@call name(…)` : authored/`@import`'d `@define` → registered **plugin `[[tools]]`** (only with `@lean-md extensions=allow`) → error. Built-in-first: a same-named macro always wins.
- `@import` / `@include` : built-in fragment → jailed `*.lmd.md` → error.
- `@render type=X` : built-in formatter (`table`/`list`) → registered **`RenderTransform`** (custom types, `wasm` feature) → clear degradation message. No header gate (sandboxed).
- `{{ … }}` inline : registered directive → bound var/param → evalexpr.

### New error codes (added to the BridgeError list)

`macro not found: X` (`@call`) · `unterminated @define`/`@if`/`@consumer` (pre-pass
comment) · `@if eval err` (container skipped) · `@X does not accept piped input`
(pipe into non-accepting bridge) · `MissingArg("piped_input")` (`@render` without pipe).

## 3.6 args & defaults (full)

- **`@smells`** `action` positional-0 default `scan` (lmd default; wrapper default is `summary`); `rule=`/`path=`
  optional; `path` jail-resolved; no `format=`.
- **`@review`** `action` positional-0 default `review`; `review`/`checklist` `path=` = FS path (jail-resolved);
  `diff-review` `diff=`/positional-1 = raw diff text (verbatim, no git, no jail); `depth=` default 3. `review` is
  composite (impact+callgraph+smells+tests).
- **`@routes`** no action; `method=`/`path=` filters; `path=` = route prefix (not FS, no jail).

## Guards (short form)

- Read-only directives never call `cache.clear()`.
- FS-path args pass through `resolve_tool_path` (jail); route prefixes / diff text do not.
- Unknown action on `@smells`/`@review` → `BridgeError::Resolve` with a `Use: …` list.

## Error codes (BridgeError)

`MissingArg` · `Resolve` · `Io` · `DepthExceeded` · `ShellDenied` · `ShellRejected`.

## See also

- Full reference: `21-lean-md.md`
- MCP tool schemas: `appendix-mcp-tools.md`, `rust/src/tools/registered/<tool>.rs`
- Design spec: `docs/lean-md/specs/2026-05-31-lmd-lean-ctx-native-design.mdai.md`
