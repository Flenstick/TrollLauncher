#!/usr/bin/env bash
# TrollLauncher installer for Arch Linux and Arch-based distros (CachyOS, EndeavourOS, Manjaro…).
#
#   curl -fsSL https://raw.githubusercontent.com/Flenstick/TrollLauncher/main/install.sh | bash
#
# Builds the trolllauncher-bin package from packaging/arch/PKGBUILD and installs it with pacman,
# so the launcher can be removed cleanly. Re-run the same command to update.
# Remove:  curl -fsSL https://raw.githubusercontent.com/Flenstick/TrollLauncher/main/install.sh | bash -s -- --uninstall
set -euo pipefail

REPO="Flenstick/TrollLauncher"
BRANCH="main"
PKG="trolllauncher-bin"

if [[ -t 1 ]]; then
	red=$'\e[38;2;229;72;77m' bold=$'\e[1m' dim=$'\e[2m' reset=$'\e[0m'
else
	red='' bold='' dim='' reset=''
fi

step() { printf '%s==>%s %s%s%s\n' "$red" "$reset" "$bold" "$*" "$reset"; }
fail() { printf '%sошибка:%s %s\n' "$red" "$reset" "$*" >&2; exit 1; }

printf '\n%s  ▟█▙  TrollLauncher%s\n%s  ─────────────────────%s\n\n' "$red$bold" "$reset" "$dim" "$reset"

command -v pacman >/dev/null || fail "нужен pacman — установщик работает только на Arch Linux и производных"
[[ "$(uname -m)" == "x86_64" ]] || fail "поддерживается только x86_64"
[[ $EUID -ne 0 ]] || fail "запускайте без sudo — пароль спросят, когда понадобится"

if [[ "${1:-}" == "--uninstall" ]]; then
	step "Удаляю TrollLauncher"
	sudo pacman -Rns --noconfirm "$PKG"
	printf '\nГотово. Сборки, миры и настройки остались в ~/.local/share/TrollLauncher.\n'
	exit 0
fi

step "Проверяю инструменты сборки пакета"
if ! command -v fakeroot >/dev/null || ! command -v makepkg >/dev/null; then
	sudo pacman -S --needed --noconfirm fakeroot pacman
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

step "Скачиваю PKGBUILD"
curl -fsSL "https://raw.githubusercontent.com/$REPO/$BRANCH/packaging/arch/PKGBUILD" -o "$work/PKGBUILD"
version="$(sed -n 's/^pkgver=//p' "$work/PKGBUILD")"

installed="$(pacman -Q "$PKG" 2>/dev/null | awk '{print $2}' || true)"
if [[ -n "$installed" ]]; then
	printf '    установлена %s, последняя %s\n' "${installed%-*}" "$version"
fi

step "Собираю и устанавливаю TrollLauncher $version"
(cd "$work" && makepkg --syncdeps --install --noconfirm --needed)

printf '\n%sГотово!%s TrollLauncher есть в меню приложений, или запустите %strolllauncher%s.\n' \
	"$bold" "$reset" "$bold" "$reset"
printf '%sОбновить — запустите эту же команду ещё раз.%s\n\n' "$dim" "$reset"
