# Spec — `lmd-executing-plans` (nativer lmd-Port von superpowers `executing-plans`)

**Datum:** 2026-07-05
**Status:** Design freigegeben (Brainstorm-Gate passiert), bereit für `lmd-writing-plans`
**Vorbild-Skill:** `lmd-subagent-driven-development` (Registrierung, Phasen-Isolation, Test-Set)
**Port-Quelle:** `superpowers/6.1.1/skills/executing-plans/SKILL.md`

---

## 1. Zweck & Positionierung

Nativer lmd-Port des superpowers-Skills `executing-plans` — die **Inline-Ausführungs-Variante**
eines `.lmd.md`-Implementierungsplans. Der Hauptagent führt den Plan **selbst, in dieser Session**
aus (kein Subagent pro Task), pausiert an Batch-Checkpoints und am finalen Whole-Branch-Gate zur
**menschlichen** Review und schließt über `finishing-a-development-branch` ab.

Der Skill ist bereits als Zielname im Ökosystem verankert: `lmd-writing-plans` verweist im
Handoff-Phase auf `lmd-executing-plans` als **Option 2 „Inline Execution"** (Gegenstück zu
Option 1 = `lmd-subagent-driven-development`).

### Abgrenzung zu SDD (bewusst, um Überlappung zu vermeiden)

| Aspekt          | SDD (Option 1)                            | executing-plans (Option 2)          |
|-----------------|-------------------------------------------|-------------------------------------|
| Task-Ausführung | frischer Subagent/Task                    | Hauptagent inline                   |
| Review          | 2-Verdikt-Reviewer-Subagent pro Task      | Mensch an Batch-Checkpoints         |
| Companions      | implementer / task-reviewer / code-reviewer | **keine**                         |
| Einsatz         | Subagents verfügbar, schnelle Iteration   | Inline mit menschlichen Checkpoints |

### Harte Vorgabe (aus `lmd-writing-plans` body)

Der Inline-Executor **MUSS** dieselbe Dispatch-Baseline (`dispatch-contract.ext`) voranstellen,
die im Subagent-Modus `@dispatch` automatisch liefert. Andernfalls leaken die ambienten Regeln
(Shell-Chaining-Verbot, Sprach-Split DE/EN, Commit-Form), die der Plan bewusst weglässt, weil er
davon ausgeht, dass der Dispatch-Contract sie nachliefert. → Wird fest in die `orient`-Phase per
`@include` eingebaut, keine Option.

---

## 2. Phasen (der Kern — render-on-invoke, phasen-isoliert)

```
orient → preflight → execute → checkpoint → final-gate → finish
```

### `orient`
- Announce: „I'm using the lmd-executing-plans skill to execute the plan."
- **Dispatch-Baseline voranstellen** (`@include` von `dispatch-contract.ext`) — Pflicht, s. §1.
- Plan **kritisch** lesen — Struktur via `lean-md render <plan> --list-phases`, **nie** `ctx_read`
  des Plans (jeder Read-Modus rendert die Quelle → wirkt leer).
- Concerns/Fragen zum Plan VOR dem Start bündeln (nicht mittendrin herauströpfeln).
- **Isolations-/Branch-Guard:** dediziertes Feature-Branch, **nie** `main`/`master` ohne
  expliziten Consent. Optional Shadow-git-Netz vor Task 1: `@call snapshot("pre-execution")`.
- **Resume:** `ctx_session load` + `ctx_knowledge recall` (MCP injiziert ACTIVE-SESSION-Block);
  bereits abgeschlossene Tasks werden nicht erneut ausgeführt (Recovery-Map = `ctx_knowledge` +
  `git log`).
- Once: `ctx_agent action=register agent_type=claude role=executor`.

### `preflight`
- Tasks enumerieren: `lean-md render <plan> --list-phases` → geordneter `name<TAB>title`-Index
  (import-unabhängig, rendert keine Bodies). Ein Todo je Task.
- **Batch-Grenzen festlegen** — Checkpoint-Punkte, an denen der Mensch reviewt (z. B. nach jeder
  Plan-Phasengruppe / vor invasiven Tasks). Die Grenzen sind Executor-Ermessen, beim Preflight
  einmal festgelegt und in `ctx_session` notiert.
- **Pre-flight-Konflikt-Scan:** Plan auf interne Widersprüche / Konflikte mit dem aktuellen Tree
  prüfen; ALLE Bedenken in EINE Frage an den Menschen bündeln — vor Task 1, nie mittendrin.

### `execute` (Per-Task-Loop, inline)
Für jede Task in Reihenfolge, bis zur nächsten Batch-Grenze oder BLOCKED:
1. `BASE = @query "git rev-parse HEAD"` beim Batch-Start notieren (Fidelity-kritisch; nie `HEAD~1`).
2. Task-Brief = `ctx_shell(command="cargo run -q --bin lean-md -- render <plan> --phase task-N
   --consumer=ai", raw=true)` — **raw ist Pflicht** (Doppel-Kompression würde den zu schreibenden
   Code mangeln). Quelldateien der Task in EINEM `ctx_multi_read` warmlesen.
3. Todo `in_progress`.
4. Snapshot um den Edit: `@call snapshot("pre-task-N")` davor, `@call snapshot("post-task-N")`
   danach (ctx_checkpoint — erfasst exakt das Geänderte).
5. Schritte **exakt** folgen (Plan hat bite-sized Steps); plan-spezifizierte Verifikation
   ausführen — i. d. R. `@call gate(<paths>)` (reformat + lint + full `cargo nextest run`) und
   `@read mode=diff` statt Copy-Paste-Inspektion. Reference-Skills anwenden, wenn der Plan es sagt.
6. Todo `completed`; `ctx_session action=task "Task N [x%]"`; durable Gotchas → `ctx_knowledge`.

Kontinuierlich (≤1 Zeile Narration zwischen Tool-Calls); Pause nur an Batch-Grenze, BLOCKED oder
echter Ambiguität.

### `checkpoint` (an Batch-Grenze)
- `HEAD = @query "git rev-parse HEAD"`.
- Diff-Vorlage an den Menschen: `@query "git diff BASE..HEAD"` + `@read mode=diff` — der Mensch
  ist der Reviewer (rein inline, kein Reviewer-Subagent).
- `@call compress()` (ctx_compress) — Kontext-Checkpoint der langen Session.
- Auf Freigabe warten. Freigegeben → nächster Batch (zurück zu `execute`). Änderungen gewünscht →
  Fixes inline, ggf. Revisit (s. §3). Fundamentaler Ansatz falsch → zurück zu `orient`/Plan-Review.

### `final-gate` (nach der letzten Task)
- Deterministischer Vorpass: `@query "git diff merge-base..HEAD" | @review diff-review` plus
  `@smells` (geänderte Oberfläche scannen).
- Findings + Diff dem Menschen vorlegen (rein inline, kein Reviewer-Subagent — bewusste
  Design-Entscheidung, treu zum superpowers-Original).
- Bestätigte Findings inline fixen (ein Durchlauf, Tests erneut über das Gate).

### `finish` (terminal)
- Branch-Abschluss über die externe Referenz `finishing-a-development-branch` (merge / PR /
  cleanup-Wahl an den Menschen präsentiert) — konsistent mit dem aktuellen SDD-`handoff`, bis ein
  lmd-Port existiert.
- Abschluss-Status über `ctx_session action=status` festhalten. Kein „next"-Render.

---

## 3. Erhaltene Funktionen (No-Loss-Check gg. superpowers `executing-plans`)

| superpowers-Funktion                              | lmd-Umsetzung                                            |
|---------------------------------------------------|---------------------------------------------------------|
| Step 1: Plan lesen + kritisch reviewen            | `orient` (kritisches `--list-phases`-Lesen)             |
| Concerns VOR Start ansprechen                     | `orient`/`preflight` (eine gebündelte Frage)            |
| Todos für Plan-Items                              | `preflight` (ein Todo je Task)                          |
| Step 2: pro Task in_progress→Steps→verify→done    | `execute` (inline Loop)                                 |
| Verifikationen nicht überspringen                 | `execute` (`@call gate`, `@read mode=diff`)             |
| Review-Checkpoints                                | `checkpoint` (Batch-Grenzen) + `final-gate`             |
| Step 3: finishing-a-development-branch            | `finish`                                                |
| Stop-and-Ask (Blocker/unklar/Verif. schlägt fehl) | Prosa-Disziplin in `execute`/`checkpoint`               |
| Revisit-Earlier-Steps (Plan geändert/Ansatz falsch)| Prosa-Disziplin in `checkpoint` (→ `orient`/Plan-Review)|
| Nie Implementierung auf main/master ohne Consent  | `orient` (Branch-Guard)                                 |
| Reference-Skills wenn Plan es sagt                | `execute` (Step 5)                                      |

**Stop-and-Ask (STOP sofort bei):** Blocker (fehlende Dependency, Test schlägt fehl, Instruktion
unklar); kritische Plan-Lücken vor dem Start; unverständliche Instruktion; wiederholt fehlschlagende
Verifikation. → Nachfragen statt raten.

**Revisit (zurück zu Review/`orient`):** Mensch aktualisiert den Plan aufgrund von Feedback;
fundamentaler Ansatz muss überdacht werden. → Blocker nicht durchdrücken.

---

## 4. Tool-Strenge (lean-ctx / lean-md durchgängig)

- Progress → `ctx_session` (`action=task|status|finding`)
- Durable Fakten/Gotchas → `ctx_knowledge` (`action=remember|recall`)
- Koordination / Register → `ctx_agent` (`register role=executor`)
- Task-Brief → CLI-`--phase`-Render (raw), **nie** `ctx_read` des Plans
- Task-Enumeration → `lean-md render <plan> --list-phases`
- Snapshots um Edits → `ctx_checkpoint` (`@call snapshot(...)`)
- Kontext-Checkpoint → `ctx_compress` (`@call compress()`)
- Verifikation → `@read mode=diff` + Recipe-`@call`s (`gate`/`verify`)
- Final-Vorpass → `ctx_review` (`@review diff-review`) + `ctx_smells` (`@smells`)
- **Verboten:** superpowers-SDD-Bash-Skripte, Scratch-Ledger-/Brief-/Report-Files, `/tmp`-Ledger.

---

## 5. Registrierung & Tests (Vorbild SDD, 1:1)

### Dateien
- `content/skills/lmd-executing-plans/SKILL.md` — Delegation-Stub (Frontmatter `name` +
  einzeilige `description`; Phasenliste; Hinweis „keine Companions").
- `content/skills/lmd-executing-plans/body.lmd.md` — `@lean-md` / `consumer: ai`; die sechs
  `@phase`-Blöcke aus §2; `@include` der Dispatch-Baseline in `orient`.
- **Keine** `companions/`.

### Code-Registrierung
- `src/skills.rs`: `const LMD_EXECUTING_PLANS_BODY = include_str!(...)`; Eintrag in `SKILLS`.
  **Kein** `COMPANIONS`-Eintrag.
- `src/skill_install.rs`: `const EXECUTING_PLANS_SKILL_MD = include_str!(...)`; Eintrag in
  `INSTALLABLE_SKILLS`. Keine `ASSETS`.
- `src/availability.rs`: `COVERAGE`-Zeilen für `lmd-executing-plans`:
  - `orient` → `include` → `fragment-compose` (Dispatch-Baseline)
  - `preflight` → `read`/`list` (Enumeration) — soweit als registrierte Direktive abbildbar
  - `execute` → `checkpoint` → `ctx_checkpoint` (snapshot); `read` → `ctx_read` (diff-Verifikation)
  - `checkpoint` → `compress` → `ctx_compress`; `query`-Diff soweit abbildbar
  - `final-gate` → `review` → `ctx_review`; `smells` → `ctx_smells`
  - `finish` → ggf. `handoff`/`compress`
  (Exakte Direktiv→Backing-Zuordnung beim Plan-Schreiben gegen `default_registry` verifizieren;
  Coverage-Gate erzwingt Registrierung.)

### Tests
- `executing_plans_install_writes_skill_md` (in `skill_install.rs`): schreibt `SKILL.md`, prüft
  `name: lmd-executing-plans`, Reference-Closure (kein `superpowers`-Token), **Frontmatter-Scalar-
  Guard** (einzeilige, nicht-leere `description`, unquoted ohne `": "`), Idempotenz.
- CRP-/Render-Test: jede der sechs Phasen rendert nicht-leer; `--list-phases` liefert alle sechs.
- Coverage-Gate (`availability`): jede gecoverte Direktive ist registriert.
- Fragment-Konsistenz-Gate (built-in == on-disk seed) bleibt grün (byte-stabil #498).

### Abnahme-Test (End-to-End)
Gegen den bestehenden Plan
`docs/lean-md/plans/2026-07-05-lmd-writing-plans-global-constraints-hardening.lmd.md`:
- `lean-md render <plan> --list-phases` liefert den Task-Index;
- `render --phase task-N` liefert isolierte Briefs;
- die `orient`/`preflight`/`execute`-Prosa des neuen Skills passt ohne Reibung auf diesen Plan.

---

## 6. Nicht-Ziele (YAGNI)

- Kein Reviewer-Companion, kein Subagent-Dispatch (das ist SDD).
- Kein eigener lmd-Port von `finishing-a-development-branch` (externe Referenz, wie SDD).
- Keine neuen Recipes/Direktiven — nur bestehende (`snapshot`/`compress`/`gate`/`verify`/`review`/
  `smells`) wiederverwenden.
- Keine per-Task-Human-Pause (Checkpoint-Takt = Batch-Grenzen, bewusst gewählt).
