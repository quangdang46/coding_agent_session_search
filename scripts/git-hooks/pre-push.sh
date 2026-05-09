#!/usr/bin/env bash
# pre-push hook — warns when commits being pushed to main lack any bead-ID reference.
#
# Per coding_agent_session_search-cuu3f. The convention: when a commit closes (or
# substantially advances) a bead, append `(coding_agent_session_search-<short-id>)`
# to the commit subject. Multi-bead commits append all relevant IDs.
#
# Stand-alone bead-tracker commits (e.g. `br sync --flush-only`) do not need
# this prefix; the hook does NOT block, only warns.
#
# Install via scripts/git-hooks/install.sh.

set -euo pipefail

# Read the push specs from stdin: <local-ref> <local-sha> <remote-ref> <remote-sha>
# We only check pushes that target `refs/heads/main`.
remote="${1:-origin}"
url="${2:-}"

# Drain stdin into an array of lines.
while IFS=' ' read -r local_ref local_sha remote_ref remote_sha; do
    [ -n "$local_ref" ] || continue
    [ "$remote_ref" = "refs/heads/main" ] || continue

    # Determine the commit range to inspect.
    if [ "$remote_sha" = "0000000000000000000000000000000000000000" ]; then
        # New branch on remote — inspect the local sha back to the merge-base.
        range="$(git merge-base origin/main "$local_sha" 2>/dev/null || true)..$local_sha"
    else
        range="${remote_sha}..${local_sha}"
    fi

    [ -n "$range" ] || continue

    # Count commits in range and how many reference a bead.
    total=0
    with_bead=0
    while IFS= read -r subject; do
        total=$((total + 1))
        if printf '%s' "$subject" | grep -qE 'coding_agent_session_search-[a-z0-9_]+(\.[0-9]+)*'; then
            with_bead=$((with_bead + 1))
        fi
    done < <(git log --format='%s' "$range" 2>/dev/null || true)

    if [ "$total" -gt 0 ] && [ "$with_bead" -eq 0 ]; then
        cat >&2 <<EOF
warning: pre-push: pushing $total commit(s) to $remote_ref but none reference a bead.
hint: per AGENTS.md "Commit-Message Convention", append (coding_agent_session_search-<id>) to commit subjects when closing a bead.
hint: stand-alone bead-tracker commits (br sync, etc.) and CI/dep bumps are exempt — push proceeds either way.
EOF
    fi
done

exit 0
