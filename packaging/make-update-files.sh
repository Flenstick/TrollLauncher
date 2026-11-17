#!/usr/bin/env bash
# Файлы быстрого автообновления (лаунчер 2.0.6+, см. apps/app-native/src/update.rs):
#   <bin>.zst                — исполняемый файл, сжатый zstd (87 МБ → ~24 МБ)
#   <bin>.sha256             — sha256 распакованного файла
#   <bin>.from-<старая>.zst  — патч от каждой из последних N версий (zstd --patch-from, ~9 МБ)
# где <bin> = trolllauncher-<v>-x86_64.bin (linux) или TrollLauncher-<v>-windows-x86_64.exe (windows).
#
# Использование: packaging/make-update-files.sh linux|windows <исполняемый файл> <версия> <папка> [N=3]
# Старые версии берутся из релизов Flenstick/TrollLauncher (нужен gh). Запускать ДО публикации нового
# релиза или когда он уже есть — его самого среди «старых» не берём.
set -euo pipefail

platform=$1 binary=$2 version=$3 out=$4 keep=${5:-3}
repo=Flenstick/TrollLauncher
command -v zstd >/dev/null || { echo "нужен zstd" >&2; exit 1; }
mkdir -p "$out"

name() { if [ "$platform" = windows ]; then echo "TrollLauncher-$1-windows-x86_64.exe"; else echo "trolllauncher-$1-x86_64.bin"; fi; }
bin=$(name "$version")

# --long=30 и --patch-from: окно должно покрывать весь файл; лаунчер распаковывает с window_log_max 31
zstd -19 --long=30 -T0 -q -f "$binary" -o "$out/$bin.zst"
(cd "$(dirname "$binary")" && sha256sum "$(basename "$binary")") | awk -v n="$bin" '{print $1"  "n}' > "$out/$bin.sha256"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
gh release list --repo "$repo" --exclude-drafts --exclude-pre-releases --limit 20 --json tagName -q '.[].tagName' \
  | sed 's/^v//' | grep -vx "$version" | head -n "$keep" | while read -r old; do
    base="$tmp/$old"
    if [ "$platform" = windows ]; then
        gh release download "v$old" --repo "$repo" -p "$(name "$old")" -O "$base" 2>/dev/null || { echo "нет exe $old — пропускаю"; continue; }
    elif gh release download "v$old" --repo "$repo" -p "$(name "$old").zst" -O "$base.zst" 2>/dev/null; then
        zstd -d -q --long=31 "$base.zst" -o "$base"
    else
        gh release download "v$old" --repo "$repo" -p "trolllauncher-$old-x86_64.tar.gz" -O "$base.tgz" 2>/dev/null || { echo "нет tarball $old — пропускаю"; continue; }
        tar -xzf "$base.tgz" -C "$tmp" && mv "$tmp/trolllauncher-$old-x86_64/trolllauncher" "$base"
    fi
    zstd -19 -T0 -q -f --patch-from="$base" "$binary" -o "$out/$bin.from-$old.zst"
    echo "патч $old → $version: $(du -h "$out/$bin.from-$old.zst" | cut -f1)"
done
ls -la "$out"/"$bin"*
