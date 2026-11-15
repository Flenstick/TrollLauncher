//! TrollLauncher — нативный интерфейс на Rust (egui + wgpu) поверх движка theseus.
#![allow(deprecated)]
#![cfg_attr(windows, windows_subsystem = "windows")]
#![recursion_limit = "256"]

mod app;
mod backend;
mod cats;
mod downloads;
mod instcfg;
mod importing;
mod filedrop;
mod modinfo;
mod msa;
mod navigation;
mod pages;
mod prefs;
mod selftest;
mod sfskin;
mod skin3d;
mod sidebar_instances;
mod sys;
mod theme;
mod update;
mod widgets;
#[cfg(windows)]
mod winstall;

use eframe::egui;

fn main() -> eframe::Result {
    update::wait_previous();
    let args: Vec<String> = std::env::args().collect();
    #[cfg(windows)]
    {
        if args.get(1).map(String::as_str) == Some("--uninstall") {
            winstall::uninstall();
            return Ok(());
        }
        if args.len() == 1 && winstall::ensure_installed() {
            return Ok(());
        }
    }
    if args.get(1).map(String::as_str) == Some("--msa-login") {
        msa::run(args.get(2).map(String::as_str).unwrap_or_default());
    }
    if args.get(1).map(String::as_str) == Some("--selftest") {
        selftest::run();
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--self-update") {
        // обновить лаунчер из терминала: trolllauncher --self-update
        let rt = tokio::runtime::Runtime::new().expect("tokio");
        rt.block_on(async {
            match update::check().await {
                Ok(Some(info)) => {
                    println!("TrollLauncher {} → {}, ставлю…", info.current, info.version);
                    match update::install(&info).await {
                        Ok(()) => println!("готово, запустите лаунчер заново"),
                        Err(e) => println!("не получилось: {e}"),
                    }
                }
                Ok(None) => println!("стоит последняя версия {}", update::current_version()),
                Err(e) => println!("не удалось проверить: {e}"),
            }
        });
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--sfatm-update") {
        // проверка автообновления sfatm на любой папке сборки (там, где лежит mods/)
        let dir = std::path::PathBuf::from(args.get(2).cloned().unwrap_or_default());
        let rt = tokio::runtime::Runtime::new().expect("tokio");
        match rt.block_on(theseus::sfatm_update::update(&dir)) {
            Ok(Some(u)) => println!("обновлено: {} → {}", u.from, u.to),
            Ok(None) => println!("обновлять нечего"),
            Err(e) => println!("ошибка: {e}"),
        }
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--import-prism") {
        selftest::import_prism(args.get(2).map(String::as_str).unwrap_or_default(), args.get(3).map(String::as_str).unwrap_or_default());
        return Ok(());
    }
    let _log = theseus::start_logger(backend::APP_ID);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TrollLauncher")
            .with_app_id(
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                    .filter(|s| s.starts_with("trolllauncher"))
                    .unwrap_or_else(|| "trolllauncher".into()),
            )
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([980.0, 640.0])
            .with_decorations(false)
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../../../packaging/linux/icons/256.png")).unwrap_or_default()),
        ..Default::default()
    };
    eframe::run_native("TrollLauncher", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
