# Design-Spec: lean-md Paket C — Revision 3 (skill-fix abandonnen + selektiver Salvage)

> Erstellt: 2026-07-11 · Branch: `feat-lmd-v2` (HEAD `b96cd9d`)
>
> **Diese Spec ersetzt den Konsolidierungs-Teil zweier Vorgänger:**
> - `2026-07-10-lmd-release-path-rev2-design.md` §5.1 (Paket C) — dessen Schritte gingen von
>   „`skill-fix` mergen, dann C2/C3 zurückrollen" aus. Überholt: `skill-fix` wird nicht gemergt.
> - `paket-c-next-session.md` (Repo-Root) — C1 (Merge), C2 (raw-read wiederherstellen),
>   C3 (`fragments.rs`-Test fixen) entfallen sämtlich. Begründung in §2.
>
> **Was unverändert gilt:** die Release-Kette hinter Paket C (Rev2 §5.4–5.9: Pack-Publish, Tag
> `v0.2.0`, `addon publish`, Upstream-Smoke) und die Verifikationsstrategie (Rev2 §4). Diese
> Spec deckt **nur** die nächste netzfreie Phase ab: die Konsolidierung selbst plus den jetzt
> lokal lauffähigen Smoke Teil 1.
>
> **Anlass:** lean-ctx ist auf `3.9.6` (PR #780 gemergt+released, PR #721 gemergt). Damit
> schließen sich V1a/V1b/V3 durch Upstream, und die Grundannahme von `skill-fix` — jeder
> `ctx_read` einer `.lmd.md` rendere sie — ist durch PR #721 (Entfernung der
> Auto-Render-Delegation) endgültig falsch.

---

## 1. Verifizierter Ausgangsstand (2026-07-11)

Alles belegt, nicht angenommen.

### 1.1 Gate-Tabelle, neu

Die Nutzer-Lage (3.9.6 lokal, PR #780 gemergt, PR #721 gemergt) gegen die Repos geprüft:

| Gate | Rev2-Stand | Stand 2026-07-11 | Beleg |
|---|---|---|---|
| V1a lokales Binary trägt Vertrag | ❌ | ✅ | `lean-ctx --version` → `3.9.6` |
| V1b Vertrag released | ❌ (#780 offen) | ✅ | 3.9.6 = released #780 |
| V3 curated Entry `listed` | 🟡 gebaut | ✅ | lean-ctx lokal auf `pr/lean-md-addon-v2` + main |
| **V0 Paket C** (Konsolidierung) | ❌ | ❌ **offen** | `skill-fix` ungemergt (5 Commits) |
| V2 / V4a / V4b (Tag + Publish) | ❌ | ❌ offen | Netz/Token/Maintainer (Rev2 §5.4–5.7) |

∴ Die nächste nötige Phase ist **V0 (Konsolidierung)** — netzfrei, sofort, komplett in unserer
Hand, bindende Vorbedingung für alles danach (Rev2: 5.1 vor 5.4, publizierte Pack-Versionen sind
immutable).

### 1.2 `feat-lmd-v2` ist bei der Read-Semantik bereits korrekt

Commit `3df0758` („correct the ctx_md_render claim, flag the .lmd.md render delegation as
transitional") hat AGENTS.md/CLAUDE.md auf die richtige Aussage gebracht: `ctx_read` einer
`.lmd.md` liefert **Roh-Source**; Rendern ist explizit/opt-in. Es gibt auf `feat-lmd-v2`
**nichts wiederherzustellen**.

### 1.3 `skill-fix` wurde gegen ein entferntes Verhalten geschrieben

Die Auto-Render-Delegation (`try_lmd_addon_render` in `ctx_read.rs`) lebte nur auf dem
lean-ctx-Branch `pr-rebuild` und ist durch PR #721 entfernt (empirisch schon in Rev2 §1.4
bestätigt: released 3.9.5/3.9.6 liest `.lmd.md` roh). Die Read-Semantik-Commits von `skill-fix`
setzen das Gegenteil voraus.

### 1.4 Commit-für-Commit-Verdikt über `skill-fix` (5 Commits)

Diffs gegen `feat-lmd-v2` geprüft:

| Commit | Inhalt | Verdikt |
|---|---|---|
| `9587936` hard-rules access map | Seed-Zeile „`.lmd.md` is a rendered artifact — every read mode renders it" + Test `hard_rules_carries_the_lmd_access_map` | ❌ **komplett falsch** — reine PR-721-Prämisse, wörtlich |
| `6657cba` read-semantics | ändert AGENTS.md/CLAUDE.md **von** „raw lmd source" **zu** „rendered in every ctx_read mode" | ❌ **invertiert** die schon korrekte `feat-lmd-v2`-Doku |
| `b7423db` preflight-Hunk | „Do NOT `ctx_read` the plan — any read mode renders it (source looks empty)" | ❌ **falsche Prämisse** (nur dieser Hunk) |
| `8f58e91` Spec+Plan | SDD-Hardening-Docs; Task 1/Ä3 kodiert dieselbe Prämisse | ⚠️ nur Docs; sterben mit dem Branch |
| `b7423db` orient-Hunk | `ctx_overview`+`ctx_repomap` vor Task 1 | ✅ **unabhängig valide** |
| `6657cba` File-Drop | löscht `.claude/rules/subagent-multi-agent.md` (149 Z.) + ersetzt CLAUDE.md-`@include` durch Inline-SDD-Block | ✅ valide Aufräumung (Skill `lmd-subagent-driven-development` existiert jetzt → Datei-Prämisse „until it exists" ist stale) |
| `1110f6b` warm-cache → #1040 | ersetzt „shared/warm cache"-Rationale durch „latency ≠ tokens; Subagent-Stubs withheld (#1040)" + Test | ✅ **die einzige echte, PR-721-unabhängige Korrektur** |

### 1.5 #1040 ist real und bestätigt

`1110f6b` stützt sich auf lean-ctx #1040. Belegt in `lean-ctx/rust/src/core/conversation.rs`:
Abschnitt „Concurrency hardening (#1040)", u. a. *„every stub is withheld because the shared id
signal can't identify the caller (#1040)"*; dazu `ctx_read`-Tests
(`cold_fallback_withheld_for_other_conversation`, `conversation_scoped_stub_withheld_for_other_conversation`).
∴ Ein Subagent-`ctx_read` ist nie warm — `ctx_multi_read` ist ein Latenz-, kein Token-Gewinn.
Der Salvage von `1110f6b` ist gerechtfertigt.

---

## 2. Entscheidungen (verbindlich, 2026-07-11, Rev. 3)

**E1 — `skill-fix` wird abandonniert, nicht gemergt.** Die validen Teile werden frisch auf
`feat-lmd-v2` re-authored. Grund: merge-dann-revert (Rev2 §5.1 / paket-c C1–C3) erzeugt reinen
Churn und eine irreführende History, während drei der fünf Commits die falsche Prämisse tragen
und ein Commit die schon korrekte Doku invertiert.

**E2 — feat-lmd-v2-Read-Semantik bleibt unangetastet** (§1.2). C2 (paket-c) entfällt.

**E3 — Kein `fragments.rs`-Test-Fix.** `9587936` landet nie, also gibt es keinen falschen Test
`hard_rules_carries_the_lmd_access_map` zu reparieren. C3 (paket-c) entfällt.

**E4 — `min_lean_ctx = "3.9.6"`.** Korrigiert Rev2-D3 („3.9.5"). Begründung: released `3.9.5`
trägt den #727-Vertrag **nicht**, released `3.9.6` schon (= #780). Der Wert dokumentiert den
Vertrag; per Rev2-D6 schützt das Preflight-Gate strukturell nicht gegen ≤ die Version, die es
selbst einführte — der reale Schutz gegen stillen Leerlauf bleibt der harte `PACK_MISSING` im
Renderer. `paket-c-next-session.md` „Nicht anfassen: min_lean_ctx" ist damit aufgehoben (die
offene lean-ctx-Entscheidung, an der es hing, ist mit dem 3.9.6-Release gefallen).

**E5 — Smoke Teil 1 (Rev2 §4.1) ist Teil dieser Phase.** Er ist mit dem installierten,
vertragstragenden 3.9.6 jetzt lokal lauffähig und beweist den Render-aus-Pack-Pfad **vor** dem
immutablen Publish. Der registry-abhängige Teil 2 (Rev2 §4.2) bleibt außerhalb.

---

## 3. Arbeitspakete

Netzfrei, in dieser Reihenfolge. Alle Salvage-Aussagen sind Re-Authoring auf `feat-lmd-v2`, kein
`git merge` und kein `cherry-pick` von `skill-fix` (der Branch wird nicht berührt).

### AP1 — Salvage #1040-Warm-Cache-Korrektur (aus `1110f6b`)

Inhaltlich identisch zu `1110f6b`. Ersetze die „shared/warm cache"-Rationale durch
„latency ≠ tokens; Subagent-Stubs withheld (#1040)" in:

- `content/core/dispatch-contract.lmd.md`
- `content/core/_fragments/parallel-dispatch.lmd.md`
- `content/skills/lmd-dispatching-parallel-agents/body.lmd.md`
- `content/skills/lmd-subagent-driven-development/body.lmd.md`

Plus Test `no_body_or_fragment_claims_a_warm_subagent_cache` in `src/skills.rs` (verbietet die
Claims „cache is already shared / shared cache / shared mcp cache / first `ctx_read` hits";
erzwingt `#1040` + „latency" in `parallel-dispatch`; schützt die „warm cache"-Formulierung von
`lmd-writing-plans` als bewusste Ausnahme).

`content/core/dispatch-contract.lmd.md` und `parallel-dispatch.lmd.md` sind embedded Seeds
(`include_str!` in `src/fragments.rs`) — Seed-Änderung ⇒ Binary-Änderung ⇒ #498-Fragment-Gate
betroffen.

### AP2 — Salvage SDD-Orientation (aus `b7423db`, nur Hunk 1)

In `content/skills/lmd-subagent-driven-development/body.lmd.md` vor Task 1 einfügen:
„before Task 1, map the ground once — `ctx_overview` + `ctx_repomap`". **Der preflight-Hunk
(„any read mode renders it") wird NICHT übernommen.**

> AP1 und AP2 fassen dieselbe Datei an → in **einem** Commit bündeln oder geordnet anwenden.

### AP3 — Salvage Cleanup (aus `6657cba`, ohne die Read-Semantik-Inversion)

- `.claude/rules/subagent-multi-agent.md` löschen (149 Z.; der SDD-„until it exists"-Kontrakt ist
  durch das existierende Skill `lmd-subagent-driven-development` überholt).
- In `CLAUDE.md` den `@rules/subagent-multi-agent.md`-`@include` durch den knappen Inline-SDD-Block
  ersetzen (Dispatch-Contract lebt als Seed `content/core/dispatch-contract.lmd.md`; Fortschritt/
  Briefs/Batons über `ctx_session`/`ctx_knowledge`/`ctx_agent`).
- **NICHT übernehmen:** die AGENTS.md- und CLAUDE.md-„Rendering"-Note-Änderung von „raw source" zu
  „rendered in every mode". Beide bleiben in der schon korrekten `feat-lmd-v2`-Fassung.

### AP4 — `min_lean_ctx` 3.9.4 → 3.9.6

In `lean-ctx-addon.toml` (Zeile mit `min_lean_ctx`), mit Kommentar, der E4 (und Rev2-D3/D6)
festhält: der Wert dokumentiert den Vertrag; für ≤ die einführende Version wirkungslos; lässt das
lokale 3.9.6-Binary im Preflight mit Gleichstand passieren.

### AP5 — Pack-Drift-Rebless (Pflicht, weil AP1/AP2 `content/skills/**` anfassen)

1. `LEAN_MD_BLESS=1 cargo nextest run --test pack_drift` → schreibt `content/skills.sha256`
2. `lean-ctx pack create --kind skills --name @dasTholo/lean-md-skills --version 0.0.0-cihash
   --from content/skills --description "lmd skills"`
3. `content/skills.ctxpkg-hash` aus `<pkg_dir>/manifest.json` (`integrity.content_hash`)

Beide Hash-Dateien liegen **neben** `content/skills/`, nie darin. Flags nicht raten:
`lean-ctx pack create --help` (siehe `docs/dev-readme.md`).

### AP6 — Gates

- `cargo nextest run` (Baseline 561 Tests; +1 durch AP1-Test)
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt` vor jedem `git add` (pro geänderter Datei; Projektregel)

### AP7 — Smoke Teil 1 (Rev2 §4.1, registry-frei)

Gegen ein `cargo build --release`-Binary; `<store>` = Store-Pfad aus AP5-Schritt 2.

| Prüfung | Kommando | Erwartung |
|---|---|---|
| Pack materialisiert lokal | `lean-ctx pack create --kind skills --name @dasTholo/lean-md-skills --version 0.2.0 --from content/skills` | Store-Pfad `…/packages/skills/@dasTholo__lean-md-skills/0.2.0/`, 42 Dateien |
| Release-Binary rendert aus dem Pack | `LEAN_MD_SKILLS_DIR=<store> ./target/release/lean-md render --skill lmd-brainstorm --phase pre-context` | nicht-leerer Render |
| **Negativprobe** | `env -u LEAN_MD_SKILLS_DIR ./target/release/lean-md render …` | Exit ≠ 0, `PACK_MISSING …` |
| Debug-Fallback greift, Release nicht | `cargo run -- render …` grün vs. Release ohne Env rot | `cfg(debug_assertions)` wirkt |
| Overlay schlägt Pack | Sentinel in `<jail>/.lean-ctx/lean-md/skills/<skill>/body.lmd.md` | Sentinel gewinnt |
| Assets + Exec-Bit | `lean-md skill install lmd-brainstorm --local` | 5 Scripts, `*.sh` mode `0755` |
| Roh-Read | `ctx_read(<datei>.lmd.md, mode=raw)` | unaufgelöste `@include` / `{{ }}` |

> Der `0.2.0`-Pack aus der ersten Zeile ist der Verifikations-Pack, **nicht** der Publish
> (Rev2 §5.4, Netz/Token). Publizierte Versionen sind immutable — deshalb bleibt der Publish
> hinter dieser Phase.

---

## 4. Was wegfällt / nicht enthalten

- **Wegfall ggü. `paket-c-next-session.md`:** C1 (Merge), C2 (raw-read wiederherstellen),
  C3 (`fragments.rs`-Test fixen) — sämtlich gegenstandslos (E1–E3).
- **`8f58e91` Spec+Plan** (`2026-07-10-lmd-sdd-skill-hardening-*`) bleiben auf `skill-fix` und
  sterben mit dem Branch — kein Revise-Aufwand auf `feat-lmd-v2`.
- **Nicht enthalten (bleibt Rev2):** Pack-Publish (V4a), Tag `v0.2.0` (V2), `addon publish` (V4b),
  Upstream-Smoke/Task 7 (V1b/V3-Nachzug). Alle brauchen Netz/Token/Maintainer.
- **Doku-Konsolidierung** von `docs/dev-readme.md` (zwei redundante Release-Regime-Abschnitte) —
  eigener Durchgang, nicht Teil von C (paket-c „Optional").

---

## 5. Risiken

| # | Risiko | Mitigation |
|---|---|---|
| R1 | AP1/AP2 fassen dieselbe SDD-body-Datei an → Reihenfolgekonflikt | In einem Commit bündeln (§3 AP2-Note) |
| R2 | Rückfall in „warm cache"-Behauptung bei künftigen Edits | Test `no_body_or_fragment_claims_a_warm_subagent_cache` (AP1) ist der Gate |
| R3 | Rebless vergessen → Drift-Gate rot in CI | AP5 ist Pflicht-AP, nicht optional; `tests/pack_drift.rs` + `.github/workflows/pack-drift.yml` fangen es |
| R4 | `#1040` wäre doch nicht real → `1110f6b`-Salvage falsch | **Ausgeräumt** (§1.5, im lean-ctx-Source belegt) |
| R5 | `.claude/rules/subagent-multi-agent.md`-Löschung bricht einen `@include` woanders | Nur CLAUDE.md referenziert es; AP3 ersetzt genau diese Referenz |

---

## 6. Definition of Done

- `skill-fix` **nicht** gemergt; die drei validen Teile (#1040-Korrektur + Test, SDD-Orient,
  Cleanup) auf `feat-lmd-v2` re-authored; die drei Falschteile nicht übernommen.
- `feat-lmd-v2`-Read-Semantik unverändert korrekt (kein Regress).
- `min_lean_ctx = "3.9.6"` mit E4-Kommentar.
- `skills.sha256` + `skills.ctxpkg-hash` neu geblesst; Drift-Gate grün.
- `cargo nextest run` + `cargo clippy --all-targets -- -D warnings` grün; `cargo fmt` sauber.
- **Smoke Teil 1 (AP7) durchlaufen:** Release-Binary rendert aus dem Pack-Store; ohne
  `LEAN_MD_SKILLS_DIR` harter `PACK_MISSING`; Overlay schlägt Pack; Debug-Fallback nur im
  Dev-Build; Roh-Read liefert unaufgelöste Direktiven.
- **Endzustand:** publish-ready + verifiziert. Nächste Phase = Rev2 §5.4–5.9 (Netz/Token/Maintainer).
