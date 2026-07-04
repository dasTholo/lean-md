# lmd-subagent-driven-development — Port-Design (Spec)

**Datum:** 2026-07-04
**Quelle:** `superpowers/6.1.1/skills/subagent-driven-development`
**Scope:** SDD-Skill (nativer lmd-Port) — **nur SDD**. Die `lmd-writing-plans`-Terseness ist
herausgelöst nach `docs/lean-md/specs/2026-07-04-lmd-writing-plans-terseness-design.md`
(**Prerequisite** — SDD konsumiert deren Ergebnis)
**Sprache:** Spec = Deutsch; aller gewobene Content (Body/Companions) + Code-Kommentare = Englisch

## Ziel

Den superpowers-Skill `subagent-driven-development` **vollständig ohne Funktionsverlust**
als nativen lmd-Skill portieren, dabei **durchgängig lean-md/lean-ctx-Funktionen** nutzen
statt der superpowers-Bash-Skripte und Datei-Artefakte. Der Skill führt einen
`.lmd.md`-Plan aus: pro Task ein frischer Implementer-Subagent, danach eine
Zwei-Verdikt-Review, am Ende eine Whole-Branch-Review — alle Handoffs über
lean-ctx-Memory/Coordination.

`lmd-subagent-driven-development` (SDD) und `lmd-writing-plans` stehen in unmittelbarem
Zusammenhang: writing-plans **produziert** phasen-isolierte `@phase`-Pläne, SDD
**konsumiert** sie per Phase-Render. Die vertagte Plan-Terseness (Decision
`plan-terseness-deferred-to-sdd`) wird im **Terseness-Spec** eingelöst, weil SDDs
Dispatch-Contract die Executor-Baseline definiert, an der „was ein Plan weglassen darf" sicher
andockt; SDD **konsumiert** das Ergebnis (`crp: compact`-Header + Recipe-Layer) und fädelt es
über die Dispatch-Bridge (s. „Prerequisite: Terseness-Spec").

## Kernentscheidungen (Brainstorm 2026-07-04)

1. **Scope:** SDD **solo**. Die writing-plans-Terseness ist herausgelöst in ihren eigenen
   Spec→Plan→Impl-Zyklus (Prerequisite); SDD referenziert deren Ergebnis, re-spezifiziert es nicht.
2. **Per-Task-Review:** EIN `task-reviewer`-Companion, ZWEI Verdikte (Spec-Compliance +
   Code-Quality) aus einem Diff-Read. Erfüllt den lean-ctx-`spec+quality`-Kontrakt.
3. **Final-Review:** **verpflichtender** `@review diff-review`-Vorlauf (Directive routet
   outbound auf `ctx_review`; Pipe `@query git diff merge-base..HEAD | @review diff-review`)
   **+ `@smells`**-Scan (→ `ctx_smells`) **+** `code-reviewer`-Companion (most-capable
   Modell) fürs Urteil. Nicht redundant: `@review`+`@smells` = deterministische Impact-/
   Caller-/Test-/Smell-Discovery-Karte (+ optional companion-runtime `ctx_quality delta`),
   Companion = LLM-Urteil.
4. **Terseness (konsumiert):** SDD hängt vom `crp: compact`-Header ab, den der Terseness-Spec
   im `plan-template` verankert, und fädelt ihn über die **Dispatch-Bridge** (`{{ crp }}`) in den
   Contract. Die volle CRP-Ankopplung (Recipe-Collapse, „avoid repeating ambient context",
   No-Loss-Set, `compact`-statt-`tdd`-Herleitung) lebt im Terseness-Spec; SDD trägt nur den
   Dispatch-Anteil (s. „CRP-Mechanik & Bindung" — auf Dispatch-CRP getrimmt).
5. **Periphere Handoffs:** lean-ctx-Äquivalente wo möglich — Isolation via dediziertem
   Feature-Branch + optionalem `@checkpoint`-Sicherheitsnetz (Shadow-git; NICHT als
   Worktree-Ersatz ausgegeben — andere Semantik); Branch-Finishing → superpowers-Referenz
   bis lmd-Port existiert.
6. **Neue Directive-Bridges (in diesem Port zu erstellen):** `@checkpoint`→`ctx_checkpoint`
   (Shadow-git-Sicherheitsnetz) und `@compress`→`ctx_compress` (Session-Checkpoint, in
   Phasen via `@call compress()` aufrufbar). Schließt die ctx_checkpoint-Bridge-Lücke; s.
   Abschnitt „Neue Directive-Bridges".
7. **Neue Renderer-Capability (in diesem Port zu erstellen): `render --list-phases`.** Eine
   import-unabhängige Phase-Outline — geordneter `name<TAB>title`-Index aller `@phase`-Blöcke
   ohne Body-Render. Schließt die Preflight-„Overview"-Lücke (Controller kennt Anzahl + IDs +
   Titel aller Tasks ohne Content) und ist **Bug-3-immun** (betritt den Whole-Doc-`@import`-
   Pfad nie). S. Abschnitt „Neue Renderer-Capability: Phase-Outline".

## Architektur & Datei-Layout

Neue Seeds (`include_str!`-gebunden):

```
content/skills/lmd-subagent-driven-development/
  SKILL.md                      # Delegation-Stub (Phasen-Index + Companions)
  body.lmd.md                   # phasen-gerendert (orient…handoff)
  companions/
    implementer.lmd.md          # Dispatch-Brief (Contract auto-prepended)
    task-reviewer.lmd.md        # Zwei-Verdikt-Review-Brief
    code-reviewer.lmd.md        # Whole-Branch-Final-Review-Brief
```

Rust-Bindung (`src/skills.rs`): `const LMD_SDD_BODY = include_str!(…/body.lmd.md)` +
Row in `SKILLS`; 3 Companion-Consts + Rows in `COMPANIONS`.

Neue Bridge-Module (Details s. „Neue Directive-Bridges"): `src/bridges/checkpoint.rs`
(`@checkpoint`→`ctx_checkpoint`), `src/bridges/compress.rs` (`@compress`→`ctx_compress`),
Registrierung in der Bridge-Registry, 2 Gloss-Rows in `content/gloss/directives.lmd.md`,
3 Recipes in `content/templates/plan-recipes.lmd.md` (`compress()`, `snapshot()`,
`task_return()`), Sink-Rename `checkpoint→compress` (+ Alias) und `fire_agent`-Erweiterung
um `return`/`handoff`/`sync` (+ `to_agent`-Attr) in `src/phases.rs`.

Neue Renderer-Capability (Details s. „Neue Renderer-Capability: Phase-Outline"):
`src/phases.rs` bekommt `iter_phase_blocks(source) -> Vec<(name, raw_body)>` (geteilter,
geordneter Phasen-Scanner) + `outline_phases(source) -> Vec<PhaseOutline>` (`{ name, title }`);
`capture_phase_bodies` wird auf `iter_phase_blocks` umgebaut (kein Doppel-Parser). CLI-Flag
`--list-phases` in `src/bin/lean_md.rs::cmd_render` (früher Zweig); Lib-Re-Export in `src/lib.rs`.

**Design-Prinzip:** Body = schlanke Koordinations-Prosa; die eigentlichen
Subagent-Instruktionen leben isoliert in den Companions (einzeln testbar) — analog
`lmd-brainstorm/spec-reviewer` und `lmd-writing-plans/plan-reviewer`.

## Body-Phasen

| Phase | Inhalt (aus superpowers-SKILL.md portiert) |
|---|---|
| `orient` | When-to-use-Check; Tool-Discipline; dedizierter Feature-Branch (nicht `main`) + optional `@checkpoint action=snapshot` (Shadow-git); Resume aus Ledger (`ctx_session load` / `ctx_knowledge recall`) |
| `preflight` | Phasen enumerieren via `render <plan> --list-phases` (geordneter `name`/`title`-Index, import-unabhängig — **kein** whole-doc `ctx_read`); Todos je Phase anlegen; Pre-Flight-Konflikt-Scan (EINE gebündelte Frage an den Menschen vor Task 1) |
| `dispatch` | Per-Task: Brief = `lean-md render <plan> --phase task-N` + Warm-`ctx_multi_read`; `@checkpoint action=snapshot` vor + nach den Implementer-Edits (erfasst die exakte Änderung); Model-Selection-Rubrik; `@dispatch` implementer; Status-Handling |
| `review` | Reviewer holt Diff selbst (`@read mode=diff`); `@dispatch` task-reviewer (2 Verdikte); ⚠️-Item-Auflösung; Reviewer-Prompt-Discipline; Fix-Loop; Task-complete + Ledger-Zeile |
| `final-review` | verpflichtender `@review diff-review`-Vorlauf (`@query git diff merge-base..HEAD \| @review diff-review`) + `@smells`-Scan → `@dispatch` code-reviewer (most-capable) mit den Vorlauf-Findings als Input (Companion darf optional `ctx_quality delta` vs. BASE ziehen); EIN Fix-Subagent für alle Findings |
| `handoff` | Phasen-Boundary-`@compress` (Controller-Kontext-Checkpoint); Branch-Finishing (superpowers-Referenz); terminal |

Red-Flags werden kontextual in die passenden Phasen gewoben (kein eigener Block).
Alle Phasen tragen `next:`-Pointer.

## Companions

- **`implementer`** — Port `implementer-prompt.md`: Fragen-vor-Start, TDD, Self-Review,
  Code-Organization, Eskalation (BLOCKED/NEEDS_CONTEXT). **Report-Handoff remapped:**
  voller Narrativ-Report → `ctx_agent post`/`diary` (bleibt aus dem Controller-Kontext);
  Agent-Return = **kompakter `category/key: value`-Status** (`status: DONE`;
  `commits: <sha,…>`; `tests: <1-Zeilen-Summary>`; `concerns: …`; `tdd_evidence: <ref>`) —
  genau das Format, das der Controller per `ctx_agent action=return` in Parent-Knowledge
  destilliert (B1).
- **`task-reviewer`** — Port `task-reviewer-prompt.md`: liest Brief (render) +
  Implementer-`ctx_agent`-Post + holt Diff selbst (`@read mode=diff`); Do-Not-Trust-Report;
  Part-1 Spec-Compliance (Missing/Extra/Misunderstood + ⚠️), Part-2 Code-Quality;
  Calibration (Critical/Important/Minor, `plan-mandated`); Output = beide Verdikte.
- **`code-reviewer`** — Whole-Branch-Final: Rubrik + Global-Constraints-Lens auf
  `merge-base..HEAD`; nimmt die `@review diff-review`- + `@smells`-Vorlauf-Findings
  (Impact/Caller/Test-Lücken + Smell-Treffer) als Input. **Optional companion-runtime:**
  darf `ctx_quality action=delta` direkt aufrufen (Navigability-/USD-Tax-Regression vs.
  BASE) — objektive Health-Evidenz neben dem Urteil, kein Gate (kein `@quality`-Bridge
  nötig; analog zur ctx_agent-Nutzung der Subagenten).

## Datenfluss (Handoffs über lean-ctx statt Dateien)

Der superpowers-Ansatz bewegt alles als Dateien (brief/report/diff), um den
Controller-Kontext sauber zu halten. lean-md erreicht dasselbe über **Warm-Cache +
Memory/Coordination-Tools** — kein `.superpowers/sdd/`-Verzeichnis, keine Bash-Skripte.

**Preflight (einmal, vor Task 1):** `render <plan> --list-phases` liefert den geordneten
Phasen-Index (`name<TAB>title`) → Todos + Task-Zählung, **ohne** Body-Content im
Controller-Kontext und **ohne** den Bug-3-anfälligen Whole-Doc-Pfad. Der eigentliche
Task-Brief bleibt der Phasen-Render (`render --phase task-N`, raw-captured).

Per-Task-Zyklus:

1. `BASE = @query "git rev-parse HEAD"` (→ `ctx_shell`; notieren — Fidelity-kritisch).
   Lokaler Git-State läuft über `@query`, NICHT über `ctx_git_read` (das liest *remote*
   Repos via URL: overview/tree/read/grep — kein lokaler HEAD-SHA).
2. `brief = lean-md render <plan> --phase task-N` + `ctx_multi_read` (Warm-Cache).
3. `@dispatch` implementer (+ Modell via Agent-Tool). Implementer: `ctx_agent register` →
   implement/TDD/commit → self-review → voller Narrativ-Report `ctx_agent post`/`diary`;
   Return = kompakter `category/key: value`-Status.
4. Status-Handling.
5. `HEAD = @query "git rev-parse HEAD"`.
6. `@dispatch` task-reviewer (BASE..HEAD, Global-Constraints verbatim). Reviewer liest
   Brief + Implementer-Post + holt Diff selbst; 2 Verdikte → `ctx_agent post`; Return =
   ✅/❌/⚠️ + Approved/Needs-fixes.
7. Fix-Loop bei Critical/Important (Fix-Subagent trägt Implementer-Contract: Tests re-run).
8. Task complete: Controller reicht den `category/key: value`-Return über den
   `on-complete=return`-Sink (`@call task_return(...)`) → `ctx_agent action=return` →
   **destilliert deterministisch in Parent-Knowledge** (confidence 0.8; nicht-passende
   Zeilen werden gelistet, nie stumm verworfen) + `ctx_session action=task "Task N [x%]"`. Das ersetzt das manuelle `ctx_knowledge
   remember` je Task (B1). **`ctx_agent brief` wird NICHT als Task-Brief genutzt** — der
   Brief ist der Plan-Render (`render --phase task-N`), die autoritative Quelle.
   A2A-Task-Board (`ctx_task`) ist optional; falls genutzt, ist der gültige In-Arbeit-State
   `working` (nicht `in_progress` — der Parser lehnt es ab). (B3)

**Nie im Controller-Kontext:** voller Report (→ `ctx_agent`), Diff (→ Reviewer holt ihn).
**Durabel (übersteht Compaction/Session):** Progress (`ctx_session`), Fakten/Commits
destilliert via `ctx_agent action=return` in Parent-Knowledge (`ctx_knowledge`) — ersetzt
das flüchtige `progress.md`, cross-session-fähig. Auto-Restore: bei Session-Start injiziert
der MCP-Server den `ACTIVE SESSION`-Block (Progress/Findings/Files) ohne Call.
**Resume:** bei Start `ctx_session load` + `ctx_knowledge recall`; complete markierte Tasks
werden NICHT neu dispatcht (Recovery-Map = `ctx_knowledge` + `git log`).
**BASE-Disziplin:** immer notierte BASE, nie `HEAD~1` (droppt Multi-Commit-Tasks).

## Fehlerbehandlung & Status-Logik

Implementer-Status (faithful): **DONE** → Review; **DONE_WITH_CONCERNS** → Concerns lesen,
Korrektheit/Scope vor Review adressieren; **NEEDS_CONTEXT** → Kontext liefern, re-dispatch;
**BLOCKED** → Blocker bewerten (Kontext → mehr Kontext/gleiches Modell; Reasoning →
fähigeres Modell; zu groß → aufteilen; Plan falsch → an Mensch). Nie dasselbe Modell ohne
Änderung erneut zwingen.

**Reviewer ⚠️-Items:** blockieren die Review nicht; der Controller löst jeden selbst auf
(hält Cross-Task-Kontext) — bestätigte Lücke = fehlgeschlagene Spec-Review → zurück an
Implementer.

**Plan-mandated / Plan-Konflikte:** Finding + Plan-Text dem Menschen vorlegen — nie stumm
verwerfen, nie plan-widersprechenden Fix ohne Rückfrage dispatchen.

**Final-Review-Findings:** EIN Fix-Subagent mit kompletter Liste (nicht pro-Finding).

**Continuous-Execution:** zwischen Tasks nicht einchecken — Stopp nur bei unlösbarem
BLOCKED, echter Ambiguität oder „alle Tasks fertig". Narration ≤1 Zeile zwischen Tool-Calls.

**Model-Selection-Ausdruck:** `@dispatch` komponiert nur den Brief; das Modell setzt der
Controller beim Agent-Tool-Call. Rubrik (mechanisch→cheap, Integration→standard,
Architektur/Final-Review→most-capable) als Guidance im Body. **Kein Engine-Change an
`@dispatch`.**

## Prerequisite: Terseness-Spec (SDD konsumiert, spezifiziert nicht)

Die writing-plans-Terseness ist **herausgelöst** (`…-lmd-writing-plans-terseness-design.md`) und
hat dort ihren eigenen Spec→Plan→Impl-Zyklus. SDD **hängt** von drei ihrer Ergebnisse ab und
re-spezifiziert sie **nicht**:

1. **`crp: compact`-Header** im `plan-template` (Terseness-Deliverable). SDDs Dispatch-Bridge
   fädelt `crp` als `{{ crp }}` in den Contract → jeder dispatchte Subagent bekommt „render in
   CRP mode `compact`" (verifiziert in `src/bridges/dispatch.rs`, Test
   `dispatch_threads_crp_compact_into_contract`). Ohne den gelandeten Header greift SDDs
   Dispatch-CRP-Zeile ins Leere.
2. **Bug 1-Fix** (quote/komma-bewusster `@call`-Argument-Split) ist **auch** SDD-Prerequisite,
   nicht nur Terseness: SDDs eigenes Recipe `task_return("status: DONE; commits: …")` ist ein
   **Ein-Arg-Recipe mit Binnen-Kommas/Semikolons** — trifft Bug 1 exakt wie `remember_decision`
   (alles nach dem ersten Komma still verschluckt / Quote-Leak). SDDs `@compress`/`@checkpoint`-
   Recipes (`snapshot("<label>")`) sind ebenfalls quoted-Ein-Arg. ∴ Bug 1 muss vor SDDs
   Recipe-Tasks landen.
3. **Co-owned `plan-recipes.lmd.md`.** SDD ergänzt `compress()/snapshot()/task_return()`,
   Terseness `gate/render_check` — dieselbe Datei, kein inhaltlicher Konflikt. Beide Sätze
   tragen die HTML-Kommentar-Erstzeile, damit `no_orphan_call`/`plan_recipes_all_documented`
   grün bleiben. Sequencing: Bug-1-Fix (Terseness Task 1) **vor** beiden Recipe-Erweiterungen.

**Sicherheits-Kopplung (SDD-seitig relevant):** Das Weglassen ambienten Kontexts (Terseness-
Regel) ist sicher, weil SDDs Dispatch-Contract (`content/core/dispatch-contract.lmd.md`)
Hard-Rules + Tool-Discipline nachliefert und `render --phase task-N` den Task self-contained
macht. **Zukunfts-Constraint:** sobald `lmd-executing-plans` portiert wird, MUSS es dieselbe
Baseline prependen — sonst leakt die Auslassung bei Inline-Ausführung.

## CRP-Mechanik & Bindung — SDD-Anteil (Dispatch-CRP)

> Die vollständige Zwei-Oberflächen-Herleitung (Render-CRP `apply_crp_hook`, `compact == tdd`
> autorenseitig, `compact`-statt-`tdd`-Begründung, Plan-Header-Bindung) lebt im **Terseness-Spec**
> (`…-lmd-writing-plans-terseness-design.md`, Abschnitt „CRP-Mechanik & Bindung"). Hier steht nur
> der Teil, den SDDs Dispatch-Bridge selbst trägt.

**Dispatch-CRP** (der reale SDD-Hebel, `src/bridges/dispatch.rs`). Der Dispatch-Bridge
substituiert `{{ crp }}` im `dispatch-contract`-Seed → die Contract-Zeile
`render in CRP mode \`compact\``. Verifiziert durch `dispatch_threads_crp_tdd_into_contract`
(+ neu `_compact_`). Das ist der einzige Ort, an dem `compact` vs `tdd` real divergiert: die
Disziplin-Anweisung an den dispatchten Subagenten. `tdd`s „zero narration" würde genau das
Reasoning unterdrücken, das task-reviewer/code-reviewer/Controller zum Bewerten (Concerns,
Design-Begründung, ⚠️-Auflösung) brauchen — daher `compact` (Detail-Beleg im Terseness-Spec).

**Quelle des `compact`-Werts:** der **Plan-Header `crp: compact`** (Terseness-Deliverable), nicht
die lean-ctx-Session-Config — self-contained + byte-stabil (#498), unabhängig vom
`compression_level` der ausführenden Session.

## Neue Directive-Bridges (in diesem Port zu erstellen)

Die `ctx_checkpoint`-Lücke (kein Bridge) wird geschlossen; zusätzlich wird `ctx_compress`
als authored Directive verfügbar. Die zwei neuen **Directives** (1, 2) sind Outbound-Bridges
über den `CodeIntelBackend` (CLI default / MCP opt-in), byte-stabile Tool-Text (#498),
headless → `BACKEND_REQUIRED`-Envelope verworfen (kein Body-Output) — identisches Muster wie
die bestehenden Work-Bridges (`@review`/`@impact`/…); Registrierung in der Bridge-Registry +
je eine Gloss-Row. Die **Sink-Erweiterung** (3) nutzt denselben `ctx.backend.call`-Pfad
(`ctx_agent`), ohne neues `@`-Directive.

**1. `@checkpoint` → `ctx_checkpoint`** (Shadow-git, getrennt von der User-`.git`).
- Zweck: Per-Task-Isolations-Sicherheitsnetz — `snapshot` vor + nach den Implementer-Edits
  erfasst exakt die Änderung; `log`/`diff`/`restore` für Recovery.
- Args: `action=snapshot|log|diff|restore`, optional `label=`/`message=`.
- Neu: `src/bridges/checkpoint.rs`; Gloss-Row `checkpoint | Checkpoint (shadow-git) {raw}`.
- Recipe: `@call snapshot("<label>")` → `@checkpoint action=snapshot label="<label>"`.

**2. `@compress` → `ctx_compress`** (Session-Kontext-Checkpoint, lange Konversationen, #541).
- Zweck: Controller-Kontext an Phasen-Boundaries checkpointen (Delta-Playbook).
- **Per `@call` in einer Phase aufrufbar (Anforderung):** neuer `plan-recipes`-Eintrag
  `@call compress()` → `@compress action=checkpoint`.
- Neu: `src/bridges/compress.rs`; Gloss-Row `compress | Compress session {raw}`.

**3. `ctx_agent`-Sink-Erweiterung** (`fire_agent`, `src/phases.rs` — kein `@`-Directive).
Die bestehenden `on-complete=post`/`diary`-Sinks (→ `ctx_agent`, `phases.rs:130-131,178`)
werden um **`return`**, **`handoff`**, **`sync`** ergänzt. Damit ist der controller-seitige
B1-Handoff **authored-via-Sink** statt roher Runtime-Call.
- `on-complete=return="<category/key:value-report>"` → `ctx_agent action=return`
  (destilliert in Parent-Knowledge, confidence 0.8).
- `on-complete=handoff="<baton>" to_agent="<id>"` → `ctx_agent action=handoff`. **`fire_agent`
  muss dafür um ein `to_agent`-Attr erweitert werden** (heute nur `action/message/category`).
- `on-complete=sync` → `ctx_agent action=sync` (Team-Status; `message` leer/ignoriert).
- Recipe: `@call task_return("status: DONE; commits: …")` → `on-complete=return=…`.
- Kein `@agent`-Directive nötig. **Subagent-seitige `register/post/handoff` bleiben zwingend
  Runtime** (Contract-Text) — nicht bridgebar (isolierter Kontext).

**Disambiguierung (Namenskollision auflösen):** Der bestehende Phase-`on-complete=checkpoint`-
Sink (`phases.rs:163`) routet HEUTE bereits auf `ctx_compress` — nicht auf `ctx_checkpoint`.
Damit „checkpoint" nicht zweierlei bedeutet, wird der Sink zu **`on-complete=compress`**
umbenannt (deprecated Alias `checkpoint` bleibt für Back-Compat, feuert weiter `ctx_compress`).
∴ eindeutig getrennt: `@checkpoint`/`ctx_checkpoint` = Shadow-git-Snapshot;
`@compress`/`ctx_compress`/`on-complete=compress` = Session-Compaction. **`ctx_checkpoint`
bleibt KEIN Worktree-Ersatz** (Snapshot/Restore ≠ Isolation) — s. Nicht-Ziele.

## Neue Renderer-Capability: Phase-Outline (`render --list-phases`)

**Zweck:** SDDs `preflight` braucht die **Struktur** eines Plans (wie viele Tasks, welche
`@phase`-IDs, welcher Titel) — nicht deren Inhalt. Heute gibt es dafür keinen sauberen Weg:
Whole-Doc-Render scheitert an Bug 3, `ctx_read mode=full` rendert ebenfalls whole-doc, und
`--phase` blind durchprobieren kennt die Namen nicht vorab. Die Outline schließt genau diese Lücke.

**Surface:** `lean-md render <plan> --list-phases` → geordnete `name<TAB>title`-Zeilen:

    task-1	Task 1 — Bug 1: quote-aware @call-arg split
    task-2	Task 2 — Bug 3: whole-doc render resolves @import

- Früher Zweig in `cmd_render`: Source laden → `outline_phases` → Index → exit. Kein Render,
  **kein `@import`**, keine Bodies.
- **Mutually exclusive** mit `--phase` (Fehler bei beidem). `--consumer`/`--crp` werden
  ignoriert (strukturelle Ausgabe — kein CRP-Suffix, kein `<!-- crp:… -->`-Footer).
- **Import-unabhängig ∴ Bug-3-immun:** scannt nur die `@phase "name"`-Marker im Plan selbst.

**Mechanik (Lib-Kern, nicht CLI-only):**
- `iter_phase_blocks(source) -> Vec<(name, raw_body)>` — geteilter, **geordneter** Phasen-
  Scanner; erbt die `@phase`/`@phase-end`-Semantik (unterminated/nested = sichtbarer Fehler,
  identisch zu `capture_phase_bodies`).
- `outline_phases(source) -> Vec<PhaseOutline>` mit `PhaseOutline { name, title }`. **Titel** =
  erste `## `/`# `-Überschrift im Body (`#`/Spaces gestrippt); Fallback: erste nicht-leere
  Nicht-Directive-Zeile; sonst leer.
- `capture_phase_bodies` wird auf `iter_phase_blocks` umgebaut → **eine** Quelle der
  Phasen-Grenzen-Semantik (kein zweiter Parser).
- Das CLI-Flag ist nur **ein** Konsument von `outline_phases`; dieselbe Lib-Funktion trägt
  später eine MCP-Variante (`ctx_md_outline` / `ctx_md_render`-Modus) — echte Pipeline, nicht
  CLI-Sonderfall.

**Reichweite:** läuft auf jeder Render-Source (Plan-Datei **und** `--skill X`, gleicher
Source-Load-Pfad) → Skill-Phasen fallen gratis mit ab.

**Landing-Unabhängigkeit:** Bug-3-immun, daher **nicht** an die Bug-1/Bug-3-Prerequisites
gekoppelt — kann eigenständig landen (eigener Commit).

**YAGNI:** kein JSON, keine Intent-Zeile, kein Global-Constraints-Dump — nur der geordnete
`name<TAB>title`-Index, byte-stabil (#498).

## Testing & Gates

`src/skills.rs`:
- `sdd_all_phases_render_nonempty` — jede Phase rendert nicht-leer.
- `sdd_phase_isolation_no_cross_phase_leak`.
- `sdd_render_is_byte_stable` (#498).
- `sdd_companions_resolve` — implementer/task-reviewer/code-reviewer nicht-leer.
- `sdd_companion_body_matches_seed_file_on_disk` (Fragment-Konsistenz).
- `no_dangling_companion_refs_in_seeds` — um die 3 Companions erweitert.
- `phases_carry_next_pointers` — SDD-Kette.
- `sdd_dispatch_implementer_composes` — `@dispatch` prependet den Dispatch-Contract.
- `dispatch_threads_crp_compact_into_contract` (`src/bridges/dispatch.rs`) — **neu**, neben
  dem bestehenden `_tdd_`-Test: `crp: compact` → Contract-Zeile „CRP mode `compact`".

`src/availability.rs`:
- `COVERAGE`-Rows (erstes Feld = voller Skill-Name-String
  `"lmd-subagent-driven-development"`): `(…, dispatch, implementer, dispatch)`,
  `(…, review, task-reviewer, dispatch)`, `(…, final-review, code-reviewer, dispatch)`,
  `(…, final-review, review, ctx_review)`, `(…, final-review, smells, ctx_smells)`.
- `every_covered_directive_is_registered` / `coverage_carries_skill_dimension` bleiben grün.
- COVERAGE-Rows für die neuen Bridges: `(…, dispatch, checkpoint, ctx_checkpoint)`,
  `(…, handoff, compress, ctx_compress)`.

Neue Bridges (`src/bridges/checkpoint.rs`, `src/bridges/compress.rs`, `src/phases.rs`):
- `checkpoint_bridge_registered` / `compress_bridge_registered` — in `default_registry`.
- `checkpoint_composes_snapshot` — `@checkpoint action=snapshot` → Outbound-Args an
  `ctx_checkpoint` (byte-stabil, #498; headless → verworfene BACKEND_REQUIRED-Envelope).
- `compress_composes_checkpoint` — `@compress action=checkpoint` → Outbound an `ctx_compress`.
- `recipe_compress_expands` — `@call compress()` rendert `@compress action=checkpoint`;
  `recipe_snapshot_expands` — `@call snapshot("l")` rendert `@checkpoint action=snapshot`.
- `oncomplete_compress_sink_fires_ctx_compress` — `on-complete=compress` feuert `ctx_compress`;
  `oncomplete_checkpoint_alias_still_fires` — deprecated `checkpoint`-Alias feuert weiter.
- Gloss: `checkpoint`/`compress`-Rows aufgelöst (kein generischer Fallback).

`ctx_agent`-Sink-Erweiterung (`src/phases.rs`, `fire_agent`):
- `oncomplete_return_sink_fires_ctx_agent_return` — `on-complete=return` → `ctx_agent action=return`.
- `oncomplete_handoff_sink_passes_to_agent` — `on-complete=handoff … to_agent="c"` → Payload
  trägt `action=handoff` + `to_agent=c` (verifiziert die neue `to_agent`-Attr-Weitergabe).
- `oncomplete_sync_sink_fires` — `on-complete=sync` → `ctx_agent action=sync`.
- `recipe_task_return_expands` — `@call task_return("status: DONE")` → `on-complete=return=…`.
- `post_diary_sinks_unchanged` — bestehende post/diary-Sinks bleiben grün (Regressionsschutz).

Phase-Outline (`src/phases.rs`, `src/bin/lean_md.rs`):
- `iter_phase_blocks_orders_phases` — zwei `@phase`-Blöcke → geordnete `(name, body)`-Liste.
- `outline_derives_title_from_first_heading` / `outline_title_falls_back_when_no_heading`.
- `outline_is_import_independent` — Source mit fehlendem `@import` listet trotzdem alle Phasen
  (kein NotFound, kein Bug-3-Pfad).
- `outline_is_byte_stable` (#498) — zwei Läufe byte-identisch.
- `capture_phase_bodies_still_matches` — Refactor auf `iter_phase_blocks` lässt die
  bestehenden `capture_phase_bodies`-Tests grün (Regressionsschutz).
- CLI-Integration `render_list_phases_emits_index` — `--list-phases` auf einer Plandatei
  emittiert `task-1<TAB>…\ntask-2<TAB>…`; `list_phases_and_phase_mutually_exclusive` — Fehler
  bei beidem; phasenlose/leere Source → leerer Output, exit 0.

writing-plans (Terseness-Spec-owned, hier nur referenziert — **nicht** in SDDs Test-Scope):
- `writing_plans_teaches_crp_compact_and_no_repeat`, `plan_template_header_declares_crp_compact`,
  `plan_recipes_carry_gate_and_render_check` gehören dem **Terseness-Spec**. SDD verlässt sich
  auf ihr GREEN als Prerequisite, spezifiziert sie aber nicht.

Co-owned `plan-recipes.lmd.md` (SDDs 3 Recipes müssen die gemeinsamen Index-Gates grün halten):
- `plan_recipes_all_documented` / `no_orphan_call` bleiben grün — `compress()/snapshot()/
  task_return()` tragen je die HTML-Kommentar-Erstzeile (wie `gate`/`render_check` des Terseness-Sets).

Behavioral (Plan-Task-Ebene, kein `cargo`-Gate):
- Render-Smoke pro Phase via CLI, byte-stabil.
- **Fidelity-Matrix** (No-Function-Loss-Nachweis): jeder superpowers-SDD-Abschnitt
  (When-to-Use, Process, Pre-Flight, Model-Selection, Status-Handling, ⚠️-Items,
  Reviewer-Prompt-Discipline, File-Handoffs→lean-ctx, Durable-Progress, Red-Flags,
  Final-Review) mappt auf eine SDD-Phase/Companion — kein Abschnitt ohne Ziel.

## Explizite Nicht-Ziele (YAGNI)

- Kein Port der peripheren Skills (`executing-plans`, `requesting-code-review`,
  `finishing-a-development-branch`, `using-git-worktrees`) — nur Referenzen/Äquivalente.
- Kein Engine-Change an `@dispatch` (kein Modell-Param) — Model-Selection ist Controller-Job.
- Keine Datei-Artefakte, keine SDD-Bash-Skripte (`ctx_session`/`ctx_agent`/`ctx_knowledge`).
- `ctx_checkpoint` ist NICHT als Worktree-Ersatz ausgegeben (Snapshot/Restore ≠ Isolation).
- **Kein SDD-Scope:** Terseness-Autorenregeln (Body `plan-format`/`bite-sized`/No-Loss),
  `gate`/`render_check`-Recipes, `lint_cmd`, `vars.toml`, Bug-1/Bug-3-Fixes — alle im
  **Terseness-Spec** (Prerequisite). SDD referenziert, spezifiziert nicht.
- **Phase-Outline:** kein JSON, keine Intent-Zeile, kein Global-Constraints-Dump; nur
  `name<TAB>title`. Kein separater `outline`-Subcommand (Flag an `render` genügt).
