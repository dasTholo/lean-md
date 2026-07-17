@lean-md
consumer: ai
crp: compact

@var test_cmd default="cargo nextest run" desc="project test runner command"
@var lint_cmd default="cargo clippy --all-targets -- -D warnings" desc="project lint gate"
@import .lean-ctx/lean-md/plan-recipes /

# lean-md scheitert leise — Implementation Plan

Spec: `docs/lean-md/specs/2026-07-17-lmd-scheitert-leise-design.md` (approved).

## Goal

Jeder der fünf Defekte ist selbst-verschleiernd: das Tool tut etwas anderes als dokumentiert
und meldet nichts. Dieses Paket zieht in jeden Fall eine Instanz ein, die den Widerspruch
bemerkt — Provenienz für Seeds (P8), ein echter Erweiterungspunkt statt eines Sonderpfads (P5),
ein deklaratives Arg-Schema, das `check` und Renderer teilen (P1), ein Duplikat-Check im Parser
(P2), eine Spannen-Prüfung gegen `ctxpkg.lock` (P9).

## Architecture

Reine **Binary**-Linie: `src/**` + `content/**` (via `include_str!`). Kein Task fasst
`content/skills/**` an. Die Seeds hängen als `include_str!` in `seeds.rs::PROJECT_SEEDS` und
werden nach `<project_root>/.lean-ctx/lean-md/` materialisiert; der neue Lock liegt eine Ebene
darüber in `<project_root>/.lean-ctx/lean-md.lock`, damit `sha256sum -c` mit relativen Pfaden
läuft.

Neue Module: `src/hashx.rs` (SHA-256, single source), `src/lock.rs` (Lock-Format),
`src/arg_schema.rs` (Directive-Arg-Schema), `src/version_gate.rs` (`version_req`-Prüfung).

## Global Constraints

- **Non-goal:** kein Publish, kein Tag, kein `pack create/export/publish`, kein
  Addon-Republish. Keine Versionsnummer wird angefasst — weder `Cargo.toml`-`version`, noch
  `lean-ctx-addon.toml`-`version`, noch `content/skills.ctxpkg-hash`.
- **Non-goal:** `content/skills/**` bleibt unberührt. `pack_drift` ist **grün** und bleibt grün
  — `bace97a` war ein Bless, kein Rot-Setzen (verifiziert 2026-07-17 im Review von task-1: bei
  `a7e722a` 2/2 PASS). Ein rotes Gate markiert **nicht** den Vorpaket-Zustand; wer diese
  Constraint als Erwartungswert benutzt hat, hat sie falsch gelesen.
- **Non-goal (mit einer Ausnahme):** die stale Seeds im Dev-Repo werden **nicht** von Hand
  geradegezogen; das Paket liefert den Mechanismus, nicht das Aufräumen. **Ausnahme, vom
  Menschen am 2026-07-17 entschieden:** `.lean-ctx/lean-md/dispatch-contract.ext.lmd.md` wird
  auf die HTML-Form der Template-Datei gezogen. Grund: der Non-Goal und das „Expected" von
  task-5 waren zusammen unerfüllbar — die `#`-Zeilen hingen sonst weiter in jedem Dispatch.
  Betrifft **nur** diese eine Datei; die übrigen stale Seeds bleiben dem Mechanismus überlassen.
- **Reihenfolge ist Architektur:** P8 (task-1…task-4) landet vor P5 (task-5). P5
  generalisiert den `.ext`-Pfad auf jedes Fragment; ohne P8 vervielfacht es den Bug, den es
  beheben soll.
- **#498 als Testgate:** jeder Output bleibt deterministische Funktion von (Dateiinhalt, Mode,
  CRP, Task). Unveränderte/inerte Seeds dürfen den Render-Output **byte-identisch** lassen.
- **D-1-Purity:** `render` und `check` bleiben lesend. Nur `lean-md mcp` (Serverstart) und
  `skill install` schreiben Seeds/Lock. Beweis je Task: Dateizustand vor == nach.
- **Fragment-Consistency-Gate** (built-in == on-disk seed) bleibt grün.
- **Prerequisite:** task-1 (`sha2` im Release-Profil + `sha256_hex` in der lib) landet vor
  task-2; ohne den Umzug kompiliert der Lock-Code nicht.
- **Prerequisite:** task-5 registriert zwei neue Seeds — task-2/task-3 müssen sie ohne Änderung
  aufnehmen (ein nachträglich hinzugekommener Seed materialisiert absent-only, ohne `.new`).

@phase "task-1"
## Task 1: `sha2` ins Release-Profil, `sha256_hex` als single source

**Warum:** das Release-Binary trägt heute keinen SHA-256-Code (`sha2` steht unter
`[dev-dependencies]`, einziger Nutzer `tests/pack_drift.rs`). Ohne den Umzug kann P8 den Lock
zur Laufzeit nicht schreiben. Zwei Definitionen von „wie hashen wir" wären genau die Drift, die
dieses Paket bekämpft — deshalb wandert die Hash-Fn in die lib und `pack_drift` nutzt sie.

**Files:** `Cargo.toml` (~), `src/hashx.rs` (neu), `src/lib.rs` (~ Modul-Deklaration),
`tests/pack_drift.rs` (~).

**Interfaces:** Produces `pub fn lean_md::hashx::sha256_hex(bytes: &[u8]) -> String` —
lowercase hex, 64 Zeichen, identisch zu `sha256sum`.

**Consumes:** `render_manifest()` @`tests/pack_drift.rs:57-78` (heute eigene `Sha256`-Kopie in
`:71-74`), `[dependencies]`/`[dev-dependencies]` @`Cargo.toml:20-37`.

Cargo.toml — `sha2` wandert von dev nach regulär:

    [dependencies]
    rushdown = "0.18"
    evalexpr = "13.1"
    serde_json = "1.0"
    regex = "1.13"
    chrono = "0.4"
    sha2 = "0.11"

    [dev-dependencies]
    tiktoken-rs = "0.12"

Neues Modul `src/hashx.rs` (verbatim — existiert noch nicht):

    //! SHA-256 hex digests — the single source for every hash lean-md writes or
    //! compares (the `lean-md.lock` provenance values and the pack-drift manifest).
    //! Two definitions of "how we hash" would be exactly the drift this module exists
    //! to make loud. Output matches coreutils `sha256sum` byte for byte, so a user can
    //! re-check any value we emit without trusting us.

    use sha2::{Digest, Sha256};

    /// Lowercase 64-char hex digest of `bytes`.
    pub fn sha256_hex(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn matches_the_known_sha256_of_abc() {
            // NIST FIPS 180-4 test vector — proves we produce what `sha256sum` produces.
            assert_eq!(
                sha256_hex(b"abc"),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            );
        }

        #[test]
        fn empty_input_has_the_canonical_digest() {
            assert_eq!(
                sha256_hex(b""),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            );
        }
    }

`src/lib.rs`: `pub mod hashx;` in alphabetischer Nachbarschaft der bestehenden
Modul-Deklarationen einfügen.

@call patch("src/lib.rs", "pub mod hashx; neben den bestehenden pub-mod-Zeilen")

`tests/pack_drift.rs`: `use sha2::{Digest, Sha256};` (`:17`) entfällt; der Hash-Block in
`render_manifest` (`:71-74`) wird ersetzt:

    let hex = lean_md::hashx::sha256_hex(&bytes);
    out.push_str(&format!("{hex}  {rel}\n"));

Neuer Test in `tests/pack_drift.rs` — beide Wege hashen nachweislich identisch:

    #[test]
    fn manifest_hash_uses_the_library_single_source() {
        // The gate and the runtime lock must never disagree on "how we hash".
        let bytes = b"lean-md drift probe";
        let mut h = sha2::Sha256::new();
        sha2::Digest::update(&mut h, bytes);
        let local: String = sha2::Digest::finalize(h).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(local, lean_md::hashx::sha256_hex(bytes));
    }

> Der Test braucht `sha2` weiterhin im Test-Scope — das liefert jetzt `[dependencies]`.

@call tdd(matches_the_known_sha256_of_abc)

**Expected:** `cargo nextest run --test pack_drift` verhält sich exakt wie vorher — das Gate
bleibt grün, weil der Refactor `content/skills/**` und das Manifest nicht anfasst. Kein
Verhaltenswechsel am bestehenden Gate.

### Verify & Close

@call verify("Cargo.toml src/hashx.rs src/lib.rs tests/pack_drift.rs")
@call gate("Cargo.toml src/hashx.rs src/lib.rs tests/pack_drift.rs")
@call commit("Cargo.toml src/hashx.rs src/lib.rs tests/pack_drift.rs", "refactor(hash): sha2 ins Release-Profil, sha256_hex als single source")
@call remember_decision("lean_md::hashx::sha256_hex ist die einzige Hash-Definition — Lock und pack_drift nutzen sie beide; sha2 steht in [dependencies], nicht dev")
@phase-end

@phase "task-2"
## Task 2: `lean-md.lock` — Format, Schreiben, Lesen

@call recall_context("sha256_hex single source, sha2 in [dependencies]")

**Warum:** `materialize_contracts` sieht nur „lokal ≠ embedded" und kann daraus nicht ableiten,
ob der Nutzer editiert hat oder ob der Seed weitergezogen ist. Der Lock konserviert den
**historischen** Seed-Hash — den ein neueres Binary nicht mehr trägt. Ohne Hash keine
Provenienz, ohne Provenienz kein dritter Modus.

**Files:** `src/lock.rs` (neu), `src/lib.rs` (~), `docs/dev-readme.md` (~ eine Tabellenzeile).

**Interfaces:** Produces

    pub const LOCK_REL: &str = ".lean-ctx/lean-md.lock";
    #[derive(Default)]
    pub struct Lock { entries: Vec<(String, String)> }   // (rel-to-.lean-ctx path, hex)
    impl Lock {
        pub fn load(project_root: &Path) -> Lock;              // absent → leer, nie Fehler
        pub fn get(&self, rel: &str) -> Option<&str>;
        pub fn set(&mut self, rel: &str, hex: &str);
        pub fn render(&self) -> String;                        // sha256sum-Format
        pub fn save(&self, project_root: &Path) -> std::io::Result<()>;
    }

**Consumes:** `sha256_hex` (task-1), `PROJECT_SEEDS` @`src/seeds.rs:24-42`,
Manifest-Renderer als Formatvorbild @`tests/pack_drift.rs:63-77`.

**Format — `sha256sum`, nicht TOML** (Spec Entscheidung 3): `<hex>␠␠<relpath>`, `#`-Kommentare,
Pfade **relativ zu `.lean-ctx/`**. Die Semantik von `sha256sum -c` *ist* die Provenienz-Frage,
also beantwortet der Nutzer sie selbst — ohne uns glauben zu müssen.

    # lean-md.lock — generated by lean-md; commit this file.
    # binary_version: 0.2.0
    # Eigene Anpassungen prüfen:  cd .lean-ctx && sha256sum -c lean-md.lock
    ad75963…  lean-md/lang/rust.lmd.md

`binary_version` kommt aus `env!("CARGO_PKG_VERSION")` und ist ein `#`-Kommentar — Metadatum,
kein Kernstück.

> **Erster Test der Runde — die unverifizierte Annahme der Spec:** GNU coreutils
> `sha256sum --check` ignoriert `#`-Zeilen. Nicht ausgeführt worden. Trägt sie nicht, wandert
> `binary_version` in einen Sidecar oder entfällt; das `sha256sum`-Format bleibt.

Test zuerst (verbatim) — Selbstprüfbarkeit gegen echtes coreutils:

    #[test]
    fn lock_is_checkable_by_coreutils_sha256sum() {
        // The whole reason SHA-256 was chosen over a dep-free hash: the user must be able
        // to re-check every value with a standard command. If coreutils cannot read our
        // file, the format has failed its only job. Also verifies the spec's unverified
        // assumption that `#` lines are ignored by --check.
        let root = std::env::temp_dir().join(format!("lmd_lock_c14n_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join(".lean-ctx/lean-md");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("probe.lmd.md"), "probe content\n").unwrap();

        let mut lock = Lock::load(&root);
        lock.set(
            "lean-md/probe.lmd.md",
            &crate::hashx::sha256_hex(b"probe content\n"),
        );
        lock.save(&root).unwrap();

        let out = std::process::Command::new("sha256sum")
            .arg("-c")
            .arg("lean-md.lock")
            .current_dir(root.join(".lean-ctx"))
            .output();
        let Ok(out) = out else {
            eprintln!("sha256sum unavailable — skipping coreutils cross-check");
            let _ = std::fs::remove_dir_all(&root);
            return;
        };
        assert!(
            out.status.success(),
            "coreutils rejected our lock: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );

        // An edited seed must read as FAILED — that is the provenance question itself.
        std::fs::write(dir.join("probe.lmd.md"), "user edit\n").unwrap();
        let out = std::process::Command::new("sha256sum")
            .arg("-c")
            .arg("lean-md.lock")
            .current_dir(root.join(".lean-ctx"))
            .output()
            .unwrap();
        assert!(!out.status.success(), "edited seed must fail sha256sum -c");

        let _ = std::fs::remove_dir_all(&root);
    }

**Expected (rot):** `Lock` existiert nicht → compile error.

Weitere Tests, verbatim:

    #[test]
    fn absent_lock_loads_empty_and_is_not_an_error() {
        let root = std::env::temp_dir().join(format!("lmd_lock_absent_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let lock = Lock::load(&root);
        assert_eq!(lock.get("lean-md/lang/rust.lmd.md"), None);
    }

    #[test]
    fn lock_round_trips_and_paths_are_relative_to_lean_ctx() {
        let root = std::env::temp_dir().join(format!("lmd_lock_rt_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx")).unwrap();
        let mut lock = Lock::load(&root);
        lock.set("lean-md/lang/rust.lmd.md", "deadbeef");
        lock.save(&root).unwrap();

        let raw = std::fs::read_to_string(root.join(".lean-ctx/lean-md.lock")).unwrap();
        assert!(
            raw.contains("deadbeef  lean-md/lang/rust.lmd.md"),
            "sha256sum format (two spaces), path relative to .lean-ctx: {raw}"
        );
        assert!(raw.contains("# binary_version: "), "binary_version comment missing");
        assert!(
            !raw.contains(".lean-ctx/lean-md/lang"),
            "paths must NOT be relative to project_root — sha256sum -c runs in .lean-ctx"
        );
        assert_eq!(Lock::load(&root).get("lean-md/lang/rust.lmd.md"), Some("deadbeef"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn lock_render_is_byte_stable() {
        // #498: same entries → same bytes, regardless of insertion order.
        let mut a = Lock::default();
        a.set("lean-md/b.lmd.md", "22");
        a.set("lean-md/a.lmd.md", "11");
        let mut b = Lock::default();
        b.set("lean-md/a.lmd.md", "11");
        b.set("lean-md/b.lmd.md", "22");
        assert_eq!(a.render(), b.render());
    }

Implementierung: `render()` sortiert die Einträge nach Pfad (Byte-Ordnung, wie
`render_manifest` in `pack_drift.rs:61`); `load()` überliest Zeilen mit führendem `#` und
leere Zeilen, splittet am ersten `"  "`; `save()` legt `.lean-ctx/` bei Bedarf an.

@call tdd(absent_lock_loads_empty_and_is_not_an_error)

**Tabellenlücke im dev-readme mitschließen:** die Zuordnungstabelle listet `content/core/**`,
`content/gloss/**` und `src/**` als Binary — **`content/templates/**` fehlt**, obwohl dort der
`.ext`-Seed und `plan-recipes`/`plan-template` liegen, alle drei via `include_str!`. Der Lock
enumeriert genau diese Seeds; die Lücke lädt sonst die nächste Spec zur selben Kollision ein.

@call patch("docs/dev-readme.md", "Zeile `content/templates/**` → Binary in die Zuordnungstabelle, neben content/core/**")

### Verify & Close

@call verify("src/lock.rs src/lib.rs docs/dev-readme.md")
@call gate("src/lock.rs src/lib.rs docs/dev-readme.md")
@call commit("src/lock.rs src/lib.rs docs/dev-readme.md", "feat(lock): lean-md.lock im sha256sum-Format (Seed-Provenienz)")
@call remember_decision("lean-md.lock: sha256sum-Format, Pfade relativ zu .lean-ctx/, binary_version als #-Kommentar; von coreutils sha256sum -c prüfbar (verifiziert)")
@phase-end

@phase "task-3"
## Task 3: `materialize_contracts` — dritter Modus (lock-basierter Refresh)

@call recall_context("lean-md.lock Format und Lock-API")

**Warum:** beide bestehenden Modi sind falsch. Absent-only lässt Seeds altern (das ist der
Mechanismus, der die vier stale Dateien im Dev-Repo erzeugt hat); `force` überschreibt echte
Anpassungen. Der Lock erlaubt erstmals, die Fälle zu trennen.

**Files:** `src/seeds.rs` (~).

**Interfaces:** Produces

    pub struct RefreshReport {
        pub healed: Vec<PathBuf>,     // stale + untouched → silently updated
        pub preserved: Vec<PathBuf>,  // user-edited → `.new` written beside it
    }
    impl RefreshReport { pub fn is_quiet(&self) -> bool }   // nichts zu melden

    /// Third mode beside absent-only and `force`.
    pub fn refresh_contracts(
        project_root: &Path,
        contracts_dir: &str,
    ) -> std::io::Result<RefreshReport>;

`materialize_contracts` (@`src/seeds.rs:41-63`) bleibt **unverändert** — `force` bleibt der
bewusste Holzhammer, absent-only bleibt der Install-Default für neue Ziele.

**Consumes:** `PROJECT_SEEDS` @`src/seeds.rs:24-42`, `Lock` (task-2), `sha256_hex` (task-1).

**Refresh-Semantik, drei Fälle** (Spec Entscheidung 3):

| lokal vs. lock | lock vs. embedded | Bedeutung            | Aktion                                          |
|----------------|-------------------|----------------------|-------------------------------------------------|
| gleich         | verschieden       | alt, unberührt       | still aktualisieren, Lock nachziehen            |
| verschieden    | —                 | **Nutzer-Anpassung** | **nie** überschreiben → `.new` daneben + melden |
| —              | gleich            | aktuell              | no-op                                           |

**Altbestand ohne Lock-Eintrag** (der heutige Zustand, 4 Dateien): Provenienz unbekannt →
konservativ `.new` + Meldung. Fehlt das Ziel ganz, wird es geschrieben und der Lock gesetzt
(absent-only-Verhalten, kein `.new`) — das ist zugleich der Pfad für einen Seed, den ein
späterer Task neu registriert (task-5).

Tests zuerst (verbatim, alle vier Fälle):

    #[test]
    fn refresh_heals_a_stale_untouched_seed_silently() {
        let root = std::env::temp_dir().join(format!("lmd_refresh_heal_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        // First refresh writes the lock for the pristine tree.
        refresh_contracts(&root, dir).unwrap();

        // Simulate "the embedded seed moved on": pin an OLD hash in the lock and put the
        // matching old content on disk. Local == lock → untouched → may heal.
        let target = root.join(dir).join("plan-recipes.lmd.md");
        let old = "# an older embedded copy\n";
        std::fs::write(&target, old).unwrap();
        let mut lock = crate::lock::Lock::load(&root);
        lock.set("lean-md/plan-recipes.lmd.md", &crate::hashx::sha256_hex(old.as_bytes()));
        lock.save(&root).unwrap();

        let report = refresh_contracts(&root, dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            PLAN_RECIPES,
            "stale + untouched must heal to the embedded seed"
        );
        assert!(report.healed.iter().any(|p| p.ends_with("plan-recipes.lmd.md")));
        assert!(report.preserved.is_empty(), "no .new for an untouched seed");
        assert!(!target.with_extension("md.new").exists(), "must not litter a .new");
        // The lock followed along, so the next run is a no-op.
        assert!(refresh_contracts(&root, dir).unwrap().is_quiet());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refresh_never_overwrites_a_user_edit_and_writes_new_beside_it() {
        let root = std::env::temp_dir().join(format!("lmd_refresh_edit_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        refresh_contracts(&root, dir).unwrap(); // lock now matches the pristine tree

        let target = root.join(dir).join("lang/rust.lmd.md");
        let edit = "# my project rule\n";
        std::fs::write(&target, edit).unwrap(); // local != lock → user edit

        let report = refresh_contracts(&root, dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            edit,
            "a user edit must NEVER be clobbered"
        );
        let new_file = root.join(dir).join("lang/rust.lmd.md.new");
        assert!(new_file.exists(), ".new must be written beside the edited seed");
        assert!(report.preserved.iter().any(|p| p.ends_with("rust.lmd.md")));
        assert!(report.healed.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn legacy_tree_without_a_lock_is_treated_conservatively() {
        // Today's state: 4 stale seeds, no lock, provenance unknown. We must not guess
        // "untouched" — that would clobber whatever the user did before locks existed.
        let root = std::env::temp_dir().join(format!("lmd_refresh_legacy_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        let target = root.join(dir).join("tooling/mcp-tools.lmd.md");
        std::fs::write(&target, "# a pre-lock local copy\n").unwrap();
        assert!(!root.join(".lean-ctx/lean-md.lock").exists());

        let report = refresh_contracts(&root, dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "# a pre-lock local copy\n",
            "unknown provenance must never be overwritten"
        );
        assert!(root.join(dir).join("tooling/mcp-tools.lmd.md.new").exists());
        assert!(report.preserved.iter().any(|p| p.ends_with("mcp-tools.lmd.md")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refresh_of_a_current_tree_is_a_silent_noop() {
        let root = std::env::temp_dir().join(format!("lmd_refresh_noop_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        refresh_contracts(&root, dir).unwrap();

        let report = refresh_contracts(&root, dir).unwrap();
        assert!(report.is_quiet(), "a current tree must produce no report at all");
        assert!(report.healed.is_empty() && report.preserved.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_newly_registered_seed_materializes_without_a_new_file() {
        // task-5 adds two seeds AFTER locks exist in the field. An absent target is not a
        // user edit — it must just appear, silently.
        let root = std::env::temp_dir().join(format!("lmd_refresh_fresh_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        materialize_contracts(&root, dir, false).unwrap();
        refresh_contracts(&root, dir).unwrap();

        let (rel, _) = PROJECT_SEEDS[0];
        std::fs::remove_file(root.join(dir).join(rel)).unwrap();
        let report = refresh_contracts(&root, dir).unwrap();
        assert!(root.join(dir).join(rel).exists(), "absent seed must be (re)written");
        assert!(report.preserved.is_empty(), "an absent target is not a user edit");
        assert!(!root.join(dir).join(format!("{rel}.new")).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

@call tdd(refresh_heals_a_stale_untouched_seed_silently)

**Doc-Kommentar auf die Wahrheit ziehen:** `src/seeds.rs:1-6` behauptet „project file overrides
the embedded seed". Für die drei Built-ins ist das falsch (`resolve()` returned früh,
`fragments.rs:59-61`); für `lang/rust`, `tooling/mcp-tools`, `plan-recipes` stimmt „override"
nur zufällig — dort existiert gar kein Built-in. Der Kommentar beschreibt jetzt die drei Modi
und verweist für die Auflösungsordnung auf `fragments.rs`; die `.ext`-Aussage zieht task-5 nach.

@call patch("src/seeds.rs", "Modul-Doc-Kommentar Zeile 1-6: drei Modi statt der falschen override-Behauptung")

### Verify & Close

@call verify("src/seeds.rs")
@call gate("src/seeds.rs")
@call review_change()
@call commit("src/seeds.rs", "feat(seeds): lock-basierter Refresh — stale heilt still, Nutzer-Edit bekommt .new")
@call remember_decision("refresh_contracts ist der dritte Modus neben absent-only und force; RefreshReport{healed,preserved}; Altbestand ohne Lock → konservativ .new")
@phase-end

@phase "task-4"
## Task 4: Wiring — MCP-Start refresht, `check` meldet, `render` bleibt lesend

@call recall_context("refresh_contracts und RefreshReport")

**Warum:** ein Endnutzer hat nur ctxpkg — er fährt `addon update`, der Pack wird getauscht,
`install_skill` läuft nie. Das Manifest kennt **keine Lifecycle-Phase und keinen Install-Hook**
(`lean-ctx-addon.toml`: nur `[artifacts]`/`[dependencies]`/`[mcp]`/`[capabilities]`), also gibt
es keine Stelle, an der lean-ctx unseren Code bei `addon update` ausführt. Der Serverstart ist
die einzige Stelle, die in jeder Session greift.

**Files:** `src/bin/lean_md.rs` (~ `cmd_mcp`, `do_check`, `cmd_check`), `src/skill_install.rs`
(~ `install_skill`).

**Consumes:** `cmd_mcp` @`src/bin/lean_md.rs:450-600`, `do_check` @`src/bin/lean_md.rs:50-60`,
`cmd_check` @`src/bin/lean_md.rs:247-254`, `install_skill` @`src/skill_install.rs:85-138`
(Aufruf `materialize_contracts(project_root, ".lean-ctx/lean-md", force)` @`:136`),
`refresh_contracts` (task-3).

**Sichtbarkeit — bewusst asymmetrisch** (Spec Entscheidung 4): beim MCP-Start ist `stdout` der
JSON-RPC-Kanal (eine Warnung dort korrumpiert das Protokoll) und `stderr` landet im
Gateway-Log, das der Agent nie sieht. Also: der Normalfall (stale + unberührt) heilt **still**;
nur der `.new`-Fall braucht Sichtbarkeit, und die kommt über `lean-md check`, wo der Nutzer
hinschaut. `stderr` bleibt zusätzlich für die Log-Diagnose.

**`project_root`:** abgeleitet wie überall sonst im Binary (der Jail-Root, gegen den auch
Fragmente aufgelöst werden). Findet sich keiner, entfällt der Seed-Teil lautlos; die
Datei-Prüfung läuft unverändert.

Neue Interfaces:

    // src/bin/lean_md.rs — check-scope helper
    fn seed_report_line(project_root: &std::path::Path) -> Option<String>;

`do_check` (heute nur Parse-Zähler, `:50-60`) bekommt den Seed-Teil als zusätzliche Zeile;
`cmd_check` reicht den `project_root` durch. `cmd_mcp` ruft `refresh_contracts` **einmal vor
der Read-Loop** (`:457`), verwirft `healed` still und schreibt für `preserved` eine Zeile nach
`stderr`. `install_skill` fährt denselben Modus: `materialize_contracts(.., force)` bleibt für
`force=true`, für `force=false` tritt `refresh_contracts` an seine Stelle.

Tests zuerst (verbatim):

    #[test]
    fn mcp_start_refreshes_seeds_but_render_and_check_do_not_write() {
        // D-1 purity: the renderer stays PURE. The wiring sits at server start, not on the
        // hot path. Proof: file state before == after for render/check.
        let root = std::env::temp_dir().join(format!("lmd_wire_pure_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        lean_md::seeds::materialize_contracts(&root, dir, false).unwrap();
        lean_md::seeds::refresh_contracts(&root, dir).unwrap();

        let target = root.join(dir).join("plan-recipes.lmd.md");
        std::fs::write(&target, "# stale untouched\n").unwrap();
        let mut lock = lean_md::lock::Lock::load(&root);
        lock.set(
            "lean-md/plan-recipes.lmd.md",
            &lean_md::hashx::sha256_hex(b"# stale untouched\n"),
        );
        lock.save(&root).unwrap();

        // render must not heal it …
        let _ = do_render("@lean-md\nconsumer: ai\n\nhi\n", root.clone(), None, None);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "# stale untouched\n");
        // … nor must check.
        let _ = do_check("@lean-md\nconsumer: ai\n\nhi\n", Some(&root));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "# stale untouched\n");

        // The MCP start path does.
        lean_md::seeds::refresh_contracts(&root, dir).unwrap();
        assert_ne!(std::fs::read_to_string(&target).unwrap(), "# stale untouched\n");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn check_reports_the_new_case_and_stays_silent_on_a_healed_one() {
        let root = std::env::temp_dir().join(format!("lmd_wire_check_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        lean_md::seeds::materialize_contracts(&root, dir, false).unwrap();
        lean_md::seeds::refresh_contracts(&root, dir).unwrap();

        // Current tree → check says nothing about seeds.
        let quiet = do_check("@lean-md\nconsumer: ai\n\nhi\n", Some(&root));
        assert!(!quiet.contains(".new"), "a current tree must not be reported: {quiet}");

        // A user edit + a refresh → .new exists → check must surface it, because stderr at
        // MCP start is a log the agent never reads.
        std::fs::write(root.join(dir).join("lang/rust.lmd.md"), "# mine\n").unwrap();
        lean_md::seeds::refresh_contracts(&root, dir).unwrap();
        let loud = do_check("@lean-md\nconsumer: ai\n\nhi\n", Some(&root));
        assert!(
            loud.contains("lang/rust.lmd.md.new"),
            "check must name the .new file: {loud}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn check_without_a_project_root_still_checks_the_file() {
        let out = do_check("@lean-md\nconsumer: ai\n\nhi\n", None);
        assert!(out.contains("lmd ok"), "the file check must survive a missing root: {out}");
    }

@call tdd(check_without_a_project_root_still_checks_the_file)

`src/skill_install.rs` — der Aufruf @`:136` wird auf den sicheren Modus gestellt; der
Kommentar `:132-135` beschreibt danach die drei Modi statt „absent-only unless force".

@call patch("src/skill_install.rs", "materialize_contracts-Aufruf Zeile 136: refresh_contracts für force=false, materialize_contracts(force=true) bleibt der Holzhammer")

**Expected:** `cargo nextest run` grün. Der Dev-Repo-Zustand danach: der nächste MCP-Start
heilt `lang/rust`, `tooling/mcp-tools`, `plan-template` und den `dispatch-contract.ext` still
(alle vier sind unberührte alte Kopien) — nicht Teil dieses Tasks, aber die erwartete Wirkung.

### Verify & Close

@call verify("src/bin/lean_md.rs src/skill_install.rs")
@call gate("src/bin/lean_md.rs src/skill_install.rs")
@call review_change()
@call commit("src/bin/lean_md.rs src/skill_install.rs", "feat(mcp): Seed-Refresh beim Serverstart; check meldet den .new-Fall")
@call remember_decision("Refresh-Wiring sitzt am MCP-Serverstart (kein Install-Hook im Addon-Manifest); render/check bleiben lesend; der .new-Fall wird über lean-md check sichtbar, nicht über stderr")
@phase-end

@phase "task-5"
## Task 5: P5 — `.ext` generisch in der Registry, Sonderpfad-Rückbau

@call recall_context("Refresh-Wiring und Lock — P8 ist vollstaendig")

**Warum jetzt und nicht früher:** der Sonderpfad `contract_ext` ist **bereits aktiv** — die
stale `dispatch-contract.ext` trägt `#`-Zeilen, die `strip_html_comments` nicht entfernt, also
überlebt sie die Inert-Prüfung und hängt heute in jedem Dispatch-Contract dieses Repos. #498 ist
im Dev-Repo schon gekippt. Landete P5 vor P8, vervielfachte dieser Task den Bug auf jedes
Fragment.

**Files:** `src/fragments.rs` (~), `src/bridges/dispatch.rs` (~ Rückbau), `src/seeds.rs` (~ zwei
neue `PROJECT_SEEDS`), `content/templates/hard-rules.ext.lmd.md` (neu),
`content/templates/parallel-dispatch.ext.lmd.md` (neu).

> Ablage neben dem Vorbild `content/templates/dispatch-contract.ext.lmd.md` (@`src/seeds.rs:31`)
> — nicht unter `content/core/`, wo die Built-ins liegen. Damit deckt die
> `content/templates/**`-Tabellenzeile aus task-2 alle drei `.ext`-Seeds ab, statt einen dritten
> Sonderfall aufzumachen.

**Interfaces:** `FragmentRegistry::resolve` (@`src/fragments.rs:56-77`) behält seine Signatur
und komponiert neu:

    base = builtin | SKILL_INCLUDES | jailed <name>.lmd.md
    ext  = jailed <name>.ext.lmd.md
    ret  = base + ext            (skip-if-inert → byte-stabil, #498)

`contract_ext` (@`src/bridges/dispatch.rs:31-39`) und `strip_html_comments` (@`:42-54`) wandern
in die Registry; die Bridge liest keine Datei mehr selbst — der Block @`:110-115` entfällt
ersatzlos, weil `resolve("dispatch-contract", …)` die `.ext` jetzt mitbringt. Die Komposition
bleibt **vor** `render_body`, damit die `.ext` an Placeholder-Substitution und
`@include`-Auflösung teilnimmt.

**Pfad-Detail — der `.ext`-Lookup geht in `contracts_dir`, NICHT an den Jail-Root.** Die beiden
Pfade sind verschieden und dürfen nicht verwechselt werden:

- Fragment-File-Fallback (@`src/fragments.rs:67`): `jail_root.join("{name}.lmd.md")` — Projekt-Root.
- `.ext` (@`src/bridges/dispatch.rs:32`): `jail_root.join(".lean-ctx/lean-md/{name}.ext.lmd.md")`
  — `contracts_dir`, und genau dorthin materialisiert `materialize_contracts` (@`src/seeds.rs:41-60`).

Der `.ext`-Lookup übernimmt den **contracts_dir-Pfad der Bridge** unverändert. Am Jail-Root
gesucht wären `hard-rules.ext`/`parallel-dispatch.ext` genau die toten Dateien, die dieser Task
beseitigen soll, und `dispatch-contract.ext` verlöre seine heutige Wirkung (Regression gegen
den P4-Fix des Vorpakets).

    const CONTRACTS_DIR: &str = ".lean-ctx/lean-md";

    /// Project extension for `name`, composed onto the resolved base. `None` when absent
    /// or inert. Lives in `contracts_dir` (where the seeds materialise), NOT at jail_root
    /// like the fragment file fallback — the two paths are deliberately different.
    fn ext(&self, name: &str, jail_root: &Path) -> Option<String> {
        let candidate = jail_root.join(format!("{CONTRACTS_DIR}/{name}.ext.lmd.md"));
        let resolved = crate::pathx::jail_path(&candidate, jail_root).ok()?;
        let raw = std::fs::read_to_string(&resolved).ok()?;
        (!strip_html_comments(&raw).trim().is_empty()).then_some(raw)
    }

`pathx::jail_path` bleibt der Guard, damit ein Symlink nach draußen weiterhin scheitert.

**Bestehende Tests, die den Umzug mitmachen** — sie leben heute im Testmodul von
`src/bridges/dispatch.rs` (@`:413-503`) und beweisen den Pfad, den dieser Task generalisiert:
`ext_fixture` (@`:416-428`, schreibt nach `.lean-ctx/lean-md/`), `dispatch_ext_rule_appears_after_contract`
(@`:441-456`), `dispatch_untouched_ext_seed_is_byte_stable` (@`:458-473`),
`dispatch_absent_ext_is_unchanged` (@`:475-489`), `dispatch_ext_jail_escape_still_rejected`
(@`:491-503`). Sie **müssen grün bleiben** — sie sind der Regressionsschutz gegen genau die
Pfad-Verwechslung. Wandern sie mit der Logik nach `src/fragments.rs`, behalten sie ihre
Fixture-Pfade unverändert.

**Abgrenzung (bewusst):** `.ext` gilt nur für die **drei Built-ins**. `lang/rust` und
`tooling/mcp-tools` haben keinen Built-in — ihre materialisierte Datei **ist** die Quelle und
wird direkt editiert; ein `.ext` wäre dort ein zweiter Weg zum selben Ziel.

Zwei neue Seeds, verbatim (reiner HTML-Kommentar — sonst nicht inert):

`content/templates/hard-rules.ext.lmd.md`:

    <!-- Hard-rules extension (project seed).
         Auto-composed after the built-in hard-rules fragment.
         Add project-specific tool-discipline rules below. Empty by default. -->

`content/templates/parallel-dispatch.ext.lmd.md`:

    <!-- Parallel-dispatch extension (project seed).
         Auto-composed after the built-in parallel-dispatch fragment.
         Add project-specific fan-out rules below. Empty by default. -->

`PROJECT_SEEDS` (@`src/seeds.rs:24-42`) bekommt beide Einträge nach dem Muster von
`dispatch-contract.ext.lmd.md`:

    (
        "hard-rules.ext.lmd.md",
        include_str!("../content/templates/hard-rules.ext.lmd.md"),
    ),
    (
        "parallel-dispatch.ext.lmd.md",
        include_str!("../content/templates/parallel-dispatch.ext.lmd.md"),
    ),

> Ohne die Seeds bliebe der Gewinn theoretisch: ein Nutzer müsste `hard-rules.ext.lmd.md`
> erfinden und wüsste nicht einmal, dass es sie geben kann. Ein Erweiterungspunkt, den niemand
> entdeckt, ist keiner.

Tests zuerst (verbatim). Die Fixture schreibt in `contracts_dir` — **nicht** an den Jail-Root;
ein Test, der die `.ext` woanders ablegt, prüft einen Pfad, den es nicht gibt:

    /// Jail root with an optional `<name>.ext.lmd.md` in contracts_dir — the same layout
    /// `materialize_contracts` produces and `ext_fixture` (bridges/dispatch.rs) uses.
    fn ext_root(tag: &str, name: &str, body: Option<&str>) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("lmd_ext_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let contracts = dir.join(".lean-ctx/lean-md");
        std::fs::create_dir_all(&contracts).unwrap();
        if let Some(b) = body {
            std::fs::write(contracts.join(format!("{name}.ext.lmd.md")), b).unwrap();
        }
        dir
    }

    #[test]
    fn ext_composes_onto_any_builtin_fragment() {
        // hard-rules.ext is a dead file today — nothing reads it. That is the bug.
        let dir = ext_root("generic", "hard-rules", Some("PROJECT RULE: no cd\n"));
        let reg = FragmentRegistry::with_builtins();
        let out = reg.resolve("hard-rules", &dir).unwrap();
        assert!(out.contains("lean-ctx"), "built-in body must survive");
        assert!(out.contains("PROJECT RULE: no cd"), "ext must be appended: {out}");
        assert!(
            out.find("lean-ctx").unwrap() < out.find("PROJECT RULE").unwrap(),
            "ext comes AFTER the built-in"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ext_at_the_jail_root_is_ignored() {
        // Pins the path decision: the fragment file fallback lives at jail_root, the .ext
        // lives in contracts_dir. Reading the ext at jail_root would make every shipped
        // seed a dead file — the exact defect this task removes.
        let dir = ext_root("wrongpath", "hard-rules", None);
        std::fs::write(dir.join("hard-rules.ext.lmd.md"), "WRONG PLACE\n").unwrap();
        let reg = FragmentRegistry::with_builtins();
        assert!(!reg.resolve("hard-rules", &dir).unwrap().contains("WRONG PLACE"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hard_rules_ext_inherits_into_dispatch_contract_via_include() {
        // The actual lever of the generalisation: a project rule lives in ONE place and
        // reaches EVERY dispatch through the contract's `@include hard-rules` — with no
        // entry in dispatch-contract.ext. Must be proven, not assumed.
        let dir = ext_root("inherit", "hard-rules", Some("PROJECT RULE: inherited\n"));
        assert!(!dir.join(".lean-ctx/lean-md/dispatch-contract.ext.lmd.md").exists());

        let src = "@lean-md\nconsumer: ai\n\n@phase \"t\"\nwork\n@phase-end\n@dispatch phase=t\n";
        let out =
            crate::skills::render_source_with_phase(src, None, None, None, dir.clone()).unwrap();
        assert!(
            out.contains("PROJECT RULE: inherited"),
            "hard-rules.ext must reach the dispatch contract via @include hard-rules: {out}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_inert_ext_leaves_the_output_byte_identical() {
        // #498: an untouched seed must not change a single byte. This is the regression
        // guard against exactly what the stale dispatch-contract.ext does today.
        let seed = crate::seeds::PROJECT_SEEDS
            .iter()
            .find(|(p, _)| *p == "hard-rules.ext.lmd.md")
            .map(|(_, c)| *c)
            .expect("hard-rules.ext must be registered");
        let with_dir = ext_root("inert", "hard-rules", Some(seed));
        let without_dir = ext_root("inert_absent", "hard-rules", None);
        let reg = FragmentRegistry::with_builtins();
        assert_eq!(
            reg.resolve("hard-rules", &with_dir).unwrap(),
            reg.resolve("hard-rules", &without_dir).unwrap(),
            "the shipped hard-rules.ext seed must be inert"
        );
        let _ = std::fs::remove_dir_all(&with_dir);
        let _ = std::fs::remove_dir_all(&without_dir);
    }

    #[test]
    fn the_parallel_dispatch_seed_is_inert_too() {
        let seed = crate::seeds::PROJECT_SEEDS
            .iter()
            .find(|(p, _)| *p == "parallel-dispatch.ext.lmd.md")
            .map(|(_, c)| *c)
            .expect("parallel-dispatch.ext must be registered");
        let with_dir = ext_root("pd_inert", "parallel-dispatch", Some(seed));
        let without_dir = ext_root("pd_absent", "parallel-dispatch", None);
        let reg = FragmentRegistry::with_builtins();
        assert_eq!(
            reg.resolve("parallel-dispatch", &with_dir).unwrap(),
            reg.resolve("parallel-dispatch", &without_dir).unwrap()
        );
        let _ = std::fs::remove_dir_all(&with_dir);
        let _ = std::fs::remove_dir_all(&without_dir);
    }

    #[test]
    fn a_markdown_only_ext_is_not_inert() {
        // The live defect, pinned: `#` lines are headings, not comments — they survive
        // strip_html_comments and get appended. A seed that WANTS to be empty must say so
        // in HTML. This is why the shipped seeds are HTML comments.
        let dir = ext_root("md", "hard-rules", Some("# a heading\n"));
        let reg = FragmentRegistry::with_builtins();
        assert!(reg.resolve("hard-rules", &dir).unwrap().contains("# a heading"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ext_takes_part_in_placeholder_substitution() {
        let dir = ext_root("ph", "dispatch-contract", Some("role is {{ role }}\n"));
        let src = "@lean-md\nconsumer: ai\n\n@phase \"t\"\nwork\n@phase-end\n@dispatch phase=t role=review\n";
        let out =
            crate::skills::render_source_with_phase(src, None, None, None, dir.clone()).unwrap();
        assert!(out.contains("role is review"), "ext must see substitution: {out}");
        assert!(!out.contains("{{ role }}"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_ext_changes_nothing() {
        let dir = ext_root("absent", "hard-rules", None);
        let reg = FragmentRegistry::with_builtins();
        assert_eq!(
            reg.resolve("hard-rules", &dir).unwrap(),
            reg.resolve("hard-rules", Path::new(".")).unwrap()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ext_lookup_is_jailed() {
        // Both the base lookup and the new ext lookup run through pathx::jail_path.
        let reg = FragmentRegistry::with_builtins();
        let err = reg.resolve("../etc/passwd", Path::new(".")).unwrap_err();
        assert!(matches!(err, ResolveError::Jail(_)));
    }

    #[test]
    fn dispatch_bridge_reads_no_file_itself() {
        // The special path is gone: composition belongs to the registry, so it applies to
        // every fragment instead of exactly one name.
        let src = include_str!("bridges/dispatch.rs");
        assert!(!src.contains("fn contract_ext"), "contract_ext must move to the registry");
        assert!(!src.contains("read_to_string"), "the bridge must not do file I/O");
    }

    #[test]
    fn no_ext_seed_for_file_backed_fragments() {
        // lang/rust and tooling/mcp-tools have no built-in — their materialised file IS the
        // source and is edited directly. A second way to the same goal would be the bug.
        for (rel, _) in crate::seeds::PROJECT_SEEDS {
            assert!(
                !rel.starts_with("lang/") || !rel.contains(".ext."),
                "no .ext seed for a file-backed fragment: {rel}"
            );
            assert!(
                !rel.starts_with("tooling/") || !rel.contains(".ext."),
                "no .ext seed for a file-backed fragment: {rel}"
            );
        }
    }

    #[test]
    fn both_new_ext_seeds_are_registered() {
        let paths: Vec<&str> = crate::seeds::PROJECT_SEEDS.iter().map(|(p, _)| *p).collect();
        assert!(paths.contains(&"hard-rules.ext.lmd.md"));
        assert!(paths.contains(&"parallel-dispatch.ext.lmd.md"));
    }

**Kopplungstest** (P8 × P5 — die alte `.ext` mit `#`-Zeilen darf nicht länger unbemerkt
angehängt werden):

    #[test]
    fn a_stale_markdown_ext_is_caught_by_the_refresh_before_it_is_composed() {
        let root = std::env::temp_dir().join(format!("lmd_ext_couple_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = ".lean-ctx/lean-md";
        crate::seeds::materialize_contracts(&root, dir, false).unwrap();
        let ext = root.join(dir).join("dispatch-contract.ext.lmd.md");
        std::fs::write(&ext, "# lean-md dispatch-contract extension\n").unwrap(); // the live defect

        // No lock → unknown provenance → preserved + .new, never silently composed away.
        let report = crate::seeds::refresh_contracts(&root, dir).unwrap();
        assert!(report.preserved.iter().any(|p| p.ends_with("dispatch-contract.ext.lmd.md")));
        let _ = std::fs::remove_dir_all(&root);
    }

@call tdd(ext_composes_onto_any_builtin_fragment)

**Doc-Kommentare auf die Wahrheit ziehen** — `src/fragments.rs:1-3` behauptet „files
override/extend them"; für die drei Built-ins ist „override" schlicht falsch. Nach diesem Task
gilt: **extend**, nicht override.

@call patch("src/fragments.rs", "Modul-Doc-Kommentar Zeile 1-3: extend statt override; .ext-Komposition benennen")
@call patch("src/seeds.rs", "Doc-Kommentar: die .ext-Seeds erweitern die Built-ins, sie ersetzen sie nicht")

@call render_check("lmd-brainstorm", "self-review")

**Expected:** der gerenderte Dispatch-Contract trägt **keine** Fremdzeilen mehr aus der stale
`.ext` — genau die drei Zeilen, die die Spec am 2026-07-17 im Output nachgewiesen hat.

### Verify & Close

@call verify("src/fragments.rs src/bridges/dispatch.rs src/seeds.rs content/templates/hard-rules.ext.lmd.md content/templates/parallel-dispatch.ext.lmd.md")
@call gate("src/fragments.rs src/bridges/dispatch.rs src/seeds.rs content/templates")
@call review_change()
@call commit("src/fragments.rs src/bridges/dispatch.rs src/seeds.rs content/templates", "feat(fragments): .ext generisch fuer jedes Fragment; Sonderpfad in dispatch.rs zurueckgebaut")
@call remember_decision("FragmentRegistry::resolve komponiert base + <name>.ext.lmd.md (skip-if-inert); .ext nur fuer die drei Built-ins; hard-rules.ext vererbt sich ueber @include in jeden Dispatch")
@phase-end

@phase "task-6"
## Task 6: P1 — deklaratives Arg-Schema (P3 fällt mit)

@call recall_context("Registry-Komposition und Dispatch-Bridge nach P5")

**Warum:** `check` parst nur — es sagte `lmd ok` zu fehlendem `phase=`, zu `role=exec`, zu
`brief=`. Der String `brief` kommt in `src/` als Argument nicht vor; die Bridge liest
`phase`/`companion`/`skill`/`role`/`to_agent` und fragt alles andere nie ab. Unbekannte
Argumente fallen lautlos auf den Boden.

**Verworfen** (Spec Entscheidung 7): Bridges validate-only aufrufen (`validate()`/`execute()`
driften auseinander) und die vier bekannten Fälle hart einprogrammieren (fängt keinen fünften).

**Files:** `src/arg_schema.rs` (neu), `src/lib.rs` (~), `src/bridges/dispatch.rs` (~),
`src/bin/lean_md.rs` (~ `do_check`).

**Interfaces:** Produces

    pub struct ArgSpec {
        pub required_one_of: &'static [&'static [&'static str]],
        pub optional: &'static [&'static str],
        pub enums: &'static [(&'static str, &'static [&'static str])],
    }
    pub fn spec(directive: &str) -> Option<&'static ArgSpec>;
    /// Err(msg) names the offending argument AND the known ones — a user with a typo
    /// must not have to guess.
    pub fn validate(directive: &str, args: &DirectiveArgs) -> Result<(), String>;

Schema-Definition, verbatim:

    /// One declaration per directive, read by BOTH `check` and the bridge. Two copies of
    /// "what is a valid argument" is the drift that produced the bug this fixes.
    static DISPATCH: ArgSpec = ArgSpec {
        required_one_of: &[&["phase"], &["skill", "companion"]],
        optional: &["role", "to_agent"],
        enums: &[("role", &["dev", "review", "test"])],
    };

    pub fn spec(directive: &str) -> Option<&'static ArgSpec> {
        match directive {
            "dispatch" => Some(&DISPATCH),
            _ => None,
        }
    }

`validate` prüft in dieser Reihenfolge: (1) jedes benannte Argument ist in einer
`required_one_of`-Gruppe oder in `optional` — sonst `unknown argument '<x>' — known: …`;
(2) genau eine Gruppe ist vollständig erfüllt — sonst die Exklusiv-/Fehlt-Meldung;
(3) jeder Enum-Wert ist gültig.

**Positional `phase` bleibt erlaubt.** Die Bridge liest heute
`args.get("phase").or_else(|| args.positional(0))` (@`src/bridges/dispatch.rs:67`) — der
bestehende Test `render_dispatch_in` (@`:437`) nutzt `phase="P"` benannt, aber
`@dispatch task-1` muss weiter tragen. `validate` zählt deshalb `positional(0)` als Erfüllung
der Gruppe `["phase"]`; nur **benannte** Argumente werden gegen die Namensliste geprüft. Ein
stillschweigend gebrochener Positional-Aufruf wäre derselbe Fehlertyp, den dieser Task behebt.

**Consumes:** `DirectiveArgs::named_pairs()` @`src/args.rs:61-63`, `get()` @`:46-51`,
Enum-Prüfung @`src/bridges/dispatch.rs:94-102`, Exklusiv-Prüfung @`:69-91`, `do_check`
@`src/bin/lean_md.rs:50-60`.

`dispatch.rs` ruft am Anfang von `execute`

    crate::arg_schema::validate("dispatch", args).map_err(BridgeError::Resolve)?;

(`validate` gibt `Result<(), String>`, die Bridge braucht `BridgeError` — dieselbe Variante, die
die heutigen Meldungen tragen, @`src/bridges/dispatch.rs:71-74`/`:97-100`) und **verliert** die
eigene Enum-Prüfung (`:94-102` → nur noch `args.get("role").unwrap_or("dev")`)
sowie den Exklusiv-Zweig `(Some(_), Some(_))` (`:70-74`). Die `(None, None)`- und
`(None, Some(_))`-Zweige verlieren ihre Fehlerpfade an das Schema; die Body-Auswahl bleibt.

`do_check` iteriert die Body-Zeilen, die mit `@` beginnen (heute nur gezählt, `:52-55`), zerlegt
sie in Name + `DirectiveArgs::parse(rest)` und validiert jede, für die `spec()` ein Schema
liefert. Unbekannte Directives bleiben unberührt — das Schema deckt heute nur `dispatch`.

Tests zuerst (verbatim):

    #[test]
    fn unknown_argument_is_rejected_and_the_known_ones_are_listed() {
        // `brief=` was swallowed and dropped — it never existed in src/. So is every future
        // typo: phse=, to-agent=.
        let out = do_check("@lean-md\nconsumer: ai\n\n@dispatch brief=x phase=y\n", None);
        assert!(out.contains("unknown argument"), "{out}");
        assert!(out.contains("brief"), "the offending arg must be named: {out}");
        assert!(out.contains("phase") && out.contains("role"), "known args must be listed: {out}");
    }

    #[test]
    fn a_bad_role_fails_in_check_not_only_at_render_time() {
        // role IS validated today — but at render time (dispatch.rs:94). check never renders,
        // so it never saw it. Green check, broken file.
        let out = do_check("@lean-md\nconsumer: ai\n\n@dispatch phase=t role=exec\n", None);
        assert!(out.contains("role"), "{out}");
        assert!(!out.contains("lmd ok"), "check must not call this file ok: {out}");
    }

    #[test]
    fn a_dispatch_without_a_brief_source_fails_in_check() {
        let out = do_check("@lean-md\nconsumer: ai\n\n@dispatch role=dev\n", None);
        assert!(!out.contains("lmd ok"), "{out}");
    }

    #[test]
    fn phase_and_companion_together_fail_in_check() {
        let out = do_check(
            "@lean-md\nconsumer: ai\n\n@dispatch phase=x skill=s companion=y\n",
            None,
        );
        assert!(!out.contains("lmd ok"), "exclusive group must be enforced in check: {out}");
    }

    #[test]
    fn a_valid_dispatch_still_checks_ok() {
        let out = do_check("@lean-md\nconsumer: ai\n\n@dispatch phase=t role=review\n", None);
        assert!(out.contains("lmd ok"), "{out}");
    }

    #[test]
    fn the_schema_is_the_only_source_of_validation() {
        // No second definition of "valid role" may survive in the bridge.
        let src = include_str!("bridges/dispatch.rs");
        assert!(
            !src.contains("\"dev\" | \"review\" | \"test\""),
            "dispatch.rs must not re-declare the role enum — arg_schema owns it"
        );
    }

@call tdd(unknown_argument_is_rejected_and_the_known_ones_are_listed)

**Expected:** die bestehenden Dispatch-Render-Tests bleiben grün — der Renderer akzeptiert
exakt, was er vorher akzeptierte, nur meldet `check` es jetzt vorher.

### Verify & Close

@call verify("src/arg_schema.rs src/bridges/dispatch.rs src/bin/lean_md.rs src/lib.rs")
@call gate("src/arg_schema.rs src/bridges/dispatch.rs src/bin/lean_md.rs src/lib.rs")
@call review_change()
@call commit("src/arg_schema.rs src/bridges/dispatch.rs src/bin/lean_md.rs src/lib.rs", "feat(check): deklaratives Arg-Schema — check und Bridge lesen dieselbe Quelle")
@call remember_decision("arg_schema::validate ist die einzige Quelle fuer Directive-Argumente; dispatch.rs validiert nicht mehr selbst; brief= faellt als unknown argument")
@phase-end

@phase "task-7"
## Task 7: P2 — Duplikat-Check im Parser

@call recall_context("arg_schema und do_check-Struktur")

**Warum:** bei doppelten `@phase`-Namen verschwindet der zweite Block spurlos —
`--list-phases` zeigt den Namen einmal, `--phase X` rendert nur den ersten. Stiller
Content-Verlust in einem Dokumentations-Tool.

**Wo:** der Check sitzt im **Parser**, nicht in `check` (Spec Entscheidung 8). Damit greift er
in `check`, `render`, `--list-phases`, MCP und CLI gleichermaßen; der Verlust wird strukturell
unmöglich statt von einem Linter gemeldet, den man überspringen kann.

**Files:** `src/phases.rs` (~).

**Interfaces:** Produces

    /// (name, first definition line, duplicate line) — 1-based lines.
    pub fn duplicate_phase(source: &str) -> Option<(String, usize, usize)>;

Aufrufer: `render_with_phases` (@`src/phases.rs:238-403`) bricht mit einer sichtbaren
Fehler-Envelope ab (dieselbe Mechanik wie
`unterminated_phase_is_a_visible_error` @`:593-596`); `iter_phase_blocks` (@`:465-489`) und
`outline_phases` (@`:501-509`) — die Quellen für `--list-phases` — bekommen denselben Gate.

**Consumes:** `parse_phase_name` @`src/phases.rs:449-451`, `PhaseError::envelope`
@`:25-30`, `outline_phases` @`:501-509`, `iter_phase_blocks` @`:465-489`.

**Die Fehlermeldung nennt beide Fundstellen** — sonst sucht der Autor in einer langen Datei:

    duplicate @phase "task-1" — first defined at line 12, again at line 88

Tests zuerst (verbatim):

    #[test]
    fn duplicate_phase_is_reported_with_both_sites() {
        let src = "@lean-md\nconsumer: ai\n\n@phase \"t\"\nfirst\n@phase-end\n@phase \"t\"\nsecond\n@phase-end\n";
        let (name, first, dup) = duplicate_phase(src).unwrap();
        assert_eq!(name, "t");
        assert_eq!((first, dup), (4, 7), "both sites, 1-based");
    }

    #[test]
    fn distinct_phases_are_not_a_duplicate() {
        let src = "@lean-md\nconsumer: ai\n\n@phase \"a\"\nx\n@phase-end\n@phase \"b\"\ny\n@phase-end\n";
        assert!(duplicate_phase(src).is_none());
    }

    #[test]
    fn duplicate_phase_aborts_render_visibly() {
        // Today the second block vanishes without a trace. Silence is the bug.
        let src = "@lean-md\nconsumer: ai\n\n@phase \"t\"\nfirst\n@phase-end\n@phase \"t\"\nsecond\n@phase-end\n";
        let out = crate::skills::render_source_with_phase(
            src,
            Some("t"),
            None,
            None,
            std::path::PathBuf::from("."),
        );
        let text = format!("{out:?}");
        assert!(text.contains("duplicate"), "render must surface the duplicate: {text}");
        assert!(text.contains("line 4") && text.contains("line 7"), "both sites: {text}");
    }

    #[test]
    fn duplicate_phase_is_visible_in_the_outline() {
        // --list-phases reads outline_phases (:501). Today it shows the name once and the
        // second block is simply gone — the surface that should expose the loss hides it.
        let src = "@lean-md\nconsumer: ai\n\n@phase \"t\"\nfirst\n@phase-end\n@phase \"t\"\nsecond\n@phase-end\n";
        assert!(duplicate_phase(src).is_some(), "the parser-level gate every surface reads");
        assert!(
            outline_phases(src).is_empty(),
            "outline must refuse a lossy source instead of listing one of two blocks"
        );
    }

Ein weiterer Test gehört ins Testmodul von `src/bin/lean_md.rs` — dieselbe Quelle, die
`check`-Oberfläche:

    #[test]
    fn duplicate_phase_fails_in_check() {
        let src = "@lean-md\nconsumer: ai\n\n@phase \"t\"\nfirst\n@phase-end\n@phase \"t\"\nsecond\n@phase-end\n";
        let out = do_check(src, None);
        assert!(!out.contains("lmd ok"), "check must not call a lossy file ok: {out}");
        assert!(out.contains("duplicate"), "{out}");
    }

@call tdd(duplicate_phase_is_reported_with_both_sites)

**Expected:** `cargo nextest run` grün, insbesondere `iter_phase_blocks_orders_phases`
(@`src/phases.rs:1009-1017`) und `outline_is_byte_stable` (@`:1048-1051`) — die bestehenden
Phasen-Tests bleiben unberührt, weil kein Fixture doppelte Namen trägt.

### Verify & Close

@call verify("src/phases.rs")
@call gate("src/phases.rs")
@call commit("src/phases.rs", "fix(parser): doppelte @phase-Namen brechen laut statt Content zu verschlucken")
@call remember_decision("duplicate_phase() sitzt im Parser (phases.rs) und greift in check/render/--list-phases/MCP; die Meldung nennt beide Fundstellen")
@phase-end

@phase "task-8"
## Task 8: P9 — `version_req`-Prüfung gegen `ctxpkg.lock`

@call recall_context("Wiring am MCP-Start und check-Meldungen aus task-4")

**Warum:** eine Mindest-Binary-Version ist heute nicht durchsetzbar — `min_lean_ctx` pinnt
lean-ctx, nicht lean-md. Erst mit dieser Prüfung darf Pack-Content künftig `lmd_render` nennen
(heute nennt er bewusst `ctx_md_render`, weil das auf beiden Binaries funktioniert).

**Files:** `src/version_gate.rs` (neu), `src/lib.rs` (~), `src/bin/lean_md.rs` (~).

**Interfaces:** Produces

    /// The pack range this binary expects. Mirrors `[[dependencies]] version_req` in
    /// lean-ctx-addon.toml — kept honest by `const_matches_the_addon_manifest`.
    pub const PACK_VERSION_REQ: &str = "^0.2";

    /// Installed pack version from `.lean-ctx/ctxpkg.lock`. READ-ONLY — that file belongs
    /// to lean-ctx (`pack install` generates it). Absent/unparsable → None, never an error.
    pub fn installed_pack_version(project_root: &Path) -> Option<String>;

    /// Some(warning) only when the installed version is OUTSIDE the range.
    pub fn drift_warning(project_root: &Path) -> Option<String>;

**Herkunft der Spanne — Konstante + Konsistenz-Gate, kein Runtime-Manifest-Lesen** (Spec
Entscheidung 9): `lean-ctx-addon.toml` wird nicht ins Binary eingebettet und liegt beim
Endnutzer nicht neben dem Binary — ein Runtime-Lookup hätte dort nichts zu lesen und ein Check,
der schweigt, weil er seine Referenz nicht findet, wäre wieder „scheitert leise". Muster des
bestehenden fragment-consistency-Gates: eine Divergenz fällt in CI, nicht beim Nutzer.

**Kritisch — nur Spannen-Verletzung, nicht Ungleichheit.** `docs/dev-readme.md`: „Binary and
pack use independent SemVer. […] That divergence *is* the benefit of the cut." Ein Check, der
bei `Pack 0.2.1 ≠ Binary 0.2.0` warnt, meckert den **gewollten Normalfall** an — Rauschen ab
Tag eins.

Lock-Format zur Referenz (`.lean-ctx/ctxpkg.lock`, TOML, gehört lean-ctx):

    [[package]]
    name = "@dastholo/lean-md-skills"
    version = "0.2.0"

Parse ohne neue Dependency: die Zeile `version = "…"` innerhalb des `[[package]]`-Blocks mit
`name = "@dastholo/lean-md-skills"`. `^0.2` in `0.x` bedeutet `>=0.2.0, <0.3.0` — von Hand
geprüft, kein `semver`-Crate.

Tests zuerst (verbatim):

    #[test]
    fn a_pack_outside_the_range_warns() {
        let root = std::env::temp_dir().join(format!("lmd_vg_out_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx")).unwrap();
        std::fs::write(
            root.join(".lean-ctx/ctxpkg.lock"),
            "[[package]]\nname = \"@dastholo/lean-md-skills\"\nversion = \"0.3.0\"\n",
        )
        .unwrap();
        let w = drift_warning(&root).expect("0.3.0 is outside ^0.2 → must warn");
        assert!(w.contains("0.3.0") && w.contains("^0.2"), "{w}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_pack_inside_the_range_is_silent_even_when_it_differs_from_the_binary() {
        // The #727 cut's whole point: a content-only fix moves the pack to 0.2.1 while the
        // binary stays 0.2.0. Warning here would be noise from day one.
        let root = std::env::temp_dir().join(format!("lmd_vg_in_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx")).unwrap();
        std::fs::write(
            root.join(".lean-ctx/ctxpkg.lock"),
            "[[package]]\nname = \"@dastholo/lean-md-skills\"\nversion = \"0.2.7\"\n",
        )
        .unwrap();
        assert_eq!(drift_warning(&root), None, "inequality inside the range is intended");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_absent_lock_is_neither_an_error_nor_output() {
        let root = std::env::temp_dir().join(format!("lmd_vg_absent_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(installed_pack_version(&root), None);
        assert_eq!(drift_warning(&root), None);
    }

    #[test]
    fn ctxpkg_lock_is_never_written() {
        // That file belongs to lean-ctx. Read-only, byte for byte.
        let root = std::env::temp_dir().join(format!("lmd_vg_ro_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".lean-ctx")).unwrap();
        let raw = "[[package]]\nname = \"@dastholo/lean-md-skills\"\nversion = \"0.2.0\"\n";
        let path = root.join(".lean-ctx/ctxpkg.lock");
        std::fs::write(&path, raw).unwrap();
        let _ = drift_warning(&root);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn const_matches_the_addon_manifest() {
        // Same shape as the fragment-consistency gate: divergence falls in CI, not on a user.
        let manifest = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("lean-ctx-addon.toml"),
        )
        .unwrap();
        assert!(
            manifest.contains(&format!("version_req = \"{PACK_VERSION_REQ}\"")),
            "PACK_VERSION_REQ drifted from lean-ctx-addon.toml"
        );
    }

@call tdd(a_pack_inside_the_range_is_silent_even_when_it_differs_from_the_binary)

**Wiring:** dieselbe Stelle und dieselbe Asymmetrie wie der Seed-Report aus task-4 — `cmd_mcp`
schreibt die Warnung beim Start nach `stderr` (Log-Diagnose), `lean-md check` hängt sie als
Zeile an, wo der Nutzer hinschaut. `render` bleibt lesend und ruft nichts davon.

@call patch("src/bin/lean_md.rs", "drift_warning neben seed_report_line in cmd_mcp (stderr) und do_check (Ausgabezeile)")

### Verify & Close

@call verify("src/version_gate.rs src/bin/lean_md.rs src/lib.rs")
@call gate("src/version_gate.rs src/bin/lean_md.rs src/lib.rs")
@call review_change()
@call commit("src/version_gate.rs src/bin/lean_md.rs src/lib.rs", "feat(version): Pack-Spanne gegen ctxpkg.lock pruefen (nur Spannen-Verletzung warnt)")
@call remember_decision("PACK_VERSION_REQ ist eine Binary-Konstante mit CI-Gate gegen lean-ctx-addon.toml; gewarnt wird NUR bei Spannen-Verletzung, Ungleichheit innerhalb der Spanne ist der gewollte Normalfall")
@phase-end
