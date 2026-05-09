#!/usr/bin/env bash
# install.sh — symlink the project's git hooks into .git/hooks/.
#
# Per coding_agent_session_search-cuu3f. The hook is opt-in (not auto-installed);
# operators run this script once after clone if they want the bead-ID
# pre-push warning.

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
GIT_HOOKS="$PROJECT_ROOT/.git/hooks"
SRC_HOOKS="$PROJECT_ROOT/scripts/git-hooks"

[ -d "$PROJECT_ROOT/.git" ] || {
    echo "ERROR: $PROJECT_ROOT is not a git repo (no .git/ directory)." >&2
    exit 1
}

mkdir -p "$GIT_HOOKS"

for hook in pre-push; do
    src="$SRC_HOOKS/${hook}.sh"
    dst="$GIT_HOOKS/$hook"
    [ -x "$src" ] || chmod +x "$src"
    if [ -L "$dst" ]; then
        rm "$dst"
    elif [ -e "$dst" ]; then
        echo "WARN: $dst already exists and is not a symlink. Backing up to ${dst}.bak.${RANDOM}." >&2
        mv "$dst" "${dst}.bak.${RANDOM}"
    fi
    ln -s "$src" "$dst"
    echo "Installed: $dst -> $src"
done

echo ""
echo "Pre-push hook installed. It warns (does not block) when pushing to main"
echo "without any bead-ID reference in the commits. To uninstall:"
echo "    rm $GIT_HOOKS/pre-push"
