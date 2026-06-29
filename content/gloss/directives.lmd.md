<!-- lmd Phase 9 gloss table (D-5). Format: 2-Spalten-Markdown-Tabelle.
     Key = Direktiven-Name oder `name:op`. Slots: {N}=positional N, {raw}=alle Args,
     {key}=benannter Arg. Lookup-Reihenfolge: name:op → name → generischer Fallback. -->

| Direktive        | Gloss-Template                         |
|------------------|----------------------------------------|
| read             | Datei `{0}` lesen                      |
| search           | Suchen nach `{0}`                      |
| list             | Verzeichnis `{0}` auflisten            |
| query            | Ausführen: `{raw}`                     |
| find             | Semantische Suche: `{raw}`             |
| symbol:refs      | Referenzen von `{1}` ermitteln         |
| symbol:def       | Definition von `{1}` finden            |
| symbol:impl      | Implementierungen von `{1}` finden     |
| symbol:overview  | Symbol-Überblick von `{1}`             |
| symbol           | Symbol-Analyse: `{raw}`                |
| graph:dependents | Abhängige von `{dependents}` ermitteln |
| graph:callers    | Aufrufer von `{callers}` ermitteln     |
| graph:callees    | Aufgerufene von `{callees}` ermitteln  |
| graph            | Graph-Analyse: `{raw}`                 |
| edit             | Code-Änderung anwenden                 |
| repomap          | Repo-Karte erstellen                   |
| impact           | Impact-Analyse für `{0}`               |
| architecture     | Architektur-Überblick                  |
| outline          | Outline von `{0}`                      |
| routes           | Routen auflisten                       |
| smells           | Code-Smells prüfen                     |
| review           | Code-Review                            |
| inspect          | Inspektionen ausführen                 |
| count            | Zählen: `{raw}`                        |
| refactor         | Refactoring: `{raw}`                   |
| reformat         | Code formatieren: `{0}`                |
