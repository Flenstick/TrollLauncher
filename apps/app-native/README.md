# TrollLauncher (Rust)

Нативный интерфейс TrollLauncher: **egui 0.37 + wgpu**, без WebKit/JS в основном окне.
Под капотом — тот же движок **theseus** (`packages/app-lib`), что и у TrollLauncher на Tauri:
общие сборки, аккаунты, настройки памяти и библиотека скинов (`~/.local/share/TrollLauncher`).

## Сборка и установка

    apps/app-native/install.sh        # собирает профиль native и кладёт в ~/.local/bin

Сборка идёт с фоновым приоритетом (CPUWeight=idle, ionice idle, ядра − 4) и не тормозит систему.
На полной скорости: `FAST=1 apps/app-native/install.sh`; число потоков — `JOBS=8`.

Java 17 обязательна для сборки (`JAVA_HOME=/usr/lib/jvm/java-17-openjdk`): форматтер
gradle в `packages/app-lib/java` падает на Java 21.0.12+ и 27.

## Что работает

- Сборки: список, запуск/остановка, время в игре, создание (Vanilla/Fabric/Forge/NeoForge/Quilt),
  переименование, удаление, открыть папку, **фото на аватарку** (клик по иконке сборки).
- Контент сборки: моды/ресурспаки/шейдеры/датапаки, вкл/выкл, удаление, обновление (по одному и всё сразу).
- Обзор Modrinth: поиск, сортировка, фильтр под версию/загрузчик сборки, установка с зависимостями,
  установка сборок (модпаков) как новых сборок.
- **Окно мода**: клик по строке контента — описание с Modrinth (Markdown, картинки), изменения версии, файл,
  ссылки, вкл/выкл, обновить, удалить (`modinfo.rs`).
- **Меню «⋯» сборки** (`instcfg.rs`): настройки сборки (своя память, аргументы Java, путь к Java, переменные окружения,
  полноэкранный режим и размер окна, хуки) и **экспорт** в Prism Launcher (zip: `mmc-pack.json` + `instance.cfg` + `minecraft/`),
  Modrinth (`.mrpack` движка) и CurseForge (zip: `manifest.json` + `overrides/`, моды файлами). Проверка без окна:
  `TL_PAGE=instance TL_DIALOG=export TL_EXPORT_TO=папка TL_EXPORT_FMT=prism|modrinth|curseforge`.
- Аккаунты: вход Microsoft (лицензия; окно входа — отдельный процесс с WebKit), Ely.by, выбор и выход.
- Скины: библиотека, **3D-модели** (крутится мышью, ходьба), Classic/Slim, избранное, PNG-файл,
  скин по нику Minecraft/Ely.by, «надеть» для Microsoft-аккаунта (Ely.by — через сайт).
- Внешний вид: **24 темы** (TrollLand, Midnight, Inferno, Шабаш, Catppuccin, Nord, Dracula, Gruvbox, Tokyo Night,
  светлые и др., `theme.rs` → `THEMES`), свой акцент поверх темы, масштаб.
- **Котики как в Prism Launcher** (`cats.rs`): 4 встроенных набора с праздничными вариантами, свои — картинки, GIF,
  папки и `catpack.json` из `~/.local/share/TrollLauncher/catpacks` и `~/.local/share/PrismLauncher/catpacks`;
  прозрачность и размещение (в углу / вписать / заполнить / растянуть). При первом запуске котик берётся из Prism.
- **Производительность как в Prism** (Настройки → Производительность): флаги Java (G1 Aikar / ZGC), «вся память сразу»,
  Feral GameMode, MangoHud — общие с TrollLauncher (`performance.json` движка); свернуть при запуске игры и вернуть после.
- **Тихий режим во время игры** (`App::calm()`): без пульса на плашке, GIF-котик на первом кадре, скин не крутится,
  перерисовка раз в секунду, список сборок раз в 30 с. Раньше плашка «играет» перерисовывала окно 30 раз/с (~30 % ядра).
- Настройки интерфейса хранятся в `~/.local/share/TrollLauncher/native-ui.json` (`prefs.rs`).
- Журнал (`logs/latest.log`, живое обновление во время игры), скриншоты сборки.

## Вход Ely.by через браузер

Приложение на Ely.by — тип **«Настольное приложение»** (публичный клиент: без секрета, с PKCE, redirect на любой
порт 127.0.0.1). Его Client ID вшит константой `ELYBY_CLIENT_ID` в `packages/app-lib/src/state/elyby.rs`;
переопределить можно переменной `ELYBY_CLIENT_ID` или файлом `~/.local/share/TrollLauncher/elyby-oauth.json`
(`{"client_id": "…"}`).

«Войти через Ely.by» открывает страницу Ely.by, лаунчер слушает свободный порт на 127.0.0.1 и ловит ответ, меняет код на
токены (PKCE S256) и сохраняет аккаунт (в базе `refresh_token` = `oauth:…`, токен продлевается сам). Без Client ID
остаётся вход по паролю.

## Проверка без интерфейса

    target/debug/trolllauncher-rs --selftest

создаёт временную Fabric-сборку, ставит Sodium с Modrinth, читает моды и удаляет сборку.

Для скриншотов: `TL_PAGE=library|browse|skins|settings|instance TL_TAB=N`, окна — `TL_DIALOG=modinfo|settings|export|ely` (`TL_MOD=имя`).
Тестовый запуск рядом с основным лаунчером: reflink-копия данных в `~/.cache/tltest/TrollLauncher`, в её `app.db` — `custom_dir` на копию и `prev_custom_dir=NULL`, запуск с `XDG_DATA_HOME=~/.cache/tltest`.

## Устройство

- `backend.rs` — вызовы theseus в tokio, ответы в интерфейс сообщениями `Msg`.
- `skin3d.rs` — 3D-игрок: кубы проецируются на CPU в меш egui, текстура с NEAREST.
- `msa.rs` — окно входа Microsoft (`--msa-login URL`, tao + wry), печатает `CODE=…`.
- `selftest.rs`, `app.rs` (каркас, диалоги, котик), `pages/*`, `widgets.rs`, `theme.rs` (темы — цвета через `TEXT()` и т.п.), `cats.rs`, `prefs.rs`.
