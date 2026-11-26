//! Theseus process management interface

use crate::state::ProcessMetadata;
pub use crate::{
    State,
    state::{Hooks, MemorySettings, Settings, WindowSize},
};
use uuid::Uuid;

// Gets each running stored process in the state
#[tracing::instrument]
pub async fn get_all() -> crate::Result<Vec<ProcessMetadata>> {
    let state = State::get().await?;
    let processes = state.process_manager.get_all();
    Ok(processes)
}

pub async fn resolve_instance_id(instance: &str) -> crate::Result<String> {
    let state = State::get().await?;
    resolve_instance_id_with_state(instance, &state)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "Unknown instance id or path: {instance}"
            ))
            .as_error()
        })
}

async fn resolve_instance_id_with_state(
    instance: &str,
    state: &State,
) -> crate::Result<Option<String>> {
    sqlx::query_scalar!(
        "
		SELECT id
		FROM instances
		WHERE id = ? OR path = ?
		ORDER BY CASE WHEN id = ? THEN 0 ELSE 1 END
		LIMIT 1
		",
        instance,
        instance,
        instance,
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(Into::into)
}

// Gets the UUID of each stored process in the state by instance id
#[tracing::instrument]
pub async fn get_by_instance_id(
    instance_id: &str,
) -> crate::Result<Vec<ProcessMetadata>> {
    let state = State::get().await?;
    let processes = state
        .process_manager
        .get_all()
        .into_iter()
        .filter(|x| x.instance_id == instance_id)
        .collect();
    Ok(processes)
}

// Kill a child process stored in the state by UUID, as a string
#[tracing::instrument]
pub async fn kill(uuid: Uuid) -> crate::Result<()> {
    let state = State::get().await?;
    state.process_manager.kill(uuid).await?;

    Ok(())
}

// Wait for a child process stored in the state by UUID
#[tracing::instrument]
pub async fn wait_for(uuid: Uuid) -> crate::Result<()> {
    let state = State::get().await?;
    state.process_manager.wait_for(uuid).await?;

    Ok(())
}

/// Игра, запущенная прошлым экземпляром лаунчера: процесс ещё жив, но в
/// `process_manager` его нет (лаунчер перезапускали). Берётся из таблицы
/// `processes`, куда пишется каждый запуск.
#[derive(Debug, Clone)]
pub struct DetachedProcess {
    pub instance_id: String,
    pub pid: u32,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

fn alive_since(
    system: &sysinfo::System,
    pid: u32,
    start_time: i64,
) -> bool {
    system
        .process(sysinfo::Pid::from_u32(pid))
        .is_some_and(|p| (p.start_time() as i64).abs_diff(start_time) <= 2)
}

/// Живые игры, которых нет в `process_manager`. Мёртвые записи удаляет.
pub async fn get_detached() -> crate::Result<Vec<DetachedProcess>> {
    let state = State::get().await?;
    let rows: Vec<(i64, i64, String)> = sqlx::query_as(
        "SELECT pid, start_time, instance_id FROM processes",
    )
    .fetch_all(&state.pool)
    .await?;
    if rows.is_empty() {
        return Ok(vec![]);
    }
    let pids: Vec<sysinfo::Pid> = rows
        .iter()
        .filter_map(|r| u32::try_from(r.0).ok().map(sysinfo::Pid::from_u32))
        .collect();
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&pids), true);
    let owned: Vec<String> = state
        .process_manager
        .get_all()
        .into_iter()
        .map(|p| p.instance_id)
        .collect();
    let mut out = vec![];
    for r in rows {
        let alive = u32::try_from(r.0)
            .is_ok_and(|pid| alive_since(&system, pid, r.1));
        if !alive {
            // Процесс из этого экземпляра снимет свою запись сам при выходе.
            if !owned.contains(&r.2) {
                let _ = sqlx::query(
                    "DELETE FROM processes WHERE pid = ? AND start_time = ?",
                )
                .bind(r.0)
                .bind(r.1)
                .execute(&state.pool)
                .await;
            }
            continue;
        }
        if owned.contains(&r.2) {
            continue;
        }
        out.push(DetachedProcess {
            instance_id: r.2,
            pid: r.0 as u32,
            start_time: chrono::DateTime::from_timestamp(r.1, 0)
                .unwrap_or_else(chrono::Utc::now),
        });
    }
    Ok(out)
}

/// Остановить игры сборки, запущенные прошлым экземпляром лаунчера
/// (вместе с дочерними процессами — обёртки вроде gamemoderun/mangohud).
pub async fn kill_detached(instance_id: &str) -> crate::Result<()> {
    let detached: Vec<DetachedProcess> = get_detached()
        .await?
        .into_iter()
        .filter(|p| p.instance_id == instance_id)
        .collect();
    if detached.is_empty() {
        return Ok(());
    }
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    for d in detached {
        let root = sysinfo::Pid::from_u32(d.pid);
        let mut victims = vec![root];
        let mut i = 0;
        while i < victims.len() {
            let parent = victims[i];
            victims.extend(
                system
                    .processes()
                    .iter()
                    .filter(|(_, p)| p.parent() == Some(parent))
                    .map(|(pid, _)| *pid),
            );
            i += 1;
        }
        for pid in victims.into_iter().rev() {
            if let Some(p) = system.process(pid) {
                p.kill();
            }
        }
    }
    Ok(())
}
