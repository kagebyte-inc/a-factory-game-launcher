use relm4::prelude::*;
use relm4::Sender;

use crate::ui::components::*;
use crate::*;

use super::{App, AppMsg};

pub fn download_factory_game(
    sender: ComponentSender<App>,
    progress_bar_input: Sender<ProgressBarMsg>,
) {
    sender.input(AppMsg::ClearDebugLog);
    sender.input(AppMsg::AppendDebugLog(
        "starting factory game install".to_string(),
    ));
    sender.input(AppMsg::SetDownloading(true));

    std::thread::spawn(move || {
        let temp_dir = CONFIG
            .launcher
            .temp
            .clone()
            .unwrap_or_else(|| CACHE_FOLDER.join("tmp"));
        let game_dir = factory_game::configured_game_dir();

        let result = factory_game::install_to(&game_dir, &temp_dir, |state| {
            match state {
                factory_game::InstallUpdate::Downloading {
                    downloaded_bytes,
                    total_bytes,
                } => {
                    let _ = progress_bar_input.send(ProgressBarMsg::DisplayProgress(true));
                    let _ = progress_bar_input.send(ProgressBarMsg::DisplayFraction(true));
                    let _ = progress_bar_input
                        .send(ProgressBarMsg::UpdateCaption(Some(tr!("downloading"))));
                    let _ = progress_bar_input.send(ProgressBarMsg::UpdateProgress(
                        downloaded_bytes,
                        total_bytes,
                    ));
                }

                factory_game::InstallUpdate::Verifying { current, total } => {
                    let _ = progress_bar_input.send(ProgressBarMsg::DisplayProgress(true));
                    let _ = progress_bar_input.send(ProgressBarMsg::DisplayFraction(true));
                    let _ = progress_bar_input
                        .send(ProgressBarMsg::UpdateCaption(Some(tr!("verifying-files"))));
                    let _ = progress_bar_input.send(ProgressBarMsg::UpdateProgressCounter(
                        current as u64,
                        total as u64,
                    ));
                }

                factory_game::InstallUpdate::Unpacking => {
                    let _ = progress_bar_input.send(ProgressBarMsg::DisplayProgress(true));
                    let _ = progress_bar_input.send(ProgressBarMsg::DisplayFraction(false));
                    let _ = progress_bar_input
                        .send(ProgressBarMsg::UpdateCaption(Some(tr!("unpacking"))));
                    let _ = progress_bar_input.send(ProgressBarMsg::UpdateProgress(0, 100));
                }

                factory_game::InstallUpdate::UnpackingProgress { percent } => {
                    let _ = progress_bar_input.send(ProgressBarMsg::DisplayProgress(true));
                    let _ = progress_bar_input.send(ProgressBarMsg::DisplayFraction(false));
                    let _ = progress_bar_input
                        .send(ProgressBarMsg::UpdateCaption(Some(tr!("unpacking"))));
                    let _ = progress_bar_input
                        .send(ProgressBarMsg::UpdateProgress(percent as u64, 100));
                }

                factory_game::InstallUpdate::Debug { message } => {
                    sender.input(AppMsg::AppendDebugLog(message));
                }
            };
        });

        if let Err(err) = result {
            sender.input(AppMsg::AppendDebugLog(format!("error: {err:?}")));
            tracing::error!("Failed to install factory game: {err:?}");

            sender.input(AppMsg::Toast {
                title: tr!("downloading-failed"),
                description: Some(err.to_string()),
            });
        }

        sender.input(AppMsg::SetDownloading(false));
        sender.input(AppMsg::UpdateLauncherState {
            perform_on_download_needed: false,
            show_status_page: true,
        });
    });
}
