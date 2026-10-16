#!/bin/sh
# Copies the public source code of Repopus into export/repopus/, ready to push to GitHub.
#
# Runs automatically before every `cargo tauri build` on macOS and Linux
# (tauri.macos.conf.json / tauri.linux.conf.json → beforeBuildCommand),
# and can also be run by hand:  sh scripts/export-source.sh
#
# The export folder keeps its own .git history: each export updates the files in place,
# so you can review the changes and commit them like in any other repository.
set -e

# Build servers (GitHub Actions etc.) don't need a source export.
if [ -n "$CI" ]; then
  exit 0
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/export/repopus"

mkdir -p "$DEST"

# Everything except build output, generated files, private/local files and the export itself.
# (--delete removes files from the export that no longer exist in the project; .git is kept.)
rsync -a --delete \
  --exclude ".git/" \
  --exclude "export/" \
  --exclude "dist/" \
  --exclude "target/" \
  --exclude "gen/" \
  --exclude ".DS_Store" \
  --exclude "*.log" \
  --exclude ".env" \
  --exclude ".env.*" \
  --exclude ".vscode/" \
  --exclude ".idea/" \
  "$ROOT/" "$DEST/"

# The GitHub Actions workflow lives in ci/ in the project and goes to its proper place in
# the public repository (.github/workflows/), where GitHub picks it up.
mkdir -p "$DEST/.github/workflows"
cp "$ROOT/ci/build.yml" "$DEST/.github/workflows/build.yml"

# Turn it into a git repository the first time (nothing is committed or pushed automatically).
if command -v git >/dev/null 2>&1 && [ ! -d "$DEST/.git" ]; then
  git -C "$DEST" init -q
  git -C "$DEST" checkout -q -b main 2>/dev/null || true
  echo "Created a new git repository in export/repopus"
fi

echo "Public source code exported to export/repopus"
