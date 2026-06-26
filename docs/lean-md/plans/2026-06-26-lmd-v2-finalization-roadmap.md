# lean-md v2 Finalisierung — Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Die offene Arbeit aus dem Spec `2026-06-26-lean-md-standalone-addon-design.md` liefern — Produkt-Dokumentation (README + INSTALL.md), Korrektheits-Nachweis (Namespacing-Befund + Live-E2E-Gates #4/#5 grün) und notierte Zukunftspfade — ausschließlich über lean-ctx-Tooling.

**Architecture:** Reine Finalisierung eines fertig dekoppelten Addons (kein neuer Render-/Backend-Code). Drei Stränge: (1) Doku als Markdown via `ctx_edit`/`Write`, mit real ausgeführten CLI-Beispielen als Verifikation; (2) Verifikation der Addon-Boundary (Namespacing + Live-Roundtrip) durch lokale Installation + nextest-`--run-ignored`; (3) Doku der Upstream-Folgevorschläge. Jede `.rs`-Berührung läuft über `ctx_refactor` (Symbol-Edits + `action=reformat` als Pre-Commit), jede Nicht-Rust-Datei über `ctx_edit`.

**Tech Stack:** Rust (Crate `lean-md` = lib `lean_md` + bin `lean-md`), `cargo nextest`, lean-ctx-CLI (`addon`/`call`), lean-ctx MCP-Tools (`ctx_read`/`ctx_edit`/`ctx_refactor`/`ctx_symbol`/`ctx_search`/`ctx_shell`/`ctx_tree`).

## Global Constraints

- **lean-ctx-Tool-Disziplin (verbindlich, jede Task):**
  - Lesen → `ctx_read(path, mode=full|signatures|map|lines:N-M)`; **nie** native `cat`/`Read` im Jail.
  - Suchen → `ctx_search(pattern, path)`; Struktur → `ctx_tree`; Navigation → `ctx_symbol`.
  - **Editieren Nicht-Rust** (`.md`/`.toml`/`.json`) → `ctx_edit(path, old_string, new_string)`. Neue Dateien → `Write` (oder `ctx_edit create=true`).
  - **Editieren Rust** (`.rs`) → `ctx_refactor` Symbol-Edits (`replace_symbol_body`/`insert_*`, name_path-adressiert); **nie** `sed`/`awk`/native Edit.
  - **Pre-Commit pro berührter `.rs`-Datei** → `ctx_refactor action=reformat path=<file>` (lean-ctx-idiomatisch). Für Nicht-Rust kein Reformat nötig.
  - Shell → `ctx_shell(command=…)`. **Kein** `&&`/`||`/`;`-Chaining — ein Befehl pro Aufruf; statt `cd <dir> && cargo …` immer `cargo … --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml`.
  - Deferred-Tool-Reflex: zeigt ein MCP-Tool deferred → `ToolSearch(query="select:<exact_tool_name>")` FIRST, dann direkt aufrufen. **Nie** `ctx_call`-Wrapper, **nie** Bash-Workaround vor ToolSearch.
- **Tests — cargo nextest knowledge (Hard-Rule + gotcha `cargo-nextest-usage-lean-md`):**
  - Immer `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml`, **nie** `cargo test`.
  - **Ignored-Tests:** `--run-ignored ignored-only` (oder `all`) — **NICHT** libtest's `-- --ignored` (nextest versteht das nicht).
  - **Doctests:** nextest führt sie **nicht** aus → bei Bedarf separat `cargo test --doc --manifest-path …`.
  - MCP-Backend-Tests: `--features mcp`. Einzelne Datei: `--test <name>`. Filter: `-E 'test(name)'`.
- **Determinismus (#498):** Doku-Beispiele müssen byte-stabil rendern; keine Timestamps/Counter in Tool-Output-Bodies.
- **Version-SSOT:** `Cargo.toml` + `lean-ctx-addon.toml` beide `0.1.0` — bei Bumps synchron halten.
- **Sprache:** Doku-Prosa **Deutsch** mit Umlauten; Code/Code-Kommentare **Englisch**. (README/INSTALL sind nutzerseitige Doku → **Englisch**, konsistent mit der bestehenden README.)
- **Branch:** direkt auf dem aktuellen Branch (`feat-lmd-v2`); **keine** Worktrees.

**Vorbedingungen / Kontext (verifiziert, Plan-Zeit 2026-06-26):**

- Spec: `docs/lean-md/specs/2026-06-26-lean-md-standalone-addon-design.md` (IST §2–§4, Roadmap §5).
- IST-README ist install-fokussiert; `INSTALL.md` fehlt; `LICENSE` (Apache-2.0) vorhanden; `docs/CONTRACT.md` vendored.
- CLI-Surface (`src/bin/lean_md.rs`): `render <file> [--consumer=human|ai] [--crp=off|compact|tdd] [-o out.md]`, `check <file>`, `mcp`.
- Direktiven-Glossar: `content/gloss/directives.lmd.md`.
- Tests: `tests/{standalone,backend_parity,determinism,addon_roundtrip}.rs`; `addon_roundtrip` ist `#[ignore]`.
- **Offen (separat, nicht Teil dieses Plans):** uncommittete `src/bin/lean_md.rs`-fmt-Änderung; uncommittete lean-ctx-Angleichung (`addon_registry.json` `version:0.1.0`, P4-Plan/decoupling-Spec `2.0.0→0.1.0`).

## File Structure

```
lean-md/
  README.md          # Task 1 — Produkt-README (was/CLI/Direktiven/Quickstart/Skills + bestehende Install/Backend/Kontrakt)
  INSTALL.md         # Task 2 — abgeleitete Install-Tiefe (Voraussetzungen, Pfade, Server-Neustart, addon-add-Config §3.4, Troubleshooting, Backend)
  docs/lean-md/
    FOLLOWUPS.md     # Task 5 — Upstream-Folgevorschläge (Host-Callback §3.4, McpBackend-Reife, Namespacing-Ticket falls Prefix)
    specs/2026-06-26-lean-md-standalone-addon-design.md   # Task 3/4 — §5.1.1/§5.1.2 Befunde nachtragen
  tests/
    addon_roundtrip.rs   # Task 4 — Tool-Name ggf. an Namespacing-Befund anpassen (ctx_edit/ctx_refactor)
```

---

### Task 1: README zur Produkt-README erweitern (§5.2.1, R1)

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: CLI-Surface aus `src/bin/lean_md.rs`; Direktiven aus `content/gloss/directives.lmd.md`.
- Produces: README mit Abschnitten „What is lean-md", „CLI", „Directives", „Quickstart", „Skills" zusätzlich zu den bestehenden (Install/Backend/Manifest).

- [ ] **Step 1: README + Glossar lesen (Kontext)**

Run: `ctx_read(path="/home/tholo/Scripts/lean-md/README.md", mode="full")`
Run: `ctx_read(path="/home/tholo/Scripts/lean-md/content/gloss/directives.lmd.md", mode="full")`
Expected: aktuelle Struktur (Install/Backend/Manifest) + Direktiven-Liste bekannt.

- [ ] **Step 2: „What is lean-md" + „CLI" nach dem Intro einfügen**

`ctx_edit` — `old_string` = der erste Abschnitt bis vor `## Install as a lean-ctx addon`; davor einfügen:

```markdown
## What it does

`.lmd.md` is Markdown with directives. The render core (`rushdown` + `evalexpr`)
evaluates macros, conditionals (`@if`/`@consumer`), expressions (`{{ }}`) and
layout fully in-process — a code-intel-free document renders standalone, with no
running lean-ctx. Code-intel directives (`@edit`, `@symbol`, `@refactor`, `@graph`,
…) are dispatched **outbound** to lean-ctx via the `CodeIntelBackend` (CLI default,
MCP opt-in), so lean-md never parses code locally.

## CLI

```sh
lean-md render <file.lmd.md> [--consumer=human|ai] [--crp=off|compact|tdd] [-o out.md]
lean-md check  <file.lmd.md>
lean-md mcp                      # stdio JSON-RPC 2.0 MCP server (ctx_md_render / ctx_md_check)
```

- `render` evaluates the document and prints Markdown (`-o` writes to a file).
  `--consumer=human` narrates directives as prose; `--crp` selects the output
  density (token-compressed rendering protocol).
- `check` parse-checks a source and reports header config + directive count.
- `mcp` serves `ctx_md_render` / `ctx_md_check` over stdio — this is the entry
  point the addon wiring spawns (`command = "lean-md"`, `args = ["mcp"]`).
```

- [ ] **Step 3: „Directives" + „Quickstart" + „Skills" einfügen**

`ctx_edit` — direkt nach dem in Step 2 eingefügten CLI-Block anfügen:

```markdown
## Directives (overview)

Render/expression: `@if` / `@consumer`, `{{ expr }}`, `@define` / `@call` / `@import`,
pipes + `@render`. Read/search: `@read`, `@search`, `@list`, `@query`, `@find`,
`@count`, `@env`, `@date`. Code-intel (outbound): `@edit`, `@symbol`, `@refactor`,
`@reformat`, `@inspect`, `@graph`, `@repomap`, `@impact`, `@architecture`, `@outline`,
`@smells`, `@review`, `@routes`. Workflow: `@phase`, `@dispatch`, `@handoff`,
`@remember`, `@recall`.

Full gloss: [`content/gloss/directives.lmd.md`](content/gloss/directives.lmd.md).

## Quickstart

```sh
cat > demo.lmd.md <<'EOF'
@lean-md
consumer: ai

@if consumer=ai
Hello {{ consumer }} — this rendered standalone, no lean-ctx needed.
@if-end
EOF
lean-md render demo.lmd.md
```

## Skills

lean-md ships an embedded pilot skill (`content/skills/lmd-brainstorm/`). The MCP
server renders it on demand via `ctx_md_render` with `skill=<name>` / `phase=<name>`
addressing against the binary-embedded body.
```

- [ ] **Step 4: Quickstart-Beispiel real ausführen (Verifikation)**

Run: `ctx_shell(command="printf '@lean-md\nconsumer: ai\n\n@if consumer=ai\nHello {{ consumer }} — standalone.\n@if-end\n' > /tmp/claude-1000/-home-tholo-Scripts-lean-md/cedbef88-23f2-471d-a4ce-148bfe5b1091/scratchpad/demo.lmd.md")`
Run: `ctx_shell(command="cargo run --quiet --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml -- render /tmp/claude-1000/-home-tholo-Scripts-lean-md/cedbef88-23f2-471d-a4ce-148bfe5b1091/scratchpad/demo.lmd.md")`
Expected: Output enthält `Hello ai — standalone.` (das `{{ consumer }}` ist aufgelöst, der `@if`-Block ist sichtbar). Falls der gerenderte Text abweicht → README-Beispiel an den realen Output angleichen (Doku darf nicht lügen).

- [ ] **Step 5: Commit**

Run: `ctx_shell(command="git -C /home/tholo/Scripts/lean-md add README.md")`
Run: `ctx_shell(command="git -C /home/tholo/Scripts/lean-md commit -m 'docs(R1): README — product overview, CLI, directives, quickstart, skills'")`

---

### Task 2: INSTALL.md ableiten (§5.2.2, R1)

**Files:**
- Create: `INSTALL.md`
- Modify: `README.md` (Verweis auf INSTALL.md)

**Interfaces:**
- Consumes: Addon-Install-Mechanik (Spec §3.4); Backend-Env-Vars aus README.
- Produces: `INSTALL.md` als Detail-Anleitung; README verlinkt sie.

- [ ] **Step 1: INSTALL.md schreiben**

`Write` `/home/tholo/Scripts/lean-md/INSTALL.md`:

```markdown
# Installing lean-md

lean-md is distributed as a lean-ctx **addon**: a standalone MCP server that the
lean-ctx gateway spawns. This guide is the detailed companion to the README's
quick install section.

## Prerequisites

- `lean-ctx >= 3.8.13` on `PATH` (the addon ecosystem + `lean-ctx addon` CLI).
- A Rust toolchain (`cargo`) to build/install from source.

## Path A — from the registry (once listed)

```sh
lean-ctx addon add lean-md
```

## Path B — from a local clone

```sh
cargo install --path .                       # puts `lean-md` on PATH
lean-ctx addon add ./lean-ctx-addon.toml     # wire it into the gateway
```

## Restart the MCP client/server

**Required after `addon add`:** restart your MCP client/server so the gateway
re-reads its catalog and the lean-md tools (`ctx_md_render` / `ctx_md_check`)
become visible. This is the most common "tool not found" cause.

## What `addon add` writes (you maintain nothing by hand)

Installation is automatic and global-only: it upserts a `[[gateway.servers]]`
entry into your global lean-ctx config (`command = "lean-md"`, `args = ["mcp"]`,
plus the declared `[capabilities]`) and records the install in
`<data_dir>/addons/installed.json`. `lean-ctx addon remove lean-md` unwinds both.

## Verify

```sh
lean-ctx call ctx_md_render --project-root . --json '{"path": "demo.lmd.md"}'
```

Expected: the rendered Markdown of `demo.lmd.md`, identical to `lean-md render demo.lmd.md`.

## Backend selection (optional — defaults to zero-config CLI)

lean-md is zero-config: by default it shells out to `lean-ctx` per code-intel
directive (`CliBackend`). To use the warm MCP backend instead, set environment
variables (build with the `mcp` feature):

| Variable | Value | Effect |
| `LEAN_MD_BACKEND` | `mcp` | opt into the MCP backend (any other value → CLI default) |
| `LEAN_MD_MCP_ENDPOINT` | e.g. `http://localhost:3100` | base URL of a lean-ctx MCP/HTTP endpoint running `tool_profile = power` |

A malformed/unreachable endpoint falls back to the CLI backend — it never bricks
rendering. See the README's "Backend selection" table for details.

## Troubleshooting

- **Tools not visible** → restart the MCP client/server (catalog re-read).
- **Tool name has a `lean-md::` prefix** → see the gateway namespacing note in the
  README; the addon path still works, only the visible tool name differs.
```

- [ ] **Step 2: README auf INSTALL.md verweisen**

`ctx_edit` README — im Abschnitt `## Install as a lean-ctx addon`, nach dem lokalen-Klon-Block, vor dem Restart-Blockquote einfügen:

```markdown
> For prerequisites, the full local-build flow, what `addon add` writes, and
> troubleshooting, see [`INSTALL.md`](INSTALL.md).
```

- [ ] **Step 3: Verifikation — Links + Markdown**

Run: `ctx_read(path="/home/tholo/Scripts/lean-md/INSTALL.md", mode="full")`
Expected: Datei existiert, keine `TBD`/Platzhalter, der Verify-Befehl referenziert `demo.lmd.md` aus dem README-Quickstart.

- [ ] **Step 4: Commit**

Run: `ctx_shell(command="git -C /home/tholo/Scripts/lean-md add INSTALL.md README.md")`
Run: `ctx_shell(command="git -C /home/tholo/Scripts/lean-md commit -m 'docs(R1): INSTALL.md — prerequisites, install paths, addon-add config, troubleshooting'")`

---

### Task 3: Namespacing-Befund verifizieren (§5.1.1, R2)

**Files:**
- Modify: `docs/lean-md/specs/2026-06-26-lean-md-standalone-addon-design.md` (§5.1.1 Befund nachtragen)

**Interfaces:**
- Consumes: installiertes Addon (Task 2 Path B).
- Produces: dokumentierter Befund `transparent` | `lean-md::`-Prefix; entscheidet, ob Task 4 Test-Toolnamen anpassen muss.

- [ ] **Step 1: Addon lokal installieren**

Run: `ctx_shell(command="cargo install --path /home/tholo/Scripts/lean-md")`
Expected: `lean-md` installiert (Binary auf PATH).
Run: `ctx_shell(command="lean-ctx addon add /home/tholo/Scripts/lean-md/lean-ctx-addon.toml -y")`
Expected: Manifest geparst, `[[gateway.servers]]`-Eintrag geschrieben, gateway enabled. (Flag `-y` gegen `lean-ctx addon --help` gegenprüfen, falls die nicht-interaktive Bestätigung anders heißt.)

- [ ] **Step 2: MCP-Server/-Client neu starten** (manuell, §5.2)

> Dieser Schritt ist nicht skriptbar: den MCP-Client/-Server neu starten, damit der Gateway-Katalog neu gelesen wird. Ohne Neustart sind die lean-md-Tools nicht sichtbar.

- [ ] **Step 3: Realen Tool-Namen ermitteln**

Run: `ctx_shell(command="lean-ctx call ctx_tools --project-root /home/tholo/Scripts/lean-md --json '{\"action\": \"find\", \"query\": \"md_render\"}'")`
Expected: der Katalog listet entweder `ctx_md_render` (transparent) **oder** `lean-md::ctx_md_render` (Prefix). (Exakte `ctx_tools`-Arg-Form gegen `appendix-mcp-tools` bzw. `lean-ctx call ctx_tools --help` gegenprüfen.)

- [ ] **Step 4: Befund in den Spec eintragen (§5.1.1)**

`ctx_edit` Spec — im Abschnitt §5.1.1 nach dem „Verifikation"-Bullet einen Befund-Satz ergänzen:

```markdown
- **Befund (2026-06-26):** Der Gateway exponiert das Tool als
  `<ctx_md_render | lean-md::ctx_md_render>` — <transparent, rückwärtskompatibel
  | Prefix erzwungen → Task 4 passt die Test-Toolnamen an + Folgeticket §FOLLOWUPS>.
```

(Die `<…>`-Platzhalter durch den realen Befund ersetzen — kein `<…>` im Commit.)

- [ ] **Step 5: Commit**

Run: `ctx_shell(command="git -C /home/tholo/Scripts/lean-md add docs/lean-md/specs/2026-06-26-lean-md-standalone-addon-design.md")`
Run: `ctx_shell(command="git -C /home/tholo/Scripts/lean-md commit -m 'docs(R2): record gateway namespacing finding (§5.1.1)'")`

---

### Task 4: Live-E2E-Gates #4/#5 grün fahren (§5.1.2, R3)

**Files:**
- Modify: `tests/addon_roundtrip.rs` (nur falls Task 3 einen Prefix ergab)
- Modify: `docs/lean-md/specs/2026-06-26-lean-md-standalone-addon-design.md` (§5.1.2 Nachweis-Notiz)

**Interfaces:**
- Consumes: installiertes Addon (Task 3), Namespacing-Befund (Task 3).
- Produces: `addon_roundtrip` läuft real grün (oder dokumentierter Befund); CI-Strategie notiert.

- [ ] **Step 1: Tool-Name im Test an den Befund angleichen (nur bei Prefix)**

Falls Task 3 `lean-md::ctx_md_render` ergab: `ctx_read(path="/home/tholo/Scripts/lean-md/tests/addon_roundtrip.rs", mode="full")`, dann `ctx_edit` den Tool-Namen-String im `via_leanctx_call`-Aufruf (`"ctx_md_render"` → `"lean-md::ctx_md_render"`). Bei transparentem Namespace: **kein** Edit, weiter zu Step 2.
> Hinweis: Tool-Namen sind String-Literale → `ctx_edit`. Würde der Test eine benannte Konstante/Funktion umbenennen, liefe das über `ctx_refactor` (Symbol-Edit) + danach `ctx_refactor action=reformat path=tests/addon_roundtrip.rs` als Pre-Commit.

- [ ] **Step 2: Ignored-Roundtrip-Gate real laufen (#4)**

Run: `ctx_shell(command="cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml --test addon_roundtrip --run-ignored ignored-only")`
Expected: `addon_render_matches_direct_render` PASS (Addon-Pfad byte-identisch zu `lean-md render`). **Nicht** `-- --ignored` verwenden — nextest braucht `--run-ignored`.

- [ ] **Step 3: Delegation-Gate prüfen (#5, lean-md-Seite)**

Run: `ctx_shell(command="lean-ctx call ctx_read --project-root /home/tholo/Scripts/lean-md --json '{\"path\": \"demo.lmd.md\", \"mode\": \"auto\"}'")`
Expected: mit installiertem Addon == direktes `ctx_md_render` (delegiert); der `@if`-Block ist gerendert, nicht roh. (Das spiegelt das lean-ctx-seitige `auto_render_delegation`-Gate #5 von der Nutzerseite.)

- [ ] **Step 4: Nachweis-Notiz in den Spec (§5.1.2)**

`ctx_edit` Spec — in §5.1.2 nach dem „Akzeptanz"-Bullet ergänzen:

```markdown
- **Nachweis (2026-06-26):** #4 `addon_roundtrip` grün via
  `cargo nextest run --test addon_roundtrip --run-ignored ignored-only` nach
  `cargo install --path .` + `addon add` + Server-Neustart. #5 Delegation
  bestätigt via `lean-ctx call ctx_read` (== `ctx_md_render`). CI-Strategie:
  opt-in Integrations-Job (install + restart skripten) ODER als „manuell
  verifiziert" dokumentiert — nie stilles `ignored`.
```

- [ ] **Step 5: Volle Suite als Regressions-Gate**

Run: `ctx_shell(command="cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml")`
Expected: alle nicht-ignorierten Tests grün (`standalone` #2, `determinism` #7, portierte §8-Direktiven-Tests); `addon_roundtrip` #4 ohne `--run-ignored` als `ignored` gelistet (erwartet).
Run: `ctx_shell(command="cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml --features mcp -E 'test(backend_parity)' --run-ignored all")`
Expected: `backend_parity` #3 (Cli==Mcp) — grün, oder `ignored` ohne erreichbaren Endpoint (dann Befund notieren, kein Fail).

- [ ] **Step 6: Commit**

Run: `ctx_shell(command="git -C /home/tholo/Scripts/lean-md add tests/addon_roundtrip.rs docs/lean-md/specs/2026-06-26-lean-md-standalone-addon-design.md")`
Run: `ctx_shell(command="git -C /home/tholo/Scripts/lean-md commit -m 'test(R3): live addon-roundtrip #4 + delegation #5 verified; record proof'")`
> Falls Step 1 keinen Test-Edit erzeugte, `tests/addon_roundtrip.rs` aus dem `add` weglassen.

---

### Task 5: Zukunftspfade notieren (§5.3, R4)

**Files:**
- Create: `docs/lean-md/FOLLOWUPS.md`

**Interfaces:**
- Consumes: §5.3 des Specs + ggf. Namespacing-Folgeticket aus Task 3.
- Produces: `FOLLOWUPS.md` als Sammelpunkt der Upstream-Folgevorschläge (kein Code).

- [ ] **Step 1: FOLLOWUPS.md schreiben**

`Write` `/home/tholo/Scripts/lean-md/docs/lean-md/FOLLOWUPS.md`:

```markdown
# lean-md — Upstream-Folgevorschläge (kein v2-Scope)

Notierte Optionen aus dem Spec §5.3 — bewusst **kein** Blocker, nur dokumentiert.

## Host-Callback (Single-Server-Reinheit, §3.4)

Echte „Ein-Prozess"-Reinheit erforderte eine neue lean-ctx-Fähigkeit: das Gateway
injiziert beim Spawn den Host-MCP-Endpoint in die Addon-Env (`[mcp].env`, z. B.
`LEAN_CTX_HOST_MCP=…`), den lean-md andialt → dieselbe Instanz, hinter der es läuft.
Steht nicht im Addon-Kontrakt (#858/#863). Heute deckt der `CliBackend`-Default alles
ab. → Upstream-Feature-Request an lean-ctx, falls die zweite-Prozess-Latenz stört.

## McpBackend-Reife

Endpoint-Discovery statt manuell gesetztem `LEAN_MD_MCP_ENDPOINT`; Parity-Gate #3
(`backend_parity`) ohne gesetzten Endpoint ist heute `ignored`. Reifung des
Performance-Pfads gegenüber dem CliBackend-Default; nur relevant bei vielen
Code-Intel-Direktiven pro Render.

## Namespacing (falls Prefix erzwungen — aus Task 3)

Falls das Gateway `lean-md::ctx_md_render` erzwingt: Upstream-Ticket für einen
transparenten/leeren Namespace, damit der Tool-Name byte-identisch zum Phase-9-Namen
bleibt. (Bei transparentem Namespace: dieser Punkt entfällt.)
```

- [ ] **Step 2: Verifikation**

Run: `ctx_read(path="/home/tholo/Scripts/lean-md/docs/lean-md/FOLLOWUPS.md", mode="full")`
Expected: Datei existiert, keine Platzhalter; falls Task 3 transparenten Namespace ergab, ist der Namespacing-Abschnitt entsprechend entschärft.

- [ ] **Step 3: Commit**

Run: `ctx_shell(command="git -C /home/tholo/Scripts/lean-md add docs/lean-md/FOLLOWUPS.md")`
Run: `ctx_shell(command="git -C /home/tholo/Scripts/lean-md commit -m 'docs(R4): note upstream follow-ups (host-callback, mcp-backend, namespacing)'")`

---

## Self-Review

- **Spec-Coverage:** §5.1.1 Namespacing = Task 3; §5.1.2 Live-Gates #4/#5 = Task 4; §5.2.1 README = Task 1; §5.2.2 INSTALL.md = Task 2; §5.3 Zukunftspfade = Task 5; §5.4 zero-config-Entscheidung = in INSTALL.md (Task 2 Backend-Abschnitt) als „defaults to zero-config" dokumentiert. `LICENSE` + Version-SSOT bereits erledigt (Spec §5.2 IST).
- **Tool-Disziplin:** jede Doku-Edit über `ctx_edit`/`Write`; jede potentielle `.rs`-Edit (Task 4 Step 1) über `ctx_edit` für String-Literale bzw. `ctx_refactor` + `action=reformat` für Symbol-Edits; alle Shell-Befehle einzeln, `--manifest-path`, kein `&&`.
- **nextest-knowledge:** ignored via `--run-ignored ignored-only`/`all` (nicht `-- --ignored`); mcp via `--features mcp`; nie `cargo test` (außer Doctests separat) — durchgängig in Tasks 1/4 angewandt.
- **Platzhalter-Scan:** die `<…>`-Marker in Task 3 Step 4 sind explizit als „durch realen Befund ersetzen" markiert (Laufzeit-Daten, kein Plan-Platzhalter); sonst keine TBD/TODO.
- **Typ-/Namens-Konsistenz:** Tool-Namen `ctx_md_render`/`ctx_md_check`, CLI `render/check/mcp`, Env `LEAN_MD_BACKEND`/`LEAN_MD_MCP_ENDPOINT`, Testdatei `addon_roundtrip.rs`, Quickstart-Datei `demo.lmd.md` durchgängig identisch über alle Tasks.
- **Offene Laufzeit-Verifikation:** `lean-ctx addon add`-Bestätigungsflag (`-y`), `ctx_tools`-Arg-Form (Task 3), `ctx_read`-Delegations-Arg-Form (Task 4) — gegen `lean-ctx … --help` / `appendix-mcp-tools` angleichen, keine erfundenen Schnittstellen.
