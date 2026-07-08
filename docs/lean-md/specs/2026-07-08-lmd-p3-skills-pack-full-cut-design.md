# Design-Spec: P3 — `kind=skills`-Pack (Full-Cut)

> Erstellt: 2026-07-08 · Branch: `feat-lmd-v2` · Roadmap-Phase: **P3** (Referenzfall #727)
> Vorgänger: P0 + P1 im Code fertig (5-leg `[artifacts]`-Matrix, `9856641..689f74c`).
> Terminal-Ziel dieser Spec: `lmd-writing-plans` → Implementierungsplan.

---

## 1. Ziel & Kernentscheidung

Die lmd-Skills verlassen das Binary. Heute sind Skill-Bodies, Companions, SKILL.md-Stubs
und Browser-Companion-Scripts über `include_str!` ins `lean-md`-Binary eingebacken (#727
nennt genau das als Auslager-Kandidaten). Diese Spec lagert sie in einen signierten,
versionierten **`kind=skills`-Pack** `@dasTholo/lean-md-skills` aus, deklariert als
**depth-1-Dependency** des Addons `@dasTholo/lean-md`. Resultat: Binary schrumpft;
Skill-Updates ohne Binary-Release (sofern die Versions-Kopplung es zulässt — siehe §5/Task 1).

**Gewählte Strategie: Full-Cut** (nicht Dual-Source). Der `include_str!`-Payload wird
**entfernt**, der Pack ist die **einzige** Body-/Asset-Quelle in Produktion. Das ist der
echte #727-Referenzfall (Binary = Infra, Pack = Content), mit höherem Risiko und mehr
beweglichen Teilen als ein additiver Fallback — bewusst so entschieden.

**Reverse-Cut bleibt intakt:** Der Pack trägt nur Content (`content/skills`). Kein
Engine-Symbol (`rushdown`/`evalexpr`) wandert; lean-ctx bekommt keine Render-Dependency.

---

## 2. Architektur — drei Embed-Kanäle, zwei davon werden gecuttet

Heute fließt `content/skills/` über **drei** unabhängige `include_str!`-Kanäle ins Binary:

| Kanal | Datei | Inhalt | Zweck | Full-Cut? |
|---|---|---|---|---|
| **1. Render-Quelle** | `src/skills.rs` | 8 Skill-Bodies + ~17 Companions | Render-Zeit (`render_skill`, `companion_body`) | **→ Pack** |
| **2. Install-Materializer** | `src/skill_install.rs` | 8 SKILL.md-Stubs + 6 `ASSETS` (`scripts/server.cjs`, `helper.js`, `frame-template.html`, `start-server.sh`, `stop-server.sh`, `render-graphs.js`) | schreibt sie beim Install nach `.claude/skills/<name>/…` | **→ Pack** |
| **3. Core-Fragmente** | `src/fragments.rs` | 3 `_includes/`-Seeds (`test-first-core`, `skill-authoring-core`, `brainstorm-gate`) + `hard-rules`, `dispatch-contract`, `parallel-dispatch` | Skill-Bodies `@include`en sie zur Render-Zeit | **bleibt embedded (Infra)** |

**Warum Kanal 3 embedded bleibt:** Ein pack-gelieferter Body macht `@include hard-rules`
etc.; bleiben die Fragmente im Binary, rendert der Body selbst-konsistent, ohne dass der
Pack die Fragmente selbst tragen muss. Der `#498`-Fragment-Consistency-Gate (built-in ==
on-disk-Seed) **bleibt unverändert grün**. Die `_includes/`-Dateien liegen zwar physisch
unter `content/skills/` und reisen dadurch im Pack redundant mit — harmlos, da die
Resolution die embedded-Builtins zuerst nimmt.

**Konsequenz Kanal 2:** Der `kind=skills`-Pack **ist** der Skill-Installer. Ob
`skill_install.rs` deshalb **gelöscht** (lean-ctx' Pack-Install materialisiert SKILL.md +
Scripts) oder **umgebaut** (read-from-pack) wird, entscheidet Task 1 empirisch.

### 2.1 Body-Resolution: drei Stufen (neu in `render_skill`)

Ersetzt die heutige embedded-`skill_body()`-Quelle durch eine Kaskade:

1. **Overlay** (bestehend, gewinnt): `<jail_root>/.lean-ctx/lean-md/skills/<name>/body.lmd.md`
2. **Pack-Store** (NEU, Produktionspfad): `<data_dir>/packages/skills/lean-md-skills/<version>/skills/<name>/body.lmd.md`
3. **Debug-Fallback** (NEU, nur Dev): `$CARGO_MANIFEST_DIR/content/skills/<name>/body.lmd.md`
   — gated auf `cfg(debug_assertions)` + Pfad-Existenz; im Release-Binary inert (kein
   `content/` daneben).

`companion_body` bekommt dieselbe Kaskade.

### 2.2 Runtime-Pack-Pfad — Arbeitshypothese (Task-1-gated)

Analog zum Binary-`[artifacts]`-Wiring (das Gateway rewritet die command auf den
**absoluten managed path**) ist die starke Hypothese: **lean-ctx löst die Pack-Version
über depth-1-Resolution/Lockfile auf und reicht `lean-md` den absoluten Pack-Store-Pfad
per Env-Var** (via `[mcp.env]` / capabilities-`env`-Allowlist) beim Spawn. `lean-md` liest
dann `$PACK_DIR/skills/<name>/…` — **kein** eigenes Lockfile-Parsing, **keine**
Versions-Logik im Binary. Fehlt die Var / existiert der Pfad nicht:
**Produktion → harter, aktionierbarer Fehler** („addon reinstall"), **kein** stiller
Leerlauf. Wird die Hypothese von Task 1 widerlegt, ändert sich nur die eine
`pack_store_path()`-Funktion; die 3-Stufen-Struktur bleibt.

---

## 3. Task-Zerlegung (6 Tasks, strikt sequenziell — Task 1 gated alles)

| # | Task | Kern | Gate / Output |
|---|---|---|---|
| **1** | **Wiring-Discovery** (kein Code) | lean-ctx `docs/guides/addons.md` §skills-pack + #727/PR743 lesen; empirisch proben: Pack bauen → `addon add` mit Dependency → beobachten *was wohin* landet | Schriftliche Wiring-Contract-Notiz (`ctx_knowledge`) — die 4 Unbekannten geklärt (s. §5) |
| **2** | **Manifest + Pack-Build** | `lean-ctx-addon.toml` → schema v2, `kind`, depth-1-Dependency `@dasTholo/lean-md-skills: ^1.0`; `min_lean_ctx` auf #727-Version; deterministischer `pack create --kind skills --from content/skills` in CI/Release-Workflow | Pack content-addressed, #498 byte-stabil; CI-Form gemäß Task-1-Versions-Befund |
| **3** | **Render-Resolution-Cut** (`src/skills.rs`) | 3-Stufen-Resolution in `render_skill`/`companion_body`: overlay → **pack-store (neu)** → debug-fallback; `include_str!`-Bodies + Companions raus | Alle Skill-Render-Tests grün gegen neue Quelle |
| **4** | **Install-Materializer-Cut** (`src/skill_install.rs`) | je nach Task-1-Befund: **löschen** *oder* **umbauen** auf read-from-pack; visual-companion Script-Pfad-Auflösung (relative `scripts/…` → resolvierter Pack-/Install-Dir) | `brainstorm_*_materializes/reference_closure` + `*_writes_skill_md`-Tests umgestellt |
| **5** | **Gates + Dev-Workflow** | neuer Gate „Pack-Inhalt == `content/skills` on disk"; #498-Fragment-Gate bleibt; debug-fallback-Render verifizieren; `AGENTS.md`/`CLAUDE.md` Dev-Notiz aktualisieren | `cargo run -- render` ohne Pack grün |
| **6** | **End-to-End** | live `addon add lean-md` installiert **beide**, Lockfile pinnt das Paar, zweiter Install offline-reproduzierbar; Bodies via Render erreichbar; Scripts materialisieren + laufen | DoD P3 (ggf. deferred, s. R2) |

---

## 4. Testing-Strategie (TDD pro Code-Task)

- **Task 3** (`skills.rs`): Resolution-Tests — pack-store-Stufe liefert Body; overlay
  gewinnt vor pack; debug-fallback nur unter `cfg(debug_assertions)`; **Produktion:
  harter Fehler** wenn Pack fehlt (nicht still leer). Die bestehenden Skill-Render-Tests
  laufen **unverändert** gegen die neue Quelle (identische Bytes → identische Renders).
- **Task 4** (`skill_install.rs`): `brainstorm_*_materializes_scripts` /
  `assets_reference_closure` / `*_writes_skill_md` werden auf den Task-1-Zweig umgestellt
  (gelöscht → durch Pack-Install-Assertion ersetzt, *oder* read-from-pack).
- **Task 5**: neuer deterministischer Gate „Pack-Inhalt == `content/skills` on disk"
  (#498-Stil, wie der bestehende Fragment-Consistency-Gate — der **bleibt**);
  debug-fallback-Render `cargo run -- render` ohne Pack grün.
- **Task 6**: `addon add lean-md` installiert beide, Lockfile pinnt das Paar, zweiter
  Install offline-reproduzierbar; visual-companion-Scripts materialisieren + laufen.

Testkommando projektweit: `cargo nextest run` (nie `cargo test`).

---

## 5. Task-1-Unbekannte (die 4 Discovery-Outputs)

1. **Runtime-Pack-Pfad**: Wie erreicht der resolvierte Pack-Pfad das gespawnte
   `lean-md`-Binary? (Env-Var-Hypothese, §2.2 — bestätigen/widerlegen.)
2. **Dependency-Deklaration**: exakte TOML-Syntax für `@dasTholo/lean-md → @dasTholo/lean-md-skills: ^1.0`
   im Manifest (schema v2, `kind`).
3. **Install-Materialisierung**: Was materialisiert lean-ctx' `kind=skills`-Install
   konkret, und wohin? → entscheidet Löschen vs. Umbau von `skill_install.rs` (Task 4).
4. **Versions-Kopplung**: Wie handhabt #727 Versionen/Lockfile/Immutability empirisch?
   → **Independent-SemVer** (eigene Pack-Linie, Content-Fix ohne Binary-Release — der
   #727-Nutzen) **vs. Lockstep** (Pack-Version == Addon-Version). Entscheidung ist ein
   explizites Task-1-Output; Task-2-CI-Form verzweigt darauf.

**Hash vs. Version (gilt unabhängig vom A/B-Befund):** Der `content_hash`/SHA256 des Packs
aktualisiert sich **automatisch** deterministisch bei jeder `content/skills`-Änderung
(#498). Die **SemVer-Version** hingegen ist der Dependency-/Lockfile-Vertrag: publizierte
Versionen sind **immutable** (Republish gleicher Version mit anderen Bytes → abgelehnt),
und Consumer lösen über die Version auf. ∴ **Content-Änderung erzwingt Versions-Bump.**
CI braucht daher einen **Drift-Gate** (Pack neu bauen, Hash asserten — fängt „Content
geändert, Bump/Publish vergessen"), analog zum `sync-manifest`-SHA-Rückfluss beim Binary.

---

## 6. Risiken & Mitigationen

| # | Risiko | Mitigation |
|---|---|---|
| **R1** | Env-Var-Hypothese (§2.2) falsch | Kontained: nur `pack_store_path()` ändert sich, 3-Stufen-Struktur bleibt |
| **R2** | Lokales 3.9.2 kann Pack **erzeugen** (Probe grün: 42 Files, 236 KB, ed25519, schema v2), aber **Consumption** (`addon add`-Dependency-Resolution) ist **ungetestet** | **Task-1-Must-Verify.** Falls Consumption erst in späterer lean-ctx-Version → Tasks 1–5 (Cut + Pack-Build) landen trotzdem, **Task 6 Live-Smoke wird deferred** (dokumentierter Dangling-Window, P0/P1-Muster-konform) |
| **R3** | Content geändert, Versions-Bump/Publish vergessen | Drift-Gate in CI (§5) |
| **R4** | Reverse-Cut-Verletzung | Unberührt — Pack ist Content, kein Engine-Symbol |
| **R5** | debug-fallback feuert im Release-Binary | `cfg(debug_assertions)` + `CARGO_MANIFEST_DIR`-Existenz; Release-Binary hat keins von beidem |

---

## 7. Definition of Done (P3)

- `include_str!`-Skill-Bodies + Companions (`skills.rs`) **und** SKILL.md-Stubs + Assets
  (`skill_install.rs`) sind aus dem Binary entfernt; Binary schrumpft messbar.
- `kind=skills`-Pack `@dasTholo/lean-md-skills` wird deterministisch aus `content/skills`
  gebaut; als depth-1-Dependency im Manifest deklariert (schema v2).
- `render_skill`/`companion_body` lösen Bodies über die 3-Stufen-Kaskade auf; Produktion
  hart-fehlerhaft bei fehlendem Pack; alle Skill-Render-Tests grün.
- Core-Fragmente (`fragments.rs`) bleiben embedded; #498-Fragment-Consistency-Gate grün.
- CI: Pack-Build + Drift-Gate; Versions-Kopplung gemäß Task-1-Befund.
- Dev-Workflow: `cargo run -- render --skill X --phase Y` ohne Pack grün (debug-fallback).
- **DoD-live (ggf. deferred, R2):** `addon add lean-md` installiert beide, Lockfile pinnt
  das Paar, zweiter Install offline-reproduzierbar; Skill-Bodies via Render erreichbar;
  visual-companion-Scripts materialisieren + laufen.
- **Nicht enthalten:** P2 (Self-Service-Publish ctxpkg.com), P4 (Publisher-Identität/Signing).
