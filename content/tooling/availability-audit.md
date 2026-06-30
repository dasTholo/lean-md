# Tool availability audit — brainstorming path (phase 10)

Coverage matrix: each brainstorming workflow step → lmd directive → lean-ctx backing.
The source of truth is `src/availability.rs::COVERAGE` (this doc-text is the
human-readable projection; the gate checks registration against `default_registry()`).

| Workflow step | lmd directive | lean-ctx backing      |
|------------------|---------------|-----------------------|
| explore          | `@read`       | `ctx_read`            |
| explore          | `@list`       | `ctx_tree`            |
| explore          | `@search`     | `ctx_search`          |
| explore          | `@find`       | `ctx_semantic_search` |
| approaches       | `@graph`      | `graph_index`         |
| approaches       | `@impact`     | `ctx_impact`          |
| write-spec       | `@edit`       | `ctx_edit`            |
| write-spec       | `@remember`   | `ctx_knowledge`       |
| self-review      | `@review`     | `ctx_review`          |
| self-review      | `@dispatch`   | fragment-compose      |
| spec-reviewer (companion) | `@dispatch` | fragment-compose |
| handoff          | `@dispatch`   | fragment-compose      |
| handoff          | `@handoff`    | `ctx_handoff`         |

## Deliberately NOT in the brainstorming path (gap list, transparent)

- `ctx_benchmark` — performance measurement, not an authoring step
- `ctx_package` — distribution, not an authoring step
- `ctx_provider` — external data sources, separate path

## lmd-test-driven-development — Coverage

TDD is prose-discipline (rendered phase-by-phase), directive-light:

| Workflow step | lmd directive | lean-ctx backing |
| red              | `@read`       | `ctx_read`       |

**Deliberate gap:** Test execution (`ctx_shell "cargo nextest run"`) is **not** a
registered directive — it runs as raw `ctx_shell`, not as a code-intel directive.
RED/GREEN verification is a prose instruction in the body, not a registry entry (transparent, not a hole).

## lmd-writing-skills — Coverage

| Workflow step | lmd directive | lean-ctx backing |
| red (baseline read) | `@read` | `ctx_read` |
| green (tester dispatch) | `@dispatch` | fragment-compose |
| companion (@include skill-authoring-core) | `@include` | fragment-compose |

The green phase dispatches a tester subagent whose brief is the companion
`testing/methodology` (`@dispatch skill="lmd-writing-skills"
companion="testing/methodology" role=test`). Test execution (subagent pressure
scenarios) remains prose-discipline, not a registered directive — noted here transparently.
