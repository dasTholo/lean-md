# lmd v2 — `lean-md` als eigenständiger Addon (Design)

> **Supersedes** `2026-06-22-lmd-phase-12-side-loaded-crate-design.md` (Workspace-
> Crate-Modell). Phase 12 löste lmd in eine `packages/lean-md/`-Crate heraus, die
> per `lmd_host`-Facade gegen die `lean_ctx`-**lib** linkt — also **compile-time
> gekoppelt** blieb. Mit dem Upstream-**Addon-Ecosystem** (`yvgude` #858, Security-
> Hardening #863) ist dieses Modell überholt: ein Addon umhüllt einen **externen
> MCP-Server** hinter `lean-ctx-addon.toml` und wird per `lean-ctx addon add` ins
> Gateway eingehängt — **kein Fork, kein Recompile, kein Lib-Link**. Dieser Spec
> beschreibt die *echte* Abkopplung: `lean-md` zieht in ein **eigenes Repository**
> und spricht lean-ctx nur noch über den **MCP/CLI-Draht** (`/v1`-Kontrakt) an.

---

## 0. Supersede — was v2 an Phase 12 revidiert

| Phase-12-Entscheidung                                                          | v2-Revision                                                                                                                            |
|--------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------|
| `packages/lean-md/` als Workspace-Member, hängt an `lean_ctx`-**lib**          | **Eigenes Repo** `dasTholo/lean-md`, hängt nur an `lean-ctx-client` (crates.io, `/v1`-Kontrakt). **Null** `lean_ctx`-Crate-Dep.        |
| `lmd_host`-Facade re-exportiert ~33 Core-Interna; `pub(crate)`→`pub`-Sweep     | **Entfällt.** Keine Rust-Symbole überqueren die Grenze. Grenze = MCP/CLI-Wire (Tool-Namen + JSON-Schemas + byte-stabile Outputs #498). |
| Gateway spawnt die `lean-md`-Binary als stdio-Child (`[[gateway.servers]]`)    | **Bleibt** — aber jetzt via offiziellem **Addon-Mechanismus** (`lean-ctx addon add`), nicht als handgepflegter Config-Eintrag.         |
| Code-Intel-Direktiven rufen `crate::tools::ctx_*::handle` als **Lib-Funktion** | Code-Intel-Direktiven rufen `ctx_*` **über den Draht** (`CliBackend`/`McpBackend`).                                                    |
| lmd erbt tree-sitter/PathJail/Redaction via Lib                                | tree-sitter **entfällt lokal** (lean-md parst keinen Code mehr); PathJail/Redaction werden **server-/host-seitig** erzwungen.          |

Nicht revidiert: Direktiven-**Verhalten** und #498-Determinismus bleiben unverändert.
v2 ist eine Boundary-/Packaging-Änderung, keine Verhaltensänderung.

---

## 1. Ziel & Abgrenzung

`lean-md` wird ein **eigenständiges Produkt**: eigenes Repo, eigener Release-Zyklus,
verteilt als lean-ctx-**Addon**.

- **eigenes Repo** `dasTholo/lean-md` (Crate `lean-md`, lib + bin).
- **eigener MCP-Server** (`lean-md mcp`, stdio) — exponiert `ctx_md_render` +
  `ctx_md_check`. Plus CLI-Pfad `lean-md render|check`.
- **abhängig nur von `lean-ctx-client`** (der publizierte, engine-unabhängige
  `/v1`-Client) für den Outbound-Code-Intel — **nicht** vom `lean_ctx`-Crate.
- verteilt per `lean-ctx addon add lean-md` (Registry-Eintrag + Manifest).

**Bewusst NICHT:**

- **Kein** Workspace-Member, **kein** `lmd_host`, **kein** `pub(crate)`→`pub`-Sweep.
- **Kein** lokales tree-sitter / kein lokaler Code-Parser — Code-Intel ist outbound.
- **Kein** Re-Implementieren von PathJail/Redaction als Sicherheitsgrenze (die
  Grenze lebt server-/host-seitig, siehe §6).
- **Kein** dynamisches `dlopen`/Plugin-ABI — Addon = separater Prozess + MCP.

---

## 2. Kopplungs-Audit (verifiziert gegen `feat-lmd-v1`)

lean-mds Kopplung an lean-ctx zerfällt in **zwei Klassen** — die Grenze verläuft
exakt dazwischen.

### 2.1 Render-Kern — self-contained (bleibt in-process)

`@if`, `{{ }}`-Eval, Makros, Container-Gating, Pipe-Render, Layout. Deps:
**`rushdown`** (Parser) + **`evalexpr`** (Eval). Braucht **null** lean-ctx. Eine
reine Render-`.lmd.md` ohne Code-Intel-Direktive rendert vollständig **standalone**.

### 2.2 Code-Intel- + Bridge-Direktiven — outbound

`@edit/@refactor/@symbol/@find/@codeintel/@reformat/@inspect` + handoff/dispatch/
session/knowledge. Diese rufen heute **Tool-Handler** `crate::tools::ctx_*::handle`
(~16) — **alle über MCP/CLI erreichbar** (§3).

### 2.3 Direkte Core-Primitive — schmaler Saum (verifiziert)

Produktive Direktaufrufe in `src/lmd` (ohne `#[cfg(test)]`):

| Primitive                               | Call-Sites (produktiv)                                           |
|-----------------------------------------|------------------------------------------------------------------|
| `core::path_resolve::resolve_tool_path` | `bridges/addressing.rs`, `bridges/edit.rs`, `bridges/inspect.rs` |
| `core::pathjail::jail_path`             | `bridges/graph.rs`, `bridges/handoff.rs`                         |

`redaction` erscheint nur als Doc-Kommentar (`audit.rs:70`), **kein** produktiver
Aufruf; `secret_detection`/`core::tokens`/`extension_registry` produktiv **gar
nicht**. Alle übrigen `std::fs::*`-Treffer sind Test-Scaffolding.

**Befund:** Die direkte Primitive-Kopplung ist **flach** — ein „Pfad auflösen +
validieren, bevor ich den Tool-Call absetze". PathJail ist server-seitig auf *jedem*
`ctx_read`/`ctx_edit`/`ctx_refactor`-Call autoritativ erzwungen; lean-mds lokale
`jail_path`-Aufrufe sind **Pre-Flight**, nicht die Sicherheitsgrenze. → §6.

### 2.4 Reverse-Kopplung `Core → lmd` (muss aus lean-ctx raus / re-homed)

| Ort                                                                                                                            | Inhalt                                           | v2-Ziel                                       |
|--------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------|-----------------------------------------------|
| `rust/src/lib.rs` `pub mod lmd;`                                                                                               | Modul-Einhängung                                 | **entfernen**                                 |
| `rust/src/server/registry.rs`                                                                                                  | `CtxMdRenderTool`/`CtxMdCheckTool`-Registrierung | **entfernen** (Tools leben im lean-md-Server) |
| `rust/src/tools/registered/{mod,ctx_md}.rs`                                                                                    | ctx_md-Tool-Defs                                 | **entfernen**                                 |
| `rust/src/cli/dispatch/lean_md.rs` (+`mod.rs`-Branch)                                                                          | `lean-ctx md …`-CLI-Pfad                         | **entfernen**                                 |
| `rust/src/core/wasm_ext.rs` (Test)                                                                                             | ruft `lmd::engine::render`                       | **entfernen**                                 |
| ~1319 Zeilen Core-Änderungen (config `[lean-md]`, `ctx_refactor`-Erweiterungen, `extension_registry::RenderTransform`, Wiring) | gemischt                                         | **Upstream-Audit** (§4.2)                     |

---

## 3. Outbound-Architektur — `CodeIntelBackend`

lean-md kapselt jeden Code-Intel-Zugriff hinter einem Trait mit zwei Impls.

```
trait CodeIntelBackend {
    fn call(&self, tool: &str, args: serde_json::Value) -> Result<String>;
}
```

### 3.1 `CliBackend` — **Default**

`lean-ctx call <tool> --project-root <root> --json '<args>'` (verifiziert:
`rust/src/cli/call_cmd.rs:194`, baut die Registry via `build_registry` selbst).

- **Stateless** — jeder Call ein frischer, kurzlebiger Prozess; ein Fehler = ein
  Exit-Code, trivial retrybar.
- **Keine `tool_profile`-Vorbedingung** — `call` dispatcht an **jedes** Tool,
  unabhängig vom Profil eines laufenden Servers.
- **Kein Verbindungszustand**, kein Endpoint-Discovery — braucht nur `lean-ctx` im
  PATH.
- Kosten: Prozess-Spawn-Latenz pro Call (bei Render bursty, kein Hot-Loop →
  verschmerzbar).

### 3.2 `McpBackend` — **opt-in**

lean-md als MCP-Client via `lean-ctx-client` gegen einen erreichbaren lean-ctx-MCP-
Endpoint. **Vorbedingung: `tool_profile = power`** (alle 72 Tools, inkl. der 16
Code-Intel-`ctx_*`).

- Vorteil: warme Verbindung → geringere Latenz bei *vielen* Calls.
- Voraussetzung: ein andialbarer Endpoint. Der stdio-Host-MCP des Agenten ist schon
  mit dem Agenten gepaart → nicht andialbar; ohne erreichbaren HTTP-Endpoint müsste
  lean-md einen eigenen `lean-ctx mcp`-Child spawnen (zweiter Prozess + State).
- Auswahl per Config/Env (z. B. `LEAN_MD_BACKEND=mcp` + `LEAN_MD_MCP_ENDPOINT=…`).

### 3.3 Topologie & das „behind"-Prinzip

```
Agent ──MCP──▶ lean-ctx (Host + Gateway)
                  │ spawnt stdio-Child (Addon, via `lean-ctx addon add`)
                  ▼
              lean-md (mcp)  ── Render-Kern in-process (rushdown/evalexpr)
                  │ Code-Intel-Direktive
                  └──▶ CodeIntelBackend
                         ├─ CliBackend:  `lean-ctx call ctx_refactor --json …`
                         └─ McpBackend:  lean-ctx-client ──▶ lean-ctx (power)
```

- **Inbound-„behind" bleibt bei beiden Backends:** der Agent spricht **nur**
  lean-ctx; lean-md ist immer Gateway-Child. Tools erreichbar via `ctx_tools`
  (find/call) durch das Gateway.
- **Das Gateway ist ein Einweg-Proxy** (Agent → Addon). Es bietet dem Addon
  **keinen Rückkanal**, um lean-ctx' *eigene* Tools zu konsumieren (in der Pipe ist
  lean-md Server, lean-ctx Client). ∴ der **Outbound-Leg verlässt das Gateway
  zwangsläufig** — bei **beiden** Backends. Der „zweite lean-ctx-Prozess" ist Folge
  der Einweg-Architektur, **kein** CLI-spezifischer Bruch; CliBackend ist davon die
  schlankere (stateless, kurzlebige) Variante.
- **Kein Re-Entrancy-Loop:** lean-md ruft outbound nur Code-Intel-Tools, **nie**
  `ctx_md_*` → das Gateway routet das an lean-ctx' eigenen Handler, nicht zurück.

### 3.4 Optionaler Zukunftspfad — Host-Callback (nicht v2-Scope)

Echte „Single-Server"-Reinheit erforderte eine **neue lean-ctx-Fähigkeit**: das
Gateway injiziert beim Spawn den Host-MCP-Endpoint in die Addon-Env
(`[mcp].env`, z. B. `LEAN_CTX_HOST_MCP=…`), den lean-md andialt → ruft dieselbe
Instanz, hinter der es läuft. Steht **nicht** im Addon-Kontrakt (#858/#863); als
Upstream-Folgevorschlag notiert, **kein** Blocker für v2 (CliBackend deckt alles).

---

## 4. Änderungen im lean-ctx-Repo

### 4.1 Reverse-Cut (§2.4)

`pub mod lmd` + `src/lmd/` + ctx_md-Registrierung + CLI-Pfad + wasm_ext-Test-Ref
entfernen. Danach **null** lmd-Symbole in `rust/src/` außerhalb von Doku/Hook.

### 4.2 Upstream-Audit der ~1319 Core-Zeilen

Trennen in:

- **Tool-Enhancement → behalten** (in lean-ctx, MCP-exponiert): jede `ctx_refactor`-
  o. ä. Erweiterung, die eine lean-md-Direktive braucht, muss über den Draht
  erreichbar bleiben. Diese Zeilen **bleiben** in lean-ctx.
- **Wiring/Render-Spezifik → entfernen**: `[lean-md]`-Config-Sektion,
  `extension_registry`-Registrierung der lmd-Engine, sonstiges in-tree-Wiring.

Output des Audits: eine Liste „behalten vs. entfernen" pro Hunk (Plan-Zeit-Artefakt).

### 4.3 Dünner Auto-Render-Delegations-Hook

Auto-Render-on-Read bleibt erhalten: `ctx_read` erkennt `.lmd.md` → **delegiert via
Gateway** an den lean-md-Addon (falls installiert), sonst Rohtext. Das ist die
**einzige** verbleibende (kleine, opt-in) lmd-Kenntnis in lean-ctx. Sie ersetzt die
alte `extension_registry::RenderTransform`-In-Tree-Kopplung durch eine Gateway-
Delegation.

### 4.4 Registry-Eintrag

Eintrag in `rust/data/addon_registry.json` (Shape laut Kontrakt §5.0):

```json
{
  "registry_version": 1,
  "addons": [
    {
      "addon": {
        "name": "lean-md",
        "display_name": "lean-md",
        "description": "Macro/directive markdown renderer for lean-ctx context engineering.",
        "author": "dasTholo",
        "homepage": "https://github.com/dasTholo/lean-md",
        "license": "Apache-2.0",
        "categories": [
          "workflow"
        ],
        "keywords": [
          "markdown",
          "macros",
          "dispatch",
          "skills"
        ],
        "min_lean_ctx": "3.8.12"
      },
      "mcp": {
        "transport": "stdio",
        "command": "lean-md",
        "args": [
          "mcp"
        ]
      }
    }
  ]
}
```

**Validator-Regeln** (`core::addons::registry::validate_entries`, läuft im
`cargo test`): eindeutiger Slug; installierbare Einträge brauchen
`author`/`homepage`/`license`/`description` (✓ erfüllt); **kein** Shell-out,
**kein** fetch-and-exec, **kein** non-HTTPS-Endpoint, **kein** unpinned Upstream.
`command = "lean-md"` ist ein direktes Executable (kein `npx`/`uvx`/`latest`,
kein `sh -c`, kein `curl`) → erzeugt **keine** `warn`/`danger`-Findings. Das ist
die Bedingung für die **verified**-Promotion (verified-Einträge müssen finding-frei
sein). Start-Tier **community** (installierbar, unaudited); Promotion zu **verified**
nach Upstream-Maintainer-Audit (Registry-controlled, §5.0).

---

## 5. Addon-Distribution (#858 / #863)

### 5.0 Verbindliche Referenz — `addon-manifest-v1` muss im neuen Repo bekannt sein

Der lean-md-Spec hängt am **stabilen v1-Kontrakt**
[`docs/contracts/addon-manifest-v1.md`](https://github.com/yvgude/lean-ctx/blob/main/docs/contracts/addon-manifest-v1.md)
(`yvgude`@main, Modul `core::addons`, CLI `lean-ctx addon`). Das neue Repo
`dasTholo/lean-md` **vendored/verlinkt diesen Kontrakt** (z. B. `docs/CONTRACT.md`
mit Quelle + gepinnter lean-ctx-Version), damit Manifest-Felder, Registry-Shape und
Install-Semantik dort die **Single Source of Truth** sind und nicht aus dem Gedächtnis
rekonstruiert werden. Manifest + Registry-Eintrag (§5.1/§4.4) sind exakt nach diesem
Kontrakt geformt.

### 5.1 Manifest (im lean-md-Repo, kontraktkonform)

```toml
[addon]
name = "lean-md"          # Slug [a-z0-9-], wird zum Gateway-Server-Namen
display_name = "lean-md"
version = "2.0.0"          # free-form
description = "Macro/directive markdown renderer for lean-ctx context engineering."
author = "dasTholo"
homepage = "https://github.com/dasTholo/lean-md"
license = "Apache-2.0"    # SPDX
categories = ["workflow"]
keywords = ["markdown", "macros", "dispatch", "skills"]
min_lean_ctx = "3.8.12"   # erste Version mit Addon-Ecosystem (informational)
# verified: NICHT setzen — registry-controlled, im handgeschriebenen Manifest
#           bedeutungslos. Trust verleiht die Registry, nicht der Eintrag selbst.

[mcp]
transport = "stdio"
command = "lean-md"       # Executable; muss im PATH installiert sein
args = ["mcp"]
```

**Installierbar vs. listed (Kontrakt):** der `[mcp]`-Block ist installierbar, weil
`stdio` ein nicht-leeres `command` hat. (Eine `listed-only`-Variante ohne `[mcp]`
wäre nur Verzeichnis-Eintrag — für lean-md nicht relevant, wir liefern einen
Endpoint.)

### 5.2 Install-Fluss

`lean-ctx addon add lean-md` → Registry-Resolve → Risk-Review der Wiring → nach
Bestätigung `[[gateway.servers]]` (global-only) → `gateway.enabled = true` → spawnt
`lean-md mcp`. Lokaltest vor Publikation: `lean-ctx addon add ./lean-ctx-addon.toml`.
Nach Install: MCP-Client neu starten, damit der Gateway-Katalog gelesen wird.

### 5.3 Geerbte Security (#863)

- **Trust-Tier** (verified/community) sichtbar in `list`/`info`/Install-Preview.
- **Risk-Review** der `[mcp]`-Wiring vor Install.
- **Untrusted-Output:** lean-mds Tool-Output wird server-seitig secret-redacted +
  audit-getaggt, **bevor** er den Agenten erreicht (`runtime::scrub_output`).
- **Opt-in OS-Sandbox** (`addons.sandbox`, `bwrap`/`sandbox-exec`) wickelt den
  gespawnten lean-md-Prozess ein.

### 5.4 Namespacing (offen, Plan-Zeit)

Gateway vergibt ggf. `lean-md::ctx_md_render`. **Anstreben:** transparenter/leerer
Namespace, damit `ctx_md_render` byte-identisch zum Phase-9-Namen bleibt
(rückwärtskompatibel). Falls Gateway das nicht erlaubt → Upstream-Folgeticket, **kein**
v2-Blocker.

---

## 6. Security-Modell — wer erzwingt was

Die Core-Primitive zerfallen in drei Eimer:

| Eimer                          | Primitive                                                                                    | Erzwingung im Addon-Modell                                                                                                                          |
|--------------------------------|----------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------|
| **Host erzwingt für lean-md**  | Redaction, Secret-Detection, Containment                                                     | #863: Addon-Runtime redacted Output server-seitig + OS-Sandbox um den Prozess. **Gratis.**                                                          |
| **Über MCP/CLI-Tool**          | alle 16 Code-Intel-Handler, knowledge, handoff, graph, shell                                 | `ctx_*` über CliBackend/McpBackend — **autoritative PathJail server-seitig inklusive.**                                                             |
| **Winziges generisches Local** | Pre-Flight-Pfad-Canonicalize (`jail_path`/`resolve_tool_path` ≈ canonicalize + Prefix-Check) | ~20 Zeilen stdlib in lean-md **oder** weglassen: relativen Pfad + `project_root` an den Tool-Call geben → Server jailt. Kein lean-ctx-Geheimwissen. |

**Kernaussage:** lean-md „verliert" PathJail/Redaction **nicht** — es war nie lean-mds
Aufgabe, sie zu *erzwingen*. Alle Datei-/Code-Operationen laufen über Tools, deren
autoritative Guards in lean-ctx (bzw. der Host-Runtime) sitzen.

---

## 7. Determinismus (#498)

- Embedded Seeds (`lean-md/core/*.lmd.md` via `include_str!`) wandern ins neue Repo;
  der `include_str!`-Pfad wird re-gerootet, die **Bytes bleiben identisch**.
- Das Fragment-Konsistenz-Gate (built-in == `lean-md/core/`-Dateien) wandert mit und
  muss byte-stabil grün bleiben.
- Tool-Output-Bodies bleiben content-adressiert, keine Timestamps/Counter.
- CliBackend/McpBackend liefern denselben Handler → byte-identische Code-Intel-
  Ergebnisse (#498 garantiert die Stabilität der lean-ctx-Outputs).

---

## 8. Test-/Parity-Strategie

Bestehende lmd-§8-Tests (#1–16, Direktiven-Verhalten) wandern ins neue Repo und
gelten **unverändert**. Neu:

1. **Standalone-Build/-Test:** `cargo build` / `cargo nextest run` im lean-md-Repo
   grün — **ohne** `lean_ctx`-Crate-Dep (nur `lean-ctx-client`). Beweist Abkopplung.
2. **Render-Kern standalone:** eine Code-Intel-freie `.lmd.md` rendert ohne
   laufenden lean-ctx (kein Backend-Call).
3. **Backend-Parity:** dieselbe Code-Intel-Direktive über `CliBackend` und
   `McpBackend` liefert byte-identisches Ergebnis.
4. **Addon-Roundtrip:** nach `lean-ctx addon add ./lean-ctx-addon.toml` ist
   `ctx_md_render`/`ctx_md_check` über den lean-ctx-Server erreichbar und das
   Ergebnis byte-identisch zum direkten `lean-md mcp`-Call.
5. **Auto-Render-Delegation:** `ctx_read` einer `.lmd.md` mit installiertem Addon
   == direktes `ctx_md_render`; ohne Addon → Rohtext (Hook §4.3).
6. **Reverse-Cut-Gate (lean-ctx):** kein `lmd`/`lean-md`-Symbol in `rust/src/`
   außerhalb von Doku + Delegations-Hook; lean-ctx baut ohne `rushdown`/`evalexpr`.
7. **Determinismus-Erhalt:** Fragment-Konsistenz-Gate grün nach `include_str!`-
   Re-Root; Seeds byte-diff vor/nach == leer.

Alle via `cargo nextest run` (Projekt-Hard-Rule).

---

## 9. Risiken & Reihenfolge

| Risiko                                                                                                         | Mitigation                                                                                                     |
|----------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------|
| **Dieser Branch hat 0 Addon-Code** (verifiziert: 0 Treffer) — `lean-ctx addon` lebt upstream (`yvgude` 3.8.12) | **T0-Vorbedingung:** lean-ctx erst auf eine Basis mit #858/#863 rebasen/mergen, `lean-ctx addon` verifizieren. |
| Upstream-Audit (§4.2) übersieht eine Tool-Capability, die eine Direktive braucht                               | Audit-Liste pro Hunk; Backend-Parity-Test (#3) + Direktiven-Tests (#1–16) decken den Verlust auf.              |
| `git filter-repo` verliert Historie/Seeds                                                                      | Extraktion aus `feat-lmd-v1`; Seed-Byte-Diff (#7) vor/nach; Trockenlauf in Scratch-Klon.                       |
| Gateway-Namespace ändert Tool-Namen                                                                            | §5.4 — transparenten Namespace anstreben; sonst Folgeticket.                                                   |
| CliBackend-Latenz bei vielen Code-Intel-Direktiven                                                             | McpBackend opt-in als Performance-Pfad (§3.2).                                                                 |
| Endpoint-Discovery für McpBackend                                                                              | CliBackend-Default umgeht das vollständig; McpBackend nur bei explizit gesetztem Endpoint.                     |

**Reihenfolge (für `writing-plans`):**

- **T0** lean-ctx-Basis mit Addon-Ecosystem (#858/#863) herstellen + verifizieren.
- **T1** `src/lmd` → neues Repo `dasTholo/lean-md` via `git filter-repo` (Historie).
- **T2** `crate::core::*`/`crate::tools::ctx_*::handle`-Aufrufe durch
  `CodeIntelBackend` (Cli default, Mcp opt-in) ersetzen; tree-sitter-Dep droppen;
  Pre-Flight-Pfad-Helfer (§6, Eimer 3).
- **T3** bin: `lean-md mcp` (Server) + `lean-md render|check` (CLI); `include_str!`-
  Seeds re-rooten; Determinismus-Gate (#7).
- **T4** lean-ctx-seitig: Reverse-Cut (§4.1) + Upstream-Audit (§4.2) + Auto-Render-
  Delegations-Hook (§4.3) + `addon_registry`-Eintrag (§4.4).
- **T5** `lean-ctx-addon.toml` + Doku; `addon add`-Roundtrip-Test (#4).
- **T6** Parity-/Integrations-Gates (#1–7).

---

*Status: v2-draft (2026-06-25). Ersetzt `2026-06-22-lmd-phase-12-side-loaded-crate-
design.md`. Verifiziert: Addon-Ecosystem `yvgude`@main (#858/#863), `lean-ctx call`
(`call_cmd.rs:194`, `build_registry`), Kopplungs-Audit gegen `feat-lmd-v1`
(`src/lmd`: path_resolve/pathjail flach, redaction nur Doc-Kommentar), 0 Addon-Code
auf diesem Branch.*
