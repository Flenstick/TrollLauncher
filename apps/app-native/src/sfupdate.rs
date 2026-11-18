//! Автообновление нашего мода sfatm во всех сборках, где он лежит в `mods/`.
//!
//! - Проверка при старте лаунчера, раз в 20 минут и после выхода из игры (latest.json кэшируется
//!   движком на 10 минут, так что сборок может быть сколько угодно — запрос один).
//! - Скачивание видно в «Загрузках» справа сверху: прогресс в МБ, ошибка, «Повторить».
//! - Если в сборке идёт игра, мод не трогаем (движок всё равно запретит менять моды):
//!   запись «Обновится после выхода из игры», а после остановки обновление ставится само.
//! - Перед запуском сборки обновление делается обязательно и с прогрессом; если оно уже
//!   качается фоном — запуск ждёт его, а не качает второй раз.

use crate::backend::{Backend, Msg};
use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};
use theseus::prelude::*;
use theseus::sfatm_update::{self as sfu, Manifest};

#[derive(Clone, Debug, PartialEq)]
pub enum SfState {
    Downloading { done: u64, total: u64 },
    /// Игра запущена — поставим после выхода
    Waiting,
    Done,
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct SfJob {
    pub inst: String,
    pub name: String,
    pub from: String,
    pub to: String,
    pub state: SfState,
    pub at: Instant,
}

static JOBS: Mutex<Vec<SfJob>> = Mutex::new(Vec::new());

/// Готовые записи сами уходят из «Загрузок» через столько времени.
const DONE_TTL: Duration = Duration::from_secs(12);

pub fn jobs() -> Vec<SfJob> {
    let mut j = JOBS.lock().unwrap();
    j.retain(|x| !(x.state == SfState::Done && x.at.elapsed() > DONE_TTL));
    j.clone()
}

/// Сколько записей показывать на значке «Загрузок».
pub fn badge() -> usize {
    jobs().iter().filter(|j| j.state != SfState::Done).count()
}

pub fn downloading() -> bool {
    JOBS.lock().unwrap().iter().any(|j| matches!(j.state, SfState::Downloading { .. }))
}

pub fn dismiss(inst: &str) {
    JOBS.lock().unwrap().retain(|j| j.inst != inst);
}

fn set(inst: &str, name: &str, from: &str, to: &str, state: SfState) {
    let mut j = JOBS.lock().unwrap();
    let job = SfJob { inst: inst.into(), name: name.into(), from: from.into(), to: to.into(), state, at: Instant::now() };
    match j.iter_mut().find(|x| x.inst == inst) {
        Some(x) => *x = job,
        None => j.push(job),
    }
}

fn state_of(inst: &str) -> Option<SfState> {
    JOBS.lock().unwrap().iter().find(|j| j.inst == inst).map(|j| j.state.clone())
}

/// Версия sfatm из имени файла (`sfatm-1.32.123.jar`, в том числе выключенного).
pub fn file_version(file_name: &str) -> Option<&str> {
    let v = file_name.strip_prefix("sfatm-")?;
    let v = v.strip_suffix(".jar.disabled").or_else(|| v.strip_suffix(".jar"))?;
    v.starts_with(|c: char| c.is_ascii_digit()).then_some(v)
}

/// Свежая версия, если latest.json уже скачан (без сети — для списка модов).
pub fn known_latest() -> Option<String> {
    LATEST.lock().unwrap().clone()
}
static LATEST: Mutex<Option<String>> = Mutex::new(None);

async fn manifest(fresh: bool) -> Option<Manifest> {
    match sfu::manifest(fresh).await {
        Ok(m) => {
            *LATEST.lock().unwrap() = m.as_ref().map(|m| m.version.clone());
            m
        }
        Err(e) => {
            eprintln!("sfatm: не удалось проверить обновление: {e}");
            None
        }
    }
}

/// Обновить одну сборку. `Ok(None)` — обновлять нечего или уже обновляется другим потоком.
async fn update_one(id: &str, m: &Manifest) -> Result<Option<sfu::Updated>, String> {
    let dir = instance::get_full_path(id).await.map_err(|e| e.to_string())?;
    let Some(old) = sfu::installed_version(&dir) else { return Ok(None) };
    if !sfu::newer(&m.version, &old) {
        return Ok(None);
    }
    let name = instance::get(id).await.ok().flatten().map(|i| i.instance.name).unwrap_or_else(|| id.to_string());
    {
        let mut j = JOBS.lock().unwrap();
        if j.iter().any(|x| x.inst == id && matches!(x.state, SfState::Downloading { .. })) {
            return Ok(None);
        }
        j.retain(|x| x.inst != id);
        j.push(SfJob { inst: id.into(), name: name.clone(), from: old.clone(), to: m.version.clone(), state: SfState::Downloading { done: 0, total: m.size }, at: Instant::now() });
    }
    let res = sfu::install(&dir, m, |done, total| {
        if let Some(x) = JOBS.lock().unwrap().iter_mut().find(|x| x.inst == id) {
            x.state = SfState::Downloading { done, total };
        }
    })
    .await;
    match res {
        Ok(u) => {
            let _ = instance::sync_content_files(id).await;
            set(id, &name, &old, &m.version, SfState::Done);
            Ok(u)
        }
        Err(e) => {
            let msg = e.to_string();
            set(id, &name, &old, &m.version, SfState::Failed(msg.clone()));
            Err(msg)
        }
    }
}

/// Ждать, пока фоновое обновление этой сборки закончится (не дольше 10 минут).
async fn wait_running(id: &str) -> Option<SfState> {
    if !matches!(state_of(id), Some(SfState::Downloading { .. })) {
        return None;
    }
    let t = Instant::now();
    while matches!(state_of(id), Some(SfState::Downloading { .. })) && t.elapsed() < Duration::from_secs(600) {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    state_of(id)
}

/// Перед запуском: обновить sfatm в этой сборке (с прогрессом). Возвращает текст для уведомления.
pub async fn before_launch(id: &str) -> Option<(String, bool)> {
    if let Some(state) = wait_running(id).await {
        return match state {
            SfState::Failed(e) => Some((format!("Не удалось обновить sfatm, запускаю старую версию: {e}"), true)),
            _ => None,
        };
    }
    let dir = instance::get_full_path(id).await.ok()?;
    sfu::installed_version(&dir)?;
    let m = manifest(false).await?;
    match update_one(id, &m).await {
        Ok(Some(u)) => Some((format!("sfatm обновлён: {} → {}", u.from, u.to), false)),
        Ok(None) => None,
        Err(e) => Some((format!("Не удалось обновить sfatm, запускаю старую версию: {e}"), true)),
    }
}

impl Backend {
    /// Проверить и обновить sfatm в сборках `ids`. В запущенных — отложить до выхода из игры.
    /// `manual` — по кнопке: мимо кэша и даже при выключенном автообновлении.
    pub fn sfatm_sync(&self, ids: Vec<String>, running: HashSet<String>, manual: bool) {
        self.task(move |tx: Sender<Msg>| async move {
            if !manual && !PerformanceSettings::get().await.sfatm_autoupdate {
                return;
            }
            let fresh = manual;
            let Some(m) = manifest(fresh).await else { return };
            let mut done = vec![];
            for id in ids {
                let Ok(dir) = instance::get_full_path(&id).await else { continue };
                let Some(old) = sfu::installed_version(&dir) else {
                    dismiss(&id);
                    continue;
                };
                if !sfu::newer(&m.version, &old) {
                    if matches!(state_of(&id), Some(SfState::Waiting)) {
                        dismiss(&id);
                    }
                    continue;
                }
                if running.contains(&id) {
                    let name = instance::get(&id).await.ok().flatten().map(|i| i.instance.name).unwrap_or_default();
                    set(&id, &name, &old, &m.version, SfState::Waiting);
                    continue;
                }
                if let Ok(Some(u)) = update_one(&id, &m).await {
                    done.push(u.to);
                    let _ = tx.send(Msg::InstallDone(id.clone()));
                }
            }
            if let Some(v) = done.first() {
                let n = done.len();
                let text = if n == 1 { format!("sfatm обновлён до {v}") } else { format!("sfatm обновлён до {v} в сборках: {n}") };
                let _ = tx.send(Msg::Toast(text, false));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions_from_file_names() {
        assert_eq!(super::file_version("sfatm-1.32.123.jar"), Some("1.32.123"));
        assert_eq!(super::file_version("sfatm-1.32.123.jar.disabled"), Some("1.32.123"));
        assert_eq!(super::file_version("sfatm-core.jar"), None);
        assert_eq!(super::file_version("create-6.0.jar"), None);
    }
}
