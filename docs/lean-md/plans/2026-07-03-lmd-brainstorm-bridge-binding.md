@lean-md
consumer: ai

@var test_cmd default="cargo nextest run"
@import .lean-ctx/lean-md/plan-recipes /

# lmd-brainstorm — Bridge-Bindung an lean-md/lean-ctx angleichen (Implementierungsplan)

**Spec:** `docs/lean-md/specs/2026-07-03-lmd-brainstorm-bridge-binding-design.md`
**Datum:** 2026-07-03

## Goal

`lmd-brainstorm` bindet die Design-Zeit-Bridges konsistent: `@find` wird in der
`explore`-Prosa real gewoben (heute nur COVERAGE-registriert, nicht demonstriert),
`@remember` wird als **Zeiger-Index** präzisiert (kein Plan-Zubringer, kein Voll-
Duplikat), die fehl-registrierte `self-review/review→ctx_review`-COVERAGE-Row wird
entfernt (Kategorienfehler: `ctx_review` ist ein Code-Review-Tool, der Spec-Review
läuft über `@dispatch → spec-reviewer`), und die bewusste Design-Zeit-Auslassung der
Change-Gates (`smells`/`review`/`reformat`) wird als scoped Code-Kommentar transparent
gemacht. Kein neues `@smells`/`@review`-Weave (Change-Gates ohne Design-Zeit-Ziel).

## Architecture

- **`content/skills/lmd-brainstorm/body.lmd.md`** (via `include_str!` als
  `LMD_BRAINSTORM_BODY` in `src/skills.rs`) — `explore`-Prosa += `@find`;
  `Documentation`-Prosa: `@remember` auf Zeiger-Form präzisiert.
- **`content/tooling/mcp-tools.lmd.md`** (via `include_str!` in `PROJECT_SEEDS`,
  `src/seeds.rs`) += eine `@find`-Usage-Zeile (strukturell-vs-semantisch), die zugleich
  die „`@search` reicht doch"-Rationalisierung entkräftet.
- **`src/availability.rs`** `COVERAGE` − die `self-review/review`-Row; + scoped
  Code-Kommentar (English) an den brainstorm-Rows (Design-Zeit-Auslassung). `GAP_LIST`
  **unverändert**.
- **`src/bin/lean_md.rs`** (Test-Modul) += Render-Smoke-Gate: `explore`-Render enthält
  `@find`.

**Seed-Const-Mechanik (wichtig):** `body.lmd.md` und `tooling/mcp-tools.lmd.md` sind
über `include_str!` gebunden (`LMD_BRAINSTORM_BODY` in `src/skills.rs`, `PROJECT_SEEDS`
in `src/seeds.rs`). `include_str!` zieht die Datei zur Compile-Zeit ein → **es genügt,
die Seed-Datei zu ändern**; die Const ist automatisch byte-synchron, keine handgepflegte
Kopie zum Nachziehen. Die materialisierte `.lean-ctx/lean-md/tooling/mcp-tools.lmd.md`
(Install-Artefakt, gitignored, absent-only) wird für die Live-Autoren-Ansicht in Task 2
refresht — kein Gate hängt daran.

## Global Constraints (jede Task inkludiert dies implizit)

- **Tests immer** `cargo nextest run` — **nie** `cargo test`.
- **Shell — kein** `&&`/`||`/`;`-Chaining: jede Anweisung ist eine eigene Invocation.
  Conditional-Gates in separate Schritte mit explizitem „Expected:" auflösen.
- **Vor jedem `git add` einer `.rs`-Datei:** `cargo fmt` (Standalone-Crate,
  `Cargo.toml` + `src/` im Repo-Root). Für reine `content/*.lmd.md`-Seeds (Markdown)
  ist kein fmt nötig.
- **`cargo clippy -- -D warnings`** muss sauber bleiben.
- **#498-Determinismus:** Body-Prosa bleibt guidance (inline-`code`-Directives, nie
  zeilenführend → kein Execute-at-Render). Kein Timestamp/Counter/Random im Output.
- **Sprache:** Chat/Plan/Spec = Deutsch mit Umlauten; **aller gewobene Content
  (Body-Prosa, `tooling/mcp-tools`-Zeile) und jeder Code-Kommentar = Englisch.**
- **Rendern der lmd-Skills in diesem Dev-Repo** läuft direkt über die CLI:
  `cargo run -q --bin lean-md -- render ...` — **nicht** über `ctx_md_render`/MCP.
- **Reihenfolge (Spec §6): A vor B** — erst der beobachtete RED-Fail (Task 1)
  rechtfertigt den Body-Edit (Task 2), dann zieht die Durchsetzung (Task 4) nach.

---

@phase "task-1"

## Task 1: RED-Baseline — Verhaltens-Pressure-Test vor dem `@find`-Weave (Spec §6-A)

**Art:** Plan-Task (bewertet LLM-Output), **kein** `cargo`-Gate. MUSS vor Task 2 laufen —
er friert die Ist-Beobachtung ein (die `explore`-Prosa nennt nur `@list/@search/@read`,
kein `@find`) als beweisbare Baseline. Iron Law (`lmd-writing-skills`): kein Skill-Edit
ohne zuerst beobachteten Fehlschlag.

**Schritt 1 — aktuellen `explore`-Body als Baseline festhalten.** Render die Phase, die
Task 2 ändern wird:

    `cargo run -q --bin lean-md -- render --skill lmd-brainstorm --phase explore --consumer=ai`

**Expected:** Der Explore-Bullet lautet „Explore with `@list`/`@search`/`@read`
(ctx_tree / ctx_search / ctx_read) before asking questions; gauge … `@graph` / `@impact`."
— **`@find` kommt nicht vor.**

**Schritt 2 — Pressure-Szenario definieren.** Ein Brainstorming-Agent soll eine Codebase
explorieren, in der die ankernde Stelle **nur semantisch** (nicht per Keyword/Pfad)
findbar ist. Konkret (English task brief für den Subagenten):

    You are brainstorming a design. You must locate the code that implements
    "the place where inbound tool results get their bytes stabilized" — the exact
    identifier/path is unknown; only the INTENT is known. Follow the rendered
    lmd-brainstorm explore guidance above. Which lmd directive do you reach for?

**Schritt 3 — Fehlschlag beobachten.** Dispatch einen frischen Subagenten mit der
Baseline-`explore`-Guidance aus Schritt 1 + dem Brief aus Schritt 2 (via `@dispatch`
oder `Agent`-Tool). **Expected (RED):** der Agent greift zu `@search`/`@list` (Keyword)
oder gibt auf — er reicht **nicht** zu `@find`, weil die Guidance es nicht nennt.
Beobachtung wörtlich festhalten.

**Schritt 4 — Baseline durabel sichern.**

    Run: `@remember` — record a compact pointer: "RED baseline: lmd-brainstorm explore
    guidance omits @find; agent falls back to @search on semantic-locate task. See
    docs/lean-md/plans/2026-07-03-lmd-brainstorm-bridge-binding.md Task 1."

**Deliverable:** dokumentierter, beobachteter Fehlschlag (kein `@find`-Reach ohne Weave).
Kein Commit (reines Beobachtungs-Artefakt).

@phase-end

---

@phase "task-2"

## Task 2: GREEN+REFACTOR — `@find` in `explore` weben + Usage-Referenz (Spec §3.1/§4)

**Files:** `content/skills/lmd-brainstorm/body.lmd.md`,
`content/tooling/mcp-tools.lmd.md`, `src/bin/lean_md.rs` (Test-Modul).
**Interfaces:** `explore`-Render enthält danach `@find`; `tooling/mcp-tools` trägt eine
`@find`-Usage-Zeile. **Content = English.**

**Schritt 1 — Render-Smoke-Gate zuerst schreiben (GREEN-Gate).** In `src/bin/lean_md.rs`
im `#[cfg(test)] mod tests` neben `skill_render_is_byte_stable_and_isolated` (Anker
`src/bin/lean_md.rs:571`) diesen Test einfügen (new code, verbatim):

    #[test]
    fn brainstorm_explore_weaves_find() {
        // §3.1: the explore guidance must demonstrate @find (semantic locate),
        // not just @search — COVERAGE registers explore/find→ctx_semantic_search.
        let jail = std::path::PathBuf::from(".");
        let out = render_skill("lmd-brainstorm", Some("explore"), None, None, jail).unwrap();
        assert!(
            out.contains("@find"),
            "explore guidance must weave @find, got: {out}"
        );
    }

**Schritt 2 — Test laufen, Fehlschlag bestätigen (RED des Code-Gates).**

    @call test("brainstorm_explore_weaves_find")

**Expected:** FAIL — der aktuelle Body enthält kein `@find`.

**Schritt 3 — `@find` in die `explore`-Prosa weben.** In
`content/skills/lmd-brainstorm/body.lmd.md` den Explore-Bullet (Anker: die Zeile
„Explore with `@list`/`@search`/`@read` (ctx_tree / ctx_search / ctx_read) before")
ersetzen durch (English, guidance — inline-`code`, nie zeilenführend):

    - Explore with `@list`/`@search`/`@read` (structural — ctx_tree / ctx_search /
      ctx_read) and `@find` (semantic locate — ctx_semantic_search) before asking
      questions; gauge a change's blast radius with `@graph` / `@impact`.

**Schritt 4 — Usage-Referenz-Zeile in `tooling/mcp-tools` ergänzen.** In
`content/tooling/mcp-tools.lmd.md` direkt nach der bestehenden `Graph/impact`-Mapping-Zeile
(die `graph`/`impact`/`find`-Bullet, endet auf `ctx_semantic_search`) diese Usage-Zeile
anfügen (English):

    - `@find <intent>` — semantic locate via ctx_semantic_search. Use at design/task
      time to find the spot a design or task anchors to; for structural (keyword/path)
      lookup use `@search` instead.

**Koordination (Spec §4):** Falls die Schwester-Arbeit (writing-plans-binding) diese
Zeile bereits eingetragen hat, ist Schritt 4 ein **Idempotenz-Check** — Präsenz
verifizieren, nicht doppelt einfügen.

**Schritt 5 — materialisierte Kopie refreshen (Live-Autoren-Ansicht).** Die
absent-only-Materialisierung überschreibt nicht; alte Kopie entfernen, damit der nächste
Install/Render sie neu schreibt:

    Run: `@edit` — delete stale `.lean-ctx/lean-md/tooling/mcp-tools.lmd.md` so the
    next materialize regenerates it from the updated seed (gitignored install artifact;
    no gate depends on it).

**Schritt 6 — Render-Smoke grün + Byte-Stabilität prüfen.**

    @call test("brainstorm_explore_weaves_find")

**Expected:** PASS. Zusätzlich manuell:
`cargo run -q --bin lean-md -- render --skill lmd-brainstorm --phase explore --consumer=ai`
**Expected:** Bullet enthält jetzt `@find`; zweiter identischer Aufruf → byte-identisch (#498).

**Schritt 7 — Pressure-Test wiederholen (GREEN, Spec §6-A).** Dispatch denselben Brief
wie Task 1 Schritt 3, jetzt mit der gewobenen `explore`-Guidance. **Expected:** der Agent
greift zu `@find`. Die strukturell-vs-semantisch-Zeile aus Schritt 4 entkräftet die
„`@search` reicht doch"-Rationalisierung (REFACTOR).

**Schritt 8 — Commit.**

    @call commit("content/skills/lmd-brainstorm/body.lmd.md content/tooling/mcp-tools.lmd.md src/bin/lean_md.rs", "feat(lmd-brainstorm): weave @find into explore + @find usage-reference; render-smoke gate")

@phase-end

---

@phase "task-3"

## Task 3: `@remember` auf Zeiger-Form präzisieren (Spec §3.3/§2.1)

**Art:** Guidance-Schärfung (kein neuer Directive-Reach) → Render-Smoke + spec-reviewer,
**kein** eigener RED (Spec §6-A). **Files:** `content/skills/lmd-brainstorm/body.lmd.md`
(`Documentation`-Phase). **Content = English.**

**Schritt 1 — Documentation-Prosa ersetzen.** In `content/skills/lmd-brainstorm/body.lmd.md`
die Zeile (Anker: „Use `@edit` to write the spec file and `@remember` to record durable
design") ersetzen durch (English):

    - Use `@edit` to write the spec file. Use `@remember` to record a compact
      pointer — the decision, a one-line gist, and the spec path — NOT a duplicate of
      the spec; the committed spec file stays the source of truth, the pointer just
      makes the decision findable across sessions.

**Schritt 2 — Render-Smoke.**

    `cargo run -q --bin lean-md -- render --skill lmd-brainstorm --phase write-spec --consumer=ai`

**Expected:** die Documentation-Prosa nennt „compact pointer" + „NOT a duplicate";
zweiter Aufruf byte-identisch (#498). (Der Skill-Body wird via `include_str!` embedded →
Const auto-synchron, kein separater Sync-Schritt.)

**Schritt 3 — Commit.**

    @call commit("content/skills/lmd-brainstorm/body.lmd.md", "docs(lmd-brainstorm): @remember as compact pointer-index, not a spec duplicate")

@phase-end

---

@phase "task-4"

## Task 4: COVERAGE bereinigen + scoped Kommentar (Spec §3.2/§5)

**Files:** `src/availability.rs`. **Interfaces:** `COVERAGE` ohne die
`self-review/review→ctx_review`-Row; scoped English-Kommentar dokumentiert die
Design-Zeit-Auslassung. `GAP_LIST` unverändert.

**Schritt 1 — die fehl-registrierte Row entfernen.** In `src/availability.rs` (Anker:
`availability.rs:17`) die Zeile

    ("lmd-brainstorm", "self-review", "review", "ctx_review"),

**löschen** und an ihre Stelle einen scoped English-Kommentar setzen (new code, verbatim):

    // brainstorm is design-time: it produces a spec, not a diff. The change-gates
    // (ctx_smells, ctx_review, ctx_refactor reformat) are deliberately NOT in this
    // path — they are task-time gates covered by lmd-writing-plans execution. The
    // spec review itself runs via the spec-reviewer @dispatch below, not ctx_review
    // (a code-review tool, not a prose-spec reviewer). Documented here for
    // transparency (not a silent hole); GAP_LIST stays for globally-unused tools.

**Warum kein GAP_LIST-Eintrag (Spec §5):** `smells`/`review`/`reformat` werden zur
Task-Zeit von der writing-plans-COVERAGE genutzt; sie in die flache, byte-gepinnte
`GAP_LIST` (global-ungenutzt) zu legen stünde im Widerspruch. Der Kommentar drückt die
skill-scoped Semantik aus.

**Schritt 2 — `cargo fmt` (vor `git add` der `.rs`-Datei).**

    `cargo fmt`

**Schritt 3 — Gates prüfen.** Alle Availability-Gates müssen grün bleiben:

    @call test("availability")

**Expected:** PASS — `every_covered_directive_is_registered` (eine Row weniger),
`coverage_carries_skill_dimension`, `coverage_carries_brainstorm_companion_and_dispatch_row`
(prüft `self-review/dispatch` + `spec-reviewer/dispatch`, **nicht** die entfernte
review-Row), `gap_list_is_byte_stable` (`GAP_LIST` unverändert → weiterhin
`ctx_benchmark\nctx_package\nctx_provider\n`).

**Schritt 4 — Clippy sauber.**

    `cargo clippy -- -D warnings`

**Expected:** keine Warnings.

**Schritt 5 — Commit.**

    @call commit("src/availability.rs", "refactor(availability): drop mis-registered brainstorm self-review/review row; document design-time omission")

@phase-end

---

@phase "task-5"

## Task 5: Full-Gate + Reference-Closure

**Art:** Verifikations-Task (kein neuer Code). Stellt sicher, dass die Gesamtänderung
grün und die Bindung konsistent ist.

**Schritt 1 — komplette Testsuite.**

    @call test("")

Hinweis: leerer Filter → volle Suite. **Expected:** alle Tests grün, inkl.
`brainstorm_explore_weaves_find` (Task 2) und der Availability-Gates (Task 4).

**Schritt 2 — Clippy gesamt.**

    `cargo clippy --all-targets -- -D warnings`

**Expected:** keine Warnings.

**Schritt 3 — Render-Determinismus final.** Beide geänderten Phasen zweimal rendern,
Ausgaben müssen byte-identisch sein (#498):

    `cargo run -q --bin lean-md -- render --skill lmd-brainstorm --phase explore --consumer=ai`
    `cargo run -q --bin lean-md -- render --skill lmd-brainstorm --phase write-spec --consumer=ai`

**Expected:** `explore` enthält `@find`; `write-spec` enthält die Zeiger-Prosa; jeweils
byte-stabil.

**Schritt 4 — Konsistenz-Grep.** Kein Rest der entfernten Row und kein Deutsch im
gewobenen Content:

    Run: `@search "self-review.*review.*ctx_review"` over `src/availability.rs`
    — Expected: no hits (row removed).

**Schritt 5 — durable Abschluss-Notiz.**

    Run: `@remember` — record a compact pointer: "lmd-brainstorm bridge-binding done:
    @find woven (explore) + usage-ref, @remember=pointer-index, review-row removed,
    design-time omission documented. See plan Task 1-5."

**Deliverable:** grüne Full-Gate, konsistente Bindung, keine offenen Rationalisierungen.

@phase-end
