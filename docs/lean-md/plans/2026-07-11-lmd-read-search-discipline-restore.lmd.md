@lean-md
consumer: ai
crp: compact

@import .lean-ctx/lean-md/plan-recipes /

# read-search-discipline — Wiederherstellung + Härtung — Implementation Plan

Spec: `docs/lean-md/specs/2026-07-11-lmd-read-search-discipline-restore-design.md` (approved).

## Goal

Die harte Read-Disziplin wiederherstellen, die das Hook-Set-Redesign versehentlich entfernte:
`read-search-discipline.py` (PreToolUse-Deny für nativen Read/Grep/List auf in-jail-Pfaden)
bytegenau rekonstruieren **und** in drei Punkten härten (`lock`/`svg` kein Passthrough mehr;
`Glob`→List-Klasse inkl. `pattern`-Pfad; `NotebookRead`→Read-Klasse), dann die settings.json-Gruppe
wieder einhängen.

## Architecture

Rein **umgebungs-lokaler** Change (`~/.claude/`), außerhalb des lean-md-Repos, nicht git-versioniert.
Zwei neue Hook-Dateien + ein settings.json-Edit; **einziger git-Commit** ist dieses Plan-Dokument.

- `~/.claude/hooks/read-search-discipline.py` — der Hook (NEUE Datei → native `Write`).
- `~/.claude/hooks/tests/test_read_search_discipline.py` — pytest (NEUE Datei → native `Write`).
- `~/.claude/settings.json` — eine PreToolUse-Gruppe einfügen (EXISTIERT → Python-Skript-Edit).
- `docs/lean-md/plans/2026-07-11-lmd-read-search-discipline-restore.lmd.md` — dieses Dokument.

**Differential-Orakel (Kern der Rekonstruktion):** die verwaiste
`~/.claude/hooks/__pycache__/read-search-discipline.cpython-314.pyc` ist noch da und lädt als Modul
(`marshal.loads(pyc.read_bytes()[16:])` → `exec`). Sie ist das **Verhaltensorakel**: der rekonstruierte
Hook muss für JEDEN ungehärteten `(tool_name, tool_input)`-Fall byte-identische `emit_deny`-Ausgabe
liefern. Nur die vier bewussten Härtungs-Divergenzen weichen ab (separat asserted).

**Mechanik-Vorbild:** `~/.claude/hooks/edit-tool-discipline.py` (via `ctx_read` lesbar, da `~/.claude`
jetzt in `extra_roots`) zeigt das stdin-JSON→classify→`deny()`→`hookSpecificOutput`-Protokoll +
`exit 0 always`. `read-search-discipline` ist dessen read+search-Gegenstück mit zusätzlicher
Jail-Awareness.

## Global Constraints

- **Kein Repo-Source-Change**; nur dieses Plan-Dokument wird committet. Hook-/Test-Dateien +
  settings.json sind env-lokal, **kein** git-Commit, **kein** cargo-Gate.
- **Jail-Awareness ist das kritische Design (nicht wegoptimieren):** Deny NUR wenn der Zielpfad
  in-jail liegt (`ctx_read`/`ctx_search`/`ctx_tree` können ihn bedienen). Out-of-jail (`-32602 path
  escapes project root`) und echte Binär-/Opaque-Formate bleiben **nativ erlaubt** — sonst wird der
  Agent aus Dateien ausgesperrt, die lean-ctx nicht serviert. `exit 0` immer.
- **Byte-Genauigkeit via Orakel:** Task 1 ist erst grün, wenn der differential-Test gegen die `.pyc`
  für alle ungehärteten Fälle 0 Abweichungen zeigt und die 4 Härtungs-Assertions erfüllt sind.
- **Nebeneffekt `extra_roots`:** seit `~/.claude` in der Jail liegt, denied der Hook nativen Read auch
  auf `~/.claude/hooks/*.py` — beabsichtigt (Hooks liest man dann via `ctx_read`).
- **Ausführungs-Mechanismus (umgebungsbedingt):** NEUE Dateien → native `Write` (erlaubt).
  EXISTIERENDE out-of-jail Dateien (settings.json) → Python-Skriptdatei via `python3 <file>`
  (`ctx_shell` sperrt `python3 -c`/`cat`/`rm`; native `Edit`/`Write`-auf-existierend denied).
- **Fresh-Session-Prerequisite:** Hooks laden bei Session-Start → die Wirkung (nativer in-jail Read
  denied) zeigt sich erst in einer **neuen** Session. Task 3 ist eine Handoff-Checkliste, in dieser
  Session nicht ausführbar.

@phase "task-1"
## Task 1: `read-search-discipline.py` rekonstruieren + härten (TDD gegen `.pyc`-Orakel)

@call recall_context("read-search-discipline Restore Härtung hook redirect soft-proxy")

**Fläche:** `~/.claude/hooks/read-search-discipline.py` (NEU, native `Write`) +
`~/.claude/hooks/tests/test_read_search_discipline.py` (NEU, native `Write`). **Kein** git-Commit.

### Schritt 1 — Geborgene Konstanten (verbatim, MIT Härtungen appliziert)

Diese Werte stammen aus dem Orakel; die **fett** markierten Zeilen sind die drei Härtungen:

    READ_TOOLS  = frozenset({"read", "read_file", "readfile", "view", "notebookread"})   # + notebookread (Härtung)
    GREP_TOOLS  = frozenset({"grep", "ripgrep", "search", "search_text", "searchtext"})
    LIST_TOOLS  = frozenset({"list_directory", "list_files", "listdirectory", "listfiles", "ls", "glob"})  # + glob (Härtung)
    PATH_KEYS   = ("file_path", "filePath", "path", "notebook_path", "notebookPath", "pattern")  # + pattern (Härtung)
    PASSTHROUGH_EXTENSIONS = frozenset({
        "bin", "eot", "gif", "gz", "ico", "jpeg", "jpg", "mov", "mp4",
        "pdf", "png", "ttf", "wasm", "webp", "woff", "woff2", "zip",
    })   # − "lock", − "svg" (Härtung: beide text + ctx_read-fähig)
    PASSTHROUGH_SUBSTRINGS = ("node_modules", "/.git/")
    REPLACEMENTS = {
        "read": ("mcp__lean-ctx__ctx_read",
                 'mcp__lean-ctx__ctx_read(path="{path}")  (cached, compressed, 10 read modes; re-reads ~13 tokens)'),
        "grep": ("mcp__lean-ctx__ctx_search",
                 'mcp__lean-ctx__ctx_search(pattern="...", path="{path}")  (compact, .gitignore-aware results)'),
        "list": ("mcp__lean-ctx__ctx_tree",
                 'mcp__lean-ctx__ctx_tree(path="{path}", depth=N)  (compact dir map with counts)'),
    }

**`pattern`-Sonderfall (Härtung Glob):** ein reines Glob-`pattern` (`"src/**/*.rs"`) ist kein Pfad —
`extract_path` muss dessen **Verzeichnis-Präfix bis zum ersten Glob-Metazeichen** (`* ? [`) auflösen
(`"src/**/*.rs"` → `"src"`, `"*.rs"` → `""` → dann kein Deny, korrekt: Repo-root-relativ nicht
auflösbar). Liegt zusätzlich ein `path`-Key vor, hat dieser Vorrang (Orakel-Verhalten:
`{"pattern":"*.rs","path":"/root"}` → `/root`).

### Schritt 2 — Funktions-Vertrag (aus dem Orakel; Signaturen sind bindend)

Baue die Funktionen nach `edit-tool-discipline.py`-Protokoll plus Jail-Logik:

- `classify(tool_name) -> "read"|"grep"|"list"|None` — lowercase-normalisierter Lookup in den drei Sets.
- `extract_path(tool_input) -> str` — erster nicht-leerer `PATH_KEYS`-Wert; für `pattern` das
  Glob-Präfix (Schritt 1). Leer → kein Deny.
- `_project_root() -> Path`, `_roots_from_config() -> list[Path]`, `jail_roots() -> list[Path]` —
  parst `~/.config/lean-ctx/config.toml`: die drei Array-Keys `allow_paths`/`extra_roots`/`read_only_roots`
  via `_ARRAY_KEY_RE` (`re.compile(r"(?m)^\s*<key>\s*=\s*\[(?P<body>.*?)\]", re.M)`) + `_STRING_RE`
  (`re.compile(r'"([^"]*)"')`), plus Projekt-Root. `jail_roots` gibt die Vereinigung.
- `is_under(child, parent) -> bool`, `in_jail(path, roots) -> bool` — Pfad-Präfix-Test.
- `is_passthrough(path) -> bool` — Extension in `PASSTHROUGH_EXTENSIONS` oder Substring in
  `PASSTHROUGH_SUBSTRINGS`.
- `deny_reason(family, path) -> str` — baut die Message aus `REPLACEMENTS[family]`. **Exakte Vorlage**
  (family gross: Read/Grep/List; tool/usage aus REPLACEMENTS):

      [read-search-discipline] native {Family} on '{path}' is denied — the path is inside the lean-ctx
      jail (project / allow_paths / extra_roots / read_only_roots) and is readable through {tool}.

      Use:
        {usage}

      Why: native {Family} bypasses lean-ctx caching + compression and the Live-Observatory metrics.
      Paths OUTSIDE the jail (which {tool} cannot serve) are NOT denied — for those native is allowed.
      To make a dependency tree like ~/.cargo/registry deny-able, add it to read_only_roots in
      config.toml so {tool} can read it.

      Note: {tool} may be DEFERRED in a subagent ("not available" / InputValidationError). Load it FIRST:
        ToolSearch(query="select:{tool}")
      Using a native read/search before invoking ToolSearch is a tool-discipline violation.

- `emit_deny(family, path)` / `main()` — stdin-JSON (`tool_name`/`toolName`, `tool_input`/`toolInput`)
  → `classify`; wenn `None` → `return 0`. `extract_path`; leer → `return 0`. Wenn `is_passthrough` **oder
  nicht** `in_jail` → `return 0` (allow). Sonst `deny()` mit `hookSpecificOutput`
  (`hookEventName=PreToolUse`, `permissionDecision=deny`, `permissionDecisionReason=deny_reason(...)`).
  `exit 0` immer, alle Exceptions → `return 0`.

### Schritt 3 — TDD: differential-Test gegen das Orakel (RED zuerst)

Schreibe `tests/test_read_search_discipline.py` (native `Write`). Der Test lädt die `.pyc` als Orakel,
ruft für eine Fall-Matrix beide (Hook via subprocess + Orakel-`main` in-proc mit `sys.stdin` gepatcht)
und vergleicht die JSON-Ausgabe byte-genau — außer für die 4 Härtungsfälle:

    import json, subprocess, sys, os, marshal, pathlib, io, contextlib
    REPO = str(pathlib.Path.home() / "Scripts/lean-md")   # project root — anchors the jail
    HOOK = pathlib.Path.home() / ".claude/hooks/read-search-discipline.py"
    PYC  = pathlib.Path.home() / ".claude/hooks/__pycache__/read-search-discipline.cpython-314.pyc"
    IN_JAIL = REPO + "/src/lib.rs"

    # Determinism: hook (subprocess) and oracle (in-proc) MUST see the same cwd,
    # else _project_root / jail_roots diverge and the byte-compare is meaningless.
    def run_hook(payload):
        p = subprocess.run([sys.executable, str(HOOK)], input=json.dumps(payload),
                           capture_output=True, text=True, cwd=REPO)
        return p.stdout.strip()

    def run_oracle(payload):
        code = marshal.loads(PYC.read_bytes()[16:])
        ns = {"__name__": "_oracle", "__file__": "read-search-discipline.py"}
        os.chdir(REPO); exec(code, ns)
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            old = sys.stdin; sys.stdin = io.StringIO(json.dumps(payload))
            try: ns["main"]()
            finally: sys.stdin = old
        return buf.getvalue().strip()

    UNCHANGED = [
        {"tool_name": "Read",  "tool_input": {"file_path": IN_JAIL}},          # deny (read)
        {"tool_name": "Grep",  "tool_input": {"path": IN_JAIL}},               # deny (grep)
        {"tool_name": "LS",    "tool_input": {"path": IN_JAIL}},               # deny (list)
        {"tool_name": "Read",  "tool_input": {"file_path": "/etc/hosts"}},     # allow (out-of-jail)
        {"tool_name": "Read",  "tool_input": {"file_path": REPO + "/src/logo.png"}},  # allow (png passthrough)
        {"tool_name": "Write", "tool_input": {"file_path": IN_JAIL}},          # allow (not a read tool)
    ]

    def test_differential_unchanged():
        for pl in UNCHANGED:
            assert run_hook(pl) == run_oracle(pl), pl

    # 4 Härtungs-Divergenzen: Hook denied, Orakel erlaubte (leere Ausgabe).
    # Absolute in-jail Pfade/pattern → cwd-unabhängig in-jail.
    def test_hardening_lock_now_denied():
        pl = {"tool_name": "Read", "tool_input": {"file_path": REPO + "/Cargo.lock"}}
        assert run_hook(pl) and not run_oracle(pl)
    def test_hardening_svg_now_denied():
        pl = {"tool_name": "Read", "tool_input": {"file_path": REPO + "/assets/icon.svg"}}
        assert run_hook(pl) and not run_oracle(pl)
    def test_hardening_glob_now_denied():
        pl = {"tool_name": "Glob", "tool_input": {"pattern": REPO + "/src/**/*.rs"}}
        assert run_hook(pl) and not run_oracle(pl)
    def test_hardening_notebookread_now_denied():
        pl = {"tool_name": "NotebookRead", "tool_input": {"notebook_path": REPO + "/nb.ipynb"}}
        assert run_hook(pl) and not run_oracle(pl)

**RED:** `python3 -m pytest ~/.claude/hooks/tests/test_read_search_discipline.py -q` → schlägt fehl,
weil `read-search-discipline.py` noch nicht existiert.

### Schritt 4 — Hook implementieren (native `Write`), GREEN

Schreibe `read-search-discipline.py` gemäß Schritt 1+2. Dann:

    python3 -m pytest ~/.claude/hooks/tests/test_read_search_discipline.py -q

**Expected:** alle Tests grün — `test_differential_unchanged` (0 Byte-Abweichung ggü. Orakel) **und**
die 4 Härtungs-Tests.

### Verify & Close

@call remember_decision("read-search-discipline.py wiederhergestellt+gehärtet (differential gegen .pyc-Orakel, byte-genau außer 4 Härtungen: lock/svg kein passthrough, Glob→list via pattern-Präfix, NotebookRead→read). Env-lokal ~/.claude/hooks/, kein git-Commit. Jail-aware: deny nur in-jail. Noch NICHT in settings.json eingehängt (Task 2).")

@phase-end

@phase "task-2"
## Task 2: settings.json-Gruppe einhängen + Cleanup + Gotcha-Korrektur

@call recall_context("read-search-discipline Restore settings.json Gruppe hook redirect")

**Fläche:** `~/.claude/settings.json` (Python-Skript-Edit, da EXISTIEREND). **Kein** git-Commit.

### Schritt 1 — Gruppe einfügen (Python-Skript, native `Write` einer neuen Skriptdatei → `python3`)

Die neue `PreToolUse`-Gruppe wird **vor** der `hook redirect`-Gruppe derselben Event-Klasse eingefügt
(hartes Deny läuft zuerst; der Soft-Proxy bleibt für erlaubte/out-of-jail Reads). Idempotent — re-run
meldet `already present`:

    import json, pathlib
    home = pathlib.Path.home()
    settings = home / ".claude/settings.json"
    data = json.loads(settings.read_text())
    hooks = data.setdefault("hooks", {}).setdefault("PreToolUse", [])
    MATCH = ("Read|read|ReadFile|read_file|View|view|Grep|grep|Search|search|"
             "ListFiles|list_files|ListDirectory|list_directory|Glob|glob")
    CMD = f"python3 {home}/.claude/hooks/read-search-discipline.py"
    present = any(any(CMD in h.get("command","") for h in g.get("hooks",[])) for g in hooks)
    if present:
        print("already present")
    else:
        group = {"matcher": MATCH, "hooks": [{"type": "command", "command": CMD}]}
        # vor der hook-redirect-Gruppe (matcher enthält '|glob' + command endet auf 'hook redirect')
        idx = next((i for i,g in enumerate(hooks)
                    if any("hook redirect" in h.get("command","") for h in g.get("hooks",[]))), len(hooks))
        hooks.insert(idx, group)
        settings.write_text(json.dumps(data, indent=2) + "\n")
        json.loads(settings.read_text())  # re-parse → raises on invalid JSON
        print(f"inserted at index {idx}")

**Expected:** `inserted at index N` (Erst-Lauf) bzw. `already present` (re-run); kein JSON-Fehler.

### Schritt 2 — Cleanup verwaiste `.pyc` (optional, Python-Skript)

    import pathlib
    pc = pathlib.Path.home() / ".claude/hooks/__pycache__"
    for stale in ("plan-discipline.cpython-314.pyc", "skill-plan-injector.cpython-314.pyc"):
        p = pc / stale
        p.unlink() if p.exists() else None
        print(("deleted " if not p.exists() else "kept ") + str(p))

**Expected:** beide `deleted`. (Die `read-search-discipline.pyc` bleibt bewusst — sie ist das
Test-Orakel aus Task 1.)

### Schritt 3 — Falsche T1-Gotcha korrigieren

@call remember_decision("KORREKTUR der T1-Gotcha (gotcha-hook-set-regime-b-637-read-redirect-on): read_redirect=on ist KEIN Ersatz für read-search-discipline. `lean-ctx hook redirect` + read_redirect=on sind ein Soft-Proxy (permissionDecision immer allow, komprimiert still), NIE das harte Deny. Die harte Read-Disziplin kommt allein von read-search-discipline.py (jetzt wiederhergestellt+gehärtet, vor der redirect-Gruppe eingehängt). Beide sind komplementär. read_redirect=on BLEIBT.")

@phase-end

@phase "task-3"
## Task 3: Fresh-Session-Smoke (Handoff — nicht in dieser Session ausführbar)

@call recall_context("read-search-discipline Smoke fresh session")

Hooks laden bei Session-Start → Checkliste für die **nächste** Session. Jeder Punkt trägt sein „Expected:".

1. Nativer Read auf in-jail-Datei (`src/lib.rs`). **Expected:** **denied** mit `[read-search-discipline]`
   → Verweis auf `ctx_read`.
2. Nativer Read auf `Cargo.lock`. **Expected:** **denied** (nicht mehr passthrough).
3. Native `Glob` mit in-jail `pattern` (`src/**/*.rs`). **Expected:** **denied** → Verweis auf `ctx_tree`/`ctx_glob`.
4. Nativer Read auf out-of-jail-Pfad (`/etc/hosts`). **Expected:** **erlaubt** (lean-ctx serviert ihn nicht).
5. Nativer Read auf ein Binärformat (`*.png` in-jail). **Expected:** **erlaubt** (passthrough).
6. Native Edit `.rs` → weiterhin denied (edit-tool-discipline); native Bash `grep` → weiterhin denied
   (bash-enforce). **Expected:** unverändert.

**Gate:** 1–3 denied + 4–6 wie erwartet → Regression behoben, Härtung wirksam. Ein rotes (1) heißt:
Session nicht frisch **oder** Task 2 nicht gelandet.

@phase-end
