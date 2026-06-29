# Skill-Token-Vergleich — Benchmark

Neutrales A/B: **A** = `superpowers/test-driven-development` (Monolith),
**B** = `lmd-test-driven-development` (Phasen-Rendering).

## Schicht A — deterministischer Trace (automatisiert)

    cargo run --example skill-token-comparison

Rendert Variante B in-process, tokenisiert beide Varianten (tiktoken-rs:
`cl100k_base` primär, `o200k_base` Parität) und schreibt `SUMMARY.md`
(byte-stabil, #498). Test: `cargo nextest run skill_token_comparison`.

Annahme `TOOL_CALL_OVERHEAD_TOKENS` (Roundtrip-Overhead pro `ctx_md_render`)
ist in `harness.rs` benannt und im `SUMMARY.md` offengelegt — justierbar.

## Schicht B — Subagent-Validierung (manuell, mdai-adaptiert)

Dieselbe Mini-TDD-Aufgabe (eine kleine Funktion + ein Bugfix) wird je Variante
gelöst, einmal pro Druck-Variante:

| Variante | Bedeutung |
|---|---|
| `cold`      | keine Beschränkung, freie Bearbeitung |
| `time`      | expliziter Zeitdruck im Prompt |
| `authority` | Tech-Lead-Override im Prompt ("mach es direkt, ohne Zeremonie") |

Jeder Subagent-Report hält **verbatim** fest: welche Skill-Artefakte real
geladen wurden (Variante B: welche Phasen tatsächlich gerendert wurden) und
wie viele Tool-Calls anfielen. Die geladenen Artefakte werden mit derselben
`harness`-Zählung nachgezählt → realer kumulierter Verbrauch.

Reports ablegen unter:

    variant-A-superpowers/<cold|time|authority>.md
    variant-B-lmd/<cold|time|authority>.md

Zweck: bestätigt/falsifiziert die Schicht-A-Hypothese — stoppen reale Agenten
bei RED/GREEN (dann lädt B `refactor`/`rationalizations` nie), und ist der
Tool-Call-Overhead kleiner als die eingesparten Inhalts-Tokens?
