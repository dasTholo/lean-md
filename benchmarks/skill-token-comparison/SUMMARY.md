# Skill-Token-Vergleich — SUMMARY

Neutrales A/B: A = superpowers (Monolith), B = lmd (Phasen-Rendering).

## Annahmen

- Tokenizer: `cl100k_base` (primär, ~3% von Claudes echtem Tokenizer); `o200k_base` (Parität mit lean-ctx-Ledger).
- Tool-Call-Overhead pro `ctx_md_render`-Roundtrip: 40 Tokens (Modellannahme, justierbar).

## Artefakte (Tokens je Familie)

| Variante | Artefakt | cl100k | o200k |
|---|---|---|---|
| A | SKILL.md | 2428 | 2414 |
| A | testing-anti-patterns.md | 1933 | 1915 |
| B | SKILL.md (stub) | 540 | 536 |
| B | phase:red | 269 | 268 |
| B | phase:green | 237 | 236 |
| B | phase:refactor | 240 | 239 |
| B | phase:rationalizations | 377 | 375 |
| B | companion:testing-anti-patterns | 797 | 792 |

## Kernmetrik (cl100k)

| Metrik | A (superpowers) | B (lmd, Vollausbau) | Δ (B−A) |
|---|---|---|---|
| Reiner Inhalt | 4361 | 2460 | -1901 |
| Inkl. Ablauf-Overhead | 4401 | 2660 | -1741 |

## Break-even (B kumulativ, Stub + k Phasen)

| k Phasen | B Inhalt | B inkl. Overhead | vs. A Inhalt | vs. A inkl. Overhead |
|---|---|---|---|---|
| 1 | 809 | 849 | B billiger | B billiger |
| 2 | 1046 | 1126 | B billiger | B billiger |
| 3 | 1286 | 1406 | B billiger | B billiger |
| 4 | 1663 | 1823 | B billiger | B billiger |

