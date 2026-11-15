//! `trolllauncher-rs --selftest`: проверка движка без интерфейса.
//! Создаёт временную сборку Fabric, ставит Sodium с Modrinth, читает моды, удаляет сборку.

use theseus::data::{ContentType, ResolutionPreferences};
use theseus::prelude::*;

pub fn run() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let res: Result<(), String> = rt.block_on(async {
        let e = |e: theseus::Error| e.to_string();
        theseus::EventState::init().await.map_err(e)?;
        State::init(crate::backend::APP_ID.to_owned()).await.map_err(e)?;
        println!("[ok] движок запущен");
        println!("[ok] сборок: {}", instance::list().await.map_err(e)?.len());
        println!("[ok] аккаунтов: {}", minecraft_auth::users().await.map_err(e)?.len());

        let job = install::create_instance("selftest-native".into(), "1.21.1".into(), ModLoader::Fabric, None, None, None, theseus::data::InstanceLink::Unmanaged)
            .await
            .map_err(e)?;
        let id = job.instance_id.clone().ok_or("нет id сборки")?;
        println!("[..] создаю сборку {id}");
        for _ in 0..600 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let j = install::get_job(job.job_id.parse().unwrap()).await.map_err(e)?;
            if j.finished.is_some() {
                println!("[ok] установка сборки: {:?}", j.status);
                break;
            }
        }
        let req = instance::InstallProjectWithDependenciesRequest {
            project_id: "AANobbMI".into(), // Sodium
            version_id: None,
            content_type: ContentType::Mod,
            selected: ResolutionPreferences::default(),
        };
        instance::install_project_with_dependencies(&id, req).await.map_err(e)?;
        let mut ok = false;
        for _ in 0..120 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if instance::get_installed_project_ids(&id).await.map_err(e)?.contains(&"AANobbMI".to_string()) {
                ok = true;
                break;
            }
        }
        println!("[{}] Sodium установлен", if ok { "ok" } else { "FAIL" });
        for c in instance::get_content_items(&id, None).await.map_err(e)? {
            println!("     мод: {} {}", c.project.map(|p| p.title).unwrap_or(c.file_name), c.version.map(|v| v.version_number).unwrap_or_default());
        }
        instance::remove(&id).await.map_err(e)?;
        println!("[ok] временная сборка удалена");
        Ok(())
    });
    if let Err(e) = res {
        println!("[FAIL] {e}");
        std::process::exit(1);
    }
}

/// `trolllauncher-rs --import-prism <папка Prism> <имя сборки>`: перенос сборки из Prism/MultiMC
/// движком theseus (как кнопка «Импорт» в Tauri-версии).
pub fn import_prism(base: &str, folder: &str) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let res: Result<(), String> = rt.block_on(async {
        let e = |e: theseus::Error| e.to_string();
        theseus::EventState::init().await.map_err(e)?;
        State::init(crate::backend::APP_ID.to_owned()).await.map_err(e)?;
        let job = install::import_instance(theseus::pack::import::ImportLauncherType::PrismLauncher, base.into(), folder.into())
            .await
            .map_err(e)?;
        println!("[..] импорт {folder} → {:?}", job.instance_id);
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            let j = install::get_job(job.job_id.parse().unwrap()).await.map_err(e)?;
            if j.finished.is_some() {
                println!("[ok] импорт: {:?}", j.status);
                break;
            }
        }
        Ok(())
    });
    if let Err(e) = res {
        println!("[FAIL] {e}");
        std::process::exit(1);
    }
}
