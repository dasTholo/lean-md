# Session-Prompt: lean-md — P3 `kind=skills`-Pack Full-Cut (#727)

> **⚠ ÜBERHOLT (2026-07-10) — maßgeblich ist `2026-07-10-lmd-release-path-rev2-design.md`.**
> Drei Aussagen sind widerlegt: (1) der `v3.9.4`-Retag-Plan unter „Blocker B1" ist tot, `3.9.5`
> ist released; (2) „`listed` = kein `mcp`-Block" ist falsch — `pub mcp: AddonMcp` ist kein
> `Option`, die `letta`-Form leert `command`/`args` und behält den Block; (3) die #727-Commit-SHAs
> existieren auf keinem Remote-Branch mehr (rebased → PR #780). Der Abschnitt „Wo wir stehen"
> und die Kennzahlen bleiben gültig.
>
> Hand-off-Datei. Stand **2026-07-10**. Die Vorgänger-Fassung (P0 `[artifacts]`-Release,
> P1 Target-Matrix, P2 hosted publish) ist abgearbeitet bzw. in P3 aufgegangen — Historie via
> `git log -- docs/lean-md/specs/lean-md-next-session.prompt.md`.
>
> Repo `lean-md`, Branch `feat-lmd-v2`, HEAD `b0c8964`. Gegenstück in lean-ctx: Branch `pr-rebuild`.
> Plan: `docs/lean-md/plans/2026-07-09-lmd-p3-skills-pack-full-cut.lmd.md`
> Spec: `docs/lean-md/specs/2026-07-08-lmd-p3-skills-pack-full-cut-design.md`

---

## Wo wir stehen

**Tasks 1–6 sind code-complete**, per Task-Review *und* Whole-Branch-Final-Review abgenommen und
committet. `cargo nextest run` → 561 passed / 16 skipped. `cargo clippy --all-targets -- -D warnings`
clean. **Task 7 (End-to-End Live-Smoke) ist offen** — reine Maintainer-Hand, kein Code, hängt an
den Vorbedingungen V1–V4.

Der `include_str!`-Cut für Skill-Content ist vollzogen. `content/skills/` (42 Dateien) wird zur
Laufzeit über die Kaskade **Overlay → Pack-Store (`$LEAN_MD_SKILLS_DIR`) → Debug-Fallback**
aufgelöst (`src/skill_source.rs`). Vier Konsumenten lesen durch sie: `skills.rs` (Bodies,
Companions), `fragments.rs` (die 3 skill-lokalen `_includes/`), `skill_install.rs` (`SKILL.md`,
Assets), `bridges/dispatch.rs` (Companion-Briefs).

**Bewusst embedded geblieben:** `content/core/hard-rules`, `content/core/dispatch-contract`,
`content/core/_fragments/parallel-dispatch`, `content/gloss/directives`. Sie sind der Grund, dass
ein Standalone-Render eines gewöhnlichen `.lmd.md` ohne Pack lauffähig bleibt; der
#498-Fragment-Consistency-Gate (built-in == on-disk) ist für sie grün.

### Commits (`ba90947..b0c8964`)

| SHA | Inhalt |
|---|---|
| `ba90947` | `lean-ctx-addon.toml`: `[[dependencies]]` + `[mcp.env]` + `min_lean_ctx` |
| `c272d17` | neu `src/skill_source.rs` — 3-Stufen-Kaskade |
| `97f4b34` | `min_lean_ctx` 3.9.3 → 3.9.4 (Manifest + Plan) |
| `3cd2a5a` | `src/skills.rs`: Bodies + Companions off `include_str!` |
| `f1b5c94` | Modul-Doc-Header korrigiert |
| `d6a64bc` | `src/fragments.rs`: 3 skill-lokale `_includes` off `include_str!`; Core bleibt builtin |
| `44feadb` | `src/skill_install.rs`: `SKILL.md`-Stubs + Assets off `include_str!` |
| `372e903` | Drift-Gate: `tests/pack_drift.rs`, `content/skills.sha256`, `content/skills.ctxpkg-hash`, CI |
| `b0c8964` | Test: jeder der 8 `INSTALLABLE_SKILLS`-Stubs resolved (Final-Review-Finding) |

(`1e1a241` dazwischen ist ein fremder Docs-Commit aus einem anderen Prozess, nicht Teil von P3.)

### Kennzahlen

- 42 Content-Dateien: 8 Bodies + 17 Companions + 8 `SKILL.md` + 6 Assets + 3 `_includes`.
  Der Final-Review hat die vier Registries in `src/` gegen `content/skills.sha256` gekreuzt —
  exakte Deckung, keine Datei, die der Code braucht und der Pack nicht trägt.
- `content/skills.ctxpkg-hash` = `16685fefde1207c1151394c0567b0bb8f252b150ffbb90b324f4235017ce8fd7`
  (lokal mit echtem `lean-ctx pack create` erzeugt, vom Reviewer unabhängig reproduziert).
- Release-Binary 5 287 560 B; Sentinel `I'm using the lmd-writing-plans skill` **nicht** mehr in
  den Binary-Bytes.

---

## Gate-Status — geprüft, nicht angenommen (2026-07-10)

| Gate | Stand | Nachweis |
|---|---|---|
| **V1** lean-ctx-Upstream-Vertrag released | ❌ **nicht erfüllt** | Tag `v3.9.4` existiert upstream (`c1124763`), enthält aber **keinen** der vier #727-Commits. `git merge-base --is-ancestor {b05bc5f14,38699d7ce,a851718b1,c87f99950} c11247639` → Exit 1 für alle vier. Sie liegen nur auf `pr-rebuild` (48 Commits vor `upstream/main`). |
| **V2** lean-md `v0.2.0` mit echten SHA-256 | ❌ nicht erfüllt | `git tag` und `git ls-remote --tags origin` leer; die 5 `[artifacts]`-Pins stehen auf `0000…0000`. |
| **V3** curated Registry-Entry auf `listed` | ❌ nicht erfüllt | `rust/data/addon_registry.json:389-440` trägt weiterhin `mcp`- und `install`-Block. |
| **V4** beide Packs hosted publiziert | ❌ nicht erfüllt | `@dasTholo/lean-md-skills` existiert nur lokal. |

> **Was `listed` technisch heißt:** es gibt kein `status`-Feld. Ein Entry ist listed-only, wenn er
> **keinen `[mcp]`-Block** hat (`manifest.rs:255`, `is_installable()`); `registry.rs:148` verlangt
> dann nur eine `homepage` — die ist vorhanden. V3 = `mcp`- und `install`-Block aus dem
> `lean-md`-Entry entfernen. Reversibel. Ein listed-only-Entry braucht **keine** Artefakt-Hashes.

---

## Blocker B1 — der Tag `v3.9.4` trägt den #727-Vertrag nicht

**Entscheidung (2026-07-10): `min_lean_ctx = "3.9.4"` bleibt.** Das lean-md-Manifest wird nicht
angefasst. Die Korrektur liegt drüben.

Der Tag `v3.9.4` zeigt auf `c1124763` ("fix: update Cargo.lock for 3.9.4"). Keiner der vier
#727-Commits (`b05bc5f14` pack_env, `38699d7ce` `[[dependencies]]`, `a851718b1` min_lean_ctx-Gate,
`c87f99950` Install-Order) ist dessen Vorfahre — sie liegen ausschließlich auf `pr-rebuild`.
Das lokal installierte `lean-ctx 3.9.4` hat sie (aus `pr-rebuild` gebaut), das **released**
`v3.9.4` nicht.

Folge, wenn das so bliebe: ein Nutzer mit released `v3.9.4` passiert das Preflight-Gate, aber sein
lean-ctx kennt weder `[[dependencies]]` noch die `{pack_dir:…}`-Expansion. Der Pack wird nie
gezogen, `LEAN_MD_SKILLS_DIR` bleibt unset, und jeder Skill-Render bricht mit `PACK_MISSING` ab —
genau der stille Leerlauf, den der Global Constraint „Produktion fehlt der Pack ⇒ harter Fehler"
verhindern soll, nur eine Ebene höher (im Installer statt im Renderer).

**Zu tun (lean-ctx-Repo):** `pr-rebuild` nach `main` mergen, Tag `v3.9.4` neu setzen (force) und
das Release neu bauen, sodass die vier Commits enthalten sind. Danach ist V1 erfüllt und
`min_lean_ctx = "3.9.4"` in lean-md korrekt.

---

## Wo die SHA-Pins herkommen (häufige Verwechslung)

**In lean-ctx sind keine SHAs zu setzen.** Die fünf `sha256 = "0000…0000"` stehen in *lean-mds
eigenem* `lean-ctx-addon.toml` und werden nicht von Hand gepflegt: der Tag `v0.2.0` löst den
5-leg-Build aus, und der `sync-manifest`-Job schreibt die echten Hashes aus `SHA256SUMS` zurück
ins Manifest. Ein lean-md-interner Zyklus (Tag → Build → Rückfluss).

In lean-ctx sind nur zwei Dinge zu tun, beide ohne Hashes: **B1** (Release `v3.9.4` mit #727) und
**V3** (curated Entry auf `listed`).

**Pack und Binary-Release sind entkoppelt.** Der Pack hängt nur an `content/skills/` und kann
publiziert werden, sobald ein lean-ctx mit `kind=skills`-Support läuft. Er *muss* sogar zuerst
publiziert sein: `addon publish` deklariert `version_req = "^0.2"` auf ihn, und die
depth-1-Resolution löst gegen den **Registry-Index** auf — ein Pack, der nur als GitHub-Asset
existiert, ist für den Resolver unsichtbar.

---

## Reihenfolge bis Task 7

Der Plan listet **V2 als Vorbedingung *vor* dem Cut**. Das ist zirkulär: ein Tag-Release vor dem
Cut baut ein Binary mit noch eingebetteten Skills, dessen SHA-Pins nach dem Cut sofort wertlos
sind. V2 gehört hinter Tasks 3–6. Tragfähig ist:

1. **lean-ctx:** `pr-rebuild` → `main`, Tag `v3.9.4` neu setzen, Release neu bauen. → löst **V1**/**B1**.
2. **Pack publizieren** (unabhängig vom Binary):
   ```
   lean-ctx pack create --kind skills --name @dasTholo/lean-md-skills --version 0.2.0 --from content/skills --description "lmd skills"
   lean-ctx pack export @dasTholo/lean-md-skills@0.2.0 --sign --output pack.ctxpkg
   lean-ctx pack publish pack.ctxpkg --token ctxp_…
   ```
   Von Hand — es liegt bewusst kein Publish-Token in der CI-Umgebung. → löst **V4a**.
3. **lean-md `v0.2.0` taggen** → 5-leg-Build → `sync-manifest` schreibt die echten SHA-Pins in die
   5 `[artifacts]`-Blöcke. → löst **V2**.
4. **`lean-ctx addon publish --namespace dasTholo`** — und **prüfen, nicht annehmen**, dass das
   publizierte `pack_manifest` das `[[dependencies]]`-Array wirklich durchreicht (`publish.rs` tut
   das seit `38699d7ce`). Ein leeres `dependencies` heißt: V1 steckt nicht in der Release-Version,
   und der Ablauf stoppt hier. → löst **V4b**.
5. **Curated Entry auf `listed`:** im lean-ctx-Repo `mcp`- und `install`-Block aus dem
   `lean-md`-Entry in `rust/data/addon_registry.json` entfernen. → löst **V3**.
6. **Task 7 Live-Smoke** fahren:
   `lean-md render docs/lean-md/plans/2026-07-09-lmd-p3-skills-pack-full-cut.lmd.md --phase task-7 --consumer=ai`

**Versions-Kopplung (Decision-Record):** Pack und Binary tragen getrennte SemVer-Linien, initial
beide `0.2.0` — bequemer Startpunkt, **kein Vertrag**. `version_req = "^0.2"` deckt auf der
`0.x`-Linie `>=0.2.0, <0.3.0`. Ein reiner Content-Fix hebt den Pack auf `0.2.1`; das Manifest wird
**nicht** angefasst, der Addon-Pack **nicht** republiziert.

---

## Offene Punkte im lean-ctx-Repo (`pr-rebuild`, uncommitted)

- `rust/data/addon_registry.json`: `min_lean_ctx` des lean-md-Entries auf `3.9.4` gehoben —
  deckt sich mit lean-md, kann so committet werden.
- `rust/src/tools/registered/ctx_read.rs`: `core::gateway::` → `core::mcp_catalog::` umbenannt.
  Gehört nicht zu #727, offenbar Kollateral einer Upstream-Umbenennung. Einordnen und committen
  oder verwerfen.

---

## Bewusst nicht gefixt (Final-Review, beide Minor)

- **Overlay überschreibt Pack-Content ohne Warnung.** `skill_source.rs` Stufe 1 gewinnt still über
  den signierten Pack — jetzt an vier Call-Sites statt einer, und der Blast-Radius deckt
  Agent-Instruktionstext ab (Bodies, Companions, `SKILL.md`, Assets). Jailed und by design, aber
  ein geplantetes Overlay in einem geklonten Repo überschreibt vertrauenswürdigen Skill-Text
  lautlos. Erwägen: stderr-Hinweis, wenn das Overlay greift.
- **`fragments.rs` mappt `SourceError::NotFound` → `ResolveError::Io`.** Die Variante verliert
  Präzision; die Meldung bleibt aktionabel (`SKILL_FILE_NOT_FOUND` / `PACK_MISSING` im Text) und
  kein Caller verzweigt darauf.

Beides ist plan-mandatierter Verbatim-Code — Änderung nur nach Rücksprache.

---

## Dev-Workflow (seit P3)

| Änderung | Kanal | Ablauf |
|---|---|---|
| `content/skills/**` | Pack | Bump + `pack create` + `pack publish`. Kein Tag, kein Binary. |
| `content/core/**`, `content/gloss/**`, `src/**` | Binary | Tag `v*` → 5-leg-Build → `sync-manifest` schreibt die SHA-Pins. |

Skill-Content ändern: editieren → `LEAN_MD_BLESS=1 cargo nextest run --test pack_drift`
(schreibt `content/skills.sha256`) → `pack create` → `content/skills.ctxpkg-hash` aus
`<pkg_dir>/manifest.json` (`integrity.content_hash`) aktualisieren → `pack export --sign` →
`pack publish` von Hand.

Lokal ohne Pack: `cargo run -- render --skill X --phase Y` greift auf den Debug-Fallback
(`$CARGO_MANIFEST_DIR/content/skills`). Im Release-Binary ist er inert (`cfg(debug_assertions)`).

Details: `docs/dev-readme.md`.

---

## Reverse-Cut-Disziplin (gilt unverändert)

lean-md ist standalone (`rushdown` + `evalexpr`); `[dependencies]` trägt **kein** `lean_ctx`
(nur optional `lean-ctx-client` hinter dem `mcp`-Feature). Der lean-ctx-`content_hash` wird
deshalb **nicht** in Rust nachgerechnet — der lokale Drift-Gate hat einen eigenen, unabhängigen
sha256 (`tests/pack_drift.rs`), der CI-Cross-Check ruft das echte lean-ctx-Binary
(`.github/workflows/pack-drift.yml`). Kein Engine-Symbol darf zurück nach `lean-ctx/rust/src` lecken.

---

## P4 — Publisher-Identität & Signing (Awareness, #728, blocked/low)

Kein aktiver Task. Beim Publishen mitdenken: `@dasTholo/lean-md` (Binary) **und**
`@dasTholo/lean-md-skills` unter **derselben** `account claim dastholo`-Identität publizieren →
gemeinsame Reputation über beide Kanäle. lean-md ist Apache-2.0/kostenlos, die
Commerce-Aktivierung ist irrelevant.

---

## Wiedereinstieg

```
ctx_session action=load
ctx_knowledge action=recall query="P3 727 full-cut"
cargo nextest run
lean-md render docs/lean-md/plans/2026-07-09-lmd-p3-skills-pack-full-cut.lmd.md --list-phases
```

`--list-phases` rendert keine Bodies und ist Bug-3-immun. Der Task-Brief ist immer der
Phasen-Render (`--phase task-N`), nie ein Roh-Read des Plans.
