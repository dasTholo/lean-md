<!-- lmd-writing-skills body — rendered phase-by-phase via ctx_md_render -->

@phase "red"
@include skill-authoring-core

## RED — write the failing test first (baseline)

Run a pressure scenario against a subagent WITHOUT the skill. This is "watch the
test fail": you must see what agents naturally do before you write anything.

Document exactly: what choice did they make? what rationalizations did they use
(verbatim)? which pressure triggered the violation? For discipline skills combine
3+ pressures (time + sunk cost + authority + exhaustion). ALWAYS run a no-guidance
control — if the control does not exhibit the failure, there is nothing to fix: stop.

next: render phase "green".
@phase-end

@phase "green"
@include skill-authoring-core

## GREEN — write the minimal skill

Write the skill that addresses those SPECIFIC baseline rationalizations — nothing
for hypothetical cases. Then run the same scenarios WITH the skill: the agent must
now comply.

Match the form to the failure:
- skips/violates a rule under pressure -> prohibition + rationalization table + red flags
- output has the wrong shape -> a positive recipe/contract stating what the output IS
- omits a required element -> a REQUIRED structural slot in the template they fill in
- behavior should depend on a condition -> a conditional keyed to an observable predicate

Micro-test the wording before full scenarios: one fresh-context sample per call,
always a no-guidance control, 5+ reps, read every flagged match manually, treat
variance as a metric (five interpretations across five reps = wording not binding).

To pressure-test the skill you just wrote, dispatch a tester subagent whose brief
is the full testing methodology:

@dispatch skill="lmd-writing-skills" companion="testing/methodology" role=test to_agent="{{ controller_id }}"

next: render phase "refactor".
@phase-end

@phase "refactor"
@include skill-authoring-core

## REFACTOR — close loopholes only under green

Agent found a NEW rationalization? Add an explicit counter, then re-test until
bulletproof. Build the rationalization table from every iteration; create a
red-flags list so agents can self-check.

STOP before moving to the next skill: do NOT batch-create skills untested. The
deployment checklist (see companion "testing/creation-checklist") is mandatory
for EACH skill. Deploying untested skills = deploying untested code.

For loophole-closing technique render the companion:
`ctx_md_render(skill="lmd-writing-skills", companion="bulletproofing")`.

After closing loopholes, re-dispatch the same tester
(`@dispatch skill="lmd-writing-skills" companion="testing/methodology" role=test`)
and re-verify the agent still complies under pressure.

next: return to RED for the next skill, or ship.
@phase-end

@phase "rationalizations"
@include skill-authoring-core

## Common Rationalizations for Skipping Testing (Excuse | Reality)

| Excuse | Reality |
| "Skill is obviously clear." | Clear to you != clear to other agents. Test it. |
| "It's just a reference." | References have gaps and unclear sections. Test retrieval. |
| "Testing is overkill." | Untested skills have issues. Always. 15 min testing saves hours. |
| "I'll test if problems emerge." | Problems = agents can't use the skill. Test BEFORE deploying. |
| "I'm confident it's good." | Overconfidence guarantees issues. Test anyway. |
| "Academic review is enough." | Reading != using. Test application scenarios. |
| "No time to test." | Deploying an untested skill wastes more time fixing it later. |

**All of these mean: test before deploying. No exceptions.**

For the full testing methodology render the companion:
`ctx_md_render(skill="lmd-writing-skills", companion="testing/methodology")`.

next: return to your active phase (red/green/refactor).
@phase-end
