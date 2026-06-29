# Reformat-Test-Robustheit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Die zwei reformat-Tests vom installierten lean-ctx + rustfmt-Fallback entkoppeln, sodass sie nur noch lean-md-Eigenleistung (Dispatch + Pass-Through) über ein injiziertes Stub-Backend prüfen.

**Architecture:** Beide Tests wechseln von `EngineContext::new` (real ausshellendes `CliBackend`) auf `EngineContext::with_backend` mit einem inline definierten Stub-`CodeIntelBackend`, das ein neutrales Sentinel `"STUB_REFORMAT_OK"` liefert. Damit ist das Testergebnis unabhängig von lean-ctx-Version, `rustfmt` im PATH und laufender IDE.

**Tech Stack:** Rust, `cargo nextest`, lean-md-internes `CodeIntelBackend`-Trait + `EngineContext::with_backend`-Seam.

## Global Constraints

- Tests immer mit `cargo nextest run` ausführen — nie `cargo test`.
- Shell ohne `&&`/`||`/`;`-Verkettung — jedes Kommando ist eine eigene Invocation.
- Vor jedem `git add` (pro geänderter Datei): `cargo fmt`.
- Zero clippy warnings, alle Tests grün.
- Output-Determinismus (#498) unberührt — alle Änderungen sind test-only.
- Kein PATH-Mangeln, keine tolerante Multi-Pfad-Assertion, kein neuer Live-Degradierungstest (YAGNI; siehe Spec).
- Etabliertes Mock-Idiom: inline `CodeIntelBackend`-Impl je Test-Modul (vgl. `Marker` in `engine.rs:933`, `RecordingBackend` in `phases.rs:602`).

---

### Task 1: `@reformat`-Bridge-Test auf Dispatch + Pass-Through umstellen

**Files:**
- Modify/Test: `src/bridges/reformat.rs` — Test `returns_backend_required_envelope_headless` (aktuell ca. `reformat.rs:107-120`)

**Interfaces:**
- Consumes: `EngineContext::with_backend(header: LeanMdHeader, jail_root: PathBuf, backend: Box<dyn CodeIntelBackend>) -> EngineContext`; `crate::backend::{CodeIntelBackend, BackendError}`; `DirectiveArgs::parse`; `ReformatBridge.execute(&Rc<EngineContext>, &DirectiveArgs)`.
- Produces: Nichts (test-only). Etabliert das Stub-Muster, das Task 2 spiegelt.

Kontext: `src/bridges/reformat.rs` importiert auf Modulebene `use crate::engine::EngineContext;`; das Test-Modul hat `use super::*;`, `use crate::header::LeanMdHeader;`, `use std::path::PathBuf;` und nutzt `use std::rc::Rc;` (über `super::*`). Der Bridge dispatcht `ctx_refactor` mit `action="reformat"` und fügt bei `path=`-Adressierung `path=<resolved abs>` ein (`reformat.rs:execute` + `addressing::build_target_with`). `resolve_tool_path` kanonikalisiert den Pfad → Assertion prüft `ends_with("r.rs")`, nicht exakte Gleichheit.

- [ ] **Step 1: Aktuellen Fehlschlag bestätigen (rustfmt im PATH)**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml -E 'test(returns_backend_required_envelope_headless)'`
Expected: FAIL — Output enthält `via rustfmt — …` statt `BACKEND_REQUIRED`/`ERROR` (nur falls `rustfmt` im PATH; sonst grün — dann ist Step 1 informativ und wird übersprungen).

- [ ] **Step 2: Test ersetzen**

Ersetze in `src/bridges/reformat.rs` den kompletten Test:

```rust
    #[test]
    fn returns_backend_required_envelope_headless() {
        let dir = std::env::temp_dir().join("lmd_reformat_degrade");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("r.rs");
        std::fs::write(&f, "fn foo() {}\n").unwrap();
        let ctx = ctx_at(dir.clone());

        let args = DirectiveArgs::parse(&format!("path={} line=1", f.to_str().unwrap()));
        let out = ReformatBridge.execute(&ctx, &args).unwrap();
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "headless reformat must degrade cleanly, got: {out}"
        );
    }
```

durch:

```rust
    #[test]
    fn dispatches_reformat_and_passes_output_through() {
        // lean-md contract: the bridge dispatches `ctx_refactor action=reformat`
        // with the resolved path and returns the backend output verbatim. Whether
        // the backend degrades headless (BACKEND_REQUIRED), reformats via a live
        // JetBrains IDE, or via a local rustfmt fallback is lean-ctx's contract —
        // not lean-md's. So we inject a stub backend and assert only dispatch +
        // pass-through (no dependency on lean-ctx version / rustfmt / running IDE).
        use crate::backend::{BackendError, CodeIntelBackend};
        use std::cell::RefCell;

        struct StubBackend {
            calls: Rc<RefCell<Vec<(String, serde_json::Value)>>>,
        }
        impl CodeIntelBackend for StubBackend {
            fn call(&self, tool: &str, args: serde_json::Value) -> Result<String, BackendError> {
                self.calls.borrow_mut().push((tool.to_string(), args));
                Ok("STUB_REFORMAT_OK".to_string())
            }
        }

        let dir = std::env::temp_dir().join("lmd_reformat_dispatch");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("r.rs");
        std::fs::write(&f, "fn foo() {}\n").unwrap();

        let calls = Rc::new(RefCell::new(Vec::new()));
        let ctx = Rc::new(EngineContext::with_backend(
            LeanMdHeader::default(),
            dir.clone(),
            Box::new(StubBackend {
                calls: calls.clone(),
            }),
        ));

        let args = DirectiveArgs::parse(&format!("path={} line=1", f.to_str().unwrap()));
        let out = ReformatBridge.execute(&ctx, &args).unwrap();

        // Pass-through: the bridge returns the backend output verbatim.
        assert_eq!(
            out, "STUB_REFORMAT_OK",
            "bridge must pass backend output through verbatim, got: {out}"
        );

        // Dispatch: exactly one ctx_refactor call, action=reformat, resolved path.
        let calls = calls.borrow();
        let (tool, sent) = calls
            .first()
            .expect("bridge must dispatch exactly one backend call");
        assert_eq!(tool, "ctx_refactor", "reformat must dispatch to ctx_refactor");
        assert_eq!(
            sent.get("action").and_then(serde_json::Value::as_str),
            Some("reformat"),
            "must set action=reformat"
        );
        let path = sent
            .get("path")
            .and_then(serde_json::Value::as_str)
            .expect("must dispatch a resolved path");
        assert!(
            path.ends_with("r.rs"),
            "must dispatch the resolved file path, got: {path}"
        );
    }
```

- [ ] **Step 3: `cargo fmt`**

Run: `cargo fmt --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml`
Expected: keine Ausgabe, Exit 0.

- [ ] **Step 4: Neuen Test ausführen**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml -E 'test(dispatches_reformat_and_passes_output_through)'`
Expected: PASS (1 test run, 0 failed) — unabhängig davon, ob `rustfmt` im PATH ist.

- [ ] **Step 5: Clippy auf das Modul**

Run: `cargo clippy --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml --all-targets`
Expected: `Finished` ohne Warnungen.

- [ ] **Step 6: Commit**

```bash
git add src/bridges/reformat.rs
git commit -m "test(reformat): assert bridge dispatch + pass-through via stub backend"
```

---

### Task 2: `@reformat`-e2e-Render-Test auf Stub-Backend umstellen

**Files:**
- Modify/Test: `src/engine.rs` — Test `reformat_renders_backend_required_e2e` (aktuell ca. `engine.rs:510-533`)

**Interfaces:**
- Consumes: `EngineContext::with_backend(...)`; `crate::backend::{CodeIntelBackend, BackendError}`; `render_body(&Rc<EngineContext>, &str) -> String`; `LeanMdHeader::default()`.
- Produces: Nichts (test-only).

Kontext: Das `engine.rs`-Test-Modul (`mod tests { use super::*; … }`) hat `Rc`, `EngineContext`, `LeanMdHeader`, `render_body` über `use super::*` im Scope. Vorbild für den inline-Stub ist `with_backend_injects_a_custom_backend` (`engine.rs:928`, `Marker`-Backend). Hier genügt ein nicht-aufzeichnender Stub: der e2e-Test prüft Dispatch durch die Pipeline + Pass-Through, nicht die Args (die deckt Task 1 ab).

- [ ] **Step 1: Aktuellen Fehlschlag bestätigen (rustfmt im PATH)**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml -E 'test(reformat_renders_backend_required_e2e)'`
Expected: FAIL — Output enthält `via rustfmt — …` statt `BACKEND_REQUIRED`/`ERROR` (nur falls `rustfmt` im PATH).

- [ ] **Step 2: Test ersetzen**

Ersetze in `src/engine.rs` den kompletten Test:

```rust
    #[test]
    fn reformat_renders_backend_required_e2e() {
        // @reformat must dispatch through the full render pipeline and degrade
        // to the BACKEND_REQUIRED envelope headless — never the unknown-directive
        // fallback, never a panic.
        let dir = std::env::temp_dir().join("lmd_gate_reformat");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("e2e.rs");
        std::fs::write(&f, "fn   spaced( ) {}\n").unwrap();
        let p = f.to_str().unwrap();

        let ctx = Rc::new(EngineContext::new(LeanMdHeader::default(), dir.clone()));
        let out = render_body(&ctx, &format!("@reformat path={p}\n"));

        assert!(
            !out.contains("unknown directive"),
            "@reformat must dispatch (not unknown-directive fallback): {out}"
        );
        assert!(
            out.contains("BACKEND_REQUIRED") || out.starts_with("ERROR"),
            "headless reformat must degrade to BACKEND_REQUIRED envelope, got: {out}"
        );
        assert!(!out.trim().is_empty(), "empty render");
    }
```

durch:

```rust
    #[test]
    fn reformat_dispatches_through_render_pipeline() {
        // @reformat must dispatch through the full render pipeline and pass the
        // backend output through — never the unknown-directive fallback, never a
        // panic. Headless degradation (BACKEND_REQUIRED) vs. live-IDE / rustfmt
        // success is lean-ctx's contract, so we inject a stub backend and assert
        // only lean-md's own behaviour: dispatch + verbatim pass-through.
        use crate::backend::{BackendError, CodeIntelBackend};
        struct StubBackend;
        impl CodeIntelBackend for StubBackend {
            fn call(&self, _tool: &str, _args: serde_json::Value) -> Result<String, BackendError> {
                Ok("STUB_REFORMAT_OK".to_string())
            }
        }

        let dir = std::env::temp_dir().join("lmd_gate_reformat");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("e2e.rs");
        std::fs::write(&f, "fn   spaced( ) {}\n").unwrap();
        let p = f.to_str().unwrap();

        let ctx = Rc::new(EngineContext::with_backend(
            LeanMdHeader::default(),
            dir.clone(),
            Box::new(StubBackend),
        ));
        let out = render_body(&ctx, &format!("@reformat path={p}\n"));

        assert!(
            !out.contains("unknown directive"),
            "@reformat must dispatch (not unknown-directive fallback): {out}"
        );
        assert!(
            out.contains("STUB_REFORMAT_OK"),
            "backend output must pass through the render pipeline, got: {out}"
        );
        assert!(!out.trim().is_empty(), "empty render");
    }
```

- [ ] **Step 3: `cargo fmt`**

Run: `cargo fmt --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml`
Expected: keine Ausgabe, Exit 0.

- [ ] **Step 4: Neuen Test ausführen**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml -E 'test(reformat_dispatches_through_render_pipeline)'`
Expected: PASS (1 test run, 0 failed).

- [ ] **Step 5: Volle Suite + Clippy (env-Robustheit verifizieren)**

Run: `cargo nextest run --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml`
Expected: alle Tests grün — insbesondere keine `returns_backend_required_envelope_headless` / `reformat_renders_backend_required_e2e` mehr (umbenannt), kein env-abhängiger Fehlschlag mit `rustfmt` im PATH.

Run: `cargo clippy --manifest-path /home/tholo/Scripts/lean-md/Cargo.toml --all-targets`
Expected: `Finished` ohne Warnungen.

- [ ] **Step 6: TODO.md-Eintrag schließen**

Entferne den OFFEN-Block in `TODO.md` (oder markiere ihn erledigt) via `ctx_edit`, da beide Tests nun backend-agnostisch sind.

- [ ] **Step 7: Commit**

```bash
git add src/engine.rs TODO.md
git commit -m "test(reformat): e2e dispatch via stub backend; close rustfmt-fallback TODO"
```

---

## Self-Review

**Spec coverage:**
- Spec §„Änderungen 1" (reformat.rs Umbenennung, StubBackend, Dispatch + verbatim Pass-Through, Drop der BACKEND_REQUIRED-Assertion) → Task 1. ✓
- Spec §„Änderungen 2" (engine.rs Umbenennung, with_backend-Stub, nicht-unknown-directive + Sentinel + nicht-leer, Drop BACKEND_REQUIRED) → Task 2. ✓
- Spec §„Mock-Platzierung" (inline je Modul) → Task 1 Step 2 + Task 2 Step 2. ✓
- Spec §„Sentinel-Wahl" (neutrales `STUB_REFORMAT_OK`) → beide Tasks. ✓
- Spec §„Akzeptanzkriterien" (grün mit/ohne rustfmt, kein Live-lean-ctx-Call für diese Fälle, clippy, fmt, #498) → Task 2 Step 5 + fmt/clippy-Steps. ✓
- Spec §„NICHT enthalten" (kein PATH-Mangeln/tolerante Assertion/Live-Test) → in Global Constraints festgehalten, nicht implementiert. ✓

**Placeholder scan:** Keine TBD/TODO/„handle edge cases"; vollständiger Test-Code in jedem ersetzenden Step.

**Type consistency:** Stub-Struct heißt in beiden Tasks `StubBackend`; Sentinel `"STUB_REFORMAT_OK"` identisch; `with_backend`-Signatur konsistent mit `engine.rs:77`; `CodeIntelBackend::call`-Signatur konsistent mit `backend.rs:31`.
