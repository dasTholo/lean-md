@lean-md
consumer: ai
crp: compact

@var test_cmd default="cargo nextest run" desc="project test runner command"
@var lint_cmd default="cargo clippy --all-targets -- -D warnings" desc="project lint gate"
@import .lean-ctx/lean-md/plan-recipes /

# workflow-token-comparison — Implementation Plan

## Goal

Deterministischer, byte-stabiler (#498) 3-Wege-Benchmark (superpowers / mdai /
lean-md) über die Kette brainstorm→writing-plans→subagent-dev, zwei Achsen:
Achse 1 = Skill-Instruktions-Overhead pro Stufe, Achse 2 = Plan-Artefakt-Größe +
Phase-isolierter Dispatch. Spec:
`docs/lean-md/specs/2026-07-11-lmd-workflow-token-benchmark-design.md`.

## Architecture

Neue Workspace-Member-Crate `benchmarks/workflow-token-comparison/` (bin), Aufruf
`cargo run -p workflow-token-comparison`, erzeugt byte-stabiles `SUMMARY.md`.
`tiktoken-rs` + mdai-Node-Subprozess-Logik leben NUR in dieser Crate — die
publizierte `lean_md`-Crate bleibt unberührt. Tokenizer/Artifact-Form wird aus
`benchmarks/skill-token-comparison/harness.rs:1-50` portiert. lmd-Render in-process
via `lean_md::skills::{render_skill, render_companion, render_source_with_phase}`
(`skills.rs:105,148,214`), `Consumer::Ai`, `crp=None`, `jail_root` = Repo-Root.

## Global Constraints

- Non-goal: **kein** lokaler Engine-Build, **kein** lean-ctx im Worktree. mdai-Engine =
  veröffentlichtes `@markdownai/core` (Binary `mai`, global installiert) — nur dieser
  markdownai-Render-Layer wird gemessen.
- #498: `SUMMARY.md` ist byte-stabil (zweifache Generierung identisch) — Test-Gate.
- `lean_md`-Crate bleibt frei von `tiktoken-rs` und Subprozess-Logik (Isolation in
  der Bench-Crate) — Reviewer prüft: kein Diff an Root-`[dependencies]`/`[lib]`/`[[bin]]`.
- mdai-Status pro Zelle sichtbar: `live` | `recorded` | `N/A`. `recorded`-Zahlen
  stammen aus `lean-ctx:/mdai-benchmark.md` (v3 S3a / v4 Part-B).
- mdai nimmt auf Achse 1 nur an der brainstorm-Stufe teil; writing-plans/subagent =
  N/A (mdai baute nie diese Skills) — Invariante im Test.
- superpowers-Version = **6.1.1** (latest installiert), Pfad env-überschreibbar
  (`SP_PLUGIN_ROOT`) + im SUMMARY offengelegt.
- Tokenizer-Parität: alle drei Systeme über **dieselbe** tiktoken-rs-Instanz.
- Ordering: T1 vor allem; T2 (mdai-Adapter) vor T3 (Achse 1) und T5 (Achse 2);
  T5 braucht T4; T6 braucht T3+T5.

@phase "task-1"
## Task 1: Workspace + Bench-Crate-Skeleton + Tokenizer-Harness

**Files:** modify `Cargo.toml` (Root); create
`benchmarks/workflow-token-comparison/Cargo.toml`,
`benchmarks/workflow-token-comparison/src/main.rs`,
`benchmarks/workflow-token-comparison/src/harness.rs`.

**Interfaces:** Bin-Crate `workflow-token-comparison`; `harness` exportiert
`enum Family { Cl100k, O200k }`, `fn token_count(&str, Family) -> usize`,
`struct Artifact { name:String, tokens_cl100k:usize, tokens_o200k:usize }`,
`Artifact::from_text(&str,&str)`.

Root `Cargo.toml` — nach dem `[[example]]`-Block (Zeilen 39-41) anfügen (NEW):

    [workspace]
    members = ["benchmarks/workflow-token-comparison"]
    resolver = "3"

Existing Root-`Cargo.toml` sonst unverändert — Anker `Cargo.toml:1-41`.

New `benchmarks/workflow-token-comparison/Cargo.toml`:

    [package]
    name = "workflow-token-comparison"
    version = "0.1.0"
    edition = "2024"
    publish = false

    [dependencies]
    lean_md = { path = "../..", package = "lean-md" }
    tiktoken-rs = "0.12"
    serde_json = "1.0"   # minimal MCP stdio JSON-RPC client for `mai serve`

    [[bin]]
    name = "workflow-token-comparison"
    path = "src/main.rs"

New `src/harness.rs` — Tokenizer/Artifact aus
`benchmarks/skill-token-comparison/harness.rs:5-50` portieren (`Family`,
`token_count`, `Artifact`, `Artifact::from_text`, `artifact_from_file`). **Wichtig:**
`Artifact::from_text` und `artifact_from_file` sind in der Quelle **privat** — beim
Port auf `pub` heben (T3/T5 rufen sie modulübergreifend als `crate::harness::…`).
Kein `collect_variant_*` — diese Achsen-Logik kommt in T3/T5.

New `src/main.rs` (Skeleton):

    #[path = "harness.rs"]
    mod harness;

    fn main() {
        // wired in T6; skeleton proves the crate builds + runs.
        println!("workflow-token-comparison: run wired in T6");
    }

@call test(token_count_empty_is_zero)

Test verbatim (in `harness.rs`, `#[cfg(test)]`):

    #[test]
    fn token_count_empty_is_zero() {
        assert_eq!(super::token_count("", super::Family::Cl100k), 0);
        assert!(super::token_count("hello world", super::Family::Cl100k) > 0);
        assert!(super::token_count("hello world", super::Family::O200k) > 0);
    }

**Expected:** `cargo run -p workflow-token-comparison` druckt die Skeleton-Zeile;
`cargo nextest run -p workflow-token-comparison` grün; `cargo build` (Root) baut
weiterhin (Workspace intakt).

### Verify & Close

@call verify("benchmarks/workflow-token-comparison/Cargo.toml")
@call gate("Cargo.toml benchmarks/workflow-token-comparison/")
@call commit("Cargo.toml benchmarks/workflow-token-comparison/", "feat(bench): workspace member + tokenizer harness skeleton")
@call remember_decision("workflow-token-comparison ist Workspace-Member; tiktoken-rs nur dort, nie in lean_md; superpowers-Ziel = 6.1.1")
@phase-end

@phase "task-2"
## Task 2: mdai-Adapter (markdownai-Subprozess + Preflight + recorded-Fallback)

@call recall_context("nur markdownai-Layer messen, nie mdai-lean-ctx bauen")

**Files:** create `benchmarks/workflow-token-comparison/src/mdai.rs`; modify
`src/main.rs` — Zeile `mod mdai;` ergänzen (sonst ist `mdai.rs` eine für Cargo
verwaiste Datei: nicht kompiliert, Test läuft nicht, Gate falsch-grün).

**Interfaces:**

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum MdaiMode { Live, Recorded }
    pub struct MdaiCli { pub mai_bin: String, pub mode: MdaiMode }
    impl MdaiCli {
        pub fn preflight() -> Self;   // env MAI_BIN (default "mai"); Live iff `<mai_bin> --version` exits 0
        // Both drive ONE `mai serve` MCP session (stdio JSON-RPC); None in Recorded mode.
        pub fn render_full(&self, cwd: &std::path::Path, rel: &str) -> Option<String>;          // read_file{path:rel, format:"ai"}
        pub fn render_phase(&self, cwd: &std::path::Path, rel: &str, phase: &str) -> Option<String>; // read_file{path:rel, phase, format:"ai"}
        pub fn list_phases(&self, cwd: &std::path::Path, rel: &str) -> Vec<String>;             // MCP list_phases (or `mai list-phases`)
    }
    // recorded fallback constants (cl100k, from mdai-benchmark.md v3 S3a / v4 Part-B):
    pub const REC_MDAI_FULL_RENDER: usize = 2266;    // v3 rendered full plan
    pub const REC_MDAI_PHASE_MIN: usize = 404;       // v4 Part-B T0
    pub const REC_MDAI_PHASE_MAX: usize = 3234;      // v4 Part-B T5

**Engine = veröffentlichtes `@markdownai/core` (Binary `mai` 1.4.0, global via
`npm i -g @markdownai/core`, in der lean-ctx-Allowlist).** `preflight`:
`<mai_bin> --version` exit 0 → `MdaiMode::Live`, sonst `MdaiMode::Recorded` (kein
Panic). **Kein** lokaler Build, **kein** lean-ctx, **kein** Worktree-Engine-Code —
die Engine ist separat installiert.

Phase-Isolation ist **validiert** und läuft über den **MCP-Server** (`mai serve`,
stdio) — es gibt kein CLI-`--phase`. Protokoll (verifiziert, newline-delimited
JSON-RPC 2.0):
1. `mai serve --cwd <cwd>` spawnen (stdin/stdout Pipes).
2. `initialize` (`protocolVersion:"2024-11-05"`, `capabilities:{}`,
   `clientInfo:{…}`) → dann Notification `notifications/initialized`.
3. `tools/call` `read_file`, args `{path:<rel>, format:"ai"}` (voll) bzw.
   `{path:<rel>, phase:<id>, format:"ai"}` (isoliert). **`path` ist relativ zu
   `--cwd`** (absolute Pfade blockt der Server). Antwort:
   `result.content` = gerenderter Markdown-String
   (Shape `{"content":"…","isMarkdownAI":true,"warnings":[]}`) → tokenisieren.
4. Server killen. Der Client sitzt minimal in `mdai.rs` (`std::process` + `serde_json`).

Achse-1-brainstorm: cwd = `MDAI_WORKTREE` (env, default
`/home/tholo/Scripts/lean-ctx-mdai-bench`), rel =
`mdai/skills/mdai-brainstorm/body.mdai.md`. Achse-2: cwd = Fixture-Genre-Dir,
rel = `plan.mdai.md`. Fixtures sind self-contained (v2-Syntax `@phase-end`, keine
externen `@include`) → sauber isolierbar.

@call tdd(mdai_preflight_never_panics)

Test verbatim:

    #[test]
    fn mdai_preflight_never_panics() {
        // With MAI_BIN pointing to a missing binary, preflight must degrade to Recorded.
        // SAFETY: single-threaded test; env mutation is local to this test binary.
        unsafe { std::env::set_var("MAI_BIN", "/nonexistent-mai-binary-xyz"); }
        let cli = super::MdaiCli::preflight();
        assert_eq!(cli.mode, super::MdaiMode::Recorded);
        assert!(cli.render_full(std::path::Path::new("/tmp"), "whatever.mdai.md").is_none());
        assert!(super::REC_MDAI_FULL_RENDER > 0 && super::REC_MDAI_PHASE_MAX > super::REC_MDAI_PHASE_MIN);
    }

**Expected:** Test grün; fehlendes/ungültiges `mai`-Binary ⇒ `Recorded`, keine Panik,
recorded-Konstanten plausibel.

### Verify & Close

@call verify("benchmarks/workflow-token-comparison/src/mdai.rs")
@call gate("benchmarks/workflow-token-comparison/src/mdai.rs")
@call commit("benchmarks/workflow-token-comparison/src/mdai.rs", "feat(bench): mdai markdownai adapter + preflight + recorded fallback")
@call remember_decision("mdai-Adapter: Engine=@markdownai/core `mai` (env MAI_BIN, default 'mai'); Live iff `mai --version` ok sonst Recorded; Phase-Isolation via `mai serve` MCP tools/call read_file{path,phase,format:ai} → result.content; path relativ zu --cwd")
@phase-end

@phase "task-3"
## Task 3: Achse 1 — Skill-Overhead (superpowers + lmd + mdai-brainstorm)

@call recall_context("Crate-Layout, superpowers 6.1.1, mdai-Adapter-API MdaiCli")

**Files:** create `benchmarks/workflow-token-comparison/src/axis1.rs`; modify
`src/main.rs` — Zeile `mod axis1;` ergänzen (`mod mdai;` steht bereits aus T2; der
Inline-Test referenziert `crate::mdai::MdaiCli`, beide `mod` müssen registriert sein).

**Interfaces:**

    pub struct StageOverhead {
        pub stage: &'static str,          // "brainstorm" | "writing-plans" | "subagent-dev"
        pub sp_content: usize,            // superpowers monolith + companions (cl100k)
        pub lmd_content: usize,           // lmd stub + all phases + companions (cl100k)
        pub lmd_cumulative: Vec<(usize, usize)>, // (k phases, cumulative cl100k incl. stub)
        pub mdai_content: Option<usize>,  // Some only for brainstorm (mdai has no other skill)
        pub mdai_status: &'static str,    // "live" | "recorded" | "n/a"
    }
    pub fn collect_axis1(jail_root: std::path::PathBuf, mdai: &crate::mdai::MdaiCli) -> Vec<StageOverhead>;

superpowers-Root env-überschreibbar (NEW const, Muster wie `SP_SKILL` in
`benchmarks/skill-token-comparison/main.rs:11`):

    const SP_PLUGIN_ROOT: &str =
        "/home/tholo/.claude/plugins/cache/claude-plugins-official/superpowers/6.1.1/skills";

Stage→(superpowers-dir, superpowers-companions[], lmd-skill, lmd-phases[], lmd-companions[]).
superpowers-Artefakte: `<SP_PLUGIN_ROOT>/<dir>/SKILL.md` + Companion-`.md`
(`artifact_from_file`, fehlende überspringen). lmd-Artefakte in-process:
`render_skill(skill, Some(phase), Some(Consumer::Ai), None, jail_root)` je Phase +
`render_companion(...)` je Companion, tokenisiert via `harness::Artifact::from_text`.

Konkrete Stage-Daten (verbatim):

    brainstorm:
      sp-dir=brainstorming  sp-companions=[spec-document-reviewer-prompt.md, visual-companion.md]
      lmd-skill=lmd-brainstorm
      lmd-phases=[pre-context, explore, questions, approaches, present-design, write-spec, self-review, handoff]
      lmd-companions=[spec-reviewer, visual-companion]
    writing-plans:
      sp-dir=writing-plans  sp-companions=[plan-document-reviewer-prompt.md]
      lmd-skill=lmd-writing-plans
      lmd-phases=[pre-context, file-structure, task-sizing, plan-format, write-plan, self-review, handoff]
      lmd-companions=[plan-reviewer]
    subagent-dev:
      sp-dir=subagent-driven-development  sp-companions=[implementer-prompt.md, task-reviewer-prompt.md]
      lmd-skill=lmd-subagent-driven-development
      lmd-phases=[orient, preflight, dispatch-mode, dispatch, parallel-dispatch, review, final-review, handoff]
      lmd-companions=[implementer, task-reviewer, code-reviewer]

mdai-brainstorm-Zelle (nur stage=="brainstorm"): Live →
`mdai.render_full(&mdai_worktree, "mdai/skills/mdai-brainstorm/body.mdai.md")`
tokenisieren (`mdai_worktree` = env `MDAI_WORKTREE`, default
`/home/tholo/Scripts/lean-ctx-mdai-bench`) → `mdai_content=Some(n)`,
`mdai_status="live"`. Recorded → `mdai_content=None`, `mdai_status="recorded"`.
Andere Stages: `mdai_content=None`, `mdai_status="n/a"`.

@call tdd(axis1_overhead_and_na_invariant)

Test verbatim:

    #[test]
    fn axis1_overhead_and_na_invariant() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mdai = crate::mdai::MdaiCli::preflight();
        let stages = super::collect_axis1(root, &mdai);
        assert_eq!(stages.len(), 3);
        for s in &stages {
            assert!(s.lmd_content > 0, "lmd {} must render", s.stage);
        }
        let bs = stages.iter().find(|s| s.stage == "brainstorm").unwrap();
        assert!(bs.sp_content > 0);
        assert!(bs.mdai_status == "live" || bs.mdai_status == "recorded");
        for s in stages.iter().filter(|s| s.stage != "brainstorm") {
            assert_eq!(s.mdai_status, "n/a");
            assert!(s.mdai_content.is_none());
        }
    }

**Expected:** `cargo nextest run -p workflow-token-comparison` grün; drei Stages,
lmd rendert überall, superpowers-brainstorm > 0, mdai nur brainstorm (live|recorded),
sonst `n/a`.

### Verify & Close

@call verify("benchmarks/workflow-token-comparison/src/axis1.rs")
@call gate("benchmarks/workflow-token-comparison/src/axis1.rs")
@call commit("benchmarks/workflow-token-comparison/src/axis1.rs", "feat(bench): axis-1 skill overhead (superpowers+lmd+mdai-brainstorm)")
@call remember_decision("Achse-1 StageOverhead: sp=Monolith+Companions, lmd=stub+phases+companions kumulativ, mdai nur brainstorm (Option+status)")
@phase-end

@phase "task-4"
## Task 4: Fixtures — 2 Genres × 3 Ports (inhaltsgleich)

**Files:** create
`benchmarks/workflow-token-comparison/fixtures/refactoring/{plan.md, plan.mdai.md, plan.lmd.md}`,
`benchmarks/workflow-token-comparison/fixtures/audit/{plan.md, plan.mdai.md, plan.lmd.md}`.

Discovery: zwei reale Quell-Pläne wählen (`@list docs/lean-md/plans/`) — ein
Refactoring-Genre, ein Audit-Genre. Für jeden:
- `plan.lmd.md` = der reale lmd-Plan (unverändert kopiert).
- `plan.mdai.md` = 1:1-Port in markdownai-**v2**-Direktiven (`@phase … @phase-end`,
  `@call`/`@constraint … @constraint-end`, argfreie self-close mit ` /`) —
  **inhaltsgleich**, **self-contained** (keine externen `@include`/`@import`, damit
  `mai serve`-Phasen sauber isolieren; Header `@markdownai v1.0`).
- `plan.md` = superpowers-Stil Prosa-Monolith derselben Substanz (keine Direktiven,
  Phasen als Markdown-Headings).

**Fairness-Protokoll (binding):** die drei Ports müssen dieselbe Substanz tragen.
Sanity-Check über gerenderte Outputs: `|tokens(lmd_full) − tokens(mdai_full)|` und
gegen `plan.md` innerhalb `FAIRNESS_TOL` (in T5 als const, Default `0.15` = 15 %).
Protokoll + Toleranz in `README.md` (T6) dokumentieren.

@call test(fixtures_present_and_nonempty)

Test verbatim (`tests/fixtures.rs`; Pfade relativ zu `CARGO_MANIFEST_DIR`):

    #[test]
    fn fixtures_present_and_nonempty() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        for genre in ["refactoring", "audit"] {
            for f in ["plan.md", "plan.mdai.md", "plan.lmd.md"] {
                let p = base.join(genre).join(f);
                let s = std::fs::read_to_string(&p).unwrap_or_else(|_| panic!("missing {}", p.display()));
                assert!(!s.trim().is_empty(), "empty fixture {}", p.display());
            }
        }
    }

**Expected:** sechs Fixture-Dateien vorhanden, keine leer.

### Verify & Close

@call verify("benchmarks/workflow-token-comparison/fixtures/")
@call gate("benchmarks/workflow-token-comparison/fixtures/")
@call commit("benchmarks/workflow-token-comparison/fixtures/", "test(bench): 2-genre × 3-port fixtures (identical substance)")
@call remember_decision("Fixture-Quellpläne: refactoring=<datei>, audit=<datei>")
@phase-end

@phase "task-5"
## Task 5: Achse 2 — Plan/Dispatch-Messung + Kostenmodell

@call recall_context("Fixture-Quellpläne + mdai Phase-Isolations-Variante")

**Files:** create `benchmarks/workflow-token-comparison/src/axis2.rs`; modify
`src/main.rs` — Zeile `mod axis2;` ergänzen.

**Interfaces:**

    pub struct EnginePlan {
        pub source: usize,           // tokens of the on-disk source (write cost)
        pub full: usize,             // full render (orchestrator read)
        pub phases: Vec<usize>,      // per isolated phase (subagent dispatch)
        pub status: &'static str,    // "live" | "recorded" | "n/a"
    }
    pub struct FixtureAxis2 {
        pub genre: &'static str,
        pub superpowers: EnginePlan, // no engine: full == source; phases all == full
        pub mdai: EnginePlan,        // markdownai adapter or recorded
        pub lmd: EnginePlan,         // render_source_with_phase
    }
    pub const PRICE_SONNET_PER_M: f64 = 3.0;   // disclosed model param, tunable
    pub const PRICE_OPUS_PER_M: f64 = 15.0;    // disclosed model param, tunable
    pub const HARD_RULES_OVERHEAD_TOKENS: usize = 480; // per-subagent floor (mdai-benchmark)
    pub const FAIRNESS_TOL: f64 = 0.15;
    pub fn collect_axis2(jail_root: std::path::PathBuf, mdai: &crate::mdai::MdaiCli) -> Vec<FixtureAxis2>;
    pub fn dispatch_cost(plan: &EnginePlan, price_per_m: f64) -> f64; // Σ(phase+overhead)/1e6 * price

lmd: `render_source_with_phase(source, None, Some(Consumer::Ai), None, jail_root)`
für `full`, `Some(phase)` je Phase (Phasenliste aus den `@phase`-IDs des
`plan.lmd.md`). superpowers: `full == source == jede phase` (keine Engine —
Subagent bekommt strukturell den Vollplan) — konkret
`superpowers.phases = vec![full; <lmd-Phasenzahl>]` (gleiche Subagent-Zahl wie lmd,
damit der Dispatch-Vergleich fair ist). mdai (Live, `status="live"`): cwd =
`fixtures/<genre>/`, rel = `plan.mdai.md`;
`mdai.render_full(cwd,"plan.mdai.md")` = `full`, je Phase aus
`mdai.list_phases(cwd,"plan.mdai.md")` ein `mdai.render_phase(cwd,"plan.mdai.md",id)`.
sonst recorded (`status="recorded"`, `full = REC_MDAI_FULL_RENDER`,
`phases = vec![REC_MDAI_PHASE_MIN, REC_MDAI_PHASE_MAX]`).

Fairness-Sanity im Collect: gerenderte `lmd.full` vs `mdai.full` vs `sp.full`
innerhalb `FAIRNESS_TOL` (mdai-Vergleich nur wenn `mdai.status=="live"`; recorded
ist eine Konstante, kein fairer Substanz-Vergleich), sonst Panic mit Delta
(verhindert „gewinnen durch Weglassen").

@call tdd(axis2_isolation_and_cost_invariants)

Test verbatim:

    #[test]
    fn axis2_isolation_and_cost_invariants() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mdai = crate::mdai::MdaiCli::preflight();
        let fx = super::collect_axis2(root, &mdai);
        assert_eq!(fx.len(), 2);
        for f in &fx {
            // superpowers: no isolation → every phase equals full plan.
            assert!(f.superpowers.phases.iter().all(|&p| p == f.superpowers.full));
            // lmd: isolated phase is strictly smaller than the full plan.
            assert!(f.lmd.phases.iter().all(|&p| p < f.lmd.full));
            // dispatch cost of full-plan-per-subagent >= isolated dispatch.
            assert!(super::dispatch_cost(&f.superpowers, super::PRICE_SONNET_PER_M)
                    >= super::dispatch_cost(&f.lmd, super::PRICE_SONNET_PER_M));
        }
    }

**Expected:** Test grün; superpowers-Phase == Vollplan, lmd-Phase < Vollplan,
Dispatch-Kosten superpowers ≥ lmd.

### Verify & Close

@call verify("benchmarks/workflow-token-comparison/src/axis2.rs")
@call review_change()
@call gate("benchmarks/workflow-token-comparison/src/axis2.rs")
@call commit("benchmarks/workflow-token-comparison/src/axis2.rs", "feat(bench): axis-2 plan/dispatch measurement + cost model")
@call remember_decision("Achse-2 EnginePlan: sp full==phase (keine Isolation), lmd via render_source_with_phase, mdai live/recorded; Kostenmodell dispatch_cost = Σ(phase+overhead)*preis")
@phase-end

@phase "task-6"
## Task 6: SUMMARY.md-Generierung + Byte-Stabilität + README

@call recall_context("Achse-1 + Achse-2 Interfaces + mdai-Mode + offengelegte Konstanten")

**Files:** create
`benchmarks/workflow-token-comparison/src/summary.rs`,
`benchmarks/workflow-token-comparison/SUMMARY.md` (generiert),
`benchmarks/workflow-token-comparison/README.md`,
`benchmarks/workflow-token-comparison/tests/stability.rs`;
modify `benchmarks/workflow-token-comparison/src/main.rs` (Zeile `mod summary;`
ergänzen + den Run verdrahten). Nach T6 deklariert `main.rs`:
`mod harness; mod mdai; mod axis1; mod axis2; mod summary;`.

**Interfaces:**

    pub fn format_summary(
        axis1: &[crate::axis1::StageOverhead],
        axis2: &[crate::axis2::FixtureAxis2],
        mdai_mode: crate::mdai::MdaiMode,
    ) -> String;

`format_summary` (Muster: `benchmarks/skill-token-comparison/harness.rs:179-248`,
`std::fmt::Write`, keine Timestamps/Counter) — Sektionen: **Annahmen** (Tokenizer,
`PRICE_*`, `HARD_RULES_OVERHEAD_TOKENS`, `FAIRNESS_TOL`, superpowers-Version 6.1.1,
mdai-Mode offengelegt), **Achse 1** (Tabelle je Stufe mit `n/a`-Zellen +
brainstorm-3-Wege), **Achse 2** (je Fixture: source/full/phase je Engine +
Dispatch-Kostenmodell Sonnet/Opus), jede mdai-Zelle mit `live|recorded|n/a`.

`main.rs` verdrahtet: `preflight` → `collect_axis1` → `collect_axis2` →
`format_summary` → `std::fs::write("SUMMARY.md", …)` (Pfad via `CARGO_MANIFEST_DIR`).

New `README.md`: Methodik, mdai-Engine-Prereq (`npm i -g @markdownai/core` →
Binary `mai`; einmalig `lean-ctx allow mai`; **kein** lokaler Build, **kein**
lean-ctx), Fairness-Protokoll + `FAIRNESS_TOL`, recorded-Modus-Erklärung
(`mai` fehlt → recorded), Aufruf/Test-Kommandos.

@call tdd(summary_is_byte_stable)

Test verbatim (`tests/stability.rs`):

    // Byte-stability (#498): regenerating the summary yields identical bytes.
    #[path = "../src/harness.rs"]  mod harness;
    #[path = "../src/axis1.rs"]    mod axis1;
    #[path = "../src/axis2.rs"]    mod axis2;
    #[path = "../src/mdai.rs"]     mod mdai;
    #[path = "../src/summary.rs"]  mod summary;

    #[test]
    fn summary_is_byte_stable() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let m = mdai::MdaiCli::preflight();
        let a1 = axis1::collect_axis1(root.clone(), &m);
        let a2 = axis2::collect_axis2(root, &m);
        let s1 = summary::format_summary(&a1, &a2, m.mode);
        let s2 = summary::format_summary(&a1, &a2, m.mode);
        assert_eq!(s1, s2, "SUMMARY.md must be byte-stable (#498)");
        assert!(s1.contains("n/a"), "mdai writing-plans/subagent must show n/a");
        assert!(s1.contains("live") || s1.contains("recorded"));
    }

**Expected:** `cargo run -p workflow-token-comparison` schreibt `SUMMARY.md` mit
beiden Achsen + mdai-Statusmarkern; `cargo nextest run -p workflow-token-comparison`
grün inkl. Byte-Stabilität + `n/a`-Invariante.

### Verify & Close

@call verify("benchmarks/workflow-token-comparison/src/summary.rs")
@call render_check("workflow-token-comparison", "task-6")
@call gate("benchmarks/workflow-token-comparison/")
@call commit("benchmarks/workflow-token-comparison/", "feat(bench): SUMMARY generation, byte-stability test, README")
@call remember_decision("workflow-token-comparison vollständig: cargo run -p erzeugt byte-stabiles SUMMARY.md; mdai live/recorded/n/a transparent")
@phase-end
