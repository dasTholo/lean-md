#!/usr/bin/env bash
# skills-pack drift gate — invoked by the `pre-commit` framework (see
# .pre-commit-config.yaml) whenever a commit stages content/skills/** or its two
# hash sidecars. Mirrors CI (pack-drift.yml) so drift never reaches origin.
#
# Catches the "changed a skill body but forgot to re-bless the hashes" mistake.
# It VERIFIES only — it never rewrites the hashes. Fix on failure:
#     LEAN_MD_BLESS=1 cargo nextest run --test pack_drift   # → content/skills.sha256
#     # refresh content/skills.ctxpkg-hash from the pack manifest (docs/dev-readme.md)
#
# Bypass a single commit (rarely needed):  git commit --no-verify
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# 1) content/skills.sha256 must match the skill bytes. LEAN_MD_BLESS is unset here.
if ! cargo nextest run --test pack_drift --no-fail-fast; then
  echo "" >&2
  echo "✗ content/skills drifted from content/skills.sha256." >&2
  echo "  Re-bless + stage:  LEAN_MD_BLESS=1 cargo nextest run --test pack_drift && git add content/skills.sha256" >&2
  exit 1
fi

# 2) content/skills.ctxpkg-hash must equal the pack's content_hash.
#    Needs the lean-ctx CLI; warn-and-skip if it is unavailable.
if command -v lean-ctx >/dev/null 2>&1; then
  loc="$(lean-ctx pack create --kind skills \
          --name @dastholo/lean-md-skills --version 0.0.0-precommit \
          --from content/skills --description 'lean-md skills (pre-commit drift)' \
        | sed -n 's/^ *Location: *//p')"
  if [ -n "$loc" ] && [ -f "$loc/manifest.json" ]; then
    got="$(python3 - "$loc/manifest.json" <<'PY'
import json, sys
print(json.load(open(sys.argv[1]))["integrity"]["content_hash"])
PY
)"
    want="$(tr -d '[:space:]' < content/skills.ctxpkg-hash)"
    if [ "$got" != "$want" ]; then
      echo "" >&2
      echo "✗ content/skills.ctxpkg-hash is stale." >&2
      echo "    pack content_hash = $got" >&2
      echo "    checked-in        = $want" >&2
      echo "  Update content/skills.ctxpkg-hash to the pack hash and stage it (docs/dev-readme.md)." >&2
      exit 1
    fi
  else
    echo "warning: could not locate pack manifest.json; skipped ctxpkg-hash check." >&2
  fi
else
  echo "warning: lean-ctx not on PATH; skipped ctxpkg-hash cross-check." >&2
fi

echo "skills pack in sync ✓"
