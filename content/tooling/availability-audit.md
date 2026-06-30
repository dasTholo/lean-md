# Tool-Verfügbarkeits-Audit — Brainstorming-Pfad (Phase 10)

Coverage-Matrix: jeder Brainstorming-Workflow-Schritt → lmd-Direktive → lean-ctx-Backing.
Quelle der Wahrheit ist `src/availability.rs::COVERAGE` (dieser Doc-Text ist die
menschenlesbare Projektion; das Gate prüft Registrierung gegen `default_registry()`).

| Workflow-Schritt | lmd-Direktive | lean-ctx-Backing      |
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

## Bewusst NICHT im Brainstorming-Pfad (Gap-Liste, transparent)

- `ctx_benchmark` — Performance-Messung, kein Authoring-Schritt
- `ctx_package` — Distribution, kein Authoring-Schritt
- `ctx_provider` — externe Datenquellen, separater Pfad

## lmd-test-driven-development — Coverage

TDD ist Prosa-Disziplin (phasenweise gerendert), direktiv-arm:

| Workflow-Schritt | lmd-Direktive | lean-ctx-Backing |
| red              | `@read`       | `ctx_read`       |

**Bewusster Gap:** Die Test-Ausführung (`ctx_shell "cargo nextest run"`) ist **keine**
registrierte Direktive — sie läuft als rohes `ctx_shell`, nicht als Code-Intel-Direktive.
RED/GREEN-Verifikation ist Prosa-Anweisung im Body, kein Registry-Eintrag (transparent, kein Loch).

## lmd-writing-skills — Coverage

| Workflow-Schritt | lmd-Direktive | lean-ctx-Backing |
| red (baseline read) | `@read` | `ctx_read` |
| green (tester dispatch) | `@dispatch` | fragment-compose |
| companion (@include skill-authoring-core) | `@include` | fragment-compose |

Die green-Phase dispatcht einen Tester-Subagenten, dessen Brief der Companion
`testing/methodology` ist (`@dispatch skill="lmd-writing-skills"
companion="testing/methodology" role=test`). Test execution (subagent pressure
scenarios) bleibt Prosa-Disziplin, keine registrierte Direktive — transparent
hier vermerkt.
