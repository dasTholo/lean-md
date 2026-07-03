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
