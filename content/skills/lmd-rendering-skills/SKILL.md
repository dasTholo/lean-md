---
name: lmd-rendering-skills
description: Use when you need to render an lmd-* skill phase or companion, or when a render call fails and you must decide whether the gateway is broken or the call was misaddressed.
---

# lmd-rendering-skills

Every `lmd-*` skill is a delegation stub. Its body lives in the version-pinned pack
`@dastholo/lean-md-skills` and renders on demand, one phase at a time. Never read a
body or companion file from disk.

## The call

In the **lean-ctx addon** topology — lean-md aggregated behind the lean-ctx gateway,
the common case and the one this skill exists for — `ctx_md_render` is NOT a direct
tool. lean-md is a separate stdio MCP server, reachable only through the gateway:

    ctx_tools(action="call", tool="lean-md::ctx_md_render",
              arguments={"skill": "<skill>", "phase": "<phase>"})

Pass exactly **one** of `phase` or `companion`:

    arguments={"skill": "<skill>", "companion": "<name>"}

Behind the gateway a direct `ctx_md_render(...)` call cannot succeed — lean-ctx holds
no lean-md code. The failure reads like a dead gateway; it is a misaddressed call.
(Wired straight into your MCP client instead — `lean-md mcp` as its own server —
lean-md exposes `ctx_md_render` as a plain direct tool; then call it bare. Step 1
below tells you which topology you are in.)

## Diagnosis order

1. `ctx_tools(action="list")` — does it show `lean-md [stdio, enabled]`? Then you
   are behind the gateway, everything works, and the call was merely misaddressed.
   Fix the call, do not fall back. (No lean-ctx gateway at all, but `ctx_md_render`
   sits in your own tool list? Direct topology — the bare call is correct.)
2. `Transport closed`? Retry once — sporadic, the gateway respawns the server.
3. Only if the server is genuinely absent: shell fallback (below).

## Shell fallback (absent server only)

Read `lean_md_bin` and `lean_md_skills_dir` from `.lean-ctx/lean-md/vars.toml`,
export `LEAN_MD_SKILLS_DIR=<lean_md_skills_dir>`, then run
`<lean_md_bin> render --skill <skill> --phase <phase> --consumer=ai`.
Without `lean_md_skills_dir` the binary reports `PACK_MISSING`. Same binary,
byte-identical output.
