@lean-md
consumer: ai
crp: compact

@var test_cmd default="cargo nextest run"
@var lint_cmd default="cargo clippy --all-targets -- -D warnings"
@import .lean-ctx/lean-md/plan-recipes /

# P3 — `kind=skills`-Pack (Full-Cut) — Implementation Plan

## Goal

Skill-Content (8 Bodies, 17 Companions, 8 `SKILL.md`-Stubs, 6 Assets, 3 skill-lokale
`_includes/`) verlässt das Binary und wird zum signierten, versionierten
`kind=skills`-Pack `@dasTholo/lean-md-skills`, deklariert als depth-1-Dependency des
Addons `@dasTholo/lean-md`. Skill-Updates ohne Binary-Release (#727).

Spec: `docs/lean-md/specs/2026-07-08-lmd-p3-skills-pack-full-cut-design.md`.

## Architecture

- **Neue Datei** `src/skill_source.rs` — die 3-Stufen-Kaskade als eine fokussierte Einheit:
  Overlay (`<jail_root>/.lean-ctx/lean-md/skills/`) → Pack-Store (`$LEAN_MD_SKILLS_DIR`) →
  Debug-Fallback (`$CARGO_MANIFEST_DIR/content/skills`, nur `cfg(debug_assertions)`).
  Alle drei Konsumenten (`skills.rs`, `fragments.rs`, `skill_install.rs`) lesen durch sie.
- **Pack-Layout (am lean-ctx-Quellcode verifiziert, nicht angenommen):**
  `collect_files` (`context_package/skills.rs:187`) sammelt Relativpfade zum `--from`-Root;
  `materialize_documents` (`:229`) schreibt sie unter `skills_dir(store, name, version)`.
  ∴ `pack create --from content/skills` ⇒ `$LEAN_MD_SKILLS_DIR/<skill>/body.lmd.md`,
  `…/<skill>/companions/<c>.lmd.md`, `…/<skill>/_includes/<n>.lmd.md`, `…/<skill>/SKILL.md`.
  **Kein** `skills/`-Präfix im Baum. R2 der Spec ist damit geschlossen.
- **Pfad-Handoff:** lean-ctx expandiert `{pack_dir:@dasTholo/lean-md-skills}` aus
  `[mcp.env]` beim Wiring zum absoluten `skills_dir` (`core/addons/pack_env.rs`,
  `expand_pack_env`). lean-md kennt nur eine Env-Var, kein Store-Layout, kein Lockfile.
- **Registry-Umbau:** `SKILLS`/`COMPANIONS`/`INSTALLABLE_SKILLS`/`ASSETS` tragen künftig
  **Namen**, keine `&'static str`-Bodies. Der Relativpfad ist aus dem Namen ableitbar
  (`<skill>/body.lmd.md` usw.) — deshalb schrumpfen die Tabellen auf Namenslisten.
- **Zwei Release-Regime:** `content/skills/**` → nur Pack (`pack create` + `pack publish`,
  kein Tag, kein `sync-manifest`); `content/core/**` + `content/gloss/**` + `src/**` →
  Binary (Tag `v*`, 5-leg-Build, SHA-Rückfluss).
- **Drift-Gate zweistufig:** (a) `cargo nextest`-Test gegen das checked-in Content-Manifest
  `content/skills.sha256`; (b) CI-Cross-Check gegen den echten lean-ctx-`content_hash`.

## Global Constraints

- **Vorbedingungen (extern, nicht Teil dieses Plans).** Tasks 3–5 sind der irreversible
  `include_str!`-Cut und dürfen erst laufen, wenn alle grün sind:
  - **V1** lean-ctx-Upstream-Vertrag **released**. Code-complete auf `pr-rebuild`
    (`b05bc5f14` pack_env, `38699d7ce` `[[dependencies]]`, `a851718b1` min_lean_ctx-Gate,
    `c87f99950` Install-Order) — der **Release** steht aus. **Harte Sperre.**
  - **V2** lean-md-Release `v0.2.0` mit echten SHA-256 in allen 5 `[artifacts]`-Blöcken.
  - **V3** curated Registry-Entry auf `listed` zurückgestuft (lean-ctx-Repo).
  - **V4** `@dasTholo/lean-md` **und** `@dasTholo/lean-md-skills` hosted publiziert.
- **Non-goal:** Skill-Tiering; P4 (Signing/Publisher-Identität); Verschieben von
  `content/lang` / `content/tooling` in den Pack; der curated-Registry-`artifacts`-Fix.
- **`seeds.rs` / `PROJECT_SEEDS` bleiben unangetastet.** Der Projekt-Seed-Kanal
  (`.lean-ctx/lean-md/`) ist ein eigener Kanal — `plan-recipes` / `plan-template` sind
  render-`@import`-Ziele und **müssen** projekt-lokal bleiben.
- **Embedded bleibt (bewusst):** `content/core/hard-rules`, `content/core/dispatch-contract`,
  `content/core/_fragments/parallel-dispatch`, `content/gloss/directives`. Der
  #498-Fragment-Consistency-Gate (built-in == on-disk) muss für diese **drei** cross-skill
  Builtins grün bleiben. Sie sind der Grund, dass ein Standalone-Render ohne Pack lauffähig ist.
- **Reverse-Cut:** lean-md nimmt **keine** `lean_ctx`-Dependency auf. Der `content_hash` der
  lean-ctx-Packs wird deshalb **nicht** in Rust nachgerechnet — der lokale Gate hat einen
  eigenen, unabhängigen sha256 (Task 6a), der CI-Cross-Check ruft das lean-ctx-Binary (6b).
- **`content/skills.sha256` liegt NEBEN `content/skills/`, niemals darin** — `collect_files`
  packt jede Datei unter dem `--from`-Root ein; eine Manifest-Datei im Baum wäre selbst
  Pack-Inhalt und der Hash würde sich rekursiv selbst invalidieren.
- **Produktion fehlt der Pack ⇒ harter Fehler**, kein stiller Leerlauf. Im Dev-Build greift
  zuvor der Debug-Fallback; im Release-Binary ist er inert (`cfg(debug_assertions)` aus).
- **Cross-Task-Prerequisite:** 2 → 3 → 4 → 5 → 6. Task 2 liefert `read_skill_file`, das
  Tasks 3–5 konsumieren. Task 1 ist unabhängig (kein `src/`), Task 7 ist Maintainer-Hand.
- **`min_lean_ctx = "3.9.4"`** — entschieden: die #727-Commits werden mit `3.9.4`
  ausgeliefert (`rust/Cargo.toml:23`). `preflight` vergleicht per `version_lt` gegen
  `env!("CARGO_PKG_VERSION")` und **bricht ab**, nicht warnt.

@phase "task-1"
## Task 1: Addon-Manifest — `[[dependencies]]` + `[mcp.env]` + `min_lean_ctx`

**Files:** Edit `lean-ctx-addon.toml`. Kein `src/`-Code.

**Consumes:** das bestehende Manifest (5 `[artifacts]`-Blöcke, `min_lean_ctx = "3.9.2"`,
`[mcp]` ohne `env`). **Produces:** ein `[[dependencies]]`-Block, ein `[mcp.env]`-Block mit
`{pack_dir:…}`-Platzhalter, `min_lean_ctx = "3.9.4"`.

**Authoring-Key ist `version_req`, nicht `version`** — es gibt keinen serde-Alias
(`docs/contracts/addon-manifest-v1.md`, lean-ctx `38699d7ce`). Ein falscher Key wird
still verworfen (kein `deny_unknown_fields`).

Ersetze exakt diese Zeile (Edit-Anker, bestehender Content):

    min_lean_ctx = "3.9.2"

durch (NEUER Content, verbatim):

    min_lean_ctx = "3.9.4"

Ersetze exakt diesen Block (Edit-Anker, bestehender Content):

    [mcp]
    transport = "stdio"
    command = "lean-md"
    args = ["mcp"]

durch (NEUER Content, verbatim):

    # Skill-Content lebt seit P3 (#727) im Pack, nicht im Binary. Der Resolver löst
    # `version_req` depth-1 gegen den Registry-Index auf; ein Pack, das nur als
    # GitHub-Asset existiert, ist für ihn unsichtbar → hosted publish ist Pflicht.
    [[dependencies]]
    name        = "@dasTholo/lean-md-skills"
    version_req = "^0.2"
    optional    = false

    [mcp]
    transport = "stdio"
    command = "lean-md"
    args = ["mcp"]

    # lean-ctx expandiert den Platzhalter beim Wiring zum absoluten `skills_dir` des
    # materialisierten Packs (core/addons/pack_env.rs). Ein `{` ohne `}` oder ein
    # unbekanntes Schema ist ein harter Parse-Fehler — kein Literal-Escape existiert.
    [mcp.env]
    LEAN_MD_SKILLS_DIR = "{pack_dir:@dasTholo/lean-md-skills}"

**Nicht anfassen:** `[addon].version = "0.2.0"` (Binary-Linie), die 5 `[artifacts]`-Blöcke
(SHA-Pins kommen aus `sync-manifest`), `[capabilities]`.

**Versions-Kopplung (Decision-Record):** Pack und Binary tragen **getrennte** SemVer-Linien,
initial beide `0.2.0` — ein bequemer Startpunkt, **kein Vertrag**. `version_req = "^0.2"`
deckt auf der `0.x`-Linie `>=0.2.0, <0.3.0`. Ein reiner Content-Fix hebt den Pack auf
`0.2.1`; das Manifest wird **nicht** angefasst, der Addon-Pack **nicht** republiziert.

**Curated-Registry-Entry (Decision-Record, Umsetzung im lean-ctx-Repo):** nach dem Cut hat
ein via curated `addon add lean-md` installiertes Binary keinen Skill-Content und zieht den
Pack nicht (Deps feuern nur auf dem hosted-`@ns/name`-Pfad). Der Entry wird auf **`listed`**
zurückgestuft (Homepage-Pointer, kein one-click), bis der hosted Pack live ist.

### Verify & Close

@call verify("lean-ctx-addon.toml")

**Expected** (dependency-frei, `tomllib` ist stdlib ≥ 3.11):

    python3 -c "import tomllib; m=tomllib.load(open('lean-ctx-addon.toml','rb')); d=m['dependencies']; assert len(d)==1, d; assert d[0]['name']=='@dasTholo/lean-md-skills'; assert d[0]['version_req']=='^0.2', 'key MUST be version_req'; assert d[0]['optional'] is False; assert m['mcp']['env']['LEAN_MD_SKILLS_DIR']=='{pack_dir:@dasTholo/lean-md-skills}'; assert m['addon']['min_lean_ctx']=='3.9.4'; assert len(m['artifacts'])==5; print('OK dep+env+gate')"

→ `OK dep+env+gate`

@call commit("lean-ctx-addon.toml", "feat(dist): declare @dasTholo/lean-md-skills as depth-1 dependency (#727)")
@phase-end

@phase "task-2"
## Task 2: `src/skill_source.rs` — die 3-Stufen-Kaskade (neue Datei, TDD)

**Files:** Create `src/skill_source.rs`. Edit `src/lib.rs` (Modul registrieren).

**Interfaces / Produces:**
- `pub const SKILLS_DIR_ENV: &str = "LEAN_MD_SKILLS_DIR"`
- `pub enum SourceError { PackMissing(String), NotFound(String), Io(String) }` (+ `Display`)
- `pub fn pack_store_root() -> Option<PathBuf>`
- `pub fn debug_fallback_root() -> Option<PathBuf>`
- `pub fn read_skill_file(rel: &str, jail_root: &Path) -> Result<String, SourceError>`

**Consumes:** `crate::pathx::jail_path(candidate, jail_root) -> Result<PathBuf, String>`
(`src/pathx.rs:14`) für die Overlay-Stufe.

**Keine Caller in dieser Task** — Tasks 3–5 hängen sich an. Diese Task ist rein additiv und
für sich testbar.

@call tdd("overlay wins over pack store")

@call tdd("pack store resolves when no overlay exists")

@call tdd("unset LEAN_MD_SKILLS_DIR in a release build is PackMissing")

@call tdd("a set-but-nonexistent LEAN_MD_SKILLS_DIR names the var in its error")

@call tdd("a missing file inside an existing pack root is NotFound, not PackMissing")

Tests laufen unter `cargo nextest run` — ein Prozess pro Test, deshalb ist die
prozessglobale Env-Mutation isoliert. Nutze den vorhandenen Helper
`crate::test_env::set_var` / `remove_var` (`src/test_env.rs`), niemals `unsafe` inline.

NEUER Content, verbatim (`src/skill_source.rs`):

    //! Skill-content resolution (#727): overlay → pack store → debug fallback.
    //!
    //! Since P3 the skill bodies, companions, `SKILL.md` stubs, assets and the
    //! skill-local `_includes/` live in the `kind=skills` pack
    //! `@dasTholo/lean-md-skills`, not in this binary. lean-ctx materializes the
    //! pack and hands its absolute directory over in one environment variable —
    //! this module is the only place that knows about it.
    //!
    //! Cross-skill core primitives (`hard-rules`, `dispatch-contract`,
    //! `parallel-dispatch`) and `gloss/directives` stay `include_str!`-embedded:
    //! a general `.lmd.md` render must work in every distribution path, with or
    //! without a pack.

    use std::path::{Path, PathBuf};

    /// Absolute `skills_dir` of the materialized pack. lean-ctx expands it from the
    /// `{pack_dir:@dasTholo/lean-md-skills}` placeholder in `[mcp.env]` at wiring
    /// time (`core/addons/pack_env.rs`). lean-md never derives the store layout.
    pub const SKILLS_DIR_ENV: &str = "LEAN_MD_SKILLS_DIR";

    #[derive(Debug)]
    pub enum SourceError {
        /// No content root at all — production install is broken. Actionable, never silent.
        PackMissing(String),
        /// A content root exists, but it does not carry this relative path.
        NotFound(String),
        Io(String),
    }

    impl std::fmt::Display for SourceError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                SourceError::PackMissing(m) => write!(f, "PACK_MISSING {m}"),
                SourceError::NotFound(p) => write!(f, "SKILL_FILE_NOT_FOUND '{p}'"),
                SourceError::Io(e) => write!(f, "SKILL_FILE_IO {e}"),
            }
        }
    }

    /// The materialized pack root, if lean-ctx wired one and it exists on disk.
    pub fn pack_store_root() -> Option<PathBuf> {
        let raw = std::env::var(SKILLS_DIR_ENV).ok()?;
        if raw.is_empty() {
            return None;
        }
        let root = PathBuf::from(raw);
        root.is_dir().then_some(root)
    }

    /// Dev-only content root: `$CARGO_MANIFEST_DIR/content/skills`. Inert in a release
    /// binary — `debug_assertions` is off there and the path does not exist on a user's
    /// machine either. Both guards must fail for production to reach `PackMissing`.
    pub fn debug_fallback_root() -> Option<PathBuf> {
        if !cfg!(debug_assertions) {
            return None;
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("content/skills");
        root.is_dir().then_some(root)
    }

    /// Jailed project overlay root — local phase iteration without a pack republish.
    fn overlay_root(jail_root: &Path) -> PathBuf {
        jail_root.join(".lean-ctx/lean-md/skills")
    }

    fn content_root() -> Result<PathBuf, SourceError> {
        if let Some(pack) = pack_store_root() {
            return Ok(pack);
        }
        if let Some(dev) = debug_fallback_root() {
            return Ok(dev);
        }
        Err(SourceError::PackMissing(match std::env::var(SKILLS_DIR_ENV) {
            Ok(raw) if !raw.is_empty() => format!(
                "{SKILLS_DIR_ENV}={raw} is not a directory — reinstall the addon: \
                 `lean-ctx addon add @dasTholo/lean-md`"
            ),
            _ => format!(
                "{SKILLS_DIR_ENV} is unset — the skills pack was never wired. \
                 Reinstall the addon: `lean-ctx addon add @dasTholo/lean-md`"
            ),
        }))
    }

    /// Read one skill-content file by its pack-relative path (e.g.
    /// `lmd-brainstorm/body.lmd.md`) through the three-stage cascade.
    ///
    /// Stage 1 is PathJail-bound: an overlay may never reach outside `jail_root`.
    /// Stages 2 and 3 take `rel` from a static registry, never from user input.
    pub fn read_skill_file(rel: &str, jail_root: &Path) -> Result<String, SourceError> {
        let overlay = overlay_root(jail_root).join(rel);
        if let Ok(resolved) = crate::pathx::jail_path(&overlay, jail_root)
            && resolved.is_file()
        {
            return std::fs::read_to_string(&resolved).map_err(|e| SourceError::Io(e.to_string()));
        }
        let candidate = content_root()?.join(rel);
        if !candidate.is_file() {
            return Err(SourceError::NotFound(rel.to_string()));
        }
        std::fs::read_to_string(&candidate).map_err(|e| SourceError::Io(e.to_string()))
    }

`src/lib.rs` — füge die Modul-Zeile alphabetisch zwischen `skill_install` und `skill_vars`
ein (Edit-Anker, bestehender Content):

    pub mod skill_install;
    pub mod skill_vars;

wird zu (NEUER Content, verbatim):

    pub mod skill_install;
    pub mod skill_source;
    pub mod skill_vars;

### Verify & Close

@call verify("src/skill_source.rs src/lib.rs")

**Expected — Debug-Fallback greift im Dev-Build, ohne dass eine Env-Var gesetzt ist:**

    cargo run -q --bin lean-md -- render --skill lmd-brainstorm --phase pre-context --consumer=ai

→ nicht-leerer Render (Task 3 verdrahtet den Pfad; hier prüft der Test-Suite-Lauf die Einheit).

@call gate("src/skill_source.rs src/lib.rs")

@call commit("src/skill_source.rs src/lib.rs", "feat(skills): three-stage skill-content cascade (overlay/pack/debug) (#727)")
@phase-end

@phase "task-3"
## Task 3: `src/skills.rs` — Bodies + Companions aus dem Binary cutten

**Files:** Edit `src/skills.rs`, `src/bin/lean_md.rs`, `src/bridges/dispatch.rs`.

**Consumes:** `crate::skill_source::{read_skill_file, SourceError}` (Task 2).
**Produces:** namensbasierte Registries + drei quellenagnostische Funktionen:

    pub fn skill_source(name: &str, jail_root: &Path) -> Result<String, SkillRenderError>
    pub fn companion_source(skill: &str, companion: &str, jail_root: &Path) -> Result<String, SkillRenderError>
    pub fn all_skill_sources(jail_root: &Path) -> Result<Vec<String>, SkillRenderError>

**Zu löschen:** die 25 `include_str!`-Konstanten `LMD_*` (`src/skills.rs:12-66`, `:19-184`),
die `&'static str`-Spalten in `SKILLS` / `COMPANIONS`, `skill_body`, `all_skill_bodies`,
`companion_body` — und separat `overlay_body` (Anker unten; Stufe 1 lebt jetzt in
`read_skill_file`).

**Pfad-Ableitung (kein Tabellen-Spaltenpaar nötig):** `<skill>/body.lmd.md` bzw.
`<skill>/companions/<companion>.lmd.md`. Companion-Namen tragen ihren Unterpfad bereits im
Namen (`testing/methodology` → `lmd-writing-skills/companions/testing/methodology.lmd.md`) —
die Ableitung ist damit für alle 17 uniform.

**Diese Task bricht das Test-Modul und muss es mitziehen.** ~25 Tests in `src/skills.rs`
rufen die gelöschten Funktionen **beim Namen** auf. Die Signaturen ändern sich zweifach:
`Option` → `Result`, und ein `jail_root`-Parameter kommt hinzu. Ohne die Migration
kompiliert das Test-Binary nicht und `cargo nextest run` (Verify-Schritt 3) fällt schon
beim Bauen um. Die Rewrites sind mechanisch:

| bisher | neu |
|---|---|
| `skill_body("x").is_some()` | `skill_source("x", &jail).is_ok()` |
| `skill_body("nope").is_none()` | `skill_source("nope", &jail).is_err()` |
| `skill_body(name).unwrap()` | `skill_source(name, &jail).unwrap()` |
| `companion_body(s, c).is_some()` | `companion_source(s, c, &jail).is_ok()` |
| `all_skill_bodies()` | `all_skill_sources(&jail).unwrap()` |

`jail_root` ist in den Tests das Repo-Root: `PathBuf::from(env!("CARGO_MANIFEST_DIR"))`.
Kein Overlay, kein `LEAN_MD_SKILLS_DIR` ⇒ der Debug-Fallback liefert exakt die Bytes, die
vorher `include_str!` lieferte. Zwei Tests brauchen mehr als Suchen-und-Ersetzen:

- `all_skill_bodies_aggregate_contains_test_cmd_decl` (`:793-803`) — `let bodies = all_skill_bodies();`
  wird `let bodies = all_skill_sources(&jail).unwrap();`. `bodies` ist jetzt `Vec<String>`;
  `flat_map(|b| scan_var_decls(b))` braucht `scan_var_decls(b.as_str())`.
- `no_dangling_companion_refs_in_seeds` (`:1017-1050`) — der Korpus wechselt Typ und Lifetime.
  Ersetze (Edit-Anker, bestehender Content, `:1030-1039`):

        let mut corpus: Vec<&'static str> = all_skill_bodies();
        corpus.extend(COMPANIONS.iter().map(|(_, _, body)| *body));

        for body in corpus {
            // Skill-scoped render calls: companion must resolve under that skill.
            for cap in call_re.captures_iter(body) {
                let skill = &cap[1];
                let companion = &cap[2];
                assert!(
                    companion_body(skill, companion).is_some(),

  durch (NEUER Content, verbatim — `&body` statt `body`, weil `captures_iter` ein `&str` will):

        let jail = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut corpus: Vec<String> = all_skill_sources(&jail).unwrap();
        for (skill, companion) in COMPANIONS {
            corpus.push(companion_source(skill, companion, &jail).unwrap());
        }

        for body in &corpus {
            // Skill-scoped render calls: companion must resolve under that skill.
            for cap in call_re.captures_iter(body) {
                let skill = &cap[1];
                let companion = &cap[2];
                assert!(
                    companion_source(skill, companion, &jail).is_ok(),

@call tdd("skill_source resolves every one of the 8 registered skills")

@call tdd("companion_source resolves every one of the 17 registered companions")

@call tdd("skill_source of an unregistered name is UnknownSkill, never a disk read")

@call tdd("render_skill and render_companion produce byte-identical output to the pre-cut binary")

Die bestehenden Render-Tests (`brainstorm_all_phases_render_nonempty`,
`writing_plans_all_phases_render_nonempty`, …) behalten ihre **Assertions** — identische
Bytes ⇒ identische Renders —, aber jede `skill_body`/`companion_body`-Zeile darin fällt unter
die Tabelle oben. Die drei `*_matches_seed_file_on_disk`-Tests (`:680`, `:729`, `:1003`)
verlieren ihren Sinn (built-in == on-disk ist tautologisch, wenn die Quelle die Datei ist) —
ersetze sie durch je einen Kaskaden-Test (Overlay gewinnt vor Debug-Fallback).

**Löschanker für `overlay_body`** (`src/skills.rs:212-226`) — es liegt außerhalb des
`12-193`-Blocks und wird nach dem `render_skill`-Edit unbenutzt; `-D warnings` bricht sonst
an `dead_code`. Lösche exakt (bestehender Content, ersatzlos):

    /// D7 body-override: a jailed project overlay at
    /// `<jail_root>/.lean-ctx/lean-md/skills/<name>/body.lmd.md` wins over the
    /// embedded const, enabling local phase iteration without a recompile.
    /// PathJail-bound (no escape outside `jail_root`).
    fn overlay_body(name: &str, jail_root: &std::path::Path) -> Option<String> {
        let candidate = jail_root
            .join(".lean-ctx/lean-md/skills")
            .join(name)
            .join("body.lmd.md");
        let resolved = crate::pathx::jail_path(&candidate, jail_root).ok()?;
        if !resolved.exists() {
            return None;
        }
        std::fs::read_to_string(&resolved).ok()
    }

NEUER Content, verbatim — ersetzt `src/skills.rs:12-193` (die 25 Konstanten, beide Tabellen
und die drei Lookup-Funktionen `skill_body` / `all_skill_bodies` / `companion_body`):

    use std::path::Path;

    /// Registry of lmd skill names. The body path is derived, not tabled:
    /// `<name>/body.lmd.md`, relative to the skill-content root.
    pub const SKILLS: &[&str] = &[
        "lmd-brainstorm",
        "lmd-test-driven-development",
        "lmd-writing-skills",
        "lmd-writing-plans",
        "lmd-subagent-driven-development",
        "lmd-executing-plans",
        "lmd-finishing-a-development-branch",
        "lmd-dispatching-parallel-agents",
    ];

    /// Registry of `(skill, companion)` pairs. The path is derived:
    /// `<skill>/companions/<companion>.lmd.md` — a companion name may carry a
    /// subdirectory (`testing/methodology`), which keeps the rule uniform.
    pub const COMPANIONS: &[(&str, &str)] = &[
        ("lmd-test-driven-development", "testing-anti-patterns"),
        ("lmd-writing-skills", "skill-anatomy"),
        ("lmd-writing-skills", "skill-discovery-optimization"),
        ("lmd-writing-skills", "bulletproofing"),
        ("lmd-writing-skills", "testing/methodology"),
        ("lmd-writing-skills", "testing/skill-types"),
        ("lmd-writing-skills", "testing/creation-checklist"),
        ("lmd-writing-skills", "claude-md-testing-example"),
        ("lmd-writing-skills", "flowchart-conventions"),
        ("lmd-writing-skills", "anthropic-best-practices"),
        ("lmd-writing-skills", "persuasion-principles"),
        ("lmd-brainstorm", "spec-reviewer"),
        ("lmd-brainstorm", "visual-companion"),
        ("lmd-writing-plans", "plan-reviewer"),
        ("lmd-subagent-driven-development", "implementer"),
        ("lmd-subagent-driven-development", "task-reviewer"),
        ("lmd-subagent-driven-development", "code-reviewer"),
    ];

    /// Body source of a known lmd skill, resolved through the content cascade.
    pub fn skill_source(name: &str, jail_root: &Path) -> Result<String, SkillRenderError> {
        if !SKILLS.contains(&name) {
            return Err(SkillRenderError::UnknownSkill(name.to_string()));
        }
        crate::skill_source::read_skill_file(&format!("{name}/body.lmd.md"), jail_root)
            .map_err(SkillRenderError::Source)
    }

    /// All skill bodies (for cross-skill `@var` aggregation in `vars --init`).
    pub fn all_skill_sources(jail_root: &Path) -> Result<Vec<String>, SkillRenderError> {
        SKILLS.iter().map(|n| skill_source(n, jail_root)).collect()
    }

    /// Source of a known `(skill, companion)` pair, resolved through the cascade.
    pub fn companion_source(
        skill: &str,
        companion: &str,
        jail_root: &Path,
    ) -> Result<String, SkillRenderError> {
        if !COMPANIONS.iter().any(|(s, c)| *s == skill && *c == companion) {
            return Err(SkillRenderError::CompanionNotFound(format!("{skill}/{companion}")));
        }
        let rel = format!("{skill}/companions/{companion}.lmd.md");
        crate::skill_source::read_skill_file(&rel, jail_root).map_err(SkillRenderError::Source)
    }

`SkillRenderError` bekommt eine Variante (Edit-Anker, bestehender Content, `:196-210`):

    #[derive(Debug)]
    pub enum SkillRenderError {
        UnknownSkill(String),
        PhaseNotFound(String),
        CompanionNotFound(String),
    }

wird zu (NEUER Content, verbatim):

    #[derive(Debug)]
    pub enum SkillRenderError {
        UnknownSkill(String),
        PhaseNotFound(String),
        CompanionNotFound(String),
        Source(crate::skill_source::SourceError),
    }

und im `Display`-`match` kommt der Arm hinzu (NEUER Content, verbatim):

            SkillRenderError::Source(e) => write!(f, "{e}"),

`render_skill` verliert seinen Overlay-Vorspann (Edit-Anker, bestehender Content, `:238-242`):

        let owned_overlay = overlay_body(name, &jail_root);
        let src: &str = match owned_overlay.as_deref() {
            Some(s) => s,
            None => skill_body(name).ok_or_else(|| SkillRenderError::UnknownSkill(name.to_string()))?,
        };
        let (mut header, body) = parse_header(src);

wird zu (NEUER Content, verbatim — die Kaskade macht das Overlay jetzt selbst):

        let owned = skill_source(name, &jail_root)?;
        let (mut header, body) = parse_header(&owned);

`render_companion` (Edit-Anker, bestehender Content, `:350-354`):

        let src = companion_body(skill, companion)
            .ok_or_else(|| SkillRenderError::CompanionNotFound(format!("{skill}/{companion}")))?;
        Ok(render_full_source(src, consumer, crp, jail_root))

wird zu (NEUER Content, verbatim — `jail_root` wird zweimal gebraucht, deshalb geklont):

        let src = companion_source(skill, companion, &jail_root)?;
        Ok(render_full_source(&src, consumer, crp, jail_root))

**Caller-Ripple 1** — `src/bridges/dispatch.rs:57` (Edit-Anker, bestehender Content):

                let Some(body) = crate::skills::companion_body(skill, c) else {
                    return Ok(format!("<!-- lmd: COMPANION_NOT_FOUND '{skill}/{c}' -->\n"));
                };
                std::borrow::Cow::Borrowed(body)

wird zu (NEUER Content, verbatim):

                let Ok(body) = crate::skills::companion_source(skill, c, &ctx.jail_root) else {
                    return Ok(format!("<!-- lmd: COMPANION_NOT_FOUND '{skill}/{c}' -->\n"));
                };
                std::borrow::Cow::Owned(body)

**Caller-Ripple 2** — `src/bin/lean_md.rs:155-162` (Edit-Anker, bestehender Content):

            Some(skill) => match lean_md::skills::skill_body(skill) {
                Some(body) => body.to_string(),
                None => {
                    eprintln!("lean-md render: unknown skill '{skill}'");
                    std::process::exit(1);
                }
            },

wird zu (NEUER Content, verbatim):

            Some(skill) => {
                let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                match lean_md::skills::skill_source(skill, &root) {
                    Ok(body) => body,
                    Err(e) => {
                        eprintln!("lean-md render: {e}");
                        std::process::exit(1);
                    }
                }
            }

**Caller-Ripple 3** — `src/bin/lean_md.rs:396-408` (Edit-Anker, bestehender Content):

        let decls: Vec<_> = match name {
            Some(n) => match skill_body(n) {
                Some(body) => scan_var_decls(body),
                None => {
                    eprintln!("lean-md skill vars --init: unknown skill '{n}'");
                    std::process::exit(1);
                }
            },
            None => all_skill_bodies()
                .iter()
                .flat_map(|b| scan_var_decls(b))
                .collect(),
        };
        let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

wird zu (NEUER Content, verbatim — `project_root` wandert vor das `match`, weil beide Zweige
ihn brauchen):

        let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let sources: Vec<String> = match name {
            Some(n) => match skill_source(n, &project_root) {
                Ok(body) => vec![body],
                Err(e) => {
                    eprintln!("lean-md skill vars --init: {e}");
                    std::process::exit(1);
                }
            },
            None => match all_skill_sources(&project_root) {
                Ok(bodies) => bodies,
                Err(e) => {
                    eprintln!("lean-md skill vars --init: {e}");
                    std::process::exit(1);
                }
            },
        };
        let decls: Vec<_> = sources.iter().flat_map(|b| scan_var_decls(b)).collect();

Der `use`-Kopf `src/bin/lean_md.rs:16` (Edit-Anker, bestehender Content):

    use lean_md::skills::{all_skill_bodies, render_companion, render_skill, skill_body};

wird zu (NEUER Content, verbatim):

    use lean_md::skills::{all_skill_sources, render_companion, render_skill, skill_source};

### Verify & Close

@call verify("src/skills.rs src/bin/lean_md.rs src/bridges/dispatch.rs")

**Expected — kein `include_str!` auf `content/skills` mehr in `skills.rs`:**

    python3 -c "s=open('src/skills.rs').read(); assert 'content/skills' not in s, 'skill include_str! remnant'; assert 'include_str!' not in s; print('OK skills.rs is source-agnostic')"

→ `OK skills.rs is source-agnostic`

**Expected — Render über den Debug-Fallback (kein Pack, kein Overlay):**

    cargo run -q --bin lean-md -- render --skill lmd-writing-plans --phase plan-format --consumer=ai

→ Ausgabe beginnt mit `## Plan Format — write the plan as a `.lmd.md` document`

@call gate("src/skills.rs src/bin/lean_md.rs src/bridges/dispatch.rs")

@call commit("src/skills.rs src/bin/lean_md.rs src/bridges/dispatch.rs", "feat(skills)!: resolve bodies + companions from the pack, drop include_str! (#727)")
@phase-end

@phase "task-4"
## Task 4: `src/fragments.rs` — die 3 skill-lokalen `_includes/` cutten

**Files:** Edit `src/fragments.rs`.

**Consumes:** `crate::skill_source::read_skill_file` (Task 2).
**Produces:** eine dreistufige `resolve()`: cross-skill-Builtin → Pack-Store (für die 3
skill-lokalen Namen) → jailed Datei-Fallback.

**Kanal 3 splittet.** Aus `fragments.rs` fliegen **nur** die drei skill-scoped
`include_str!`-Builtins (`TEST_FIRST_CORE`, `SKILL_AUTHORING_CORE`, `BRAINSTORM_GATE`,
`:22-36`). `HARD_RULES`, `DISPATCH_CONTRACT`, `PARALLEL_DISPATCH` bleiben builtin — sie sind
allgemeine lmd-Primitive, die jedes `.lmd.md` nutzt (ein User-Plan via `@dispatch` /
`@include hard-rules` rendert nie eine Skill).

**Die Datei-Fallback-Stufe MUSS erhalten bleiben** — sie ist die user-erweiterbare
`<jail_root>/<name>.lmd.md`-Schicht und hat mit dem Cut nichts zu tun.

@call tdd("test-first-core resolves through the pack stage, not through builtins")

@call tdd("hard-rules, dispatch-contract and parallel-dispatch stay builtin-resolved")

@call tdd("an unknown fragment still falls back to a jailed <name>.lmd.md file")

Der bestehende `brainstorm_gate_matches_seed_file_on_disk` (`:194`) wird zum
Pack-Resolutions-Test; `parallel_dispatch_matches_seed_file_on_disk` (`:211`) bleibt
**unverändert** — er ist der #498-Fragment-Consistency-Gate für die verbleibenden Builtins.

Lösche `src/fragments.rs:19-36` (die drei skill-lokalen Konstanten samt Doc-Kommentaren) und
ersetze `with_builtins` + `resolve` (Edit-Anker, bestehender Content, `:57-81`) durch (NEUER
Content, verbatim):

    /// Fragment names that live inside a skill's `_includes/`, mapped to their owning
    /// skill. They travel with that skill in the `kind=skills` pack (#727) — flat
    /// global name, skill-scoped storage.
    const SKILL_INCLUDES: &[(&str, &str)] = &[
        ("test-first-core", "lmd-test-driven-development"),
        ("skill-authoring-core", "lmd-writing-skills"),
        ("brainstorm-gate", "lmd-brainstorm"),
    ];

    impl FragmentRegistry {
        pub fn with_builtins() -> Self {
            let mut builtins = HashMap::new();
            builtins.insert("hard-rules", HARD_RULES);
            builtins.insert("dispatch-contract", DISPATCH_CONTRACT);
            builtins.insert("parallel-dispatch", PARALLEL_DISPATCH);
            Self { builtins }
        }

        /// Three stages: cross-skill builtin → skill-local `_includes/` in the pack →
        /// jailed `<name>.lmd.md` file. The file stage stays user-extensible.
        pub fn resolve(&self, name: &str, jail_root: &Path) -> Result<String, ResolveError> {
            if let Some(content) = self.builtins.get(name) {
                return Ok((*content).to_string());
            }
            if let Some((_, skill)) = SKILL_INCLUDES.iter().find(|(n, _)| *n == name) {
                let rel = format!("{skill}/_includes/{name}.lmd.md");
                return crate::skill_source::read_skill_file(&rel, jail_root)
                    .map_err(|e| ResolveError::Io(e.to_string()));
            }
            let candidate = jail_root.join(format!("{name}.lmd.md"));
            let resolved = crate::pathx::jail_path(&candidate, jail_root)
                .map_err(|_| ResolveError::Jail(format!("{name} escapes jail")))?;
            if !resolved.exists() {
                return Err(ResolveError::NotFound(name.to_string()));
            }
            std::fs::read_to_string(&resolved).map_err(|e| ResolveError::Io(e.to_string()))
        }
    }

### Verify & Close

@call verify("src/fragments.rs")

**Expected — genau 3 `include_str!` übrig, alle unter `content/core`:**

    python3 -c "import re; s=open('src/fragments.rs').read(); inc=re.findall(r'include_str!\(\"([^\"]+)\"\)', s); assert len(inc)==3, inc; assert all('content/core' in p for p in inc), inc; assert 'content/skills' not in s; print('OK 3 core builtins remain')"

→ `OK 3 core builtins remain`

@call gate("src/fragments.rs")

@call commit("src/fragments.rs", "feat(fragments)!: skill-local _includes resolve from the pack; core stays builtin (#727)")
@phase-end

@phase "task-5"
## Task 5: `src/skill_install.rs` — `SKILL.md`-Stubs + Assets aus dem Pack

**Files:** Edit `src/skill_install.rs`.

**Consumes:** `crate::skill_source::read_skill_file` (Task 2).
**Produces:** `INSTALLABLE_SKILLS` / `ASSETS` als Namenslisten; `install_skill` liest Stub
und Assets zur Laufzeit aus der Kaskade.

**`install_skill` bleibt bestehen und schreibt weiterhin `.claude/skills/<name>/`** — der
Bridge-Schritt ist das Discovery-Interface des Agenten, nicht der Content-Kanal. Nur die
Quelle der Bytes wechselt.

**`materialize_contracts` bleibt am Ende unverändert stehen** — `seeds.rs` / `PROJECT_SEEDS`
ist ein eigener Kanal (Global Constraints).

@call tdd("install_skill writes SKILL.md read from the content cascade")

@call tdd("brainstorm install materializes all 5 scripts with the .sh executable bit")

@call tdd("install_skill of an unknown name errors before touching the filesystem")

@call tdd("install_skill surfaces PackMissing as an io::Error instead of writing an empty stub")

**Ein bestehender Test bricht am Typwechsel und muss mit.** `brainstorm_assets_reference_closure`
(`:286-297`) destrukturiert `ASSETS` dreistellig und liest die `content`-Spalte, die diese Task
entfernt — ohne Migration kompiliert das Test-Modul nicht. Ersetze seinen Rumpf (Edit-Anker,
bestehender Content, `:289-296`):

        for (skill, fname, content) in ASSETS {
            if *skill == "lmd-brainstorm" {
                assert!(
                    !content.to_lowercase().contains("superpowers"),
                    "asset {fname} still references superpowers"
                );
            }
        }

durch (NEUER Content, verbatim — die Bytes kommen jetzt aus der Kaskade statt aus der Tabelle):

        let jail = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for (skill, fname) in ASSETS {
            if *skill == "lmd-brainstorm" {
                let content =
                    crate::skill_source::read_skill_file(&format!("{skill}/{fname}"), &jail)
                        .expect("brainstorm asset resolves");
                assert!(
                    !content.to_lowercase().contains("superpowers"),
                    "asset {fname} still references superpowers"
                );
            }
        }

Ersetze `src/skill_install.rs:9-81` (die 14 `include_str!`-Konstanten und beide Tabellen)
durch (NEUER Content, verbatim):

    /// Installable lmd skills. The stub path is derived: `<name>/SKILL.md`.
    pub const INSTALLABLE_SKILLS: &[&str] = &[
        "lmd-test-driven-development",
        "lmd-brainstorm",
        "lmd-writing-skills",
        "lmd-writing-plans",
        "lmd-subagent-driven-development",
        "lmd-executing-plans",
        "lmd-finishing-a-development-branch",
        "lmd-dispatching-parallel-agents",
    ];

    /// Non-rendered helper files materialized verbatim into the installed skill dir
    /// (skill, pack-relative filename). Read from the content cascade at install time.
    const ASSETS: &[(&str, &str)] = &[
        ("lmd-writing-skills", "render-graphs.js"),
        ("lmd-brainstorm", "scripts/server.cjs"),
        ("lmd-brainstorm", "scripts/helper.js"),
        ("lmd-brainstorm", "scripts/frame-template.html"),
        ("lmd-brainstorm", "scripts/start-server.sh"),
        ("lmd-brainstorm", "scripts/stop-server.sh"),
    ];

Ersetze `skill_md` (`:103-108`) durch (NEUER Content, verbatim):

    /// The `SKILL.md` stub of an installable skill, read through the content cascade.
    /// `project_root` doubles as the jail root, so a project overlay wins here too.
    fn skill_md(name: &str, project_root: &Path) -> std::io::Result<String> {
        if !INSTALLABLE_SKILLS.contains(&name) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("unknown installable skill: {name}"),
            ));
        }
        crate::skill_source::read_skill_file(&format!("{name}/SKILL.md"), project_root)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }

Ersetze in `install_skill` den Kopf **und die ganze Asset-Schleife** (Edit-Anker,
bestehender Content, `:130-159` — von `let body =` bis zur schließenden Klammer der
`for`-Schleife):

        let body = skill_md(name).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("unknown installable skill: {name}"),
            )
        })?;
        let dir = target_dir(name, scope, project_root);
        std::fs::create_dir_all(&dir)?;
        let target = dir.join("SKILL.md");
        std::fs::write(&target, body)?;
        let mut created_parents: std::collections::HashSet<std::path::PathBuf> =
            std::collections::HashSet::new();
        for (skill, fname, content) in ASSETS {
            if *skill == name {
                let asset_path = dir.join(fname);
                if let Some(parent) = asset_path.parent()
                    && created_parents.insert(parent.to_path_buf())
                {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&asset_path, content)?;
                #[cfg(unix)]
                if fname.ends_with(".sh") {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perm = std::fs::metadata(&asset_path)?.permissions();
                    perm.set_mode(0o755);
                    std::fs::set_permissions(&asset_path, perm)?;
                }
            }
        }

durch (NEUER Content, verbatim — jede Quelle wird gelesen, BEVOR irgendetwas geschrieben
wird, damit ein fehlender Pack keinen halbinstallierten Skill-Ordner hinterlässt; die
Schleife verliert ihr `if *skill == name`, weil `filter` das bereits erledigt):

        let body = skill_md(name, project_root)?;
        let assets: Vec<(&str, String)> = ASSETS
            .iter()
            .filter(|(skill, _)| *skill == name)
            .map(|(_, fname)| {
                crate::skill_source::read_skill_file(&format!("{name}/{fname}"), project_root)
                    .map(|content| (*fname, content))
                    .map_err(|e| std::io::Error::other(e.to_string()))
            })
            .collect::<std::io::Result<_>>()?;
        let dir = target_dir(name, scope, project_root);
        std::fs::create_dir_all(&dir)?;
        let target = dir.join("SKILL.md");
        std::fs::write(&target, body)?;
        let mut created_parents: std::collections::HashSet<std::path::PathBuf> =
            std::collections::HashSet::new();
        for (fname, content) in &assets {
            let asset_path = dir.join(fname);
            if let Some(parent) = asset_path.parent()
                && created_parents.insert(parent.to_path_buf())
            {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&asset_path, content)?;
            #[cfg(unix)]
            if fname.ends_with(".sh") {
                use std::os::unix::fs::PermissionsExt;
                let mut perm = std::fs::metadata(&asset_path)?.permissions();
                perm.set_mode(0o755);
                std::fs::set_permissions(&asset_path, perm)?;
            }
        }

Der abschließende `materialize_contracts`-Aufruf (`:164`) und `Ok(target)` bleiben stehen.

### Verify & Close

@call verify("src/skill_install.rs")

**Expected — kein Content mehr im Binary, `.claude/skills/` wird weiterhin geschrieben:**

    python3 -c "s=open('src/skill_install.rs').read(); assert 'include_str!' not in s, 'include_str! remnant'; assert '.claude/skills' in s, 'bridge step lost'; print('OK stubs+assets read from pack')"

→ `OK stubs+assets read from pack`

@call gate("src/skill_install.rs")

@call commit("src/skill_install.rs", "feat(install)!: SKILL.md stubs + assets read from the pack (#727)")
@phase-end

@phase "task-6"
## Task 6: Drift-Gate (Rust-Test + CI-Cross-Check) + Dev-Workflow

**Files:** Create `content/skills.sha256`, `tests/pack_drift.rs`,
`.github/workflows/pack-drift.yml`, `docs/dev-readme.md`. Edit `Cargo.toml` (dev-dep `sha2`).

**Warum zwei Gates.** Der lean-ctx-`content_hash` ist `sha256(content_json)` über
zstd-komprimierte, base64-kodierte `DocumentBlob`s (`context_package/skills.rs:104-106`).
lean-md darf ihn nicht nachrechnen (Reverse-Cut: keine `lean_ctx`-Dependency). Also:

- **(a) lokal, `cargo nextest`:** ein eigener, unabhängiger sha256 über sortierte
  Relativpfade + Dateibytes, checked-in als `content/skills.sha256`. Fängt „Content
  geändert, Bump/Publish vergessen" bei jedem Testlauf.
- **(b) CI, echter lean-ctx:** baut den Pack mit dem realen Binary und vergleicht den
  `content_hash` aus `<pkg_dir>/manifest.json` gegen `content/skills.ctxpkg-hash`. Fängt eine
  Divergenz zwischen unserer Hash-Definition und der von lean-ctx.

**`content/skills.sha256` liegt neben `content/skills/`, nicht darin** — sonst wäre die Datei
selbst Pack-Inhalt (`collect_files` nimmt alles unter `--from`) und der Hash invalidierte sich
rekursiv. Gleiches gilt für `content/skills.ctxpkg-hash`.

@call tdd("the on-disk content of content/skills hashes to the checked-in manifest")

@call tdd("a manifest line for a file that no longer exists fails the gate")

`Cargo.toml` — Edit-Anker, bestehender Content:

    [dev-dependencies]
    tiktoken-rs = "0.12"

wird zu (NEUER Content, verbatim — `sha2` ist test-only, kein Produktions-Dep):

    [dev-dependencies]
    tiktoken-rs = "0.12"
    sha2 = "0.10"

NEUER Content, verbatim (`tests/pack_drift.rs`):

    //! Drift gate (#727/#498): `content/skills` must hash to the checked-in manifest.
    //!
    //! Published pack versions are immutable — the lockfile pins `artifact_sha256`.
    //! Any content change therefore forces a PACK version bump plus a republish, never
    //! a binary bump. This gate is what makes "content changed, bump forgotten" loud.
    //!
    //! Bless a legitimate change with:
    //!     LEAN_MD_BLESS=1 cargo nextest run --test pack_drift
    //! then bump the pack version and republish (see docs/dev-readme.md).
    //!
    //! The hash is lean-md's own definition, independent of lean-ctx's `content_hash`
    //! (which compresses before hashing). The CI job `pack-drift.yml` cross-checks the
    //! two against the real lean-ctx binary.

    use std::path::{Path, PathBuf};

    use sha2::{Digest, Sha256};

    fn skills_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("content/skills")
    }

    fn manifest_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("content/skills.sha256")
    }

    /// Relative `/`-separated paths of every regular file under `root`, sorted by byte
    /// order — the same collection rule lean-ctx's `collect_files` applies (dotfiles,
    /// `node_modules`, `target` and symlinks skipped).
    fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("read dir") {
            let entry = entry.expect("dir entry");
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }
            let ft = entry.file_type().expect("file type");
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                collect(root, &entry.path(), out);
            } else if ft.is_file() {
                let rel = entry
                    .path()
                    .strip_prefix(root)
                    .expect("under root")
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push(rel);
            }
        }
    }

    fn render_manifest() -> String {
        let root = skills_root();
        let mut rels = Vec::new();
        collect(&root, &root, &mut rels);
        rels.sort();
        assert!(!rels.is_empty(), "content/skills is empty");
        let mut out = String::from(
            "# lean-md skills-pack content manifest (#498, #727)\n\
             # Regenerate: LEAN_MD_BLESS=1 cargo nextest run --test pack_drift\n\
             # A changed hash means: bump the PACK version and republish it.\n\
             # The binary version is untouched — the two SemVer lines are independent.\n",
        );
        for rel in &rels {
            let bytes = std::fs::read(root.join(rel)).expect("read file");
            let mut h = Sha256::new();
            h.update(&bytes);
            out.push_str(&format!("{:x}  {rel}\n", h.finalize()));
        }
        out
    }

    #[test]
    fn skills_content_matches_the_checked_in_manifest() {
        let rendered = render_manifest();
        let path = manifest_path();
        if std::env::var("LEAN_MD_BLESS").is_ok() {
            std::fs::write(&path, &rendered).expect("write manifest");
            return;
        }
        let checked_in = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(
            checked_in,
            rendered,
            "content/skills drifted from content/skills.sha256.\n\
             Bless with: LEAN_MD_BLESS=1 cargo nextest run --test pack_drift\n\
             then bump the pack version and republish (docs/dev-readme.md)."
        );
    }

    #[test]
    fn every_manifest_entry_names_a_file_that_exists() {
        let root = skills_root();
        let manifest = std::fs::read_to_string(manifest_path()).expect("manifest exists");
        let mut seen = 0usize;
        for line in manifest.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let (_, rel) = line.split_once("  ").expect("`<sha256>  <relpath>`");
            assert!(root.join(rel).is_file(), "manifest names a missing file: {rel}");
            seen += 1;
        }
        assert!(seen >= 30, "suspiciously few entries: {seen}");
    }

Erzeuge das Manifest einmalig (der Test schreibt es selbst):

    LEAN_MD_BLESS=1 cargo nextest run --test pack_drift

**Expected:** `content/skills.sha256` existiert, beginnt mit `# lean-md skills-pack content manifest`.

NEUER Content, verbatim (`.github/workflows/pack-drift.yml`) — **eigener Workflow, nicht
`release.yml`**: dessen `on: push: tags: ['v[0-9]*']` würde den Gate genau dann nicht feuern,
wenn eine reine Skill-Änderung gepusht wird, und ein Pack-Job am `v*`-Tag brächte den
verworfenen Binary/Pack-Lockstep durch die Hintertür zurück.

    name: Pack Drift

    on:
      push:
        paths:
          - 'content/skills/**'
          - 'content/skills.sha256'
          - 'content/skills.ctxpkg-hash'
          - 'tests/pack_drift.rs'
      pull_request:
        paths:
          - 'content/skills/**'
          - 'content/skills.sha256'
          - 'content/skills.ctxpkg-hash'
          - 'tests/pack_drift.rs'

    permissions:
      contents: read

    jobs:
      manifest:
        name: Content manifest is in sync
        runs-on: ubuntu-latest
        steps:
          - uses: actions/checkout@v4 # v4
            with:
              persist-credentials: false
          - uses: dtolnay/rust-toolchain@29eef336d9b2848a0b548edc03f92a220660cdb8 # stable
            with:
              toolchain: stable
          - name: Neutralize dev-only cargo config
            run: rm -f .cargo/config.toml rust-toolchain.toml
          - name: Drift gate
            run: cargo test --test pack_drift

      ctxpkg-hash:
        name: lean-ctx content_hash cross-check
        runs-on: ubuntu-latest
        steps:
          - uses: actions/checkout@v4 # v4
            with:
              persist-credentials: false
          - name: Fetch the lean-ctx binary (min_lean_ctx from the addon manifest)
            run: |
              VER=$(python3 -c "import tomllib;print(tomllib.load(open('lean-ctx-addon.toml','rb'))['addon']['min_lean_ctx'])")
              curl -fsSL "https://github.com/yvgude/lean-ctx/releases/download/v${VER}/lean-ctx-x86_64-unknown-linux-gnu.tar.gz" | tar xz
              chmod +x lean-ctx
          - name: Build the pack and compare its content_hash
            run: |
              ./lean-ctx pack create --kind skills \
                --name @dasTholo/lean-md-skills \
                --version "0.0.0-cihash" \
                --from content/skills \
                --description "lean-md skills (CI drift check)" | tee create.log
              export PKG_DIR=$(sed -n 's/^ *Location: *//p' create.log)
              test -n "$PKG_DIR" || { echo "pack create printed no Location"; exit 1; }
              python3 - <<'PY'
              import json, os, sys
              pkg = os.environ["PKG_DIR"]
              got = json.load(open(f"{pkg}/manifest.json"))["integrity"]["content_hash"]
              want = open("content/skills.ctxpkg-hash").read().strip()
              if got != want:
                  sys.exit(f"lean-ctx content_hash drift: pack={got} checked-in={want}\n"
                           f"Update content/skills.ctxpkg-hash, bump the pack version, republish.")
              print("OK content_hash matches", got)
              PY

> **`--version 0.0.0-cihash`** ist Absicht: der `content_hash` hängt **nur** am Content
> (`sha256(content_json)`), nicht an Name/Version — nur der äußere `sha256` chained
> `name:version:content_hash`. Der CI-Job baut also nie eine echte Version und kann keinen
> Publish auslösen. **Kein Publish-Token liegt in dieser Workflow-Umgebung.**

Erzeuge `content/skills.ctxpkg-hash` **einmalig lokal** (Maintainer, mit installiertem
lean-ctx ≥ 3.9.4) und committe die 64 Hex-Zeichen + Newline:

    lean-ctx pack create --kind skills --name @dasTholo/lean-md-skills --version 0.0.0-cihash --from content/skills --description "lean-md skills (CI drift check)"

**Expected:** die Ausgabezeile `  Location: <pkg_dir>` — daraus
`python3 -c "import json;print(json.load(open('<pkg_dir>/manifest.json'))['integrity']['content_hash'])"`
> `content/skills.ctxpkg-hash`.

NEUER Content, verbatim (`docs/dev-readme.md`) — **falls die Datei bereits existiert
(untracked), hänge den Abschnitt an, statt sie zu überschreiben**:

    ## Zwei Release-Regime (seit P3, #727)

    | Änderung                                     | Kanal  | Ablauf                                                        |
    |----------------------------------------------|--------|---------------------------------------------------------------|
    | `content/skills/**`                          | Pack   | Bump + `pack create` + `pack publish`. Kein Tag, kein Binary. |
    | `content/core/**`, `content/gloss/**`, `src/**` | Binary | Tag `v*` → 5-leg-Build → `sync-manifest` schreibt die SHA-Pins. |

    Pack und Binary tragen **unabhängige** SemVer-Linien (initial beide `0.2.0`). Publizierte
    Pack-Versionen sind immutable (das Lockfile pinnt `artifact_sha256`), also erzwingt jede
    Content-Änderung einen **Pack**-Bump — nie einen Binary-Bump. `version_req = "^0.2"` deckt
    `0.2.x` ab; erst ein Sprung auf `0.3.x` verlangt einen Manifest-Bump + Addon-Republish.

    ### Skill-Content ändern

    1. `content/skills/**` editieren.
    2. `LEAN_MD_BLESS=1 cargo nextest run --test pack_drift` — schreibt `content/skills.sha256`.
    3. `lean-ctx pack create --kind skills --name @dasTholo/lean-md-skills --version <neu> --from content/skills --description "lmd skills"`
    4. `content/skills.ctxpkg-hash` aus `<pkg_dir>/manifest.json` (`integrity.content_hash`) aktualisieren.
    5. `lean-ctx pack export @dasTholo/lean-md-skills@<neu> --sign --output pack.ctxpkg`
    6. `lean-ctx pack publish pack.ctxpkg --token ctxp_…` — **von Hand**. CI verifiziert nur;
       es liegt bewusst kein Publish-Token in der Workflow-Umgebung.

    ### Lokal ohne Pack entwickeln

    `cargo run -- render --skill X --phase Y` greift auf den Debug-Fallback
    (`$CARGO_MANIFEST_DIR/content/skills`). Im Release-Binary ist er inert
    (`cfg(debug_assertions)`), dort ist ein fehlendes `LEAN_MD_SKILLS_DIR` ein harter Fehler.

### Verify & Close

@call verify("content/skills.sha256 tests/pack_drift.rs .github/workflows/pack-drift.yml docs/dev-readme.md Cargo.toml")

**Expected — Gate ist scharf: eine Content-Änderung bricht ihn, Revert heilt ihn:**

    printf '\n<!-- drift probe -->\n' >> content/skills/lmd-brainstorm/body.lmd.md
    cargo nextest run --test pack_drift

→ `skills_content_matches_the_checked_in_manifest` schlägt fehl mit `content/skills drifted`.

    git checkout -- content/skills/lmd-brainstorm/body.lmd.md
    cargo nextest run --test pack_drift

→ beide Tests grün.

**Expected — Debug-Fallback-Render ohne Pack (DoD „Dev-Workflow"):**

    cargo run -q --bin lean-md -- render --skill lmd-executing-plans --phase pre-context --consumer=ai

→ nicht-leerer Render.

**Expected — DoD „Binary schrumpft messbar": kein Skill-Content mehr eingebacken.**
`content/skills` misst ~236 KB; genau diese Bytes waren `include_str!`-Payload:

    cargo build --release
    python3 -c "import os,sys; b=os.path.getsize('target/release/lean-md'); c=sum(os.path.getsize(os.path.join(r,f)) for r,_,fs in os.walk('content/skills') for f in fs); print(f'binary={b} skills_on_disk={c}'); sys.exit(0)"
    strings target/release/lean-md | grep -c "I'm using the lmd-writing-plans skill" || true

→ die `grep -c`-Zeile gibt `0`: kein Skill-Body-Text liegt mehr im Release-Binary. Notiere
`binary=…` als Referenzwert im Commit-Body (Vergleich gegen den Pre-Cut-Build auf `HEAD~5`).

@call gate("content/skills.sha256 tests/pack_drift.rs .github/workflows/pack-drift.yml docs/dev-readme.md Cargo.toml")

@call commit("content/skills.sha256 content/skills.ctxpkg-hash tests/pack_drift.rs .github/workflows/pack-drift.yml docs/dev-readme.md Cargo.toml", "ci(pack): content drift gate + lean-ctx content_hash cross-check (#727)")
@phase-end

@phase "task-7"
## Task 7: End-to-End Live-Smoke (Maintainer-Hand, kein Code)

**Files:** keine. Diese Task ist die DoD-live-Checkliste; sie läuft **nach** V1–V4 und
verifiziert den Vertrag gegen echte Registry, echtes Lockfile, echtes Binary.

**Consumes:** ein released lean-ctx ≥ 3.9.4 (V1), lean-md `v0.2.0` mit echten `[artifacts]`-SHA
(V2), curated-Entry auf `listed` (V3), beide Packs hosted publiziert (V4).

**Reihenfolge (jeder Schritt ein eigener Aufruf, kein Chaining):**

    lean-ctx pack publish lean-md-skills-0.2.0.ctxpkg --token ctxp_…

**Expected:** `@dasTholo/lean-md-skills@0.2.0` erscheint im Registry-Index.

    lean-ctx addon publish --namespace dasTholo

**Expected:** der `kind=addon`-Pack trägt das `[[dependencies]]`-Array durch
(`publish.rs` reicht es seit `38699d7ce` weiter). **Prüfen, nicht annehmen** — ein leeres
`dependencies` im publizierten `pack_manifest` bedeutet, dass V1 nicht in der Release-Version
steckt, und Task 7 stoppt hier.

    lean-ctx addon add @dasTholo/lean-md

**Expected, in dieser Reihenfolge:**
1. `preflight` passiert das `min_lean_ctx`-Gate (ein älteres lean-ctx bricht hier ab).
2. Die Consent-Surface listet den Pack: `+ @dasTholo/lean-md-skills@0.2.0`.
3. Der Pack wird SHA-verifiziert und read-only nach
   `<store>/skills/@dasTholo__lean-md-skills/0.2.0/` materialisiert.
4. Das Binary kommt aus dem **GitHub-Release** (`[artifacts]`), nicht aus der Registry.
5. `LEAN_MD_SKILLS_DIR` steht im gewireten `[mcp.env]` und zeigt auf genau dieses Verzeichnis.
6. Das Lockfile pinnt **Binary und Pack** (`artifact_sha256` je Eintrag).

**Smoke — Bodies erreichbar, Scripts materialisieren und laufen:**

    lean-md render --skill lmd-brainstorm --phase pre-context --consumer=ai

**Expected:** nicht-leerer Render (Quelle = Pack-Store, nicht Binary, nicht Debug-Fallback).

    lean-md skill install lmd-brainstorm --local

**Expected:** `.claude/skills/lmd-brainstorm/SKILL.md` plus `scripts/server.cjs`,
`helper.js`, `frame-template.html`, `start-server.sh` (Modus `0755`), `stop-server.sh`.

    bash .claude/skills/lmd-brainstorm/scripts/start-server.sh

**Expected:** Server startet; anschließend `stop-server.sh` beendet ihn sauber.

**Reproduzierbarkeit — zweiter Install offline:**

    lean-ctx addon remove @dasTholo/lean-md

    lean-ctx addon add @dasTholo/lean-md

**Expected:** identische Pack-Version und identischer `artifact_sha256` aus dem Lockfile,
ohne Netzzugriff auf den Registry-Index.

**Negativprobe — Produktion ohne Pack ist hart-fehlerhaft, nicht still:**

    env -u LEAN_MD_SKILLS_DIR lean-md render --skill lmd-brainstorm --phase pre-context

**Expected:** Exit ≠ 0, Meldung `PACK_MISSING LEAN_MD_SKILLS_DIR is unset …
lean-ctx addon add @dasTholo/lean-md`. (Gilt für das **Release**-Binary; ein `cargo run`
greift zuvor auf den Debug-Fallback.)

@call remember_decision("P3 (#727) live: @dasTholo/lean-md-skills@0.2.0 is the sole skill-content source in production; binary and pack carry independent SemVer lines; content changes bump the pack only.")
@phase-end
