@lean-md
consumer: ai

@phase "pre-context"
## Finishing a Development Branch — Pre-Context

**When to use:** implementation is complete, all tests pass, and you must decide how to
integrate the work (merge / PR / keep / discard). Announce: "I'm using the
lmd-finishing-a-development-branch skill to complete this work."

**Ambient baseline (MANDATORY):** inline the built-in hard-rules so tool-discipline holds.

@include hard-rules

**Core principle:** Verify tests → Detect environment → Present options → Execute choice →
Clean up.

next: render phase "verify-tests".
@phase-end

@phase "verify-tests"
## Step 1 — Verify Tests (gate BEFORE any option)

Run the project test suite before presenting options — `@call gate(<paths>)` (reformat +
lint + full suite; project-agnostic via `test_cmd`/`lint_cmd`), or the raw runner if no gate
recipe is in scope: `@query "<test_cmd>"`.

**If tests fail — STOP. Do NOT present options:**
> Tests failing (<N> failures). Must fix before completing: [failures]. Cannot proceed with
> merge/PR until tests pass.

Never proceed with failing tests.

**If tests pass:** next: render phase "detect-env".
@phase-end

@phase "detect-env"
## Step 2 + 3 — Detect Environment & Base Branch

Determine workspace state — it selects the menu variant and the cleanup path:

- `GIT_DIR = @query "git rev-parse --git-dir"` (abs), `GIT_COMMON = @query "git rev-parse --git-common-dir"` (abs).
- `WORKTREE = @query "git rev-parse --show-toplevel"`.

| State | Menu | Cleanup |
|---|---|---|
| `GIT_DIR == GIT_COMMON` (normal repo) | 4 options | none |
| `GIT_DIR != GIT_COMMON`, named branch | 4 options | provenance-based |
| `GIT_DIR != GIT_COMMON`, detached HEAD | 3 options (no merge) | none (externally managed) |

Base branch: `@query "git merge-base HEAD main"` (fallback `master`); if neither resolves,
ask the human "This branch split from main — is that correct?".

next: render phase "present-options".
@phase-end

@phase "present-options"
## Step 4 — Present Options (structured, no explanation)

**Normal repo / named-branch worktree — present EXACTLY these 4:**
> Implementation complete. What would you like to do?
> 1. Merge back to <base-branch> locally
> 2. Push and create a Pull Request
> 3. Keep the branch as-is (I'll handle it later)
> 4. Discard this work
> Which option?

**Detached HEAD — present EXACTLY these 3 (no local merge):**
> Implementation complete. You're on a detached HEAD (externally managed workspace).
> 1. Push as new branch and create a Pull Request
> 2. Keep as-is (I'll handle it later)
> 3. Discard this work

Don't add explanation — keep options concise. On the human's choice, render the matching
option phase:
- Merge locally → phase "merge-local"
- Push / PR → phase "create-pr"
- Keep as-is → phase "keep-as-is"
- Discard → phase "discard"
@phase-end

@phase "merge-local"
## Option 1 — Merge Locally (order is binding)

Merge first → verify → cleanup → delete. Wrong order fails (`branch -d` errors while a live
worktree still references the branch).

1. `cd` main repo root: `MAIN = @query "git -C \"$(git rev-parse --git-common-dir)/..\" rev-parse --show-toplevel"`; `cd $MAIN`.
2. `@query "git checkout <base-branch>"`; `@query "git pull"`; `@query "git merge <feature-branch>"`.
3. Re-verify tests on the MERGED result: `@call gate(<paths>)` — Expected: PASS. Never ship a red merge.
4. Cleanup workspace (Options 1 & 4 only — 2 & 3 always preserve the worktree):
   - If `GIT_DIR == GIT_COMMON`: normal repo — no worktree to clean up. Done.
   - Else if `WORKTREE` is under `.worktrees/` or `worktrees/` (provenance — we created it):
     `cd` the main repo root FIRST (else `git worktree remove` fails silently from inside the
     worktree), then `@query "git worktree remove <WORKTREE>"` + `@query "git worktree prune"`.
   - Otherwise the harness owns this workspace — do NOT remove it.
5. Delete the branch (ONLY after worktree removal): `@query "git branch -d <feature-branch>"`.

This is a terminal option — record the close via `ctx_session action=status`.
@phase-end

@phase "create-pr"
## Option 2 — Push and Create PR

`@query "git push -u origin <feature-branch>"`. **Do NOT clean up the worktree** — the human
needs it alive to iterate on PR feedback. No branch delete. Never force-push without an
explicit request.

Terminal — record the close via `ctx_session action=status`.
@phase-end

@phase "keep-as-is"
## Option 3 — Keep As-Is

Report: "Keeping branch <name>. Worktree preserved at <path>." No cleanup, no branch delete.

Terminal — record the close via `ctx_session action=status`.
@phase-end

@phase "discard"
## Option 4 — Discard (typed confirmation REQUIRED)

**Confirm first — wait for the exact word `discard`:**
> This will permanently delete:
> - Branch <name>
> - All commits: <commit-list>
> - Worktree at <path>
> Type 'discard' to confirm.

Only on the exact confirmation `discard`:

1. `cd` main repo root (as in merge-local step 1).
2. Cleanup workspace (same provenance rule as merge-local):
   - If `GIT_DIR == GIT_COMMON`: normal repo — no worktree to clean up.
   - Else if `WORKTREE` is under `.worktrees/` or `worktrees/`: `cd` main root FIRST, then
     `@query "git worktree remove <WORKTREE>"` + `@query "git worktree prune"`.
   - Otherwise the harness owns the workspace — do NOT remove it.
3. Force-delete the branch: `@query "git branch -D <feature-branch>"`.

Terminal — record the close via `ctx_session action=status`.
@phase-end
