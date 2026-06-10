use relm4::{prelude::*, Sender};

use gtk::glib::clone;

use anime_launcher_sdk::components::wine;
use anime_launcher_sdk::wincompatlib::prelude::*;

use crate::*;
use crate::ui::components::*;

use super::{App, AppMsg};

pub fn download_wine(sender: ComponentSender<App>, progress_bar_input: Sender<ProgressBarMsg>) {
    let mut config = Config::get().unwrap();

    match wine::get_downloaded(&CONFIG.components.path, &config.game.wine.builds) {
        Ok(_) => {
            let preferred = crate::factory_game::preferred_wine_version(&config.components.path)
                .ok()
                .flatten();

            let downloaded_preferred = preferred
                .as_ref()
                .filter(|wine| wine.is_downloaded_in(&config.game.wine.builds))
                .cloned();

            // Select downloaded preferred version
            if let Some(wine) = downloaded_preferred {
                config.game.wine.selected = Some(wine.name);

                Config::update(config);

                sender.input(AppMsg::UpdateLauncherState {
                    perform_on_download_needed: false,
                    show_status_page: true,
                });
            }
            // Or download preferred/new one if none is available
            else {
                // Choose selected wine version or use latest available one
                let wine = match preferred {
                    Some(version) => version,
                    None => {
                        let latest = wine::Version::latest(&CONFIG.components.path)
                            .expect("Failed to get latest wine version");

                        match &config.game.wine.selected {
                            Some(version) => {
                                match wine::Version::find_in(&config.components.path, version) {
                                    Ok(Some(version)) => version,
                                    _ => latest,
                                }
                            }

                            None => latest,
                        }
                    }
                };

                // Download wine version
                match Installer::new(wine.uri.clone()) {
                    Ok(mut installer) => {
                        if let Some(temp_folder) = &config.launcher.temp {
                            installer.temp_folder = temp_folder.to_path_buf();
                        }

                        sender.input(AppMsg::SetDownloading(true));

                        std::thread::spawn(clone!(
                            #[strong]
                            sender,
                            move || {
                                installer.install(
                                    &config.game.wine.builds,
                                    clone!(
                                        #[strong]
                                        sender,
                                        move |state| {
                                            match &state {
                                                InstallerUpdate::DownloadingError(err) => {
                                                    tracing::error!("Downloading failed: {err}");

                                                    sender.input(AppMsg::Toast {
                                                        title: tr!("downloading-failed"),
                                                        description: Some(err.to_string()),
                                                    });
                                                }

                                                InstallerUpdate::UnpackingError(err) => {
                                                    tracing::error!("Unpacking failed: {err}");

                                                    sender.input(AppMsg::Toast {
                                                        title: tr!("unpacking-failed"),
                                                        description: Some(err.clone()),
                                                    });
                                                }

                                                _ => (),
                                            }

                                            #[allow(unused_must_use)]
                                            {
                                                progress_bar_input.send(
                                                    ProgressBarMsg::UpdateFromState(
                                                        DiffUpdate::InstallerUpdate(state),
                                                    ),
                                                );
                                            }
                                        }
                                    ),
                                );

                                config.game.wine.selected = Some(wine.name.clone());

                                let prefix_update_result = wine
                                    .to_wine(
                                        config.components.path.clone(),
                                        Some(config.game.wine.builds.join(&wine.name)),
                                    )
                                    .with_prefix(&config.game.wine.prefix)
                                    .with_loader(WineLoader::Current)
                                    .with_arch(WineArch::Win64)
                                    .update_prefix(None::<&str>);

                                if let Err(err) = prefix_update_result {
                                    tracing::error!("Failed to update wine prefix: {err}");

                                    sender.input(AppMsg::Toast {
                                        title: tr!("wine-prefix-update-failed"),
                                        description: Some(err.to_string()),
                                    });
                                }

                                Config::update(config);

                                sender.input(AppMsg::SetDownloading(false));
                                sender.input(AppMsg::UpdateLauncherState {
                                    perform_on_download_needed: false,
                                    show_status_page: true,
                                });
                            }
                        ));
                    }

                    Err(err) => sender.input(AppMsg::Toast {
                        title: tr!("wine-install-failed"),
                        description: Some(err.to_string()),
                    }),
                }
            }
        }

        Err(err) => sender.input(AppMsg::Toast {
            title: tr!("downloaded-wine-list-failed"),
            description: Some(err.to_string()),
        }),
    }
}
