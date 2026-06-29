# Tool-Verfügbarkeits-Audit — Brainstorming-Pfad (Phase 10)

Coverage-Matrix: jeder Brainstorming-Workflow-Schritt → lmd-Direktive → lean-ctx-Backing.
Quelle der Wahrheit ist `rust/src/lmd/availability.rs::COVERAGE` (dieser Doc-Text ist die
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
| handoff          | `@dispatch`   | fragment-compose      |
| handoff          | `@handoff`    | `ctx_handoff`         |

## Bewusst NICHT im Brainstorming-Pfad (Gap-Liste, transparent)

- `ctx_benchmark` — Performance-Messung, kein Authoring-Schritt
- `ctx_package` — Distribution, kein Authoring-Schritt
- `ctx_provider` — externe Datenquellen, separater Pfad
