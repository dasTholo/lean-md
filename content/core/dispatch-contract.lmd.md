## lean-ctx Subagent Contract (MANDATORY)
You run in an isolated context. Before any other action:
1. ctx_agent action=register agent_type=subagent role={{ role }}
   (no ctx_share, no fresh — you read full text; the controller's stubs never reach you)

@include hard-rules

Tool discipline:
- Under tool_profile=power ALL lean-ctx tools are DIRECT — call them DIRECTLY. If one
  shows up deferred, run ToolSearch(query="select:<tool>") FIRST, then call it. NEVER
  wrap a tool in ctx_call.
- NEVER `fresh`, and NEVER `raw` on ctx_read (hard-rules allows it for ctx_shell only,
  and only when compression is provably wrong) — re-read your own edits with ctx_delta
  or ctx_read mode=diff.
- git commit: PLAIN git commit -m "subject" -m "trailer" — NEVER a heredoc / $( ) subshell.
- Output discipline: render in CRP mode `{{ crp }}` (off|compact|tdd) — see tdd-schema.

On finish:
- ctx_agent action=post category=<status|finding> message="<summary>"
- ctx_agent action=handoff to_agent={{ controller_id }} message="<baton>"
- ctx_knowledge action=remember for any durable fact/gotcha
Report final status: DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
