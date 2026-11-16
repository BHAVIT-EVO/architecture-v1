#!/usr/bin/env bash
# Fails when a UI bundle could show sample data.
#
# The app has no mock mode: every work, permission, activity row and
# diagnostic number comes from Rust through the bridge (docs/BRIDGE.md). The
# browser preview still uses ui/data/works.json, so this check is what keeps
# that file, and the sample titles inside it, out of the shipped app.
#
# Usage: bash scripts/check-ui-bundle.sh <path-to-Resources/ui> [more dirs...]
set -euo pipefail

status=0
for dir in "$@"; do
  if [ ! -d "$dir" ]; then
    echo "check-ui-bundle: $dir is not a directory" >&2
    status=1
    continue
  fi
  while IFS= read -r file; do
    echo "check-ui-bundle: sample corpus found: $file" >&2
    status=1
  done < <(find "$dir" -type f -name 'works.json' 2>/dev/null)

  # A sample work title inside any JSON in the bundle means a corpus shipped,
  # whatever it is called. (The same words in tests or comments are fine.)
  while IFS= read -r file; do
    echo "check-ui-bundle: sample title found in $file" >&2
    status=1
  done < <(grep -rl --binary-files=without-match --include='*.json' 'Frontend prompt for product redesign' "$dir" 2>/dev/null)
done

if [ "$status" -eq 0 ]; then
  echo "check-ui-bundle: no sample data in $*"
fi
exit "$status"
