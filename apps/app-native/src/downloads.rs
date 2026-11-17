use crate::{
    app::App,
    backend::{Backend, Job, Msg},
    theme::*,
    widgets::*,
};
use crate::sfupdate::{SfJob, SfState};
use eframe::egui::{self, Sense, Ui, vec2};
use theseus::install::{
    self, InstallJobSnapshot, InstallJobStatus, InstallPhaseId,
};

#[derive(Clone, Copy)]
pub enum JobAction {
    Retry,
    Cancel,
    Dismiss,
}

fn progress_fraction(progress: &install::InstallProgress) -> Option<f32> {
    let (current, total) = progress
        .secondary
        .as_ref()
        .filter(|bytes| bytes.total > 0)
        .map(|bytes| (bytes.current, bytes.total))
        .unwrap_or((progress.current, progress.total));
    (total > 0).then(|| (current as f32 / total as f32).clamp(0.0, 1.0))
}

impl Job {
    pub fn from_snapshot(snapshot: InstallJobSnapshot) -> Self {
        let progress_label = snapshot
            .progress
            .as_ref()
            .map(|p| {
                let mut text = format!("{} / {}", p.current, p.total);
                if let Some(bytes) = &p.secondary {
                    text.push_str(&format!(
                        " · {:.1} / {:.1} МБ",
                        bytes.current as f64 / 1_048_576.0,
                        bytes.total as f64 / 1_048_576.0
                    ));
                }
                text
            })
            .unwrap_or_default();
        Self {
            id: snapshot.job_id,
            title: snapshot
                .display
                .map(|d| d.title)
                .unwrap_or_else(|| "Установка".into()),
            progress: snapshot.progress.as_ref().and_then(progress_fraction),
            status: snapshot.status,
            phase: snapshot.phase,
            progress_label,
            error: snapshot
                .rollback_error
                .or(snapshot.error)
                .map(|error| error.message),
            can_cancel: snapshot.can_cancel,
            canceling: snapshot.canceling,
            paused: snapshot.paused,
            instance_id: snapshot.instance_id,
        }
    }

    fn status_label(&self) -> &'static str {
        if self.canceling {
            return "Отмена установки…";
        }
        if self.paused {
            return "Приостановлено";
        }
        match self.status {
            InstallJobStatus::Failed => "Ошибка установки",
            InstallJobStatus::Interrupted => "Установка прервана",
            InstallJobStatus::Canceled => "Установка отменена",
            InstallJobStatus::Succeeded => "Установка завершена",
            InstallJobStatus::Queued => "Подготовка установки…",
            InstallJobStatus::Running => match self.phase {
                InstallPhaseId::PreparingInstance => "Создание сборки",
                InstallPhaseId::ResolvingPack => "Проверка состава сборки",
                InstallPhaseId::DownloadingPackFile => "Скачивание архива",
                InstallPhaseId::ReadingPackManifest => "Чтение архива сборки",
                InstallPhaseId::DownloadingContent => {
                    "Скачивание модов и ресурсов"
                }
                InstallPhaseId::ExtractingOverrides => {
                    "Распаковка конфигов и файлов"
                }
                InstallPhaseId::ResolvingMinecraft => {
                    "Получение версии Minecraft"
                }
                InstallPhaseId::ResolvingLoader => "Получение загрузчика модов",
                InstallPhaseId::PreparingJava => "Подготовка Java",
                InstallPhaseId::DownloadingMinecraft => {
                    "Скачивание файлов Minecraft"
                }
                InstallPhaseId::RunningLoaderProcessors => {
                    "Установка загрузчика модов"
                }
                InstallPhaseId::Finalizing => "Завершение установки",
                InstallPhaseId::RollingBack => "Восстановление после ошибки",
            },
        }
    }
}

impl App {
    /// Обновление sfatm в «Загрузках»: прогресс, ожидание выхода из игры, ошибка с «Повторить».
    pub fn sfatm_job_ui(&mut self, ui: &mut Ui, job: &SfJob) {
        label(ui, format!("sfatm {} → {} · {}", job.from, job.to, job.name), semibold(14.0), TEXT());
        match &job.state {
            SfState::Downloading { done, total } => {
                label(ui, "Обновление sfatm", regular(13.0), TEXT_DIM());
                let mb = |b: u64| b as f64 / 1_048_576.0;
                if *total > 0 {
                    label(ui, format!("{:.1} / {:.1} МБ", mb(*done), mb(*total)), regular(12.0), TEXT_DIM());
                    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 4.0), Sense::hover());
                    progress_bar(ui.painter(), rect, (*done as f32 / *total as f32).clamp(0.0, 1.0));
                } else {
                    ui.spinner();
                }
            }
            SfState::Waiting => {
                label(ui, "Обновится сам после выхода из игры", regular(13.0), TEXT_DIM());
            }
            SfState::Done => {
                label(ui, "sfatm обновлён", regular(13.0), TEXT_DIM());
            }
            SfState::Failed(e) => {
                label(ui, "Не удалось обновить sfatm", regular(13.0), DANGER());
                ui.add(egui::Label::new(egui::RichText::new(e).font(regular(12.5)).color(DANGER())).wrap());
            }
        }
        ui.horizontal_wrapped(|ui| {
            if let SfState::Failed(e) = &job.state {
                if button(ui, "Повторить", Some(Icon::Refresh), Kind::Secondary, 30.0).clicked() {
                    let running = self.running.keys().cloned().collect();
                    self.be.sfatm_sync(vec![job.inst.clone()], running, true);
                }
                if button(ui, "Скопировать ошибку", Some(Icon::Copy), Kind::Secondary, 30.0).clicked() {
                    ui.ctx().copy_text(e.clone());
                }
            }
            if matches!(job.state, SfState::Done | SfState::Failed(_)) && button(ui, "Убрать", None, Kind::Secondary, 30.0).clicked() {
                crate::sfupdate::dismiss(&job.inst);
            }
        });
    }

    pub fn job_ui(&mut self, ui: &mut Ui, job: &Job) {
        let failed = matches!(
            job.status,
            InstallJobStatus::Failed | InstallJobStatus::Interrupted
        );
        label(ui, &job.title, semibold(14.0), TEXT());
        label(
            ui,
            job.status_label(),
            regular(13.0),
            if failed { DANGER() } else { TEXT_DIM() },
        );
        if !job.status.is_finished() {
            if !job.progress_label.is_empty() {
                label(ui, &job.progress_label, regular(12.0), TEXT_DIM());
            }
            if let Some(progress) = job.progress {
                let (rect, _) = ui.allocate_exact_size(
                    vec2(ui.available_width(), 4.0),
                    Sense::hover(),
                );
                progress_bar(ui.painter(), rect, progress);
            } else {
                ui.spinner();
            }
        }
        if let Some(error) = &job.error {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(error)
                        .font(regular(12.5))
                        .color(DANGER()),
                )
                .wrap(),
            );
        }
        ui.horizontal_wrapped(|ui| {
            if failed
                && button(
                    ui,
                    "Повторить",
                    Some(Icon::Refresh),
                    Kind::Secondary,
                    30.0,
                )
                .clicked()
            {
                self.be.job_action(job.id.clone(), JobAction::Retry);
            }
            if job.can_cancel
                && !job.canceling
                && button(ui, "Отменить", None, Kind::Secondary, 30.0).clicked()
            {
                self.be.job_action(job.id.clone(), JobAction::Cancel);
            }
            if job.status.is_finished()
                && button(ui, "Убрать", None, Kind::Secondary, 30.0).clicked()
            {
                self.be.job_action(job.id.clone(), JobAction::Dismiss);
            }
            if let Some(error) = &job.error {
                if button(
                    ui,
                    "Скопировать ошибку",
                    Some(Icon::Copy),
                    Kind::Secondary,
                    30.0,
                )
                .clicked()
                {
                    ui.ctx().copy_text(error.clone());
                }
            }
        });
    }
}

impl Backend {
    pub fn job_action(&self, id: String, action: JobAction) {
        self.task(move |tx| async move {
            let result = async {
                let id = id.parse::<uuid::Uuid>().map_err(|e| e.to_string())?;
                match action {
                    JobAction::Retry => {
                        install::retry_job(id).await.map(|_| ())
                    }
                    JobAction::Cancel => {
                        install::cancel_job(id).await.map(|_| ())
                    }
                    JobAction::Dismiss => install::dismiss_job(id).await,
                }
                .map_err(|e| e.to_string())
            }
            .await;
            if let Err(error) = result {
                let _ = tx.send(Msg::Toast(error, true));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn job(status: InstallJobStatus) -> Job {
        Job {
            id: "test".into(),
            title: "VSE".into(),
            progress: Some(0.05),
            status,
            phase: InstallPhaseId::DownloadingContent,
            progress_label: "0 / 115".into(),
            error: None,
            can_cancel: false,
            canceling: false,
            paused: false,
            instance_id: None,
        }
    }
    #[test]
    fn byte_progress_moves_while_a_large_mod_is_downloading() {
        let progress = install::InstallProgress {
            current: 0,
            total: 115,
            secondary: Some(install::InstallProgressSecondary {
                current: 25,
                total: 100,
            }),
        };
        assert_eq!(progress_fraction(&progress), Some(0.25));
        assert_eq!(
            progress_fraction(&install::InstallProgress {
                current: 0,
                total: 0,
                secondary: None
            }),
            None
        );
    }

    #[test]
    fn failed_imports_are_not_presented_as_running_downloads() {
        let failed = job(InstallJobStatus::Failed);
        assert_eq!(failed.status_label(), "Ошибка установки");
        assert!(failed.status.is_finished());
        assert_eq!(
            job(InstallJobStatus::Running).status_label(),
            "Скачивание модов и ресурсов"
        );
    }
}
