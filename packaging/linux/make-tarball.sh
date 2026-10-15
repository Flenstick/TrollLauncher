#!/usr/bin/env bash
# Packs the release binary into the tarball that the Arch package downloads.
# Usage: packaging/linux/make-tarball.sh [output-dir]   (run after `pnpm tauri build --no-bundle` in apps/app)
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/apps/app/Cargo.toml" | head -n1)"
out="${1:-$root/target/dist}"
name="trolllauncher-$version-x86_64"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

dir="$stage/$name"
mkdir -p "$dir/icons"
install -Dm755 "$root/target/release/theseus_gui" "$dir/trolllauncher"
install -Dm644 "$root/packaging/linux/trolllauncher.desktop" "$dir/trolllauncher.desktop"
install -Dm644 "$root/packaging/linux/trolllauncher.svg" "$dir/icons/trolllauncher.svg"
cp "$root"/packaging/linux/icons/*.png "$dir/icons/"
install -Dm644 "$root/apps/app/LICENSE" "$dir/LICENSE"

mkdir -p "$out"
tar -C "$stage" -czf "$out/$name.tar.gz" "$name"
sha256sum "$out/$name.tar.gz"
