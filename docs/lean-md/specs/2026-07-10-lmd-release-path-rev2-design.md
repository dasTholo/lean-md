# Design-Spec: lean-md Release-Pfad — Revision 2 (Upstream-Realität)

> Erstellt: 2026-07-10 · Branch: `feat-lmd-v2` (HEAD `ec28841`)
>
> **Diese Spec ersetzt zwei Vorgänger. Beide sind überholt:**
> - `2026-07-10-lmd-release-path-design.md` — Revision 1. Ihre §2-Fundstellenarbeit war
>   korrekt für den damaligen Stand, ihre Grundannahme ist es nicht mehr: sie setzt ein
>   lokal aus `pr-rebuild` gebautes lean-ctx voraus (V1a erfüllt) und einen noch nicht
>   veröffentlichten `3.9.5`-Release. Beides gilt nicht mehr.
> - `lean-md-next-session.md` — P3-Handoff. Enthält drei widerlegte Aussagen: der
>   `v3.9.4`-Retag-Plan (tot, `3.9.5` ist released), „listed = kein `mcp`-Block"
>   (falsch — siehe §5.5) und die #727-Commit-SHAs (existieren auf keinem Remote mehr).
>
> **Autorität für die lean-ctx-Seite:** `/home/tholo/Scripts/lean-ctx/docs/specs/2026-07-10-pr721-pr727-split-design.md`
> **Autorität für Paket C (neu):** `2026-07-11-lmd-paket-c-rev3-abandon-skill-fix-design.md` — ersetzt `paket-c-next-session.md`.
>
> **Stand 2026-07-11 (erledigt):** `3.9.6` ist upstream released und lokal installiert — **mit**
> dem #727-Vertrag (PR #780 gemergt) und dem gemergten PR #721. Damit sind **V1a, V1b und V3
> geschlossen**; §1.1–1.3 dokumentieren nur noch den historischen 3.9.5-Zwischenstand. **Paket C
> (§5.1, = Gate V0) ist ausgeführt und abgenommen** — `skill-fix` abandonniert statt gemergt;
> 6/6 Tasks per Zwei-Verdikt-Review + Whole-Branch-Final-Review durch (keine Findings), Suite
> 562/562 grün, Smoke Teil 1 (§4.1) grün. Commits `2a93810..0d0fd72`, Branch `feat-lmd-v2` lokal
> gehalten (kein Push). Maßgeblich: `2026-07-11-lmd-paket-c-rev3-abandon-skill-fix-design.md`.
> **Verbleibend: V2, V4a, V4b** (§5.4–5.7 — Netz/Token/Maintainer).
>
> **Phase V2 + Registry-Nachzug §5.8 — in Arbeit:** eigene Spec
> `2026-07-11-lmd-v2-tag-registry-nachzug-design.md` (Ansatz A, kein RC, via CI).
> **Der Fortschritt dieser Phase wird in `ctx_knowledge` gehalten** (nicht in dieser
> Datei) — Schlüssel `phase-design-v2-registry-nachzug` und die je Task/Gate
> ergänzten `progress`-Einträge. Diese Spec bleibt der statische Gate-Überblick;
> laufender Stand → Knowledge abfragen (`ctx_knowledge action=recall`).

---

## 1. Verifizierter Ausgangsstand (2026-07-10)

Alles hier ist belegt, nicht angenommen. Kommandos gegen `/home/tholo/Scripts/lean-ctx`.

### 1.1 (historisch) 3.9.5 trug den #727-Vertrag nicht — 3.9.6 trägt ihn

Bei Rev. 2 stand `upstream/main` auf `87559528c release 3.9.5` — **ohne** `{pack_dir:}`-Expander,
`[[dependencies]]`-Authoring und `min_lean_ctx`-Preflight; nur die `kind=skills`-Basis war gemergt.
Empirisch: `addon add --dry-run` nannte den Skills-Pack nicht (stiller Leerlauf). **Erledigt:** PR
#780 (diese drei Fixes) ist gemergt und als **3.9.6** released; die installierte `lean-ctx 3.9.6`
trägt den Vertrag. Die Pack-Erzeugung (`pack create --kind skills`, 42 Dateien) hing ohnehin nie
an #780.

### 1.2 (erledigt) Die #727-Folge-Fixes sind über PR #780 gemergt

PR #780 — `fix(addons): #727 follow-up fixes …` — ist **gemergt** und in **3.9.6** released:
`{pack_dir:}`-Expander, `[[dependencies]]`-Authoring (in den publizierten Pack durchgereicht),
`min_lean_ctx`-Preflight, Deps-vor-Wiring, Self-Dependency-Guard. Ein lokales 3.9.6-Binary meldet
`3.9.6` und trägt den Vertrag — relevant für D3 und D7 (§3).

### 1.3 (erledigt) PR #721 ist gemergt

PR #721 — `feat: integrate lean-md as an external lean-ctx addon (+ LSP formatter routing)` — ist
**gemergt**. Der curated Registry-Entry übernimmt den `letta`-Slot in der vorgeschriebenen Form
(`mcp`-Block behalten, `command`/`args` leer, `integration = "mcp"`, `min_lean_ctx = "3.9.4"`); die
Auto-Render-Delegation ist entfernt (§1.4). **V3 ist geschlossen.** Offener Nachzug: `min_lean_ctx`
im Entry auf `"3.9.6"` heben (§5.8).

### 1.4 Die Auto-Render-Delegation ist weg

`git grep try_lmd_addon_render upstream/main -- rust/src` → Exit 1. Empirisch gegen die
installierte `3.9.5`: `ctx_read(content/core/dispatch-contract.lmd.md, mode="raw")` liefert
`{{ role }}` und unaufgelöstes `@include hard-rules` — Roh-Bytes.

∴ **Die Löschbedingung des Übergangsabsatzes in `AGENTS.md`/`CLAUDE.md` ist eingetreten.**
Er wird gelöscht, nicht abgeschwächt. Kein released lean-ctx hat die Delegation je gehabt;
die lokale Instanz hat sie nun auch nicht mehr.

### 1.5 Die Consent-Vorschau bleibt netz-zwingend — auch in #780

`origin/pr/addon-pack-deps:rust/src/cli/addon_deps.rs:63` (`resolve_declared_deps`) ruft
`deps::resolve_dependencies` **ohne** den `already_satisfied`-Fast-Path, den der Install-Pfad
hat (`pack_remote.rs:203`). Bei Fehlschlag: `std::process::exit(1)`.

Der eigene Docstring räumt zusätzlich ein: *„This is a **preview only**: it picks the highest
in-range version and does not consult `ctxpkg.lock`."*

∴ `addon add ./lean-ctx-addon.toml` bricht **ohne erreichbaren Registry-Index hart ab**, bevor
das `min_lean_ctx`-Gate greift. Das ist ein Upstream-Bug, kein Konfigurationsproblem — und es
bestimmt die Reihenfolge der Verifikation (§4).

### 1.6 lean-md-Seite

`feat-lmd-v2` @ `ec28841`. P3 code-complete: `content/skills/` (42 Dateien) löst über die
3-Stufen-Kaskade auf, 561 Tests grün, clippy clean, Whole-Branch-Review abgenommen.
`skill-fix` (SDD-Hardening) ist **unmerged**, konfliktfrei mergebar.
`lean-ctx-addon.toml` trägt `min_lean_ctx = "3.9.4"` und fünf `sha256 = "0000…"`.

---

## 2. Revidierte Gate-Tabelle

| Gate | Stand | Beleg |
|---|---|---|
| **V0** Paket C (Konsolidierung) | ✅ **geschlossen** (2026-07-11) | 6/6 Tasks abgenommen, `2a93810..0d0fd72`, Suite 562/562, Smoke Teil 1 grün |
| **V1a** lokales lean-ctx trägt den Vertrag | ✅ **geschlossen** | installierte `3.9.6` trägt den Vertrag |
| **V1b** Vertrag released | ✅ **geschlossen** | PR #780 gemergt, als `3.9.6` released |
| **V2** `v0.2.0` mit echten SHA-256 | ❌ offen | keine Tags; fünf `0000…`-Pins |
| **V3** curated Entry `listed` | ✅ **geschlossen** | PR #721 gemergt |
| **V4a** Skills-Pack publiziert | ❌ offen — **entkoppelt** | `pack create --kind skills` läuft lokal |
| **V4b** `addon publish` | ❌ offen | Netz/Token/Maintainer (§3, D7) |

**Der Kern:** V0 (2026-07-11) und V1a/V1b/V3 sind geschlossen. Verbleibend (V2, V4a, V4b) liegt in
unserer Hand; `pack publish`/`addon publish` sind client-seitig und laufen unter dem installierten
3.9.6-Binary.

---

## 3. Entscheidungen (verbindlich, 2026-07-10, Rev. 2)

**D1 — bleibt.** Die `.lmd.md`-Auto-Render-Delegation kommt aus lean-ctx heraus; `ctx_read`
liefert Roh-Bytes, Rendern ist explizit (`ctx_md_render` / CLI). Empirisch bestätigt (§1.4).
Folge: der Übergangsabsatz in `AGENTS.md`/`CLAUDE.md` wird **gelöscht**.

**D2 — bleibt, vollzogen.** #727 (Vertrag, PR #780) und #721 (Addon-Integration + Registry)
sind getrennt.

**D3 — ersetzt, dann fortgeschrieben.** Rev. 2 setzte `min_lean_ctx = "3.9.5"`, weil `3.9.6`
damals unreleased war. **Jetzt gilt `min_lean_ctx = "3.9.6"`** (Rev3-E4): released `3.9.5` trägt
den #727-Vertrag nicht, released `3.9.6` (= PR #780) schon; das lokale 3.9.6-Binary passiert den
Preflight mit Gleichstand. Der Wert dokumentiert den Vertrag; per D6 schützt das Gate ohnehin
nicht gegen ≤ die einführende Version.

**D4 — bleibt, verschärft.** Die Doku beschreibt jetzt schlicht die Gegenwart; die
befristete Ausnahme entfällt ersatzlos.

**D5 — bleibt, präzisiert.** Der Live-Smoke ist zweigeteilt, aber die Trennlinie verläuft
anders als in Rev. 1 angenommen — siehe §4.

**D6 — neu. `min_lean_ctx` schützt strukturell nicht gegen ≤ 3.9.5.** Das Preflight-Gate ist
selbst einer der #727-Fixes (`ed64d30a9`). Ein Binary ohne den Fix liest den Wert nie ein.
Das Gate kann daher **nie** gegen Versionen unterhalb seiner selbst schützen — der Wert wird
ausschließlich von Binaries ausgewertet, die den Vertrag ohnehin haben. Der reale Schutz gegen
den stillen Leerlauf ist der harte `PACK_MISSING` im Renderer. Rev. 1 schrieb dem Gate in
Risiko N1 eine Wirkung zu, die es nicht hat.

**D7 — neu. `addon publish` erfordert ein lokal installiertes #780-Binary.** `publish.rs`
reicht das `[[dependencies]]`-Array erst seit `2da5a7fb6` durch. Mit einer released `3.9.5`
entsteht ein `pack_manifest` mit leerem `dependencies` — ein still kaputter Install für jeden
Abnehmer. Die Stopp-Bedingung aus Rev. 1 §5.3 Schritt 4 bleibt, bekommt aber eine
**Vorbedingung** statt nur einer Nachprüfung.

---

## 4. Verifikationsstrategie

Rev. 1 teilte den Smoke entlang „lokal prüfbar vs. released". Das ist falsch. Die tatsächliche
Trennlinie ist **„braucht einen erreichbaren Registry-Index oder nicht"** — wegen §1.5: die
Consent-Vorschau hat keinen Offline-Zweig und beendet den Prozess.

### 4.1 Teil 1 — registry-frei (sofort nach dem Dev-Binary)

| Prüfung | Kommando | Erwartung |
|---|---|---|
| Pack materialisiert lokal | `lean-ctx pack create --kind skills --name @dasTholo/lean-md-skills --version 0.2.0 --from content/skills` | Store-Pfad `…/packages/skills/@dasTholo__lean-md-skills/0.2.0/`, 42 Dateien |
| Release-Binary rendert aus dem Pack | `LEAN_MD_SKILLS_DIR=<store> ./target/release/lean-md render --skill lmd-brainstorm --phase pre-context` | nicht-leerer Render |
| **Negativprobe** | `env -u LEAN_MD_SKILLS_DIR ./target/release/lean-md render …` | Exit ≠ 0, `PACK_MISSING …` |
| Debug-Fallback greift, Release-Fallback nicht | `cargo run -- render …` grün vs. Release ohne Env rot | `cfg(debug_assertions)` wirkt |
| Overlay schlägt Pack | Sentinel in `<jail>/.lean-ctx/lean-md/skills/<skill>/body.lmd.md` | Sentinel gewinnt |
| Assets + Exec-Bit | `lean-md skill install lmd-brainstorm --local` | 5 Scripts, `*.sh` mode `0755` |
| Roh-Read | `ctx_read(<datei>.lmd.md, mode=raw)` | unaufgelöste `@include` / `{{ }}` |

### 4.2 Teil 2 — registry-abhängig (erst nach V4a, oder gegen einen Mock)

| Prüfung | Erwartung |
|---|---|
| `addon add ./lean-ctx-addon.toml` | Consent-Vorschau **nennt** `@dasTholo/lean-md-skills` |
| `min_lean_ctx`-Gate | Preflight passiert bei Gleichstand `3.9.5` |
| `{pack_dir:}`-Expansion | `LEAN_MD_SKILLS_DIR` = absoluter Store-Pfad, kein Literal |
| Lockfile | pinnt Addon **und** Pack |
| Zweiter Install | offline reproduzierbar (`already_satisfied`-Fast-Path) |

Alternative ohne Publish: `--registry` / `CTXPKG_REGISTRY` auf einen Mock-Server richten.

---

## 5. Arbeitspakete

### 5.1 Paket C — Konsolidierung (lean-md, netzfrei) — ✅ **erledigt 2026-07-11**

**Ausgeführt & abgenommen** via `lmd-subagent-driven-development` (ein Implementer-Subagent pro
Task, Zwei-Verdikt-Review dazwischen, Whole-Branch-Final-Review clean — keine Findings). Branch
`feat-lmd-v2`, lokal gehalten (29 Commits vor `origin/feat-lmd-v2`, bewusst kein Push).

| SHA | Task | Inhalt |
|---|---|---|
| `3b82bfc` | AP1 | #1040-Warm-Cache-Korrektur über 4 Seeds + Test `no_body_or_fragment_claims_a_warm_subagent_cache` |
| `3c26b1c` | AP2 | SDD-Orientation-Block (`ctx_overview` + `ctx_repomap`), nur Orient-Hunk |
| `ff17fe5` | AP3 | `.claude/rules/subagent-multi-agent.md` gelöscht, SDD-Contract inline in `CLAUDE.md` |
| `0875341` | AP4 | `min_lean_ctx` 3.9.4 → **3.9.6** (+ E4-Kommentar) |
| `0d0fd72` | AP5 | Pack-Drift neu geblesst; `skills.ctxpkg-hash` = `8114591a52ebe4b3ff0dac54d067958d41c4396c66faa0651f536e448ba06b95` |

AP7 (Smoke Teil 1) = reine Verifikation, kein Commit. Suite **562/562** grün (+1 ggü. 561 =
AP1-Test), clippy clean, #498-Fragment-Consistency grün, `pack_drift` grün **ohne** `LEAN_MD_BLESS`
(content_hash vom Reviewer via frischem `pack create` unabhängig reproduziert — #498-deterministisch).

**Ursprünglicher Schnitt — maßgeblich ist `2026-07-11-lmd-paket-c-rev3-abandon-skill-fix-design.md`.**
`skill-fix` wurde **nicht** gemergt, sondern abandonniert: drei seiner fünf Commits tragen die von
PR #721 entfernte Auto-Render-Prämisse, einer invertiert die schon korrekte
`feat-lmd-v2`-Read-Semantik. Re-authored werden nur die validen Teile — die #1040-Warm-Cache-
Korrektur (+ Test), die SDD-Orientation, die superpowers-File-Aufräumung; `min_lean_ctx` →
`3.9.6`; Rebless; Gates; Smoke Teil 1. Der frühere „mergen + C2/C3 zurückrollen"-Weg (und
`paket-c-next-session.md`) entfällt — reiner Churn, siehe Rev3 §2.

> **Reihenfolge bindend:** 5.1 vor 5.4. Publizierte Pack-Versionen sind immutable (das
> Lockfile pinnt `artifact_sha256`); jede spätere Content-Änderung kostet `0.2.1` + Republish.

### 5.2 Dev-Binary (lean-ctx, lokal) — ~~schließt V1a~~ **erledigt**

Entfällt: das installierte `lean-ctx 3.9.6` trägt bereits Vertrag (PR #780) und Registry-Entry
(PR #721). V1a ist ohne Integrations-Build geschlossen. `lean-ctx --version` → `3.9.6`.

### 5.3 Lokaler Smoke, Teil 1 (§4.1) — ✅ **erledigt 2026-07-11**

Alle sieben Prüfzeilen grün (Release-Binary): Render aus Pack-Store; harter `PACK_MISSING` ohne
`LEAN_MD_SKILLS_DIR`; Overlay schlägt Pack; Debug-Fallback nur im Dev-Build (`cfg(debug_assertions)`);
Assets `*.sh` mode `0755`; `.lmd.md`-Roh-Read liefert unaufgelöste Direktiven. Der `0.2.0`-Pack war
der Verifikations-Pack, **nicht** der Publish (§5.4).

### 5.4 Pack publizieren — schließt V4a

`pack create` → `pack export --sign` → `pack publish pack.ctxpkg --token ctxp_…`.
Braucht Netz und Token, Maintainer-Hand. Vorbedingung: 5.1 abgeschlossen.

### 5.5 Lokaler Smoke, Teil 2 (§4.2)

### 5.6 Tag `v0.2.0` — schließt V2

Tag → 5-leg-Build → `sync-manifest` schreibt die echten SHA-256 in die fünf
`[artifacts]`-Blöcke. Ein lean-md-interner Zyklus; in lean-ctx sind **keine** SHAs zu setzen.

### 5.7 `addon publish --namespace dasTholo` — schließt V4b

**Vorbedingung (D7):** läuft unter dem #780-Binary aus 5.2.
**Stopp-Bedingung:** das publizierte `pack_manifest` muss `[[dependencies]]` tragen. Leeres
Array ⇒ falsches Binary ⇒ hier abbrechen.

### 5.8 Upstream-Merge — ~~schließt V1b und V3~~ **erledigt**

PR #780 gemergt und als `3.9.6` released; PR #721 gemergt. **V1b und V3 geschlossen.**

**Nachzuziehen in der Registry:** der curated Entry trägt `min_lean_ctx = "3.9.4"` (§1.3). Nach
D3 muss er auf `"3.9.6"` — eine Zeile in `rust/data/addon_registry.json`, sonst behauptet die
Registry einen anderen Vertrag als lean-mds Manifest.

**Was `listed` technisch heißt** (Korrektur an `lean-md-next-session.md`): Es gibt **kein**
`status`-Feld, und `listed` heißt **nicht** „`mcp`-Block entfernen" — `pub mcp: AddonMcp` ist
kein `Option`, alle 21 Entries tragen den Block. `is_installable()` =
`to_gateway_server().resolve().is_ok()` (`core/addons/manifest.rs:257`); ohne auflösbares
Kommando schlägt das fehl und der Entry gilt als listed. Die `letta`-Form: Block behalten,
`command`/`args` leeren. Genau das tut `03b6413ee`.

### 5.9 Task 7 — Voll-Smoke

Erst nach 5.8. `lean-md render docs/lean-md/plans/2026-07-09-lmd-p3-skills-pack-full-cut.lmd.md --phase task-7 --consumer=ai`

---

## 6. Risiken

| # | Risiko | Mitigation |
|---|---|---|
| **N1** | ~~`min_lean_ctx` lässt ein vertragsloses Binary durch~~ | **Ersetzt durch D6.** Das Gate *kann* das nicht abfangen — es existiert in solchen Binaries nicht. Schutz = harter `PACK_MISSING` im Renderer (existiert, greift). |
| **N2** | ~~Force-Retag `v3.9.4`~~ | **Entfällt.** `3.9.5` ist released; kein Retag. |
| **N3** | Doku-Rückdrehung nach dem Pack-Publish | Reihenfolge 5.1 vor 5.4 ist bindend; Strafe wäre `0.2.1` + Republish. |
| **N5** | `addon add`-Vorschau ohne Offline-Zweig, harter `exit(1)` (§1.5) | Akzeptiert (D5). Bestimmt die Smoke-Reihenfolge (§4). Upstream melden; auch #780 fixt es nicht. |
| **N6** | `already_satisfied` verifiziert `artifact_sha256` nicht gegen den Store (`deps.rs`) | Nicht lean-md-Scope. Upstream melden. |
| **N7** | **neu** — `addon publish` unter einem Nicht-#780-Binary erzeugt leeres `dependencies` | 5.2 ist Vorbedingung von 5.7; Stopp-Bedingung dort prüfen. |
| **N8** | **neu** — PR #780 wird nicht gemergt oder inhaltlich verändert | Der lean-md-Release (5.1–5.7) läuft davon unabhängig durch; nur der Nutzer-Install steht still. Kein lean-md-Code hängt an #780. |

---

## 7. Definition of Done

**Paket C (5.1): ✅ erfüllt (2026-07-11, `2a93810..0d0fd72`).** `skill-fix` abandonniert;
#1040-Korrektur (+ Test) + SDD-Orient + Cleanup re-authored; `feat-lmd-v2`-Read-Semantik unverändert
korrekt; `min_lean_ctx = "3.9.6"`; `skills.sha256` + `skills.ctxpkg-hash` neu geblesst, Drift-Gate
grün ohne `LEAN_MD_BLESS`; Suite 562/562 + clippy grün; Whole-Branch-Final-Review ohne Findings.

**Dev-Binary (5.2):** erledigt — `lean-ctx --version` = `3.9.6` (Vertrag + Registry-Entry bereits
installiert). V1a geschlossen.

**Smoke Teil 1 (4.1): ✅ erfüllt (2026-07-11).** Release-Binary rendert aus dem Pack-Store; ohne
`LEAN_MD_SKILLS_DIR` harter `PACK_MISSING`; Overlay schlägt Pack; Debug-Fallback nur im Dev-Build.

**Release (5.4–5.7):** V4a, V2, V4b geschlossen; das publizierte `pack_manifest` trägt
`[[dependencies]]`.

**Upstream (5.8–5.9):** V1b und V3 geschlossen; Task 7 durchlaufen.

**Nicht enthalten:** Skill-Tiering (P3-Design §8), P4 (Publisher-Identität/Signing), das
Verschieben von `content/lang` / `content/tooling` in den Pack, der Offline-Fix der
`addon add`-Vorschau (Upstream, §1.5), der `artifact_sha256`-Fix in `already_satisfied`
(Upstream, N6).

---
