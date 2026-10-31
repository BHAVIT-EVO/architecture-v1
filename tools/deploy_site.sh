#!/usr/bin/env bash
# Publish the site to the public demo repository.
#
#   bash tools/deploy_site.sh
#
# The demo repo is separate from this one on purpose: this repository stays
# private because boundary.json holds one person's real browsing. The public
# repo only ever receives the four published artifacts.
#
# Requires: git credentials that can push to BHAVIT-EVO/evo-boundary-demo.
set -euo pipefail

REPO_URL="${EVO_DEMO_REPO:-https://github.com/BHAVIT-EVO/evo-boundary-demo}"
SITE="$(cd "$(dirname "$0")/.." && pwd)/site"
WORK="${TMPDIR:-/tmp}/evo-boundary-demo"

if [ ! -f "$SITE/index.html" ]; then
  echo "site/index.html is missing — run: python3 tools/build_site_v2.py" >&2
  exit 1
fi

echo "==> cloning $REPO_URL"
rm -rf "$WORK"
git clone --depth 1 "$REPO_URL" "$WORK"

echo "==> copying artifacts"
cp "$SITE/index.html" "$SITE/demo.html" "$SITE/film.html" "$WORK/"
mkdir -p "$WORK/fonts"
cp "$SITE"/fonts/*.woff2 "$WORK/fonts/"

cd "$WORK"
git add -A
if git diff --cached --quiet; then
  echo "==> nothing changed; the live site is already current"
  exit 0
fi

git -c user.name="Evo" -c user.email="evo@local" commit -q -m "site: publish $(date -u +%Y-%m-%dT%H:%MZ)"
echo "==> pushing"
git push origin HEAD:main

echo
echo "Live in about a minute:  https://bhavit-evo.github.io/evo-boundary-demo/"
