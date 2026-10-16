#!/bin/sh
# Builds Repopus for macOS, Windows and Linux with one command:
#
#   1. macOS: here, on this Mac (`cargo tauri build`, which also exports the public source)
#   2. Windows and Linux: on GitHub's servers (ci/build.yml), from the public repository
#      in export/repopus. Changes are only committed and pushed after you say yes.
#   3. All installers are collected in dist/<version>/
#
# Usage:  sh scripts/build-all.sh
#
# Needs once:
#   * the GitHub CLI:  brew install gh  &&  gh auth login
#   * the public repository on GitHub, connected to export/repopus
#     (see README → "Sharing the source code")
set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO="$ROOT/export/repopus"
TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
VERSION="$(sed -n 's/.*"version": *"\([^"]*\)".*/\1/p' "$ROOT/src-tauri/tauri.conf.json" | head -1)"
OUT="$ROOT/dist/$VERSION"

stop() {
  echo ""
  echo "✋ $1"
  exit 1
}

# ---------------------------------------------------------------- checks first

command -v cargo >/dev/null 2>&1 || stop "Rust isn't installed (see README → Building)."
command -v gh >/dev/null 2>&1 || stop "The GitHub CLI is missing: brew install gh && gh auth login"
gh auth status >/dev/null 2>&1 || stop "The GitHub CLI isn't logged in: gh auth login"

echo "Repopus $VERSION · building macOS, Windows and Linux"
mkdir -p "$OUT"

# ---------------------------------------------------------------- 1. macOS, here

echo ""
echo "==> 1/3  macOS (on this Mac)"
cd "$ROOT"
cargo tauri build
mkdir -p "$OUT/macos"
cp "$TARGET"/release/bundle/dmg/*.dmg "$OUT/macos/"

[ -d "$REPO/.git" ] || stop "export/repopus isn't a git repository yet. Build once, then connect it to GitHub (README → Sharing the source code)."
git -C "$REPO" remote get-url origin >/dev/null 2>&1 || stop "export/repopus has no GitHub remote yet: cd export/repopus && git remote add origin https://github.com/<you>/repopus.git"

# ---------------------------------------------------------------- 2. Windows + Linux, on GitHub

echo ""
echo "==> 2/3  Windows and Linux (on GitHub)"
cd "$REPO"
if [ -n "$(git status --porcelain)" ]; then
  echo "The public source has changes GitHub doesn't have yet:"
  git status --short | head -25
  printf "Commit them as \"Repopus %s\" and push to GitHub? [y/N] " "$VERSION"
  read -r answer
  case "$answer" in
    y | Y | yes | Yes) ;;
    *) stop "Stopped. The macOS build is in dist/$VERSION/macos; nothing was pushed." ;;
  esac
  git add -A
  git commit -q -m "Repopus $VERSION"
fi
BRANCH="$(git branch --show-current)"
git push -q origin "$BRANCH"

latest_run() {
  gh run list --workflow build.yml --limit 1 --json databaseId -q '.[0].databaseId' 2>/dev/null || true
}
BEFORE="$(latest_run)"
gh workflow run build.yml --ref "$BRANCH" || stop "GitHub couldn't start the build. Is ci/build.yml on GitHub as .github/workflows/build.yml?"

# Wait until GitHub shows the new run.
RUN=""
for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do
  sleep 4
  RUN="$(latest_run)"
  [ -n "$RUN" ] && [ "$RUN" != "$BEFORE" ] && break
done
[ -n "$RUN" ] && [ "$RUN" != "$BEFORE" ] || stop "The build didn't show up on GitHub. Look under Actions on github.com."

echo "Building on GitHub (run $RUN). This usually takes 10–20 minutes…"
gh run watch "$RUN" --exit-status >/dev/null || stop "The GitHub build failed. Details: gh run view $RUN --web"

# ---------------------------------------------------------------- 3. collect

echo ""
echo "==> 3/3  Downloading the installers"
for name in windows linux macos-intel; do
  rm -rf "$OUT/$name"
  gh run download "$RUN" --name "repopus-$name" --dir "$OUT/$name"
done

echo ""
echo "✅ Done. Installers for all systems are in dist/$VERSION/:"
cd "$OUT" && find . -type f \( -name "*.dmg" -o -name "*.msi" -o -name "*.exe" -o -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" \) | sed 's#^\./#   #' | sort
