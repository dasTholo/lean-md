# testing-anti-patterns-Companion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Den `testing-anti-patterns`-Companion der Skill `lmd-test-driven-development` liefern — Out-of-band-Render-Maschinerie (`companion`-Param + Registry-Spalte), den portierten Seed, Pointer-Auflösung, treuen Upstream-Stub und Phasen-Wegweiser.

**Architecture:** Companion-Bodies sind binary-embedded (`include_str!`) und werden über einen neuen flachen Render-Pfad `render_companion(skill, companion, …)` gerendert (kein Phasen-Capture — ein Block). Eine `COMPANIONS`-Registry-Tabelle keyed auf `(skill, companion)` spiegelt die bestehende `SKILLS`-Tabelle. `phase` XOR `companion` wird an der CLI/MCP-Grenze erzwungen. Der Companion zieht `@include test-first-core` (selbsttragende Iron-Law-Disziplin bei isoliertem Laden). CLI und MCP treffen denselben `render_companion`-Handler → byte-identisch (#498).

**Tech Stack:** Rust (standalone crate `lean_md`, lib + bin), `cargo nextest`, `serde_json` (MCP JSON-RPC), `include_str!`-Embedding.

## Global Constraints

- **Tests:** immer `cargo nextest run`, **nie** `cargo test`. Crate standalone; `Cargo.toml` + `src/` im Repo-Root → Kommandos laufen aus dem Repo-Root (kein `cd`, kein `--manifest-path`).
- **Shell:** kein `&&`/`||`/`;`-Chaining — jedes Kommando ist eine eigene Invocation.
- **Vor jedem `git add`** (je geänderte Datei): `cargo fmt`.
- **No worktrees** — direkt auf Branch `feat-lmd-v2`.
- **Determinismus (#498):** Render-Output = deterministische Funktion von (Inhalt, Mode, CRP, Task) — keine Timestamps/Counter/Random; embedded Seed byte-identisch zur on-disk-Datei (Fragment-Consistency-Gate).
- **Quality:** Zero clippy warnings, alle Tests grün. Code + Code-Kommentare englisch.
- **Non-Rust-Edits** über `mcp__lean-ctx__ctx_edit`; `.rs`-Symboledits ggf. `mcp__lean-ctx__ctx_refactor`. Native `Write` für neue Dateien ok.

---

### Task 1: Companion-Seed + Embedding + Registry-Lookup

**Files:**
- Create: `content/skills/lmd-test-driven-development/companions/testing-anti-patterns.lmd.md`
- Modify: `src/skills.rs` (Embed-Const + `COMPANIONS`-Tabelle + `companion_body()`; nahe `SKILLS`/`skill_body`, Zeilen ~14–34)
- Test: `src/skills.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `include_str!`-Embedding-Muster (bestehend, `LMD_TEST_DRIVEN_DEVELOPMENT_BODY`), `test-first-core`-Fragment (bestehend, `@include`).
- Produces: `pub fn companion_body(skill: &str, companion: &str) -> Option<&'static str>`; `const COMPANIONS: &[(&str, &str, &str)]` = `(skill, companion_name, body)`.

- [ ] **Step 1: Companion-Seed schreiben**

`content/skills/lmd-test-driven-development/companions/testing-anti-patterns.lmd.md`:

```markdown
# Testing Anti-Patterns (lmd companion — load when writing/changing tests or adding mocks)

@include test-first-core

Test what the code does, not what the mocks do. Mocks isolate; they are not the thing under test.

## The Iron Laws
1. NEVER test mock behavior.
2. NEVER add test-only methods to production code.
3. NEVER mock without understanding the dependency.

## Anti-Pattern 1 — Testing mock behavior
Asserting that a mock exists proves the mock works, not that the code works. Never assert on `*-mock` ids/handles.
Gate: BEFORE asserting on a mock — "real behavior, or just mock existence?" IF existence: STOP — delete the assertion or unmock the component.

## Anti-Pattern 2 — Test-only methods in production
A method only ever called from tests (cleanup/`destroy`-style) pollutes the production type and can fire in production.
Gate: BEFORE adding a method — "only used by tests?" IF yes: STOP — put it in test utilities. "Does this type own this resource's lifecycle?" IF no: STOP — wrong type.

## Anti-Pattern 3 — Mocking without understanding
Mocking away a method whose side effect the test depends on makes the test pass (or fail) for the wrong reason.
Gate: BEFORE mocking — "what side effects does the real method have, and does the test depend on them?" IF yes: mock at the lower (slow/external) level, not the method the test needs. IF unsure: run with the real impl first, then mock minimally. Red flag: "I'll mock this to be safe."

## Anti-Pattern 4 — Incomplete mocks
A partial mock with only the fields you know about fails silently when downstream code reads an omitted field.
Gate: BEFORE building a mock response — mirror the COMPLETE real structure, every field the system may consume. If uncertain, include all documented fields.

## Anti-Pattern 5 — Integration tests as afterthought
"Implementation complete, tests later" is not done. Testing is part of implementation, not an optional follow-up.
Gate: failing test first → minimal code → refactor → THEN claim complete.

## Quick Reference
| Anti-pattern | Fix |
| Assert on mock elements | Test real behavior or unmock it |
| Test-only methods in production | Move to test utilities |
| Mock without understanding | Understand dependencies first, mock minimally |
| Incomplete mocks | Mirror the real structure completely |
| Tests as afterthought | TDD — tests first |
| Over-complex mocks | Prefer integration tests with real components |

## Red Flags
- Assertions checking for `*-mock` ids.
- Methods only called from test files.
- Mock setup is >50% of the test.
- The test fails when you remove the mock.
- Can't explain why the mock is needed; "mocking just to be safe".

Bottom line: mocks are tools to isolate, not things to test. If TDD reveals you are testing a mock, test real behavior or question why you are mocking at all.
```

- [ ] **Step 2: Failing test schreiben** — in `src/skills.rs` im `mod tests`:

```rust
    #[test]
    fn companion_registry_resolves_testing_anti_patterns() {
        assert!(
            companion_body("lmd-test-driven-development", "testing-anti-patterns").is_some()
        );
        assert!(companion_body("lmd-test-driven-development", "nope").is_none());
        assert!(companion_body("nope", "testing-anti-patterns").is_none());
    }

    #[test]
    fn companion_body_matches_seed_file_on_disk() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let disk = std::fs::read_to_string(
            std::path::Path::new(manifest).join(
                "content/skills/lmd-test-driven-development/companions/testing-anti-patterns.lmd.md",
            ),
        )
        .unwrap();
        assert_eq!(
            companion_body("lmd-test-driven-development", "testing-anti-patterns").unwrap(),
            disk,
            "embedded companion drifted from seed file"
        );
    }
```

- [ ] **Step 3: Run test — erwartet FAIL**

Run: `cargo nextest run -E 'test(companion_registry_resolves_testing_anti_patterns)'`
Expected: FAIL — `companion_body` existiert nicht (Compile-Fehler).

- [ ] **Step 4: Implementierung** — in `src/skills.rs` nach `LMD_TEST_DRIVEN_DEVELOPMENT_BODY` (Zeile ~15) das Embed-Const, und nach `SKILLS` (Zeile ~24) Tabelle + Lookup:

```rust
const LMD_TESTING_ANTI_PATTERNS_COMPANION: &str = include_str!(
    "../content/skills/lmd-test-driven-development/companions/testing-anti-patterns.lmd.md"
);

/// Registry of embedded companions (skill, companion name → embedded body).
/// Out-of-band on-demand references attached to a skill (Spec #2, E1/A).
const COMPANIONS: &[(&str, &str, &str)] = &[(
    "lmd-test-driven-development",
    "testing-anti-patterns",
    LMD_TESTING_ANTI_PATTERNS_COMPANION,
)];

/// Embedded body for a known `(skill, companion)` pair, or `None` if unknown.
pub fn companion_body(skill: &str, companion: &str) -> Option<&'static str> {
    COMPANIONS
        .iter()
        .find(|(s, c, _)| *s == skill && *c == companion)
        .map(|(_, _, body)| *body)
}
```

- [ ] **Step 5: Run tests — erwartet PASS**

Run: `cargo nextest run -E 'test(companion_registry_resolves) + test(companion_body_matches_seed)'`
Expected: PASS (beide Tests).

- [ ] **Step 6: fmt + Commit**

```bash
cargo fmt
git add content/skills/lmd-test-driven-development/companions/testing-anti-patterns.lmd.md src/skills.rs
git commit -m "feat(skills): embed testing-anti-patterns companion + COMPANIONS registry"
```

---

### Task 2: `render_companion` — flacher Render-Pfad + `CompanionNotFound`

**Files:**
- Modify: `src/skills.rs` (`SkillRenderError` erweitern; `render_full_source`-Helper extrahieren; `render_companion` hinzufügen)
- Test: `src/skills.rs` (`mod tests`)

**Interfaces:**
- Consumes: `companion_body()` (Task 1), `parse_header`, `EngineContext::new`, `render_body`, `crate::skill_vars::{load_vars, scan_var_decls}` (alle bestehend, wie in `render_skill`).
- Produces: `pub fn render_companion(skill: &str, companion: &str, consumer: Option<Consumer>, crp: Option<CrpMode>, jail_root: PathBuf) -> Result<String, SkillRenderError>`; `SkillRenderError::CompanionNotFound(String)`.

- [ ] **Step 1: Failing test schreiben** — `src/skills.rs` `mod tests`:

```rust
    #[test]
    fn companion_renders_all_anti_pattern_markers() {
        let out = render_companion(
            "lmd-test-driven-development",
            "testing-anti-patterns",
            None,
            None,
            PathBuf::from("."),
        )
        .unwrap();
        for marker in [
            "Anti-Pattern 1",
            "Anti-Pattern 2",
            "Anti-Pattern 3",
            "Anti-Pattern 4",
            "Anti-Pattern 5",
            "Quick Reference",
            "Red Flags",
        ] {
            assert!(out.contains(marker), "companion missing '{marker}': {out}");
        }
    }

    #[test]
    fn companion_includes_test_first_core_iron_law() {
        let out = render_companion(
            "lmd-test-driven-development",
            "testing-anti-patterns",
            None,
            None,
            PathBuf::from("."),
        )
        .unwrap();
        assert!(
            out.contains("NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST"),
            "companion must @include test-first-core (Iron Law marker): {out}"
        );
    }

    #[test]
    fn unknown_companion_errors() {
        let err = render_companion(
            "lmd-test-driven-development",
            "does-not-exist",
            None,
            None,
            PathBuf::from("."),
        )
        .unwrap_err();
        assert!(matches!(err, SkillRenderError::CompanionNotFound(_)));
    }

    #[test]
    fn companion_render_is_deterministic() {
        let jail = PathBuf::from(".");
        let a = render_companion(
            "lmd-test-driven-development",
            "testing-anti-patterns",
            None,
            None,
            jail.clone(),
        )
        .unwrap();
        let b = render_companion(
            "lmd-test-driven-development",
            "testing-anti-patterns",
            None,
            None,
            jail,
        )
        .unwrap();
        assert_eq!(a, b, "render_companion must be deterministic (#498)");
    }
```

- [ ] **Step 2: Run test — erwartet FAIL**

Run: `cargo nextest run -E 'test(companion_renders_all_anti_pattern_markers)'`
Expected: FAIL — `render_companion`/`CompanionNotFound` existieren nicht (Compile-Fehler).

- [ ] **Step 3: Implementierung**

3a — `SkillRenderError` (Zeile ~36) um Variante erweitern:

```rust
#[derive(Debug)]
pub enum SkillRenderError {
    UnknownSkill(String),
    PhaseNotFound(String),
    CompanionNotFound(String),
}
```

3b — `Display`-`match` (Zeile ~44) um den Arm ergänzen:

```rust
            SkillRenderError::CompanionNotFound(c) => write!(f, "COMPANION_NOT_FOUND '{c}'"),
```

3c — gemeinsamen Helper extrahieren (private fn, oberhalb `render_skill`). Den `None`-Arm von `render_skill` darauf umstellen, damit DRY:

```rust
/// Render a full body source flat (no phase capture): header overrides + the
/// `@var` pre-pass (default-if-absent), then a single `render_body` pass.
/// Shared by `render_skill` (phase=None) and `render_companion`.
fn render_full_source(
    src: &str,
    consumer: Option<Consumer>,
    crp: Option<CrpMode>,
    jail_root: PathBuf,
) -> String {
    let (mut header, body) = parse_header(src);
    if let Some(c) = consumer {
        header.consumer = c;
    }
    if let Some(m) = crp {
        header.crp = m;
    }
    let ctx = Rc::new(EngineContext::new(header, jail_root));
    ctx.vars_seed(crate::skill_vars::load_vars(&ctx.jail_root));
    for decl in crate::skill_vars::scan_var_decls(body) {
        ctx.var_set_default(&decl.name, &decl.default);
    }
    render_body(&ctx, body)
}
```

3d — `render_companion` hinzufügen (nach `render_skill`):

```rust
/// Render a skill's on-demand companion as one flat block (no phase sequence).
/// Out-of-band like the body; embedded-only (no overlay layer — YAGNI).
pub fn render_companion(
    skill: &str,
    companion: &str,
    consumer: Option<Consumer>,
    crp: Option<CrpMode>,
    jail_root: PathBuf,
) -> Result<String, SkillRenderError> {
    let src = companion_body(skill, companion)
        .ok_or_else(|| SkillRenderError::CompanionNotFound(format!("{skill}/{companion}")))?;
    Ok(render_full_source(src, consumer, crp, jail_root))
}
```

3e — in `render_skill` den `None`-Arm auf den Helper umstellen (kein Verhaltenswechsel; Overlay bleibt vorgelagert). Ersetze:

```rust
    match phase {
        None => Ok(render_body(&ctx, body)),
        Some(p) => {
```

ist hier nicht 1:1 möglich (Helper baut eigenen ctx). **Stattdessen** den `None`-Arm so lassen wie er ist — der Helper wird NUR von `render_companion` genutzt. (DRY-Hinweis: `render_skill` muss seinen ctx für `capture_phase_bodies` behalten; eine Vereinheitlichung würde mehr verändern als sie spart → bewusst getrennt.)

> Hinweis Implementer: 3e ist ein **No-op am bestehenden `render_skill`** — nur 3a–3d sind echte Änderungen. Der Helper `render_full_source` wird in Task 2 ausschließlich von `render_companion` konsumiert. (`render_skill` unverändert lassen vermeidet Regress an den 12 bestehenden Tests.)

- [ ] **Step 4: Run tests — erwartet PASS**

Run: `cargo nextest run -E 'test(companion)'`
Expected: PASS (alle companion-Tests aus Task 1 + 2).

- [ ] **Step 5: Voller Lauf (Regress-Check)**

Run: `cargo nextest run`
Expected: PASS — alle bestehenden `render_skill`-Tests weiterhin grün.

- [ ] **Step 6: fmt + Commit**

```bash
cargo fmt
git add src/skills.rs
git commit -m "feat(skills): render_companion flat path + CompanionNotFound"
```

---

### Task 3: CLI `--companion` + `phase` XOR `companion`

**Files:**
- Modify: `src/bin/lean_md.rs` (`RenderArgs` + `parse_render_flags` + `cmd_render` + Usage)
- Test: `src/bin/lean_md.rs` (`mod tests`)

**Interfaces:**
- Consumes: `render_companion` (Task 2), `render_skill` (bestehend).
- Produces: CLI-Flag `--companion <name>`; XOR-Fehler bei `--phase` + `--companion`.

- [ ] **Step 1: Failing test schreiben** — `src/bin/lean_md.rs` `mod tests`:

```rust
    #[test]
    fn render_flags_parse_companion() {
        let a = parse_render_flags(&[
            "--skill".to_string(),
            "lmd-test-driven-development".to_string(),
            "--companion".to_string(),
            "testing-anti-patterns".to_string(),
        ]);
        assert_eq!(a.skill.as_deref(), Some("lmd-test-driven-development"));
        assert_eq!(a.companion.as_deref(), Some("testing-anti-patterns"));
        assert_eq!(a.phase, None);
    }
```

- [ ] **Step 2: Run test — erwartet FAIL**

Run: `cargo nextest run -E 'test(render_flags_parse_companion)'`
Expected: FAIL — Feld `companion` existiert nicht (Compile-Fehler).

- [ ] **Step 3: Implementierung**

3a — `RenderArgs` (Zeile ~62) um Feld erweitern:

```rust
#[derive(Debug, Default, PartialEq)]
struct RenderArgs {
    file: Option<String>,
    consumer: Option<Consumer>,
    crp: Option<CrpMode>,
    out: Option<String>,
    skill: Option<String>,
    phase: Option<String>,
    companion: Option<String>,
}
```

3b — `parse_render_flags` (nach dem `"--phase"`-Arm, Zeile ~99) Arm ergänzen:

```rust
            "--companion" => {
                i += 1;
                a.companion = rest.get(i).cloned();
            }
```

3c — `cmd_render` (Zeile ~135) den Skill-Zweig auf XOR + Dispatch umstellen. Ersetze den Block ab `if let Some(skill) = a.skill.as_deref() {` bis zum `return;` durch:

```rust
    if let Some(skill) = a.skill.as_deref() {
        if a.phase.is_some() && a.companion.is_some() {
            eprintln!("lean-md render: --phase and --companion are mutually exclusive");
            std::process::exit(1);
        }
        let jail = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let result = match a.companion.as_deref() {
            Some(companion) => render_companion(skill, companion, a.consumer, a.crp, jail),
            None => render_skill(skill, a.phase.as_deref(), a.consumer, a.crp, jail),
        };
        match result {
            Ok(rendered) => match a.out {
                Some(out) => {
                    if let Err(e) = std::fs::write(&out, &rendered) {
                        eprintln!("lean-md render: write {out}: {e}");
                        std::process::exit(1);
                    }
                }
                None => print!("{rendered}"),
            },
            Err(e) => {
                eprintln!("lean-md render: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
```

3d — Import ergänzen (Zeile ~16, neben `use lean_md::skills::render_skill;`):

```rust
use lean_md::skills::{render_companion, render_skill};
```

(die bestehende Einzel-`use`-Zeile entsprechend ersetzen).

3e — Usage-Text (Zeile ~121) die render-Zeile ergänzen:

```rust
                 \n  render <file.lmd.md|--skill NAME [--phase P | --companion C]> [--consumer=human|ai] [--crp=off|compact|tdd] [-o out.md]\
```

- [ ] **Step 4: Run tests — erwartet PASS**

Run: `cargo nextest run -E 'test(render_flags_parse_companion) + test(render_flags_parse_skill_and_phase)'`
Expected: PASS.

- [ ] **Step 5: Manuelle Smoke (optional, nicht blockierend)**

Run: `cargo run -q -- render --skill lmd-test-driven-development --companion testing-anti-patterns`
Expected: gerenderter Companion-Text mit „Anti-Pattern 1" … „Red Flags".

- [ ] **Step 6: fmt + Commit**

```bash
cargo fmt
git add src/bin/lean_md.rs
git commit -m "feat(cli): render --companion + phase XOR companion guard"
```

---

### Task 4: MCP `companion`-Param + XOR + CLI==MCP-Gleichheit

**Files:**
- Modify: `src/bin/lean_md.rs` (`tool_defs` inputSchema; `tools/call`-`ctx_md_render`-Handler)
- Test: `src/bin/lean_md.rs` (`mod tests`)

**Interfaces:**
- Consumes: `render_companion` (Task 2), JSON-RPC-Handler (bestehend).
- Produces: MCP-Param `companion` an `ctx_md_render`; XOR-Fehler (`-32602`); byte-identischer Output zu CLI (gleicher `render_companion`-Aufruf).

- [ ] **Step 1: Failing test schreiben** — `src/bin/lean_md.rs` `mod tests`:

```rust
    #[test]
    fn mcp_companion_matches_cli_render_companion() {
        // CLI==MCP (#498): both surfaces call render_companion → byte-identical.
        let jail = std::path::PathBuf::from(".");
        let cli = render_companion(
            "lmd-test-driven-development",
            "testing-anti-patterns",
            None,
            None,
            jail,
        )
        .unwrap();
        assert!(cli.contains("Anti-Pattern 1"));
        assert!(cli.contains("NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST"));
    }

    #[test]
    fn tool_defs_expose_companion_param() {
        let defs = tool_defs();
        let schema = defs[0]["inputSchema"]["properties"].clone();
        assert!(
            schema.get("companion").is_some(),
            "ctx_md_render must expose a 'companion' param: {schema}"
        );
    }
```

- [ ] **Step 2: Run test — erwartet FAIL**

Run: `cargo nextest run -E 'test(tool_defs_expose_companion_param)'`
Expected: FAIL — `companion`-Property fehlt im Schema.

- [ ] **Step 3: Implementierung**

3a — `tool_defs` (Zeile ~211) Property nach `"phase"` ergänzen:

```rust
                    "phase":     { "type": "string", "description": "Render only this named phase of the skill (requires skill; mutually exclusive with companion)" },
                    "companion": { "type": "string", "description": "Render a skill's named companion reference (requires skill; mutually exclusive with phase)" }
```

(Komma hinter der `phase`-Zeile beachten.)

3b — `tools/call` → `"ctx_md_render"` → der `if let Some(skill) = …`-Zweig (Zeile ~358). Nach dem `phase`-Parse die XOR-Prüfung + Dispatch einsetzen. Ersetze:

```rust
                        if let Some(skill) = args.get("skill").and_then(Value::as_str) {
                            let phase = args.get("phase").and_then(Value::as_str);
```

durch:

```rust
                        if let Some(skill) = args.get("skill").and_then(Value::as_str) {
                            let phase = args.get("phase").and_then(Value::as_str);
                            let companion = args.get("companion").and_then(Value::as_str);
```

und ersetze den Aufruf-`match` (Zeile ~374) `match render_skill(skill, phase, consumer, crp, jail) {` durch:

```rust
                            if phase.is_some() && companion.is_some() {
                                return_err = Some(rpc_err(
                                    &id,
                                    -32602,
                                    "phase and companion are mutually exclusive",
                                ));
                            }
                            let result = match companion {
                                Some(c) => render_companion(skill, c, consumer, crp, jail),
                                None => render_skill(skill, phase, consumer, crp, jail),
                            };
                            match result {
```

> Implementer-Hinweis: Der `return_err`-Pfad oben passt nicht zur bestehenden Ausdrucks-Struktur (der `match`-Arm liefert direkt `resp`). **Einfacher, struktur-treu:** die XOR-Prüfung als früh-`if` mit eigenem `rpc_err`-Ausdruck. Konkret den ganzen Aufruf-Block so schreiben:

```rust
                            let jail = std::env::current_dir()
                                .unwrap_or_else(|_| std::path::PathBuf::from("."));
                            if phase.is_some() && companion.is_some() {
                                rpc_err(&id, -32602, "phase and companion are mutually exclusive")
                            } else {
                                let result = match companion {
                                    Some(c) => render_companion(skill, c, consumer, crp, jail),
                                    None => render_skill(skill, phase, consumer, crp, jail),
                                };
                                match result {
                                    Ok(rendered) => rpc_ok(
                                        &id,
                                        json!({ "content": [{ "type": "text", "text": rendered }] }),
                                    ),
                                    Err(e) => rpc_err(&id, -32602, &format!("{e}")),
                                }
                            }
```

(D. h. den bestehenden `let jail = …; match render_skill(...) { Ok => rpc_ok, Err => rpc_err }`-Block durch obigen Block ersetzen. `consumer`/`crp`-Parsing davor bleibt unverändert.)

- [ ] **Step 4: Run tests — erwartet PASS**

Run: `cargo nextest run -E 'test(tool_defs_expose_companion_param) + test(mcp_companion_matches_cli)'`
Expected: PASS.

- [ ] **Step 5: Voller Lauf**

Run: `cargo nextest run`
Expected: PASS (alles grün).

- [ ] **Step 6: fmt + Commit**

```bash
cargo fmt
git add src/bin/lean_md.rs
git commit -m "feat(mcp): ctx_md_render companion param + phase XOR companion"
```

---

### Task 5: Pointer-Auflösung + Phasen-Wegweiser im Body

**Files:**
- Modify: `content/skills/lmd-test-driven-development/body.lmd.md` (vollständige Überschreibung)
- Test: `src/skills.rs` (`mod tests`)

**Interfaces:**
- Consumes: `render_skill` mit `phase` (bestehend).
- Produces: `rationalizations`-Phase mit konkretem `companion`-Render-Aufruf (kein „Spec #2"-Platzhalter); jede Phase endet mit korrektem `next:`-Zeiger.

- [ ] **Step 1: Failing test schreiben** — `src/skills.rs` `mod tests`:

```rust
    #[test]
    fn rationalizations_points_to_companion_render() {
        let out = render_skill(
            "lmd-test-driven-development",
            Some("rationalizations"),
            None,
            None,
            PathBuf::from("."),
        )
        .unwrap();
        assert!(
            out.contains("companion=\"testing-anti-patterns\""),
            "rationalizations must carry the concrete companion render call: {out}"
        );
        assert!(
            !out.contains("ported in Spec #2"),
            "the Spec #2 placeholder must be gone: {out}"
        );
    }

    #[test]
    fn phases_carry_next_pointers() {
        for (phase, needle) in [
            ("red", "next: render phase \"green\""),
            ("green", "next: render phase \"refactor\""),
            ("refactor", "next: render phase \"red\""),
        ] {
            let out = render_skill(
                "lmd-test-driven-development",
                Some(phase),
                None,
                None,
                PathBuf::from("."),
            )
            .unwrap();
            assert!(out.contains(needle), "phase {phase} missing next-pointer '{needle}': {out}");
        }
    }
```

- [ ] **Step 2: Run test — erwartet FAIL**

Run: `cargo nextest run -E 'test(rationalizations_points_to_companion_render) + test(phases_carry_next_pointers)'`
Expected: FAIL — Body trägt noch den „Spec #2"-Platzhalter und keine `next:`-Zeiger.

- [ ] **Step 3: Body überschreiben** — `content/skills/lmd-test-driven-development/body.lmd.md` vollständig ersetzen durch:

```markdown
<!-- lmd-test-driven-development body — rendered phase-by-phase via ctx_md_render -->

@phase "red"
@include test-first-core

## RED — write the failing test first

Write exactly one failing test that pins the next behavior. Then **Verify RED (mandatory)**:
run `ctx_shell "cargo nextest run"` and confirm the test fails *for the right reason*
(it asserts the missing behavior — not a compile error, not a typo).

Good: the test names the behavior and fails on the assertion.
Bad: it fails only because the symbol does not exist yet — that proves nothing about behavior.

next: render phase "green".
@phase-end

@phase "green"
@include test-first-core

## GREEN — minimal code to pass

Write the least code that makes the failing test pass. Then **Verify GREEN (mandatory)**:
run `ctx_shell "cargo nextest run"` and confirm the test passes.

YAGNI: no speculative parameters, no extra abstraction, no code the test does not demand.

next: render phase "refactor".
@phase-end

@phase "refactor"
@include test-first-core

## REFACTOR — clean up only under green

Refactor only under green: remove duplication, improve names, extract helpers.
No new behavior here — if you need new behavior, return to RED. Re-run
`ctx_shell "cargo nextest run"` after each change; it must stay green.

next: render phase "red" for the next behavior.
@phase-end

@phase "rationalizations"
@include test-first-core

## Common Rationalizations (Excuse | Reality)

| Excuse | Reality |
| "I'll test after." | Code-after-test is production code without a failing test. |
| "It's too simple to break." | Simple code breaks; the test is cheap. |
| "The test passed right away." | It never failed — no evidence it tests anything. |
| "Refactor needs a quick prod tweak." | A prod tweak is new behavior — go back to RED. |

**Why order matters:** the failing test is the only proof the test exercises the behavior.
**When stuck:** shrink the test until one tiny behavior is in scope, then RED → GREEN.

Verification checklist: test written first · RED observed · minimal GREEN · refactor under green.
For testing anti-patterns (mocks, test-only methods, incomplete mocks), render the companion:
`ctx_md_render(skill="lmd-test-driven-development", companion="testing-anti-patterns")`.

next: return to your active phase (red/green/refactor).
@phase-end
```

- [ ] **Step 4: Run tests — erwartet PASS**

Run: `cargo nextest run -E 'test(rationalizations_points_to_companion_render) + test(phases_carry_next_pointers)'`
Expected: PASS.

- [ ] **Step 5: Regress-Check (Seed-Consistency + Phasen-Isolation)**

Run: `cargo nextest run -E 'test(tdd_body_matches_seed_file_on_disk) + test(tdd_phases_render_isolated_no_cross_leak) + test(every_tdd_phase_includes_test_first_core)'`
Expected: PASS — embedded Body == on-disk-Seed, keine Cross-Phase-Leaks, Iron Law in jeder Phase.

- [ ] **Step 6: fmt + Commit**

```bash
cargo fmt
git add content/skills/lmd-test-driven-development/body.lmd.md src/skills.rs
git commit -m "feat(skills): resolve companion pointer + phase next-pointers in TDD body"
```

---

### Task 6: SKILL.md-Stub — treue Upstream-Adaption

**Files:**
- Modify: `content/skills/lmd-test-driven-development/SKILL.md` (vollständige Überschreibung)
- Test: `src/skill_install.rs` ODER `src/skills.rs` (`mod tests`) — Stub liegt on-disk; Test liest die Datei via `CARGO_MANIFEST_DIR` und prüft Orientierungs-Marker.

**Interfaces:**
- Consumes: nichts Neues (statischer Stub).
- Produces: orientierender Stub-Body; `description`-Frontmatter unverändert (Trigger-only).

- [ ] **Step 1: Failing test schreiben** — `src/skills.rs` `mod tests`:

```rust
    #[test]
    fn skill_md_stub_carries_orientation() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let stub = std::fs::read_to_string(
            std::path::Path::new(manifest)
                .join("content/skills/lmd-test-driven-development/SKILL.md"),
        )
        .unwrap();
        // Frontmatter trigger unchanged (SDO/discovery).
        assert!(stub.contains(
            "description: Use when implementing any feature or bugfix, before writing implementation code"
        ));
        // Orientation layer (E6).
        assert!(stub.contains("NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST"));
        assert!(stub.contains("Where this runs"));
        for call in [
            "phase=\"red\"",
            "phase=\"green\"",
            "phase=\"refactor\"",
            "phase=\"rationalizations\"",
            "companion=\"testing-anti-patterns\"",
        ] {
            assert!(stub.contains(call), "stub missing render call '{call}'");
        }
        // Companion trigger (E7, upstream wording) + final rule + XOR.
        assert!(stub.contains("When adding mocks or test utilities"));
        assert!(stub.contains("never both"));
        assert!(stub.contains("Otherwise → not TDD"));
    }
```

- [ ] **Step 2: Run test — erwartet FAIL**

Run: `cargo nextest run -E 'test(skill_md_stub_carries_orientation)'`
Expected: FAIL — der aktuelle Stub trägt weder Iron Law noch „Where this runs" noch die Companion-Zeile.

- [ ] **Step 3: SKILL.md überschreiben** — `content/skills/lmd-test-driven-development/SKILL.md` vollständig ersetzen durch:

```markdown
---
name: lmd-test-driven-development
description: Use when implementing any feature or bugfix, before writing implementation code
---

# Test-Driven Development (lmd delegation stub)

Write the test first. Watch it fail. Write minimal code to pass.
**Core principle:** If you didn't watch the test fail, you don't know if it tests the right thing.
**Violating the letter of the rules is violating the spirit of the rules.**

This skill's detail is rendered on demand, one phase at a time, by the lean-md
engine. Never read a body or companion file from disk — fetch via the tool.

## Where this runs
`ctx_md_render` is provided by the lean-md addon (lean-ctx MCP server, or the
`lean-md` CLI). You do NOT need the lean-md source checked out — every body is
embedded in the running tool.

## When to Use
**Always:** new features · bug fixes · refactoring · behavior changes.
**Exceptions (ask your human partner):** throwaway prototypes · generated code · config files.
Thinking "skip TDD just this once"? Stop. That's rationalization.

## The Iron Law
    NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST
Wrote code before the test? Delete it. Start over. Delete means delete.

## Red → Green → Refactor (render each step as you reach it)
- **RED**      `ctx_md_render(skill="lmd-test-driven-development", phase="red")`
               — write one failing test, then Verify RED (watch it fail correctly).
- **GREEN**    `ctx_md_render(skill="lmd-test-driven-development", phase="green")`
               — minimal code to pass, then Verify GREEN.
- **REFACTOR** `ctx_md_render(skill="lmd-test-driven-development", phase="refactor")`
               — clean up only under green.
- **rationalizations** `ctx_md_render(skill="lmd-test-driven-development", phase="rationalizations")`
               — read when tempted to skip a step.

## Testing Anti-Patterns
When adding mocks or test utilities, render the companion to avoid common pitfalls:
`ctx_md_render(skill="lmd-test-driven-development", companion="testing-anti-patterns")`
- Testing mock behavior instead of real behavior
- Adding test-only methods to production classes
- Mocking without understanding dependencies

Pass exactly one of `phase` or `companion`, never both.

## Final Rule
    Production code → test exists and failed first
    Otherwise → not TDD
No exceptions without your human partner's permission.
```

- [ ] **Step 4: Run test — erwartet PASS**

Run: `cargo nextest run -E 'test(skill_md_stub_carries_orientation)'`
Expected: PASS.

- [ ] **Step 5: fmt + Commit**

```bash
cargo fmt
git add content/skills/lmd-test-driven-development/SKILL.md src/skills.rs
git commit -m "feat(skills): upstream-adapted SKILL.md orientation stub"
```

---

### Task 7: COVERAGE-Audit-Eintrag für den Companion

**Files:**
- Modify: `src/availability.rs` (`COVERAGE`-Tabelle + Test)
- Test: `src/availability.rs` (`mod tests`)

**Interfaces:**
- Consumes: `default_registry()` (bestehend); `"include"`-Directive ist registriert (`bridges/include.rs:25`).
- Produces: COVERAGE-Zeile für den Companion-`@include`.

- [ ] **Step 1: Failing test schreiben** — `src/availability.rs` `mod tests`:

```rust
    #[test]
    fn coverage_carries_companion_row() {
        let has_companion = COVERAGE.iter().any(|(skill, step, directive, _)| {
            *skill == "lmd-test-driven-development"
                && *step == "testing-anti-patterns"
                && *directive == "include"
        });
        assert!(has_companion, "COVERAGE must record the companion @include row");
    }
```

- [ ] **Step 2: Run test — erwartet FAIL**

Run: `cargo nextest run -E 'test(coverage_carries_companion_row)'`
Expected: FAIL — Zeile fehlt.

- [ ] **Step 3: Implementierung** — `src/availability.rs`, in `COVERAGE` (nach der `("lmd-test-driven-development", "red", "read", "ctx_read")`-Zeile, ~Zeile 23) ergänzen:

```rust
    // Companion (Spec #2): the testing-anti-patterns reference pulls the
    // discipline block via `@include test-first-core` (the include directive).
    ("lmd-test-driven-development", "testing-anti-patterns", "include", "fragment-compose"),
```

- [ ] **Step 4: Run tests — erwartet PASS**

Run: `cargo nextest run -E 'test(coverage_carries_companion_row) + test(every_covered_directive_is_registered)'`
Expected: PASS — neue Zeile vorhanden UND `include` ist im `default_registry()`.

- [ ] **Step 5: fmt + Commit**

```bash
cargo fmt
git add src/availability.rs
git commit -m "feat(availability): COVERAGE row for testing-anti-patterns companion"
```

---

### Task 8: Abschluss-Verifikation (alle Gates)

**Files:** keine Änderung — reine Verifikation.

- [ ] **Step 1: Voller Testlauf**

Run: `cargo nextest run`
Expected: PASS — alle Tests grün (inkl. Seed-Consistency, Phasen-Isolation, Companion, CLI/MCP, Stub, COVERAGE).

- [ ] **Step 2: Clippy**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: keine Warnungen.

- [ ] **Step 3: fmt-Check**

Run: `cargo fmt --check`
Expected: keine Diffs.

- [ ] **Step 4: Companion-Render-Smoke (CLI)**

Run: `cargo run -q -- render --skill lmd-test-driven-development --companion testing-anti-patterns`
Expected: Companion mit Iron Law (aus `@include test-first-core`) + „Anti-Pattern 1" … „Red Flags".

- [ ] **Step 5: XOR-Smoke (CLI)**

Run: `cargo run -q -- render --skill lmd-test-driven-development --phase red --companion testing-anti-patterns`
Expected: exit 1, Meldung „--phase and --companion are mutually exclusive".

---

## Self-Review

**Spec-Coverage (§8 in diesem Spec):**
- Companion-Render-Param CLI → Task 3; MCP → Task 4. ✓
- Registry-Companion-Spalte → Task 1 (`COMPANIONS`). ✓
- `testing-anti-patterns.lmd.md`-Seed (kondensiert, sprachneutral, `@include test-first-core`) → Task 1. ✓
- Pointer-Auflösung in `rationalizations` → Task 5. ✓
- SKILL.md-Stub-Ersetzung (E6) + Companion-Trigger (E7) → Task 6. ✓
- Phasen-Wegweiser (E7) → Task 5. ✓
- Gates: Fragment/Seed-Consistency → Task 1 (companion) + Task 5 (body Regress); Param-Disjunktion → Task 3 (CLI) + Task 4 (MCP); CLI==MCP → Task 4; Stub-Orientierung → Task 6; Wegweiser → Task 5. ✓
- COVERAGE-Eintrag → Task 7. ✓
- `phase` XOR `companion` (E5) → Task 3 + Task 4. ✓
- `@include test-first-core` im Companion (E3) → Task 1 (Seed) + Task 2 (Render-Test). ✓

**Typ-Konsistenz:** `companion_body(skill, companion) -> Option<&'static str>` (Task 1) und `render_companion(skill, companion, consumer, crp, jail_root) -> Result<String, SkillRenderError>` (Task 2) durchgängig identisch in Task 3/4 verwendet. `SkillRenderError::CompanionNotFound(String)` einheitlich. `RenderArgs.companion: Option<String>` (Task 3). ✓

**Platzhalter-Scan:** keine TBD/TODO; jeder Code-Step zeigt vollständigen Code. ✓

> **Hinweis Implementer (Task 2, Step 3e):** `render_full_source` wird bewusst nur von `render_companion` genutzt; `render_skill` bleibt unverändert, um die 12 bestehenden Tests nicht zu gefährden. Das ist eine bewusste, dokumentierte Nicht-Vereinheitlichung (geringe Duplikation vs. Regress-Risiko).
