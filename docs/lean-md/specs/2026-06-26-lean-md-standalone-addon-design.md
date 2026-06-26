---
title: lean-md v2 — standalone Addon: Architektur, IST-Stand & Roadmap
slug: lean-md-standalone-addon
status: draft
date: 2026-06-26
supersedes: docs/lean-md/specs/2026-06-22-lmd-lean-ctx-native-design-v2.md
integrates: docs/lean-md/specs/2026-06-25-lmd-v2-addon-decoupling-design.md
lean_ctx_version: 3.8.13
consumer: ai
note: >
  Konsolidierter Spec für den standalone-Addon-Stand. Reset auf den verifizierten
  Code-Stand BEIDER Repos (lean-md @ feat-lmd-v1, lean-ctx @ feat-lmd-v1, 3.8.13):
  die Boundary-/Packaging-Änderung aus dem decoupling-design (2026-06-25) ist
  umgesetzt UND die im native-design-v2 (2026-06-22) noch als „offen" geführten
  Phasen 7–11 sind implementiert. v2 trägt den abgeschlossenen Stand als kompakten
  IST (§2–§4) und richtet die Detailbeschreibung auf die verbleibende offene Arbeit
  (§5): Korrektheits-Nachweis (Namespacing + Live-E2E-Gates), Dokumentation
  (README + INSTALL.md), notierte Zukunftspfade.
---

# lean-md v2 — standalone Addon: Architektur, IST-Stand & Roadmap

> lean-md ist ein **eigenständiges Produkt** (eigenes Repo `dasTholo/lean-md`,
> Crate `lean-md` = lib `lean_md` + bin `lean-md`), verteilt als lean-ctx-**Addon**.
> Der Render-Kern (`rushdown` + `evalexpr`) läuft **in-process** und braucht **null**
> lean-ctx; jede Code-Intel-Direktive ist **outbound** über `CodeIntelBackend`
> (CLI default, MCP opt-in). lean-md hält **keinen** `lean_ctx`-Crate-Dep — nur
> optional `lean-ctx-client` hinter dem `mcp`-Feature.

> **Stand 2026-06-26:** Boundary umgesetzt + Phasen 0–11 implementiert (§2).
> Offen ist nur noch §5: Korrektheits-Nachweis, Dokumentation, Zukunftspfade.

---

## 0. Supersede — was v2 gegenüber native-design-v2 revidiert

`2026-06-22-lmd-lean-ctx-native-design-v2.md` beschrieb lmd als **in-tree
lean-ctx-Erweiterung** (`rust/src/lmd/`, 26 Bridges) mit **offener** Phase 7–11.
Beide Annahmen sind überholt:

| native-design-v2 (2026-06-22)                                              | v2-Revision (dieser Spec)                                                                                                       |
| lmd lebt in `lean-ctx/rust/src/lmd/`, hängt an der `lean_ctx`-**lib**       | **Eigenes Repo** `dasTholo/lean-md`; **null** `lean_ctx`-Crate-Dep, nur optional `lean-ctx-client` (`mcp`-Feature).            |
| Code-Intel ruft `crate::tools::ctx_*::handle` als **Lib-Funktion**          | Code-Intel **outbound** über `CodeIntelBackend` (`CliBackend` default / `McpBackend` opt-in).                                  |
| lokales tree-sitter / Core-Primitive (PathJail/Redaction) via Lib geerbt    | **Kein** lokaler Code-Parser; PathJail/Redaction werden **server-/host-seitig** erzwungen (§4).                                |
| Phase **7–11 offen** (`@dispatch`/`@handoff`, TDD-Hook, Layout, Pilot-Skill) | **implementiert** — `bridges/{dispatch,handoff}.rs`, `crp*.rs` (TDD/CRP), `content/{core,skills,lang,tooling,templates}/` (§2). |
| Verteilung als in-tree-Tools (`pub mod lmd`, Registry-Registrierung)        | Verteilung als **Addon** (`lean-ctx addon add`); lean-ctx-seitiger Reverse-Cut vollzogen (§2.2).                               |

**Nicht revidiert:** Direktiven-**Verhalten** und #498-Determinismus bleiben
unverändert. v2 ist eine Boundary-/Packaging-/Status-Aktualisierung, keine
Verhaltensänderung. Die Boundary-Herleitung selbst lebt weiter im
**integrierten** `2026-06-25-lmd-v2-addon-decoupling-design.md` (nicht ersetzt).

---

## 1. Ziel & Abgrenzung

- **eigenes Repo** `dasTholo/lean-md` (`origin` gesetzt), Crate `lean-md`
  (lib `lean_md` + bin `lean-md`), `edition = 2024`, `license = Apache-2.0`.
- **eigener MCP-Server** (`lean-md mcp`, stdio) — exponiert `ctx_md_render` +
  `ctx_md_check`. Plus CLI-Pfad `lean-md render|check`.
- **Render-Kern self-contained** — `rushdown` (Parser) + `evalexpr` (Eval); eine
  Code-Intel-freie `.lmd.md` rendert vollständig standalone (kein Backend-Call).
- **Code-Intel outbound** über `lean-ctx-client` (`/v1`-Kontrakt) ODER `lean-ctx`-CLI.
- verteilt per `lean-ctx addon add lean-md` (Registry-Eintrag + Manifest).

**Bewusst NICHT:**

- **Kein** Workspace-Member, **kein** `lmd_host`, **kein** `lean_ctx`-Crate-Dep.
- **Kein** lokales tree-sitter / kein lokaler Code-Parser — Code-Intel ist outbound.
- **Kein** Re-Implementieren von PathJail/Redaction als Sicherheitsgrenze (§4).
- **Kein** dynamisches `dlopen`/Plugin-ABI — Addon = separater Prozess + MCP.
- **Kein** Agent-Spawn durch `@dispatch` — reiner Prompt-Renderer (Determinismus).

---

## 2. Verifizierter IST-Stand — beide Repos dekoppelt

Verifiziert gegen `lean-md@feat-lmd-v1` und `lean-ctx@feat-lmd-v1` (3.8.13),
2026-06-26.

### 2.1 lean-md-Repo — vollständige Fläche (lib + bin + content)

**Crate-Form** (`Cargo.toml`): `name = "lean-md"`, `version = "0.1.0"`,
deps `rushdown 0.18` + `evalexpr 13.1` + `serde_json` + `regex` + `chrono`;
`[features] mcp = ["dep:lean-ctx-client"]` (default leer).

| Komponente                          | Code-Ort (verifiziert)                                                  | Status |
| Render-Kern (Parser/Engine/Render)  | `parser/{block,inline,mod}.rs` `engine.rs` `render.rs` `node.rs`        | ✅      |
| Header / Macros / Phases            | `header.rs` `macros.rs` `phases.rs`                                      | ✅      |
| Bridge-Registry (30 Bridges)        | `bridges/mod.rs` (`default_registry`)                                    | ✅      |
| R-Bridges (read/search/list/…)      | `bridges/{read,include,search,list,env,date,count,query}.rs`            | ✅      |
| Code-Intel-Bridges                  | `bridges/{edit,symbol,refactor,reformat,inspect,find,graph}.rs`         | ✅      |
| Map/Quality-Bridges                 | `bridges/{repomap,impact,architecture,outline,smells,review,routes}.rs` | ✅      |
| Macro/Extension-Bridges             | `bridges/{call,render}.rs`                                               | ✅      |
| Session/Knowledge-Bridges           | `bridges/{remember,recall}.rs`                                           | ✅      |
| **Dispatch & Hand-over (Phase 7)**  | `bridges/{dispatch,handoff}.rs` `bridges/addressing.rs`                 | ✅      |
| **TDD/CRP-Render (Phase 8)**        | `crp.rs` `crp_proto.rs` `crp_schema.rs` `signatures.rs` `gloss.rs`      | ✅      |
| **Skills/Availability (Phase 10/11)** | `skills.rs` `seeds.rs` `availability.rs` `auto_findings.rs`            | ✅      |
| **Outbound-Backend**                | `backend.rs` (`CodeIntelBackend` trait, Cli default / Mcp opt-in)       | ✅      |
| MCP-Server + CLI                    | `bin/lean_md.rs` (`mcp`/`render`/`check`)                               | ✅      |
| **Embedded content-Seeds**          | `content/core/{hard-rules,dispatch-contract}.lmd.md` + `_fragments/`, `content/{lang,tooling,templates,gloss}/`, `content/skills/lmd-brainstorm/{SKILL.md,body.lmd.md}` | ✅ |

### 2.2 lean-ctx-Repo — Reverse-Cut vollzogen

Verifiziert gegen `lean-ctx@feat-lmd-v1` (git log):

- **Reverse-Cut:** `pub mod lmd` + `src/lmd/` + root `lean-md/`-Seed-Dir +
  `rushdown`/`evalexpr`-Deps entfernt; `lean-ctx md`-CLI-Pfad + `[lean-md]`-Config
  + lmd-Skill-Installer entfernt. Reverse-Cut-Gate (#6) grün.
- **Auto-Render-Delegations-Hook:** `ctx_read` einer `.lmd.md` delegiert via Gateway
  an den lean-md-Addon (falls installiert), sonst Rohtext (ersetzt die in-tree
  `RenderTransform`). Delegation-Gate (#5, ohne-Addon-Pfad) grün.
- **Registry-Eintrag:** lean-md in `rust/data/addon_registry.json` registriert,
  Tier **community**; Tool-Count-SSOT restauriert, MCP-Manifest/Reference-Docs
  re-generiert.

### 2.3 Test-Gates (IST)

`lean-md/tests/`: `standalone.rs` (#2, Code-Intel-freier Render ohne Backend),
`backend_parity.rs` (#3, Cli==Mcp, `mcp`-Feature, ohne Endpoint `ignored`),
`addon_roundtrip.rs` (#4, **`ignored`** — braucht installiertes Addon),
`determinism.rs` (#7, Seed-Byte-Identität + render byte-stabil). Die portierten
§8-Direktiven-Tests (#1–16) gelten unverändert. → via `cargo nextest run`.

---

## 3. Architektur-Invarianten

### 3.1 `CodeIntelBackend` — die lean-md ↔ lean-ctx-Grenze

```
trait CodeIntelBackend { fn call(&self, tool: &str, args: Value) -> Result<String>; }
```

- **`CliBackend` (Default):** `lean-ctx call <tool> --project-root <root> --json '<args>'`.
  Stateless, kurzlebig, kein Endpoint-Discovery — braucht nur `lean-ctx` im PATH;
  **keine** `tool_profile`-Vorbedingung (`call` dispatcht an jedes Tool).
- **`McpBackend` (opt-in, `mcp`-Feature):** warme Verbindung via `lean-ctx-client`
  gegen einen erreichbaren MCP/HTTP-Endpoint. Auswahl per Env
  (`LEAN_MD_BACKEND=mcp` + `LEAN_MD_MCP_ENDPOINT=<url>`); Fallback auf CLI bei
  unerreichbarem/fehlerhaftem Endpoint. **Vorbedingung: `tool_profile = power`.**

### 3.2 Topologie & das „behind"-Prinzip

```
Agent ──MCP──▶ lean-ctx (Host + Gateway)
                  │ spawnt stdio-Child (Addon, via `lean-ctx addon add`)
                  ▼
              lean-md (mcp)  ── Render-Kern in-process (rushdown/evalexpr)
                  │ Code-Intel-Direktive
                  └──▶ CodeIntelBackend
                         ├─ CliBackend:  `lean-ctx call ctx_* --json …`
                         └─ McpBackend:  lean-ctx-client ──▶ lean-ctx (power)
```

Der Agent spricht **nur** lean-ctx; lean-md ist immer Gateway-Child (inbound).
Das Gateway ist ein Einweg-Proxy (Agent → Addon) und bietet dem Addon **keinen**
Rückkanal → der Outbound-Leg verlässt das Gateway zwangsläufig (bei beiden
Backends). **Kein Re-Entrancy-Loop:** lean-md ruft outbound nur Code-Intel-Tools,
**nie** `ctx_md_*`.

### 3.3 Determinismus (#498)

- Tool-Output ist deterministische Funktion aus (Dateiinhalt, Modus, CRP-Modus,
  Task). Keine Timestamps/Counter in Output-Bodies; Artefakt-Pfade content-adressiert.
- Embedded Seeds (`content/core/*.lmd.md` via `include_str!`) byte-identisch;
  Fragment-Konsistenz-Gate (built-in == on-disk) bleibt grün.
- `CliBackend`/`McpBackend` treffen denselben Handler → byte-identische Ergebnisse.

### 3.4 Addon-Config — Erstellung & Defaults (verifiziert gegen `core::addons`)

**Es gibt zwei Config-Ebenen — sauber getrennt:**

**(a) Server-Wiring — von `lean-ctx addon add` automatisch geschrieben.** Der
Addon-Autor pflegt **keine** lean-ctx-Config von Hand; `addon add <name|path>`
(→ `core::addons::install::install`) erzeugt sie:

1. `manifest.validate()` (Slug + Capabilities) → `to_gateway_server()` baut
   `GatewayServer { name, transport, enabled=true, command, args, env,
   binary_sha256, url, headers, capabilities }` **1:1 aus dem Manifest**.
2. Revocation-Check (Kill-Switch) + `policy::gate` (Capability-/Sandbox-Policy,
   Security-Floor #865) — ein geblockter Addon mutiert die Config **nie**.
3. `Config::update_global` (**global-only**, kein Projekt-Merge): setzt
   `gateway.enabled = true` (falls aus) + **upsert** `[[gateway.servers]]`
   (gleichnamiger Eintrag wird ersetzt → idempotent).
4. Record in `<data_dir>/addons/installed.json` (`name`/`version`/`source`/
   `gateway_server`/`granted_capabilities`/`content_hash`); Gateway-Katalog-Cache
   invalidiert. `addon remove` macht beides rückgängig.

**(b) Manifest-Defaults (`#[serde(default)]`, `AddonMcp`/`AddonCapabilities`):**

- `[mcp]`: `transport = stdio`, `command/url = ""`, `args = []`, `env = {}`,
  `sha256 = ""`, `headers = {}`. lean-md setzt `command = "lean-md"`, `args = ["mcp"]`.
- `[capabilities]` **absent** → `None` → Legacy-`addons.sandbox`-Verhalten;
  **present** → secure-by-default Capability-Modell (Default strikt:
  `network=none`, `filesystem=read_only`, `exec` restricted). lean-md deklariert
  **explizit** `network=none`, `filesystem=read_write`, `exec=["lean-ctx"]`
  (Callback-Addon, declared+audited — **nicht** OS-enforced, 9fbe855e).

**(c) lean-md-Runtime-Config = Env-Vars, KEINE eigene Config-Datei.** Die
Backend-Auswahl (`backend::default_backend`) liest **nur** `LEAN_MD_BACKEND` +
`LEAN_MD_MCP_ENDPOINT`; ohne Env → `CliBackend` (Default). Diese Vars lassen sich
im Manifest `[mcp].env` **vorbelegen** → der Gateway injiziert sie beim Spawn in
den lean-md-Child. lean-md hält bewusst **keine** TOML/JSON-Config (kein
Config-Discovery, kein State) — siehe offene Entscheidung §5.4.

---

## 4. Security-Modell — erben, nicht neu erfinden

| Eimer                          | Primitive                                            | Erzwingung im Addon-Modell                                                                 |
| **Host erzwingt für lean-md**  | Redaction, Secret-Detection, Containment             | Addon-Runtime redacted Output server-seitig (`runtime::scrub_output`) + opt-in OS-Sandbox. |
| **Über MCP/CLI-Tool**          | alle Code-Intel-Handler, knowledge, handoff, shell   | `ctx_*` über Cli/Mcp-Backend — **autoritative PathJail server-seitig inklusive.**          |
| **Winziges generisches Local** | Pre-Flight-Pfad-Canonicalize (parent-jail in `bin`)  | stdlib in lean-md; **nicht** die Sicherheitsgrenze, nur Pre-Flight.                        |

lean-md „verliert" PathJail/Redaction **nicht** — es war nie lean-mds Aufgabe, sie
zu *erzwingen*. Alle Datei-/Code-Operationen laufen über Tools, deren autoritative
Guards in lean-ctx (bzw. der Host-Runtime) sitzen.

---

## 5. Offene Arbeit (Roadmap)

### 5.1 Korrektheits-Nachweis

#### 5.1.1 Namespacing-Auflösung (§5.4 decoupling-design)

**Offene Laufzeit-Frage:** Vergibt das Gateway dem Addon einen Namespace
(`lean-md::ctx_md_render`) oder bleibt der Tool-Name transparent (`ctx_md_render`,
byte-identisch zum Phase-9-Namen)?

- **Anzustreben:** transparenter/leerer Namespace → rückwärtskompatibel zum
  bisherigen Tool-Namen; Auto-Render-Delegations-Hook + bestehende Aufrufer
  bleiben unverändert.
- **Verifikation:** nach `lean-ctx addon add ./lean-ctx-addon.toml` + Server-Neustart
  den real exponierten Tool-Namen via `ctx_tools`/Gateway-Katalog prüfen.
- **Befund (2026-06-26):** Der Gateway exponiert das Tool als `lean-md::ctx_md_render` —
  Prefix erzwungen (Quelle: `rust/src/core/gateway/catalog.rs`,
  `format!("{}::{}", server.name, tool)`). Transparenter Namespace ist nicht möglich,
  da lean-ctx den Namespace hart einbaut → Task 4 passt die Test-Toolnamen an +
  Upstream-Folgeticket; **kein** v2-Blocker.
- **Falls Prefix erzwungen:** Test-Toolnamen in `addon_roundtrip.rs` / Delegation-Gate
  anpassen + Upstream-Folgeticket; **kein** v2-Blocker.

#### 5.1.2 Live-E2E-Gates #4/#5 grün fahren

`addon_roundtrip.rs` (#4) und der mit-Addon-Pfad des Delegation-Gates (#5) sind
`#[ignore]` (brauchen reale Installation + MCP-Server-Neustart, laufen nicht im
Standard-`nextest`). Liefergegenstand:

- **dokumentierter manueller Durchlauf:** `cargo install --path .` →
  `lean-ctx addon add ./lean-ctx-addon.toml` → Server-Neustart →
  `cargo nextest run --test addon_roundtrip -- --ignored` (Expected: byte-identisch
  zu `lean-md render`).
- **CI-Strategie:** entweder ein opt-in Integrations-Job (Addon installieren +
  Server-Neustart skripten) ODER explizit als „manuell verifiziert"-Gate
  dokumentieren — Transparenz statt stillem `ignored`.
- **Akzeptanz:** Addon-Pfad == direkter `lean-md mcp`-Pfad (#4); `ctx_read` einer
  `.lmd.md` mit Addon == direktes `ctx_md_render`, ohne Addon → Rohtext (#5).
- **Nachweis (2026-06-26):** #4 `addon_roundtrip` BLOCKED — `lean-ctx call
  ctx_md_render` liefert in lean-ctx 3.8.13 `error: unknown tool 'ctx_md_render'`
  (ebenso `lean-md::ctx_md_render` und `lean-md/ctx_md_render`); die CLI-`call`-
  Route routet nicht zu Gateway-/Addon-Tools, nur zu lean-ctx-eigenen Built-ins.
  Der lean-md MCP-Server selbst ist korrekt verdrahtet: direktes JSON-RPC
  (`initialize` + `tools/call ctx_md_render`) liefert byte-identische Ausgabe zu
  `lean-md render` (verifiziert 2026-06-26, Eingabe `@date\nroundtrip marker\n`
  → `2026-06-26\nroundtrip marker\n`). #5 Delegation BLOCKED — `lean-ctx call
  ctx_read` → `-32603: session not available` (kein Projekt-Session ohne aktive
  MCP-Verbindung vom MCP-Client). CI-Strategie: `addon_roundtrip.rs`
  `via_leanctx_call` muss redesigned werden — entweder `lean-md mcp` direkt via
  JSON-RPC ansprechen (umgeht lean-ctx-call), oder warten bis lean-ctx Addon-Tools
  via `lean-ctx call` exponiert. Kein v2-Blocker; Upstream-Folgeticket empfohlen.

### 5.2 Dokumentation

**IST:** `README.md` ist install-fokussiert (Addon-Install, Backend-Auswahl,
Kontrakt-Verweis); keine `INSTALL.md`; `docs/CONTRACT.md` vendored
(`lean-ctx@2946c165a`); `LICENSE` (Apache-2.0, Rechteinhaber dasTholo) ergänzt;
Version auf `0.1.0` vereinheitlicht (`Cargo.toml` == Manifest). `Cargo.toml`
`include` packt `README.md` + `INSTALL.md` + `LICENSE` + `docs/CONTRACT.md` mit.

#### 5.2.1 README erweitern (Produkt-README)

Ergänzen um:
- **Was ist lean-md** — Render-Kern in-process + outbound Code-Intel (kurzer Pitch).
- **CLI-Surface** — `lean-md render <file>` / `lean-md check <file>` / `lean-md mcp`
  (mit Beispiel-Output).
- **Direktiven-Überblick** — die R/E-Direktiven-Fläche kompakt (Verweis auf
  `content/gloss/directives.lmd.md` als Detail-Glossar) + `consumer=ai|human`-Modus.
- **`.lmd.md`-Quickstart** — minimales Beispiel (Header + eine Direktive) →
  gerenderter Output.
- **Skills** — `content/skills/lmd-brainstorm/` als Pilot; render-on-invoke via
  `ctx_md_render(skill, phase)`.
- Bestehende Abschnitte (Addon-Install, Backend-Auswahl, Manifest-Kontrakt) bleiben.

#### 5.2.2 INSTALL.md ableiten

Die Install-Tiefe aus der README in eine dedizierte `INSTALL.md` herausziehen +
erweitern:
- **Voraussetzungen** — `lean-ctx ≥ 3.8.13` im PATH, Rust-Toolchain für
  `cargo install --path .`.
- **Pfad A — Registry** (`lean-ctx addon add lean-md`, sobald gelistet).
- **Pfad B — lokaler Klon** (`lean-ctx addon add ./lean-ctx-addon.toml`).
- **Server-Neustart-Schritt** (Gateway-Katalog re-read) — prominent, da häufige
  Fehlerquelle.
- **Verifikation** (`ctx_md_render` erreichbar) + **Troubleshooting** (Tool nicht
  sichtbar → Neustart; Namespacing-Prefix → §5.1.1).
- **Backend-Auswahl** (Env-Var-Tabelle, Verweis statt Duplikat).
- **Was `addon add` schreibt** — kurz erklären, dass der Install automatisch einen
  globalen `[[gateway.servers]]`-Eintrag + `<data_dir>/addons/installed.json`
  anlegt und `addon remove` beides zurücknimmt (Mechanik §3.4) — damit der Nutzer
  weiß, welche Config entsteht und dass er nichts von Hand pflegen muss.
- README verlinkt auf `INSTALL.md` für die Tiefe (README = Überblick, INSTALL = Detail).

### 5.3 Zukunftspfade (notiert, kein v2-Blocker)

- **Host-Callback (§3.4 decoupling-design):** echte „Single-Server"-Reinheit
  erforderte eine neue lean-ctx-Fähigkeit — Gateway injiziert beim Spawn den
  Host-MCP-Endpoint in die Addon-Env (`[mcp].env`, z. B. `LEAN_CTX_HOST_MCP=…`),
  den lean-md andialt → dieselbe Instanz, hinter der es läuft. Steht **nicht** im
  Addon-Kontrakt; Upstream-Folgevorschlag. CliBackend deckt heute alles ab.
- **McpBackend-Reife:** Endpoint-Discovery (statt manuell gesetztem
  `LEAN_MD_MCP_ENDPOINT`) + Parity-Gate #3 ohne gesetzten Endpoint (heute
  `ignored`). Reifung des Performance-Pfads gegenüber dem CliBackend-Default;
  nur relevant bei vielen Code-Intel-Direktiven pro Render.

### 5.4 Addon-Config — offene Entscheidung (zero-config vs. `[mcp].env`-Defaults)

Die Config-Mechanik selbst ist verifiziert (§3.4); offen ist nur die Default-Politik:

- **IST:** lean-md ist **zero-config** — keine eigene TOML/JSON-Datei, Backend rein
  über Env (`LEAN_MD_BACKEND`/`LEAN_MD_MCP_ENDPOINT`), Default `CliBackend`. Das
  Manifest deklariert **kein** `[mcp].env` → der gespawnte Child erbt keine
  vorbelegten Backend-Vars.
- **Entscheidung (Vorschlag, im Spec festzuhalten):** zero-config **beibehalten** —
  `CliBackend` braucht keine Config, und ein leeres `[mcp].env` hält das Manifest
  finding-frei (Audit). `[mcp].env`-Vorbelegung **nur** dokumentieren als Opt-in für
  MCP-Backend-Deployments (z. B. `LEAN_MD_BACKEND=mcp` + `LEAN_MD_MCP_ENDPOINT=…`),
  **kein** eigenes Config-Discovery in lean-md einführen (kein State, headless,
  Determinismus #498). → in README/INSTALL als „so konfigurierst du das MCP-Backend"
  erklären, nicht als Default setzen.

---

## 6. Test-/Parity-Strategie

**IST-Gates (grün, §2.3):**

1. `standalone.rs` (#2) — Code-Intel-freier Render ohne laufenden lean-ctx.
2. `backend_parity.rs` (#3) — Cli==Mcp byte-identisch (`mcp`-Feature; ohne
   Endpoint `ignored`).
3. `determinism.rs` (#7) — built-in Seeds == on-disk; render byte-stabil über Läufe.
4. portierte §8-Direktiven-Tests (#1–16) — Direktiven-Verhalten unverändert.
5. lean-ctx-seitig: Reverse-Cut-Gate (#6), Delegation-Gate ohne-Addon (#5-Teil).

**Offene Gate-Nachweise (§5.1):**

6. `addon_roundtrip.rs` (#4) — Addon-Pfad == direkter Pfad (heute `ignored`; §5.1.2).
7. Delegation-Gate mit-Addon (#5-Teil) — `ctx_read` `.lmd.md` == `ctx_md_render`
   (heute `ignored`; §5.1.2).
8. Namespacing-Verifikation (§5.1.1) — realer Tool-Name nach Install.

Alle via `cargo nextest run` (Projekt-Hard-Rule); Live-Gates per `-- --ignored`
nach Installation + Server-Neustart.

---

## 7. Risiken & Reihenfolge

| Risiko                                                                          | Mitigation                                                                                         |
| Gateway-Namespace ändert Tool-Namen (`lean-md::ctx_md_render`)                  | §5.1.1 — transparenten Namespace anstreben; sonst Test-Toolname anpassen + Folgeticket.            |
| Live-Gates bleiben dauerhaft `ignored` → Addon-Pfad faktisch unverifiziert      | §5.1.2 — dokumentierter manueller Durchlauf ODER opt-in CI-Job; nie stilles `ignored`.             |
| Version-SSOT über mehrere Artefakte driftet (Cargo / Manifest / Specs)          | **erledigt** — alle auf `0.1.0` vereinheitlicht; künftige Bumps an `Cargo.toml` + `lean-ctx-addon.toml` synchron halten. |
| `lean-ctx-client` zeigt auf **lokalen Pfad** (`Cargo.toml`-Kommentar)            | git/crates.io-Switch ist Distribution-Scope (hier NICHT enthalten); als Vorbedingung notieren.     |
| README ohne Produkt-/Usage-Inhalt → schwer einsteigbar                          | §5.2.1 — README erweitern; INSTALL.md ableiten.                                                    |

**Reihenfolge (für `writing-plans`):**

- **R1 Dokumentation (§5.2):** README erweitern → `INSTALL.md` ableiten. `LICENSE`
  (Apache-2.0) + Version-SSOT (`0.1.0`) bereits erledigt. Kein Code, sofort lieferbar.
- **R2 Namespacing-Verifikation (§5.1.1):** Addon lokal installieren, Server-Neustart,
  realen Tool-Namen prüfen → Befund (transparent | Prefix + Folgeticket).
- **R3 Live-E2E-Gates (§5.1.2):** #4/#5 manuell grün fahren (dokumentiert) + CI-Strategie
  festlegen.
- **R4 Zukunftspfade (§5.3):** als Upstream-Folgevorschläge notieren (kein Code in v2).

---

*Status: v2-draft (2026-06-26). Supersedet `2026-06-22-lmd-lean-ctx-native-design-v2.md`;
integriert `2026-06-25-lmd-v2-addon-decoupling-design.md` als umgesetzte Boundary-Baseline.
Verifiziert: lean-md@feat-lmd-v1 (30 Bridges + content-Seeds + Cli/Mcp-Backend + mcp/render/check-bin),
lean-ctx@feat-lmd-v1 3.8.13 (Reverse-Cut, addon_registry community, Auto-Render-Hook, Gates #5/#6),
`Cargo.toml` (lean-ctx-client optional/lokal, `mcp`-Feature), `docs/CONTRACT.md` (lean-ctx@2946c165a).*
