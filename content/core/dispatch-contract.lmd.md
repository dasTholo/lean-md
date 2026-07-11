## lean-ctx Subagent Contract (MANDATORY)
You run in an isolated context. Before any other action:
1. ctx_agent action=register agent_type=subagent role={{ role }}
   (no ctx_share, no fresh — you read full text; the controller's stubs never reach you)

@include hard-rules

Tool discipline:
- Under tool_profile=power ALL lean-ctx tools are DIRECT — call them DIRECTLY. If one
  shows up deferred, run ToolSearch(query="select:<tool>") FIRST, then call it. NEVER
  wrap a tool in ctx_call.
- NEVER fresh, NEVER raw — to re-read your own edits use ctx_delta or ctx_read mode=diff.
- Search → ctx_search (never grep/rg); read files → ctx_read (never cat).
- Rust (*.rs) non-symbol edits → `ctx_read mode=anchored` → `ctx_patch` (patch by
  LINE:HASH anchor, never re-emit old text); `ctx_edit` only tiny-span/replace-all;
  symbol nav/refactor → `ctx_refactor` / @symbol.
- git commit: PLAIN git commit -m "subject" -m "trailer" — NEVER a heredoc / $( ) subshell.
- Output discipline: render in CRP mode `{{ crp }}` (off|compact|tdd) — see tdd-schema.

On finish:
- ctx_agent action=post category=<status|finding> message="<summary>"
- ctx_agent action=handoff to_agent={{ controller_id }} message="<baton>"
- ctx_knowledge action=remember for any durable fact/gotcha
Report final status: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
