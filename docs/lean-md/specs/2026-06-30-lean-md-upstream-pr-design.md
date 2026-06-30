---
title: lean-md — Upstream-PR (Addon-Integration in lean-ctx) & crates.io-Distribution
slug: lean-md-upstream-pr
status: draft
date: 2026-06-30
integrates: docs/lean-md/specs/2026-06-26-lean-md-standalone-addon-design.md
lean_ctx_version: 3.8.16
consumer: ai
note: >
  Design für einen merge-fähigen PR feat-lean-md-addon → yvgude/lean-ctx:main,
  der lean-md als externes lean-ctx-Addon integriert, plus die Distribution über
  crates.io (cargo install), damit ein externer User das Addon per `addon add`
  installieren kann. Hält außerdem die PR-Beschreibung (Lean-md_PR.md, untracked,
  englisch) nach .github/pull_request_template.md fest.
---

# lean-md — Upstream-PR & crates.io-Distribution

> **Ziel:** Ein merge-fähiger PR `feat-lean-md-addon` → `yvgude/lean-ctx:main`,
> der lean-md als externes Addon in lean-ctx integriert (Auto-Render-Delegation,
> Registry-Eintrag, Formatter-Routing, Tests) **und** ein extern installierbares
> lean-md über crates.io bereitstellt (`[install] manager = "cargo"`). Begleitet
> von einer englischen PR-Beschreibung `Lean-md_PR.md` (untracked) nach dem
> Upstream-Template.

## 0. Kontext & Ausgangslage

- **Upstream-Addon-System (Stand 3.8.16):** ausgereift. Manifest-Contract `v1`
  *stable* (`docs/contracts/addon-manifest-v1.md`). Registrierung = Eintrag in
  `rust/data/addon_registry.json` (Merge-Request). Es gibt `[capabilities]`-
  Sandbox, ein Audit-Gate (`lean-ctx addon audit`), den Registry-Validator
  (`lean-ctx addon registry validate`), Trust-Tiers (verified/community),
  optionale `sha256`-Binärpins, `[pricing]` und den **Bootstrap-Engine**
  (`[install]`-Block mit `uv|pip|cargo|npm|brew|dotnet`, fixe argv-Templates,
  kein Shell/`curl|sh`).
- **lean-md heute:** standalone Crate (`lean-md` = lib `lean_md` + bin
  `lean-md`), Render-Kern in-process (`rushdown` + `evalexpr`), Code-Intel
  outbound über `CodeIntelBackend` (CLI default, MCP opt-in). MCP-Server via
  `lean-md mcp` (2 Tools: `ctx_md_render`, `ctx_md_check`). Manifest
  `lean-ctx-addon.toml` vorhanden, community-Tier.
- **Repos & Remotes:** `upstream = yvgude/lean-ctx` (PR-Ziel),
  `origin = dasTholo/lean-ctx` (Fork). Arbeitsbranch: `feat-lean-md-addon`.
- **Verifiziert (2026-06-30):** lokale Installation `lean-ctx addon add
  /…/lean-md/lean-ctx-addon.toml` läuft sauber durch; nach MCP-Neustart zeigt
  `ctx_tools list` → `lean-md [stdio, enabled] — 2 tool(s)`. Gateway erzwingt den
  Tool-Prefix `lean-md::` (R2-Befund, kein Blocker).

## 1. Problem

Der bestehende `lean-md`-Registry-Eintrag ist **fabricated wiring**: Er hat einen
`[mcp]`-Block (`command = "lean-md"`), aber **keinen** `[install]`-Block und keine
externe Binär-Quelle. Für einen externen User schreibt `addon add lean-md` zwar
die Gateway-Config, aber das Binary existiert nirgends → erster Tool-Call schlägt
mit „command not found" fehl. Genau das verbietet der Guide („never with
fabricated wiring") — ein Maintainer lehnt das zu Recht ab.

Zusätzlich enthält die Registry noch den **veralteten** `lmd`-Eintrag (listed,
ohne `[mcp]`, `homepage → feat-lmd-v1`), der vom standalone `lean-md` abgelöst
wird.

## 2. Scope

**In Scope (voller Integrations-Scope):**

1. **Distribution** über crates.io (`cargo install lean-md`) → externer User kann
   `lean-ctx addon add lean-md` als One-Click ausführen.
2. **Manifest-Anpassung** (`[install]`-Block + `network = "full"`).
3. **Upstream-PR** mit der Code-Integration, dem installable Registry-Eintrag und
   der Entfernung des alten `lmd`-Eintrags.
4. **Verifikations-Kette** als Akzeptanzkriterien.
5. **PR-Beschreibung** `Lean-md_PR.md` (untracked, englisch) nach
   `.github/pull_request_template.md`.

**Out of Scope (Folge):** transparenter Tool-Namespace (Gateway-Prefix-Befund
R2, Upstream-Folgeticket), Host-Callback, `McpBackend`-Reife, prebuilt-Binär-
Distribution (npm-Wrapper/GitHub-Release-Assets).

## 3. Distribution — crates.io + `cargo install`

### 3.1 Warum cargo/crates.io

`cargo install lean-md` mit **Default-Features** baut from source und braucht nur
crates.io-Crates (`rushdown`, `evalexpr`, `serde_json`, `regex`, `chrono`). Die
`lean-ctx-client`-Dependency hängt ausschließlich am **optionalen** `mcp`-Feature
(`default = []`) und wird beim Default-Install **nicht** gezogen — die
null-`lean_ctx`-Invariante des standalone Crates bleibt gewahrt. Der
Default-Backend (`CliBackend`) ruft `lean-ctx call`, das der User ohnehin hat.

> **Begründung gegen die Alternativen:** Der lean-ctx Bootstrap-Engine kann
> **nicht** direkt aus einem GitHub Release herunterladen (Non-Goal: kein
> `curl|sh`/fetch-and-exec; nur die 6 Package-Manager mit fixen argv-Templates).
> Ein npm-Wrapper (prebuilt, wie `packages/lean-ctx-bin`) wäre möglich, wurde
> aber verworfen. Gewählt: **cargo/crates.io** (from source, kein prebuilt nötig,
> kein Zwischen-Wrapper).

### 3.2 crates.io-Publikation (lean-md-Repo)

- **Name:** `lean-md` ist auf crates.io frei (Suche findet nur `lean2md`).
- **Publish-Inhalt:** Default-Features; `lean-ctx-client` bleibt außen vor.
- **Stolperstein `[[example]]`:** Das Example zeigt auf
  `benchmarks/skill-token-comparison/main.rs`, das **nicht** im `include` von
  `Cargo.toml` steht → `cargo publish` bricht ab (Target-Pfad fehlt im Paket).
- **Entscheidung — Benchmarks NICHT ins Release:** `benchmarks/` darf **nicht**
  Teil des publizierten Crates sein. Daher **nicht** ins `include` aufnehmen,
  sondern das `[[example]]`-Target aus dem **publizierten** Manifest entfernen,
  sodass cargo den `benchmarks/`-Pfad gar nicht erst verlangt. Umsetzung:
  - **bevorzugt:** Benchmark in einen eigenen, nicht-publizierten Workspace-/Sub-
    Crate (`publish = false`) auslagern und das `[[example]]` aus dem lean-md-
    Manifest streichen — Benchmark bleibt lokal lauffähig, Release bleibt schlank;
  - **minimal:** das `[[example]]` aus `Cargo.toml` entfernen (Dateien bleiben im
    Repo unter `benchmarks/`, sind aber kein Cargo-Target und nicht im `include`
    → fließen nicht ins Crate).
  Das `include` bleibt in **beiden** Fällen ohne `benchmarks/`.
- **Gate:** `cargo publish --dry-run` muss grün sein **und** das gepackte Crate
  (`cargo package --list`) darf **keine** `benchmarks/`-Datei enthalten, bevor
  real publiziert wird.
- **Version:** `0.1.0` (SSOT mit `[addon].version` im Manifest).

### 3.3 Publish nur von `main` + GitHub Action

- **Branch-Policy:** crates.io-Publish erfolgt **ausschließlich vom `main`-Branch**
  des lean-md-Repos. Feature-Branches publizieren **nie**.
- **GitHub Action (einzurichten, lean-md-Repo):** Ein dedizierter Release-
  Workflow, der den Publish automatisiert. Anforderungen:
  - **Trigger:** nur auf `main` — entweder Push eines Version-Tags (`v*`) auf
    `main` oder ein manueller `workflow_dispatch`, der auf `main` eingeschränkt
    ist. Kein Publish auf Push in Feature-Branches.
  - **Schritte:** Checkout → Rust-Toolchain → `cargo publish --dry-run` (Gate) →
    `cargo publish` mit `CARGO_REGISTRY_TOKEN` (GitHub Secret).
  - **Idempotenz:** Publish einer bereits existierenden Version schlägt
    erwartungsgemäß fehl → Version-Bump ist Pflicht pro Release.
  - **Guard:** Workflow prüft, dass `[addon].version` (Manifest) == `package.version`
    (Cargo.toml) == Tag, bevor publiziert wird.

### 3.4 Manifest-Anpassung (`lean-ctx-addon.toml`)

Zwei Änderungen am bestehenden Manifest:

```toml
[install]
manager = "cargo"
package = "lean-md"
version = "0.1.0"
bin     = "lean-md"

[capabilities]
network    = "full"          # geändert (war "none")
filesystem = "read_write"
exec       = ["lean-ctx"]
```

**Warum `network = "full"` Pflicht ist:** Ein deklarierter `[install]`-Block
setzt `trust::wiring_uses_network(manifest) == true` (`trust.rs:243` — „ein
`[install]`-Block fetcht ein Paket aus einer Registry → braucht Netzwerk").
Bleibt `network = "none"`, erzeugt das Audit `cap_net_underdeclared`
(`RiskLevel::Danger`, **blocking**, `audit.rs:128–135`; auch im Registry-Validator
`registry.rs:186`) → der Eintrag wäre nicht listbar. `cargo install` lädt
tatsächlich aus dem Netz, also ist `full` **ehrlich** deklariert.

> **Trade-off:** Die laufende Addon-Runtime verliert die Egress-Sandbox (obwohl
> sie selbst kein Netz bräuchte — `CliBackend` ruft lokal `lean-ctx`). Das ist
> der dokumentierte Preis der `[install]`-Route und transparent (`exec` +
> `filesystem` bleiben minimal deklariert). Akzeptiert.

## 4. Upstream-PR — Inhalt

PR `feat-lean-md-addon` → `yvgude/lean-ctx:main`. Kern = Commit `89aa115ab`
(„integrate lean-md as external lean-ctx addon") + Test-Fix `334f6b37e`, ergänzt
um die Registry-Anpassungen aus §3.4/§5.

| Bereich | Datei(en) | Zweck |
|---|---|---|
| Auto-Render-Hook | `rust/src/tools/registered/ctx_read.rs` | `.lmd.md` → Gateway-Delegation an lean-md, raw-Fallback |
| Extension-Infra | `rust/src/core/extension_registry.rs`, `core/cache.rs` | `RenderTransform`-Trait-Gerüst |
| Formatter-Routing | `rust/src/tools/ctx_refactor.rs`, `rust/src/lsp/format/mod.rs`, `lsp/mod.rs` | extension-basiertes Reformat (`.rs`→rustfmt) + headless Fallback |
| Registry | `rust/data/addon_registry.json` | **+** installable `lean-md` (community, `[install] cargo`, `network=full`), **−** alter `lmd` listed-Eintrag |
| Doku | `docs/reference/21-lean-md.md`, `appendix-mcp-tools.md` | Addon-Integrations-Referenz |
| Tests | `rust/tests/reverse_cut_gate.rs`, `auto_render_delegation.rs`, `integration_tests.rs`, `ctx_refactor_tests.rs` | Reverse-Cut + Delegation + Raw-Fallback-Gates |
| Aufräumen (bewusst) | `docs/superpowers/*`, `rust/.config/nextest.toml` | Löschung eigener, früher versehentlich durchgerutschter Docs — siehe §6 |

**Registry-Eintrag (Ziel-Zustand `lean-md`):**

```json
{
  "addon": {
    "name": "lean-md",
    "display_name": "lean-md",
    "description": "Macro/directive markdown renderer for lean-ctx context engineering.",
    "author": "dasTholo",
    "homepage": "https://github.com/dasTholo/lean-md",
    "license": "Apache-2.0",
    "categories": ["workflow"],
    "keywords": ["markdown", "macros", "dispatch", "skills"],
    "min_lean_ctx": "3.8.12"
  },
  "mcp": { "transport": "stdio", "command": "lean-md", "args": ["mcp"] },
  "install": { "manager": "cargo", "package": "lean-md", "version": "0.1.0", "bin": "lean-md" },
  "capabilities": { "network": "full", "filesystem": "read_write", "exec": ["lean-ctx"] }
}
```

- Alten `lmd`-Eintrag **entfernen** (alleiniger Nachfolger: `lean-md`).
- `min_lean_ctx` bleibt `3.8.12` (Addon-Schnittstelle seit 3.8.12 stabil; ein
  höheres Minimum würde grundlos Nutzer ausschließen).
- Trust-Tier **community**, **kein** `sha256` (from-source-Build → Hash je Build/
  Plattform verschieden; verified/paid wird ohnehin registry-seitig vergeben).

## 5. Reihenfolge (kritisch)

1. lean-md-Repo: `[[example]]`/`include` fixen → `cargo publish --dry-run` grün.
2. GitHub-Action (§3.3) einrichten; `main` auf Version `0.1.0` bringen.
3. **`cargo publish`** von `main` (via Action) → `lean-md 0.1.0` auf crates.io.
4. Lokal verifizieren: `cargo install lean-md --version 0.1.0` installiert das Bin.
5. Manifest + Registry-Eintrag auf `[install] cargo` + `network=full` umstellen.
6. **Erst dann** der installable Upstream-PR. (Reihenfolge verhindert fabricated
   wiring: `addon add lean-md` setzt ein publiziertes Paket voraus.)

## 6. Bewusste Löschungen — Behandlung im PR

Der 3-Punkt-Diff `upstream/main...feat-lean-md-addon` entfernt 6 Dateien unter
`docs/superpowers/` (+ eine `nextest.toml`-Zeile, ~1610 Zeilen). git schreibt sie
Yves Gugger zu (Merge-Commit), es sind aber **eigene, in einem früheren PR
versehentlich durchgerutschte Dokumente von dasTholo**, die hier aufgeräumt
werden. Entscheidung: Die Löschungen **bleiben** Teil des PRs und werden in
`Lean-md_PR.md` unter „Notes for reviewers" **transparent** angemerkt (Datei-
Liste + Begründung „cleanup of my own docs that slipped into an earlier merge"),
damit der Maintainer keine Überraschung im Diff findet.

## 7. „Lauffähig machen" — Verifikations-Kette (Akzeptanzkriterien)

Exakt die Schritte aus `docs/guides/addons.md` + `addon-manifest-v1.md`:

1. `lean-ctx addon audit /…/lean-md/lean-ctx-addon.toml` → Verdict `pass`/`review`
   (Coherence: `network=full` ↔ `[install]`; `exec=["lean-ctx"]` deklariert ↔
   Callback). **Kein** `cap_net_underdeclared`.
2. `lean-ctx addon registry validate rust/data/addon_registry.json` → grün
   (eindeutiger Slug, author/homepage/license/description vorhanden, kein
   Shell-out, gepinnte `[install]`-Version).
3. `cargo install lean-md --version 0.1.0` → `lean-md` im PATH (after publish).
4. `lean-ctx addon add lean-md -y` → `cargo install` läuft, Gateway gewired.
5. MCP-Client-Neustart → `ctx_tools list` zeigt `lean-md [stdio, enabled] —
   2 tool(s)` (lokal bereits bestätigt).
6. Tool-Roundtrip: `lean-md::ctx_md_render` liefert byte-identisch zu
   `lean-md render` (Namespacing-Prefix `lean-md::` ist erwartet, R2).

**CI-Gates (PR-Template-Checkboxen, via `.github/workflows/ci.yml` im
lean-ctx-Repo):** `cargo test`, `cargo clippy --all-targets --all-features -D
warnings`, `cargo fmt --check`. Die Live-Addon-Gates (`addon_roundtrip`,
Delegation-mit-Addon) bleiben `#[ignore]` (kein Addon im CI installiert), werden
aber im PR-Text als manuell verifiziert dokumentiert.

## 8. `Lean-md_PR.md` — Struktur & Ablage

- **Ablage:** `/home/tholo/Scripts/lean-ctx/Lean-md_PR.md` (Repo-Root, neben
  `.github/`). **Untracked** — nicht committen; via `.git/info/exclude`
  ausgeschlossen, damit es nicht im PR-Diff landet.
- **Sprache:** **Englisch** (Upstream-PR).
- **Gliederung** exakt nach `.github/pull_request_template.md`:
  - **Summary** — Was (lean-md als externes Addon; Reverse-Cut in-tree `lmd` →
    standalone Repo) + Warum (Entkopplung ohne Fork/Recompile, nutzt das in 3.8.x
    gereifte Addon-Ökosystem) + Kernpunkte (`.lmd.md` Auto-Render-Delegation,
    installable Registry-Eintrag via `cargo install`, Formatter-Routing-
    Extraktion, Reverse-Cut-Gates).
  - **Test plan** — die 3 CI-Checkboxen + Zeile für die manuelle Live-Addon-
    Verifikation (audit → publish → install → add → ctx_tools → roundtrip).
  - **Notes for reviewers** — Risk areas (ctx_read Delegations-Hook mit raw-
    Fallback; Formatter-Routing); Backwards compat (kein `[capabilities]`-Zwang
    für bestehende Manifeste; `lmd`-Entfernung ist Registry-only); **Removed
    files**-Hinweis (§6, transparent); offene Punkte/Folgetickets (transparenter
    Namespace, Host-Callback, CI-Job für Live-Gates); Docs updated.
  - **Contributor License Agreement** — CLA-Hinweis aus dem Template; falls
    First-time-Contributor die Signatur-Zeile „I have read the CLA Document and I
    hereby sign the CLA" als Reminder.

## 9. Offene Punkte / Folgetickets (Upstream)

- **Transparenter Tool-Namespace:** Gateway erzwingt `lean-md::`-Prefix
  (`gateway/catalog.rs`). Opt-in transparenter Namespace als Upstream-Folgeticket
  (kein PR-Blocker; Tests nutzen Prefix-Namen).
- **Host-Callback:** Gateway injiziert Host-MCP-Endpoint in Addon-Env (würde den
  `McpBackend` ohne separaten `lean-ctx call` ermöglichen).
- **CI-Job für Live-Addon-Gates:** sobald ein Addon im CI installiert werden kann.

## 10. Akzeptanzkriterien (Definition of Done)

- [ ] `lean-md 0.1.0` auf crates.io (publiziert von `main` via GitHub Action).
- [ ] `cargo install lean-md --version 0.1.0` installiert das Binary sauber.
- [ ] `cargo package --list` enthält **keine** `benchmarks/`-Datei (Release schlank).
- [ ] Manifest + Registry-Eintrag mit `[install] cargo` + `network=full`; alter
      `lmd`-Eintrag entfernt.
- [ ] `lean-ctx addon audit` → `pass`/`review` ohne `cap_net_underdeclared`.
- [ ] `lean-ctx addon registry validate` grün.
- [ ] `addon add lean-md` → MCP-Neustart → `ctx_tools` zeigt 2 Tools; Roundtrip
      byte-identisch.
- [ ] CI grün: `cargo test`, `cargo clippy … -D warnings`, `cargo fmt --check`.
- [ ] `Lean-md_PR.md` (untracked, englisch) nach Template, inkl. transparentem
      Removed-files-Hinweis.
