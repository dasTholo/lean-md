# Design — Release 0.2.1: Changelog, Docs-Refresh, eindeutiger Addon-Install

Datum: 2026-07-18 · Branch: `feat-lmd-v2` · **Kein PR gegen main.**

## Problem

Seit dem einzigen Tag `v0.2.0` haben sich **beide** Release-Linien bewegt:

- **Binary** (`src/**`, `content/core`, `content/templates`, `Cargo.toml`): Seeds/Lock-
  Provenienz, Seed-Refresh + Ack-Kanal, `version_gate`, deklaratives `arg_schema`,
  `@phase`-Fixes, `check`→exit 1, MSRV 1.97, dep-bumps (~3350 Zeilen).
- **Skills-Pack** (`content/skills/**`): Stubs geslimt/single-sourced, `lmd-rendering-skills`
  neu, TDD-body, Companion-Edits.

Daraus folgen drei Lücken:

1. **Kein CHANGELOG** — die Änderungen sind nirgends beschrieben.
2. **README.md + INSTALL.md referenzieren durchgehend `0.2.0`** und enthalten **keine**
   Endnutzer-Update-Anleitung (`addon update`).
3. **Eindeutigkeits-Lücke:** Eine neue Session macht `lean-ctx addon add dastholo/lean-md`
   und liest den `kind=addon`-Pack aus der Registry. Dessen `lean-ctx-addon.toml` pinnt die
   `[artifacts.*]` (url + sha256) — aktuell auf **v0.2.0**. `sync-manifest` (CI) patcht nur
   die **im Repo eingecheckte** Manifest-Datei, **nicht** den published Pack. Solange der
   Addon-Pack nicht neu published wird, zieht jede neue Session weiter das v0.2.0-Binary.
   Die `dev-readme` dokumentiert nur den skill-only-Fall („addon.toml untouched → kein
   Republish") und schweigt zum Binary+Pack-Fall.

## Ziel

`0.2.1` sauber ausliefern (beide Linien), Docs aktualisieren, die Republish-Lücke
**beheben und dokumentieren**. Danach findet eine neue Session eindeutig `0.2.1`.

## Ausführungsgrenze

Der Agent **bereitet vor und committet** auf `feat-lmd-v2`. Tag-Push (triggert Release-CI)
und die Token-gebundenen Publishes (Skills-Pack, Addon-Pack) führt **der Nutzer** per
Runbook aus. Kein `ctxp_`-Token im Agenten-Kontext.

## Änderungen

### A · `CHANGELOG.md` (neu)

Keep-a-Changelog-Format, **eine Datei, zwei entkoppelte Versionslinien**:

- `## [binary 0.2.1]` — Seeds/Lock (`content/seeds.sha256`, `lean-md.lock` im
  sha256sum-Format, History-Heal), Seed-Refresh beim Serverstart + Ack-Kanal,
  `version_gate` (Pack-Spanne vs. `ctxpkg.lock`, case-insensitiv), deklaratives
  `arg_schema` (check + Bridge, eine Quelle), `@phase`-Duplikat/Fence-Härtung,
  `check` liefert exit 1, `.ext`-Fragment-Generalisierung, MSRV 1.96→1.97, sha2
  0.10→0.11, regex 1.12→1.13.
- `## [skills-pack 0.2.1]` — 8 Delegation-Stubs geslimt + Render-Handle single-sourced,
  `lmd-rendering-skills` Bootstrap-Skill neu (mit jedem `skill install` mitgezogen),
  TDD-body-Update, Companion-Edits (bulletproofing, testing/methodology).

Beschreibungen aus den Commit-Betreffen destilliert, nicht erfunden.

### B · `README.md` + `INSTALL.md`

- Alle `0.2.0`-Referenzen → `0.2.1` (Pack-Version, `<version>`-Beispiele, Doc-Links).
- **NEU — Endnutzer-Update-Abschnitt** (fehlt heute): `lean-ctx addon update lean-md`
  zieht das neue side-by-side-Binary **und** den neuen Skills-Pack (health-gated,
  auto-prune), danach MCP-Client neu starten. Das ist die direkte Antwort auf „wie
  bekommt der Endnutzer die neueste Version".

### C · `docs/dev-readme.md`

Neuer Abschnitt **„Binary + Pack gleichzeitig"**: die bestehende Zwei-Regime-Tabelle
deckt nur skill-only ab. Ergänzung — ändert sich das Binary, ändert sich
`lean-ctx-addon.toml` (version + artifacts), also **muss** der `kind=addon`-Pack via
`lean-ctx addon publish --namespace dastholo` neu published werden. Harte Sequenz:
**Tag → CI-Build → `sync-manifest` → dann erst** `addon publish` (braucht die echten
v0.2.1-sha256). Die bestehende Aussage „addon.toml untouched → kein Republish" bleibt
korrekt für skill-only, wird aber explizit auf diesen Fall eingegrenzt.

### D · Version-Bumps (Repo)

- `Cargo.toml` `version = "0.2.1"` (+ `Cargo.lock`).
- `lean-ctx-addon.toml` `addon.version = "0.2.1"`. **`[artifacts.*]` sha256/url NICHT
  anfassen** — die schreibt `sync-manifest` nach dem Tag.
- Skills-Pack auf `0.2.1`: `content/skills.sha256` + `content/skills.ctxpkg-hash`
  neu blessen (`LEAN_MD_BLESS=1 cargo nextest run --test pack_drift`).
- `version_req = "^0.2"` bleibt (deckt 0.2.x) — **kein** Addon-Manifest-Bump nötig.

## Verifikation (vor Commit)

- `cargo nextest run` grün — besonders `pack_drift`, `seed_history`, `determinism`,
  `version_gate`.
- `cargo fmt` je geänderter Datei vor `git add`.
- „Ohne target-Binary": weder `addon publish` noch Skills-`pack create/export/publish`
  brauchen ein kompiliertes `lean-md` in `target/` — sie laufen über `lean-ctx`; das
  lean-md-Binary kommt als GitHub-Release-Asset (url + sha256). Im Runbook als
  Expected-Check verankert.

## F · Runbook (Datei für den Nutzer, nicht ausgeführt)

Geordnete Befehlsfolge mit „Expected:"-Checks:

1. Commit der Vorbereitung auf `feat-lmd-v2`.
2. `git tag v0.2.1 && git push --tags` → Release-CI (5-leg-Build) → `sync-manifest`
   patcht `[artifacts.*]` sha256 + committet auf `feat-lmd-v2`.
3. `git pull` (den sync-manifest-Commit holen).
4. Skills-Pack: `pack create --version 0.2.1` → `content/skills.ctxpkg-hash` prüfen →
   `pack export --sign` → `pack publish --token ctxp_…`.
5. Addon-Pack: `lean-ctx addon publish --namespace dastholo` (liest das gepatchte
   Manifest, published den `kind=addon`-Pack neu).
6. Smoke: in einem sauberen Kontext `lean-ctx addon update lean-md` → zieht 0.2.1.

Jeder Schritt token-/CI-gebunden bleibt beim Nutzer.

## Non-Goals

- Kein `0.3.x`-Sprung, also **kein** `version_req`-Bump im Addon-Manifest.
- Keine Änderung an `[artifacts.*]` von Hand (CI-Domäne).
- Kein PR gegen `main`.
- Kein unrelated Refactoring.
