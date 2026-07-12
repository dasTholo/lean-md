@lean-md
consumer: ai
crp: compact

@var test_cmd default="cargo nextest run" desc="project test runner command"
@var lint_cmd default="cargo clippy --all-targets -- -D warnings" desc="project lint gate"
@import .lean-ctx/lean-md/plan-recipes /

# lean-md V2-Tail — Publish-Vorlauf + bedingter Registry-Smoke — Implementation Plan

Spec: `docs/lean-md/specs/2026-07-11-lmd-v2-tail-publish-vorlauf-design.md` (Ansatz A).
Der Plan führt den **credential-freien** Schwanz §5.4–5.9 jetzt aus; die zwei echten
Hosted-Publishes + ihr Post-Smoke sind ein niedergeschriebener, **nicht** agent-auto-
ausgeführter Runbook-Tail (task-5), gegatet auf Token + Daemon-Rebuild + #721-Merge.

Rendert eine Task nach der anderen: `lean-md render <plan.lmd.md> --phase task-N`.

## Goal

Maximale Verifikation **vor** dem irreversiblen (immutablen) Hosted-Publish: den Skills-Pack
`@dasTholo/lean-md-skills 0.2.0` lokal materialisieren + auditieren, die render- und
installer-seitigen Invarianten credential-frei re-verifizieren, und den echten Publish als
gegateten Runbook niederschreiben.

## Architecture

- **Kein Code-Impact.** Diese Phase ändert weder `content/` noch `src/` → reine Registry-/
  Verifikations-Ops. Das Binary embedded via `include_str!` nur `content/{core,gloss,
  templates,lang,tooling}`; `content/skills/` lebt im **Pack** (E2-Nachtrag) → **kein neues Tag**.
- **Delivery-Kette (E4, harter Invariant):** der Deps-Resolver sieht nur den Registry-Index;
  ein nur-als-GH-Asset existierender Pack ist unsichtbar. `pack publish @dasTholo/lean-md-skills`
  ist Voraussetzung, damit die Skills via verbatim weitergereichte `[[dependencies]]` +
  `{pack_dir:}`-Expansion beim `addon add` laufen — orthogonal zur listed/installable-Frage.
- **Verifikations-Ziele (lean-ctx-Quellcode, spec §2):** `artifact_install.rs`
  (`fetch_verified`/`ensure_addon_binary`, mandatory sha256-Pin), `manifest.rs`
  (`[[dependencies]]`→`pack_env::expand_pack_env`), `registry.rs` (`validate_entries`,
  `flagship_lean_md_is_listed`). Anker leben in der Spec — hier nicht dupliziert (output_rule #2).

## Global Constraints

- **Non-Goal:** keine `content/`- oder `src/`-Änderung diese Phase → kein `pack`-Bump-Zwang,
  **kein neues Git-Tag** (E2-Nachtrag). Ein Diff, der `content/skills/` oder `src/` berührt, ist
  ein Review-Stopp.
- **Non-Goal:** echte Hosted-Publishes (`pack publish` / `addon publish`), Hosted-Re-Smoke,
  #721-Merge, `installable`-Flip des kuratierten Entrys (E3), Skill-Tiering, P4-Signing —
  **alle** credential-/daemon-gegatet, ausschließlich als task-5-Runbook, **nie** agent-auto-run.
- **#498 Determinismus:** der lokale Pack trägt ctxpkg-hash `6491dc4e`; Render-Smokes sind
  byte-stabil (zweiter Render == erster) — Test-Gate.
- **Immutability (E2/R3):** vor jedem Publish muss verifiziert sein, dass die Registry
  `@dasTholo/lean-md-skills 0.2.0` **nicht** kennt. Erste Runbook-Vorbedingung.
- **E4-Stop (D7):** das publizierte `pack_manifest` muss nicht-leeres `[[dependencies]]` tragen;
  leeres Array ⇒ falsches Binary ⇒ Runbook abbrechen.
- **Task-Abhängigkeit:** task-0 Ergebnisse gaten downstream — 0.4 (Signierschlüssel) bestimmt
  task-1 Export-Modus (`--sign` vs. unsigniert); 0.5 (Mock-Feasibility) entscheidet, ob task-4
  jetzt läuft oder in den Runbook wandert.

@phase "task-0"
## Task 0: Pre-Flight / origin-reconcile (bindende Vorbedingung)

**Files:** keine Änderung — nur Probes + persistierte Entscheidungen. Jede Probe mit explizitem Expected.

**0.1 — Ungepushten `43fb487` klären.** Er ändert nur `.pre-commit-config.yaml` (+6 Zeilen),
berührt weder `content/` noch `src/` → kein Pack-/Binary-Impact. Reine Branch-Hygiene.

    git show --stat 43fb487

Expected: `1 file changed` — nur `.pre-commit-config.yaml`. Entscheidung push **oder** bewusst
lokal halten festhalten (kein Blocker für die übrigen Tasks).

**0.2 — SHA-Kreuzprobe.** Die fünf `[artifacts].sha256` in `lean-ctx-addon.toml` == GH-Release
`SHA256SUMS`, byte-genau.

    lean-ctx -c "grep -A1 '\[\[artifacts\]\]' lean-ctx-addon.toml"
    gh release view v0.2.0 --json assets -q '.assets[].name'

Expected: die fünf Manifest-SHAs (`af5642…`,`3a3b0e…`,`9e3800…`,`365dee…`,`1b092f…`) stimmen
mit den Release-`SHA256SUMS` überein. Mismatch = harter Stopp.

**0.3 — Immutability-Check (lokal).** Bestätigen, dass nie ein `0.2.0`-Pack publiziert wurde.

    lean-ctx -c "git log --oneline --all | grep -i 'pack publish' || echo NONE"

Expected: `NONE` — lokal lief kein `pack publish` (der echte Registry-Check ist erste
Runbook-Vorbedingung, task-5.1).

**0.4 — Signierschlüssel-Präsenz.** Bestimmt, ob task-1 `pack export --sign` signiert oder
unsigniert für die Mock-Smoke exportiert (Signatur wandert sonst in den Runbook, R2).

    lean-ctx -c "ls ~/.config/lean-ctx/keys 2>/dev/null || echo NO_KEY"

Expected: ed25519-Schlüssel vorhanden → task-1 mit `--sign`; `NO_KEY` → task-1 unsigniert.

**0.5 — Mock-Feasibility.** Akzeptiert `lean-ctx` einen lokalen Registry-Index für die
Dep-Resolution (`--registry`/`CTXPKG_REGISTRY`)? Entscheidet, ob task-4 jetzt läuft oder in den
Runbook wandert (R1).

    lean-ctx addon add --help

Expected: ein `--registry`/`--index`-Flag (oder respektiertes `CTXPKG_REGISTRY`-Env) existiert →
Mock feasibel, task-4 läuft; sonst → task-4 komplett in Runbook (task-5), kein Fake-Smoke.

### Verify & Close

@call remember_decision("v2-tail task-0: 43fb487=<push|hold>; SHA-crosscheck=<pass|fail>; immutability 0.2.0=frei(lokal); signing=<sign|unsigned>; mock-index=<feasible|runbook>")
@phase-end

@phase "task-1"
## Task 1: V4a-Prep + Audit — Pack materialisieren + Publish-Gate offline

**Files:** liest `content/skills/` + `lean-ctx-addon.toml`; schreibt nur in den lokalen
Pack-Store (kein Repo-Diff). Export-Modus aus task-0.4.

@call recall_context("v2-tail task-0: signing decision + immutability")

**1.1 — Pack erzeugen.** Deterministisch (#498).

    lean-ctx pack create --kind skills --name @dasTholo/lean-md-skills --version 0.2.0 --from content/skills

Expected: lokaler Store, **42 Dateien**, ctxpkg-hash **`6491dc4e`**. Abweichender Hash =
Determinismus-Regression → Stopp.

**1.2 — Pack exportieren.** `--sign` nur falls task-0.4 einen Schlüssel fand; sonst unsigniert.

    lean-ctx pack export --sign        # signed-Zweig
    lean-ctx pack export               # unsigned-Zweig (kein Schlüssel, R2 → Signatur in Runbook)

Expected: `.ctxpkg`-Artefakt materialisiert; im unsigned-Zweig Notiz „Signatur pending Runbook".

**1.3 — Addon-Audit (Publish-Gate, netzfrei).** Löst keine Deps auf.

    lean-ctx addon audit ./lean-ctx-addon.toml

Expected: `pass` oder `review` (wiring-risk + capability-coherence + malware). `exit≠0` = Stopp,
kein Weitergehen.

### Verify & Close

@call remember_decision("v2-tail task-1: pack 0.2.0 hash=6491dc4e (42 files), export=<signed|unsigned>, audit=<pass|review>")
@phase-end

@phase "task-2"
## Task 2: Smoke Teil 1 re-verify (Rev2 §4.1, credential-frei)

**Files:** keine Änderung. Verifiziert das **Release**-Binary gegen den Pack-Store aus task-1.
Jede Assertion eigene Invocation.

**2.1 — Release-Binary bauen.**

    cargo build --release --bin lean-md

Expected: `target/release/lean-md` existiert.

**2.2 — Harter `PACK_MISSING` ohne Pack-Dir.**

    lean-ctx -c "env -u LEAN_MD_SKILLS_DIR target/release/lean-md render --skill lmd-writing-plans --phase pre-context"

Expected: `exit≠0`, Fehler `PACK_MISSING` (Release-Build hat **keinen** Debug-Fallback).

**2.3 — Render aus dem Pack-Store + Overlay schlägt Pack.**

    lean-ctx -c "LEAN_MD_SKILLS_DIR=<pack-store> target/release/lean-md render --skill lmd-writing-plans --phase pre-context"

Expected: nicht-leerer Render aus dem Pack; ein Overlay-Skill (falls gesetzt) gewinnt gegen die
Pack-Version.

**2.4 — Asset-Bits + `.lmd.md`-Roh-Read.**

    lean-ctx -c "find content/skills -name '*.sh' -exec stat -c '%a %n' {} +"
    target/release/lean-md source content/skills/lmd-executing-plans/SKILL.md

Expected: alle `*.sh` mode `0755`; der Roh-Read liefert **unaufgelöste** `@`-Direktiven (kein
Render).

### Verify & Close

@call remember_decision("v2-tail task-2: smoke Teil 1 grün — PACK_MISSING hart, overlay>pack, *.sh=0755, raw-read unresolved")
@phase-end

@phase "task-3"
## Task 3: Render-Voll-Smoke (Rev2 §5.9-Renderseite, credential-frei)

**Files:** keine Änderung. Rendert eine echte Plan-Phase gegen das Release-Binary; prüft
non-empty + Byte-Stabilität (#498).

    lean-md render docs/lean-md/plans/2026-07-09-lmd-p3-skills-pack-full-cut.lmd.md --phase task-7 --consumer=ai

Expected: nicht-leerer Task-7-Block; ein zweiter identischer Render liefert **byte-gleiche**
Ausgabe (Determinismus).

@call render_check("docs/lean-md/plans/2026-07-09-lmd-p3-skills-pack-full-cut.lmd.md", "task-7")

### Verify & Close

@call remember_decision("v2-tail task-3: render-voll-smoke grün — task-7 non-empty + byte-stabil")
@phase-end

@phase "task-4"
## Task 4: Smoke Teil 2 — bedingt (Rev2 §4.2, credential-frei via Mock-Index)

**Gate:** läuft **nur**, wenn task-0.5 `mock-index=feasible` ergab. Sonst → dieser gesamte Task
wandert unverändert in task-5 (Runbook), **kein Fake-Smoke**.

@call recall_context("v2-tail task-0: mock-index feasibility + immutability")

**Setup.** Der Mock-Index trägt **nur** `@dasTholo/lean-md-skills 0.2.0`; der Binär kommt aus
dem **public GH-Release** (token-frei).

    lean-ctx addon add ./lean-ctx-addon.toml --registry <mock-index>

Expected — der volle Chain end-to-end:
- consent-preview nennt den Pack `@dasTholo/lean-md-skills`;
- `min_lean_ctx`-Gate akzeptiert bei Gleichstand (`3.9.6`);
- `ensure_addon_binary` zieht das Linux-Triple von der public URL + matcht sha256 `af5642…`
  (Beweis, dass lean-ctx' Installer die released Hashes akzeptiert);
- `{pack_dir:}` expandiert zu einem **absoluten** Store-Pfad (kein Literal);
- Lockfile pinnt **Addon und Pack**.

**Fallback (kein Mock möglich):** diesen Block wörtlich in task-5.3 übernehmen, hier als
„deferred → Runbook" markieren.

### Verify & Close

@call remember_decision("v2-tail task-4: smoke Teil 2 = <grün-gegen-mock | deferred-runbook>")
@phase-end

@phase "task-5"
## Task 5: Runbook-Tail niederschreiben (KEIN Agent-Auto-Run)

**Files:** hängt den gegateten Runbook an diese Plan-Datei/`ctx_knowledge` an. **Kein Kommando
dieses Tasks wird vom Agenten ausgeführt** — Gate: gültige Publish-Credentials **und**
Daemon-Rebuild aus #721 **und** #721 gemergt. Der Runbook wird nur **verifiziert vollständig
niedergeschrieben**, nicht getriggert.

Niederzuschreibender Tail (verbatim, in Reihenfolge):

**5.1 — Pack publish (schließt V4a).** Vorbedingung: Registry kennt kein `0.2.0` (echter
Immutability-Check, R3).

    lean-ctx pack publish pack.ctxpkg --token ctxp_…

Expected: Registry akzeptiert `@dasTholo/lean-md-skills 0.2.0`. Kollision ⇒ abbrechen (immutable).

**5.2 — Addon publish (schließt V4b).**

    lean-ctx addon publish --namespace dasTholo

**Stopp-Bedingung (E4/D7):** das publizierte `pack_manifest` trägt nicht-leeres
`[[dependencies]]`; leeres Array ⇒ falsches Binary ⇒ **abbrechen**.

**5.3 — Hosted-Re-Smoke + Integrity-Lock.** Smoke Teil 2 (bzw. der task-4-Fallback-Block) gegen
die **echte** Registry.

    lean-ctx addon verify

Expected: Integrity-Lock grün; der volle Chain aus task-4 reproduziert gegen die live Registry.

**5.4 — §5.8 abschließen.** PR #721 gemergt bestätigen; `min_lean_ctx` im kuratierten Entry auf
`3.9.6` (Entry bleibt `listed`, E3 — kein `installable`-Flip).

### Verify & Close

@call remember_decision("v2-tail task-5: Runbook-Tail vollständig niedergeschrieben + gegatet (token+daemon+#721); NICHT ausgeführt")
@phase-end

## Gated Publish Runbook — V2-Tail (resolved & corrected · NICHT agent-auto-run)

> Niedergeschrieben von task-5 (2026-07-12). **Kein Kommando hier wird automatisch ausgeführt.**
> **Gate (ALLE müssen halten, bevor irgendein Kommando läuft):** valide Publish-Credentials
> (`ctxp_…`-Token) **und** Daemon-Rebuild aus PR #721 **und** #721 gemergt.

### Vorbedingungen (verifizieren VOR 5.1)

- **P0 — Pack-Drift-Status verifizieren (bereits aufgelöst, KEIN offener Blocker).** Frühere Sessions
  notierten eine committete Drift (`content/skills.ctxpkg-hash` `8114591a…` vs. real `6491dc4e…`,
  eingeführt von `d64e1fd`). **Diese Drift ist bereits behoben:** Commit `4c3cbce` („rebless pack
  hashes …", Ancestor von BASE `78194e3`) zog `content/skills.{ctxpkg-hash,sha256}` nach — der
  checked-in `ctxpkg-hash` == `6491dc4e…` == der real berechnete Hash (bei BASE **und** HEAD
  identisch), CI „Pack Drift" grün. **Vor 5.1 nur verifizieren:** `git show HEAD:content/skills.ctxpkg-hash`
  == `6491dc4e…`. **Kein Rebless, kein Version-Bump** — die publizierten Bytes bleiben `6491dc4e`
  bei fixer Version `0.2.0` (E2). Nur falls wider Erwarten ein Mismatch auftritt: erst dann rebless
  + Version-Bump erwägen.
- **P1 — Immutability echt prüfen (R3).** Bestätigen, dass die **Registry** `@dasTholo/lean-md-skills
  0.2.0` **nicht** kennt (task-0.3 `NONE` deckt nur lokal ab). Kollision ⇒ abbrechen (immutable).
- **P2 — Signing.** Ein ed25519-Key **ist** verfügbar (task-0.4 `NO_KEY` war ein False-Negative:
  `ls`/`find` allowlist-gejailt; `pack create` meldete `Signed: ed25519`). Daher **mit `--sign`**
  exportieren/publishen.

### 5.1 — Pack publish (schließt V4a)

    lean-ctx pack create --kind skills --name @dasTholo/lean-md-skills --version 0.2.0 --from content/skills --description "lean-md rendering skills"
    lean-ctx pack export --sign --output=<pfad>/pack.ctxpkg
    lean-ctx pack publish <pfad>/pack.ctxpkg --token ctxp_…

Expected: Registry akzeptiert `@dasTholo/lean-md-skills 0.2.0`. Kollision ⇒ abbrechen.
Korrektur ggü. Ur-Plan: `pack create` braucht `--description`; `pack export` braucht `--output=`
(+ `--sign` aus P2). Verifiziert in task-1.

### 5.2 — Addon publish (schließt V4b)

    lean-ctx addon publish --namespace dasTholo

**Stopp (E4/D7):** das publizierte **Addon**-`pack_manifest` muss nicht-leeres `[[dependencies]]`
tragen; leeres Array ⇒ falsches Binary ⇒ **abbrechen**. (Hinweis: der Skills-**Pack** selbst trägt
korrekt `dependencies: []` — die E4-Prüfung gilt dem Addon-Manifest, nicht dem Pack.)

### 5.3 — Hosted-Re-Smoke + Integrity-Lock (enthält den deferred task-4-Block)

    lean-ctx addon add @dasTholo/lean-md        # bzw. ./lean-ctx-addon.toml gegen die LIVE Registry
    lean-ctx addon verify

Expected — voller Chain aus task-4, jetzt gegen die **echte** Registry:
- consent-preview nennt den Pack `@dasTholo/lean-md-skills`;
- `min_lean_ctx`-Gate akzeptiert bei Gleichstand `3.9.6` (installiert `3.9.7` ≥);
- `ensure_addon_binary` zieht das Linux-Triple von der public URL + matcht sha256 `af5642…`;
- `{pack_dir:}` expandiert zu einem **absoluten** Store-Pfad (kein Literal) — **damit ist die
  task-2/2.3-Materialisierungslücke end-to-end geschlossen:** `LEAN_MD_SKILLS_DIR={pack_dir}`
  zeigt auf den **materialisierten** Skills-Baum (nicht den verpackten Store) → non-empty Render;
- Lockfile pinnt **Addon UND Pack**;
- Integrity-Lock grün.

### 5.4 — §5.8 abschließen

PR #721 gemergt bestätigen; `min_lean_ctx` im kuratierten Registry-Entry auf `3.9.6` setzen
(Entry bleibt `listed`, E3 — **kein** `installable`-Flip).
