# Design-Spec: lean-md V2-Tail — Publish-Vorlauf + bedingter Registry-Smoke

> Erstellt: 2026-07-11 · Branch: `feat-lmd-v2` · Ansatz **A**
>
> **Kontext:** Setzt `2026-07-10-lmd-release-path-rev2-design.md` fort. Dort sind
> **V0, V1a/V1b/V3 und V2** geschlossen (Tag `v0.2.0` released, fünf echte SHA-256 im
> Manifest, Pack-Drift-Fix + pre-commit-Gate grün). Diese Spec plant den **verbleibenden
> Schwanz §5.4–5.9** (V4a, Smoke Teil 2, V4b, Voll-Smoke) — so weit **credential-frei**
> ausführbar wie möglich, der Rest als niedergeschriebener, gegateter **Runbook-Tail**.
> Laufender Fortschritt lebt in `ctx_knowledge`, nicht hier.

---

## 1. Ausgangslage (verifiziert 2026-07-11)

- `feat-lmd-v2` liegt **1 Commit vor** `origin` (`43fb487 add cargo to pre-commit`, **ungepusht**).
  d35b580 (sync-manifest, echte SHAs) + 4c3cbce (Rebless, ctxpkg-hash `6491dc4e`) sind auf beiden.
- Tag `v0.2.0 → 5ee2c62` zeigt auf den **Vor-Fix-Commit** (per Rev2 §7 kein Retag). Tag-Inhalt ≠ aktueller Branch-Inhalt.
- `lean-ctx-addon.toml` (Authoring-Manifest, **hosted**): `[artifacts]`×5 mit echten SHAs
  (`af5642…`, `3a3b0e…`, `9e3800…`, `365dee…`, `1b092f…`), `[[dependencies]] @dasTholo/lean-md-skills ^0.2`,
  `[mcp] command=lean-md args=[mcp]`, `[mcp.env] LEAN_MD_SKILLS_DIR="{pack_dir:@dasTholo/lean-md-skills}"`,
  `[capabilities] network=none filesystem=read_write exec=["lean-ctx"]`, `min_lean_ctx=3.9.6`.
- **Kuratierter Entry** (`lean-ctx rust/data/addon_registry.json`, PR #721): clean **`listed`** —
  Keys nur `addon`+`mcp`, `mcp.command` leer, kein `[artifacts]`, keine `[[dependencies]]`, kein Alt-Rest.
- **GitHub-Release `v0.2.0` existiert + ist public** — die `[artifacts].url` sind live konsumierbar.
- **Keine Publish-Credentials konfiguriert** (Endpoint + `CTXPKG_TOKEN` fehlen); ed25519-Signierschlüssel unklar.
- Der lokale lean-ctx-Daemon wird gerade aus dem #721-Branch neu gebaut → die registry-abhängige
  Verifikation ist zusätzlich auf „Daemon rebuilt + #721 gemergt" blockiert.

## 2. Quellcode-Verankerung (lean-ctx, geprüft)

- **`artifact_install.rs`** (unified installer #724/#725): `ensure_addon_binary`→`fetch_verified` —
  mandatory sha256-Pin (leer → refuse *vor* Netz), Download→`.tmp`→sha256-Verify→`0o555`→atomic rename;
  `policy=locked` blockt. Vier der sechs Modul-Tests üben direkt `fetch_verified`/`ensure_addon_binary`.
  Die fünf Manifest-SHAs werden **hier** konsumiert.
- **`manifest.rs`**: `AddonManifest` trägt `artifacts`+`dependencies`+`capabilities` (alle `#[serde(default)]`);
  der **kuratierte Registry deserialisiert in denselben Struct** (`registry.rs: parse→Vec<AddonManifest>`).
  `expand_pack_env_maps_declared_dependency_to_pack_dir` (:628) beweist die Kette
  `[[dependencies]]`→`ResolvedDep`→`pack_env::expand_pack_env` → `LEAN_MD_SKILLS_DIR = <store>/skills/@dasTholo__lean-md-skills/0.2.0`.
  `addon publish` reicht `[[dependencies]]` **verbatim in `PackageManifest.dependencies`** (:112).
- **`registry.rs: validate_entries`** (:133): installable-Entry braucht author/homepage/license/description,
  kein `shell_exec`/`fetch_exec`/`insecure_url`/`unpinned`/`cap_net_underdeclared`.
  Test **`flagship_lean_md_is_listed`** (:407) verankert den `listed`-Status bewusst.

## 3. Entscheidungen (verbindlich)

**E1 — Ansatz A.** Ein Plan über den ganzen Schwanz. Alles credential-frei Verifizierbare wird
*jetzt* ausgeführt; die zwei echten Hosted-Publishes + ihr Post-Smoke sind ein niedergeschriebener,
**nicht agent-auto-ausgeführter** Runbook-Tail, gegatet auf Credentials + Daemon-Rebuild + #721-Merge.
Maximale Verifikation *vor* dem irreversiblen (immutablen) Publish.

**E2 — Pack-Version `0.2.0`.** Nichts ist bisher publiziert → `0.2.0` ist in der Registry frei
(der `0.2.0`-Verifikations-Pack aus Rev2 §5.3 war nur `pack create` lokal). Bewusster Trade-off:
die publizierten `0.2.0`-Pack-Bytes (post-Rebless, `6491dc4e`) ≠ die Skills-Bytes im Tag `v0.2.0`
(Rebless liegt *nach* dem Tag). Akzeptiert; per Rev2 §7 kein Retag. Task 0 verifiziert, dass die
Registry `@dasTholo/lean-md-skills 0.2.0` noch nicht kennt (Immutability-Kollision).

**Tag ⟺ Pack-Entkopplung (E2-Nachtrag).** Ein neues Git-Tag ist an das *Binary*-Release gekoppelt
(`release.yml`), **nicht** an den Pack. Das Binary embedded via `include_str!` nur
`content/{core,gloss,templates,lang,tooling}` (+ `src/`, geprüft in `fragments.rs`/`gloss.rs`/`seeds.rs`);
`content/skills/` lebt im **Pack**. ∴ eine `content/skills/`-Änderung → nur `pack publish`
(Version-Bump, immutable), **kein** Tag; erst ein Eingriff in `src/` oder das embedded-Subset zwingt
zu Tag + Release + `[artifacts]`-SHA-Nachzug. **Diese Phase ändert nichts an `content/`** (V4a/V4b =
reine Registry-Ops) → **kein neues Tag** (Rev2 §5.6 mit V2 erledigt; der Runbook-Tail listet korrekt keinen Tag-Schritt).

**E3 — Kuratierter Entry bleibt `listed`.** lean-md ist mit `listed` **voll auslieferbar**: der
Install-Pfad ist der **Hosted-Pack** `addon add dasTholo/lean-md` (ein namespaced Ref löst gegen die
Hosted-Registry auf und lädt den `kind=addon`-Pack mit dem eingebetteten vollen Manifest; der
kuratierte Entry wird dabei nicht konsultiert). Die **Skills laufen** über die verbatim
weitergereichten `[[dependencies]]` + `{pack_dir:}`-Expansion (§2), **automatisch beim `addon add`**
— kein separates Kommando nötig. Ein installierbarer kuratierter Entry *wäre* validator-machbar
(nur `flagship_lean_md_is_listed`→`…is_installable` flippen), koppelte aber die versions-gepinnten
`[artifacts]`-URLs an lean-ctx' Release-Kadenz (Bundle-Snapshot-Staleness bei jeder neuen
lean-md-Version) und dupliziert das Manifest in lean-ctx' Quellbaum (Drift-Risiko). Der einzige
Gewinn — die Kurzform `addon add lean-md` — wiegt das nicht auf. **Non-Goal** dieser Phase.

**E4 — `pack publish @dasTholo/lean-md-skills` ist der harte Invariant.** Der Deps-Resolver sieht
**nur** den Registry-Index; ein Pack, das ausschließlich als GH-Asset existiert, ist unsichtbar →
`addon add` bricht mit „no installable version matches". Der Skills-Pack **muss** hosted publiziert
sein, damit die Skills laufen — in *jeder* Entry-Variante. Das ist die eigentliche Delivery-
Voraussetzung, orthogonal zur listed/installable-Frage.

## 4. Arbeitspakete

### Task 0 — Pre-Flight / origin-reconcile *(bindende Vorbedingung)*
1. **Ungepushten `43fb487` klären** — ändert **nur** `.pre-commit-config.yaml` (+6 Zeilen), berührt
   **weder `content/` noch `src/`** → **kein Pack- oder Binary-Impact** (kann den Pack nicht
   kontaminieren, zwingt kein Tag). Reine Branch-Hygiene: pushen oder bewusst lokal halten. Der
   Grundsatz „Pack nur aus sanktioniertem `content/skills`-Stand" gilt generell, greift hier aber nicht.
2. **SHA-Kreuzprobe** — die fünf `[artifacts].sha256` == GH-Release `SHA256SUMS`, byte-genau.
3. **Immutability-Check** — bestätigen, dass nie ein `0.2.0`-Pack publiziert wurde (lokal: kein
   `pack publish` lief; der echte Registry-Check ist erste Runbook-Vorbedingung).
4. **Signierschlüssel-Präsenz prüfen** — bestimmt, ob `pack export --sign` jetzt signiert oder
   unsigniert für die Mock-Smoke (Signatur dann in den Runbook).
5. **Mock-Feasibility klären** — akzeptiert `lean-ctx` einen lokalen Registry-Index für die
   Dep-Resolution (`--registry`/`CTXPKG_REGISTRY`)? Entscheidet, ob Smoke Teil 2 jetzt läuft (§4.3)
   oder in den Runbook wandert.

### V4a-Prep + Audit *(credential-frei, ausführbar)*
- `pack create --kind skills --name @dasTholo/lean-md-skills --version 0.2.0 --from content/skills`
  → lokaler Store, 42 Dateien, ctxpkg-hash `6491dc4e` (deterministisch #498).
- `pack export --sign` (bzw. unsigniert, s. Task 0.4) → `.ctxpkg`.
- `addon audit ./lean-ctx-addon.toml` → Publish-Gate offline (wiring-risk + capability-coherence +
  malware); exit≠0 = Stopp. Löst **keine** Deps auf → netzfrei.

### Smoke Teil 1 re-verify *(credential-frei — Rev2 §4.1)*
Release-Binary rendert aus dem Pack-Store; ohne `LEAN_MD_SKILLS_DIR` harter `PACK_MISSING`;
Overlay schlägt Pack; Debug-Fallback nur im Dev-Build; Assets `*.sh` mode `0755`; `.lmd.md`-Roh-Read
liefert unaufgelöste Direktiven.

### Smoke Teil 2 — bedingt *(§4.2 Rev2, credential-frei via Mock-Index)*
**Nur falls Task 0.5 einen sauberen Mock-Harness bestätigt.** Der Mock-Index muss **nur**
`@dasTholo/lean-md-skills 0.2.0` tragen — der Binär kommt aus dem **public GH-Release** (token-frei).
`addon add ./lean-ctx-addon.toml` verifiziert dann den vollen Chain: consent-preview nennt den Pack,
`min_lean_ctx`-Gate bei Gleichstand, `ensure_addon_binary` zieht+matcht das Linux-Triple `af5642…`
gegen die public URL (end-to-end-Beweis, dass lean-ctx' Installer die released Hashes akzeptiert),
`{pack_dir:}`-Expansion (absoluter Store-Pfad, kein Literal), Lockfile pinnt Addon **und** Pack.
**Fallback:** kein Mock möglich → Teil 2 komplett in den Runbook-Tail. Kein Fake-Smoke.

### Render-Voll-Smoke *(credential-frei — Rev2 §5.9-Renderseite)*
`lean-md render docs/lean-md/plans/2026-07-09-lmd-p3-skills-pack-full-cut.lmd.md --phase task-7 --consumer=ai`.

### Runbook-Tail *(kein Agent-Auto-Run — gegatet auf Token + Daemon-Rebuild + #721-Merge)*
1. `pack publish pack.ctxpkg --token ctxp_…` (schließt V4a). Vorbedingung: Registry kennt kein `0.2.0`.
2. `addon publish --namespace dasTholo` (schließt V4b). **Stopp-Bedingung (Rev2 D7):** das publizierte
   `pack_manifest` trägt nicht-leeres `[[dependencies]]`; leeres Array ⇒ falsches Binary ⇒ abbrechen.
3. **Hosted-Re-Smoke** (Smoke Teil 2 gegen die echte Registry) + `addon verify` (Integrity-Lock).
4. §5.8 abschließen: PR #721 gemergt bestätigen; `min_lean_ctx`-Wert im kuratierten Entry auf `3.9.6`
   (Entry bleibt `listed`).

## 5. Risiken

| # | Risiko | Mitigation |
|---|---|---|
| R1 | Kein lokaler Registry-Mock → Teil 2 nicht jetzt verifizierbar | Task 0.5 klärt Feasibility zuerst; ehrlicher Fallback = Runbook. Kein Blocker für die übrigen Tasks. Die Binary-Verifikation selbst ist getesteter lean-ctx-Code. |
| R2 | Kein ed25519-Signierschlüssel | Unsigniert exportieren für die Mock-Smoke; Signatur in den Runbook. |
| R3 | Immutability: `0.2.0` bereits publiziert | Registry-Check als erste Publish-Vorbedingung; da nie publiziert → frei. |
| R4 | Ungepushter `43fb487` = Branch-Drift | Task 0.1 entscheidet push/hold; Pack nur aus sanktioniertem Stand. |
| R5 | `addon add`-Vorschau ohne Offline-Zweig, harter `exit(1)` (Rev2 §1.5) | Bestimmt die Smoke-Reihenfolge; Mock-Index oder Runbook. Upstream-Bug, akzeptiert. |

## 6. Definition of Done (diese Phase)

Pack materialisiert (`0.2.0`, `6491dc4e`) + auditiert (`pass`/`review`); Smoke Teil 1 grün
re-verifiziert; Render-Voll-Smoke grün; Smoke Teil 2 **entweder** grün gegen Mock **oder**
dokumentiert im Runbook-Tail; Runbook-Tail vollständig + gegatet niedergeschrieben.

**Nicht enthalten (Non-Goals):** echte Hosted-Publishes (`pack`/`addon publish`), Hosted-Re-Smoke,
#721-Merge — alle credential-/daemon-gegatet; der Umbau des kuratierten Entrys auf `installable`
(E3, test-verankert + Kadenz-Kopplung); Skill-Tiering; P4 (Signing/Publisher-Identität); die
Upstream-Fixes (`addon add`-Offline-Zweig, `already_satisfied`-sha256-Verify).
