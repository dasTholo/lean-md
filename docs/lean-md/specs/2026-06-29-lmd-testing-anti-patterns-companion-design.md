# `testing-anti-patterns`-Companion — Design (Spec #2, Baustein 1)

> **Status:** Design abgeschlossen, bereit für Implementation-Plan.
> **Vorgänger:** `2026-06-29-lmd-test-driven-development-foundation-design.md` (Spec #1)
> **Upstream-Vorlage:** `superpowers/6.0.3/skills/test-driven-development/testing-anti-patterns.md`
> **Branch:** `feat-lmd-v2`

## 1. Kontext & Ziel

Spec #1 hat `lmd-test-driven-development` (4 phasen-isolierte Render-Blöcke
`red`/`green`/`refactor`/`rationalizations`) plus das geteilte Skill-Plattform-Fundament
(Registry, Body-Override D7, `ctx_md_render`-Skill-Verdrahtung, `skill install/remove`,
COVERAGE-Dimension) geliefert. Bewusst **nicht** in Spec #1 (R4, Scope-Abgrenzung §8):
der Port der Upstream-`testing-anti-patterns.md` als Companion. Spec #1 trägt dafür nur
einen **Vorwärts-Pointer** in der `rationalizations`-Phase (`body.lmd.md:52`):

> `(For testing anti-patterns, see the companion ported in Spec #2 — testing-anti-patterns.)`

Dieser Pointer zeigt aktuell ins Leere — der Companion existiert nicht, kein Render-Target,
keine Registry-Spalte. **Ziel dieses Specs:** den ersten Companion-Baustein von Spec #2
liefern — die **Out-of-band-Companion-Maschinerie** (Render-Param + Registry-Spalte) plus
den portierten `testing-anti-patterns`-Seed, sodass der Pointer auflösbar wird. Die
Maschinerie ist bewusst **wiederverwendbar** für künftige Companions (writing-skills u. a.).

**Nicht-Ziel:** `lmd-writing-skills` (restlicher Spec-#2-Scope); generische Multi-Companion-Discovery
über das eine Target hinaus (YAGNI — das Registry-Schema trägt aber bereits `Option`, also
erweiterbar); Rust-Übersetzung der Upstream-Beispiele (siehe E2); Eingriff in den render core.

## 2. Upstream-Analyse (Vorlage)

Die Upstream-`testing-anti-patterns.md` ist ein **flacher On-demand-Reference** (~250 Zeilen),
**nicht** phasenzyklisch. Trigger: „Load this reference when: writing or changing tests,
adding mocks, or tempted to add test-only methods to production code."

Inhalt (5 Anti-Patterns, je Violation/Why/Fix/Gate-Function):

1. **Testing Mock Behavior** — assert auf Mock-Existenz statt echtes Verhalten.
2. **Test-Only Methods in Production** — Produktionsklasse mit nur-für-Tests-Code verschmutzt.
3. **Mocking Without Understanding** — Mock zerstört Seiteneffekt, von dem der Test abhängt.
4. **Incomplete Mocks** — Partial-Mock verbirgt strukturelle Annahmen, scheitert still.
5. **Integration Tests as Afterthought** — Tests als optionaler Nachgedanke statt TDD.

Plus: Quick-Reference-Tabelle (Anti-Pattern | Fix), Red-Flags-Liste, „TDD Prevents These
Anti-Patterns"-Abschluss (Rückbindung an TDD), Bottom-Line. Alle Codebeispiele sind
TypeScript/React/vitest.

## 3. Entscheidungen (E)

| ID | Frage                          | Entscheidung                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
|----|--------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| E1 | Render-Mechanik / Anbindung    | **Option A — eigenes Companion-Target + Registry-Spalte.** Separater `companion`-Render-Param `ctx_md_render(skill, companion="…")`, neue Registry-Spalte. Saubere Trennung Zyklus-Phasen ↔ On-demand-Reference; wiederverwendbar als Fundament für künftige Companions (vom Spec §8 als Fundament-Baustein deklariert). Verworfen: B (5. Off-Sequence-Phase — semantischer Phasen-Missbrauch, nicht wiederverwendbar), C (eigenständige Skill — Disziplin-Duplikat, falsche Kopplung; Upstream ist explizit *Companion*, kein Peer).                                                                               |
| E2 | Inhaltstreue & Beispielsprache | **Kondensiert + sprachneutral.** Je Anti-Pattern ~2 Zeilen (Kern-Regel + eine Gate-Frage als Trip-Wire); Quick-Reference- + Red-Flags-Liste behalten; lange TS-Codeblöcke raus, konkrete Beispiele als Prosa-Regel. Spiegelt den Body-Port-Präzedenzfall (TDD-Skill ~200→~50 Zeilen verdichtet) und hält den embedded Seed klein (#498). Verworfen: Voll-Port (Stilbruch, Seed-Bloat); Rust-Übersetzung (3/5 Anti-Patterns sind konzeptuell; erfundene mockall-Story = Ballast).                                                                                                                                    |
| E3 | Disziplin-Block im Companion   | **`@include test-first-core` im Companion (5. Include-Site).** Der Companion ist ein separates Render-Target — `body.lmd.md` ist an seinem Render nicht beteiligt, die 4 Phasen-Includes tragen nicht herein. Da der Companion isoliert geladen wird (Trigger „beim Mocken/Test-Schreiben"), oft ohne aktive Phase, muss er die Iron-Law-Trip-Wire selbst tragen — spiegelt den Upstream-„TDD Prevents These"-Abschluss. **Kein Duplikat:** `@include` ist eine Direktive auf das eine Fragment `_includes/test-first-core.lmd.md`. Verworfen: Verweis statt Include (bricht Selbsttrag-Logik der Block-Isolation). |
| E4 | Materialisierung               | **Out-of-band wie der Skill-Body.** Companion wird **nie** nach `.claude/skills/` materialisiert; nur der `SKILL.md`-Stub wird via `skill install` installiert, der Companion fließt ausschließlich übers MCP/CLI-Tool.                                                                                                                                                                                                                                                                                                                                                                                             |
| E5 | Param-Disjunktion              | **`phase` XOR `companion`.** Beide gesetzt → Fehler; unbekannter Companion-Name → `None`/Fehler analog unbekannter Phase.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| E6 | Stub-Body — Orientierung       | **Treue Adaption des Upstream-`SKILL.md`.** Der bewährte Upstream-Skill ist die Orientierungsvorlage: sein `SKILL.md` gibt in *einem* Lesen die volle Orientierung. Unser Stub trägt dieselbe **Orientierungsschicht** (Overview/Core-Principle, When to Use, **Iron Law**, Red→Green→Refactor-Map, „Testing Anti-Patterns"-Trigger, Final Rule — Wortlaut nah am Upstream), die per-Phase-**Detail** (Good/Bad, Verify-Schritte, Rationalizations-Tabelle) bleibt im gerenderten Block → keine Duplikation. `description` bleibt **Trigger-only** (SDO/Discovery, identisch zum Upstream). Zusätzlich: „Where this runs"-Block (Tool kommt vom lean-md-Addon, kein src nötig — lean-md gilt als verfügbar, kein Fehlerpfad) — die fehlende W-Antwort für frische Sessions. Verworfen: dünner Minimal-Stub (zu wenig Orientierung — der ursprüngliche Anstoß). |
| E7 | Companion-Trigger + Phasen-Wegweiser | **Upstream-Wortlaut für den Companion-Trigger** im Stub („When adding mocks or test utilities, …" + 3-Bullet-Preview), zusätzlich zum Pointer in der `rationalizations`-Phase (§5.4). **„next:"-Zeiger** am Ende jedes gerenderten Phasen-Blocks (`red`→„next: render `green`", … `refactor`→„next: RED for the next behavior") — führt die Session nach dem Stub vom Render-Output selbst weiter (Upstream hat das implizit über das eine Dokument; bei phasenweisem Rendern macht es die Führung robust gegen Stub-Vergessen). |

## 4. Architektur

### 4.1 Render-Pfad

```
ctx_md_render(skill="lmd-test-driven-development", companion="testing-anti-patterns")
  → Registry-Lookup (skill, companion) → embedded Seed (include_str!)
  → render core (resolve @include test-first-core)
  → byte-stabiler Companion-Output (ein flacher Block, keine Phasen-Sequenz)
```

Der Companion ist **ein flacher Block** — Render gibt den ganzen Body zurück
(entspricht der Upstream-„load this reference"-Semantik, kein Phasen-Capture).

### 4.2 Out-of-band-Datenfluss (wie Spec #1)

`CliBackend` (default) und `McpBackend` (`mcp`-Feature) treffen denselben Handler →
byte-identischer Output (#498). Render-Core parst keinen Code lokal.

## 5. Komponenten

### 5.1 Seed-Datei

- **Pfad:** `content/skills/lmd-test-driven-development/companions/testing-anti-patterns.lmd.md`
- **Embedding:** `include_str!` in `src/fragments.rs` (byte-stabil, #498).
- **Struktur (kondensiert, sprachneutral):**
    - **Kopf:** `@include test-first-core` (E3 — selbsttragende Iron-Law-Disziplin).
    - **5 Anti-Patterns** je ~2 Zeilen: Kern-Regel + Gate-Frage als Trip-Wire
      (Muster: „BEFORE <Aktion>: <Frage>? → IF <Verletzung>: STOP <Fix>").
    - **Quick-Reference-Tabelle** (Anti-Pattern | Fix) — hochverdichtet, behalten.
    - **Red-Flags**-Liste (knapp) — die actionable Smell-Signale.
    - Keine langen TS-Codeblöcke; konkrete Beispiele als Prosa-Regel (z. B.
      „assert nie auf `*-mock`-Test-IDs").

### 5.2 Registry-Spalte

- `SKILLS`-Tabelle um Companion-Bezug erweitern — entweder
  `SKILLS: &[(name, body, companion: Option<&str>)]` oder separate
  `COMPANIONS: &[(skill, companion_name, body)]`-Tabelle keyed auf `(skill, companion)`.
  Verhalten identisch; finale Form im Plan. TDD-Eintrag → Companion `testing-anti-patterns`.
- Unbekannte Skill/Companion-Kombination → `None`.

### 5.3 `ctx_md_render`-Param (MCP + CLI)

- MCP-Tool `ctx_md_render`: optionaler `companion`-Param neben `phase`.
- CLI: `render --skill X --companion Y` spiegelt `--phase`.
- Gleicher Handler → CLI == MCP byte-identisch (#498).
- `phase` XOR `companion` (E5): beide → Fehler; unbekannt → Fehler.

### 5.4 Pointer-Auflösung

- `rationalizations`-Phase in `body.lmd.md` (aktuell Zeile mit „see the companion ported
  in Spec #2"): ersetzen durch konkrete Lade-Anweisung
  `ctx_md_render(skill="lmd-test-driven-development", companion="testing-anti-patterns")`.

### 5.5 SKILL.md-Stub-Ersetzung (E6/E7)

- Datei: `content/skills/lmd-test-driven-development/SKILL.md` — Stub-Body ersetzen durch
  die treue Upstream-Adaption (Orientierungsschicht). Sektionen:
    - **Header:** Overview/Core-Principle („Write the test first. Watch it fail…") + Hinweis,
      dass die Detailtiefe phasenweise gerendert wird; nie Body/Companion von Disk lesen.
    - **Where this runs:** `ctx_md_render` kommt vom lean-md-Addon (lean-ctx MCP oder
      `lean-md`-CLI); kein src nötig (Bodies eingebettet). lean-md gilt als verfügbar —
      kein Tool-fehlt-Fehlerpfad (der Stub existiert nur, weil `skill install` das Addon
      bereits registriert hat).
    - **When to Use:** Always (new features · bug fixes · refactoring · behavior changes) /
      Exceptions (ask human: prototypes · generated code · config).
    - **The Iron Law:** `NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST` (verbatim).
    - **Red → Green → Refactor:** je Schritt der konkrete `ctx_md_render(..., phase=…)`-Aufruf.
    - **Testing Anti-Patterns:** Upstream-Trigger („When adding mocks or test utilities, …")
      + 3-Bullet-Preview + `ctx_md_render(..., companion="testing-anti-patterns")`.
    - **`phase` XOR `companion`** + **Final Rule** (verbatim Upstream).
- `description`-Frontmatter bleibt unverändert (Trigger-only, identisch zum Upstream).

### 5.6 Phasen-Wegweiser („next:"-Zeiger, E7)

- Jeder gerenderte Phasen-Block in `body.lmd.md` schließt mit einem `next:`-Zeiger:
  `red`→„next: render `green`", `green`→„next: render `refactor`",
  `refactor`→„next: render `red` for the next behavior", `rationalizations`→zurück zur
  aktiven Phase. Führt die Session ohne erneutes Stub-Lesen. Byte-stabil, deterministisch (#498).

## 6. Tests (TDD — RED zuerst)

Alle Tests via `cargo nextest run`. RED vor Implementation beobachten.

1. **Companion-Render:** `companion="testing-anti-patterns"` rendert alle 5 Anti-Pattern-Marker
    + Quick-Reference-Marker.
2. **Disziplin-Include:** `@include test-first-core` im Companion aufgelöst — Iron-Law-Marker
   (z. B. „NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST") present.
3. **Unbekannter Companion** → Fehler.
4. **`phase` + `companion` gleichzeitig** → Fehler (E5).
5. **CLI == MCP:** `render --companion …` byte-identisch zum MCP-Handler-Output.
6. **Fragment-Consistency-Gate:** built-in `include_str!`-Seed == on-disk-Seed (für den
   Companion erweitern).
7. **Pointer-Auflösung:** `rationalizations`-Render enthält die konkrete `companion`-Render-Anweisung,
   nicht mehr den „Spec #2"-Platzhalter.
8. **Stub-Orientierung (E6):** `SKILL.md` enthält Iron-Law-Marker, „Where this runs",
   When-to-Use, alle vier `phase=…`-Aufrufe, den Companion-Trigger samt
   `companion="testing-anti-patterns"`-Aufruf und die Final Rule; `description`-Frontmatter
   unverändert.
9. **Phasen-Wegweiser (E7):** jeder gerenderte Phasen-Block endet mit dem korrekten
   `next:`-Zeiger (`red`→`green`→`refactor`→`red`).

## 7. Determinismus & Quality Bar (#498)

- Render-Output = deterministische Funktion von (Companion-Inhalt, CRP-Mode, Task) —
  keine Timestamps/Counter/Random im Output-Body.
- Embedded Seed byte-identisch zur on-disk-Datei (Fragment-Consistency-Gate green).
- Zero clippy warnings, alle Tests grün; `cargo fmt` vor jedem `git add`.
- COVERAGE-Dimension: Companion-Port als Audit-Eintrag ergänzen.

## 8. Scope-Abgrenzung

- **In diesem Spec:** Companion-Render-Param (`ctx_md_render` + CLI `--companion`),
  Registry-Companion-Spalte, `testing-anti-patterns.lmd.md`-Seed (kondensiert + sprachneutral,
  mit `@include test-first-core`), Pointer-Auflösung in `rationalizations`,
  **SKILL.md-Stub-Ersetzung** (Upstream-Adaption, E6) + **Companion-Trigger im Stub** (E7),
  **Phasen-Wegweiser** (`next:`-Zeiger, E7), Gates (Fragment-Consistency erweitert,
  Param-Disjunktion, CLI==MCP, Stub-Orientierung, Wegweiser), COVERAGE-Eintrag.
- **Nicht hier (→ restlicher Spec #2):** `lmd-writing-skills`; weitere Companions über das
  eine Target hinaus (Schema bleibt via `Option` erweiterbar, keine generische Discovery-UI).
- **→ Spec #3:** `lmd-brainstorm`-Re-Anchor auf native Konsumption.
- **Verworfen (YAGNI):** 5. Off-Sequence-Phase (E1-B); eigenständige `lmd-testing-anti-patterns`-Skill
  (E1-C); Rust-Übersetzung der Beispiele (E2).
