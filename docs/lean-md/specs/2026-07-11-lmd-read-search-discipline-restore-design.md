# read-search-discipline — Wiederherstellung + Härtung — Design

Status: approved (2026-07-11). Env-lokal (`~/.claude/`, `~/.config/lean-ctx/`), kein Repo-Code —
**nur dieses Spec-Dokument** wird committet.

## Problem

Das Hook-Set-Redesign (Regime B, Spec `2026-07-11-lmd-hook-set-redesign-design.md`) hat
`read-search-discipline.py` entfernt — unter der Annahme, `read_redirect="on"` + der Rust-native
`lean-ctx hook redirect` würden nativen Read transparent umleiten und so dieselbe Disziplin
liefern. In einer frischen Session (Hooks neu geladen) wurde beobachtet: **nativer Read wird roh
genutzt**, nicht auf `ctx_read` gezwungen.

Ursache (aus dem Docstring des Original-Hooks, aus der verwaisten `.pyc` geborgen): `lean-ctx hook
redirect` liefert für nativen Read/Grep **immer `permissionDecision: "allow"`** — es proxied still
durch lean-ctx oder fällt auf ein blankes `allow` zurück. Das ist ein **weicher Nudge, kein hartes
Deny**. Die harte Read-Disziplin kam allein von `read-search-discipline.py`. `read_redirect` und der
Hook sind **komplementär** (Proxy/Compress vs. hartes Deny), nicht redundant — die T1-Gotcha
(„read_redirect ersetzt read-search-discipline") war falsch.

## Ziel

`read-search-discipline.py` wiederherstellen **und** in drei Punkten härten, damit weniger nativer
Read durchschlüpft.

## Architektur

PreToolUse-Hook neben `edit-tool-discipline.py` / `bash-enforce-ctx-shell.py`. **Jail-aware**: hartes
Deny für nativen Read/Grep/List **nur wenn `ctx_read`/`ctx_search`/`ctx_tree` den Pfad bedienen
können** (Ziel liegt in der PathJail: project root ∪ allow_paths ∪ extra_roots ∪ `read_only_roots`).
Out-of-jail-Pfade (`ctx_read` → `-32602 path escapes project root`) und echte Binär-/Opaque-Formate
bleiben **nativ erlaubt** — sonst würde der Agent aus Dateien ausgesperrt, die lean-ctx selbst nicht
serviert. Der Deny-Reason zeigt auf das passende `ctx_*`-Tool. Exit 0 immer (Entscheidung in stdout,
nie den Workflow brechen).

Wiederherzustellendes Original (bytegenau aus `.pyc` rekonstruierbar):
`READ_TOOLS`/`GREP_TOOLS`/`LIST_TOOLS`, `PATH_KEYS`, `PASSTHROUGH_*`, `REPLACEMENTS`, plus
`classify`/`extract_path`/`jail_roots`/`in_jail`/`is_passthrough`/`deny_reason`/`emit_deny`/`main`
und die Config-Parser (`_roots_from_config`/`_project_root`).

## Änderungen

**1 — Restore.** `~/.claude/hooks/read-search-discipline.py` + `~/.claude/hooks/tests/
test_read_search_discipline.py` aus der `.pyc` rekonstruieren. settings.json-Gruppe wieder einhängen:
`PreToolUse` matcher `Read|read|ReadFile|read_file|View|view|Grep|grep|Search|search|ListFiles|
list_files|ListDirectory|list_directory|Glob|glob` → `python3 …/read-search-discipline.py` (vor
`hook redirect` derselben Event-Klasse).

**2 — Härtung Passthrough.** `lock` und `svg` aus `PASSTHROUGH_EXTENSIONS` entfernen: beide sind Text,
`ctx_read`-fähig und oft groß (Cargo.lock/package-lock sollen komprimiert gelesen werden). Echte
Binärformate (`bin eot gif gz ico jpeg jpg mov mp4 pdf png ttf wasm webp woff woff2 zip`) bleiben.
`PASSTHROUGH_SUBSTRINGS` (`node_modules`, `/.git/`) bleiben.

**3 — Härtung Matcher/Classify.** `Glob` steht im settings.json-Matcher, wird aber von `classify()`
nicht erkannt (nicht in `LIST_TOOLS`) → rutscht als `allow` durch. Fix: `glob` in die List-Klasse
aufnehmen (Deny-Reason verweist auf `ctx_tree`/`ctx_glob`). `NotebookRead`/`notebookread` in die
Read-Klasse aufnehmen (`PATH_KEYS` trägt `notebook_path` bereits). settings.json-Matcher und
Hook-interne Sets konsistent halten.
Pfad-Extraktion beachten: `Glob` trägt seinen Pfad im `pattern`-Key (optional zusätzlich `path`),
nicht in den bisherigen `PATH_KEYS` — `extract_path()`/`classify()` müssen den Glob-Basis-Pfad
auflösen (Verzeichnisanteil des Patterns bzw. `path`), sonst bleibt `in_jail` unbestimmt und der
Deny fällt fail-open auf `allow` zurück (Härtung liefe ins Leere).

**4 — Config-Kopplung.** `read_redirect="on"` bleibt (komplementär). `read_only_roots` **unverändert**
(`~/.cargo/registry`, `~/.rustup/toolchains`, superpowers-cache) — `~/.config/lean-ctx` bzw. `~/.claude`
werden **nicht** aufgenommen: Config-Inspektion läuft über `lean-ctx config show`, nicht `ctx_read`.
Die falsche T1-Gotcha in `ctx_knowledge` wird korrigiert (read_redirect ≠ Ersatz; die zwei sind
komplementär, `read-search-discipline` ist die harte Disziplin).

## Global Constraints / Non-Goals

- Kein Repo-Source-Change; nur dieses Spec-Dokument wird committet.
- `lean-ctx hook redirect` und `hook read-dedup` unangetastet.
- Ausführungs-Mechanismus (umgebungsbedingt, wie im Hook-Set-Redesign): out-of-jail Edits/Deletes
  laufen über eine Python-Skriptdatei (`python3 <file>`), da `ctx_read`/`ctx_patch` gejailt sind,
  native `Edit`/`MultiEdit` denied und `ctx_shell` `cat`/`rm`/`python3 -c` sperrt. Native `Write`
  legt neue Skriptdateien an.
- Verwaiste `.pyc` (`plan-discipline`, `skill-plan-injector`) im `__pycache__` optional entfernen.

## Testing / Verify

- pytest: `test_read_search_discipline.py` wiederhergestellt und grün (deny in-jail, allow out-of-jail,
  passthrough Binärformate, `Cargo.lock` jetzt deny, `Glob` jetzt deny).
- Fresh-Session-Smoke: nativer in-jail Read (`src/lib.rs`) → **denied** mit `ctx_read`-Hinweis;
  `Cargo.lock` → **denied** (nicht mehr passthrough); out-of-jail Read → **erlaubt**; native Edit `.rs`
  → weiterhin denied (edit-tool-discipline); native Bash `grep` → weiterhin denied (bash-enforce).
