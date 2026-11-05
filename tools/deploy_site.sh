#!/usr/bin/env bash
# Publish the site to the public demo repository.
#
#   bash tools/deploy_site.sh
#
# The demo repo is separate from this one on purpose: this repository stays
# private because boundary.json holds one person's real browsing. The public
# repo only ever receives the published artifacts.
#
# Requires: git credentials that can push to BHAVIT-EVO/evo-boundary-demo.
set -euo pipefail

REPO_URL="${EVO_DEMO_REPO:-https://github.com/BHAVIT-EVO/evo-boundary-demo}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SITE="$ROOT/site"
WORK="${TMPDIR:-/tmp}/evo-boundary-demo"

# ---------------------------------------------------------------- guards
if [ ! -f "$SITE/index.html" ]; then
  echo "site/index.html is missing — run: python3 tools/build_site_v2.py" >&2
  exit 1
fi

# source.html only exists after the console rebuild. If it is missing, this
# checkout is stale — most likely an old origin/<branch> that was never
# fetched, which silently publishes last week's demo.
if [ ! -f "$SITE/source.html" ]; then
  cat >&2 <<'MSG'
This checkout predates the console rebuild (no site/source.html).

That means origin/<branch> is stale. Fetch first, then reset:

  git fetch origin
  git reset --hard origin/arena/2873f4c9-architecture-v1
  git log --oneline -1        # must not be ae67276
  bash tools/deploy_site.sh
MSG
  exit 1
fi

echo "==> deploying $(git -C "$ROOT" rev-parse --short HEAD) on $(git -C "$ROOT" rev-parse --abbrev-ref HEAD)"
echo
printf '    %-14s %8s  %s\n' index.html "$(wc -c <"$SITE/index.html" | tr -d ' ')" "the report"
printf '    %-14s %8s  %s\n' demo.html  "$(wc -c <"$SITE/demo.html"  | tr -d ' ')" "the console"
printf '    %-14s %8s  %s\n' source.html "$(wc -c <"$SITE/source.html" | tr -d ' ')" "source excerpts"
printf '    %-14s %8s  %s\n' film.html  "$(wc -c <"$SITE/film.html"  | tr -d ' ')" "concept film"
printf '    %-14s %8s  %s\n' fonts/     "$(ls "$SITE"/fonts/*.woff2 | wc -l | tr -d ' ') files" "self-hosted type"
echo

# ---------------------------------------------------------------- publish
echo "==> cloning $REPO_URL"
rm -rf "$WORK"
git clone --depth 1 "$REPO_URL" "$WORK"

echo "==> copying artifacts"
cp "$SITE/index.html" "$SITE/demo.html" "$SITE/source.html" "$SITE/film.html" "$WORK/"
mkdir -p "$WORK/fonts"
cp "$SITE"/fonts/*.woff2 "$WORK/fonts/"

cd "$WORK"
git add -A
if git diff --cached --quiet; then
  echo "==> nothing to change: the public repo already matches this checkout ($(git -C "$ROOT" rev-parse --short HEAD))"
  echo
  echo "If the live page still looks old, it is the browser cache, not the deploy."
  exit 0
fi

git -c user.name="Evo" -c user.email="evo@local" commit -q -m "site: publish $(date -u +%Y-%m-%dT%H:%MZ)"
echo "==> changed:"
git show --stat --oneline HEAD | tail -n +2 | sed 's/^/    /'
echo "==> pushing"
git push origin HEAD:main

echo
echo "Live in about a minute:  https://bhavit-evo.github.io/evo-boundary-demo/"
echo "Then hard-refresh (Cmd+Shift+R) — Pages caches aggressively."
