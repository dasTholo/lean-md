# Design-Spec: P3 — `kind=skills`-Pack (Full-Cut)

> Erstellt: 2026-07-08 · Revidiert: 2026-07-08 (Brainstorm: Wiring gegen lean-ctx-Source verifiziert)
> Branch: `feat-lmd-v2` · Roadmap-Phase: **P3** (Referenzfall #727)
> Vorgänger: P0 + P1 im Code fertig (5-leg `[artifacts]`-Matrix, `9856641..689f74c`).
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

## 2. Wiring gegen lean-ctx-Source verifiziert (#727)

Der frühere „Consumption ungetestet"-Vorbehalt (alte R2) ist **präzisiert**: der
depth-1-Dependency-Mechanismus ist in der lean-ctx-Quelle (`origin/main`, lokal
`/home/tholo/Scripts/lean-ctx`) **implementiert und gelesen**:

| Fakt | Fundstelle (lean-ctx) | Aussage |
|---|---|---|
| **`PackageKind::Skills`** | `rust/src/core/context_package/manifest.rs:15` | Enum `Context`(default)·`Skills`·`Addon`·`Grammar` (GH #724); `Skills` erfordert schema v2 |
| **`dependencies`-Feld** | `manifest.rs:60` | `pub dependencies: Vec<PackageDependency>` — mehrere, je `optional` |
| **Depth-1-Resolver** | `context_package/deps.rs:1` | *„Depth-1 dependency resolution at install time (#727, Phase 3) … On `pack install` / `addon add`, the direct dependencies … resolve"* |
| **Dep-Regeln** | `deps.rs:56` | Dependency muss **`@ns/name`**-scoped sein; SemVer-Range; keine Self-Dep; höchste non-yanked Version |
| **Skills-Materialisierung** | `cli/pack_cmd.rs:1176` `apply_or_report` | `kind=skills` → **read-only + SHA-256-verifiziert** nach `skills::skills_dir(registry_root, name, version)`; „Files load from disk, not sessions" |
| **Lockfile / offline** | `deps.rs` `already_satisfied` | Lockfile pinnt die Version; zweiter Install offline-reproduzierbar |

**Verbleibender Vorbehalt:** Gelesen wurde `origin/main` — das ist **vor** dem gepinnten
`min_lean_ctx = "3.9.2"` (der vendored Contract `docs/CONTRACT.md` ist noch v1, ohne
`kind`/`dependencies`). „Im Source implementiert" ≠ „läuft in 3.9.2". **Welche
Runtime-Version die `addon add`-Dep-Resolution tatsächlich ausführt**, ist die verbleibende
Task-1-Prüfung (§5, R2).

---

## 3. Distributions-Modell — **hosted ctxpkg-Pack ist Pflicht**

Kritischer Wiring-Befund (`cli/addon_cmd.rs` `cmd_add`, L279–381): Die Dep-Resolution feuert
**nur**, wenn ein Package-Manifest `pm` vorliegt — und `pack_manifest = Some(pm)` entsteht
**ausschließlich** im hosted-`@ns/name`-Pfad (`fetch_addon_pack`). Die anderen zwei
Auflösungspfade setzen es **nicht**:

| `addon add <target>` | Pfad | `pack_manifest` | Deps aufgelöst? |
|---|---|---|---|
| `./lean-ctx-addon.toml` | local | `None` | **nein** |
| `lean-md` (bundled/curated registry) | curated | `None` | **nein** |
| `@dasTholo/lean-md` (hosted ctxpkg) | hosted pack | **`Some(pm)`** | **ja, depth-1** |

∴ **Damit `addon add` den Skills-Pack automatisch mitzieht, MUSS lean-md als hosted
ctxpkg-`kind=addon`-Pack ausgeliefert werden** (`addon publish --namespace dasTholo` →
Nutzer: `addon add @dasTholo/lean-md`). Das ist ein **Distributions-Pivot**: weg von
`cargo install`/reinem curated-`[artifacts]`, hin zu „GitHub-Release + `addon add`".

**Das GH-Binary bleibt der Auslieferungsweg** — das `lean-ctx-addon.toml` mit dem
5-leg-`[artifacts]`-Block (P0/P1) wird verbatim in den `kind=addon`-Pack gewrappt. Ablauf
bei `addon add @dasTholo/lean-md`:

1. Binary aus **GitHub-Release** (`[artifacts]`) → managed bin dir, sha256-gepinnt.
2. `resolve_dependencies(pm)` → höchstes non-yanked SemVer-Match von `@dasTholo/lean-md-skills`
   gegen den ctxpkg-Index.
3. Consent-Surface listet den Pack (`+ @dasTholo/lean-md-skills@1.x`).
4. Nach dem Wiring: `install_declared_dependencies` materialisiert den Pack read-only +
   SHA-verifiziert nach `skills_dir(registry_root, name, version)`.
5. Lockfile pinnt Binary **und** Pack — zweiter Install offline-reproduzierbar.

**Curated-Registry-Eintrag nach dem Full-Cut (explizite Entscheidung, Task 2):** Ein via
curated `addon add lean-md` (bundled `addon_registry.json`) installiertes Binary hat nach dem
Cut **keinen** Skill-Content und zieht den Pack **nicht** (Tabelle oben) → jedes
`render --skill` wäre hart-fehlerhaft. Task 2 MUSS daher den bundled/curated-Eintrag
adressieren — **eine** der Optionen wählen und begründen: (a) aus dem curated Registry
**entfernen** (nur noch hosted `@dasTholo/lean-md`), (b) auf **listed** zurückstufen
(Homepage-Pointer, kein one-click), (c) skill-los stehenlassen (nur General-Render). Default-
Empfehlung: **(b) listed**, bis der hosted Pack live ist.

---

## 4. Architektur — vier Embed-Kanäle, zwei davon werden gecuttet

| # | Kanal | Datei | Inhalt | P3 |
|---|---|---|---|---|
| **1** | Render-Quelle (Bodies+Companions) | `src/skills.rs` | 8 Skill-Bodies + ~17 Companions | **→ Pack** |
| **2** | Install-Materializer | `src/skill_install.rs` | 8 SKILL.md-Stubs + 6 ASSETS (5× `lmd-brainstorm/scripts/`, `lmd-writing-skills/render-graphs.js`) | **→ Pack** |
| **3a** | Skill-lokale `_includes/` | `src/fragments.rs` | `test-first-core`, `skill-authoring-core`, `brainstorm-gate` (`content/skills/<n>/_includes/`) | **→ Pack** |
| **3b** | Cross-Skill-Core-Primitive | `src/fragments.rs` | `hard-rules`, `dispatch-contract`, `parallel-dispatch` (`content/core/`) | **bleibt embedded** |
| **4** | Gloss-Tabelle | `src/gloss.rs` | `gloss/directives.lmd.md` | **bleibt embedded** |

**Kanal 3 splittet** (Kern der Revision): die skill-lokalen `_includes/` sind
**skill-scoped** (`test-first-core` = nur TDD) → sie reisen mit ihrem Skill in den Pack und
werden aus `fragments.rs` gecuttet. Die cross-skill Core-Primitive sind **allgemeine
lmd-Primitive** → sie bleiben embedded (Standalone-Render, §1).

### 4.1 Body-/Companion-/`_includes`-Resolution: drei Stufen

Ersetzt die embedded-Quelle in `render_skill`/`companion_body` **und** die
Fragment-Resolution der 3 skill-lokalen `_includes/` durch eine Kaskade:

1. **Overlay** (bestehend, gewinnt): `<jail_root>/.lean-ctx/lean-md/skills/<name>/…`
2. **Pack-Store** (NEU, Produktionspfad): der materialisierte Pack-Dir
   (`skills_dir(registry_root, "lean-md-skills", <version>)/skills/<name>/…`)
3. **Debug-Fallback** (NEU, nur Dev): `$CARGO_MANIFEST_DIR/content/skills/<name>/…`
   — gated auf `cfg(debug_assertions)` + Pfad-Existenz; im Release-Binary inert.

**Registry-Umbau (Task 3):** die `SKILLS`/`COMPANIONS`-Tabellen mappen heute
Name→`&'static str`-Body; nach dem Cut mappen sie Name→**Relativpfad** (gültige Namen
bleiben erhalten, Kaskade löst Pack-relativ auf). `FragmentRegistry` (`fragments.rs`)
behält die 3 cross-skill Builtins und bekommt für die 3 skill-lokalen `_includes/`-Namen
eine Pack-Store-Stufe. **Fragment-Kaskade (separat ausformulieren, Task 3):** `resolve()` =
cross-skill-builtin → **pack-store (neu, für die 3 skill-lokalen Namen)** → **jail-Datei-Fallback
(bestehend, MUSS erhalten bleiben)**. Der Cut entfernt nur die 3 `include_str!`-Builtins der
skill-lokalen `_includes/`, **nicht** die Datei-Fallback-Stufe von `resolve()`.

### 4.2 Runtime-Pack-Pfad — Arbeitshypothese (Task-1-gated)

Der Pack materialisiert deterministisch nach `skills_dir(registry_root, name, version)`
(§2). Die starke Hypothese: **lean-ctx reicht `lean-md` diesen absoluten Pfad beim Spawn per
Env-Var** (via `[mcp.env]` / capabilities-`env`-Allowlist), analog zum
Binary-`[artifacts]`-Wiring; **kein** eigenes Lockfile-Parsing im Binary. Fehlt die Var /
existiert der Pfad nicht: **Produktion → harter, aktionierbarer Fehler** („addon reinstall"),
**kein** stiller Leerlauf. Wird die Hypothese von Task 1 widerlegt, ändert sich nur die eine
`pack_store_path()`-Funktion; die 3-Stufen-Struktur bleibt.

---

## 5. Task-Zerlegung (6 Tasks, strikt sequenziell — Task 1 gated alles)

| # | Task | Kern | Gate / Output |
|---|---|---|---|
| **1** | **Wiring-Discovery** (kein Code, **STOP-Gate**) | verbleibende Unbekannte empirisch klären (§2/§3 sind teils schon verifiziert); Pack bauen → publish-Probe → `addon add @ns/lean-md` mit Dependency → *was landet wohin* | Wiring-Contract-Notiz (`ctx_knowledge`) — die 3 offenen Punkte (unten) geklärt. **HALT-Bedingung:** existiert in **keiner** aktuell ausgelieferten lean-ctx-Version ein Path-Delivery-Mechanismus für Unknown #2 (wie `skills_dir` das gespawnte Binary erreicht), wird der **irreversible Full-Cut NICHT ausgeführt** — Tasks 3/4 pausieren; Fallback: Pack bauen (Task 2) + `include_str!` vorerst behalten (Dual-Source), bis ein Mechanismus shippt. |
| **2** | **Manifest + Pack-Build** | `dependencies`-Deklaration `@dasTholo/lean-md → @dasTholo/lean-md-skills: ^1.0` (schema v2, gemäß Task-1-Syntax); deterministischer `pack create --kind skills --from content/skills` in CI/Release | Pack content-addressed, #498 byte-stabil; CI-Form gemäß Task-1-Versions-Befund |
| **3** | **Render-Resolution-Cut** (`src/skills.rs` + `src/fragments.rs`) | 3-Stufen-Resolution in `render_skill`/`companion_body`; skill-lokale `_includes/` aus `fragments.rs` cutten + Pack-Store-Stufe; `include_str!`-Skill-Bodies+Companions+`_includes` raus; cross-skill Core bleibt builtin | Alle Skill-Render-Tests grün gegen neue Quelle |
| **4** | **Install-Materializer-Cut** (`src/skill_install.rs`) | je nach Task-1-Befund: **löschen** *oder* **umbauen** auf read-from-pack; visual-companion Script-Pfad-Auflösung | `brainstorm_*_materializes/reference_closure` + `*_writes_skill_md`-Tests umgestellt |
| **5** | **Gates + Dev-Workflow** | neuer Gate „Pack-Inhalt == `content/skills` on disk"; #498-Fragment-Gate bleibt für die 3 cross-skill Builtins; debug-fallback-Render verifizieren; `AGENTS.md`/`CLAUDE.md` Dev-Notiz | `cargo run -- render` ohne Pack grün |
| **6** | **End-to-End** | live `addon add @dasTholo/lean-md` installiert Binary + Pack, Lockfile pinnt das Paar, zweiter Install offline-reproduzierbar; Bodies via Render erreichbar; Scripts materialisieren + laufen | DoD P3 (ggf. deferred, s. R2) |

### Task-1-Unbekannte (die 3 verbliebenen Discovery-Outputs)

1. **Dependency-Deklarations-Syntax**: Trägt das `lean-ctx-addon.toml` einen
   `[dependencies]`-Block, den `addon publish` ins Package-Manifest hebt — oder braucht es
   einen von-Hand-gebauten Pack? (Mechanismus steht; nur die **Deklarations-Oberfläche** offen.)
2. **Runtime-Pack-Pfad**: Wie erreicht der `skills_dir`-Pfad das gespawnte `lean-md`?
   (Env-Var-Hypothese §4.2 bestätigen/widerlegen.)
3. **Versions-Kopplung**: **Independent-SemVer** (eigene Pack-Linie, Content-Fix ohne
   Binary-Release — der #727-Nutzen) **vs. Lockstep** (Pack-Version == Addon-Version).
   Explizites Task-1-Output; Task-2-CI-Form verzweigt darauf.

**Hash vs. Version:** `content_hash`/SHA256 aktualisiert sich automatisch deterministisch bei
jeder `content/skills`-Änderung (#498). Die **SemVer-Version** ist der Dependency-Vertrag:
publizierte Versionen sind **immutable** (Republish gleicher Version, andere Bytes →
abgelehnt). ∴ **Content-Änderung erzwingt Versions-Bump.** CI braucht einen **Drift-Gate**
(Pack neu bauen, Hash asserten — fängt „Content geändert, Bump/Publish vergessen"), analog
zum `sync-manifest`-SHA-Rückfluss beim Binary.

---

## 6. Projekt-Seeds (`.lean-ctx/lean-md/`) — separater Kanal, unverändert

`seeds.rs::materialize_contracts` (`PROJECT_SEEDS`) schreibt beim Install nach
`<project>/.lean-ctx/lean-md/` — **nicht** der Pack, ein eigener Kanal. Er bleibt in P3
**unverändert** (embedded + lokal-materialisiert). Drei Funktionsklassen, unterschiedlicher
Zwang:

| Seed | Klasse | Muss projekt-lokal? |
|---|---|---|
| `plan-recipes`, `plan-template` | **Render-`@import`-Ziel** (Plan-Meta-Head `@import … /plan-recipes /`) | **Ja, hart** — Pack-Store-Pfad erfüllt projekt-relatives `@import` nicht |
| `dispatch-contract.ext` | **`@dispatch`-Extension** (render-time, user-erweiterbar) | **Ja** |
| `lang/rust`, `tooling/mcp-tools` | **Prosa-Referenzdoc** (agent-read, **kein** `@include`/`@import` — grep-verifiziert: 0 Treffer) | weich (nur Datei muss lesbar sein) |

Konsequenz: P3 fasst `seeds.rs` **nicht** an. Ein späteres Verschieben von `lang`/`tooling`
in den Pack-Store ist möglich (nichts `@import`t sie), aber **YAGNI** — hier out-of-scope.

---

## 7. Testing-Strategie (TDD pro Code-Task)

- **Task 3** (`skills.rs` + `fragments.rs`): Resolution-Tests — pack-store-Stufe liefert Body
  **und** die 3 skill-lokalen `_includes/`; overlay gewinnt vor pack; debug-fallback nur unter
  `cfg(debug_assertions)`; **Produktion: harter Fehler** wenn Pack fehlt. Bestehende
  Skill-Render-Tests laufen **unverändert** (identische Bytes → identische Renders). Die 3
  cross-skill Core-Builtins bleiben builtin-aufgelöst (kein Pack nötig).
- **Task 4** (`skill_install.rs`): `brainstorm_*_materializes_scripts` /
  `assets_reference_closure` / `*_writes_skill_md` auf den Task-1-Zweig umgestellt.
- **Task 5**: neuer deterministischer Gate „Pack-Inhalt == `content/skills` on disk"
  (#498-Stil). Der bestehende Fragment-Consistency-Gate bleibt — jetzt für die **3
  cross-skill Builtins** (built-in == on-disk); die **3 skill-lokalen `_includes/`** fallen
  unter den neuen Pack-Drift-Gate (pack == on-disk).
- **Task 6**: `addon add @dasTholo/lean-md` installiert beide, Lockfile pinnt das Paar,
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

| # | Risiko | Mitigation |
|---|---|---|
| **R1** | Env-Var-Hypothese (§4.2) falsch, **aber irgendein** Path-Delivery-Mechanismus existiert | Kontained: nur `pack_store_path()` ändert sich, 3-Stufen-Struktur bleibt. (Existiert **gar kein** Mechanismus → nicht R1, sondern die Task-1-HALT-Bedingung greift, s. §5.) |
| **R2** | Mechanismus in `origin/main` verifiziert, aber `min_lean_ctx=3.9.2` / vendored Contract v1 → **welche Runtime `addon add`-Deps auflöst, ungetestet** | **Task-1-Must-Verify.** Falls Consumption erst in späterer lean-ctx-Version → Tasks 1–5 (Cut + Pack-Build) landen trotzdem, **Task 6 Live-Smoke deferred** (dokumentierter Dangling-Window, P0/P1-konform) |
| **R3** | Distributions-Pivot: Deps nur auf hosted-ctxpkg-Pfad (§3) — curated-`addon add lean-md` zieht sie **nicht** | Bewusst: lean-md als hosted ctxpkg-Pack ausliefern; `[artifacts]`-Binary reist im Pack mit |
| **R4** | Content geändert, Versions-Bump/Publish vergessen | Drift-Gate in CI (§5) |
| **R5** | Reverse-Cut-Verletzung | Unberührt — Pack ist Content, kein Engine-Symbol |
| **R6** | debug-fallback feuert im Release-Binary | `cfg(debug_assertions)` + `CARGO_MANIFEST_DIR`-Existenz; Release-Binary hat keins von beidem |
| **R7** | Standalone-Render (`cargo run`/Dev) ohne Pack bricht | Primitive (`core`+`gloss`) bleiben embedded → General-Render immer lauffähig; Skill-Render via debug-fallback |

---

## 10. Definition of Done (P3)

- Skill-Content — Bodies + Companions (`skills.rs`), SKILL.md-Stubs + Assets
  (`skill_install.rs`) **und** die 3 skill-lokalen `_includes/` (`fragments.rs`) — ist aus
  dem Binary entfernt; Binary schrumpft messbar.
- **Ein** `kind=skills`-Pack `@dasTholo/lean-md-skills` wird deterministisch aus
  `content/skills` gebaut; als depth-1-Dependency im Manifest deklariert (schema v2).
- `render_skill`/`companion_body` **und** die 3 skill-lokalen `_includes/` lösen über die
  3-Stufen-Kaskade auf; Produktion hart-fehlerhaft bei fehlendem Pack; alle
  Skill-Render-Tests grün.
- Cross-Skill-Core-Primitive (`hard-rules`, `dispatch-contract`, `parallel-dispatch`) +
  `gloss/directives` bleiben embedded; #498-Fragment-Consistency-Gate (built-in == on-disk)
  bleibt grün.
- `seeds.rs`/`PROJECT_SEEDS` unverändert (§6).
- CI: Pack-Build + Drift-Gate; Versions-Kopplung gemäß Task-1-Befund.
- Dev-Workflow: `cargo run -- render --skill X --phase Y` ohne Pack grün (debug-fallback).
- **DoD-live (ggf. deferred, R2):** `addon add @dasTholo/lean-md` installiert Binary + Pack,
  Lockfile pinnt das Paar, zweiter Install offline-reproduzierbar; Skill-Bodies via Render
  erreichbar; visual-companion-Scripts materialisieren + laufen.
- **Nicht enthalten:** Skill-Tiering (§8), P2 (Self-Service-Publish ctxpkg.com als Produkt),
  P4 (Publisher-Identität/Signing), Verschieben von `lang`/`tooling` in den Pack.
