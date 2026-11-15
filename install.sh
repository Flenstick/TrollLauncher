#!/usr/bin/env bash
# Установка TrollLauncher на Linux (x86_64) одной командой:
#
#   curl -fsSL https://raw.githubusercontent.com/Flenstick/TrollLauncher/main/install.sh | bash
#
# Arch и производные (CachyOS, EndeavourOS, Manjaro): собирает пакет trolllauncher-bin из
# packaging/arch/PKGBUILD и ставит через pacman — удаляется чисто.
# Остальные дистрибутивы (Ubuntu, Mint, Fedora, openSUSE…) или с ключом --local: скачивает
# последний релиз, проверяет SHA-256 и кладёт лаунчер в ~/.local/bin + ярлык в меню, без sudo.
# Дальше лаунчер обновляется сам. Повторный запуск команды тоже обновляет.
#
# Удалить:  curl -fsSL https://raw.githubusercontent.com/Flenstick/TrollLauncher/main/install.sh | bash -s -- --uninstall
set -euo pipefail

REPO="Flenstick/TrollLauncher"
BRANCH="main"
PKG="trolllauncher-bin"
BIN_DIR="$HOME/.local/bin"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"

if [[ -t 1 ]]; then
	red=$'\e[38;2;91;87;255m' bold=$'\e[1m' dim=$'\e[2m' reset=$'\e[0m'
else
	red='' bold='' dim='' reset=''
fi

step() { printf '%s==>%s %s%s%s\n' "$red" "$reset" "$bold" "$*" "$reset"; }
fail() { printf '%sошибка:%s %s\n' "$red" "$reset" "$*" >&2; exit 1; }

printf '\n%s  ▟█▙  TrollLauncher%s\n%s  ─────────────────────%s\n\n' "$red$bold" "$reset" "$dim" "$reset"

[[ "$(uname -s)" == "Linux" ]] || fail "этот установщик для Linux; для Windows скачайте exe со страницы релизов"
[[ "$(uname -m)" == "x86_64" ]] || fail "поддерживается только x86_64"
[[ $EUID -ne 0 ]] || fail "запускайте без sudo — пароль спросят, только если понадобится"

mode="auto"
for arg in "$@"; do
	[[ "$arg" == "--local" && "$mode" == "auto" ]] && mode="local"
done
for arg in "$@"; do
	[[ "$arg" == "--uninstall" ]] && mode="uninstall"
done
if [[ "$mode" == "auto" ]]; then
	if command -v pacman >/dev/null; then mode="pacman"; else mode="local"; fi
fi

remove_local() {
	rm -f "$BIN_DIR/trolllauncher" "$DATA/applications/trolllauncher.desktop" "$DATA/icons/hicolor/scalable/apps/trolllauncher.svg"
	for size in 32 64 128 256 512; do
		rm -f "$DATA/icons/hicolor/${size}x${size}/apps/trolllauncher.png"
	done
}

if [[ "$mode" == "uninstall" ]]; then
	step "Удаляю TrollLauncher"
	if command -v pacman >/dev/null && pacman -Q "$PKG" >/dev/null 2>&1; then
		sudo pacman -Rns --noconfirm "$PKG"
	fi
	remove_local
	printf '\nГотово. Сборки, миры и настройки остались в %s/TrollLauncher.\n' "$DATA"
	exit 0
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

if [[ "$mode" == "pacman" ]]; then
	step "Проверяю инструменты сборки пакета"
	if ! command -v fakeroot >/dev/null || ! command -v makepkg >/dev/null; then
		sudo pacman -S --needed --noconfirm fakeroot pacman
	fi
	step "Скачиваю PKGBUILD"
	curl -fsSL "https://raw.githubusercontent.com/$REPO/$BRANCH/packaging/arch/PKGBUILD" -o "$work/PKGBUILD"
	version="$(sed -n 's/^pkgver=//p' "$work/PKGBUILD")"
	installed="$(pacman -Q "$PKG" 2>/dev/null | awk '{print $2}' || true)"
	[[ -n "$installed" ]] && printf '    установлена %s, последняя %s\n' "${installed%-*}" "$version"
	step "Собираю и устанавливаю TrollLauncher $version"
	(cd "$work" && makepkg --syncdeps --install --noconfirm --needed)
	remove_local
	printf '\n%sГотово!%s TrollLauncher есть в меню приложений, или запустите %strolllauncher%s.\n\n' "$bold" "$reset" "$bold" "$reset"
	exit 0
fi

command -v curl >/dev/null || fail "нужен curl"
command -v tar >/dev/null || fail "нужен tar"

step "Ищу последнюю версию"
tag="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
[[ -n "$tag" ]] || fail "не удалось узнать последнюю версию (GitHub недоступен?)"
version="${tag#v}"
name="trolllauncher-$version-x86_64"
base="https://github.com/$REPO/releases/download/$tag"

step "Скачиваю TrollLauncher $version"
curl -fL --progress-bar "$base/$name.tar.gz" -o "$work/$name.tar.gz"
curl -fsSL "$base/$name.tar.gz.sha256" -o "$work/$name.tar.gz.sha256"
(cd "$work" && sha256sum -c --quiet "$name.tar.gz.sha256") || fail "контрольная сумма не совпала — архив повреждён, попробуйте ещё раз"
tar -xzf "$work/$name.tar.gz" -C "$work"

step "Устанавливаю в $BIN_DIR"
src="$work/$name"
install -Dm755 "$src/trolllauncher" "$BIN_DIR/trolllauncher"
install -Dm644 "$src/icons/trolllauncher.svg" "$DATA/icons/hicolor/scalable/apps/trolllauncher.svg"
for size in 32 64 128 256 512; do
	install -Dm644 "$src/icons/$size.png" "$DATA/icons/hicolor/${size}x${size}/apps/trolllauncher.png"
done
mkdir -p "$DATA/applications"
sed "s|^Exec=trolllauncher|Exec=$BIN_DIR/trolllauncher|" "$src/trolllauncher.desktop" > "$DATA/applications/trolllauncher.desktop"
command -v update-desktop-database >/dev/null && update-desktop-database -q "$DATA/applications" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -qtf "$DATA/icons/hicolor" 2>/dev/null || true

# Нужна системная WebKitGTK 4.1 (окно входа Microsoft) и GTK 3
libs="$( (ldconfig -p 2>/dev/null || /sbin/ldconfig -p 2>/dev/null) || true)"
if [[ "$libs" != *libwebkit2gtk-4.1.so* ]]; then
	printf '\n%sНужна библиотека WebKitGTK 4.1:%s\n' "$bold" "$reset"
	if command -v apt >/dev/null; then echo "    sudo apt install libwebkit2gtk-4.1-0"
	elif command -v dnf >/dev/null; then echo "    sudo dnf install webkit2gtk4.1"
	elif command -v zypper >/dev/null; then echo "    sudo zypper install libwebkit2gtk-4_1-0"
	else echo "    пакет webkit2gtk 4.1 вашего дистрибутива"; fi
fi

printf '\n%sГотово!%s TrollLauncher есть в меню приложений.\n' "$bold" "$reset"
case ":$PATH:" in
	*":$BIN_DIR:"*) printf 'Из терминала: %strolllauncher%s\n' "$bold" "$reset" ;;
	*) printf 'Из терминала: %s%s/trolllauncher%s\n' "$bold" "$BIN_DIR" "$reset" ;;
esac
printf '%sДальше лаунчер обновляется сам.%s\n\n' "$dim" "$reset"
