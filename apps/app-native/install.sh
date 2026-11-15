#!/bin/bash
# Сборка и установка TrollLauncher (Rust) в ~/.local/bin (пункт меню уже указывает туда).
# Java 17 нужна для Java-части движка: форматтер gradle не работает с Java 21.0.12+/27.
#
# Сборка идёт фоном и не тормозит систему: CPUWeight=idle (процессор только когда он свободен),
# низкий приоритет диска, не больше JOBS потоков (по умолчанию ядра − 4).
# Быстро на полной мощности: FAST=1 apps/app-native/install.sh
set -e
cd "$(dirname "$0")/../.."
JOBS=${JOBS:-$(( $(nproc) > 6 ? $(nproc) - 4 : 2 ))}
build=(env JAVA_HOME=/usr/lib/jvm/java-17-openjdk cargo build --profile native -p trolllauncher-native -j "$JOBS")
if [ -n "$FAST" ]; then
    "${build[@]}"
elif command -v systemd-run >/dev/null; then
    systemd-run --user --scope --quiet -p CPUWeight=idle -p IOWeight=1 \
        nice -n 19 ionice -c 3 "${build[@]}"
else
    nice -n 19 ionice -c 3 "${build[@]}"
fi
install -m755 target/native/trolllauncher-rs ~/.local/bin/trolllauncher-rs
echo "Готово: ~/.local/bin/trolllauncher-rs"
