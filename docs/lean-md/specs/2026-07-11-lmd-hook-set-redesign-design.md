# Hook-Set-Redesign — „Soft-Reads, strikte Edits" — Design

**Datum:** 2026-07-11
**Branch:** feat-lmd-v2
**Status:** Design (approved) → bereit für Plan
**Scope:** umgebungs-lokal (`~/.claude/**`, `~/.config/lean-ctx/config.toml`) — **außerhalb** des lean-md-Repos, nicht git-versioniert, nicht test-abgedeckt.

## Problem

Das Claude-Code-Hook-Set (`~/.claude/settings.json` + `~/.claude/hooks/*.py`) stammt
aus der superpowers-Ära und wurde vor den nativen lean-ctx-Read-Features gebaut. Es
enthält tote Einträge, einen inerten Hook und eine ungenutzte Reibungsquelle:
native-Read-Denies, obwohl lean-ctx Reads inzwischen nativ transparent umleiten kann.
Auslöser: der Nutzer hat `read_dedup = "on"` konfiguriert und fragt, welche Hooks jetzt
überflüssig sind.

## Ausgangsbefund (Ist-Zustand, Claude Code Guard-Host)

Config: `read_dedup = "on"`, `read_redirect` **unset** → Default `auto`
(→ Read-Redirect auf Guard-Hosts **aus**, Schutz der native-Edit read-before-write-Guard, #637).

| # | Hook | Event/Matcher | Befund |
|---|------|---------------|--------|
| 1 | `skill-plan-injector.py` | Post/Skill | **Datei fehlt** → toter Eintrag |
| 2 | `lean-ctx hook observe` | Post/`.*` | native Telemetrie — behalten |
| 3 | `lean-ctx hook read-dedup` | Post/Read | = `read_dedup=on` — behalten |
| 4 | `lean-ctx-policy-guard.py` | Pre/Bash+Edit+ctx_* | Allowlist-/Config-Schutz — behalten |
| 5 | `bash-enforce-ctx-shell.py` + `hook rewrite` | Pre/Bash | hard-deny native Bash — behalten |
| 6 | `edit-tool-discipline.py` | Pre/Edit+Write | hard-deny native Edit — behalten |
| 7 | `plan-discipline.py` | Pre/Write+TaskCreate | **stale Scope** `docs/superpowers/` → feuert in `docs/lean-md/` nie |
| 8 | `preToolUse.mjs` (markdownai) | Pre/Read | fremdes Tool — nicht anfassen |
| 9 | `read-search-discipline.py` | Pre/Read+Grep | hard-deny native Read/Grep in-jail |
| 10 | `lean-ctx hook redirect` | Pre/Read+Glob | native soft redirect — **inert** unter `read_redirect=auto` am Guard-Host |

Schlüssel-Fakten (aus `lean-ctx/rust/src/core/config/read_redirect.rs` +
`read_dedup.rs`):
- `read_redirect=auto` schaltet den **Read**-Redirect auf Guard-Hosts (Claude Code /
  CodeBuddy) ab; der **Grep/Glob**-Redirect ist guard-safe und bleibt **immer aktiv**.
- ∴ `read-search-discipline.py`s Grep-Deny ist bereits heute redundant (nativer
  Grep-Redirect deckt es); nur sein **Read**-Deny ist unter `auto` tragend.
- Auf einem Guard-Host sind die strikten Read-Denies die **einzige** Quelle für
  Read-Kompression, solange der native Read-Redirect aus ist.

## Ziel

Reibung auf dem dominanten Read-Kanal entfernen, ohne die Edit-Disziplin (Basis der
laufenden ctx_patch-Initiative) aufzugeben — plus toten Ballast beseitigen.

## Entscheidung: Regime B — „Soft-Reads, strikte Edits"

Der Hebel: **native Edit wird ohnehin per `edit-tool-discipline.py` verweigert** → die
read-before-write-Guard ist gegenstandslos → `read_redirect="on"` ist **gefahrlos**.
Damit übernimmt der native Redirect (#10) die Read-Kompression transparent und der
Python-Read-Deny (#9) wird vollständig überflüssig.

### Änderungen

**A. Config (`~/.config/lean-ctx/config.toml`):**
- `read_redirect = "on"` **hinzufügen** (aktuell unset→auto). Aktiviert den nativen
  Read-Redirect auch auf Claude Code → native Reads werden transparent komprimiert.
- `read_dedup = "on"` **bleibt** (dedupt Re-Reads unabhängig davon).

**B. Hooks entfernen (`~/.claude/settings.json` + Datei):**
- **#1 `skill-plan-injector.py`** — PostToolUse/Skill-Eintrag entfernen (Datei fehlt bereits).
- **#7 `plan-discipline.py`** — PreToolUse/Write+TaskCreate-Eintrag **und** Datei
  entfernen. Nur WARN-Modus, stale Scope, Drift-Patterns überlappen mit #5/#6.
- **#9 `read-search-discipline.py`** — PreToolUse/Read+Grep-Eintrag **und** Datei
  entfernen. Durch `read_redirect=on` (Read) + guard-safen Grep/Glob-Redirect ersetzt.

**C. Hooks behalten (unverändert):**
- #2 `hook observe`, #3 `hook read-dedup`, #4 `lean-ctx-policy-guard.py`,
  #5 `bash-enforce-ctx-shell.py` (+ `hook rewrite`), #6 `edit-tool-discipline.py`,
  #10 `lean-ctx hook redirect` (wird durch `read_redirect=on` erst voll wirksam).
- #8 markdownai-Node-Hooks — fremdes Tool, nicht anfassen.

### Kopplungs-Invariante (kritisch — in der Doku prominent)

`read_redirect="on"` ist **nur** sicher, solange `edit-tool-discipline.py` native Edit
verweigert (sonst bricht der Read-Redirect die native-Edit read-before-write-Guard,
#637). Config und Hooks leben beide global unter `~/` und reisen zusammen. Wer die
Edit-Disziplin je entfernt, MUSS `read_redirect` auf `auto` zurücknehmen. Dies wird als
Kommentar in `config.toml` neben `read_redirect` verankert.

## Nicht-Ziele

- Kein Anfassen der markdownai-Hooks (#8) — fremdes Tool.
- Keine Änderung an den lean-ctx-nativen Hook-Subcommands (`observe`/`rewrite`/
  `redirect`/`read-dedup`) — die kommen aus dem lean-ctx-Binary, nicht aus diesem Setup.
- Keine Repo-Änderung — dieser Redesign berührt ausschließlich `~/.claude/**` +
  `~/.config/lean-ctx/config.toml`. Der parallele ctx_patch-Plan (Task 4) ändert
  `edit-tool-discipline.py` + `lean-ctx-policy-guard.py` inhaltlich; dieser Redesign ist
  dazu orthogonal (entfernt andere Hooks, ändert Config).

## Verifikation (manuell — kein Repo-Test)

1. `python3 -c "import json; json.load(open('/home/tholo/.claude/settings.json'))"` → valides JSON; die drei Einträge #1/#7/#9 fehlen.
2. Dateien `plan-discipline.py`, `read-search-discipline.py` gelöscht; `skill-plan-injector.py` war nie vorhanden.
3. Neue Claude-Code-Session:
   - native Read auf eine in-jail-Datei (z. B. `src/lib.rs`) → **läuft transparent komprimiert durch**, kein `[read-search-discipline]`-Deny mehr.
   - native Edit auf eine `.rs`-Datei → weiterhin denied (`edit-tool-discipline`) → ctx_patch/ctx_edit/ctx_refactor.
   - native Bash `grep …` → weiterhin denied (`bash-enforce-ctx-shell`).
   - `ctx_patch` auf `config.toml` mit `shell_allowlist`-Zuweisung → weiterhin denied (`lean-ctx-policy-guard`, nach ctx_patch-Plan-Task-4).
4. Re-Read derselben Datei → ~13-Token-Stub (`read_dedup=on` weiter aktiv).

## Risiken

- **Kopplung** (s. o.) — mitigiert durch config.toml-Kommentar.
- **Globale Config wirkt auf alle Hosts:** `read_redirect=on` ist auf Nicht-Guard-Hosts
  (Cursor/Zed) ein No-op (dort war auto bereits on). Kein Regressions-Risiko dort.
- **Out-of-repo, kein Test-Gate:** rein manueller Smoke-Test; als eigener Plan-Task mit
  expliziten „Expected:"-Checks geführt, nicht durch `cargo nextest` abgedeckt.
