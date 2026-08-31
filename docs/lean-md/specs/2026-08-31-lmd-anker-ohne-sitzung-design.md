# `@read` ohne Sitzung — Design für die Reparatur der Anker-Kette

**Datum:** 2026-08-31 · **Status:** entworfen, nicht implementiert
**Auslöser:** `Noospehre/docs/werkzeug/2026-08-25-der-anker-ohne-sitzung.md` (andere Session)
**Betroffene Repos:** `lean-md` (Pakete B, C) · `lean-ctx` (Paket A)

## 1. Ausgangslage

`@read`-Anker rendern in keinem Plan. Der Befund der Vorsession ist in seinem Kern
bestätigt, in vier Punkten aber falsch oder unvollständig. Dieses Dokument hält den
verifizierten Stand fest und legt die Reparatur in drei unabhängige Pakete.

### 1.1 Verifiziert

| Behauptung | Prüfung |
|---|---|
| `lean-ctx call ctx_read` → `error: -32603: session not available` | reproduziert, exit 2 |
| `rust/src/cli/call_cmd.rs:130` `oneshot_ctx` setzt `cache` + `bm25_cache`, kein `session` | bestätigt, auch gegen `origin/main` 09f3d7942 (3.10.1) |
| `ctx_read.rs:156` ist die Abbruchstelle | bestätigt |
| `git log -S'session' -- rust/src/cli/call_cmd.rs` → 0 Treffer | bestätigt |
| Betroffen: `ctx_fill`, `ctx_handoff`, `ctx_knowledge`, `ctx_multi_read`, `ctx_read`, `ctx_session`, `ctx_shell`, `ctx_workflow` | bestätigt (13 Fundstellen) |
| `lean-md/src/backend.rs:63-80` wählt `CliBackend` per Default | bestätigt |

### 1.2 Korrekturen am Befund

- **`@symbol` ist nicht betroffen.** `@symbol`/`@search` laufen über `ctx_search`,
  das keine Session verlangt; `lean-ctx call ctx_search` liefert Treffer. Die
  gemeinsame Wurzel existiert nicht.
- **Der Gateway-`@import`-Fehler liegt in lean-md, nicht in der Addon-Verdrahtung.**
  `mcp_load_source` (`src/bin/lean_md.rs:566`) setzt `jail_root` auf das
  *Elternverzeichnis der Plandatei*, während der CLI-Pfad auf *cwd* jailt. Ein
  projektrelatives `@import .lean-ctx/lean-md/…` liegt damit außerhalb.
- **Ein env-Stellknopf existiert.** `[gateway.servers.env]` in
  `~/.config/lean-ctx/config.toml` wird bereits mit `LEAN_MD_SKILLS_DIR` benutzt.
  Nur `cwd` fehlt im `GatewayServer`-Struct (`rust/src/core/mcp_catalog/config.rs:75-105`).
- **Der Informationsverlust ist größer als beschrieben.** „Nur die Einbettung fehlt"
  gilt allein für den CLI-Weg mit `--phase` (dort degradiert `@read` zu einem
  HTML-Kommentar). Im Whole-Doc-/Gateway-Pfad greift der Phasen-Executor:
  `PHASE_ABORTED "t1" at @read (line 11): BACKEND_REQUIRED: …` — der Rest der Phase
  **und alle `@on complete`-Sinks** fallen weg (empirisch reproduziert).
- **Reparaturweg 2 der Vorsession ist ein toter Schalter.** `Cargo.toml` hat
  `default = []`; `mcp` wird im Release nirgends aktiviert, `McpBackend` ist in der
  ausgelieferten Binary nicht einkompiliert. `LEAN_MD_BACKEND=mcp` kann nichts tun.
- **Die Addon-Randnotiz ist gegenstandslos.** lean-ctx 3.10.x meldet
  „Addon and plugin commands have been removed"; `addon info` existiert nicht mehr.

### 1.3 Neuer Befund — jede `@phase` tötet den Gateway-Render

`session_decision` (`src/phases.rs:230`) setzt beim Öffnen jeder Phase einen
`ctx_session`-Backendcall ab. Über den Gateway ist das ein rekursiver
`lean-ctx call` **in den laufenden lean-ctx-Server**:

    ctx_tools call lean-md::ctx_md_render {content: "@phase \"t1\"\n…"}
    → downstream tools/call failed: Transport closed

Reproduziert mit reinem `content` ohne jeden `@read`. Dasselbe Addon-Binary über
stdio direkt beantwortet denselben Aufruf normal. Pläne mit Phasen sind über den
Gateway also generell nicht renderbar — unabhängig von der Sitzungsfrage.

## 2. Entwurf

Drei Pakete, ohne Abhängigkeit untereinander. Keines nutzt `McpBackend`; der
CLI-Backend bleibt der einzige Weg (Entscheidung des Nutzers).

**Aufteilung in Pläne:** Paket A ist ein eigener Zyklus im `lean-ctx`-Repo
(Branch → PR → Merge → `cargo install`) und gehört nicht in denselben Plan wie
B und C, die beide im `lean-md`-Arbeitsbaum liegen. Also zwei Pläne: einer für A,
einer für B+C.

### 2.1 Paket A — die Wurzel (lean-ctx)

`oneshot_ctx` (`rust/src/cli/call_cmd.rs:130`) erhält das fehlende Feld, nach dem
Muster drei Zeilen darüber und dem bestehenden Aufruf in `server/post_dispatch.rs:91`:

```rust
session: Some(std::sync::Arc::new(tokio::sync::RwLock::new(
    crate::core::session::SessionState::load_latest_for_project_root(&project_root)
        .unwrap_or_default(),
))),
```

`ctx_read` braucht die Session an genau zwei Stellen: `task.description`
(Task-Fokus, `ctx_read.rs:168`) und `state.id` (Attribution, `:126`). Eine leere
Default-Session genügt vollständig; `load_latest_for_project_root` liefert
zusätzlich den Task-Fokus, wenn eine Sitzung zum Projekt existiert, und weist
breite/unsichere Roots von sich aus ab.

**Reichweite:** alle acht sitzungsabhängigen Tools über `lean-ctx call`.
**Nicht behoben:** der Gateway-cwd (Paket C) und der Rekursionstod (§1.3).

**Auslieferung.** `origin` = `dasTholo/lean-ctx` (push frei), `upstream` =
`yvgude/lean-ctx` (push gesperrt) → in `main` nur per PR aus dem Fork.
Branch `fix/oneshot-session` von `origin/main` ist angelegt. Schritte: Patch →
Regressionstest neben den bestehenden `call_cmd`-Tests → `cargo nextest run` →
push zu `origin` → PR gegen `upstream/main`. Lokal wirksam erst nach
`cargo install --path rust`: die PATH-Binary ist es, die `CliBackend` spawnt.

### 2.2 Paket B — lean-md wird störungsfest

**B1 — Fehlerklassen trennen.** `DirectiveBridge` bekommt eine Trait-Methode
`read_only() -> bool` (Default `false`, nach dem Muster des bestehenden
`accepts_pipe`). Ein `BridgeError::Backend` aus einer Bridge mit `read_only() ==
true` wird in `render::dispatch_result` zu `Ok(Ersatz)` statt `Err`; damit setzt
`render_with_phases` (`src/phases.rs:380-394`) kein `sc.aborted` mehr, und
Phasenrest wie `@on complete`-Sinks überleben einen Infrastrukturausfall.

`read_only() == true` bekommen genau: `read`, `search`, `symbol`, `find`, `graph`,
`impact`, `outline`, `repomap`, `architecture`, `smells`, `review`, `routes`,
`inspect`, `recall`, `list`. Alles andere bleibt beim Default — insbesondere
`edit`, `refactor`, `reformat`, `dispatch`, `handoff`, `remember`, `checkpoint`
und `query`: dort wäre ein stiller Weiterlauf falsch. Autorenfehler
(`MissingArg`, `Resolve`/Jail, `ShellDenied`) brechen unverändert ab, unabhängig
von `read_only`.

**B2 — Fallback über sitzungsfreie lean-ctx-Tools.** Kein lokaler Lesepfad in
lean-md: die Bytes bleiben server-seitig gejailt und redigiert (Spec §6). Der
`ReadBridge` weicht stattdessen auf ein Tool aus, das ohne Sitzung arbeitet:

| `@read mode=` | Ersatz | belegt |
|---|---|---|
| `signatures`, `map`, `auto` | `ctx_outline path=…` | verifiziert: Signaturen **mit Zeilennummern**, 1 400 → 296 tok |
| `full`, `lines:N-M`, `anchored`, `raw`, … | keiner → Notiz | kein sitzungsfreies Äquivalent gefunden (`ctx_compress`, `ctx_crush`, `ctx_git_read`, `ctx_summary` geprüft) |

Der Ersatz wird offen ausgewiesen, byte-stabil (#498), ohne Zeitstempel:

    <!-- lmd:@read fallback=ctx_outline (ctx_read ohne Sitzung) -->
    <Outline-Ausgabe>

Greift auch das nicht, rendert der Anker als sichtbarer Selbstlese-Auftrag statt
als HTML-Kommentar:

    > ⚠ @read src/seal.rs mode=full — Backend ohne Sitzung.
    >   Selbst lesen: ctx_read(path="src/seal.rs", mode="full")

`@symbol body` bleibt unangetastet — es läuft bereits über `ctx_search`.

**B3 — keine neue Direktiven-Syntax.** `@read … symbol=X` wird *nicht* eingeführt;
`@symbol body name=X` deckt den Fall ab (YAGNI).

### 2.3 Paket C — der Gateway-Zweig (lean-md)

- **C1 `phase` im `path`/`content`-Zweig auswerten.** `src/bin/lean_md.rs:795-820`
  liest `phase` heute nur im `skill`-Zweig; der else-Zweig ruft `do_render`, das
  `render_source_with_phase(.., None, ..)` festverdrahtet. Empirisch: ein Aufruf mit
  `phase: "t1"` rendert t1 **und** t2. Fix: `phase` durchreichen, `PhaseNotFound`
  als `-32602` melden.
- **C2 `jail_root` = Projektroot.** `mcp_load_source` (`:566`) sucht künftig vom
  Verzeichnis der Plandatei aufwärts nach `.lean-ctx/` bzw. `.git/` und nimmt den
  Fund als Jail-Wurzel; ohne Fund bleibt das Elternverzeichnis. Damit löst
  `@import .lean-ctx/lean-md/plan-recipes` auf, und CLI- und Gateway-Render laufen
  auf derselben Wurzel. **Der Jail wächst dadurch nach oben** — gehört als
  Entscheidung in Spec §7 vermerkt.

  **Entscheidung (umgesetzt).** Der Gateway-Jail wandert vom Plan-Verzeichnis auf den
  Projektroot (erster Vorfahre mit `.lean-ctx/` oder `.git/`). Er wächst damit nach
  oben; das ist der Preis dafür, daß CLI- und Gateway-Render dieselbe Wurzel sehen und
  `@import .lean-ctx/lean-md/…` überhaupt auflöst. Ohne Marker bleibt es beim
  Elternverzeichnis.
- **C3 Sinks im MCP-Modus abschalten (Auto-Erkennung).** Läuft die Binary als
  `lean-md mcp`, werden `ctx_session`/`ctx_knowledge`/`ctx_agent`-Sinks zu No-ops
  (`session_decision`, `session_set_task`, `session_add_finding`, `@on complete`).
  Kein Konfigurationsknopf, kein env-Schalter: die Rekursion
  `Gateway → lean-md → lean-ctx call ctx_session → Gateway` entsteht gar nicht erst.
  Bewusste Divergenz zum CLI-Pfad, wo die Sinks weiter feuern.

## 3. Tests

| Paket | Test |
|---|---|
| A | `lean-ctx call ctx_read --json '{"path":…,"mode":"signatures"}'` liefert Inhalt, exit 0 |
| B1 | Fake-Backend mit `NonZero` in einer Phase → kein `PHASE_ABORTED`, Text nach dem Anker und `@on complete` bleiben |
| B1 | `@edit` mit demselben Fake-Backend → weiterhin `PHASE_ABORTED` |
| B2 | `mode=signatures` + fehlschlagendes `ctx_read` → genau ein `ctx_outline`-Call, Fallback-Marker im Output |
| B2 | `mode=full` + fehlschlagendes `ctx_read` → Notiz, kein zweiter Backendcall |
| B2 | Byte-Stabilität: zwei Renders derselben Quelle sind identisch (#498) |
| C1 | MCP `tools/call` mit `path` + `phase` → nur die benannte Phase |
| C2 | Fixture: Plan in `<root>/docs/plans/`, Seed in `<root>/.lean-ctx/lean-md/` → `@import` löst auf |
| C3 | Sink-zählendes Fake-Backend im `mcp`-Modus → 0 Calls; im CLI-Modus unverändert |

Alle Tests über `cargo nextest run` (nie `cargo test`).

## 4. Risiken

- **A:** `load_latest_for_project_root` scannt bei *jedem* Anker-Subprozeß das
  Sitzungsverzeichnis. Bei vielen Ankern messbar; Rückfallebene ist
  `SessionState::default()` ohne Scan (kostet den Task-Fokus).
- **B2:** `ctx_outline` antwortet strukturell anders als `ctx_read mode=signatures`.
  Der Fallback-Marker macht das sichtbar; Golden-Tests müssen beide Formen kennen.
- **C2:** größerer Jail beim Gateway-Render. Bewusst, aber sicherheitsrelevant.
- **C3:** Sinks verschwinden im Gateway-Pfad. Alternative (Puffern und im Ergebnis
  anhängen) wurde verworfen — mehr Aufwand, kein Gewinn für den Anker-Fall.

## 5. Offene Punkte

- Das `mcp`-Feature ist im Release nicht kompiliert und damit toter Code inkl.
  `LEAN_MD_BACKEND`/`LEAN_MD_MCP_ENDPOINT`. Entweder im README als
  Quell-Build-Option dokumentieren oder entfernen — nicht Teil dieser Pakete.
- `GatewayServer` (lean-ctx) hat kein `cwd`-Feld. Nach C2 wird es nicht mehr
  gebraucht; ein späterer Bedarf wäre ein eigener Upstream-Vorschlag.
