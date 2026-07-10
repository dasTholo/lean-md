# SDD-Skill härten, Projekt-Doku entdoppeln — Design

**Datum:** 2026-07-10
**Branch:** `feat-lmd-v2`
**Vorgänger:** `docs/lean-md/specs/2026-07-04-lmd-md-source-read-hardening-design.md`

## Problem

Mit `content/skills/lmd-subagent-driven-development/` existiert der native SDD-Skill.
`.claude/rules/subagent-multi-agent.md` trägt ihr eigenes Verfallsdatum im Text
(*„until `lmd-subagent-driven-development` exists"*) und adressiert einen
Ausführungspfad — `superpowers:subagent-driven-development` —, der in diesem Repo
nicht mehr unterstützt wird. Die Datei ist zu ~85 % im Skill und im Seed
`content/core/dispatch-contract.lmd.md` aufgegangen.

Parallel dazu trägt die Projekt-Doku zwei sachliche Fehler und eine unfertige
Vorgänger-Spec:

1. **`CLAUDE.md:32-34` behauptet das Gegenteil der Realität** — `ctx_read` einer
   `.lmd.md` liefert *keine* Roh-Source, sondern rendert. Die Vorgänger-Spec führt
   das unter Non-Goals: *„Kein Eingriff ins lean-ctx-Addon-Routing (`ctx_read
   mode=raw` weiterhin rendernd)"*. Genau dafür wurde `lean-md source` gebaut.
2. **Die „warm cache"-Begründung ist falsch.** Skill, Seed und Fragment behaupten,
   der erste `ctx_read` eines Subagenten treffe den warmen Cache des Controllers.
   Gemessen trifft er ihn nie.
3. **Die Zugriffs-Map aus der 2026-07-04-Spec wurde nie in den Seed gezogen.**
   `content/core/hard-rules.lmd.md` trägt vier Zeilen; der geteilte
   `.lmd.md`-Zugriffsfakt fehlt.

## Messgrundlage

Beide Korrekturen stehen auf Messung, nicht auf Code-Lektüre.

**Stub-Auslieferung (Ä4).** Ein Subagent las eine Datei, die der Controller zuvor
voll gelesen hatte:

| Leser | Read 1 | Read 2 |
|---|---|---|
| Controller | Volltext | `[unchanged 22L]` Stub |
| Subagent | Volltext | Volltext |

Der Subagent bekommt nie einen Stub. Ursache: `rust/src/core/conversation.rs:271`
(lean-ctx, #1040) — sobald mehr als eine Conversation im Zeitfenster aktiv war,
wird **jeder** Stub zurückgehalten. Der Stub ist eine Rückreferenz auf Inhalt, den
ein fremder Kontext nie gesehen hat; ihn auszuliefern wäre der Defekt, ihn
zurückzuhalten ist korrekt.

Ergänzend (`src/tools/ctx_read/mod.rs:850` in lean-ctx): ein Read in einem
komprimierten Modus trifft den `compressed_cache_key`-Pfad **ohne**
Conversation-Gate. Das spart Rechenzeit, aber keine Tokens — derselbe Text wird
erneut gesendet.

∴ `ctx_multi_read` vor dem Dispatch ist ein **Latenz**-Gewinn, kein Token-Gewinn.
Die Schlussfolgerung „kein `ctx_share`" bleibt gültig; nur die Begründung ist neu.

**`.lmd.md`-Lesesemantik (Ä2).** `ctx_read` auf `content/core/dispatch-contract.lmd.md`
liefert nach `ctx_cache invalidate` reproduzierbar den **gerenderten** Text
(expandiertes `@include hard-rules`, eval-err-Marker für unbesetzte
Template-Variablen). `lean-md source` derselben Datei liefert die Roh-Bytes.

## Entscheidungen

- **`superpowers:subagent-driven-development` ist kein unterstützter Pfad mehr.**
  Damit verliert das Bash-Skript-Verbot der rules-Datei seinen Adressaten.
- **Waisen-Inhalte:** nur `ctx_overview` + `ctx_repomap` (Orientierung beim
  Plan-Start) werden gerettet. `ctx_rules`, `ctx_impact`/`ctx_callgraph` und die
  Precedence-Notiz fallen per YAGNI weg — der Skill nutzt sie nirgends.
- **Kein `@var`-Umbau** von `src/bridges/dispatch.rs`. Siehe Non-Goals.

## Änderungen

### Ä1 — `.claude/rules/subagent-multi-agent.md` löschen

Einziger Konsument ist die `@rules/`-Zeile in `CLAUDE.md:45`. `.claude/rules/`
enthält keine weitere Datei.

`CLAUDE.md:36-45` (Sektion *„Subagent-Driven Execution"*) wird ersetzt. Sie trägt
zwei veraltete Aussagen: sie nennt `superpowers:subagent-driven-development` als
Auslöser, und sie fordert *„The controller MUST prepend the Dispatch Contract to
each subagent prompt"* — was `@dispatch` seit D-7 automatisch tut.

Die Ersatz-Sektion enthält genau drei Aussagen: SDD-Pläne werden mit dem Skill
`lmd-subagent-driven-development` ausgeführt; der Dispatch-Contract lebt als Seed
`content/core/dispatch-contract.lmd.md` (`include_str!` in `src/fragments.rs`) und
wird von `@dispatch` automatisch vorangestellt; Fortschritt, Briefs und Batons
laufen über `ctx_session` / `ctx_knowledge` / `ctx_agent`, nie über Scratch-Dateien.
Die `@rules/`-Zeile entfällt ersatzlos.

### Ä2 — `CLAUDE.md:32-34` korrigieren

Bestehender Text (Edit-Anker):

    Note: `ctx_read` einer `.lmd.md` liefert Roh-Source (Skill-/Plan-Quelle für
    Edit-Anker lesen); Rendern/Preview ist explizit über die CLI (oben) bzw.
    `ctx_md_render`.

Neuer Text: `.lmd.md` wird beim Lesen **gerendert** (Addon-Routing, bewusst so —
Non-Goal der 2026-07-04-Spec). Roh-Source für Edit-Anker ausschließlich über
`lean-md source <file>`.

### Ä3 — Zugriffs-Map in `content/core/hard-rules.lmd.md` nachziehen

Die 2026-07-04-Spec listete `content/core/hard-rules.lmd.md` unter *Betroffene
Dateien* („geteilter `.lmd.md`-Zugriffs-Fakt"). Umgesetzt wurde nur das CLI-Verb.

Ergänzt wird die Zugriffs-Map:

    .lmd.md ist ein gerendertes Artefakt — jeder Lesemodus rendert es:
    - Recipe-Makro-API    → lean-md render … --signatures
    - Plan-/Skill-Phase   → lean-md render … --phase <p>
    - Roh-Source zum Edit → lean-md source <file>

Reichweite: `@include hard-rules` zieht den Block in jeden Skill und in jeden
Subagenten-Contract. Der Fragment-Consistency-Gate (#498, built-in == on-disk-Seed)
hält die Byte-Identität.

### Ä4 — „warm"-Begründung korrigieren

Vier Fundstellen. Der Wortlaut wird von *„der erste `ctx_read` des Subagenten trifft
den warmen Cache"* auf *„Subagenten lesen voll — Stubs werden konversationsübergreifend
zurückgehalten (#1040); `ctx_multi_read` spart Latenz, keine Tokens"* umgestellt.

| Datei | Stelle |
|---|---|
| `content/core/dispatch-contract.lmd.md` | Z. 4 |
| `content/core/_fragments/parallel-dispatch.lmd.md` | Z. 45-46 |
| `content/skills/lmd-subagent-driven-development/body.lmd.md` | Phase `dispatch`, Schritt 2; Phase `parallel-dispatch`, Schritt 1 |
| `content/skills/lmd-dispatching-parallel-agents/body.lmd.md` | Z. 38-39 |

Das `ctx_share`-Verbot bleibt unverändert bestehen — nur seine Begründung wechselt
von „Cache ist warm" zu „ein warmer Cache nützte dem Subagenten ohnehin nichts".

**Wortlaut nach Ort dosieren.** `dispatch-contract.lmd.md` wird von `@dispatch` an
**jeden** Subagenten-Prompt vorangestellt — dort steht ein terser Halbsatz, keine
#1040-Erklärung. Die ausführliche Begründung gehört in
`_fragments/parallel-dispatch.lmd.md` und die Skill-Bodies.

**Kein fünfter Treffer.** `content/skills/lmd-writing-plans/body.lmd.md:135` nennt
ebenfalls einen „warm cache", meint aber die Just-in-time-Auflösung von
`path:line`-Ankern im Plan — **nicht** den Erstlesevorgang eines Subagenten. Diese
Stelle bleibt unangetastet.

### Ä5 — `ctx_overview` + `ctx_repomap` in die `orient`-Phase

Die einzige Waise aus der rules-Datei. Sie ergänzt den bestehenden Resume-Schritt
(`ctx_session load` + `ctx_knowledge recall`).

### Ä6 — `preflight` schärfen

Die Phase warnt bereits korrekt: *„Do NOT `ctx_read` the plan — any read mode
renders it."* Ergänzt wird der positive Pfad: `lean-md source <plan>` liefert die
Roh-Source für exakte Edit-Anker.

## Non-Goals

- **Bug A — Whole-Doc-/Phase-Render-Divergenz.** Ein Inline-Code-Span, der in einem
  Listenpunkt innerhalb eines `@phase`-Blocks über einen Zeilenumbruch läuft,
  verliert im Whole-Doc-Render seine Span-Grenze; `{{ controller_id }}` wird dann als
  `LmdInline` evaluiert (`src/render.rs:57` `resolve_value` → `src/macros.rs:258`
  `eval_string`) und hinterlässt einen eval-err-Kommentar. Der Phase-Render
  (`--phase dispatch`) ist sauber — **der konsumierte Brief war nie betroffen.**
  Eigener Debugging-Zyklus. Minimal-Reproducer:

      @lean-md
      consumer: ai

      @phase "alpha"
      5. `@dispatch skill="s" role=dev
         to_agent="{{ controller_id }}"` — tail.
      @phase-end

  Whole-Doc-Render → eval-err. `--phase alpha` → literal. Ohne `@phase` → literal.

- **Kein `@var`-Umbau** von `src/bridges/dispatch.rs`. Geprüft und verworfen:
  `{{ var controller_id }}` heilt zwar den Whole-Doc-Render (Default greift), wird
  aber im Phase-Render gar nicht aufgelöst (Code-Span ist dort literal) — der Brief
  zeigte künftig `{{ var controller_id }}` statt `{{ controller_id }}`. Wir würden
  das Symptom im kaputten Pfad zudecken und den intakten Pfad verschlechtern.
  Die Sentinels (`CONTROLLER_ID_SENTINEL`, `TO_AGENT_SENTINEL`) bleiben.

- **Kein lean-ctx-Fix.** `#1040` (Stub-Zurückhaltung) ist korrektes Verhalten, kein
  Defekt. Das Addon-Routing für `.lmd.md` bleibt unangetastet.

- **Kein Rust-Code**, abgesehen vom Regressionstest der Verifikation.

## Verifikation

- `cargo nextest run` grün — insbesondere der Fragment-Consistency-Gate nach Ä3/Ä4
  (`src/fragments.rs`, built-in == on-disk-Seed).
- `lean-md render --skill lmd-subagent-driven-development --phase orient --consumer=ai`
  → enthält `ctx_overview` und `ctx_repomap`.
- `lean-md render --skill lmd-subagent-driven-development --phase preflight --consumer=ai`
  → nennt `lean-md source`.
- `lean-md render --skill lmd-brainstorm --phase pre-context --consumer=ai`
  → der `Hard Rules`-Block trägt die Zugriffs-Map (Ä3 wirkt über `@include`).
- `ctx_search "subagent-multi-agent"` über `CLAUDE.md` → 0 Treffer.
- Neuer Regressionstest in `src/skills.rs`: kein Skill-Body und kein Fragment
  behauptet noch, der erste `ctx_read` eines Subagenten sei warm bzw. treffe den
  Cache des Controllers. **Der Korpus muss beide Klassen abdecken** — die
  Skill-Bodies (`all_skill_bodies`) *und* die eingebetteten Fragment-Seeds
  (`DISPATCH_CONTRACT`, `PARALLEL_DISPATCH` in `src/fragments.rs`). Der Wortlaut
  steht in beiden; ein Test nur über die Bodies liefe grün und übersähe den Seed.

## Betroffene Dateien

- `.claude/rules/subagent-multi-agent.md` — **löschen**
- `CLAUDE.md` — Ä1 (Sektion ersetzen), Ä2 (Lesesemantik korrigieren)
- `content/core/hard-rules.lmd.md` — Ä3 (Zugriffs-Map)
- `content/core/dispatch-contract.lmd.md` — Ä4
- `content/core/_fragments/parallel-dispatch.lmd.md` — Ä4
- `content/skills/lmd-subagent-driven-development/body.lmd.md` — Ä4, Ä5, Ä6
- `content/skills/lmd-dispatching-parallel-agents/body.lmd.md` — Ä4
- `src/skills.rs` — Regressionstest
