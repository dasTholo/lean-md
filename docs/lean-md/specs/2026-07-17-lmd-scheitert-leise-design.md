# lean-md scheitert leise — `check`-Semantik, `.ext`-Generalisierung, Seed-Provenienz

**Status:** approved (Brainstorm 2026-07-17)
**Umfang:** implementieren + committen. **Kein Publish, kein Tag** — der Release wird nach
Abschluss dieses Pakets separat entschieden und gefahren, gemeinsam mit dem Vorpaket
„Render-Aufrufkonvention" (2026-07-16), das ebenfalls ohne Publish endete.
**Version:** 0.2.1 ist der **Kandidat**, keine gezogene Nummer. Berührt ausschließlich die
**Binary**-Linie (`src/**` + `content/**` via `include_str!`); **kein** Task fasst
`content/skills/**` an. In diesem Paket wird **keine** Versionsnummer angefasst — weder in
`lean-ctx-addon.toml` noch in `content/skills.ctxpkg-hash`.

## Problem

Der rote Faden aller Punkte: **das Tool tut etwas anderes als dokumentiert und sagt nichts.**
Jeder Defekt ist selbst-verschleiernd — es gibt kein Signal, an dem ein Autor merken könnte,
dass seine Datei nicht das tut, was sie behauptet.

Fünf Ausprägungen, aus dem Code belegt:

- **`check` parst nur.** Es sagte `lmd ok` zu fehlendem `phase=`, zu `role=exec` (existiert
  nicht), zu doppelten Phasen-Namen. Ein Autor mit grünem `check` hält seine Datei für
  korrekt — das ist der Vertrauensbruch. `role` *wird* validiert, aber erst zur Render-Zeit
  (`dispatch.rs:94`); `check` rendert nicht und sieht es deshalb nie.
- **Doppelte `@phase`-Namen verschlucken Content.** Der zweite Block verschwindet spurlos,
  `--list-phases` zeigt den Namen einmal, `--phase X` rendert nur den ersten. Stiller
  Content-Verlust in einem Dokumentations-Tool.
- **`@dispatch brief=` wird geschluckt und verworfen.** Der String `brief` kommt in `src/`
  als Argument nicht vor; die Bridge liest `phase`/`companion`/`skill`/`role`/`to_agent` und
  fragt alles andere nie ab. Unbekannte Argumente fallen lautlos auf den Boden.
- **Die Fragment-Doc-Kommentare lügen.** `fragments.rs:1-3` behauptet „files override/extend
  them", `seeds.rs:1-6` behauptet „project file overrides the embedded seed". Für die drei
  Built-ins (`hard-rules`, `dispatch-contract`, `parallel-dispatch`) ist beides falsch: sie
  sind weder ersetzbar noch erweiterbar, `resolve()` returned früh (`:59-61`). Für
  `lang/rust`, `tooling/mcp-tools`, `plan-recipes` stimmt „override" nur zufällig — dort
  existiert gar kein Built-in, es sind reine Dateien.
- **Materialisierte Seeds altern still.** `materialize_contracts` ist absent-only; nach jeder
  Seed-Änderung im `content/` wird die lokale Kopie nie nachgezogen. `force=true` existiert
  genau dafür, aber es gibt keinen Aufrufer außer `skill install --force`, und den fährt
  niemand nach einem `addon update`.

### Der Live-Befund

Im Dev-Repo selbst (2026-07-17) sind **vier von fünf** materialisierten Seeds veraltet — keine
davon eine Anpassung, alle schlicht alte Kopien:

| Datei                          | Zustand                                                     |
|--------------------------------|-------------------------------------------------------------|
| `dispatch-contract.ext.lmd.md` | stale **und wirksam** — `#`-Zeilen statt `<!-- -->`, daher nicht inert |
| `lang/rust.lmd.md`             | stale — trägt noch „`@edit` is for non-symbol changes"      |
| `tooling/mcp-tools.lmd.md`     | stale — ohne den anchored-loop                              |
| `plan-template.lmd.md`         | stale — alte `#498`-Referenz                                |
| `plan-recipes.lmd.md`          | identisch                                                   |

Nur die erste ist **strukturell** kaputt: sie soll leer sein, sagt es aber in Markdown statt in
HTML und wird deshalb angehängt (siehe Entscheidung 6). Die anderen drei sind korrekt
aufgebaut und bloß veraltet. Für den Refresh macht das keinen Unterschied — für die Diagnose
schon.

`lang/rust` und `tooling/mcp-tools` haben **keinen** Built-in — für sie gewinnt der jailed-File-
Fallback. Das Repo rendert also real die überholte Edit-Regel statt der aktuellen, die im Seed
steht. Der Defekt hat bereits Content beeinflusst, und nichts hat es gemeldet.

**Wurzel in allen fünf Fällen:** es fehlt die Instanz, die den Widerspruch bemerkt. Weder
kennt `check` die Directive-Semantik, noch kennt `materialize_contracts` die Provenienz einer
Datei, noch sagt irgendwer, dass ein Fragment nur so tut, als sei es erweiterbar.

## Designentscheidungen

### 1. Verteilung: reine Binary-Linie

`docs/dev-readme.md` ordnet `src/**` und `content/core/**` dem **Binary** zu, nur
`content/skills/**` dem **Pack**. Alle fünf Tasks liegen in `src/**`; auch der `.ext`-Seed ist
Binary-Content, weil er über `include_str!` in `PROJECT_SEEDS` hängt und nicht im Pack liegt.

**Tabellenlücke im dev-readme:** die Zuordnungstabelle listet `content/core/**`,
`content/gloss/**` und `src/**` als Binary — **`content/templates/**` fehlt**, obwohl dort der
`.ext`-Seed und `plan-recipes`/`plan-template` liegen, alle drei via `include_str!`. Die
Zuordnung folgt zwingend aus dem Einbettungsmechanismus, aber die Lücke lädt die nächste Spec
zur selben Kollision ein. Sie wird in Task P8 mitgeschlossen (eine Tabellenzeile), weil P8 die
`PROJECT_SEEDS` ohnehin anfasst.

**Daraus folgt:** dieses Paket erzwingt keinen Pack-Bump. Der Pack-Bump wird ohnehin fällig —
wegen der Stub-Straffung aus dem Vorpaket, wo `pack_drift` seit `bace97a` rot steht und wartet.
Beide Pakete gehen **gemeinsam** raus, wenn released wird: ein Tag, ein 5-Leg-Build, eine
Choreografie statt zweier. Das ist genau der Grund, warum das Vorpaket auf seinen Publish
verzichtet hat — und warum auch dieses Paket beim Commit endet.

### 2. Versionskandidat 0.2.1 — P1/P2 sind Bugfixes, keine Breaking Changes

P1 und P2 sind streng genommen verhaltensbrechend: eine Datei mit doppelten `@phase`-Namen
rendert heute und bricht danach; `@dispatch brief=…` rendert heute und bricht danach. In `0.x`
ist Minor der Breaking-Slot (Vorspec, Entscheidung 3).

**Angesteuert wird dennoch 0.2.1** (gezogen wird die Nummer erst beim Release, nicht hier).
Begründung: was hier bricht, ist kein funktionierender
Zustand, sondern ein bisher unentdeckter Defekt. Eine Datei mit doppelten Phasen war nie
korrekt — sie hat stillschweigend Content verloren. `brief=` wurde nie gerendert, es war immer
schon ein Autorenfehler. Ein Patch, der einen stillen Fehler laut macht, ist die Definition
von „scheitert leise beheben", nicht eine Verhaltensänderung.

**Der Preis, ehrlich benannt:** bei einem Konsumenten mit so einer Datei bricht der Build nach
`addon update`, ohne dass er etwas geändert hat. Das gehört in die **Release-Notes**, nicht in
eine Fußnote. Bewusst so entschieden, nicht übersehen.

### 3. P8 — `lean-md.lock` (Seed-Provenienz)

**Problem:** `materialize_contracts` sieht nur „lokal ≠ embedded" und kann daraus nicht
ableiten, ob der Nutzer editiert hat oder ob der Seed weitergezogen ist. Beide verfügbaren
Modi sind falsch: absent-only lässt Seeds altern, `force` überschreibt echte Anpassungen. Es
fehlt der dritte.

**Entscheidung:** Provenienz mitschreiben, nach dem Vorbild von pacman `.pacnew` / dpkg
conffiles. Neu: **`.lean-ctx/lean-md.lock`** — neben `ctxpkg.lock`, bewusst in dessen Sprache
(„generated by …; commit this file"), aber **lean-md-eigen**. Nicht in `ctxpkg.lock` hinein:
die Datei gehört lean-ctx, wird von `lean-ctx pack install` generiert und verfolgt die
**Pack**-Linie; die Seeds sind **Binary**-Content. Zwei Linien, zwei Locks.

**Warum ein Hash und nicht ein Byte-Vergleich:** „lokal == embedded" ließe sich direkt
vergleichen — der Seed steckt via `include_str!` im Binary. Aber „lokal == der Stand von
damals" braucht den **historischen** Seed-Content, den ein neueres Binary nicht mehr trägt.
Nur der Hash konserviert ihn. Ohne Hash keine Provenienz, ohne Provenienz kein dritter Modus.

**`sha2` muss von `[dev-dependencies]` nach `[dependencies]`.** Heute steht es in
`Cargo.toml:35-37` unter dev — einziger Nutzer ist `tests/pack_drift.rs`. Das **Release-Binary
trägt keinen SHA-256-Code** und könnte den Lock zur Laufzeit nicht schreiben. Das ist eine
Voraussetzung von P8, kein Nebendetail: ohne den Umzug kompiliert der Task nicht.

Gewählt wurde SHA-256 (nicht ein dependency-freier FNV-1a), weil der Nutzer jeden Wert selbst
nachrechnen können muss. Ein Lock, dessen Werte niemand prüfen kann, ist bei einer
Fehldiagnose wertlos — und Fehldiagnosen sind das Thema dieses Pakets.
`std::hash::DefaultHasher` scheidet aus: laut std-Doku nicht stabil über Rust-Releases — ein
`rustup update` würde jeden Lock-Wert entwerten und eine `.new`-Flut auslösen.

**Format: `sha256sum`, nicht TOML.** Der Lock übernimmt das Format von
`content/skills.sha256` (`<hex>␠␠<relpath>`, `#`-Kommentare), das `pack_drift.rs:57-75` bereits
erzeugt — **nicht** das TOML seines Nachbarn `ctxpkg.lock`.

Grund: **die Semantik von `sha256sum -c` ist identisch mit der Provenienz-Frage.** „Weicht die
Datei vom festgehaltenen Hash ab?" ist exakt „hat der Nutzer editiert?". Damit beantwortet der
Nutzer sie selbst — ein Standardbefehl, ohne lean-md, ohne uns glauben zu müssen. Genau die
Selbstprüfbarkeit, wegen der SHA-256 gewählt wurde; in TOML verpufft sie, weil jeder Wert von
Hand verglichen werden müsste. Die Konsistenz mit dem Nachbarn `ctxpkg.lock` wiegt das nicht
auf: die beiden Dateien beantworten verschiedene Fragen.

Die Pfade sind **relativ zum Lock-Verzeichnis** (`.lean-ctx/`), damit `sha256sum -c` ohne
Pfad-Gefummel läuft. `binary_version` steht als `#`-Kommentar; lean-md parst ihn.

> **ANNAHME (in der Brainstorm-Session nicht verifiziert):** GNU coreutils `sha256sum --check`
> ignoriert Zeilen mit führendem `#`. Das entspricht dem Verhalten von `md5sum.c`, ist hier
> aber **nicht ausgeführt** worden (`sha256sum` steht nicht in der Shell-Allowlist), und
> BSD/macOS liefert `shasum` statt `sha256sum`. **Der erste P8-Test verifiziert sie** — trägt
> sie nicht, wandert `binary_version` in einen Sidecar oder entfällt (er ist Metadatum, kein
> Kernstück). Das `sha256sum`-Format selbst bleibt davon unberührt.

```
# lean-md.lock — generated by lean-md; commit this file.
# binary_version: 0.2.0
# Eigene Anpassungen prüfen:  cd .lean-ctx && sha256sum -c lean-md.lock
ad75963…  lean-md/lang/rust.lmd.md
b9f7f43…  lean-md/tooling/mcp-tools.lmd.md
5c1e802…  lean-md/dispatch-contract.ext.lmd.md
7e2a441…  lean-md/hard-rules.ext.lmd.md
c93f018…  lean-md/parallel-dispatch.ext.lmd.md
```

```
$ cd .lean-ctx && sha256sum -c lean-md.lock
lean-md/lang/rust.lmd.md: OK               ← unberührt → darf still heilen
lean-md/tooling/mcp-tools.lmd.md: FAILED   ← angepasst → bekommt .new
```

**Hash-Funktion in die lib — single source.** `render_manifest()` lebt heute in
`tests/pack_drift.rs`, weil `sha2` dev-only ist. Mit dem Umzug nach `[dependencies]` wandert
die Hash-Berechnung in die lib (`sha256_hex`), und `pack_drift.rs` nutzt sie statt einer
eigenen Kopie. Zwei Definitionen von „wie hashen wir" wären genau die Drift-Sorte, die dieses
Paket bekämpft.

Inhalt: Binary-Version + Hash je Seed **zum Zeitpunkt der Materialisierung**.

Refresh-Semantik, drei Fälle:

| lokal vs. lock | lock vs. embedded | Bedeutung            | Aktion                                          |
|----------------|-------------------|----------------------|-------------------------------------------------|
| gleich         | verschieden       | alt, unberührt       | still aktualisieren, Lock nachziehen            |
| verschieden    | —                 | **Nutzer-Anpassung** | **nie** überschreiben → `.new` daneben + melden |
| —              | gleich            | aktuell              | no-op                                           |

**Altbestand ohne Lock** (der heutige Zustand, 4 Dateien): Provenienz unbekannt → konservativ
`.new` + Meldung. Der Nutzer entscheidet einmal, danach existiert der Lock und alles läuft
automatisch. `materialize_contracts` behält `force` als Holzhammer; der neue, sichere Modus
tritt daneben.

### 4. P8 — Sicherstellung ohne Install-Hook

**Aus dem Manifest belegt:** `lean-ctx-addon.toml` kennt `[artifacts]`, `[dependencies]`,
`[mcp]` (stdio-Spawn) und `[capabilities]` — **keine Lifecycle-Phase, keinen Install-Hook**.
lean-ctx wired beim Install nur `LEAN_MD_SKILLS_DIR = "{pack_dir:…}"` und spawnt danach den
Server. Es gibt **keine Stelle**, an der lean-ctx lean-md-Code bei `addon add` / `addon update`
ausführt.

`materialize_contracts` läuft heute ausschließlich in `install_skill` (`skill_install.rs:136`).
Ein Endnutzer hat nur ctxpkg: er fährt `addon update`, der Pack wird getauscht, `install_skill`
läuft nie — die Seeds altern still weiter. **Das ist der Mechanismus, der die vier Dateien im
Dev-Repo hat altern lassen.**

**Entscheidung:** `lean-md mcp` fährt **beim Serverstart** den neuen lock-basierten Modus aus
Entscheidung 3 — **weder** absent-only **noch** `force`. Das ist der ganze Punkt: absent-only
würde die vier stale Dateien des Live-Befunds nie heilen, `force` würde Nutzer-Anpassungen
zerstören. Der Lock erlaubt erstmals, beide Fälle zu trennen: unberührt+alt heilt still,
angepasst bekommt `.new`. Der Gateway spawnt den Server in jeder Session, also greift es nach
jedem `addon update` ohne Zutun des Nutzers. `install_skill` fährt denselben Modus.

`materialize_contracts` bekommt dafür einen dritten Modus neben absent-only und `force`; die
beiden bestehenden bleiben unverändert erhalten (`--force` bleibt der bewusste Holzhammer).

**`render` und `check` bleiben lesend.** Der Renderer behält seine D-1-Zusage („PURE renderer",
`dispatch.rs:1`) und zahlt kein I/O auf dem heißen Pfad. Das Wiring sitzt am Serverstart, nicht
am Tool-Aufruf.

**Sichtbarkeit der Meldung — bewusst asymmetrisch:** beim MCP-Start ist `stdout` der
JSON-RPC-Kanal (eine Warnung dort korrumpiert das Protokoll), und `stderr` landet im
Gateway-Log, das der Agent nie sieht. Deshalb: der Normalfall (stale + unberührt) heilt
**still** — dort braucht niemand eine Meldung. Nur der Ausnahmefall (Nutzer-Anpassung +
`.new`) braucht Sichtbarkeit, und die kommt über **`lean-md check`**, wo der Nutzer hinschaut.
`stderr` bleibt zusätzlich für die Log-Diagnose.

**Scope von `lean-md check`:** `cmd_check` nimmt heute nur einen Dateipfad. Für den
Seed-Vergleich braucht es den `project_root` — abgeleitet wie überall sonst im Binary (der
Jail-Root, gegen den auch Fragmente aufgelöst werden), `contracts_dir` bleibt der etablierte
Default `.lean-ctx/lean-md`. Findet sich kein Projekt-Root, entfällt der Seed-Teil des Checks
lautlos; die Datei-Prüfung läuft unverändert.

### 5. P5 — `.ext` generisch in der Registry

**Verworfen: Overlay vor Built-in.** Die Vorspec führte das als Alternative („mächtig, ist
breaking") und notierte einen Konflikt: „ginge P5 auf Overlay-vor-Built-in, würde der
`.ext`-Fix aus Entscheidung 2 obsolet". Beide Punkte werden hier zurückgewiesen:

- **Overlay ist *replace*, `.ext` ist *extend* — verschiedene Semantiken.** Overlay als
  einziger Weg zwingt ein Projekt, den kompletten Contract zu **kopieren**, um eine Zeile zu
  ergänzen; die Kopie driftet dann für immer vom Upstream weg und bekommt nie wieder einen
  Fix. Das ist exakt das Argument, mit dem die Vorspec (Entscheidung 5) den handkopierten
  CLAUDE.md-Block abgeschafft hat. Ein Drift-Generator.
- **Overlay öffnet ein Guardrail-Loch.** Overlay-first machte auch `test-first-core.lmd.md`
  und `brainstorm-gate.lmd.md` lokal ersetzbar — ein Projekt könnte die TDD-Iron-Law
  wegdefinieren. Erweitern ist harmlos, Ersetzen nicht.

**Entscheidung:** der `.ext`-Fix wird nicht obsolet, sondern **generalisiert**. Entscheidung 2
der Vorspec war kein Zwischenschritt, sondern die richtige Richtung — nur zu eng gefasst (ein
Sonderpfad für genau einen Fragment-Namen).

`FragmentRegistry::resolve` lernt `<name>.ext.lmd.md` für **jedes** Fragment:

```
base = builtin | SKILL_INCLUDES | jailed <name>.lmd.md
ext  = jailed <name>.ext.lmd.md
ret  = base + ext            (skip-if-inert → byte-stabil, #498)
```

`contract_ext` und `strip_html_comments` wandern aus `dispatch.rs:31-54` in die Registry; die
Bridge liest keine Datei mehr selbst (Rückbau des Sonderpfads). Die Komposition bleibt **vor**
`render_body`, damit die `.ext` an Placeholder-Substitution und `@include`-Auflösung teilnimmt.

**Gewinn:** `hard-rules.ext.lmd.md` und `parallel-dispatch.ext.lmd.md` wirken erstmals — heute
wären das tote Dateien, die ein Nutzer anlegen kann, ohne dass irgendetwas passiert. Built-ins
bleiben unersetzbar, die Iron-Law bleibt geschützt.

**Die beiden neuen `.ext` werden `PROJECT_SEEDS`-Einträge.** Sonst bliebe der Gewinn
theoretisch: heute materialisiert `seeds.rs:24-42` nur `dispatch-contract.ext.lmd.md` — ein
Nutzer müsste `hard-rules.ext.lmd.md` erfinden und wüsste nicht einmal, dass es sie geben
kann. Ein Erweiterungspunkt, den niemand entdeckt, ist keiner. Mit dem Seed sieht er im
Verzeichnis, was erweiterbar ist, und der Kommentar erklärt es am Ort des Bedarfs — statt in
einer Doku, die er nicht durchsucht, bevor er weiß, dass es etwas zu suchen gibt.

Beide folgen dem bestehenden Muster von `dispatch-contract.ext.lmd.md` (reiner
HTML-Kommentar, damit skip-if-inert greift):

```
<!-- Hard-rules extension (project seed).
     Auto-composed after the built-in hard-rules fragment.
     Add project-specific tool-discipline rules below. Empty by default. -->
```

**Abgrenzung:** `.ext` gilt nur für die **drei Built-ins** (`hard-rules`, `dispatch-contract`,
`parallel-dispatch`) — sie sind unersetzbar, deshalb braucht es den Anbau. `lang/rust` und
`tooling/mcp-tools` haben keinen Built-in; ihre materialisierte Datei **ist** die Quelle und
wird direkt editiert. Ein `.ext` wäre dort ein zweiter Weg zum selben Ziel und wird bewusst
nicht angelegt.

**Die HTML-Kommentar-Form gilt NUR für `.ext`-Seeds — nicht für die übrigen.** Die
Inert-Prüfung beantwortet genau eine Frage: „ist die Datei leer genug, um sie *nicht*
anzuhängen?" Sie stellt sich ausschließlich bei `.ext`, weil nur die an ein Built-in angehängt
werden und ein unveränderter, leerer Seed den Output nicht verändern darf (#498). Bei
`dispatch-contract.ext` sind die `#`-Zeilen deshalb ein Bug: die Datei *will* leer sein, kann
es dem Code aber nicht sagen, weil Markdown-Überschriften kein Kommentar sind.

`lang/rust.lmd.md` und `tooling/mcp-tools.lmd.md` durchlaufen die Inert-Prüfung **nie**. Sie
werden nicht angehängt, sondern aufgelöst; ihr Inhalt *soll* erscheinen, samt der
`#`-Überschrift, mit der beide beginnen (`# Rust language pack (lmd)`, `# MCP tool pack
(lmd)`). Eine Umstellung auf HTML-Kommentare würde ihren Inhalt auskommentieren und die
Fragmente stillschweigend leeren — ein neuer Bug derselben Familie.

| Datei-Art               | Rolle                          | Kommentar-Form                |
|-------------------------|--------------------------------|-------------------------------|
| `<name>.ext.lmd.md`     | wird angehängt, darf leer sein | **HTML** — sonst nicht inert  |
| `lang/*`, `tooling/*`   | echtes Fragment mit Inhalt     | Markdown, wie jeder Content   |

Die vier stale Dateien im Dev-Repo brauchen deshalb **keine** unterschiedliche Behandlung im
Refresh, wohl aber eine unterschiedliche Diagnose: `dispatch-contract.ext` ist strukturell
kaputt (`#` statt `<!--`), `lang/rust` und `tooling/mcp-tools` sind bloß veraltete Kopien —
richtig aufgebaut, nur mit der alten Edit-Regel. P8 heilt beide gleich: der Hash weist sie als
unberührt aus, der Refresh zieht sie auf den aktuellen Seed.

Beide Doc-Kommentare (`fragments.rs:1-3`, `seeds.rs:1-6`) werden auf die Wahrheit gezogen:
**extend**, nicht override. Damit ist P5 vollständig aufgelöst.

### 6. P8 vor P5 — erzwungene Reihenfolge

**Der Defekt ist bereits aktiv, nicht hypothetisch.** Der P4-Fix aus dem Vorpaket ist
committed; `contract_ext` (`dispatch.rs:31`) liest die stale `.ext`, und
`strip_html_comments` (`:42`) entfernt nur `<!-- … -->`. Die drei `#`-Zeilen sind
Markdown-Überschriften, überleben die Inert-Prüfung und **hängen heute in jedem
Dispatch-Contract dieses Repos**. Verifiziert am 2026-07-17 durch
`lean-md render --skill lmd-brainstorm --phase self-review`: die drei Zeilen erscheinen im
gerenderten Contract, zwischen dem Hard-Rules-Block und dem Task-Block.

#498 ist damit im Dev-Repo **schon gekippt** — der Contract ist keine reine Funktion des
Built-ins mehr, sondern trägt Bytes aus einer alten Datei, die niemand angefasst hat. Das ist
zugleich der schärfste Beleg für die Diagnose dieses Pakets: der Defekt stand die ganze Zeit
sichtbar im Output, und nichts hat ihn gemeldet.

P5 generalisiert diesen Pfad auf **jedes** Fragment. Landet P5 vor P8, vervielfacht das Paket
den Bug, den es beheben soll (`hard-rules.ext` und `parallel-dispatch.ext` kämen hinzu). Die
Reihenfolge ist Architektur, nicht Geschmack.

### 7. P1 — deklaratives Arg-Schema (P3 fällt mit)

**Verworfen: Bridges validate-only aufrufen** — jede Bridge bräuchte einen garantiert
seiteneffektfreien Validate-Pfad, und `validate()` / `execute()` driften auseinander, sobald
jemand eine Prüfung nur in `execute` einbaut.
**Verworfen: die vier bekannten Fälle hart einprogrammieren** — fängt keinen fünften Fall und
keine künftige Directive; die nächste Lücke fällt wieder erst beim Nutzer auf.

**Entscheidung:** jede Directive deklariert **einmal** ihr Schema — Pflichtargumente,
optionale, Enum-Werte, Exklusiv-Gruppen. `check` **und** die Bridges lesen dieselbe Quelle:

```
SCHEMA["dispatch"] = {
  required_one_of: [["phase"], ["skill", "companion"]],
  optional: ["role", "to_agent"],
  enums: { role: ["dev", "review", "test"] },
}
```

Die Enum-Prüfung in `dispatch.rs:94-102` wird auf das Schema umgestellt, **nicht** dupliziert —
sonst entsteht genau die Drift zwischen „was `check` prüft" und „was der Renderer akzeptiert",
die den Bug erzeugt hat.

**P3 ist damit kein eigener Task**, sondern ein Testfall: `brief=` fällt als unbekanntes
Argument, ebenso jeder künftige Tippfehler (`phse=`, `to-agent=`). Die Alternative, `brief=`
echte Semantik zu geben, wurde verworfen — `companion=` liefert den Brief bereits; zwei
Mechanismen für einen Zweck.

### 8. P2 — Duplikat-Check im Parser

Der Check sitzt im **Parser**, nicht in `check`. Damit greift er in `check`, `render`,
`--list-phases`, MCP und CLI gleichermaßen; stiller Content-Verlust wird strukturell
unmöglich statt von einem Linter gemeldet, den man überspringen kann.

Die Fehlermeldung nennt **beide** Fundstellen (erste Definition + Duplikat), sonst sucht der
Autor in einer langen Datei.

### 9. P9 — Versions-Drift gegen `ctxpkg.lock`

Die Vorspec ließ bei P6 eine Lücke ausdrücklich offen:

> „In Pack-Content wandert er erst, wenn ein Binary **mit** dem Alias als Mindestversion
> durchsetzbar ist — was `min_lean_ctx` nicht leistet (es pinnt lean-ctx, nicht lean-md). Ein
> Mechanismus dafür fehlt; er gehört in die „scheitert leise"-Runde."

**Entscheidung:** lean-md liest `.lean-ctx/ctxpkg.lock` — **ausschließlich lesend**, nie
schreibend (die Datei gehört lean-ctx). Daraus wird die installierte Pack-Version bekannt und
gegen die vom Binary erwartete Spanne geprüft.

**Herkunft der Spanne — Konstante + Konsistenz-Gate, kein Runtime-Manifest-Lesen.** `^0.2`
steht heute nur in `lean-ctx-addon.toml`; die Datei wird **nicht** ins Binary eingebettet und
liegt beim Endnutzer nicht neben dem Binary — ein Runtime-Lookup hätte dort nichts zu lesen.
Das Binary trägt die Spanne deshalb als Konstante, abgesichert durch einen Test, der sie gegen
`lean-ctx-addon.toml` prüft. Das ist exakt das Muster des bestehenden
fragment-consistency-Gates (built-in == on-disk seed): eine Divergenz fällt in CI, nicht beim
Nutzer. Kein Runtime-Parse, kein Suchpfad, byte-stabil (#498).

**Verworfen: das installierte Addon-Manifest zur Laufzeit suchen** — es gibt keinen
zugesicherten Pfad dorthin, der Lookup könnte still ins Leere greifen, und ein Check, der
schweigt, weil er seine Referenz nicht findet, wäre wieder „scheitert leise".

**Kritische Einschränkung — nur Spannen-Verletzung, nicht Ungleichheit.** `docs/dev-readme.md`
sagt:

> „Binary and pack use **independent SemVer**. […] A content-only fix moves the pack to
> `0.2.1` while the binary stays at `0.2.0`. **That divergence *is* the benefit of the cut.**"

Ein Check, der bei `Pack 0.2.1 ≠ Binary 0.2.0` warnt, würde also den **gewollten Normalfall**
anmeckern — Warnungs-Rauschen ab Tag eins, und der Sinn des #727-Schnitts wäre verbaut. Gewarnt
wird ausschließlich, wenn die Pack-Version **außerhalb `version_req`** liegt (`^0.2` ⊅ `0.3.0`).
Ungleichheit innerhalb der Spanne ist Absicht und schweigt.

Damit wird eine Mindest-Binary-Version durchsetzbar — die Voraussetzung dafür, dass
Pack-Content künftig `lmd_render` nennen darf (heute nennt er bewusst `ctx_md_render`, weil das
auf beiden Binaries funktioniert).

### 10. P7 — raus aus dem Paket

`Transport closed` ist sporadisch, nicht argumentabhängig (der Verdacht auf `consumer:"ai"`
wurde per Retest widerlegt), und hat **keinen Repro**. Ein Plan-Task „finde einen Repro" hat
kein Abschlusskriterium und blockiert das Paket.

Die Retry-Guidance steht bereits in `lmd-rendering-skills` („einmal wiederholen — der Gateway
respawnt"); das genügt, bis der Fehler greifbar wird. P7 bleibt offen für eine eigene Runde,
sobald eine Reproduktion existiert.

## Task-Reihenfolge

```
P8  lean-md.lock + Refresh-Semantik + MCP-Start-Wiring   ← zuerst (erzwungen)
P5  .ext generisch in der Registry, Sonderpfad-Rückbau
P1  deklaratives Arg-Schema (P3 fällt mit)
P2  Duplikat-Check im Parser
P9  version_req-Prüfung gegen ctxpkg.lock
```

## Tests (TDD, rot zuerst)

**P8**

- Stale Seed + unberührt (Hash == Lock) → Refresh aktualisiert, Lock wird nachgezogen.
- Seed mit Nutzer-Edit (Hash ≠ Lock) → `.new` entsteht, **Original unangetastet**.
- Altbestand ohne Lock → konservativ `.new`, nichts wird überschrieben.
- Aktueller Seed → no-op, kein `.new`, keine Meldung.
- `lean-md mcp` schreibt/pflegt den Lock beim Start; `render`/`check` schreiben **nicht**
  (Beweis für die D-1-Purity: Dateizustand vor == nach).
- `lean-md check` meldet den `.new`-Fall sichtbar; der Still-Heil-Fall erzeugt **keine** Meldung.
- **Der Lock ist mit `sha256sum -c` prüfbar** — unberührte Seeds `OK`, editierte `FAILED`.
  Das ist der Selbstprüfbarkeits-Test: das Format muss von coreutils gelesen werden können,
  nicht nur von uns.
- Pfade im Lock sind relativ zu `.lean-ctx/` (sonst findet `-c` die Dateien nicht).
- `sha2` ist im **Release**-Profil verfügbar (nicht nur unter `cfg(test)`) — der Task
  kompiliert sonst nicht.
- `sha256_hex` lebt in der lib; `pack_drift.rs` nutzt sie statt einer eigenen Kopie
  (single source — beide Manifeste hashen nachweislich identisch).
- `pack_drift` bleibt nach dem Umbau unverändert grün/rot wie zuvor (kein Verhaltenswechsel
  am bestehenden Gate).

**P5**

- `hard-rules.ext.lmd.md` mit Regel → erscheint im Output nach dem Built-in (heute tote Datei).
- Unveränderter Seed → Output byte-identisch zu „ohne `.ext`" (#498) — kein bestehender
  Dispatch-Test kippt.
- `.ext` fehlt → unverändert.
- Jail-Escape greift weiterhin (`../etc/passwd.ext`).
- `.ext` nimmt an Placeholder-Substitution teil (`{{ role }}` in der `.ext` wird ersetzt).
- **Kopplungstest:** alte `.ext` mit `#`-Zeilen wird von P8 erkannt, **bevor** P5 sie anhängt.
- `dispatch.rs` liest keine Datei mehr selbst (Sonderpfad zurückgebaut).
- `hard-rules.ext` und `parallel-dispatch.ext` sind `PROJECT_SEEDS`-Einträge und
  materialisieren beim Install/MCP-Start (Entdeckbarkeit — sonst bleibt der Gewinn theoretisch).
- Beide neuen Seeds sind **inert** (reiner HTML-Kommentar) → Output byte-identisch zu „ohne
  Seed" (#498). Der Regressionsschutz gegen den Fehler, den der stale `.ext` heute macht.
- Die neuen Seeds landen im Lock (P8 kennt sie, sobald P5 sie registriert) — ein danach
  hinzugekommener Seed materialisiert absent-only, ohne `.new`.
- **Kein** `.ext`-Seed für `lang/rust` / `tooling/mcp-tools` (kein Built-in → die Datei ist
  die Quelle; ein zweiter Weg wäre der Fehler).

**P1/P3**

- `@dispatch brief=x phase=y` → ERR unknown argument, mit Auflistung der bekannten.
- `@dispatch role=exec` → ERR **in `check`**, nicht erst beim Render.
- `@dispatch` ohne `phase=`/`companion=` → ERR in `check`.
- `@dispatch phase=x companion=y` → ERR (Exklusiv-Gruppe), in `check`.
- Schema ist die einzige Quelle: `dispatch.rs` validiert nicht mehr eigenständig.

**P2**

- Doppelte `@phase` → ERR in `check` **und** `render` **und** `--list-phases` **und** MCP.
- Die Fehlermeldung nennt beide Fundstellen.

**P9**

- Pack außerhalb `version_req` → WARN.
- Pack innerhalb der Spanne, Version ≠ Binary → **kein Output** (Regressionsschutz gegen
  Rauschen im gewollten Normalfall).
- `ctxpkg.lock` fehlt → kein Fehler, kein Output.
- `ctxpkg.lock` wird **nicht** geschrieben (Dateizustand vor == nach).
- Konsistenz-Gate: die Spannen-Konstante im Binary == `version_req` in `lean-ctx-addon.toml`
  (Muster des fragment-consistency-Gates; Divergenz fällt in CI, nicht beim Nutzer).

**Global**

- `cargo nextest run` grün, zero clippy warnings.
- `pack_drift` bleibt rot (aus dem Vorpaket) bis zum Release-Schnitt — erwartet, kein Fehler.

## Umfang dieses Pakets: implementieren + committen, KEIN Publish

Dieses Paket endet beim Commit. **Kein `pack create`, kein `pack export`, kein `pack publish`,
kein Tag, kein Addon-Republish.** Der Release wird nach Abschluss separat entschieden und
gefahren.

| # | Schritt                                                                            | Linie  |
|---|------------------------------------------------------------------------------------|--------|
| 1 | Branch von `feat-lmd-v2`; P8 → P5 → P1 → P2 → P9; `cargo fmt`; `cargo nextest run` | —      |
| 2 | Commits (Code + Doc-Kommentare + dev-readme-Tabellenzeile)                         | Binary |

**Erwartet nach Abschluss:** `pack_drift` meldet in CI weiterhin, dass `content/skills/` nicht
zum letzten publizierten Pack passt. Das ist **kein Fehler**, sondern die Funktion des Gates —
es erinnert an den Schnitt, der später kommt. Der Zustand besteht seit dem Vorpaket
(`bace97a`) und wird von diesem Paket nicht verändert: es fasst `content/skills/**` nicht an.

Die Versionsnummern in `lean-ctx-addon.toml` und `content/skills.ctxpkg-hash` bleiben
**unangetastet** — sie werden erst beim tatsächlichen Release gezogen. Ob 0.2.1 dann die
richtige Nummer ist, entscheidet sich dort, nicht hier.

## Release-Choreografie (Referenz — NICHT Teil dieses Pakets)

Wenn beide Pakete fertig sind, wird **einmal** released. Die Reihenfolge ist erzwungen, nicht
Geschmack (übernommen aus der Vorspec):

1. Skills-Pack muss publiziert sein, **bevor** das Addon republished wird — der Resolver löst
   `version_req` depth-1 gegen den Registry-Index auf; ein unpublizierter Pack ist unsichtbar.
2. Der Tag muss den 5-Leg-Build ausgelöst haben, **bevor** `sync-manifest` die echten SHA-256
   in `[artifacts]` zurückschreibt (Bot-Commit auf `feat-lmd-v2`, nicht auf einen Tag).
3. Das Addon-Pack darf erst **nach** dem `sync-manifest`-Commit gebaut werden, sonst pinnt es
   Platzhalter-SHAs.

| # | Schritt                                                                                                            | Linie  |
|---|--------------------------------------------------------------------------------------------------------------------|--------|
| 1 | `pack create --version <v>` → `content/skills.ctxpkg-hash` aus `manifest.json` (`integrity.content_hash`) → commit | Pack   |
| 2 | `pack export --sign` → `pack publish --token ctxp_…` — **von Hand**, CI hat bewusst kein Token                     | Pack   |
| 3 | `lean-ctx-addon.toml`: Version + Artefakt-URLs; Tag `v<v>` → 5-Leg-Build → `sync-manifest`-Bot-Commit              | Binary |
| 4 | Addon-Pack (`kind=addon`) exportieren + publizieren                                                                | Binary |

Solange der Skills-Pack in `0.2.x` bleibt, ist `version_req = "^0.2"` **unangetastet**. Die
konkreten Nummern werden beim Release festgelegt, nicht hier.

**Release-Notes müssen dann tragen:** P1/P2 machen bisher stille Fehler laut. Eine Datei mit
doppelten `@phase`-Namen oder unbekannten Directive-Argumenten bricht nach dem Update, ohne
dass der Konsument etwas geändert hat. Das ist ein Bugfix, aber er ist sichtbar.

## Bewusst NICHT in diesem Paket

- **P7 — `Transport closed`.** Kein Repro, kein Abschlusskriterium (Entscheidung 10). Eigene
  Runde, sobald der Fehler greifbar ist.
- **Die vier stale Seeds im Dev-Repo geradeziehen.** Der Nutzer zieht sie selbst nach; dieses
  Paket liefert den Mechanismus, nicht das Aufräumen.
- **`docs/CONTRACT.md` existiert nicht.** `AGENTS.md` und `CLAUDE.md` verweisen beide darauf
  als „vendored addon contract" — ein baumelnder Verweis, genau die Sorte, die das Vorpaket bei
  den Stubs beseitigt hat. Gehört gefixt, aber nicht hier.
- **`lmd_render` in Pack-Content.** Wird erst durch P9 durchsetzbar; der Umzug ist eine eigene
  Entscheidung nach dem Release.
- **Der Release selbst.** Kein `pack create`/`publish`, kein Tag, kein Addon-Republish, keine
  gezogene Versionsnummer. Wird nach Abschluss separat entschieden und gefahren.

## Nachgelagert: Konsumenten

In `canfdchela` sind `.claude/skills/*/SKILL.md` lokal gefixt (Commit `8b0f4a8`), und
`.lean-ctx/lean-md/` ist dort **nicht** angepasst. `install_skill()` überschreibt die Stubs bei
jedem Install (`skill_install.rs:104`) — erst der Upstream-Fix ist dauerhaft. Bis zum Publish
bleibt der lokale Fix dort die Zwischenlösung; danach `addon update` fahren. Die Seeds heilt der MCP-Start dann selbst (
Entscheidung 4); der
`.new`-Fall tritt dort nicht auf, weil nichts angepasst wurde.
