#!/usr/bin/env bash
# Zip everything in target/bundled (plugin bundles + standalone app) together
# with the license files into dist/infinati-kick-<platform>.zip.
# Usage: scripts/package.sh <platform>   e.g. linux-x64, windows-x64, macos-universal
set -euo pipefail

platform="${1:?usage: package.sh <platform>}"
root="$(cd "$(dirname "$0")/.." && pwd)"
stage="$root/dist/infinati-kick-$platform"
zip="$root/dist/infinati-kick-$platform.zip"

rm -rf "$stage" "$zip"
mkdir -p "$stage"
cp -R "$root/target/bundled/." "$stage/"
cp "$root/LICENSE" "$root/NOTICE" "$root/README.md" "$stage/"

cd "$root/dist"
if [[ "$(uname)" == "Darwin" ]]; then
    # ditto keeps bundle structure, symlinks and permissions intact.
    ditto -c -k --keepParent "infinati-kick-$platform" "$zip"
else
    cmake -E tar cf "$zip" --format=zip "infinati-kick-$platform"
fi
echo "created $zip"
