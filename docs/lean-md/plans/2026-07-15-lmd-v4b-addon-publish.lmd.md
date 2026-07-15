@lean-md
consumer: ai
crp: compact

@var test_cmd default="cargo nextest run" desc="project test runner command"
@var lint_cmd default="cargo clippy --all-targets -- -D warnings" desc="project lint gate"
@import .lean-ctx/lean-md/plan-recipes /

# lean-md Addon-Publish V4b — Implementation Plan

Quelle: `docs/lean-md/specs/2026-07-15-lmd-v4b-addon-publish-design.md` (Ansatz A).
Rendert eine Task pro Aufruf: `lean-md render <plan.lmd.md> --phase task-N`.

## Goal

Der letzte Delivery-Schritt V4b: den hosted Publish des `kind=addon`-Packs
`@dastholo/lean-md@0.2.0` nach ctxpkg.com abschließen. Alles credential-frei
Verifizierbare läuft **jetzt** (Task 1, agent-ausführbar); der eine irreversible
Hosted-Publish + Post-Smoke ist ein niedergeschriebener, **nicht** agent-auto-
ausgeführter Runbook-Tail (Task 2), gegatet auf die bewusste Maintainer-Auslösung
(Token `ctxp_…`, Scope `dastholo`, liegt vor).

## Architecture

Delivery-Grenze: **lokales Binary + Token → ctxpkg.com**. Kein Merge, kein
Netzwerk außer der Registry. Das installierte `lean-ctx 3.9.9 (official)` (build
`d4968f2`) trägt die #727-Maschinerie (D7-Forwarding `2da5a7fb6`, `{pack_dir:}`-
Expander `97ec2c569`, `min_lean_ctx`-Enforcement `ed64d30a9`) — per git-Ancestry
gegen released Tag `v3.9.6` (`372d91c`) bewiesen (Spec §2). Authoring-Manifest:
`lean-ctx-addon.toml` (`[artifacts]`×5 echte SHAs, `[[dependencies]] @dastholo/
lean-md-skills ^0.2`, `[mcp.env] LEAN_MD_SKILLS_DIR="{pack_dir:@dastholo/lean-md-
skills}"`, `min_lean_ctx=3.9.6`). Der Skills-Pack `@dastholo/lean-md-skills@0.2.0`
ist bereits hosted (V4a, immutable).

## Global Constraints

- **E7 (harte, nutzer-mandatierte Invariante):** `main` trägt **niemals**
  `docs/lean-md/`. Diese Phase führt **keinen** `main`-Merge aus — hält die
  Invariante aber als bindende Randbedingung für jeden Folge-Merge fest
  (`git rm -r --cached docs/lean-md/` lebt in `2026-07-12-lmd-docs-refresh` task-4).
- **Immutability:** der Hosted-Publish ist irreversibel; `0.2.0`-Kollision ⇒
  abbrechen (kein Retract). Recovery: Version-Bump `0.2.1` + republish.
- **Namespace `dastholo` (klein)** überall im Delivery-Pfad + `--namespace dastholo`.
  GitHub-Handle `dasTholo` bleibt in URLs/author/LICENSE bewusst unangetastet.
- **E6 — Publish ⟂ Merge:** braucht `pr/lean-md-addon-v2 → origin/main` **nicht**;
  `origin/main` trägt die Maschinerie bereits.
- **Task 2 ist NICHT agent-auto-ausführbar.** Gate = bewusste Maintainer-Auslösung.
  Ein Agent verifiziert und schreibt nieder — er triggert `addon publish` nie selbst.
- **Non-Goals:** `installable`-Flip (E5), der Merge (E6), der clean-`main`-Branch
  ohne `docs/lean-md/` (eigener Plan), P4 (Signing/Publisher-Identität).

@phase "task-1"
## Task 1: Credential-freier Pre-Flight (agent-ausführbar)

Reine Re-Bestätigung; keine offene Entscheidung. **Keine Datei-Änderung, kein
Commit** — nur Verifikation + Persistenz der Verdikte via `remember_decision`.
Anker: Spec §4 Task 1, §1–§2. Root: Repo-Wurzel (`lean-ctx-addon.toml`, `src/`,
`Cargo.toml` liegen dort).

@call recall_context("V4a: @dastholo/lean-md-skills@0.2.0 hosted, content-hash 6491dc4e, artifact-sha 5b77377c, Namespace-Reconcile @dasTholo->@dastholo committet 14f0d6b")

### 1.1 — Namespace-Konsistenz

    lean-ctx -c "grep -nE '(dependencies|pack_dir)' lean-ctx-addon.toml"

**Expected:** `[[dependencies]]`-Block → `@dastholo/lean-md-skills` version `^0.2`;
`[mcp.env]` → `LEAN_MD_SKILLS_DIR="{pack_dir:@dastholo/lean-md-skills}"`. Beide
Stellen tragen `dastholo` (klein). Ein `dasTholo` in einer der beiden Stellen ⇒ fail.

### 1.2 — Immutability (lokale Vorab-Bestätigung)

Der **echte** Registry-Check ist erste Runbook-Vorbedingung (R2, Task 2.1) und
braucht den Live-Index; hier nur die credential-freie Vorab-Bestätigung, dass
`@dastholo/lean-md` (das Addon, **nicht** der Skills-Pack) nie publiziert wurde —
aus V4a-Wissen/Ancestry (kein Registry-Record). **Expected:** kein bekannter
Publish-Record für `@dastholo/lean-md@0.2.0`; die Immutability-Freiheit wird in
2.1/R2 live abgesichert.

### 1.3 — SHA-Kreuzprobe

    lean-ctx -c "grep -nE 'sha256' lean-ctx-addon.toml"
    gh release download v0.2.0 --repo dasTholo/lean-md --pattern SHA256SUMS -O -

**Expected:** die fünf `[artifacts].sha256` (`af5642…`, `3a3b0e…`, `9e3800…`,
`365dee…`, `1b092f…`) == die fünf Einträge in GH-Release `v0.2.0` `SHA256SUMS`,
**byte-genau**. Jede Abweichung ⇒ fail (Release-Artefakt ≠ Manifest).
Soft-Vorbedingung: `gh` ist authentifiziert (`gh auth status`); der Asset-Download
ist ein public Release-Fetch (Netzwerk).

### 1.4 — `addon publish --check`

    lean-ctx addon publish ./lean-ctx-addon.toml --check --namespace dastholo

**Expected:** baut `@dastholo/lean-md@0.2.0` (kind=addon, 5492 B), Audit **`pass`**,
5 Triples (`aarch64-linux`, `x86_64-linux`, `aarch64-darwin`, `x86_64-darwin`,
`x86_64-windows-msvc`), `LEAN_MD_SKILLS_DIR`-child_env erkannt, **nichts
hochgeladen**. Ancestry-Notiz (§2) als D7-Beleg festhalten (unten `remember`).

### 1.5 — Skills-Pack-Präsenz

    lean-ctx addon add dastholo/lean-md-skills --check

**Expected:** die Registry löst `@dastholo/lean-md-skills@0.2.0` (hosted seit V4a);
consent-preview nennt den hosted Skills-Pack. Harter Invariant — der Deps-Resolver
sieht **nur** den Registry-Index; Abwesenheit ⇒ harter Stop (Task 2 nicht auslösen).

### Verify & Close

@call remember_decision("V4b Task-1 Pre-Flight grün: Namespace dastholo konsistent (deps + pack_dir), 5 [artifacts].sha256 == GH v0.2.0 SHA256SUMS byte-genau, `addon publish --check` pass mit 5 Triples + child_env, Skills-Pack @dastholo/lean-md-skills@0.2.0 hosted. D7-Beleg: publish-Binary d4968f2 enthaelt 2da5a7fb6 (Ancestry v3.9.6/372d91c) -> [[dependencies]] wird durchgereicht. Task 2 (Runbook) ist maintainer-gegatet, kein Auto-Run.")

@phase-end

@phase "task-2"
## Task 2: Gegateter Runbook-Tail (KEIN Agent-Auto-Run)

> **GATE — NICHT AUTOMATISCH AUSFÜHREN.** Dieser Block ist ein verbatim
> niedergeschriebener Runbook. Auslösung nur durch die bewusste Maintainer-Hand
> (Token `ctxp_…`, Scope `dastholo`, liegt vor). Ein Agent führt `addon publish`
> **nie** selbst aus — er liefert diesen Text und stoppt.

In Reihenfolge; jeder Schritt gegen die echte Registry (ctxpkg.com).

### 2.1 — Addon publish (schließt V4b)

**Vorbedingung R2** — Registry kennt kein `0.2.0`:

    lean-ctx addon add dastholo/lean-md --check

Expected: die Registry löst `@dastholo/lean-md@0.2.0` **nicht** auf (nie
publiziert). Löst sie auf ⇒ **abbrechen** (Kollision, immutable).

Dann publish:

    lean-ctx addon publish ./lean-ctx-addon.toml --namespace dastholo --token ctxp_…

Expected: Registry akzeptiert `@dastholo/lean-md 0.2.0`. Kollision ⇒ abbrechen (immutable).

### 2.2 — Post-Publish D7-Assert

Am publizierten `pack_manifest` prüfen, dass `dependencies` **nicht-leer** ist:

    lean-ctx addon add dastholo/lean-md --check

Expected: consent-preview listet `[[dependencies]] @dastholo/lean-md-skills ^0.2`
(nicht-leer). Per Ancestry erwartet ✓. Leeres Array ⇒ falsches Binary ⇒
**abbrechen**. Recovery (immutable, kein Retract): Version-Bump `0.2.1` + republish.

### 2.3 — Hosted-Re-Smoke

    lean-ctx addon add dastholo/lean-md

Expected: voller Chain reproduziert — consent-preview nennt den Skills-Pack;
`min_lean_ctx`-Gate bei ≥3.9.6; `ensure_addon_binary` zieht+matcht das Linux-Triple
`af5642…` gegen die public URL; `{pack_dir:}`-Expansion (absoluter Store-Pfad);
Lockfile pinnt Addon **und** Pack. (R6: der `addon add`-Vorschau fehlt upstream ein
Offline-Zweig — akzeptiert; bestimmt nur die Smoke-Reihenfolge, irrelevant für den
token-getriebenen Live-Publish.)

### 2.4 — Integrity-Lock

    lean-ctx addon verify

Expected: Integrity-Lock grün.

### Definition of Done (V4b)

Registry akzeptiert `@dastholo/lean-md 0.2.0`; Post-Publish-`dependencies`
nicht-leer; Hosted-Re-Smoke reproduziert den vollen Chain; `addon verify` grün.

@phase-end
