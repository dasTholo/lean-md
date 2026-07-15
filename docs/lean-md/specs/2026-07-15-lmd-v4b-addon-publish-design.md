# Design-Spec: lean-md Addon-Publish V4b — Publish-Vorlauf + gegateter Runbook

> Erstellt: 2026-07-15 · Branch: `feat-lmd-v2` · Ansatz **A** (Vorlauf jetzt, immutable Publish als gegateter Runbook)
>
> **Kontext:** Setzt `2026-07-11-lmd-v2-tail-publish-vorlauf-design.md` fort. Dort wurde
> **V4a** geschlossen: `@dastholo/lean-md-skills@0.2.0` ist hosted auf ctxpkg.com publiziert
> (immutable, ed25519-signiert, content-hash `6491dc4e`, artifact-sha `5b77377c…`). Diese Spec
> plant den **einzigen verbleibenden Delivery-Schritt V4b**: den hosted Publish des
> `kind=addon`-Packs `@dastholo/lean-md`. Laufender Fortschritt lebt in `ctx_knowledge`, nicht hier.

---

## 1. Ausgangslage (git-ancestry-verifiziert 2026-07-15)

- **V4a erledigt & immutable.** `@dastholo/lean-md-skills@0.2.0` hosted; Namespace-Reconcile
  `@dasTholo → @dastholo` (Token-Scope klein) committet (`14f0d6b`). GitHub-Handle `dasTholo`
  in URLs/author/LICENSE bewusst unangetastet.
- **Authoring-Manifest `lean-ctx-addon.toml`** (hosted, geprüft): `[artifacts]`×5 mit echten SHAs
  (`af5642…`, `3a3b0e…`, `9e3800…`, `365dee…`, `1b092f…`), `[[dependencies]] @dastholo/lean-md-skills ^0.2`,
  `[mcp] command=lean-md args=[mcp]`, `[mcp.env] LEAN_MD_SKILLS_DIR="{pack_dir:@dastholo/lean-md-skills}"`,
  `[capabilities] network=none filesystem=read_write exec=["lean-ctx"]`, `min_lean_ctx=3.9.6`.
- **GitHub-Release `v0.2.0` existiert + public** — die `[artifacts].url` sind live konsumierbar.
- **Publish-Token liegt vor** (`ctxp_…`, Scope `dastholo`).
- **Kuratierter Registry-Entry** (lean-ctx `rust/data/addon_registry.json`, PR #721): clean `listed`.
  PR #721 gemergt (Nutzer-Bestätigung); der lokale lean-ctx-Repo ist aber nicht auf `origin/main`.

## 2. Quellcode-/Release-Verankerung (lean-ctx, git-ancestry-geprüft)

Der komplette #727-Addon-Stack landete in **released Tag `v3.9.6`** (`372d91c`) — bracket-bewiesen:

| Commit | Rolle | Vorfahre v3.9.5 (`87559528`) | Vorfahre v3.9.6 (`372d91c`) | Vorfahre d4968f2 (installiert) |
|---|---|:--:|:--:|:--:|
| `2da5a7fb6` | `[[dependencies]]` → published pack forwarded (D7-Stop) | ✗ (exit 1) | ✓ | ✓ |
| `97ec2c569` | `{pack_dir:}`-Expander für `[mcp.env]` (Consumer) | ✗ (exit 1) | ✓ | ✓ |
| `ed64d30a9` | `min_lean_ctx`-Enforcement im Preflight | — | ✓ | ✓ |

- **Installiertes Binary** = `lean-ctx 3.9.9 (official)`, build-commit `d4968f2` (#828,
  „shadow_mode default true"); `d4968f2` ist Vorfahre von `origin/main` **und** enthält beide
  Maschinerie-Commits → das publizierende Binary reicht `[[dependencies]]` durch.
- **`addon publish --check --namespace dastholo`** baut `@dastholo/lean-md@0.2.0` (kind=addon,
  5492 B), Audit **`pass`**, 5 Triples (`aarch64/x86_64-linux`, `aarch64/x86_64-darwin`,
  `x86_64-windows-msvc`), `LEAN_MD_SKILLS_DIR`-child_env erkannt, nichts hochgeladen.
- **Released Tags v3.9.6..v3.9.9 existieren upstream** (`v3.9.9 = 2d8433a`) → jeder Release
  ab dem `min_lean_ctx`-Floor trägt Forwarding + Expansion.

> **Widerlegtes Wissen:** die frühere Notiz „released 3.9.7 forciert `dependencies:[]` → Stop"
> ist per direkter Ancestry **falsch**. Das Forwarding (D7) ist vorab erfüllt; die Sorge entfällt.

## 3. Entscheidungen (verbindlich)

**E1 — Ansatz A.** Ein Plan über den ganzen Rest. Alles credential-frei Verifizierbare läuft
*jetzt* (agent-ausführbar); der eine irreversible Hosted-Publish + sein Post-Smoke ist ein
niedergeschriebener, **nicht agent-auto-ausgeführter** Runbook-Tail, gegatet nur noch auf die
bewusste Maintainer-Auslösung (Token liegt bereits vor).

**E2 — Publish mit installiertem `3.9.9-official`** (build `d4968f2`). Nutzer-Wahl. Das
D7-Forwarding ist per Ancestry (§2) bewiesen; ein Dev-Build (`pr/lean-md-addon-v2`) ist **nicht** nötig.

**E3 — Namespace `dastholo`** (klein) überall im Delivery-Pfad. Der GitHub-Handle `dasTholo`
bleibt in URLs/author/LICENSE. Der Runbook nutzt `--namespace dastholo` (nicht die veraltete
`dasTholo`-Form der V4a-Spec).

**E4 — `min_lean_ctx = 3.9.6` bleibt.** Exakt der erste Release mit der Maschinerie (§2). Nicht
anfassen.

**E5 — Kuratierter Entry bleibt `listed`.** Install-Pfad ist der namespaced Ref
`addon add dastholo/lean-md` (löst gegen die Hosted-Registry, lädt den `kind=addon`-Pack mit dem
eingebetteten vollen Manifest; der kuratierte Entry wird nicht konsultiert). Skills laufen über
die verbatim weitergereichten `[[dependencies]]` + `{pack_dir:}`-Expansion **automatisch beim
`addon add`**. Ein `installable`-Flip ist Non-Goal (Kadenz-Kopplung + Manifest-Drift, unverändert
gegenüber der V4a-Spec E3).

**E6 — Publish und Merge sind entkoppelt.** Der Hosted-Publish nutzt nur das lokale Binary + Token
→ ctxpkg.com; er braucht `pr/lean-md-addon-v2 → origin/main` **nicht**. `origin/main` trägt die
Maschinerie bereits; der Branch fügt nur den kuratierten `listed`-Entry, Reverse-Cut-Test-Gates,
Formatter-Routing und Docs hinzu. Der Merge ist ein eigener Upstream-Track (Non-Goal dieser Phase).

**E7 — `main` trägt niemals `docs/lean-md/`** *(harte Invariante, Nutzer-mandatiert)*. Jede
Operation, die einen lean-md-`main`-Branch erzeugt oder nach `main` mergt, **muss** das gesamte
Dev-Doku-Verzeichnis `docs/lean-md/` (Pläne + Specs, inkl. dieser Datei) ausschließen — es gehört
nicht auf `main`. `docs/dev-readme.md`, `README.md`, `INSTALL.md` bleiben getrackt. Der Mechanismus
lebt im eigenen Plan `2026-07-12-lmd-docs-refresh` task-4 (`git rm -r --cached docs/lean-md/`);
diese Phase führt selbst **keinen** `main`-Merge aus, hält die Invariante aber als bindende
Randbedingung für jeden Folge-Merge fest.

## 4. Arbeitspakete

### Task 1 — Credential-freier Pre-Flight *(agent-ausführbar)*
Reine Re-Bestätigung; keine offene Entscheidung mehr.
1. **Namespace-Konsistenz** — `lean-ctx-addon.toml`: `[[dependencies]]` und
   `{pack_dir:@dastholo/lean-md-skills}` tragen `dastholo` (klein).
2. **Immutability-Check** — die Registry kennt `@dastholo/lean-md 0.2.0` **nicht** (nie publiziert;
   der echte Registry-Check ist erste Runbook-Vorbedingung, R2).
3. **SHA-Kreuzprobe** — die fünf `[artifacts].sha256` == GH-Release `v0.2.0` `SHA256SUMS`, byte-genau.
4. **`addon publish --check --namespace dastholo`** → Verdict `pass`, 5 Triples; Ancestry-Notiz (§2)
   als D7-Beleg festhalten.
5. **Skills-Pack-Präsenz** — `@dastholo/lean-md-skills@0.2.0` hosted (V4a) re-confirm; harter
   Invariant (Deps-Resolver sieht nur den Registry-Index).

### Task 2 — Gegateter Runbook-Tail *(KEIN Agent-Auto-Run)*
Verbatim niederschreiben, in Reihenfolge; Gate: bewusste Maintainer-Auslösung (Token vorhanden).

**2.1 — Addon publish (schließt V4b).** Vorbedingung: Registry kennt kein `0.2.0` (R2).

    lean-ctx addon publish ./lean-ctx-addon.toml --namespace dastholo --token ctxp_…

Expected: Registry akzeptiert `@dastholo/lean-md 0.2.0`. Kollision ⇒ abbrechen (immutable).

**2.2 — Post-Publish D7-Assert** am publizierten `pack_manifest`: `dependencies` **nicht-leer**
(`@dastholo/lean-md-skills ^0.2`). Per Ancestry erwartet ✓; leeres Array ⇒ falsches Binary ⇒
abbrechen. Recovery (immutable, kein Retract): Version-Bump `0.2.1` + republish.

**2.3 — Hosted-Re-Smoke** gegen die echte Registry:

    lean-ctx addon add dastholo/lean-md

Expected: voller Chain reproduziert — consent-preview nennt den Skills-Pack; `min_lean_ctx`-Gate
bei ≥3.9.6; `ensure_addon_binary` zieht+matcht das Linux-Triple `af5642…` gegen die public URL;
`{pack_dir:}`-Expansion (absoluter Store-Pfad); Lockfile pinnt Addon **und** Pack.

**2.4 — Integrity-Lock.**

    lean-ctx addon verify

Expected: Integrity-Lock grün.

## 5. Risiken

| # | Risiko | Mitigation |
|---|---|---|
| R1 | Publish-Binary reicht `dependencies` nicht durch (D7) | Per Ancestry (§2) ausgeschlossen (`d4968f2` enthält `2da5a7fb6`); Post-Publish-Assert 2.2 als Belt-and-Suspenders. |
| R2 | Immutability: `0.2.0` bereits publiziert | Registry-Check als erste Publish-Vorbedingung (2.1); nie publiziert → frei. |
| R3 | Namespace-Casing (`dasTholo` vs `dastholo`) bricht die Kette | Task 1.1 verifiziert `dastholo` überall; Token-Scope ist `dastholo`. |
| R4 | Consumer mit released lean-ctx < 3.9.6 | `min_lean_ctx=3.9.6`-Gate (§2, `ed64d30a9`) refust vor der Installation; korrekter Floor. |
| R5 | `docs/lean-md/` leckt bei einem Folge-Merge nach `main` | E7-Invariante; der `main`-erzeugende Schritt (`2026-07-12-lmd-docs-refresh` task-4) nimmt `docs/lean-md/` per `git rm -r --cached` aus dem Index. |
| R6 | `addon add`-Vorschau ohne Offline-Zweig (Upstream, akzeptiert) | Bestimmt nur die Smoke-Reihenfolge; irrelevant für den token-getriebenen Live-Publish. |

## 6. Definition of Done (diese Phase)

Task 1 grün (Namespace konsistent, Immutability frei, 5 SHAs == `SHA256SUMS`, `--check` `pass`,
Skills-Pack hosted); Task 2 als vollständiger, verbatim-korrekter, gegateter Runbook-Tail
niedergeschrieben (Namespace `dastholo`, D7-Assert, Hosted-Re-Smoke, `addon verify`).

**Nicht enthalten (Non-Goals):** die tatsächliche Runbook-Auslösung (`addon publish` — Maintainer-Hand);
der Merge `pr/lean-md-addon-v2 → origin/main` (E6, entkoppelter Upstream-Track); der
`installable`-Flip des kuratierten Entrys (E5); die Ausführung des clean-`main`-Branches ohne
`docs/lean-md/` (eigener Plan `2026-07-12-lmd-docs-refresh` task-4 — diese Spec hält nur die
E7-Invariante fest); P4 (Signing/Publisher-Identität).
