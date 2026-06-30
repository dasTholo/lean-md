# Skill-Body-Variablen (`@var` + `.lean-ctx/lean-md/vars.toml`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Skill-Bodies tragen konfigurierbare Variablen (`@var NAME default="…"` deklariert, `{{ var NAME }}` verwendet), die ein gitignored `.lean-ctx/lean-md/vars.toml` zur Render-Zeit überschreibt; Erstnutzer ist der Test-Befehl in `lmd-test-driven-development`.

**Architecture:** Ein neuer dualer `@var`-Bridge (Block = Deklaration → leer + Default-falls-absent; Inline `{{ var NAME }}` = Lookup) liest/schreibt eine neue `EngineContext.vars`-Map. `render_skill` führt einen Pre-Pass auf dem **vollen** Body aus (erst `vars.toml` seeden, dann `@var`-Defaults füllen), bevor die isolierte Phase gerendert wird — so überlebt die Variable die Phasen-Isolation. Ein `skill vars --init [name]` generiert ein kommentiertes Template.

**Tech Stack:** Rust (lib `lean_md` + bin `lean-md`), `rushdown`/`evalexpr` Render-Core, kein neues Dependency (TOML-Subset von Hand geparst).

## Global Constraints

- **Tests:** immer `cargo nextest run`, nie `cargo test`.
- **Shell:** kein `&&`/`||`/`;`-Chaining — jeder Befehl ist eine eigene Invocation; statt `cd <dir> && cargo …` → `cargo … --manifest-path <dir>/Cargo.toml`.
- **Vor jedem `git add`** (pro geänderter Datei): `cargo fmt`.
- **Branch:** direkt auf `feat-lmd-v2` arbeiten — keine Worktrees.
- **Determinismus (#498):** Output ist deterministische Funktion von (Body, Mode, CRP, Task, `vars.toml`-Inhalt). Keine Timestamps/Counter/Random in Output-Bodies. `CliBackend` == `McpBackend`.
- **Quality-Bar:** `cargo clippy --all-targets -- -D warnings` = 0 Warnings, `cargo fmt --check` clean, `cargo nextest run` grün. Keine Stubs/Platzhalter/Mock-Daten.
- **Sprache:** Code + Code-Kommentare Englisch; Commit-Messages konventionell.
- **Render-Arg-Ebene (`--var key=val` / `vars`-Objekt) ist bewusst DEFERRED** — nicht implementieren.

---

### Task 1: `EngineContext.vars` — Feld + Accessoren

Fundament: die Variablen-Map mit Interior Mutability (konsistent zu `param_scope`) plus drei Accessoren. Alle weiteren Tasks bauen darauf.

**Files:**
- Modify: `src/engine.rs` (Struct `EngineContext` @18-54; `new` @57-75; `with_backend` @77-98; Accessoren nach `param` @128-133; Tests im `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: nichts.
- Produces:
  - Feld `pub vars: RefCell<HashMap<String, String>>`
  - `pub fn var_get(&self, name: &str) -> Option<String>`
  - `pub fn vars_seed(&self, map: HashMap<String, String>)` (Override-Schicht; überschreibt vorhandene Keys)
  - `pub fn var_set_default(&self, name: &str, val: &str)` (setzt nur falls Key absent)

- [ ] **Step 1: Failing Tests schreiben**

In `src/engine.rs` im bestehenden `#[cfg(test)] mod tests` ergänzen:

```rust
#[test]
fn var_set_default_inserts_when_absent() {
    let ctx = std::rc::Rc::new(EngineContext::new(
        crate::header::LeanMdHeader::default(),
        std::path::PathBuf::from("."),
    ));
    ctx.var_set_default("k", "v");
    assert_eq!(ctx.var_get("k"), Some("v".to_string()));
}

#[test]
fn var_set_default_does_not_overwrite() {
    let ctx = std::rc::Rc::new(EngineContext::new(
        crate::header::LeanMdHeader::default(),
        std::path::PathBuf::from("."),
    ));
    ctx.var_set_default("k", "first");
    ctx.var_set_default("k", "second");
    assert_eq!(ctx.var_get("k"), Some("first".to_string()));
}

#[test]
fn vars_seed_then_default_keeps_seed() {
    let ctx = std::rc::Rc::new(EngineContext::new(
        crate::header::LeanMdHeader::default(),
        std::path::PathBuf::from("."),
    ));
    let mut m = std::collections::HashMap::new();
    m.insert("k".to_string(), "config".to_string());
    ctx.vars_seed(m);
    ctx.var_set_default("k", "default");
    assert_eq!(ctx.var_get("k"), Some("config".to_string()));
}

#[test]
fn var_get_unknown_is_none() {
    let ctx = std::rc::Rc::new(EngineContext::new(
        crate::header::LeanMdHeader::default(),
        std::path::PathBuf::from("."),
    ));
    assert_eq!(ctx.var_get("nope"), None);
}
```

- [ ] **Step 2: Test laufen lassen, Fehlschlag verifizieren**

Run: `cargo nextest run -p lean-md var_set_default vars_seed var_get_unknown`
Expected: FAIL — `no method named var_set_default`/`var_get`/`vars_seed` found for struct `EngineContext` (Kompilierfehler).

- [ ] **Step 3: Feld + Accessoren implementieren**

In `struct EngineContext` (nach `phase_bodies` @44, vor `imported` @46) ergänzen:

```rust
    /// Skill-body variables (`@var`). Seeded from `.lean-ctx/lean-md/vars.toml`
    /// (override) then filled with `@var …default=` defaults (default-if-absent).
    /// Interior mutability mirrors `param_scope`; read inline via `{{ var NAME }}`.
    pub vars: RefCell<HashMap<String, String>>,
```

In **`new`** (nach `phase_bodies: RefCell::new(HashMap::new()),` @70) **und** in **`with_backend`** (nach @93) jeweils ergänzen:

```rust
            vars: RefCell::new(HashMap::new()),
```

Accessoren in `impl EngineContext` direkt nach `param` (nach @133) einfügen:

```rust
    /// Look up a skill-body variable; `None` if unset.
    pub fn var_get(&self, name: &str) -> Option<String> {
        self.vars.borrow().get(name).cloned()
    }
    /// Seed the override layer (from `vars.toml`) — overwrites existing entries.
    pub fn vars_seed(&self, map: HashMap<String, String>) {
        let mut v = self.vars.borrow_mut();
        for (k, val) in map {
            v.insert(k, val);
        }
    }
    /// Set a default only if the key is absent (config takes precedence).
    pub fn var_set_default(&self, name: &str, val: &str) {
        self.vars
            .borrow_mut()
            .entry(name.to_string())
            .or_insert_with(|| val.to_string());
    }
```

- [ ] **Step 4: Tests laufen lassen, Erfolg verifizieren**

Run: `cargo nextest run -p lean-md var_set_default vars_seed var_get_unknown`
Expected: PASS (4 Tests).

- [ ] **Step 5: Quality-Bar + Commit**

Run: `cargo fmt`
Run: `cargo clippy --all-targets -- -D warnings`
Expected: 0 Warnings.

```bash
git add src/engine.rs
git commit -m "feat(engine): EngineContext.vars map + var_get/vars_seed/var_set_default accessors"
```

---

### Task 2: `@var`-Bridge (dual: Deklaration + Lookup)

Der Bridge nach Vorlage `src/bridges/env.rs`. Hat `args` ein `default=`-Token → Deklarations-Modus (`var_set_default`, rendert leer); sonst Lookup-Modus (`var_get` oder `""`).

**Files:**
- Create: `src/bridges/var.rs`
- Modify: `src/bridges/mod.rs` (`pub mod var;` @33-34; `reg.register(Box::new(var::VarBridge));` in `default_registry` @146)

**Interfaces:**
- Consumes: `EngineContext::var_get`/`var_set_default` (Task 1); `DirectiveArgs::positional`/`get`; `BridgeError::MissingArg`.
- Produces: `pub struct VarBridge` (`impl DirectiveBridge`, `name() -> "var"`), registriert in `default_registry()`.

- [ ] **Step 1: Failing Tests schreiben**

`src/bridges/var.rs` komplett anlegen (Bridge + Tests in einem):

```rust
//! `@var` bridge — skill-body variables (Spec: @var + .lean-ctx/lean-md/vars.toml).
//! Dual-role like `@include`: a block declaration `@var NAME default="…" desc="…"`
//! registers a default (config-precedence) and renders empty; the inline
//! `{{ var NAME }}` looks the value up. Template for this bridge: `env.rs`.
use std::rc::Rc;

use super::{BridgeError, DirectiveBridge};
use crate::args::DirectiveArgs;
use crate::engine::EngineContext;

/// `@var NAME default="…" [desc="…"]` (declaration) / `{{ var NAME }}` (lookup).
pub struct VarBridge;

impl DirectiveBridge for VarBridge {
    fn name(&self) -> &'static str {
        "var"
    }

    fn execute(&self, ctx: &Rc<EngineContext>, args: &DirectiveArgs) -> Result<String, BridgeError> {
        let name = args.positional(0).ok_or(BridgeError::MissingArg("name"))?;
        match args.get("default") {
            // Declaration mode: set default only if absent (config wins), render empty.
            Some(default) => {
                ctx.var_set_default(name, default);
                Ok(String::new())
            }
            // Lookup mode: resolved value, or empty if unknown (author error, no panic).
            None => Ok(ctx.var_get(name).unwrap_or_default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::LeanMdHeader;
    use std::path::PathBuf;

    fn ctx() -> Rc<EngineContext> {
        Rc::new(EngineContext::new(LeanMdHeader::default(), PathBuf::from(".")))
    }

    #[test]
    fn declaration_renders_empty_and_sets_default() {
        let ctx = ctx();
        let out = VarBridge
            .execute(&ctx, &DirectiveArgs::parse(r#"test_cmd default="cargo test""#))
            .unwrap();
        assert_eq!(out, "");
        assert_eq!(ctx.var_get("test_cmd"), Some("cargo test".to_string()));
    }

    #[test]
    fn declaration_does_not_override_config() {
        let ctx = ctx();
        let mut m = std::collections::HashMap::new();
        m.insert("test_cmd".to_string(), "cargo nextest run".to_string());
        ctx.vars_seed(m);
        VarBridge
            .execute(&ctx, &DirectiveArgs::parse(r#"test_cmd default="cargo test""#))
            .unwrap();
        assert_eq!(ctx.var_get("test_cmd"), Some("cargo nextest run".to_string()));
    }

    #[test]
    fn lookup_returns_value() {
        let ctx = ctx();
        ctx.var_set_default("test_cmd", "cargo test");
        let out = VarBridge
            .execute(&ctx, &DirectiveArgs::parse("test_cmd"))
            .unwrap();
        assert_eq!(out, "cargo test");
    }

    #[test]
    fn lookup_unknown_is_empty() {
        let out = VarBridge
            .execute(&ctx(), &DirectiveArgs::parse("nope"))
            .unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn missing_name_errors() {
        let err = VarBridge
            .execute(&ctx(), &DirectiveArgs::parse(""))
            .unwrap_err();
        assert!(matches!(err, BridgeError::MissingArg(_)));
    }

    #[test]
    fn var_is_registered() {
        assert!(super::super::default_registry().get("var").is_some());
    }
}
```

- [ ] **Step 2: Test laufen lassen, Fehlschlag verifizieren**

Run: `cargo nextest run -p lean-md bridges::var`
Expected: FAIL — `module \`var\` not found` / `var` ist nicht in `default_registry` (Kompilierfehler `var_is_registered`).

- [ ] **Step 3: Modul + Registrierung ergänzen**

In `src/bridges/mod.rs` Modul-Deklaration alphabetisch nach `symbol` (@33) einfügen:

```rust
pub mod var;
```

In `default_registry()` (nach `reg.register(Box::new(render::RenderBridge));` @146) ergänzen:

```rust
    reg.register(Box::new(var::VarBridge));
```

- [ ] **Step 4: Tests laufen lassen, Erfolg verifizieren**

Run: `cargo nextest run -p lean-md bridges::var`
Expected: PASS (6 Tests, inkl. `var_is_registered` = Gate 1).

- [ ] **Step 5: Quality-Bar + Commit**

Run: `cargo fmt`
Run: `cargo clippy --all-targets -- -D warnings`
Expected: 0 Warnings.

```bash
git add src/bridges/var.rs src/bridges/mod.rs
git commit -m "feat(bridges): dual @var bridge (declaration sets default, inline looks up) + register"
```

---

### Task 3: `src/skill_vars.rs` — Loader + Scanner + Template-Generator

Drei skill-agnostische Primitive: `load_vars` (jailed `vars.toml` → Override-Map, TOML-Subset von Hand), `scan_var_decls` (voller Body → `Vec<VarDecl>`), `render_vars_template` (Decls → kommentierter `vars.toml`-Inhalt, Dedup nach Name first-wins + `# WARN:` bei divergierenden Defaults).

**Files:**
- Create: `src/skill_vars.rs`
- Modify: `src/lib.rs` (`pub mod skill_vars;` zwischen `skill_install` @26 und `skills` @27)

**Interfaces:**
- Consumes: `crate::pathx::jail_path(&Path, &Path) -> Result<PathBuf, String>`; `crate::args::DirectiveArgs::parse/positional/get`.
- Produces:
  - `pub struct VarDecl { pub name: String, pub default: String, pub desc: Option<String> }` (derive `Debug, Clone, PartialEq`)
  - `pub fn load_vars(jail_root: &Path) -> HashMap<String, String>`
  - `pub fn scan_var_decls(body: &str) -> Vec<VarDecl>`
  - `pub fn render_vars_template(decls: &[VarDecl]) -> String`

- [ ] **Step 1: Failing Tests schreiben**

`src/skill_vars.rs` mit Tests anlegen (Implementierung folgt in Step 3 — zunächst nur die `use`-Zeile + leeres Modulgerüst lassen, damit der Test kompiliert-und-fehlschlägt). Erst die Tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_vars_reads_jailed_toml() {
        let root = std::env::temp_dir().join(format!("lmd_vars_load_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join(".lean-ctx/lean-md");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("vars.toml"), "test_cmd = \"cargo nextest run\"\n").unwrap();
        let map = load_vars(&root);
        assert_eq!(map.get("test_cmd").map(String::as_str), Some("cargo nextest run"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn load_vars_absent_is_empty() {
        let root = std::env::temp_dir().join(format!("lmd_vars_absent_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert!(load_vars(&root).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn parse_handles_comments_blanks_and_bad_lines() {
        let text = "# header comment\n\ngood = \"value\"\nbad line no equals\nflag = bare\n";
        let map = parse_vars_toml(text);
        assert_eq!(map.get("good").map(String::as_str), Some("value"));
        assert!(!map.contains_key("bad"));
        assert!(!map.contains_key("flag")); // unquoted value skipped
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn scan_extracts_name_default_desc() {
        let body = "<!-- comment -->\n@var test_cmd default=\"cargo test\" desc=\"runner\"\n@phase \"red\"\nuse {{ var test_cmd }}\n@phase-end\n";
        let decls = scan_var_decls(body);
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0], VarDecl {
            name: "test_cmd".to_string(),
            default: "cargo test".to_string(),
            desc: Some("runner".to_string()),
        });
    }

    #[test]
    fn scan_ignores_bare_lookup_lines() {
        // A line without `default=` is a lookup, not a declaration.
        let body = "@var test_cmd\n";
        assert!(scan_var_decls(body).is_empty());
    }

    #[test]
    fn render_template_emits_desc_and_default() {
        let decls = vec![VarDecl {
            name: "test_cmd".to_string(),
            default: "cargo test".to_string(),
            desc: Some("runner".to_string()),
        }];
        let out = render_vars_template(&decls);
        assert!(out.contains("# test_cmd: runner"), "desc comment: {out}");
        assert!(out.contains("test_cmd = \"cargo test\""), "assignment: {out}");
    }

    #[test]
    fn render_template_divergent_defaults_warn_single_entry() {
        let decls = vec![
            VarDecl { name: "test_cmd".to_string(), default: "cargo test".to_string(), desc: None },
            VarDecl { name: "test_cmd".to_string(), default: "cargo nextest run".to_string(), desc: None },
        ];
        let out = render_vars_template(&decls);
        assert!(out.contains("# WARN: test_cmd"), "divergence warn: {out}");
        assert_eq!(out.matches("test_cmd = ").count(), 1, "exactly one entry: {out}");
    }

    #[test]
    fn render_template_same_default_dedups_silently() {
        let decls = vec![
            VarDecl { name: "test_cmd".to_string(), default: "cargo test".to_string(), desc: None },
            VarDecl { name: "test_cmd".to_string(), default: "cargo test".to_string(), desc: None },
        ];
        let out = render_vars_template(&decls);
        assert!(!out.contains("# WARN:"), "no warn for identical default: {out}");
        assert_eq!(out.matches("test_cmd = ").count(), 1);
    }
}
```

- [ ] **Step 2: Test laufen lassen, Fehlschlag verifizieren**

Zuerst `pub mod skill_vars;` in `src/lib.rs` zwischen Zeile 26 (`pub mod skill_install;`) und Zeile 27 (`pub mod skills;`) einfügen:

```rust
pub mod skill_vars;
```

Run: `cargo nextest run -p lean-md skill_vars`
Expected: FAIL — `cannot find function \`load_vars\`` / `parse_vars_toml` / `scan_var_decls` / `render_vars_template` / `VarDecl` (Kompilierfehler).

- [ ] **Step 3: Implementierung schreiben**

Oben in `src/skill_vars.rs` (vor dem `#[cfg(test)] mod tests`) einfügen:

```rust
//! Skill-body variables config (Spec: @var). Three skill-agnostic primitives:
//! `load_vars` (jailed vars.toml → override map), `scan_var_decls` (full body →
//! declarations), `render_vars_template` (declarations → commented vars.toml).
//! Hermetic; no `toml` dependency (flat `key = "value"` subset parsed by hand).

use std::collections::HashMap;
use std::path::Path;

/// A `@var NAME default="…" [desc="…"]` declaration scanned from a skill body.
#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub name: String,
    pub default: String,
    pub desc: Option<String>,
}

/// Load `<jail_root>/.lean-ctx/lean-md/vars.toml` (jailed) into an override map.
/// Absent / un-jailed (`..`/symlink escape) / unreadable → empty map (no error,
/// the common case). Flat `key = "value"` subset only.
pub fn load_vars(jail_root: &Path) -> HashMap<String, String> {
    let candidate = jail_root.join(".lean-ctx/lean-md/vars.toml");
    let Ok(resolved) = crate::pathx::jail_path(&candidate, jail_root) else {
        return HashMap::new();
    };
    let Ok(text) = std::fs::read_to_string(&resolved) else {
        return HashMap::new();
    };
    parse_vars_toml(&text)
}

/// Parse the flat `key = "value"` TOML subset. Best-effort: `#` comments and
/// blank lines are ignored; a line without a quoted string value is skipped.
fn parse_vars_toml(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, rest)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let rest = rest.trim();
        // Value must be a double-quoted string (>= 2 chars: the two quotes).
        if key.is_empty() || rest.len() < 2 || !rest.starts_with('"') || !rest.ends_with('"') {
            continue;
        }
        let val = &rest[1..rest.len() - 1];
        map.insert(key.to_string(), val.to_string());
    }
    map
}

/// Extract every `@var NAME default="…" [desc="…"]` block declaration from the
/// FULL body (one source, two users: render pre-pass + `vars --init`). A bare
/// `@var NAME` (no `default=`) is a lookup, not a declaration, and is skipped.
pub fn scan_var_decls(body: &str) -> Vec<VarDecl> {
    let mut out = Vec::new();
    for line in body.lines() {
        let Some(rest) = line.trim_start().strip_prefix("@var ") else {
            continue;
        };
        let args = crate::args::DirectiveArgs::parse(rest);
        let (Some(default), Some(name)) = (args.get("default"), args.positional(0)) else {
            continue;
        };
        out.push(VarDecl {
            name: name.to_string(),
            default: default.to_string(),
            desc: args.get("desc").map(str::to_string),
        });
    }
    out
}

/// Render a commented `vars.toml` from declarations. Dedup by name (first wins);
/// a divergent later default emits a `# WARN:` line (transparent, no silent
/// overwrite). Same name + same default → one silent entry.
pub fn render_vars_template(decls: &[VarDecl]) -> String {
    let mut out = String::from(
        "# lmd skill vars — generated by `lean-md skill vars --init`\n\
         # Edit values; renders pull them automatically (precedence: this file > seed default).\n",
    );
    let mut seen: HashMap<String, String> = HashMap::new();
    for d in decls {
        if let Some(first) = seen.get(&d.name) {
            if first != &d.default {
                out.push_str(&format!(
                    "# WARN: {} has divergent defaults across skills (kept \"{}\", ignored \"{}\")\n",
                    d.name, first, d.default
                ));
            }
            continue;
        }
        seen.insert(d.name.clone(), d.default.clone());
        out.push('\n');
        if let Some(desc) = &d.desc {
            out.push_str(&format!("# {}: {}\n", d.name, desc));
        }
        out.push_str(&format!("{} = \"{}\"\n", d.name, d.default));
    }
    out
}
```

- [ ] **Step 4: Tests laufen lassen, Erfolg verifizieren**

Run: `cargo nextest run -p lean-md skill_vars`
Expected: PASS (8 Tests — Gates 4, 5, 6 + Template-Verhalten inkl. Divergenz-Warn).

- [ ] **Step 5: Quality-Bar + Commit**

Run: `cargo fmt`
Run: `cargo clippy --all-targets -- -D warnings`
Expected: 0 Warnings.

```bash
git add src/skill_vars.rs src/lib.rs
git commit -m "feat(skill-vars): load_vars (jailed TOML subset) + scan_var_decls + render_vars_template"
```

---

### Task 4: `render_skill`-Pre-Pass (Config seeden → `@var`-Defaults füllen)

Vor dem Render (beide Pfade `phase=None` und `Some(p)`): `vars.toml` jailed laden und seeden, dann `@var`-Defaults aus dem **vollen** Body füllen (Default-falls-absent). Hermetisch getestet über einen synthetischen Overlay-Body in einem temporären Jail (unabhängig von der echten Seed, die erst Task 5 ändert).

**Files:**
- Modify: `src/skills.rs` (`render_skill` @69-101 — Pre-Pass-Hook nach `EngineContext::new` @94; Tests im `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `crate::skill_vars::load_vars`/`scan_var_decls` (Task 3); `EngineContext::vars_seed`/`var_set_default` (Task 1); `VarBridge` für `{{ var … }}`-Dispatch (Task 2); `overlay_body` (vorhanden, @54-64).
- Produces: Pre-Pass-Verhalten in `render_skill` (keine Signatur-Änderung).

- [ ] **Step 1: Failing Tests schreiben**

In `src/skills.rs` im `#[cfg(test)] mod tests` ergänzen (nutzt den vorhandenen Skill-Namen `lmd-test-driven-development`, dessen Overlay-Body den Seed ersetzt — `overlay_body` gewinnt immer):

```rust
/// Write a synthetic overlay body declaring a var at body-top (outside phases)
/// and using it inside an isolated phase — the phase-isolation crux.
fn write_var_overlay(root: &std::path::Path) {
    let dir = root.join(".lean-ctx/lean-md/skills/lmd-test-driven-development");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("body.lmd.md"),
        "@var demo_cmd default=\"DEFAULT_VAL\" desc=\"d\"\n\
         @phase \"p1\"\nP1 uses {{ var demo_cmd }} here\n@phase-end\n\
         @phase \"p2\"\nP2_FOREIGN_MARKER\n@phase-end\n",
    )
    .unwrap();
}

#[test]
fn prepass_default_applies_and_phase_isolated() {
    let root = std::env::temp_dir().join(format!("lmd_var_default_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    write_var_overlay(&root);
    let out = render_skill(
        "lmd-test-driven-development",
        Some("p1"),
        None,
        None,
        root.clone(),
    )
    .unwrap();
    assert!(out.contains("DEFAULT_VAL"), "@var default must resolve in isolated phase: {out}");
    assert!(!out.contains("P2_FOREIGN_MARKER"), "no cross-phase leak: {out}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn prepass_config_overrides_default() {
    let root = std::env::temp_dir().join(format!("lmd_var_override_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    write_var_overlay(&root);
    std::fs::write(
        root.join(".lean-ctx/lean-md/vars.toml"),
        "demo_cmd = \"OVERRIDE_VAL\"\n",
    )
    .unwrap();
    let out = render_skill(
        "lmd-test-driven-development",
        Some("p1"),
        None,
        None,
        root.clone(),
    )
    .unwrap();
    assert!(out.contains("OVERRIDE_VAL"), "vars.toml must win over @var default: {out}");
    assert!(!out.contains("DEFAULT_VAL"), "default must be shadowed by config: {out}");
    let _ = std::fs::remove_dir_all(&root);
}
```

- [ ] **Step 2: Test laufen lassen, Fehlschlag verifizieren**

Run: `cargo nextest run -p lean-md prepass_default_applies prepass_config_overrides`
Expected: FAIL — `{{ var demo_cmd }}` rendert leer (kein Pre-Pass), `out.contains("DEFAULT_VAL")`/`OVERRIDE_VAL` falsch.

- [ ] **Step 3: Pre-Pass implementieren**

In `render_skill` (`src/skills.rs`) direkt nach `let ctx = Rc::new(EngineContext::new(header, jail_root));` (@94), vor dem `match phase {`-Block einfügen:

```rust
    // `@var` pre-pass (Spec): seed the override layer from `vars.toml`, then fill
    // `@var …default=` defaults from the FULL body (default-if-absent). Runs on the
    // full body so isolated phases see vars declared at body-top (outside phases).
    ctx.vars_seed(crate::skill_vars::load_vars(&ctx.jail_root));
    for decl in crate::skill_vars::scan_var_decls(body) {
        ctx.var_set_default(&decl.name, &decl.default);
    }
```

- [ ] **Step 4: Tests laufen lassen, Erfolg verifizieren**

Run: `cargo nextest run -p lean-md prepass_default_applies prepass_config_overrides`
Expected: PASS (Gate 3 Präzedenz + Gate 8 Phasen-Isolation).

Run: `cargo nextest run -p lean-md skills`
Expected: PASS — alle bestehenden `skills`-Tests bleiben grün (kein Regress).

- [ ] **Step 5: Quality-Bar + Commit**

Run: `cargo fmt`
Run: `cargo clippy --all-targets -- -D warnings`
Expected: 0 Warnings.

```bash
git add src/skills.rs
git commit -m "feat(skills): @var render-skill pre-pass — seed vars.toml then fill @var defaults"
```

---

### Task 5: TDD-Skill-Seed auf `@var` umstellen

`@var test_cmd default="cargo test"` ganz oben deklarieren, die drei hartkodierten `cargo nextest run` durch `{{ var test_cmd }}` ersetzen. Damit rendert der Skill zero-config `cargo test`, mit `vars.toml` `cargo nextest run` (realer `.lean-ctx/lean-md/`-E2E-Test). Die Seed-Konsistenz-Gate (`include_str!` == on-disk) bleibt automatisch grün.

**Files:**
- Modify: `content/skills/lmd-test-driven-development/body.lmd.md` (Deklaration nach Zeile 1, vor `@phase "red"`; `cargo nextest run` → `{{ var test_cmd }}` in Zeilen 9, 22, 34)
- Modify: `src/skills.rs` (zwei neue Tests im `#[cfg(test)] mod tests` — Gate 2 + Gate 9; vorhandene Gate 10 `tdd_body_matches_seed_file_on_disk` bleibt unverändert)

**Interfaces:**
- Consumes: Pre-Pass aus Task 4; `@var`-Bridge aus Task 2.
- Produces: geänderte Seed; `test_cmd`-Variable mit Default `cargo test`.

- [ ] **Step 1: Failing Tests schreiben**

In `src/skills.rs` im `#[cfg(test)] mod tests` ergänzen (hermetische Temp-Jails — **nicht** `PathBuf::from(".")`, da das Repo lokal eine `vars.toml` haben kann):

```rust
#[test]
fn tdd_red_renders_default_test_cmd_without_config() {
    let root = std::env::temp_dir().join(format!("lmd_tdd_default_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let out = render_skill(
        "lmd-test-driven-development",
        Some("red"),
        None,
        None,
        root.clone(),
    )
    .unwrap();
    assert!(out.contains("cargo test"), "default test_cmd must render: {out}");
    assert!(!out.contains("cargo nextest run"), "no override without vars.toml: {out}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn tdd_red_renders_overridden_test_cmd_with_config() {
    let root = std::env::temp_dir().join(format!("lmd_tdd_override_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root.join(".lean-ctx/lean-md")).unwrap();
    std::fs::write(
        root.join(".lean-ctx/lean-md/vars.toml"),
        "test_cmd = \"cargo nextest run\"\n",
    )
    .unwrap();
    let out = render_skill(
        "lmd-test-driven-development",
        Some("red"),
        None,
        None,
        root.clone(),
    )
    .unwrap();
    assert!(out.contains("cargo nextest run"), "vars.toml must override: {out}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn tdd_render_is_byte_stable_with_config() {
    let root = std::env::temp_dir().join(format!("lmd_tdd_determinism_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root.join(".lean-ctx/lean-md")).unwrap();
    std::fs::write(
        root.join(".lean-ctx/lean-md/vars.toml"),
        "test_cmd = \"cargo nextest run\"\n",
    )
    .unwrap();
    let a = render_skill("lmd-test-driven-development", Some("red"), None, None, root.clone()).unwrap();
    let b = render_skill("lmd-test-driven-development", Some("red"), None, None, root.clone()).unwrap();
    assert_eq!(a, b, "two renders with same vars.toml must be byte-identical");
    let _ = std::fs::remove_dir_all(&root);
}
```

- [ ] **Step 2: Test laufen lassen, Fehlschlag verifizieren**

Run: `cargo nextest run -p lean-md tdd_red_renders_default tdd_red_renders_overridden tdd_render_is_byte_stable`
Expected: FAIL — die Seed enthält noch hartkodiertes `cargo nextest run`; der Default-Test findet kein `cargo test`, der Default-Test sieht fälschlich `cargo nextest run`.

- [ ] **Step 3: Seed umschreiben**

In `content/skills/lmd-test-driven-development/body.lmd.md`:

(a) Nach Zeile 1 (`<!-- … -->`) und der Leerzeile, **vor** `@phase "red"`, die Deklaration einfügen:

```
@var test_cmd default="cargo test" desc="Test runner command; this project uses 'cargo nextest run'"
```

(b) Drei Ersetzungen (jeweils nur den Befehl innerhalb `ctx_shell "…"`):

- Zeile 9: `run \`ctx_shell "cargo nextest run"\` and confirm the test fails *for the right reason*` → `run \`ctx_shell "{{ var test_cmd }}"\` and confirm the test fails *for the right reason*`
- Zeile 22: `run \`ctx_shell "cargo nextest run"\` and confirm the test passes.` → `run \`ctx_shell "{{ var test_cmd }}"\` and confirm the test passes.`
- Zeile 34: `` `ctx_shell "cargo nextest run"` after each change; it must stay green.`` → `` `ctx_shell "{{ var test_cmd }}"` after each change; it must stay green.``

> Editieren über `ctx_edit(path, old_string, new_string)` (die `.lmd.md`-Datei liegt im Jail; native Read/Edit ist geblockt). Pro Ersetzung ein eindeutiger `old_string`.

- [ ] **Step 4: Tests laufen lassen, Erfolg verifizieren**

Run: `cargo nextest run -p lean-md tdd_red_renders_default tdd_red_renders_overridden tdd_render_is_byte_stable tdd_body_matches_seed_file_on_disk`
Expected: PASS (Gate 2 Config-Override, Gate 9 Determinismus, Gate 10 Seed-Konsistenz).

Run: `cargo nextest run -p lean-md skills`
Expected: PASS — `tdd_phases_render_isolated_no_cross_leak`, `every_tdd_phase_includes_test_first_core` etc. bleiben grün (sie prüfen Marker, nicht den Befehlstext).

- [ ] **Step 5: Quality-Bar + Commit**

Run: `cargo fmt`
Run: `cargo clippy --all-targets -- -D warnings`
Expected: 0 Warnings.

```bash
git add content/skills/lmd-test-driven-development/body.lmd.md src/skills.rs
git commit -m "feat(skills): TDD seed uses @var test_cmd (default cargo test) instead of hardcoded nextest"
```

---

### Task 6: `lean-md skill vars --init [name]` — Template-Generator (CLI)

Sub-Sub-Command unter `skill`. **Name optional:** mit Name → ein Skill; ohne Name → Aggregation über **alle** `SKILLS`. Schreibt `.lean-ctx/lean-md/vars.toml` nur falls absent (Clobber-Schutz); existiert sie → Template nach stdout + Hinweis auf stderr (Exit 0). Die schreib-Logik liegt testbar in `skill_vars.rs`; der Bin ist ein dünner Wrapper.

**Files:**
- Modify: `src/skill_vars.rs` (`InitOutcome` + `write_vars_template`; Tests — Gate 7)
- Modify: `src/skills.rs` (`pub fn all_skill_bodies() -> Vec<&'static str>`)
- Modify: `src/bin/lean_md.rs` (`cmd_skill` @261 — `vars`-Branch; neue `cmd_skill_vars` + Usage @120-124)

**Interfaces:**
- Consumes: `scan_var_decls`/`render_vars_template` (Task 3); `skill_body` (vorhanden @28) + neue `all_skill_bodies`.
- Produces:
  - `pub enum InitOutcome { Written(PathBuf), Exists(PathBuf) }`
  - `pub fn write_vars_template(decls: &[VarDecl], project_root: &Path) -> std::io::Result<InitOutcome>` (schreibt `<project_root>/.lean-ctx/lean-md/vars.toml` nur falls absent)
  - `pub fn all_skill_bodies() -> Vec<&'static str>`

- [ ] **Step 1: Failing Tests schreiben**

In `src/skill_vars.rs` im `#[cfg(test)] mod tests` ergänzen:

```rust
#[test]
fn write_template_creates_file_when_absent() {
    let root = std::env::temp_dir().join(format!("lmd_init_write_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let decls = vec![VarDecl {
        name: "test_cmd".to_string(),
        default: "cargo test".to_string(),
        desc: Some("runner".to_string()),
    }];
    let outcome = write_vars_template(&decls, &root).unwrap();
    let target = root.join(".lean-ctx/lean-md/vars.toml");
    assert!(matches!(outcome, InitOutcome::Written(ref p) if *p == target));
    let body = std::fs::read_to_string(&target).unwrap();
    assert!(body.contains("# test_cmd: runner"));
    assert!(body.contains("test_cmd = \"cargo test\""));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn write_template_does_not_clobber_existing() {
    let root = std::env::temp_dir().join(format!("lmd_init_clobber_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root.join(".lean-ctx/lean-md")).unwrap();
    let target = root.join(".lean-ctx/lean-md/vars.toml");
    std::fs::write(&target, "test_cmd = \"USER_EDIT\"\n").unwrap();
    let decls = vec![VarDecl {
        name: "test_cmd".to_string(),
        default: "cargo test".to_string(),
        desc: None,
    }];
    let outcome = write_vars_template(&decls, &root).unwrap();
    assert!(matches!(outcome, InitOutcome::Exists(_)));
    // User edit untouched.
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "test_cmd = \"USER_EDIT\"\n");
    let _ = std::fs::remove_dir_all(&root);
}
```

In `src/skills.rs` im `#[cfg(test)] mod tests` ergänzen (Gate 7b Aggregations-Pfad):

```rust
#[test]
fn all_skill_bodies_aggregate_contains_test_cmd_decl() {
    let bodies = all_skill_bodies();
    let decls: Vec<_> = bodies
        .iter()
        .flat_map(|b| crate::skill_vars::scan_var_decls(b))
        .collect();
    assert!(
        decls.iter().any(|d| d.name == "test_cmd"),
        "aggregating @var across all SKILLS must surface test_cmd"
    );
}
```

- [ ] **Step 2: Test laufen lassen, Fehlschlag verifizieren**

Run: `cargo nextest run -p lean-md write_template all_skill_bodies_aggregate`
Expected: FAIL — `cannot find function \`write_vars_template\``/`all_skill_bodies`, `InitOutcome` unbekannt (Kompilierfehler).

- [ ] **Step 3: Implementierung schreiben**

In `src/skill_vars.rs` (nach `render_vars_template`, `use std::path::Path;` ist schon importiert; `PathBuf` ergänzen): die `use`-Zeile auf `use std::path::{Path, PathBuf};` erweitern und einfügen:

```rust
/// Result of `write_vars_template`: a fresh write, or a no-op because the file
/// already exists (clobber-protected — the user merges manually).
pub enum InitOutcome {
    Written(PathBuf),
    Exists(PathBuf),
}

/// Write the generated template to `<project_root>/.lean-ctx/lean-md/vars.toml`
/// ONLY if absent (no clobber of user edits). Creates parent dirs as needed.
pub fn write_vars_template(decls: &[VarDecl], project_root: &Path) -> std::io::Result<InitOutcome> {
    let target = project_root.join(".lean-ctx/lean-md/vars.toml");
    if target.exists() {
        return Ok(InitOutcome::Exists(target));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target, render_vars_template(decls))?;
    Ok(InitOutcome::Written(target))
}
```

In `src/skills.rs` (nach `skill_body` @28-33) einfügen:

```rust
/// All embedded skill bodies (for cross-skill `@var` aggregation in `vars --init`).
pub fn all_skill_bodies() -> Vec<&'static str> {
    SKILLS.iter().map(|(_, b)| *b).collect()
}
```

In `src/bin/lean_md.rs`:

(a) Imports oben ergänzen (zu `use lean_md::skills::render_skill;` @16):

```rust
use lean_md::skills::{all_skill_bodies, render_skill, skill_body};
use lean_md::skill_vars::{InitOutcome, render_vars_template, scan_var_decls, write_vars_template};
```

(b) In `cmd_skill` (@261) den `vars`-Branch **vor** dem `name`-Guard (@269) einsetzen — direkt nach `let sub = rest.first().map_or("", String::as_str);`:

```rust
    if sub == "vars" {
        cmd_skill_vars(&rest[1..]);
        return;
    }
```

(c) Neue Funktion nach `cmd_skill` (nach @294) einfügen:

```rust
fn cmd_skill_vars(rest: &[String]) {
    if !rest.iter().any(|a| a == "--init") {
        eprintln!("lean-md skill vars: missing --init");
        std::process::exit(1);
    }
    // Optional skill name: present → that skill; absent → aggregate across all.
    let name = rest.iter().find(|a| !a.starts_with('-')).map(String::as_str);
    let decls: Vec<_> = match name {
        Some(n) => match skill_body(n) {
            Some(body) => scan_var_decls(body),
            None => {
                eprintln!("lean-md skill vars --init: unknown skill '{n}'");
                std::process::exit(1);
            }
        },
        None => all_skill_bodies()
            .iter()
            .flat_map(|b| scan_var_decls(b))
            .collect(),
    };
    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    match write_vars_template(&decls, &project_root) {
        Ok(InitOutcome::Written(p)) => println!("wrote {}", p.display()),
        Ok(InitOutcome::Exists(p)) => {
            eprintln!("{} existiert bereits — nicht überschrieben", p.display());
            print!("{}", render_vars_template(&decls));
        }
        Err(e) => {
            eprintln!("lean-md skill vars --init: {e}");
            std::process::exit(1);
        }
    }
}
```

(d) Usage-String (@120-124) um die `vars`-Zeile erweitern:

```rust
            \n  skill  <install|remove> <name> [--global|--local]\
            \n  skill  vars --init [name]"
```

- [ ] **Step 4: Tests laufen lassen, Erfolg verifizieren**

Run: `cargo nextest run -p lean-md write_template all_skill_bodies_aggregate`
Expected: PASS (Gate 7 write/clobber + Gate 7b Aggregations-Pfad).

Run: `cargo build -p lean-md --bin lean-md`
Expected: erfolgreicher Build (Bin-Verdrahtung kompiliert).

- [ ] **Step 5: Quality-Bar + Commit**

Run: `cargo fmt`
Run: `cargo clippy --all-targets -- -D warnings`
Expected: 0 Warnings.

```bash
git add src/skill_vars.rs src/skills.rs src/bin/lean_md.rs
git commit -m "feat(cli): skill vars --init [name] — single or all-skills vars.toml template, no clobber"
```

---

### Task 7: Lokales Setup + End-to-End-Verifikation (Schluss-Gate)

Lokale (gitignored) `vars.toml` für dieses Repo anlegen, dann die vollständige Quality-Bar + ein CLI-E2E gegen das echte Binary. Kein neuer Produktivcode — nur Verifikation und ein lokaler Override.

**Files:**
- Create: `.lean-ctx/lean-md/vars.toml` (lokal, gitignored — `.lean-ctx/lean-md/` steht bereits in `.gitignore`; **nicht** committen)

**Interfaces:**
- Consumes: alles aus Tasks 1–6.
- Produces: keine Code-Artefakte.

- [ ] **Step 1: Lokale `vars.toml` generieren**

Run: `cargo run -p lean-md --bin lean-md -- skill vars --init lmd-test-driven-development`
Expected: `wrote …/.lean-ctx/lean-md/vars.toml` (falls noch absent) ODER stderr „existiert bereits …“ + Template auf stdout (falls vorhanden).

Anschließend den Wert auf das Projekt-Tooling setzen (über `ctx_edit` / Write) — Datei-Inhalt:

```toml
# lmd skill vars — lmd-test-driven-development
# Werte editieren; Renders ziehen sie automatisch (Präzedenz: diese Datei > Seed-Default).

# test_cmd: Test runner command; this project uses 'cargo nextest run'
test_cmd = "cargo nextest run"
```

- [ ] **Step 2: CLI-E2E — Override greift**

Run: `cargo run -p lean-md --bin lean-md -- render --skill lmd-test-driven-development --phase red`
Expected: Output enthält `ctx_shell "cargo nextest run"` (lokale `vars.toml` überschreibt den Default) und **nicht** `cargo test`.

- [ ] **Step 3: Clobber-Schutz verifizieren**

Run: `cargo run -p lean-md --bin lean-md -- skill vars --init lmd-test-driven-development`
Expected: stderr „…/.lean-ctx/lean-md/vars.toml existiert bereits — nicht überschrieben“, Template auf stdout, Exit 0, lokale Datei unverändert (`test_cmd = "cargo nextest run"`).

- [ ] **Step 4: Vollständige Quality-Bar**

Run: `cargo fmt --check`
Expected: keine Ausgabe (clean).

Run: `cargo clippy --all-targets -- -D warnings`
Expected: 0 Warnings.

Run: `cargo nextest run`
Expected: gesamte Suite grün (inkl. aller neuen Gates 1–10 + bestehender Determinismus-/Konsistenz-Tests).

- [ ] **Step 5: Bestätigung (Gate 11)**

Verifiziere, dass `.lean-ctx/lean-md/vars.toml` **nicht** im `git status` als tracked/staged erscheint:

Run: `git status --short`
Expected: `.lean-ctx/lean-md/vars.toml` taucht **nicht** auf (gitignored). Kein `git add`/Commit dieser Datei.

> Dieser Task erzeugt keinen Commit (reine Verifikation + lokaler, gitignored Override). Die Branch-Finalisierung (merge/PR) entscheidet der User separat.

---

## Self-Review

**Spec-Coverage (Komponenten → Tasks):**
- `EngineContext.vars` + Accessoren → Task 1 ✅
- `src/bridges/var.rs` + Registrierung → Task 2 ✅ (Gate 1)
- `src/skill_vars.rs` (`load_vars`/`scan_var_decls`/`render_vars_template`) + `lib.rs` → Task 3 ✅ (Gates 4, 5, 6)
- `render_skill`-Pre-Pass → Task 4 ✅ (Gate 3, Gate 8)
- Seed `body.lmd.md` (`@var` + `{{ var test_cmd }}`) → Task 5 ✅ (Gate 2, Gate 9, Gate 10)
- `skill vars --init [name]` (single + all-skills + Clobber-Schutz) → Task 6 ✅ (Gate 7, Gate 7b)
- Lokales Setup + Quality-Bar/E2E → Task 7 ✅ (Gate 11)

**Spec-Gates → Tasks:** 1→T2, 2→T5, 3→T4, 4→T3, 5→T3, 6→T3, 7→T6, 7b→T3(Divergenz)+T6(Aggregation), 8→T4, 9→T5, 10→T5, 11→T7. Alle abgedeckt.

**Typ-Konsistenz:** `VarDecl{name,default,desc}` einheitlich (T3/T6); `var_get`/`vars_seed`/`var_set_default` Signaturen identisch genutzt (T1→T2→T4); `InitOutcome::{Written,Exists}` (T6); `all_skill_bodies()`/`skill_body()` Rückgaben (`Vec<&'static str>` / `Option<&'static str>`) konsistent verwendet.

**Bewusst DEFERRED (nicht im Plan, per Spec):** Render-Arg-Ebene `--var key=val`/`vars`-Objekt, per-Skill-Vars, Nicht-String-Werte, `--merge`/`--update`.

**Hinweis zur Phasen-Isolation (Knackpunkt):** Der Pre-Pass (T4) scannt den **vollen** `body`, bevor `capture_phase_bodies` die isolierte Phase zieht — deshalb sieht die isolierte Phase die am Body-Top deklarierte Variable, obwohl die `@var`-Zeile selbst nicht im Phasen-Body liegt. Genau dafür ist der T4-Overlay-Test (`prepass_default_applies_and_phase_isolated`) gebaut.
