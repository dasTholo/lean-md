@lean-md
consumer: ai

@var test_cmd default="cargo test" desc="project test runner command"
@import .lean-ctx/lean-md/plan-recipes /

# <Feature> Implementation Plan — copy this template into docs/lean-md/plans/

A `.lmd.md` plan renders ONE task at a time: `lean-md render <plan.lmd.md> --phase task-N`.
Everything a task needs sits inside its `@phase` block; shared vocabulary lives in the
meta-head above (the two active lines `@var test_cmd …` and `@import … /plan-recipes /`).
Prose lines and inline-code directives are guidance — only a directive that STARTS a line
is active.

## Meta-head (already active above)

`@var test_cmd default="cargo test"` declares the test command once; override it in
`.lean-ctx/lean-md/vars.toml` (vars.toml wins) with no plan edit. `@import
.lean-ctx/lean-md/plan-recipes /` loads the macro library so `@call` resolves in every
task phase. Below the meta-head, write Goal / Architecture / Global Constraints as prose
copied from the spec.

## One @phase per task — the pattern

For EXISTING code, anchor it instead of pasting: `@read src/lib.rs mode=signatures`,
`@symbol some_fn`, or `path:line`. For NEW code, show it verbatim. Keep interfaces,
commands and "Expected:" verbatim. Verify with `@read mode=diff`. For boilerplate, call a
recipe: `@call test("case_name")`, `@call commit("paths", "message")`, `@call
tdd("case")`. Discover the recipes with `lean-md render
.lean-ctx/lean-md/plan-recipes.lmd.md --signatures`.

@phase "task-1"
## Task 1: <name> (rendered example — replace with your real first task)

**Files:** Create `src/foo.rs`. **Interfaces:** Produces `pub fn foo(x: u32) -> u32`.

New code (shown verbatim because it does not exist yet):

    pub fn foo(x: u32) -> u32 { x + 1 }

@call test(foo_adds_one)
@call commit(src/foo.rs, feat: add foo)
@phase-end
