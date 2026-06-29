# Skill-Token-Vergleich Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ein in-process Rust-Benchmark, das den Token-Verbrauch der TDD-Skills `superpowers/test-driven-development` (Variante A, Monolith) und `lmd-test-driven-development` (Variante B, Phasen-Rendering) misst und einen deterministischen `SUMMARY.md`-Report schreibt.

**Architecture:** Kernlogik in `benchmarks/skill-token-comparison/harness.rs` (pub fns; nutzt `tiktoken-rs` zum Zählen + `lean_md::skills` zum In-Process-Rendern von Variante B). Ein Integrationstest `tests/skill_token_comparison.rs` bindet `harness.rs` via `#[path]` ein und treibt die TDD-Zyklen (nextest-fähig). `main.rs` ist ein dünner Wrapper, der `harness.rs` via `mod harness;` einbindet, alles verdrahtet und `SUMMARY.md` schreibt. Verdrahtung als `[[example]]` mit explizitem `path` + `tiktoken-rs` nur als `dev-dependency` → `lean_md` bleibt tokenizer-frei.

**Tech Stack:** Rust (edition 2024), `tiktoken-rs = "0.12"` (dev-dep), `lean_md` lib (`skills::render_skill`/`render_companion`, `header::Consumer`, `crp_proto::CrpMode`).

## Global Constraints

- **Tests**: immer `cargo nextest run`, nie `cargo test`. Der Integrationstest `tests/skill_token_comparison.rs` wird von nextest erfasst.
- **Shell**: kein `&&`/`||`/`;`-Chaining; jeder Befehl ist eine eigene Invocation. Statt `cd dir && cargo …` → `cargo … --manifest-path <dir>/Cargo.toml`.
- **Vor `git add`** (pro geänderter `.rs`-Datei): `cargo fmt`.
- **Keine Worktrees** — direkt auf dem aktuellen Branch (`feat-lmd-v2`).
- **Determinismus (#498)**: Report-Output ist reine Funktion von (Datei-Inhalt, Phase, CRP-Modus, Tokenizer-Familie). Keine Timestamps/Zähler im Report-Body. Artefakt-Pfade content-adressiert, nicht zeitbasiert.
- **`tiktoken-rs` ausschließlich als `dev-dependency`** — kein Eintrag in `[dependencies]`, kein `#[cfg]`-Gate (der Code lebt im Example/Test-Target, wo dev-deps automatisch sichtbar sind). Den Tokenizer-Helper NICHT nach `src/` ziehen.
- **`lean_md`-API (verbatim)**:
  - `lean_md::skills::render_skill(name: &str, phase: Option<&str>, consumer: Option<Consumer>, crp: Option<CrpMode>, jail_root: PathBuf) -> Result<String, SkillRenderError>`
  - `lean_md::skills::render_companion(skill: &str, companion: &str, consumer: Option<Consumer>, crp: Option<CrpMode>, jail_root: PathBuf) -> Result<String, SkillRenderError>`
  - `lean_md::header::Consumer::{Ai, Human}`
  - `lean_md::crp_proto::CrpMode::{Off, Compact, Tdd}`
- **Feste Pfade (benannte Konstanten im Code)**:
  - superpowers SKILL: `/home/tholo/.claude/plugins/cache/claude-plugins-official/superpowers/6.0.3/skills/test-driven-development/SKILL.md`
  - superpowers Companion: `/home/tholo/.claude/plugins/cache/claude-plugins-official/superpowers/6.0.3/skills/test-driven-development/testing-anti-patterns.md`
  - lmd Stub SKILL: `<repo>/content/skills/lmd-test-driven-development/SKILL.md`
- **Skill-/Phasen-Identitäten**: Skill `lmd-test-driven-development`; Phasen `["red", "green", "refactor", "rationalizations"]`; Companion `testing-anti-patterns`.

---

## Datei-Struktur

| Datei | Verantwortung |
|---|---|
| `Cargo.toml` (modify) | `[dev-dependencies] tiktoken-rs` + `[[example]]`-Target |
| `benchmarks/skill-token-comparison/harness.rs` (create) | Kernlogik: `token_count`, `Family`, `Artifact`, `collect_variant_a/b`, `Metrics`, `compute_metrics`, `format_summary` |
| `benchmarks/skill-token-comparison/main.rs` (create) | dünner Wrapper: bindet `harness`, ruft echte Pfade auf, schreibt `SUMMARY.md` |
| `benchmarks/skill-token-comparison/README.md` (create) | Schicht-B-Protokoll (Druck-Varianten, Ablage der Subagent-Reports) |
| `tests/skill_token_comparison.rs` (create) | Integrationstest, treibt alle TDD-Zyklen via `#[path]`-eingebundenes `harness` |

---

### Task 1: Cargo-Wiring + Tokenizer-Helper (`token_count`)

Bringt das Build-/Test-Gerüst hoch und liefert die deterministische Token-Zählung für beide Familien. Erstes testbares Deliverable.

**Files:**
- Modify: `Cargo.toml` (nach `[features]`-Block anhängen)
- Create: `benchmarks/skill-token-comparison/harness.rs`
- Create: `tests/skill_token_comparison.rs`

**Interfaces:**
- Produces:
  - `pub enum Family { Cl100k, O200k }`
  - `pub fn token_count(text: &str, family: Family) -> usize`

- [ ] **Step 1: Cargo-Wiring**

In `Cargo.toml` ans Dateiende anhängen:

```toml
[dev-dependencies]
tiktoken-rs = "0.12"

[[example]]
name = "skill-token-comparison"
path = "benchmarks/skill-token-comparison/main.rs"
```

- [ ] **Step 2: Minimal-`main.rs`, damit das Example-Target existiert**

Create `benchmarks/skill-token-comparison/main.rs`:

```rust
//! Skill token-comparison benchmark (Schicht A). Renders both TDD skill
//! variants in-process, tokenizes the artifacts, writes SUMMARY.md.
//! tiktoken-rs is a dev-dependency; this code lives ONLY in the example
//! target (never bundled into the lean_md lib/bin). No #[cfg] gate needed.

#[path = "harness.rs"]
mod harness;

fn main() {
    // Wired up in Task 6.
}
```

- [ ] **Step 3: Write the failing test**

Create `tests/skill_token_comparison.rs`:

```rust
//! Integration test for the skill-token-comparison harness.
//! Binds the example's harness module via #[path] so nextest drives it
//! without pulling tiktoken-rs into the lean_md lib.

#[path = "../benchmarks/skill-token-comparison/harness.rs"]
mod harness;

use harness::{token_count, Family};

#[test]
fn token_count_empty_is_zero() {
    assert_eq!(token_count("", Family::Cl100k), 0);
    assert_eq!(token_count("", Family::O200k), 0);
}

#[test]
fn token_count_known_short_string() {
    // "hello world" = 2 BPE tokens in both cl100k_base and o200k_base.
    assert_eq!(token_count("hello world", Family::Cl100k), 2);
    assert_eq!(token_count("hello world", Family::O200k), 2);
}

#[test]
fn token_count_is_deterministic() {
    let text = "fn main() { println!(\"hi\"); }";
    assert_eq!(
        token_count(text, Family::Cl100k),
        token_count(text, Family::Cl100k)
    );
}
```

- [ ] **Step 4: Run test to verify it fails**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml skill_token_comparison`
Expected: FAIL — compile error (`harness.rs` missing / `token_count` not found).

- [ ] **Step 5: Write minimal implementation**

Create `benchmarks/skill-token-comparison/harness.rs`:

```rust
//! Core logic for the skill-token-comparison benchmark.
//! Shared by main.rs (example target) and tests/skill_token_comparison.rs
//! (integration test) via #[path]. Uses tiktoken-rs (dev-dep) + lean_md lib.

/// Tokenizer family. cl100k_base ~ Claude (~3% of actual); o200k_base matches
/// the lean-ctx savings ledger (COUNTING_FAMILY) for cross-comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Cl100k,
    O200k,
}

/// Count BPE tokens of `text` under `family`. Empty text → 0.
pub fn token_count(text: &str, family: Family) -> usize {
    if text.is_empty() {
        return 0;
    }
    let bpe = match family {
        Family::Cl100k => tiktoken_rs::cl100k_base(),
        Family::O200k => tiktoken_rs::o200k_base(),
    }
    .expect("tiktoken base tables load");
    bpe.encode_with_special_tokens(text).len()
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml skill_token_comparison`
Expected: PASS (3 tests).

- [ ] **Step 7: Format + Commit**

```bash
cargo fmt --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml
git add Cargo.toml benchmarks/skill-token-comparison/harness.rs benchmarks/skill-token-comparison/main.rs tests/skill_token_comparison.rs
git commit -m "feat(bench): skill-token-comparison wiring + tiktoken token_count helper"
```

---

### Task 2: Artefakt-Sammlung Variante A (superpowers, von Disk)

Liest die superpowers-Artefakte von einem Basis-Verzeichnis und tokenisiert sie. Test ist hermetisch (Fixture-Verzeichnis), `main()` nutzt später die echte Pfad-Konstante.

**Files:**
- Modify: `benchmarks/skill-token-comparison/harness.rs`
- Modify: `tests/skill_token_comparison.rs`

**Interfaces:**
- Consumes: `Family`, `token_count` (Task 1)
- Produces:
  - `pub struct Artifact { pub name: String, pub tokens_cl100k: usize, pub tokens_o200k: usize }`
  - `pub fn collect_variant_a(skill_md: &std::path::Path, companion_md: &std::path::Path) -> Vec<Artifact>`
  - Verhalten: liest beide Dateien; je vorhandener Datei ein `Artifact` (`name` = Dateiname). Fehlende Datei → übersprungen (kein Panic).

- [ ] **Step 1: Write the failing test**

In `tests/skill_token_comparison.rs` anhängen:

```rust
use harness::{collect_variant_a, Artifact};
use std::fs;

fn write_fixture(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    fs::write(&p, body).unwrap();
    p
}

#[test]
fn collect_variant_a_tokenizes_present_files() {
    let dir = std::env::temp_dir().join(format!("stc_a_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let skill = write_fixture(&dir, "SKILL.md", "# TDD\nWrite the test first.\n");
    let comp = write_fixture(&dir, "testing-anti-patterns.md", "# Anti-Patterns\nNo mocks.\n");

    let arts = collect_variant_a(&skill, &comp);

    assert_eq!(arts.len(), 2);
    assert_eq!(arts[0].name, "SKILL.md");
    assert!(arts[0].tokens_cl100k > 0);
    assert!(arts[0].tokens_o200k > 0);
    assert_eq!(arts[1].name, "testing-anti-patterns.md");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn collect_variant_a_skips_missing_companion() {
    let dir = std::env::temp_dir().join(format!("stc_a_miss_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let skill = write_fixture(&dir, "SKILL.md", "# TDD\n");
    let missing = dir.join("testing-anti-patterns.md");

    let arts = collect_variant_a(&skill, &missing);

    assert_eq!(arts.len(), 1);
    assert_eq!(arts[0].name, "SKILL.md");
    let _ = fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml skill_token_comparison`
Expected: FAIL — `collect_variant_a` / `Artifact` not found.

- [ ] **Step 3: Write minimal implementation**

In `benchmarks/skill-token-comparison/harness.rs` anhängen:

```rust
use std::path::Path;

/// One tokenized skill artifact, counted under both families.
#[derive(Debug, Clone)]
pub struct Artifact {
    pub name: String,
    pub tokens_cl100k: usize,
    pub tokens_o200k: usize,
}

impl Artifact {
    fn from_text(name: &str, text: &str) -> Self {
        Artifact {
            name: name.to_string(),
            tokens_cl100k: token_count(text, Family::Cl100k),
            tokens_o200k: token_count(text, Family::O200k),
        }
    }
}

fn artifact_from_file(path: &Path) -> Option<Artifact> {
    let text = std::fs::read_to_string(path).ok()?;
    let name = path.file_name()?.to_string_lossy().to_string();
    Some(Artifact::from_text(&name, &text))
}

/// Variant A (superpowers monolith): the full SKILL.md plus its companion.
/// Missing files are skipped (no panic).
pub fn collect_variant_a(skill_md: &Path, companion_md: &Path) -> Vec<Artifact> {
    [skill_md, companion_md]
        .into_iter()
        .filter_map(artifact_from_file)
        .collect()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml skill_token_comparison`
Expected: PASS (alle Tests inkl. Task 1).

- [ ] **Step 5: Format + Commit**

```bash
cargo fmt --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml
git add benchmarks/skill-token-comparison/harness.rs tests/skill_token_comparison.rs
git commit -m "feat(bench): collect_variant_a — tokenize superpowers artifacts from disk"
```

---

### Task 3: Artefakt-Sammlung Variante B (lmd, in-process Render)

Rendert die lmd-Phasen + Companion in-process via `lean_md::skills` und tokenisiert jeden Output; liest zusätzlich den Stub-`SKILL.md`.

**Files:**
- Modify: `benchmarks/skill-token-comparison/harness.rs`
- Modify: `tests/skill_token_comparison.rs`

**Interfaces:**
- Consumes: `Family`, `token_count`, `Artifact` (Tasks 1–2)
- Produces:
  - `pub const LMD_PHASES: [&str; 4] = ["red", "green", "refactor", "rationalizations"];`
  - `pub fn collect_variant_b(stub_md: &std::path::Path, jail_root: std::path::PathBuf) -> Vec<Artifact>`
  - Verhalten: ein `Artifact` für den Stub (name `"SKILL.md (stub)"`, falls Datei vorhanden), je Phase ein `Artifact` (name `"phase:<p>"`), ein `Artifact` für den Companion (name `"companion:testing-anti-patterns"`). Render via `render_skill`/`render_companion` mit `Consumer::Ai`, `crp = None`.

- [ ] **Step 1: Write the failing test**

In `tests/skill_token_comparison.rs` anhängen:

```rust
use harness::{collect_variant_b, LMD_PHASES};
use std::path::PathBuf;

#[test]
fn collect_variant_b_renders_phases_and_companion() {
    // Stub file optional; use the real repo stub if present, else a temp one.
    let dir = std::env::temp_dir().join(format!("stc_b_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let stub = dir.join("SKILL.md");
    std::fs::write(&stub, "# stub\n").unwrap();

    let arts = collect_variant_b(&stub, PathBuf::from("."));

    // 1 stub + 4 phases + 1 companion = 6.
    assert_eq!(arts.len(), 6);
    for p in LMD_PHASES {
        let name = format!("phase:{p}");
        let a = arts.iter().find(|a| a.name == name).expect("phase artifact");
        assert!(a.tokens_cl100k > 0, "phase {p} rendered empty");
    }
    let comp = arts
        .iter()
        .find(|a| a.name == "companion:testing-anti-patterns")
        .expect("companion artifact");
    assert!(comp.tokens_cl100k > 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn collect_variant_b_is_deterministic() {
    let dir = std::env::temp_dir().join(format!("stc_b_det_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let stub = dir.join("SKILL.md");
    std::fs::write(&stub, "# stub\n").unwrap();

    let a = collect_variant_b(&stub, PathBuf::from("."));
    let b = collect_variant_b(&stub, PathBuf::from("."));
    let names_a: Vec<_> = a.iter().map(|x| (x.name.clone(), x.tokens_cl100k)).collect();
    let names_b: Vec<_> = b.iter().map(|x| (x.name.clone(), x.tokens_cl100k)).collect();
    assert_eq!(names_a, names_b, "variant B must be byte-stable (#498)");
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml skill_token_comparison`
Expected: FAIL — `collect_variant_b` / `LMD_PHASES` not found.

- [ ] **Step 3: Write minimal implementation**

In `benchmarks/skill-token-comparison/harness.rs` anhängen:

```rust
use lean_md::header::Consumer;
use lean_md::skills::{render_companion, render_skill};
use std::path::PathBuf;

const LMD_SKILL: &str = "lmd-test-driven-development";
const LMD_COMPANION: &str = "testing-anti-patterns";

/// The lmd TDD phase sequence, rendered on demand one at a time.
pub const LMD_PHASES: [&str; 4] = ["red", "green", "refactor", "rationalizations"];

/// Variant B (lmd phased rendering): stub SKILL.md + each rendered phase +
/// the rendered companion. Rendering is in-process (Consumer::Ai, crp=None).
pub fn collect_variant_b(stub_md: &Path, jail_root: PathBuf) -> Vec<Artifact> {
    let mut arts = Vec::new();

    if let Ok(text) = std::fs::read_to_string(stub_md) {
        arts.push(Artifact::from_text("SKILL.md (stub)", &text));
    }

    for phase in LMD_PHASES {
        let rendered = render_skill(
            LMD_SKILL,
            Some(phase),
            Some(Consumer::Ai),
            None,
            jail_root.clone(),
        )
        .unwrap_or_else(|e| panic!("render phase {phase}: {e}"));
        arts.push(Artifact::from_text(&format!("phase:{phase}"), &rendered));
    }

    let companion = render_companion(
        LMD_SKILL,
        LMD_COMPANION,
        Some(Consumer::Ai),
        None,
        jail_root,
    )
    .unwrap_or_else(|e| panic!("render companion: {e}"));
    arts.push(Artifact::from_text(
        &format!("companion:{LMD_COMPANION}"),
        &companion,
    ));

    arts
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml skill_token_comparison`
Expected: PASS.

- [ ] **Step 5: Format + Commit**

```bash
cargo fmt --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml
git add benchmarks/skill-token-comparison/harness.rs tests/skill_token_comparison.rs
git commit -m "feat(bench): collect_variant_b — in-process render of lmd phases + companion"
```

---

### Task 4: Metrik-Berechnung (Kernmetrik, Overhead-Modell, Break-even)

Berechnet aus den Artefakt-Listen die 2-Zeilen-Kernmetrik (reiner Inhalt / inkl. Tool-Call-Overhead) und die kumulativen B-Kosten je Phasenzahl k=1..4 für den Break-even.

**Files:**
- Modify: `benchmarks/skill-token-comparison/harness.rs`
- Modify: `tests/skill_token_comparison.rs`

**Interfaces:**
- Consumes: `Artifact` (Task 2), `LMD_PHASES` (Task 3)
- Produces:
  - `pub const TOOL_CALL_OVERHEAD_TOKENS: usize = 40;` (geschätzter Roundtrip-Overhead pro `ctx_md_render`-Aufruf; im Report offengelegt + justierbar)
  - `pub struct Metrics { pub a_content: usize, pub a_with_overhead: usize, pub b_content: usize, pub b_with_overhead: usize, pub b_cumulative: Vec<(usize, usize, usize)> }`
    - `b_cumulative[i]` = `(k, content_k, with_overhead_k)` für k=1..=4 erreichte Phasen (Stub immer mitgezählt; with_overhead addiert `k * TOOL_CALL_OVERHEAD_TOKENS`). Familie: cl100k.
  - `pub fn compute_metrics(variant_a: &[Artifact], variant_b: &[Artifact]) -> Metrics`
  - Annahmen (in cl100k gerechnet): A = 1 Skill-Load (Overhead `1 * TOOL_CALL_OVERHEAD_TOKENS`). B-Stub-Artefakt = `name == "SKILL.md (stub)"`, B-Phasen = `name` startet mit `"phase:"`, Companion zählt NICHT in `b_cumulative` (wird nur on-demand geladen), aber in `b_content`/`b_with_overhead` (Vollausbau) mit.

- [ ] **Step 1: Write the failing test**

In `tests/skill_token_comparison.rs` anhängen:

```rust
use harness::{compute_metrics, Metrics, TOOL_CALL_OVERHEAD_TOKENS};

fn art(name: &str, t: usize) -> Artifact {
    Artifact { name: name.to_string(), tokens_cl100k: t, tokens_o200k: t }
}

#[test]
fn compute_metrics_core_and_breakeven() {
    // A: SKILL.md=100, companion=50  → content 150
    let a = vec![art("SKILL.md", 100), art("testing-anti-patterns.md", 50)];
    // B: stub=10, 4 phases=20 each, companion=30
    let b = vec![
        art("SKILL.md (stub)", 10),
        art("phase:red", 20),
        art("phase:green", 20),
        art("phase:refactor", 20),
        art("phase:rationalizations", 20),
        art("companion:testing-anti-patterns", 30),
    ];

    let m: Metrics = compute_metrics(&a, &b);

    assert_eq!(m.a_content, 150);
    assert_eq!(m.a_with_overhead, 150 + TOOL_CALL_OVERHEAD_TOKENS); // 1 load
    // B full content = 10 + 80 + 30 = 120
    assert_eq!(m.b_content, 120);
    // B full overhead = 4 render calls (phases) + 1 companion render = 5
    assert_eq!(m.b_with_overhead, 120 + 5 * TOOL_CALL_OVERHEAD_TOKENS);

    // Cumulative (stub + k phases, companion excluded): k=1 → 10+20=30 content
    assert_eq!(m.b_cumulative[0], (1, 30, 30 + 1 * TOOL_CALL_OVERHEAD_TOKENS));
    assert_eq!(m.b_cumulative[3], (4, 90, 90 + 4 * TOOL_CALL_OVERHEAD_TOKENS));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml skill_token_comparison`
Expected: FAIL — `compute_metrics` / `Metrics` / `TOOL_CALL_OVERHEAD_TOKENS` not found.

- [ ] **Step 3: Write minimal implementation**

In `benchmarks/skill-token-comparison/harness.rs` anhängen:

```rust
/// Estimated extra tokens per ctx_md_render roundtrip (tool-use block:
/// name + {skill, phase} args + tool_result wrapper). Disclosed in SUMMARY.md
/// and tunable — this is a model assumption, not a measured constant.
pub const TOOL_CALL_OVERHEAD_TOKENS: usize = 40;

/// Aggregated A/B comparison (all token sums in cl100k).
#[derive(Debug, Clone)]
pub struct Metrics {
    pub a_content: usize,
    pub a_with_overhead: usize,
    pub b_content: usize,
    pub b_with_overhead: usize,
    /// (k, cumulative_content, cumulative_with_overhead) for k=1..=phase count.
    /// Stub always included; companion excluded (on-demand only).
    pub b_cumulative: Vec<(usize, usize, usize)>,
}

fn sum_cl100k(arts: &[Artifact]) -> usize {
    arts.iter().map(|a| a.tokens_cl100k).sum()
}

/// Compute the core metric + cumulative break-even table.
pub fn compute_metrics(variant_a: &[Artifact], variant_b: &[Artifact]) -> Metrics {
    let a_content = sum_cl100k(variant_a);
    let a_with_overhead = a_content + TOOL_CALL_OVERHEAD_TOKENS; // single skill load

    let stub = variant_b
        .iter()
        .find(|a| a.name == "SKILL.md (stub)")
        .map(|a| a.tokens_cl100k)
        .unwrap_or(0);
    let phases: Vec<usize> = variant_b
        .iter()
        .filter(|a| a.name.starts_with("phase:"))
        .map(|a| a.tokens_cl100k)
        .collect();
    let companion: usize = variant_b
        .iter()
        .filter(|a| a.name.starts_with("companion:"))
        .map(|a| a.tokens_cl100k)
        .sum();

    let b_content = stub + phases.iter().sum::<usize>() + companion;
    // Full-build overhead = one render call per phase + one per companion.
    let full_calls = phases.len() + variant_b.iter().filter(|a| a.name.starts_with("companion:")).count();
    let b_with_overhead = b_content + full_calls * TOOL_CALL_OVERHEAD_TOKENS;

    let mut b_cumulative = Vec::new();
    let mut running = stub;
    for (i, ph) in phases.iter().enumerate() {
        let k = i + 1;
        running += ph;
        b_cumulative.push((k, running, running + k * TOOL_CALL_OVERHEAD_TOKENS));
    }

    Metrics {
        a_content,
        a_with_overhead,
        b_content,
        b_with_overhead,
        b_cumulative,
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml skill_token_comparison`
Expected: PASS.

- [ ] **Step 5: Format + Commit**

```bash
cargo fmt --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml
git add benchmarks/skill-token-comparison/harness.rs tests/skill_token_comparison.rs
git commit -m "feat(bench): compute_metrics — core metric, overhead model, break-even table"
```

---

### Task 5: Report-Formatierung (`format_summary`, deterministisch)

Erzeugt den `SUMMARY.md`-Body als reine Funktion der Metriken + Artefakte. Keine Timestamps/Zähler (#498).

**Files:**
- Modify: `benchmarks/skill-token-comparison/harness.rs`
- Modify: `tests/skill_token_comparison.rs`

**Interfaces:**
- Consumes: `Artifact`, `Metrics`, `TOOL_CALL_OVERHEAD_TOKENS`
- Produces:
  - `pub fn format_summary(variant_a: &[Artifact], variant_b: &[Artifact], metrics: &Metrics) -> String`
  - Enthält: Überschrift, Annahmen-Block (Tokenizer cl100k primär/o200k Parität, Overhead-Konstante), Per-Artefakt-Tabelle (beide Familien), Kernmetrik-Tabelle (content / inkl. Overhead, A vs. B, Δ), Break-even-Tabelle (k=1..4). Kein Timestamp.

- [ ] **Step 1: Write the failing test**

In `tests/skill_token_comparison.rs` anhängen:

```rust
use harness::format_summary;

#[test]
fn format_summary_is_deterministic_and_has_sections() {
    let a = vec![art("SKILL.md", 100), art("testing-anti-patterns.md", 50)];
    let b = vec![
        art("SKILL.md (stub)", 10),
        art("phase:red", 20),
        art("phase:green", 20),
        art("phase:refactor", 20),
        art("phase:rationalizations", 20),
        art("companion:testing-anti-patterns", 30),
    ];
    let m = compute_metrics(&a, &b);

    let s1 = format_summary(&a, &b, &m);
    let s2 = format_summary(&a, &b, &m);

    assert_eq!(s1, s2, "summary must be byte-stable (#498)");
    assert!(s1.contains("# Skill-Token-Vergleich"));
    assert!(s1.contains("cl100k_base"));
    assert!(s1.contains("o200k_base"));
    assert!(s1.contains("Break-even"));
    assert!(s1.contains("Reiner Inhalt"));
    // No timestamp/date marker in the body (#498).
    assert!(!s1.contains("Datum:"), "no date marker expected");
    assert!(!s1.contains("Generated"), "no generation timestamp expected");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml skill_token_comparison`
Expected: FAIL — `format_summary` not found.

- [ ] **Step 3: Write minimal implementation**

In `benchmarks/skill-token-comparison/harness.rs` anhängen:

```rust
use std::fmt::Write as _;

/// Render the deterministic SUMMARY.md body (no timestamps, #498).
pub fn format_summary(variant_a: &[Artifact], variant_b: &[Artifact], metrics: &Metrics) -> String {
    let mut s = String::new();

    s.push_str("# Skill-Token-Vergleich — SUMMARY\n\n");
    s.push_str("Neutrales A/B: A = superpowers (Monolith), B = lmd (Phasen-Rendering).\n\n");

    s.push_str("## Annahmen\n\n");
    s.push_str("- Tokenizer: `cl100k_base` (primär, ~3% von Claudes echtem Tokenizer); ");
    s.push_str("`o200k_base` (Parität mit lean-ctx-Ledger).\n");
    let _ = writeln!(
        s,
        "- Tool-Call-Overhead pro `ctx_md_render`-Roundtrip: {} Tokens (Modellannahme, justierbar).\n",
        TOOL_CALL_OVERHEAD_TOKENS
    );

    s.push_str("## Artefakte (Tokens je Familie)\n\n");
    s.push_str("| Variante | Artefakt | cl100k | o200k |\n|---|---|---|---|\n");
    for a in variant_a {
        let _ = writeln!(s, "| A | {} | {} | {} |", a.name, a.tokens_cl100k, a.tokens_o200k);
    }
    for b in variant_b {
        let _ = writeln!(s, "| B | {} | {} | {} |", b.name, b.tokens_cl100k, b.tokens_o200k);
    }
    s.push('\n');

    s.push_str("## Kernmetrik (cl100k)\n\n");
    s.push_str("| Metrik | A (superpowers) | B (lmd, Vollausbau) | Δ (B−A) |\n|---|---|---|---|\n");
    let _ = writeln!(
        s,
        "| Reiner Inhalt | {} | {} | {} |",
        metrics.a_content,
        metrics.b_content,
        metrics.b_content as i64 - metrics.a_content as i64
    );
    let _ = writeln!(
        s,
        "| Inkl. Ablauf-Overhead | {} | {} | {} |",
        metrics.a_with_overhead,
        metrics.b_with_overhead,
        metrics.b_with_overhead as i64 - metrics.a_with_overhead as i64
    );
    s.push('\n');

    s.push_str("## Break-even (B kumulativ, Stub + k Phasen)\n\n");
    s.push_str("| k Phasen | B Inhalt | B inkl. Overhead | vs. A Inhalt | vs. A inkl. Overhead |\n|---|---|---|---|---|\n");
    for (k, content, with_oh) in &metrics.b_cumulative {
        let c_cmp = if *content <= metrics.a_content { "B billiger" } else { "B teurer" };
        let o_cmp = if *with_oh <= metrics.a_with_overhead { "B billiger" } else { "B teurer" };
        let _ = writeln!(s, "| {k} | {content} | {with_oh} | {c_cmp} | {o_cmp} |");
    }
    s.push('\n');

    s
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml skill_token_comparison`
Expected: PASS.

- [ ] **Step 5: Format + Commit**

```bash
cargo fmt --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml
git add benchmarks/skill-token-comparison/harness.rs tests/skill_token_comparison.rs
git commit -m "feat(bench): format_summary — deterministic SUMMARY.md body"
```

---

### Task 6: `main()` verdrahten + `SUMMARY.md` schreiben

Verbindet alles mit den echten Pfaden und schreibt den Report. Verifikation per Example-Lauf (kein nextest-Test für `main` selbst).

**Files:**
- Modify: `benchmarks/skill-token-comparison/main.rs`

**Interfaces:**
- Consumes: alle `harness`-Funktionen (Tasks 1–5)

- [ ] **Step 1: `main()` implementieren**

`benchmarks/skill-token-comparison/main.rs` ersetzen durch:

```rust
//! Skill token-comparison benchmark (Schicht A). Renders both TDD skill
//! variants in-process, tokenizes the artifacts, writes SUMMARY.md.
//! tiktoken-rs is a dev-dependency; this code lives ONLY in the example
//! target (never bundled into the lean_md lib/bin). No #[cfg] gate needed.

#[path = "harness.rs"]
mod harness;

use std::path::{Path, PathBuf};

const SP_SKILL: &str = "/home/tholo/.claude/plugins/cache/claude-plugins-official/superpowers/6.0.3/skills/test-driven-development/SKILL.md";
const SP_COMPANION: &str = "/home/tholo/.claude/plugins/cache/claude-plugins-official/superpowers/6.0.3/skills/test-driven-development/testing-anti-patterns.md";

fn main() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stub = repo_root.join("content/skills/lmd-test-driven-development/SKILL.md");
    let out = repo_root.join("benchmarks/skill-token-comparison/SUMMARY.md");

    let variant_a = harness::collect_variant_a(Path::new(SP_SKILL), Path::new(SP_COMPANION));
    let variant_b = harness::collect_variant_b(&stub, repo_root.clone());
    let metrics = harness::compute_metrics(&variant_a, &variant_b);
    let summary = harness::format_summary(&variant_a, &variant_b, &metrics);

    std::fs::write(&out, summary).expect("write SUMMARY.md");
    println!("wrote {}", out.display());
}
```

- [ ] **Step 2: Example bauen**

Run: `cargo build --example skill-token-comparison --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml`
Expected: kompiliert ohne Fehler/Warnungen.

- [ ] **Step 3: Example ausführen**

Run: `cargo run --example skill-token-comparison --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml`
Expected: Ausgabe `wrote …/benchmarks/skill-token-comparison/SUMMARY.md`; Datei existiert.

- [ ] **Step 4: Determinismus prüfen (zweiter Lauf identisch)**

Run: `cargo run --example skill-token-comparison --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml`
Dann: `git -C /home/tholo/Scripts/lean-md status --short benchmarks/skill-token-comparison/SUMMARY.md`
Expected: nach dem zweiten Lauf KEINE Änderung an `SUMMARY.md` (byte-stabil) — falls die Datei in Step 3 neu war, ist sie nach Step 4 unverändert.

- [ ] **Step 5: Format + Commit (inkl. generierter SUMMARY.md)**

```bash
cargo fmt --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml
git add benchmarks/skill-token-comparison/main.rs benchmarks/skill-token-comparison/SUMMARY.md
git commit -m "feat(bench): wire main() — write deterministic SUMMARY.md (Schicht A)"
```

---

### Task 7: Schicht-B-Protokoll (README, kein Code)

Dokumentiert das mdai-adaptierte Subagent-Druck-Varianten-Protokoll (`cold`/`time`/`authority`) und wie die Reports abgelegt werden. Reproduzierbarkeit der validierenden Schicht.

**Files:**
- Create: `benchmarks/skill-token-comparison/README.md`

- [ ] **Step 1: README schreiben**

Create `benchmarks/skill-token-comparison/README.md`:

```markdown
# Skill-Token-Vergleich — Benchmark

Neutrales A/B: **A** = `superpowers/test-driven-development` (Monolith),
**B** = `lmd-test-driven-development` (Phasen-Rendering).

## Schicht A — deterministischer Trace (automatisiert)

    cargo run --example skill-token-comparison

Rendert Variante B in-process, tokenisiert beide Varianten (tiktoken-rs:
`cl100k_base` primär, `o200k_base` Parität) und schreibt `SUMMARY.md`
(byte-stabil, #498). Test: `cargo nextest run skill_token_comparison`.

Annahme `TOOL_CALL_OVERHEAD_TOKENS` (Roundtrip-Overhead pro `ctx_md_render`)
ist in `harness.rs` benannt und im `SUMMARY.md` offengelegt — justierbar.

## Schicht B — Subagent-Validierung (manuell, mdai-adaptiert)

Dieselbe Mini-TDD-Aufgabe (eine kleine Funktion + ein Bugfix) wird je Variante
gelöst, einmal pro Druck-Variante:

| Variante | Bedeutung |
|---|---|
| `cold`      | keine Beschränkung, freie Bearbeitung |
| `time`      | expliziter Zeitdruck im Prompt |
| `authority` | Tech-Lead-Override im Prompt ("mach es direkt, ohne Zeremonie") |

Jeder Subagent-Report hält **verbatim** fest: welche Skill-Artefakte real
geladen wurden (Variante B: welche Phasen tatsächlich gerendert wurden) und
wie viele Tool-Calls anfielen. Die geladenen Artefakte werden mit derselben
`harness`-Zählung nachgezählt → realer kumulierter Verbrauch.

Reports ablegen unter:

    variant-A-superpowers/<cold|time|authority>.md
    variant-B-lmd/<cold|time|authority>.md

Zweck: bestätigt/falsifiziert die Schicht-A-Hypothese — stoppen reale Agenten
bei RED/GREEN (dann lädt B `refactor`/`rationalizations` nie), und ist der
Tool-Call-Overhead kleiner als die eingesparten Inhalts-Tokens?
```

- [ ] **Step 2: Commit**

```bash
git add benchmarks/skill-token-comparison/README.md
git commit -m "docs(bench): Schicht-B subagent pressure-variant protocol"
```

---

## Self-Review

**Spec-Coverage:**
- §2 Kernmetrik (2 Zeilen + Break-even) → Task 4 (`compute_metrics`) + Task 5 (Tabellen). ✓
- §3 mdai-Druck-Varianten → Task 7 (README-Protokoll). ✓
- §4 In-process Rust-Harness, Cargo-Example, tiktoken-rs dev-dep → Task 1 (Wiring) + Tasks 2–6. ✓
- §4.1 cl100k primär / o200k Parität → Task 1 (`Family`) + Task 5 (beide Spalten). ✓
- §4.1 kein cfg-Gate, tiktoken nur dev-dep → Global Constraints + Task 1. ✓
- §4.2 Schicht A (Variante A von Disk, Variante B in-process) → Tasks 2–3, verdrahtet in Task 6. ✓
- §4.2 Overhead-Konstante benannt + offengelegt → Task 4 (`TOOL_CALL_OVERHEAD_TOKENS`) + Task 5. ✓
- §4.3 Schicht B → Task 7. ✓
- §4.4 SUMMARY.md-Tabellen → Task 5. ✓
- §6 Erfolgskriterien (byte-stabiler Report, eindeutige Antworten) → Task 5 (Determinismus-Test) + Task 6 (Lauf). ✓

**Platzhalter-Scan:** keine TBD/TODO; jeder Code-Step zeigt vollständigen Code. ✓

**Typ-Konsistenz:** `Family`, `Artifact{name,tokens_cl100k,tokens_o200k}`, `Metrics{a_content,a_with_overhead,b_content,b_with_overhead,b_cumulative}`, `LMD_PHASES`, `TOOL_CALL_OVERHEAD_TOKENS`, Funktions-Signaturen `token_count`/`collect_variant_a`/`collect_variant_b`/`compute_metrics`/`format_summary` — über alle Tasks identisch verwendet. ✓
