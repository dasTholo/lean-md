# TODO

## FIXME BUG: `lean-ctx call ctx_tools` (CLI) paniced — kein Tokio-Reactor

**Status:** offen — blockiert den CLI-basierten Live-Gate #4 (`addon_roundtrip.rs`).
**Repo:** lean-ctx (lokaler Build `3.8.12-lmd`, branch `feat-lmd-v1`).
**Ort:** `rust/src/tools/ctx_tools.rs:37`.
**Entdeckt:** 2026-06-26 (lmd-v2-Finalisierung, Task 4).

### Symptom

```
lean-ctx call ctx_tools --project-root /home/tholo/Scripts/lean-md --json \
  '{"action":"call","tool":"lean-md::ctx_md_render","arguments":{"path":"…lmd.md"}}'
→ lean-ctx: unexpected error
  Details: there is no reactor running, must be called from the context of a Tokio 1.x runtime
  Location: src/tools/ctx_tools.rs:37
  [exit:1]
```

Der CLI-`call`-Pfad ruft `ctx_tools` außerhalb eines Tokio-Runtimes auf (vermutlich
`block_on`/`Handle::current()` ohne aktiven Reactor). Der MCP-Pfad ist nicht betroffen
(läuft bereits im async-Runtime).

### Fix-Richtung

- In `ctx_tools.rs:37` den Gateway-Call innerhalb eines Tokio-Runtimes ausführen
  (eigenes `Runtime::new()?.block_on(...)` im CLI-Pfad, oder den CLI-`call`-Dispatch
  generell in einen Runtime wrappen).
- Danach: `lean-ctx call ctx_tools {"action":"call","tool":"lean-md::ctx_md_render",…}`
  muss byte-identisch zu `lean-md render <file>` liefern.

---

## Folge: Test `addon_roundtrip.rs` nutzt den falschen Aufrufweg

**Repo:** lean-md. **Datei:** `tests/addon_roundtrip.rs` (`#[ignore]`).

Der Test ruft `lean-ctx call ctx_md_render` direkt auf — das ist **nicht** der
Gateway-Weg. Addon-Tools hängen am **`ctx_tools`-Downstream-Gateway**, nicht am
`ctx_call`/`ctx_discover_tools`-Router (die nur lean-ctx-eigene 77 Tools abdecken).

Korrekter Aufruf (sobald obiger CLI-Bug gefixt ist):

```
lean-ctx call ctx_tools --project-root . --json \
  '{"action":"call","tool":"lean-md::ctx_md_render","arguments":{"path":"<f>.lmd.md"}}'
```

- [ ] Nach CLI-Fix: `via_leanctx_call` in `addon_roundtrip.rs` auf den
      `ctx_tools action=call`-Pfad umstellen (Tool-Handle `lean-md::ctx_md_render`).
- [ ] Dann `cargo nextest run --test addon_roundtrip --run-ignored ignored-only` grün.
- [ ] Spec §5.1.2 mit echtem Nachweis aktualisieren.

---

## Verifiziert OK (kein Bug)

- Gateway **funktioniert** via MCP `ctx_tools`:
  - `ctx_tools action=list` → `lean-md [stdio, enabled] — 2 tool(s)`.
  - `ctx_tools action=find query="render lmd markdown"` → listet
    `lean-md::ctx_md_render` + `lean-md::ctx_md_check`.
  - `ctx_tools action=call tool="lean-md::ctx_md_render" {"path":…}` rendert
    **byte-identisch** zu `lean-md render <file>` → **Gate #4 funktional bestätigt
    (MCP-Gateway-Pfad)**.
- Addon installiert+enabled+capabilities-korrekt; `lean-md mcp` JSON-RPC korrekt.
- Namespacing: Gateway-Handle ist `lean-md::ctx_md_render` (Prefix bestätigt, live).

### Verweise

- lean-ctx: `rust/src/tools/ctx_tools.rs:37`
- lean-md: `tests/addon_roundtrip.rs`, Spec §5.1.1/§5.1.2
