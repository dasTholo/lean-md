# Design-Spec: P3 — `kind=skills`-Pack (Full-Cut)

> Erstellt: 2026-07-08 · Revidiert: 2026-07-09 (**Wiring-Discovery abgeschlossen** — Task 1
> entfällt; zwei lean-ctx-Upstream-Blocker gefunden, Vertrag designt)
> Branch: `feat-lmd-v2` · Roadmap-Phase: **P3** (Referenzfall #727)
> Vorgänger: P0 + P1 im Code fertig (5-leg `[artifacts]`-Matrix, `9856641..689f74c`).
> **Vorbedingung (NEU, extern):** lean-ctx-Upstream-PR gemäß
> `lean-ctx/docs/lean-md/specs/2026-07-09-addon-pack-dependencies-design.md`.
> Terminal-Ziel dieser Spec: `lmd-writing-plans` → Implementierungsplan.

---

## 1. Ziel & Kernentscheidung

Die **Skill-Inhalte** verlassen das Binary. Heute sind Skill-Bodies, Companions,
SKILL.md-Stubs, Browser-Companion-Scripts **und die skill-lokalen `_includes/`-Seeds**
über `include_str!` ins `lean-md`-Binary eingebacken (#727 nennt genau das als
Auslager-Kandidaten). Diese Spec lagert sie in **einen** signierten, versionierten
**`kind=skills`-Pack** `@dasTholo/lean-md-skills` aus, deklariert als **depth-1-Dependency**
des Addons `@dasTholo/lean-md`. Resultat: Binary schrumpft; Skill-Updates ohne
Binary-Release.

**Gewählte Strategie: Full-Cut, Ein-Pack.** Der Skill-Content-`include_str!`-Payload wird
**entfernt**; der Pack ist die **einzige** Body-/Companion-/Asset-/`_includes/`-Quelle in
Produktion. **Ein** Pack für allen Skill-Content (keine Aufteilung in mehrere Skill-Packs —
siehe §8 Tiering).

**Was NICHT wandert (bewusst embedded):**

- **Cross-Skill-Core-Primitive** — `hard-rules`, `dispatch-contract`, `parallel-dispatch`
  (`content/core/…`) — allgemeine lmd-Primitive, die **jedes** `.lmd.md` nutzt (auch ein
  User-Plan via `@dispatch`/`@include hard-rules`, ohne je eine Skill zu rendern).
- **Gloss-Tabelle** — `content/gloss/directives.lmd.md` — render-time bei **jedem**
  Directive-Lookup gebraucht.
- **Projekt-Seeds** — `content/lang/*`, `content/tooling/*`, `content/templates/*`
  (`seeds.rs::PROJECT_SEEDS`) — separater Install-Zeit-Kanal (§6).

Grund: Diese halten das Binary in **jedem** Distributionspfad ein self-contained
General-Renderer (verifiziert: `render/check/source/mcp` in `src/bin/lean_md.rs` arbeiten
auf beliebigen `.lmd.md`; nur `render --skill X` fasst Skill-Content an). Der Shrink-Nutzen
dieser Primitive ist ohnehin winzig — der große Payload sind die Skill-Bodies.

**Reverse-Cut bleibt intakt:** Der Pack trägt nur Content. Kein Engine-Symbol
(`rushdown`/`evalexpr`) wandert; lean-ctx bekommt keine Render-Dependency.

---

## 2. Wiring-Discovery — abgeschlossen (2026-07-09)

Die frühere Task-1-Discovery ist **durchgeführt** (gegen `/home/tholo/Scripts/lean-ctx`,
Branch `pr-rebuild`, v3.9.3-Merge) und von einem unabhängigen Reviewer gegen die Quelle
verifiziert. Ergebnis: der **Consumer-Pfad ist vollständig**, aber **zwei Blocker** liegen
auf dem Autoren-/Auslieferungs-Pfad. Beide sind lean-ctx-Upstream-Arbeit, nicht lean-md.

### 2.1 Was existiert (verifiziert)

| Fakt                               | Fundstelle (lean-ctx)                      | Aussage                                                                                  |
|------------------------------------|--------------------------------------------|------------------------------------------------------------------------------------------|
| **`PackageKind::Skills`**          | `core/context_package/manifest.rs`         | schema v2                                                                                |
| **`PackageManifest.dependencies`** | `core/context_package/manifest.rs:120-126` | `Vec<PackageDependency>`; Felder: `name`, **`version_req`**, `optional`                  |
| **`ResolvedDep`**                  | `core/context_package/deps.rs:23-34`       | `{name, namespace, slug, version, artifact_sha256}`                                      |
| **Depth-1-Resolver**               | `deps.rs:40-99`                            | `@ns/name`-scoped, SemVer-Range, keine Self-Dep, höchste non-yanked Version              |
| **Registry-Auflösung**             | `deps.rs:77`                               | `remote::fetch_versions(registry_base, …)` — **nur** Registry-Index                      |
| **`addon add`-Verdrahtung**        | `cli/addon_cmd.rs:366,382,413`             | resolve → Consent-Preview → `install_declared_dependencies`                              |
| **Skills-Materialisierung**        | `context_package/skills.rs:261`            | `skills_dir(store_root, name, version)`; `@ns/name` → `@ns__name` on disk                |
| **Env-Durchreichung**              | `core/addons/env_scrub.rs:33,45-47`        | `declared_env` (`[mcp.env]`) wird **nach** `env_clear()` gesetzt → keine Allowlist nötig |

### 2.2 Die zwei Blocker (Upstream)

1. **Kein Dependency-Authoring.** `AddonManifest` (`core/addons/manifest.rs:83-111`) hat
   **kein** `dependencies`-Feld; `build_addon_pack` setzt hart
   `dependencies: Vec::new()` (`core/addons/publish.rs:135`). Ein `[[dependencies]]`-Block
   im `lean-ctx-addon.toml` wird **still verworfen** (kein `deny_unknown_fields`).
   ∴ Auch der hosted-Pfad (§3) liefert heute ein `pack_manifest` **ohne** Deps.

2. **Kein Pack-Pfad-Handoff.** `skills_dir(...)` erreicht das gespawnte `lean-md` nie. Der
   `[artifacts]`-Rewrite von `mcp.command` (`addon_cmd.rs:474`) existiert nur für Binaries.
   `LEAN_CTX_DATA_DIR`/`XDG_DATA_HOME` stehen nicht in `BASE_ENV_ALLOWLIST`
   (`core/addons/capabilities.rs`) → das Addon kann den Store-Root nicht einmal selbst
   ableiten.

Ein dritter Befund macht beide zu **stillen** Fehlern: **`addon.min_lean_ctx` wird nirgends
gelesen** (`core/addons/manifest.rs:48`, tot). Eine alte lean-ctx installiert das Addon,
ignoriert `[[dependencies]]` und wired es trotzdem.

### 2.3 Der designte Vertrag (Upstream-PR)

Spezifiziert in `lean-ctx/docs/lean-md/specs/2026-07-09-addon-pack-dependencies-design.md`:

1. `AddonManifest.dependencies: Vec<PackageDependency>`; `publish.rs` reicht sie durch.
   Authoring-Key ist **`version_req`** (nicht `version`), kein serde-Alias.
2. Neues `core/addons/pack_env.rs`: reine Funktion
   `expand_pack_env(declared_env, &[ResolvedDep]) -> Result<BTreeMap<String,String>, String>`.
   Der Autor deklariert `LEAN_MD_SKILLS_DIR = "{pack_dir:@dasTholo/lean-md-skills}"` in
   `[mcp.env]`; lean-ctx expandiert den Platzhalter beim Wiring zum absoluten
   `skills_dir(store_root, name, resolved_version)`. Unbekannter Pack / unbekanntes Schema
   → harter Fehler, nichts wird gewired.
3. `min_lean_ctx`-Gate in `install::preflight` gegen `env!("CARGO_PKG_VERSION")` — **abort**,
   nicht warn.
4. Dep-Install rückt **vor** `provision_and_wire`; `provision_and_wire` nimmt die aufgelösten
   Deps als Parameter (zwingt `cmd_update` zur Symmetrie — heute ein Live-Bug:
   `addon_cmd.rs:728` ruft `:819` ohne `resolve_dependencies`).

**Versions-Kopplung: Lockstep.** Der Pack ships als **`0.2.0`**, dieselbe Version wie
lean-md-Crate und Addon; deklariert als `version_req = "^0.2"` (auf einer `0.x`-Linie:
`>=0.2.0, <0.3.0`). Der Mechanismus bleibt eine SemVer-Range; Lockstep ist eine
lean-md-Release-**Policy**: Binary und Pack werden aus demselben Tag geschnitten. Preis:
ein reiner Content-Fix erzwingt einen Bump, und weil das Binary die Linie teilt, wird es
mitgebumpt.

---

## 3. Distributions-Modell — **hosted ctxpkg-Pack ist Pflicht**

Zwei Kanäle, nicht einer. Das **Binary** kommt aus dem GitHub-Release über
`[artifacts.<triple>]` (#725) — kein crates.io, kein `[install]`-Bootstrap. Der **Pack**
kann diesen Kanal nicht nutzen: `resolve_one` (`deps.rs:77`) löst Versionen ausschließlich
über den Registry-Index auf. Ein Pack, der nur als GitHub-Asset existiert, ist für den
Resolver unsichtbar → `addon add` scheitert mit „no installable version matches".

Zusätzlich feuert die Dep-Resolution **nur**, wenn ein Package-Manifest `pm` vorliegt — und
`pack_manifest = Some(pm)` entsteht **ausschließlich** im hosted-`@ns/name`-Pfad
(`fetch_addon_pack`). Die anderen zwei Auflösungspfade setzen es **nicht**:

| `addon add <target>`                 | Pfad        | `pack_manifest` | Deps aufgelöst? |
|--------------------------------------|-------------|-----------------|-----------------|
| `./lean-ctx-addon.toml`              | local       | `None`          | **nein**        |
| `lean-md` (bundled/curated registry) | curated     | `None`          | **nein**        |
| `@dasTholo/lean-md` (hosted ctxpkg)  | hosted pack | **`Some(pm)`**  | **ja, depth-1** |

∴ **lean-md MUSS als hosted ctxpkg-`kind=addon`-Pack ausgeliefert werden**
(`addon publish --namespace dasTholo` → Nutzer: `addon add @dasTholo/lean-md`), **und** der
Skills-Pack muss hosted publiziert sein. Hosted-Publish (#726/P2) ist damit von „optional,
empfohlen" zur **harten Vorbedingung** von P3 geworden (V4, §5.1).

**Das GH-Binary bleibt der Auslieferungsweg** — das `lean-ctx-addon.toml` mit dem
5-leg-`[artifacts]`-Block (P0/P1) wird verbatim in den `kind=addon`-Pack gewrappt. Ablauf
bei `addon add @dasTholo/lean-md` (nach dem Upstream-PR, §2.3):

1. `preflight` — `min_lean_ctx`-Gate.
2. `resolve_dependencies(pm)` → höchstes non-yanked Match von `@dasTholo/lean-md-skills`
   gegen `^0.2`.
3. Consent-Surface listet den Pack (`+ @dasTholo/lean-md-skills@0.2.0`).
4. `install_declared_dependencies` materialisiert den Pack read-only + SHA-verifiziert nach
   `skills_dir(store_root, name, version)`.
5. Binary aus **GitHub-Release** (`[artifacts]`) → managed bin dir, sha256-gepinnt.
6. `[mcp.env]`-Expansion: `LEAN_MD_SKILLS_DIR` → absoluter Pack-Pfad; Wiring.
7. Lockfile pinnt Binary **und** Pack — zweiter Install offline-reproduzierbar.

**Curated-Registry-Eintrag nach dem Full-Cut (Task 1):** Ein via curated `addon add lean-md`
(bundled `addon_registry.json`) installiertes Binary hat nach dem Cut **keinen** Skill-Content
und zieht den Pack **nicht** (Tabelle oben) → jedes `render --skill` wäre hart-fehlerhaft.
Gewählt: **(b) auf `listed` zurückstufen** (Homepage-Pointer, kein one-click), bis der hosted
Pack live ist. Alternativen waren (a) aus dem curated Registry entfernen, (c) skill-los
stehenlassen. Der Entry liegt im lean-ctx-Repo (`rust/data/addon_registry.json`, generiert
via `gen_registry`) — hier nur als Decision-Record.

> **Separater Arbeitsblock, nicht P3:** derselbe Entry pinnt heute noch
> `install: manager=cargo` und hat **keinen** `artifacts`-Block → `addon add lean-md` ist
> **unabhängig von P3** rot. Fix = Registry-Arbeit nach dem `v0.2.0`-Release (echte SHA-256;
> die `0000…`-Platzhalter laufen in einen Hash-Mismatch **nach** dem Download,
> `artifact_install.rs:133`, nicht in einen sauberen Refuse). Zudem: `fetch_verified`
> entpackt **nicht** — das Release-Asset muss das **rohe Binary** sein, kein `.tar.gz`.

---

## 4. Architektur — vier Embed-Kanäle, zwei davon werden gecuttet

| #      | Kanal                             | Datei                  | Inhalt                                                                                            | P3                  |
|--------|-----------------------------------|------------------------|---------------------------------------------------------------------------------------------------|---------------------|
| **1**  | Render-Quelle (Bodies+Companions) | `src/skills.rs`        | 8 Skill-Bodies + ~17 Companions                                                                   | **→ Pack**          |
| **2**  | Install-Materializer              | `src/skill_install.rs` | 8 SKILL.md-Stubs + 6 ASSETS (5× `lmd-brainstorm/scripts/`, `lmd-writing-skills/render-graphs.js`) | **→ Pack**          |
| **3a** | Skill-lokale `_includes/`         | `src/fragments.rs`     | `test-first-core`, `skill-authoring-core`, `brainstorm-gate` (`content/skills/<n>/_includes/`)    | **→ Pack**          |
| **3b** | Cross-Skill-Core-Primitive        | `src/fragments.rs`     | `hard-rules`, `dispatch-contract`, `parallel-dispatch` (`content/core/`)                          | **bleibt embedded** |
| **4**  | Gloss-Tabelle                     | `src/gloss.rs`         | `gloss/directives.lmd.md`                                                                         | **bleibt embedded** |

**Kanal 3 splittet** (Kern der Revision): die skill-lokalen `_includes/` sind
**skill-scoped** (`test-first-core` = nur TDD) → sie reisen mit ihrem Skill in den Pack und
werden aus `fragments.rs` gecuttet. Die cross-skill Core-Primitive sind **allgemeine
lmd-Primitive** → sie bleiben embedded (Standalone-Render, §1).

### 4.1 Body-/Companion-/`_includes`-Resolution: drei Stufen

Ersetzt die embedded-Quelle in `render_skill`/`companion_body` **und** die
Fragment-Resolution der 3 skill-lokalen `_includes/` durch eine Kaskade:

1. **Overlay** (bestehend, gewinnt): `<jail_root>/.lean-ctx/lean-md/skills/<name>/…`
2. **Pack-Store** (NEU, Produktionspfad): der materialisierte Pack-Dir aus
   `pack_store_root()` (§4.2).
3. **Debug-Fallback** (NEU, nur Dev): `$CARGO_MANIFEST_DIR/content/skills/<name>/…`
   — gated auf `cfg(debug_assertions)` + Pfad-Existenz; im Release-Binary inert.

> **Im Plan zu verifizieren (Task 2, Anker statt Annahme):** der Relativpfad-Präfix im Pack.
> `pack create --from content/skills` sammelt Pfade relativ zum `--from`-Root
> (`context_package/skills.rs::collect_files`); ob der materialisierte Baum unter
> `skills_dir(...)` direkt `<skill>/body.lmd.md` oder `skills/<skill>/body.lmd.md` trägt,
> wird beim ersten lokalen `pack create` + `materialize_documents` **empirisch festgestellt**
> — nicht geraten.

**Registry-Umbau (Task 2):** die `SKILLS`/`COMPANIONS`-Tabellen mappen heute
Name→`&'static str`-Body; nach dem Cut mappen sie Name→**Relativpfad** (gültige Namen
bleiben erhalten, Kaskade löst Pack-relativ auf). `FragmentRegistry` (`fragments.rs`)
behält die 3 cross-skill Builtins und bekommt für die 3 skill-lokalen `_includes/`-Namen
eine Pack-Store-Stufe. **Fragment-Kaskade (separat ausformulieren, Task 2):** `resolve()` =
cross-skill-builtin → **pack-store (neu, für die 3 skill-lokalen Namen)** → **jail-Datei-Fallback
(bestehend, MUSS erhalten bleiben)**. Der Cut entfernt nur die 3 `include_str!`-Builtins der
skill-lokalen `_includes/`, **nicht** die Datei-Fallback-Stufe von `resolve()`.

### 4.2 Runtime-Pack-Pfad — **geklärt**

`pack_store_root()` liest die Env-Var **`LEAN_MD_SKILLS_DIR`**. Sie enthält den **absoluten
`skills_dir`-Pfad**, den lean-ctx beim Wiring aus dem
`{pack_dir:@dasTholo/lean-md-skills}`-Platzhalter expandiert hat (§2.3). **Kein**
Lockfile-Parsing im Binary, **keine** Ableitung des Store-Layouts — lean-md kennt nur einen
absoluten Pfad.

Fehlt die Var oder existiert der Pfad nicht: **Produktion → harter, aktionierbarer Fehler**
(`PackMissing`, „addon reinstall"), **kein** stiller Leerlauf. Im Dev-Build greift zuvor der
Debug-Fallback.

---

## 5. Vorbedingungen (extern) & Task-Zerlegung

### 5.1 Vorbedingungen — P3 startet nicht davor

Diese vier Gates liegen **außerhalb** dieses Plans. Tasks 2 und 3 (der irreversible
`include_str!`-Cut) dürfen erst laufen, wenn alle grün sind:

| #      | Gate                                                                                               | Wo                                              |
|--------|----------------------------------------------------------------------------------------------------|-------------------------------------------------|
| **V1** | Upstream-Vertrag released (dependencies-Authoring, `pack_env`, `min_lean_ctx`-Gate, Install-Order) | lean-ctx `pr-rebuild` → PR → Release            |
| **V2** | lean-md-Release `v0.2.0` mit echten SHA-256 in allen 5 `[artifacts]`-Blöcken                       | lean-md CI (`sync-manifest`)                    |
| **V3** | curated Registry-Entry adressiert (`listed`, §3)                                                   | lean-ctx `addon_registry.json` + `gen_registry` |
| **V4** | `@dasTholo/lean-md` **und** `@dasTholo/lean-md-skills` hosted publiziert                           | `addon publish --namespace dasTholo` (#726)     |

**V1 ist die harte Sperre.** Ohne sie ist `[[dependencies]]` wirkungslos und
`LEAN_MD_SKILLS_DIR` nie gesetzt — Task 2 würde ein Binary erzeugen, dessen `render --skill`
zur Laufzeit hart fehlschlägt.

### 5.2 Tasks (5, strikt sequenziell)

| #     | Task                                                             | Kern                                                                                                                                                                                                                                                                                                                                                                     | Gate / Output                                                                             |
|-------|------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------|
| **1** | **Manifest + Pack-Build**                                        | `[[dependencies]]` (`name = "@dasTholo/lean-md-skills"`, **`version_req = "^0.2"`**, `optional = false`); `[mcp.env] LEAN_MD_SKILLS_DIR = "{pack_dir:@dasTholo/lean-md-skills}"`; `min_lean_ctx` auf die V1-Release-Version; deterministischer `pack create --kind skills --from content/skills` in CI + Drift-Gate; curated-Entry-Entscheidung (§3) als Decision-Record | Pack content-addressed, #498 byte-stabil; `cargo build` grün (kein `src/`-Code berührt)   |
| **2** | **Render-Resolution-Cut** (`src/skills.rs` + `src/fragments.rs`) | 3-Stufen-Resolution in `render_skill`/`companion_body`; `pack_store_root()` = `LEAN_MD_SKILLS_DIR`; skill-lokale `_includes/` aus `fragments.rs` cutten + Pack-Store-Stufe; `include_str!`-Bodies+Companions+`_includes` raus; cross-skill Core bleibt builtin; Caller-Ripple `src/bin/lean_md.rs` + `bridges/dispatch.rs`                                               | Alle Skill-Render-Tests grün gegen die neue Quelle; `PackMissing` hart in Release         |
| **3** | **Install-Materializer-Cut** (`src/skill_install.rs`)            | `INSTALLABLE_SKILLS` + `ASSETS` auf read-from-pack umbauen (**nicht** löschen — der Bridge-Schritt schreibt weiterhin `.claude/skills/`-Stubs); visual-companion Script-Pfad-Auflösung                                                                                                                                                                                   | `brainstorm_*_materializes` / `assets_reference_closure` / `*_writes_skill_md` umgestellt |
| **4** | **Gates + Dev-Workflow**                                         | neuer Gate „Pack-Inhalt == `content/skills` on disk"; #498-Fragment-Gate bleibt für die 3 cross-skill Builtins; debug-fallback-Render verifizieren; `AGENTS.md`/`CLAUDE.md` Dev-Notiz                                                                                                                                                                                    | `cargo run -- render --skill X --phase Y` ohne Pack grün                                  |
| **5** | **End-to-End (Live-Smoke)**                                      | `addon add @dasTholo/lean-md` installiert Binary + Pack; Consent listet den Pack; Lockfile pinnt das Paar; zweiter Install offline-reproduzierbar; Bodies via Render erreichbar; Scripts materialisieren + laufen                                                                                                                                                        | DoD P3                                                                                    |

**Der alte Task 1 (Wiring-Discovery, STOP-Gate) entfällt** — durchgeführt am 2026-07-09,
Ergebnis in §2. Seine drei Unbekannten sind beantwortet: Deklarations-Key = `version_req`
(nach Upstream-PR), Pfad-Delivery = `LEAN_MD_SKILLS_DIR` via `{pack_dir:}`-Platzhalter,
Versions-Kopplung = Lockstep `0.2.0`. Sein HALT-Verdikt (`halt-dual-source`) ist in die
Vorbedingung V1 überführt: **kein Dual-Source-Umbau**, sondern warten auf den Vertrag.

**Hash vs. Version:** `content_hash`/SHA256 aktualisiert sich automatisch deterministisch bei
jeder `content/skills`-Änderung (#498). Die **SemVer-Version** ist der Dependency-Vertrag:
publizierte Versionen sind **immutable** (Lockfile pinnt `artifact_sha256`; Republish gleicher
Version mit anderen Bytes → abgelehnt). ∴ **Content-Änderung erzwingt Versions-Bump** — unter
Lockstep auch für das Binary. CI braucht einen **Drift-Gate** (Pack neu bauen, Hash asserten —
fängt „Content geändert, Bump/Publish vergessen"), analog zum `sync-manifest`-SHA-Rückfluss
beim Binary.

---

## 6. Projekt-Seeds (`.lean-ctx/lean-md/`) — separater Kanal, unverändert

`seeds.rs::materialize_contracts` (`PROJECT_SEEDS`) schreibt beim Install nach
`<project>/.lean-ctx/lean-md/` — **nicht** der Pack, ein eigener Kanal. Er bleibt in P3
**unverändert** (embedded + lokal-materialisiert). Drei Funktionsklassen, unterschiedlicher
Zwang:

| Seed                             | Klasse                                                                                          | Muss projekt-lokal?                                                      |
|----------------------------------|-------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------|
| `plan-recipes`, `plan-template`  | **Render-`@import`-Ziel** (Plan-Meta-Head `@import … /plan-recipes /`)                          | **Ja, hart** — Pack-Store-Pfad erfüllt projekt-relatives `@import` nicht |
| `dispatch-contract.ext`          | **`@dispatch`-Extension** (render-time, user-erweiterbar)                                       | **Ja**                                                                   |
| `lang/rust`, `tooling/mcp-tools` | **Prosa-Referenzdoc** (agent-read, **kein** `@include`/`@import` — grep-verifiziert: 0 Treffer) | weich (nur Datei muss lesbar sein)                                       |

Konsequenz: P3 fasst `seeds.rs` **nicht** an. Ein späteres Verschieben von `lang`/`tooling`
in den Pack-Store ist möglich (nichts `@import`t sie), aber **YAGNI** — hier out-of-scope.

---

## 7. Testing-Strategie (TDD pro Code-Task)

- **Task 2** (`skills.rs` + `fragments.rs`): Resolution-Tests — pack-store-Stufe liefert Body
  **und** die 3 skill-lokalen `_includes/`; overlay gewinnt vor pack; debug-fallback nur unter
  `cfg(debug_assertions)`; **Produktion: harter Fehler** wenn `LEAN_MD_SKILLS_DIR` fehlt oder
  der Pfad nicht existiert. Bestehende Skill-Render-Tests laufen **unverändert** (identische
  Bytes → identische Renders). Die 3 cross-skill Core-Builtins bleiben builtin-aufgelöst.
- **Task 3** (`skill_install.rs`): `brainstorm_*_materializes_scripts` /
  `assets_reference_closure` / `*_writes_skill_md` gegen den Pack-Store umgestellt.
- **Task 4**: neuer deterministischer Gate „Pack-Inhalt == `content/skills` on disk"
  (#498-Stil). Der bestehende Fragment-Consistency-Gate bleibt — jetzt für die **3
  cross-skill Builtins** (built-in == on-disk); die **3 skill-lokalen `_includes/`** fallen
  unter den neuen Pack-Drift-Gate (pack == on-disk).
- **Task 5**: `addon add @dasTholo/lean-md` installiert beide, Lockfile pinnt das Paar,
  zweiter Install offline-reproduzierbar; visual-companion-Scripts materialisieren + laufen.

Testkommando projektweit: `cargo nextest run` (nie `cargo test`).

---

## 8. Skill-Tiering (`full`/`minimal`) — Non-Goal in P3, Future

Idee: `minimal` exponiert nur die Kern-Skills (brainstorm, writing-plans, executing-plans),
`full` alle. **Rechtfertigt kein Multi-Pack**, drei Gründe (verifiziert):

1. **Kein Opt-in.** `addon add` löst **optionale** Deps nie auf und hat **keinen**
   `--with`/`--profile`/`--skills`-Schalter (`addon_cmd.rs` L367/L849, grep-bestätigt).
2. **Footprint spart ~nichts.** Ein ungenutzter, materialisierter Body kostet zur Laufzeit
   nichts (read-only Disk, gerendert nur bei `render --skill X`; alle Skills = ~236 KB).
3. **Skill-Graph splittet nicht sauber.** `subagent-driven-development` →
   `test-driven-development` + `dispatching-parallel-agents` + `parallel-dispatch`; ein
   „minimal"-Tier, in das ein Workflow hineinläuft, dangling-t.

**Der echte Hebel** ist „welche `SKILL.md`-Stubs sieht der Agent" — ein **Install-Profil über
EINEN Pack** (der Bridge-Schritt schreibt nur die Stubs des Tiers), **kein** Multi-Pack.
Multi-Pack lohnt nur bei echten Achsen (Release-Kadenz, Trust/Pricing, Fremd-Addon-Reuse).
→ Future, separat von P3 nachrüstbar.

---

## 9. Risiken & Mitigationen

| #      | Risiko                                                                                                      | Mitigation                                                                                                                                                                                                |
|--------|-------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **R1** | **V1 landet nicht** (Upstream-PR abgelehnt / verzögert)                                                     | P3 bleibt gestoppt. Fork-Release aus `dasTholo/lean-ctx` ist der Ausweichpfad; `min_lean_ctx` pinnt dann die Fork-Version. **Kein** Dual-Source-Umbau als Zwischenlösung — er kostet mehr als das Warten. |
| **R2** | Pack-Relativpfad-Präfix anders als angenommen (§4.1)                                                        | Kontained: nur die Pfad-Komposition der `pack_store_root()`-Konsumenten ändert sich. Task 2 stellt ihn empirisch fest, **bevor** Code entsteht.                                                           |
| **R3** | Distributions-Pivot: Deps nur auf hosted-ctxpkg-Pfad (§3) — curated-`addon add lean-md` zieht sie **nicht** | Bewusst: lean-md als hosted ctxpkg-Pack ausliefern; `[artifacts]`-Binary reist im Pack mit; curated-Entry auf `listed`                                                                                    |
| **R4** | Content geändert, Versions-Bump/Publish vergessen                                                           | Drift-Gate in CI (§5.2)                                                                                                                                                                                   |
| **R5** | Reverse-Cut-Verletzung                                                                                      | Unberührt — Pack ist Content, kein Engine-Symbol                                                                                                                                                          |
| **R6** | debug-fallback feuert im Release-Binary                                                                     | `cfg(debug_assertions)` + `CARGO_MANIFEST_DIR`-Existenz; Release-Binary hat keins von beidem                                                                                                              |
| **R7** | Standalone-Render (`cargo run`/Dev) ohne Pack bricht                                                        | Primitive (`core`+`gloss`) bleiben embedded → General-Render immer lauffähig; Skill-Render via debug-fallback                                                                                             |
| **R8** | Lockstep zwingt Binary-Bump für reinen Content-Fix                                                          | Bewusst akzeptiert (§2.3) — **eine** Versionsnummer statt zweier driftender.                                                                                                                              |

---

## 10. Definition of Done (P3)

- Skill-Content — Bodies + Companions (`skills.rs`), SKILL.md-Stubs + Assets
  (`skill_install.rs`) **und** die 3 skill-lokalen `_includes/` (`fragments.rs`) — ist aus
  dem Binary entfernt; Binary schrumpft messbar.
- **Ein** `kind=skills`-Pack `@dasTholo/lean-md-skills@0.2.0` wird deterministisch aus
  `content/skills` gebaut; als depth-1-Dependency (`version_req = "^0.2"`) im Manifest
  deklariert (schema v2).
- `render_skill`/`companion_body` **und** die 3 skill-lokalen `_includes/` lösen über die
  3-Stufen-Kaskade auf; `pack_store_root()` liest `LEAN_MD_SKILLS_DIR`; Produktion
  hart-fehlerhaft bei fehlendem Pack; alle Skill-Render-Tests grün.
- Cross-Skill-Core-Primitive (`hard-rules`, `dispatch-contract`, `parallel-dispatch`) +
  `gloss/directives` bleiben embedded; #498-Fragment-Consistency-Gate (built-in == on-disk)
  bleibt grün.
- `seeds.rs`/`PROJECT_SEEDS` unverändert (§6).
- CI: Pack-Build + Drift-Gate; Lockstep-Bump-Regel dokumentiert.
- Dev-Workflow: `cargo run -- render --skill X --phase Y` ohne Pack grün (debug-fallback).
- **DoD-live:** `addon add @dasTholo/lean-md` installiert Binary + Pack, Consent listet den
  Pack, Lockfile pinnt das Paar, zweiter Install offline-reproduzierbar; Skill-Bodies via
  Render erreichbar; visual-companion-Scripts materialisieren + laufen.
- **Nicht enthalten:** Skill-Tiering (§8), P4 (Publisher-Identität/Signing), Verschieben von
  `lang`/`tooling` in den Pack, der Registry-Entry-`artifacts`-Fix (separater Block, §3).
  **P2 (hosted publish) ist nicht mehr „nicht enthalten", sondern Vorbedingung V4** (§5.1).
