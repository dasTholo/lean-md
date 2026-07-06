# Design-Spec: lmd-finishing-a-development-branch

**Datum:** 2026-07-06
**Status:** genehmigt (Brainstorm), bereit für Planung
**Vorlage:** `superpowers/6.1.1/skills/finishing-a-development-branch`

## Ziel

Den superpowers-Skill `finishing-a-development-branch` als nativen lmd-Skill
`lmd-finishing-a-development-branch` portieren — **vollständiger Ersatz ohne
Funktionsverlust**. Der Port nutzt lean-md-Funktionen (`@phase`-Isolation,
`@define`/`@call`-Makros, `@query`, `@include`) und wird von allen lmd-Skills
statt der externen superpowers-Referenz verwendet.

## Nicht-Ziele (YAGNI)

- Kein Subagent-Dispatch (`@dispatch`) — der Skill ist rein interaktiv (User wählt
  Integrations-Option); kein Reviewer-Subagent.
- Keine Companions — das Original hat keinen Reviewer, der Port auch nicht.
- Kein Ausdünnen des Worktree-Verhaltens — voll erhalten (Skill ist global für
  alle Repos gedacht).

## Architektur

Rein interaktiver Inline-Skill (main agent, kein Subagent), render-on-demand über
den lean-ctx-Engine. Terminaler Zustand nach Options-Ausführung; kein Folge-Skill.

### Phasen-Layout (`next:`-verzweigt)

| Phase | Zweck | lean-md-Funktionen |
|---|---|---|
| `pre-context` | Announce + Ambient-Baseline (Inline-Ausführung) | `@include hard-rules` |
| `verify-tests` | Testlauf; Fehlschlag → **STOP**, keine Optionen | `@call gate(<paths>)` — **projekt-agnostisch** via `test_cmd`/`lint_cmd` (`vars.toml`), erhält npm/cargo/pytest/go-Fidelity des Originals |
| `detect-env` | GIT_DIR vs GIT_COMMON, base-branch bestimmen | `@call detect_env()`, `@query "git merge-base HEAD main"` |
| `present-options` | 4 Optionen (normal/named-worktree) **oder** 3 (detached HEAD) | verzweigt via `next:` je nach Wahl |
| `merge-local` | Opt 1: checkout base → pull → merge → **re-verify** → cleanup → `branch -d` | `@query`, `@call gate()`, `@call cleanup_worktree()` |
| `create-pr` | Opt 2: `git push -u`, **kein** cleanup (User iteriert am PR) | `@query` |
| `keep-as-is` | Opt 3: Report, **kein** cleanup | `@query` |
| `discard` | Opt 4: **typed "discard"-Confirm** → cleanup → `branch -D` | `@call cleanup_worktree()` |

Die Verzweigung nach `present-options` folgt dem `next:`-Muster aus
`lmd-executing-plans`: die gewählte Option wird als isolierte Phase gerendert; die
drei nicht-gewählten kosten nie Kontext (Token-Hebel).

### Makros (`@define` im Meta-Head bzw. `_includes/`-Fragment, DRY)

- **`detect_env()`** — ermittelt `GIT_DIR` / `GIT_COMMON` / `WORKTREE_PATH` und
  wählt das Menü:
  - `GIT_DIR == GIT_COMMON` (normales Repo) → 4 Optionen, kein Worktree-Cleanup.
  - `GIT_DIR != GIT_COMMON`, named branch → 4 Optionen, Provenance-Cleanup.
  - `GIT_DIR != GIT_COMMON`, detached HEAD → 3 Optionen (kein merge), kein Cleanup.
- **`cleanup_worktree()`** — Provenance-Check (`.worktrees/` / `worktrees/`),
  `cd` main-root **zuerst**, dann `git worktree remove` + `git worktree prune`.
  Läuft **nur** für `merge-local` + `discard`. Fremd-/harness-eigene Worktrees
  werden nicht entfernt.

## Datenfluss / Kontrollfluss

1. `pre-context`: Baseline setzen, Announce.
2. `verify-tests`: Tests grün? Nein → STOP (Ausgabe der Fehler, kein Menü). Ja → weiter.
3. `detect-env`: Environment + base-branch → bestimmt Menü-Variante.
4. `present-options`: exakt 4 (bzw. 3) strukturierte Optionen, keine Erklärung.
5. User wählt → isolierte Option-Phase wird gerendert und ausgeführt.
6. Terminal: Schlusszustand via `ctx_session action=status`.

## Fehlerbehandlung / Guards (Inline statt dupliziertem Block)

Die „Common Mistakes" + „Red Flags" des Originals werden als knappe Inline-Guards
in die betreffenden Phasen eingebettet:

- `verify-tests`: niemals mit fehlschlagenden Tests fortfahren.
- `merge-local`: merge zuerst → Tests auf Ergebnis re-verifizieren → **dann**
  worktree remove → **dann** `branch -d` (Reihenfolge zwingend, sonst schlägt
  `branch -d` fehl, weil der Worktree den Branch referenziert).
- `cleanup_worktree()`: `cd` main-root **vor** `git worktree remove` (sonst
  stiller Fehlschlag aus dem Worktree heraus); nur `.worktrees/`/`worktrees/`
  entfernen (Provenance).
- `discard`: getippte `discard`-Bestätigung zwingend; kein force-push ohne
  expliziten Wunsch.

## Rewiring (erfüllt „in allen Skills verwenden")

Die zwei bestehenden Verweise auf die externe superpowers-Referenz werden auf den
neuen Skill umgestellt (Verweistext, kein `@call` — separater Skill):

- `content/skills/lmd-executing-plans/body.lmd.md:121` (`finish`-Phase).
- `content/skills/lmd-subagent-driven-development/body.lmd.md:131-132`
  (`finish`-Phase).

Beide: „until an lmd port exists, follow the external finishing-a-development-branch
reference" → **invoke `lmd-finishing-a-development-branch` skill**.

## Registrierung (5 Stellen, analog `lmd-executing-plans`)

1. `content/skills/lmd-finishing-a-development-branch/{SKILL.md, body.lmd.md}` —
   Delegations-Stub + Body.
2. `src/skills.rs` — `include_str!` Body → `SKILLS`-Registry + Render-Test (alle
   Phasen non-empty; `pre-context` trägt die hard-rules-Baseline).
3. `src/skill_install.rs` — `include_str!` SKILL.md → `INSTALLABLE_SKILLS` +
   Install-Test.
4. `src/availability.rs` — `COVERAGE`-Rows (Phase→Tool-Mapping) + Coverage-Test.
5. `cargo fmt` je geänderte Datei vor `git add`; `cargo nextest run` grün, zero
   clippy warnings.

## Fidelity-Matrix (Original → Port, jede Einheit abgedeckt)

| Original | Port |
|---|---|
| Step 1 Verify Tests + „Skipping test verification" | `verify-tests` + STOP-Guard |
| Step 2 Detect Environment (3 States) | `detect-env` / `detect_env()` |
| Step 3 Determine Base Branch | `detect-env` (`@query merge-base`) |
| Step 4 Present Options (4 vs 3) | `present-options` |
| Step 5 Execute (Opt 1-4) | 4 isolierte Option-Phasen |
| Step 6 Cleanup Workspace (Provenance) | `cleanup_worktree()` Makro |
| Quick-Reference-Matrix | Inline-Tabelle in `present-options` |
| Common Mistakes / Red Flags | Inline-Guards je Phase |

## Determinismus (#498)

Body als `include_str!`-Seed byte-identisch (built-in == on-disk). Keine
Timestamps/Counter im Render-Output. Fragment-Consistency-Gate muss grün bleiben.

## Bewusste Planungs-Deferrals (keine Design-Löcher)

Diese Punkte sind absichtlich der Plan-Phase überlassen, nicht ungelöste
Design-Fragen:

- Genaue `COVERAGE`-Row-Tupel (Phase→Tool) beim Planen festlegen — orientiert an
  den tatsächlich in den Phasen genutzten `ctx_*`-Tools.
- Makro-Verfügbarkeit im Skill-Body: klären, ob `gate()`/`detect_env()`/
  `cleanup_worktree()` per `@import`/`@include` gezogen oder im Meta-Head
  `@define`t werden (Muster von `lmd-executing-plans` als Vorbild).
- Prüfen, ob ein Root-`.claude/skills/`-Stub für die Claude-Code-Discovery
  install-generiert wird (analog bestehender Skills).
