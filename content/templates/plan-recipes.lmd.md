# plan-recipes — project-local macro library for .lmd.md implementation plans
#
# Imported by every generated plan via:  @import .lean-ctx/lean-md/plan-recipes /
# CONVENTION: each @define's FIRST body line is an HTML-comment description
# (<!-- ... -->). It feeds `lean-md render plan-recipes.lmd.md --signatures` and
# MUST stay present (index-completeness gate). Values like test_cmd are overridden
# in .lean-ctx/lean-md/vars.toml (vars.toml wins) with no plan or skill edit.

@import .lean-ctx/lean-md/lang/rust /

@define test(name)
<!-- Run one test by name via the project test command ({{ var test_cmd }}) -->
Run: {{ var test_cmd }} {{ name }}
@define-end

@define commit(paths, msg)
<!-- Stage the given paths and commit with a message -->
Run:
    git add {{ paths }}
    git commit -m "{{ msg }}"
@define-end

@define tdd(name)
<!-- One red-to-green TDD cycle: failing test, run red, implement, run green -->
1. Write the failing test `{{ name }}`.
2. Run: {{ var test_cmd }} {{ name }} — Expected: FAIL.
3. Implement the minimal code to pass.
4. Run: {{ var test_cmd }} {{ name }} — Expected: PASS.
@define-end

@define verify(paths)
<!-- Inspect a change via unified diff instead of copy-pasting the code -->
Run: `@read {{ paths }} mode=diff` — review exactly what changed on these paths.
@define-end

@define review_change()
<!-- Post-change review gate: fused impact + caller-tracking + smells + test-discovery -->
Run: `@query git diff | @review diff-review` — fused review verdict on the working diff.
@define-end

@define check_smells(path)
<!-- Code-smell findings on a path (ctx_smells, default scan) -->
Run: `@smells {{ path }}` — surface code-smell findings on the changed file.
@define-end

@define inspect(path)
<!-- IDE inspections if an IDE backend is live, else a headless smell scan -->
1. Run: `@inspect {{ path }}` — IDE diagnostics (priority; needs a running IDE backend).
2. If it returns BACKEND_REQUIRED (no IDE), run: `@smells {{ path }}` instead (headless fallback).
@define-end

@define reformat_commit(paths, msg)
<!-- Reformat (rustfmt via ctx_refactor) then stage + commit the paths -->
1. Run: `@reformat {{ paths }}` — format before committing.
2. Stage {{ paths }} and commit with message: {{ msg }}
@define-end

@define remember_decision(content)
<!-- Persist a durable fact/gotcha at task end (ctx_knowledge write) -->
Run: `@remember {{ content }}` — save the decision as a durable fact.
@define-end

@define recall_context(query)
<!-- Pull durable context at task start (ctx_knowledge recall) -->
Run: `@recall {{ query }}` — pull the durable context a prior task saved.
@define-end

@define callers(symbol)
<!-- Who calls this symbol — anchor for refactor tasks (ctx_callgraph) -->
Run: `@graph callers {{ symbol }}` — list callers to anchor a refactor.
@define-end

@define gate(paths)
<!-- Pre-commit quality bar: reformat, lint, full test suite (lint_cmd/test_cmd via vars.toml) -->
1. Run: `@reformat {{ paths }}`
2. Run: {{ var lint_cmd }} — Expected: clean.
3. Run: {{ var test_cmd }} — Expected: PASS.
@define-end

@define render_check(skill, phase)
<!-- Render one skill/plan phase via the CLI; assert non-empty + byte-stable (#498) -->
Run: cargo run -q --bin lean-md -- render --skill {{ skill }} --phase {{ phase }} --consumer=ai
— Expected: non-empty, no eval err, byte-stable across two runs.
@define-end
