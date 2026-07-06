@lean-md
consumer: ai
crp: compact

@var test_cmd default="cargo nextest run" desc="project test runner (never cargo test)"
@var lint_cmd default="cargo clippy --all-targets -- -D warnings" desc="project lint gate"
@import .lean-ctx/lean-md/plan-recipes /

# lmd-finishing-a-development-branch — Implementation Plan

Register the native lmd skill `lmd-finishing-a-development-branch` (full-fidelity port of the
superpowers `finishing-a-development-branch`) and rewire the two existing lmd skills that still
point at the external reference.

## Architecture

- Skills ship via `include_str!` from `content/skills/<name>/` — that is the delivery path, NOT
  `docs/lean-md/plans/assets/`. The authoritative seeds are **already written + render-verified**
  in place:
  - body → `content/skills/lmd-finishing-a-development-branch/body.lmd.md` (renders as 8 phases;
    `pre-context` inlines `@include hard-rules`; `merge-local`/`discard` carry the provenance
    cleanup inline — verified via `lean-md render <body> --list-phases`).
  - stub → `content/skills/lmd-finishing-a-development-branch/SKILL.md` (single-line
    `description`, no `": "` mapping indicator, no `superpowers` token).
- The seed content is lmd source carrying `@phase`/`@include` directives; it therefore lives as
  its own seed file and is anchored (`@read`), never pasted inline into this `.lmd.md` plan
  (inline `@phase` would collide with the plan's own phase isolation).
- Registration = three Rust rows: `src/skills.rs` (body, `include_str!` + `SKILLS`),
  `src/skill_install.rs` (stub, `include_str!` + `INSTALLABLE_SKILLS`), `src/availability.rs`
  (`COVERAGE`). No `fragments.rs` change — the worktree cleanup is duplicated inline in the two
  option phases (phase isolation requires it; a shared builtin would add a 6th registration site
  and a fragment-consistency gate surface for ~8 lines).
- `@call gate/...` and `@query "git …"` inside a rendered skill body are literal executor
  references (inline-code, not line-start directives) — not expanded at render time.

## Global Constraints

- Non-goal: seed content + three registry rows only — NO engine/renderer/bridge/`fragments.rs` change.
- Reference-closure (test gate): the body seed AND the SKILL.md stub must NOT contain the token
  `superpowers` (native-port gate — asserted in Task 1 & 2).
- #498: `include_str!` embeds the on-disk seed byte-identically; existing fragment-consistency
  and byte-stability gates stay green.
- Fidelity: exactly-4 / exactly-3 option menus and provenance cleanup
  (`.worktrees/`/`worktrees/` only, `cd` main-root before `git worktree remove`,
  `git worktree prune` after) must survive — already encoded in the body seed.
- Prerequisites: Task 1 (skills.rs body) lands before Task 3 (COVERAGE references the phases) and
  is independent of Task 4 (rewiring); Task 2 (install) needs the SKILL.md seed (already written).

@phase "task-1"
## Task 1: Register the skill body (skills.rs)

**Files:** Modify `src/skills.rs`. The body seed
`content/skills/lmd-finishing-a-development-branch/body.lmd.md` is already written and
render-verified — anchor it, do not rewrite it: `@read content/skills/lmd-finishing-a-development-branch/body.lmd.md mode=map`.

Anchor the existing executing-plans embed at `skills.rs:61-62` (`const LMD_EXECUTING_PLANS_BODY`)
and the registry row at `skills.rs:76`. Add the const after `LMD_EXECUTING_PLANS_BODY`:

    const LMD_FINISHING_BODY: &str =
        include_str!("../content/skills/lmd-finishing-a-development-branch/body.lmd.md");

Append the registry row inside `SKILLS` (after the `lmd-executing-plans` entry at `skills.rs:76`):

    ("lmd-finishing-a-development-branch", LMD_FINISHING_BODY),

Add tests in the `skills.rs` test module (mirror `executing_plans_all_phases_render_nonempty`
at `skills.rs:486` and `executing_plans_orient_carries_hard_rules_baseline` at `skills.rs:517`):

    #[test]
    fn finishing_all_phases_render_nonempty() {
        let jail = std::path::PathBuf::from(".");
        for p in [
            "pre-context",
            "verify-tests",
            "detect-env",
            "present-options",
            "merge-local",
            "create-pr",
            "keep-as-is",
            "discard",
        ] {
            let out = render_skill("lmd-finishing-a-development-branch", Some(p), None, None, jail.clone())
                .unwrap_or_else(|_| panic!("phase {p} failed to render"));
            assert!(!out.trim().is_empty(), "phase {p} must render non-empty");
        }
        assert!(
            skill_body("lmd-finishing-a-development-branch").is_some(),
            "lmd-finishing-a-development-branch must be in the SKILLS registry"
        );
        assert!(
            !skill_body("lmd-finishing-a-development-branch")
                .unwrap()
                .to_lowercase()
                .contains("superpowers"),
            "body seed must be reference-closed (no superpowers token)"
        );
    }

    #[test]
    fn finishing_pre_context_carries_hard_rules_baseline() {
        let out = render_skill(
            "lmd-finishing-a-development-branch",
            Some("pre-context"),
            None,
            None,
            std::path::PathBuf::from("."),
        )
        .unwrap();
        assert!(
            out.contains("Hard Rules (lmd built-in)"),
            "pre-context must inline the ambient baseline via @include hard-rules: {out}"
        );
    }

    #[test]
    fn finishing_merge_local_carries_provenance_cleanup() {
        let out = render_skill(
            "lmd-finishing-a-development-branch",
            Some("merge-local"),
            None,
            None,
            std::path::PathBuf::from("."),
        )
        .unwrap();
        assert!(
            out.contains("git worktree remove") && out.contains("git worktree prune"),
            "merge-local must carry the provenance cleanup: {out}"
        );
    }

### Verify & Close

@call verify(src/skills.rs)
@call gate(src/skills.rs)
@call commit("src/skills.rs", "feat(lmd-finishing): register skill body (8 phases, option-branched)")
@call remember_decision("lmd-finishing-a-development-branch body registered in SKILLS; 8 phases; option phases terminal; cleanup duplicated inline (no fragments.rs)")
@phase-end

@phase "task-2"
## Task 2: Register the install stub (skill_install.rs)

**Files:** Modify `src/skill_install.rs`. The stub seed
`content/skills/lmd-finishing-a-development-branch/SKILL.md` is already written — anchor it:
`@read content/skills/lmd-finishing-a-development-branch/SKILL.md mode=full`.

Anchor the executing-plans embed at `skill_install.rs:15-16` (`const EXECUTING_PLANS_SKILL_MD`)
and the `INSTALLABLE_SKILLS` row at `skill_install.rs:25`. Add the const after
`EXECUTING_PLANS_SKILL_MD`:

    const FINISHING_SKILL_MD: &str =
        include_str!("../content/skills/lmd-finishing-a-development-branch/SKILL.md");

Append the installable row (after the `lmd-executing-plans` entry at `skill_install.rs:25`):

    ("lmd-finishing-a-development-branch", FINISHING_SKILL_MD),

Add a test mirroring `executing_plans_install_writes_skill_md` at `skill_install.rs:356`
(install to a temp root; assert the stub is written, carries `name:`, is reference-closed, has a
valid same-line `description:` scalar, and is idempotent):

    #[test]
    fn finishing_install_writes_skill_md() {
        let root =
            std::env::temp_dir().join(format!("lmd_finishing_install_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let skill_md =
            install_skill("lmd-finishing-a-development-branch", Scope::Local, &root, false).unwrap();
        assert!(skill_md.exists(), "SKILL.md must be written");
        let written = std::fs::read_to_string(&skill_md).unwrap();
        assert!(
            written.contains("name: lmd-finishing-a-development-branch"),
            "stub frontmatter missing"
        );
        assert!(
            !written.contains("superpowers"),
            "native port must not carry a 'superpowers' reference"
        );
        let desc_line = written
            .lines()
            .find(|l| l.starts_with("description:"))
            .expect("description key missing");
        let value = desc_line["description:".len()..].trim();
        assert!(!value.is_empty(), "description must be a non-empty same-line scalar: {desc_line:?}");
        let quoted = value.starts_with('"') || value.starts_with('\'');
        assert!(
            quoted || !value.contains(": "),
            "unquoted description scalar must not contain ': ': {value}"
        );
        let again =
            install_skill("lmd-finishing-a-development-branch", Scope::Local, &root, false).unwrap();
        assert_eq!(again, skill_md, "install must be idempotent");
        let _ = std::fs::remove_dir_all(&root);
    }

### Verify & Close

@call verify(src/skill_install.rs)
@call gate(src/skill_install.rs)
@call commit("src/skill_install.rs", "feat(lmd-finishing): install stub (no companions/assets)")
@phase-end

@phase "task-3"
## Task 3: COVERAGE rows (availability.rs)

**Files:** Modify `src/availability.rs`.

Anchor the executing-plans COVERAGE rows ending at `availability.rs:194` and the list terminator
`];` at `availability.rs:195`. Insert the finishing rows BEFORE the `];` (directive names must be
registered — `include`/`query` are core bridges, confirmed in `bridges/mod.rs`
`default_registry`). Match the exact backing string sibling `@query` rows use — anchor
`@read src/availability.rs mode=lines:8-60` before writing:

    ("lmd-finishing-a-development-branch", "pre-context", "include", "fragment-compose"),
    ("lmd-finishing-a-development-branch", "detect-env", "query", "ctx_shell"),
    ("lmd-finishing-a-development-branch", "merge-local", "query", "ctx_shell"),

Add a test mirroring `coverage_rows_executing_plans` at `availability.rs:309`:

    #[test]
    fn coverage_rows_finishing() {
        let rows: Vec<&(&str, &str, &str, &str)> = COVERAGE
            .iter()
            .filter(|r| r.0 == "lmd-finishing-a-development-branch")
            .collect();
        assert!(!rows.is_empty(), "lmd-finishing-a-development-branch must have COVERAGE rows");
        let has = |step: &str, dir: &str| rows.iter().any(|r| r.1 == step && r.2 == dir);
        assert!(has("pre-context", "include"), "pre-context → include (hard-rules baseline)");
        assert!(has("detect-env", "query"), "detect-env → query (git state)");
        assert!(has("merge-local", "query"), "merge-local → query (git merge/cleanup)");
    }

The pre-existing `every_covered_directive_is_registered` gate (`availability.rs:220`) then proves
every new directive resolves in `default_registry`.

### Verify & Close

@call verify(src/availability.rs)
@call gate(src/availability.rs)
@call commit("src/availability.rs", "feat(lmd-finishing): COVERAGE rows (include/query per phase)")
@phase-end

@phase "task-4"
## Task 4: Rewire the two skills that reference the external skill

**Files:** Modify `content/skills/lmd-executing-plans/body.lmd.md` and
`content/skills/lmd-subagent-driven-development/body.lmd.md`.

**No-loss:** these are seed edits — the embedded `include_str!` copy and the on-disk seed stay
byte-identical (the const re-embeds on build).

In `content/skills/lmd-executing-plans/body.lmd.md` — the `finish` phase at `body.lmd.md:120-121`
currently reads:

    - Branch finishing via the external finishing-a-development-branch reference (merge / PR /
      cleanup choice presented to the human) — until an lmd port exists.

Replace with:

    - Branch finishing: invoke the lmd-finishing-a-development-branch skill (merge / PR / keep /
      cleanup choice presented to the human).

In `content/skills/lmd-subagent-driven-development/body.lmd.md` — the `finish` phase at
`body.lmd.md:131-132` currently reads:

    Branch finishing: until an lmd port exists, follow the external
    finishing-a-development-branch reference (merge / PR / cleanup choice presented to the human).

Replace with:

    Branch finishing: invoke the lmd-finishing-a-development-branch skill (merge / PR / keep /
    cleanup choice presented to the human).

Verify both bodies still render their `finish` phase:

@call render_check("lmd-executing-plans", "finish")
@call render_check("lmd-subagent-driven-development", "finish")

Add a rewiring gate test in `skills.rs` (both bodies now name the port; neither carries the stale
"until an lmd port exists" wording):

    #[test]
    fn finish_phases_are_rewired_to_lmd_port() {
        for name in ["lmd-executing-plans", "lmd-subagent-driven-development"] {
            let body = skill_body(name).unwrap();
            assert!(
                body.contains("lmd-finishing-a-development-branch"),
                "{name} finish phase must invoke the lmd port"
            );
            assert!(
                !body.contains("until an lmd port exists"),
                "{name} must drop the stale external-reference wording"
            );
        }
    }

### Verify & Close

@call verify(content/skills/lmd-executing-plans/body.lmd.md)
@call gate(src/skills.rs)
@call commit("content/skills/lmd-executing-plans/body.lmd.md content/skills/lmd-subagent-driven-development/body.lmd.md src/skills.rs", "feat(lmd-finishing): rewire executing-plans + subagent-driven finish phases to the lmd port")
@call remember_decision("finish phases of lmd-executing-plans + lmd-subagent-driven-development now invoke lmd-finishing-a-development-branch (superpowers reference fully replaced)")
@phase-end
