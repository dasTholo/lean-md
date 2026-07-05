<!-- lmd Phase 9 gloss table (D-5). Format: 2-column markdown table.
     Key = directive name or `name:op`. Slots: {N}=positional N, {raw}=all args,
     {key}=named arg. Lookup order: name:op → name → generic fallback. -->

| Directive        | Gloss template                         |
|------------------|----------------------------------------|
| read             | Read file `{0}`                        |
| search           | Search for `{0}`                       |
| list             | List directory `{0}`                   |
| query            | Run: `{raw}`                           |
| find             | Semantic search: `{raw}`               |
| symbol:refs      | Resolve references of `{1}`            |
| symbol:def       | Find definition of `{1}`               |
| symbol:impl      | Find implementations of `{1}`          |
| symbol:overview  | Symbol overview of `{1}`               |
| symbol           | Symbol analysis: `{raw}`               |
| graph:dependents | Resolve dependents of `{dependents}`   |
| graph:callers    | Resolve callers of `{callers}`         |
| graph:callees    | Resolve callees of `{callees}`         |
| graph            | Graph analysis: `{raw}`                |
| edit             | Apply code change                      |
| repomap          | Build repo map                         |
| impact           | Impact analysis for `{0}`              |
| architecture     | Architecture overview                  |
| outline          | Outline of `{0}`                       |
| routes           | List routes                            |
| smells           | Check code smells                      |
| review           | Code review                            |
| inspect          | Run inspections                        |
| count            | Count: `{raw}`                         |
| refactor         | Refactor: `{raw}`                      |
| reformat         | Format code: `{0}`                     |
| checkpoint       | Checkpoint (shadow-git) `{raw}`        |
| compress         | Compress session `{raw}`               |
