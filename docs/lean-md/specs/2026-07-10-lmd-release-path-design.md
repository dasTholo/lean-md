# Design-Spec: lean-md Release-Pfad — lean-ctx lokal, Delegation raus

> Erstellt: 2026-07-10 · Branch: `feat-lmd-v2` (HEAD `3df0758`)
> Vorgänger-Specs: `2026-07-08-lmd-p3-skills-pack-full-cut-design.md` (P3-Design),
> `lean-md-next-session.prompt.md` (P3-Handoff)
>
> **Schwester-Dokumente — diese Spec dupliziert sie nicht, sie ordnet sie ein:**
> - `paket-c-next-session.md` (Repo-Root) — die ausführbare Anleitung für §5.1
>   (`skill-fix`-Merge + Doku-Rückdrehung). **Autorität für Paket C.**
> - `/home/tholo/Scripts/lean-ctx/docs/specs/2026-07-10-pr721-pr727-split-design.md` —
>   die lean-ctx-Seite (Paket A = #727-Folge-Fixes, Paket B = PR #721).
>   **Autorität für alles im lean-ctx-Repo.**
>
> Der frühere Entwurf `lean-ctx/pr721-next-session.prompt.md` ist durch die Split-Design-Spec
> **überholt** — er behauptet u. a. fälschlich, ein listed-Entry dürfe keinen `mcp`-Block haben.
>
> **Anlass:** P3 ist code-complete, aber die Vorbedingungen V1–V4 des P3-Designs sind gegen
> einen Zustand formuliert, der so nicht mehr gilt. Diese Spec untersucht den tatsächlichen
> lean-ctx-Fortschritt und leitet daraus die verbleibenden Schritte für lean-md ab.
>
> **Grundannahme (neu):** lean-ctx-Änderungen liegen lokal vor (`pr-rebuild`), die Binary ist
> baubar. Das entkoppelt einen Teil der Verifikation vom Upstream-Release.

---

## 1. Ausgangslage

`feat-lmd-v2` trägt den vollzogenen P3-Cut (`ba90947..3df0758`): Skill-Content ist aus dem
Binary entfernt, `content/skills/` (42 Dateien) löst über die 3-Stufen-Kaskade auf
(Overlay → `$LEAN_MD_SKILLS_DIR` → Debug-Fallback). 561 Tests grün, clippy clean,
Whole-Branch-Review abgenommen. Offen ist allein **Task 7** des P3-Plans: der End-to-End-
Live-Smoke.

Parallel existiert `skill-fix` (`6657cba`, eigener Worktree), der den SDD-Hardening-Plan
vollständig abgearbeitet hat und mit `feat-lmd-v2` konfliktfrei mergebar ist (`git merge-tree`).

---

## 2. lean-ctx-Fortschritt — gegen die Quelle verifiziert (2026-07-10)

Alle Aussagen hier sind belegt, nicht angenommen. Fundstellen relativ zu
`/home/tholo/Scripts/lean-ctx`.

### 2.1 Was upstream bereits gemergt ist

| Fakt | Beleg |
|---|---|
| Die **#727-Basis** — `kind=skills`-Packs + depth-1-Dependency-Resolution — ist in `main` | Commit `683113f5 feat(pack): kind=skills content packs + depth-1 dependency resolution (GH #727) (#743)` |
| Unified Distribution Phase 1 (`[artifacts]`, managed binaries) | `ba7102dda` (#729) |
| Unified Distribution Phase 2 (`addon publish`, hosted installs) | `2afd821a2` (#734) |

Das P3-Design (§2.2) nannte zwei „Upstream-Blocker". Beide sind inzwischen **Basis**, nicht
Blocker. Was fehlt, sind die Folge-Fixes.

### 2.2 Was nur auf `pr-rebuild` liegt

Sechs Commits, alle `(#727)`:

| SHA | Inhalt |
|---|---|
| `b05bc5f14` | `{pack_dir:}`-Expander für `[mcp.env]` |
| `38699d7ce` | `[[dependencies]]`-Authoring-Surface, in den publizierten Pack durchgereicht |
| `a851718b1` | `min_lean_ctx` im Preflight **erzwingen** statt ignorieren |
| `c87f99950` | Deps **vor** dem Wiring installieren; `{pack_dir:}` in `[mcp.env]` expandieren |
| `39ff2cde0` | Deps auf jeder Quelle auflösen, installierte Versionen wiren |
| `18beb7bfe` | Self-Dependency-Guard auf dem Resolve-Pfad |

Ohne diese sechs zieht ein `addon add @dasTholo/lean-md` den Skills-Pack nicht,
`LEAN_MD_SKILLS_DIR` bleibt unset, und jeder Skill-Render bricht mit `PACK_MISSING` ab.

### 2.3 Der Tag `v3.9.4` trägt sie nicht

`v3.9.4` zeigt auf `c1124763` („fix: update Cargo.lock for 3.9.4"). Für alle vier
Kern-Commits gilt:

    git merge-base --is-ancestor {b05bc5f14,38699d7ce,a851718b1,c87f99950} c11247639  → Exit 1

Grund (aus dem Commit-Graph): `c1124763` liegt auf dem `main`-Strang, der per `a4a842b4d`
(„Merge branch 'main' into pr-rebuild") **nach** den #727-Commits hereingezogen wurde. Beide
Stränge treffen sich erst im Merge-Commit — und der liegt hinter dem Tag.

∴ `pr-rebuild` ⊇ #727 ✅ · `v3.9.4` ⊉ #727 ❌

Die lokal installierte `lean-ctx 3.9.4` ist aus `pr-rebuild` gebaut und **hat** die Fixes.
Das released `v3.9.4` hat sie nicht. `min_lean_ctx = "3.9.4"` im lean-md-Manifest ist damit
gegenüber dem Release **zu schwach** — es lässt ein Binary passieren, das den Vertrag nicht
kennt.

### 2.4 Was lokal ohne Netz funktioniert

| Fähigkeit | Beleg | Netz/Token? |
|---|---|---|
| `pack create --kind skills` materialisiert direkt in den Store | `cli/pack_cmd.rs:139`, `create_skills_pack` `:400-454` | nein |
| `pack import <file.ctxpkg>` / `pack install --file=` | `cli/pack_cmd.rs:918`, `:478-494` | nein |
| `pack export --sign` (lokaler ed25519-Key) | `cli/pack_cmd.rs:777`, `keys::load_or_create` `:873-885` | nein |
| `addon add <pfad/zu/lean-ctx-addon.toml>` | `cli/addon_cmd.rs:283-290` (`is_local_path` → `AddonManifest::from_path`) | nein |
| curated Registry-Lookup | `core/addons/registry.rs:15` (`include_str!` der `addon_registry.json`) | nein |
| `min_lean_ctx`-Preflight | `core/addons/install.rs:42` — vergleicht gegen `env!("CARGO_PKG_VERSION")` des **laufenden** Binaries | nein |
| `pack publish` | `cli/pack_cmd.rs:1109`, `ctxp_`-Token geprüft `:1143-1153` | **ja, beides** |

Store-Layout (bestätigt): `skills_dir(store, name, version)` =
`store_root.join("skills").join(name.replace('/', "__")).join(version)`
(`core/context_package/skills.rs:261-265`), Store-Root = `<data_dir>/packages`
(`core/context_package/registry.rs:49-53`). Konkret:
`~/.local/share/lean-ctx/packages/skills/@dasTholo__lean-md-skills/0.2.0/`.

`{pack_dir:}` expandiert daraus (`core/addons/pack_env.rs:63-89`, Aufrufer
`cli/addon_cmd.rs:437-446`), braucht aber einen `ResolvedDep`-Eintrag (Name + Version) —
der Pfad wird **nicht** per Store-Scan gefunden.

### 2.5 Der eine Punkt, der offline nicht durchläuft

`addon add` hat zwei Dep-Pfade, und nur einer ist offline-fähig:

- **Install-Pfad** — `pack_remote::install_declared_dependencies` (`cli/pack_remote.rs:185-252`)
  prüft zuerst `deps::already_satisfied` (`:203` → `core/context_package/deps.rs:164-185`):
  Lockfile-Treffer **und** Pack im lokalen Store ⇒ kein Netz.
- **Consent-Vorschau** — `resolve_declared_deps` (`cli/addon_cmd.rs:905-929`) ruft
  `deps::resolve_one` **ohne** diesen Fast-Path ⇒ schlägt **immer** gegen den Registry-Index
  an (`core/context_package/remote.rs:70-78`, Default `ctxpkg.com/api`).

∴ Ein vollständiger `addon add`-Durchlauf ist **auch mit lokalem Pack + Lockfile** nicht
netzfrei. Das ist ein Upstream-Bug (die Vorschau hat keinen Offline-Zweig), kein
Konfigurationsproblem.

> **Nebenbefund (Sicherheit):** `already_satisfied` verifiziert `artifact_sha256` nicht gegen
> den Store-Inhalt — es kopiert den Hash aus dem Lockfile durch (`deps.rs:164-185`). Ein
> handgeschriebener Lockfile-Eintrag mit beliebigem Hash passiert den Fast-Path.
> Nicht Teil dieser Spec; upstream melden.

### 2.6 Die `.lmd.md`-Render-Delegation

`try_lmd_addon_render` (`rust/src/tools/registered/ctx_read.rs:51`) bekommt den `mode`
**nicht als Parameter**. Es prüft nur Endung, Gateway-Status und Katalog-Eintrag
`ctx_md_render` (`:56-73`) und delegiert dann. `handle_inner` ruft es vor der Cache-Maschinerie
auf (`:183`).

∴ **Jeder** `ctx_read`-Modus rendert eine `.lmd.md`, `mode=raw` eingeschlossen.
Empirisch bestätigt: `ctx_read(content/core/dispatch-contract.lmd.md, mode="raw")` liefert
expandierte `@include`s und `<!-- lmd:{{ }} eval err … "role" -->`, während
`lean-md source` dieselbe Datei mit `{{ role }}` und unaufgelöstem `@include hard-rules` zeigt.

**Aber die Delegation ist nie released worden.** `git grep try_lmd_addon_render upstream/main`
→ kein Treffer. Sie existiert ausschließlich auf `pr-rebuild` (`d5f94ec9d`). Jede
veröffentlichte lean-ctx-Version liest `.lmd.md` roh. Nur wer gegen ein **lokal aus
`pr-rebuild` gebautes** Binary arbeitet, sieht das Rendern — so wie diese Entwicklungsumgebung.

**Zwei Folgerungen:**

1. Die Doku-Korrektur (§5.1) **wartet auf nichts**. Die Aussagen auf `skill-fix` beschreiben
   Verhalten, das kein Nutzer je hatte. Sie sind schon heute falsch, unabhängig von PR #721.
2. Der Übergangsabsatz in `AGENTS.md`/`CLAUDE.md` (aus `3df0758`) ist zu stark formuliert
   („bis PR #721 gemergt ist"). Richtig: er gilt nur für die lokale Dev-Instanz.

Der Maintainer hat in PR #721 (2026-07-08) ausdrücklich den **Gegen**zustand gelobt:
*„The `.lmd.md` raw-read decision (no auto-render delegation) is the right call — it keeps
`ctx_read` simple and deterministic."* Auf `origin/pr/lean-md-addon` war die Delegation mit
`15c3683f3` bereits entfernt; `pr-rebuild` hat sie (neu aufgebaut) wieder.

---

## 3. Neubewertung der P3-Vorbedingungen

Das P3-Design (§5.1) definierte V1–V4 als harte Sperre für die Tasks 3–5. Diese sind
inzwischen implementiert (die Sperre wurde bewusst überstimmt, nachdem der Cut als
code-complete-aber-nicht-release-fähig akzeptiert wurde). Für den **Release** gilt:

| # | Ursprünglich | Neubewertung |
|---|---|---|
| **V1** | „lean-ctx-Upstream-Vertrag released" | **zerfällt.** **V1a** (lokal gebautes lean-ctx trägt den Vertrag) ist **erfüllt** — daran hängt jede lokale Verifikation. **V1b** (released) ist **offen** und blockiert nur den Nutzer-Install. |
| **V2** | „lean-md `v0.2.0` mit echten SHA-256" | **offen**, aber **falsch einsortiert**: ein Tag-Release *vor* dem Cut baut ein Binary mit eingebetteten Skills; dessen Pins wären nach dem Cut wertlos. V2 gehört **hinter** den Cut. |
| **V3** | „curated Entry auf `listed`" | **offen**, aber **kein Blocker**: der curated Entry dient allein der Auffindbarkeit über `addon search`. Ausgeliefert wird über `addon publish --namespace` (hosted); der `[artifacts]`-Block lebt in lean-mds eigenem Manifest. **Achtung:** `listed` heißt **nicht** „`mcp`-Block entfernen" — siehe §5.3 Schritt 5. |
| **V4** | „beide Packs hosted publiziert" | **offen.** Der Skills-Pack muss **zuerst** publiziert sein: `version_req = "^0.2"` löst depth-1 gegen den **Registry-Index** auf; ein Pack, der nur als GitHub-Asset oder lokal existiert, ist für den Resolver unsichtbar. |

**Konsequenz:** Der Release ist nicht durch V1 blockiert, sondern nur der *Nutzer-Install*.
Alles, was lean-md selbst verifizieren kann, kann **jetzt** verifiziert werden.

---

## 4. Entscheidungen (verbindlich, 2026-07-10)

- **D1 — Die `.lmd.md`-Auto-Render-Delegation kommt aus lean-ctx heraus.** Wir folgen dem
  Maintainer. `ctx_read` liefert Roh-Bytes; Rendern ist explizit (`ctx_md_render` / CLI).
  Umsetzung in PR #721, nicht in lean-md.
- **D2 — `#727` und `#721` werden getrennt.** Die sechs Folge-Fixes gehen in einen eigenen,
  kleinen lean-ctx-PR gegen `main`. Er entblockt den lean-md-Release; #721 reift unabhängig.
- **D3 — `min_lean_ctx = "3.9.4"` bleibt**, unter der Bedingung, dass der Release, der die
  #727-Fixes trägt, ebenfalls `3.9.4` heißt (Tag neu setzen). Geht er als `3.9.5` raus, ist
  eine Zeile in `lean-ctx-addon.toml` nachzuziehen — der CI-Job `pack-drift.yml` liest den
  Wert aus dem Manifest und zieht automatisch mit.
- **D4 — Doku beschreibt die Zielsemantik, die Gegenwart als befristete Ausnahme.**
  Umgesetzt in `3df0758`: `AGENTS.md` + `CLAUDE.md` sagen „raw source", tragen aber einen
  Übergangsabsatz mit Löschbedingung. Der SDD-Skill-Body und die `skill-fix`-Stellen folgen
  beim Merge.
- **D5 — Der Live-Smoke wird zweigeteilt.** Was ohne Registry prüfbar ist, wird jetzt geprüft
  (§5.2). Der `addon add`-Vollpfad folgt nach hosted publish.

---

## 5. Weitere Schritte für lean-md

### 5.1 Konsolidierung (jetzt, netzfrei) — „Paket C"

> **Ausführbare Anleitung: `paket-c-next-session.md` im Repo-Root.** Sie ist die Autorität für
> die Schritte; hier steht nur die Einordnung. Wichtig aus §2.6: Paket C **wartet auf nichts** —
> die Delegation ist nie released worden, die Doku ist schon heute falsch.

1. **`skill-fix` → `feat-lmd-v2` mergen.** Konfliktfrei (`git merge-tree`). Bringt vier
   sinnvolle Commits (SDD-Hardening) plus zwei Stellen, die nach D1 falsch werden.
2. **Korrektur-Commit** — dieselben Stellen, die `3df0758` bereits auf `feat-lmd-v2` gefixt
   hat, für die `skill-fix`-Seite:
   - `AGENTS.md`, `CLAUDE.md` (aus `6657cba`) — auf Raw-Read + Übergangsnotiz
   - `content/core/hard-rules.lmd.md` (aus `9587936`) — Access-Map entfernen
   - `src/fragments.rs` — Test `hard_rules_carries_the_lmd_access_map` (assertet
     `"rendered artifact"` und `"lean-md source"`) wandert mit dem Seed
   - `content/skills/lmd-subagent-driven-development/body.lmd.md`, Phase `preflight` — der
     Satz „Do NOT `ctx_read` the plan — any read mode renders it" stand **schon vor**
     `skill-fix` dort und muss ebenfalls
   - Der Commit referenziert `docs/lean-md/plans/2026-07-10-lmd-sdd-skill-hardening.lmd.md`
     Task 1 (Ä3) ausdrücklich als **revidiert**, samt Kopf-Notiz in der zugehörigen Spec.
3. **Rebless** — weil Schritt 2 `content/skills/**` anfasst:
   - `LEAN_MD_BLESS=1 cargo nextest run --test pack_drift` → `content/skills.sha256`
   - `content/skills.ctxpkg-hash` aus `pack create … --version 0.0.0-cihash`,
     `integrity.content_hash` aus `<pkg_dir>/manifest.json`
   - beide Dateien liegen **neben** `content/skills/`, nie darin
4. `cargo nextest run` + `cargo clippy --all-targets -- -D warnings` grün.

> **Reihenfolge ist bindend:** 5.1 muss **vor** 5.3 laufen. Publizierte Pack-Versionen sind
> immutable (das Lockfile pinnt `artifact_sha256`); jede spätere Content-Änderung kostet einen
> Bump auf `0.2.1` plus Republish.

### 5.2 Lokaler Vertrags-Smoke (jetzt, ohne Registry)

Verifiziert alles, was V1a hergibt. Vorbedingung: lean-ctx aus `pr-rebuild` gebaut.

| Prüfung | Kommando | Erwartung |
|---|---|---|
| Pack materialisiert lokal | `lean-ctx pack create --kind skills --name @dasTholo/lean-md-skills --version 0.2.0 --from content/skills` | Store-Pfad `…/packages/skills/@dasTholo__lean-md-skills/0.2.0/`, 42 Dateien |
| Release-Binary rendert **aus dem Pack** | `cargo build --release`; `LEAN_MD_SKILLS_DIR=<store-pfad> ./target/release/lean-md render --skill lmd-brainstorm --phase pre-context` | nicht-leerer Render |
| **Negativprobe** — Produktion ohne Pack ist hart-fehlerhaft | `env -u LEAN_MD_SKILLS_DIR ./target/release/lean-md render --skill lmd-brainstorm --phase pre-context` | Exit ≠ 0, `PACK_MISSING LEAN_MD_SKILLS_DIR is unset …` |
| Debug-Fallback greift, Release-Fallback nicht | `cargo run -- render --skill …` (grün) vs. Release ohne Env (rot) | `cfg(debug_assertions)` wirkt |
| Overlay schlägt Pack | Sentinel in `<jail>/.lean-ctx/lean-md/skills/<skill>/body.lmd.md` | Sentinel gewinnt |
| `min_lean_ctx`-Gate | `lean-ctx addon add ./lean-ctx-addon.toml` mit lokal gebautem `3.9.4` | Preflight passiert (`install.rs:42`) |
| Assets + Exec-Bit | `lean-md skill install lmd-brainstorm --local` | 5 Scripts, `*.sh` mode `0755` |

**Nicht** lokal prüfbar (§2.5): der `addon add`-Vollpfad inklusive Consent-Vorschau,
Lockfile-Pinning und Offline-Reproduzierbarkeit des zweiten Installs. Der braucht einen
erreichbaren Registry-Index (`--registry` / `CTXPKG_REGISTRY` — ein Mock-Server genügt).

### 5.3 Release (nach dem lean-ctx-Merge)

1. **lean-ctx:** PR mit den sechs #727-Fixes mergen; Tag `v3.9.4` neu setzen (oder `v3.9.5`,
   dann D3 nachziehen); Release bauen. → schließt **V1b**
2. **Pack publizieren** (braucht Token, Maintainer-Hand):
   `pack create` → `pack export --sign` → `pack publish pack.ctxpkg --token ctxp_…`
   → schließt **V4a**
3. **lean-md `v0.2.0` taggen** → 5-leg-Build → `sync-manifest` schreibt die echten SHA-256 in
   die fünf `[artifacts]`-Blöcke. → schließt **V2**
4. **`lean-ctx addon publish --namespace dasTholo`** → schließt **V4b**
   **Stopp-Bedingung:** das publizierte `pack_manifest` muss das `[[dependencies]]`-Array
   tragen. Leeres Array ⇒ das benutzte lean-ctx hat die #727-Fixes nicht ⇒ hier abbrechen.
5. **curated Entry auf `listed`** (lean-ctx-Repo, nicht lean-md): `install`-Block entfernen,
   `mcp`-Block **behalten** und `command`/`args` leeren — die `letta`-Form.
   `pub mcp: AddonMcp` ist **kein** `Option`; alle 21 Entries tragen den Block.
   `is_installable()` = `to_gateway_server().resolve().is_ok()` (`core/addons/manifest.rs:257`);
   ohne auflösbares Kommando schlägt das fehl und der Entry gilt als listed.
   Der Installationspfad ist ohnehin `@dasTholo/lean-md` (hosted), nicht der Registry-Slug.
   → schließt **V3**. Details: `lean-ctx/docs/specs/2026-07-10-pr721-pr727-split-design.md` §3.4–3.5
6. **Task 7 Voll-Smoke** (P3-Plan, `--phase task-7`): `addon add @dasTholo/lean-md`,
   Consent listet den Pack, Lockfile pinnt das Paar, zweiter Install offline-reproduzierbar.

---

## 6. Risiken

| # | Risiko | Mitigation |
|---|---|---|
| **N1** | Der Release, der die #727-Fixes trägt, heißt `3.9.5`, `min_lean_ctx` bleibt auf `3.9.4` | Dann lässt das Gate ein vertragsloses `3.9.4` durch → stiller `PACK_MISSING` beim Nutzer. D3 zwingt zur Entscheidung; `pack-drift.yml` liest den Wert aus dem Manifest und deckt Drift auf. |
| **N2** | Tag `v3.9.4` wird force-verschoben; Downloader mit gecachten Assets bekommen es nicht mit | Bewusste Abwägung. Alternative `v3.9.5` ist sauberer, kostet eine Manifest-Zeile. |
| **N3** | Doku-Rückdrehung (5.1) nach dem Pack-Publish | Reihenfolge in 5.1 ist bindend; Bump auf `0.2.1` + Republish wäre die Strafe. |
| **N4** | Delegation-Ausbau (D1) bricht den `reverse_cut_gate.rs`-Test oder `ctx_read_lmd_md_raw.rs` | Beide sind in PR #721 mitzuziehen; der Raw-Fallback existiert bereits vollständig (`ctx_read.rs:59,67` steigen sauber aus). |
| **N5** | `addon add`-Vorschau ohne Offline-Zweig blockiert jeden netzfreien Voll-Smoke | Akzeptiert (D5). Upstream melden; optional in PR A mitfixen. |
| **N6** | `already_satisfied` verifiziert `artifact_sha256` nicht gegen den Store | Nicht lean-md-Scope. Upstream melden (§2.5). |

---

## 7. Definition of Done

**Konsolidierung (5.1):**
- `skill-fix` gemergt; Korrektur-Commit revidiert Ä3 und referenziert Plan + Spec.
- `AGENTS.md`, `CLAUDE.md`, `hard-rules`-Seed, `fragments.rs`-Test und SDD-`preflight`
  beschreiben Raw-Read; der Übergangsabsatz trägt seine Löschbedingung.
- `content/skills.sha256` + `content/skills.ctxpkg-hash` neu geblesst; Drift-Gate grün.
- Gesamte Suite + clippy grün.

**Lokaler Smoke (5.2):**
- Release-Binary rendert aus dem Pack-Store; ohne `LEAN_MD_SKILLS_DIR` bricht es hart mit
  `PACK_MISSING` ab (kein stiller Leerlauf).
- Overlay schlägt Pack; Debug-Fallback greift nur im Dev-Build.
- `min_lean_ctx`-Preflight passiert mit dem lokal gebauten lean-ctx.

**Release (5.3):**
- V1b, V2, V3, V4 geschlossen; das publizierte `pack_manifest` trägt `[[dependencies]]`.
- Task 7 des P3-Plans durchlaufen.

**Nicht enthalten:** Skill-Tiering (P3-Design §8), P4 (Publisher-Identität/Signing), das
Verschieben von `content/lang` / `content/tooling` in den Pack, der Offline-Fix der
`addon add`-Vorschau (§2.5, Upstream).
