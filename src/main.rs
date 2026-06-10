use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use relm4::prelude::*;
use anime_launcher_sdk::config::ConfigExt;
use anime_launcher_sdk::genshin::config::{Config, Schema};
use anime_launcher_sdk::genshin::states::LauncherState;
use anime_launcher_sdk::anime_game_core::prelude::*;
use anime_launcher_sdk::anime_game_core::genshin::prelude::*;
use anime_launcher_sdk::sessions::SessionsExt;
use anime_launcher_sdk::genshin::sessions::Sessions;
use tracing_subscriber::prelude::*;
use tracing_subscriber::filter::*;

pub mod move_files;
pub mod i18n;
pub mod background;
pub mod factory_game;
pub mod ui;

use ui::main::*;
use ui::first_run::main::*;

pub const APP_ID: &str = "moe.takasaki.a-factory-game-launcher";
pub const APP_RESOURCE_PATH: &str = "/moe/takasaki/a-factory-game-launcher";
pub const APP_FOLDER_NAME: &str = "a-factory-game-launcher";
pub const LEGACY_APP_FOLDER_NAMES: &[&str] = &["a-factory-game-launcher", "anime-game-launcher"];

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_DEBUG: bool = cfg!(debug_assertions);

/// Sets to `true` when the `App` component is ready (fully initialized)
pub static READY: AtomicBool = AtomicBool::new(false);

// TODO: get rid of using this function in all the components' events
//       e.g. by converting preferences pages into Relm4 Components
/// Check if the app is ready
pub fn is_ready() -> bool {
    READY.load(Ordering::Relaxed)
}

/// Check if a Wayland compositor is available by looking at WAYLAND_DISPLAY
/// first, then falling back to the wayland-0 socket in XDG_RUNTIME_DIR.
pub fn is_wayland_available() -> bool {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return true;
    }

    let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") else {
        return false;
    };

    std::path::Path::new(&runtime_dir)
        .join("wayland-0")
        .exists()
}

fn xdg_data_dir(folder_name: &str) -> anyhow::Result<PathBuf> {
    let path = std::env::var("XDG_DATA_HOME")
        .map(|data| format!("{data}/{folder_name}"))
        .or_else(|_| std::env::var("HOME").map(|home| format!("{home}/.local/share/{folder_name}")))
        .or_else(|_| {
            std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .map(|username| format!("/home/{username}/.local/share/{folder_name}"))
        })
        .map(PathBuf::from)
        .or_else(|_| std::env::current_dir().map(|current| current.join("data")))
        .map_err(|err| anyhow::anyhow!("Failed to find launcher folder: {err}"))?;

    Ok(path.canonicalize().unwrap_or(path))
}

fn xdg_cache_dir(folder_name: &str) -> anyhow::Result<PathBuf> {
    let path = std::env::var("XDG_CACHE_HOME")
        .map(|cache| format!("{cache}/{folder_name}"))
        .or_else(|_| std::env::var("HOME").map(|home| format!("{home}/.cache/{folder_name}")))
        .or_else(|_| {
            std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .map(|username| format!("/home/{username}/.cache/{folder_name}"))
        })
        .map(PathBuf::from)
        .or_else(|_| std::env::current_dir().map(|current| current.join("cache")))
        .map_err(|err| anyhow::anyhow!("Failed to find launcher's cache folder: {err}"))?;

    Ok(path.canonicalize().unwrap_or(path))
}

/// Get default launcher dir path.
///
/// If `LAUNCHER_FOLDER` variable is set, then its value will be returned.
/// Otherwise returns `$HOME/.local/share/a-factory-game-launcher`.
pub fn launcher_dir() -> anyhow::Result<PathBuf> {
    std::env::var("LAUNCHER_FOLDER")
        .map(PathBuf::from)
        .or_else(|_| xdg_data_dir(APP_FOLDER_NAME))
}

/// Get launcher's cache dir path.
///
/// If `CACHE_FOLDER` variable is set, then its value will be returned.
/// Otherwise returns `$HOME/.cache/a-factory-game-launcher`.
pub fn cache_dir() -> anyhow::Result<PathBuf> {
    std::env::var("CACHE_FOLDER")
        .map(PathBuf::from)
        .or_else(|_| xdg_cache_dir(APP_FOLDER_NAME))
}

fn ensure_sdk_paths() -> anyhow::Result<()> {
    if std::env::var_os("LAUNCHER_FOLDER").is_none() {
        std::env::set_var("LAUNCHER_FOLDER", launcher_dir()?);
    }

    if std::env::var_os("CACHE_FOLDER").is_none() {
        std::env::set_var("CACHE_FOLDER", cache_dir()?);
    }

    Ok(())
}

fn rewrite_legacy_path(path: &mut PathBuf, legacy_base: &Path, current_base: &Path) -> bool {
    let Ok(suffix) = path.strip_prefix(legacy_base) else {
        return false;
    };

    let updated = current_base.join(suffix);

    if *path == updated {
        false
    } else {
        *path = updated;
        true
    }
}

fn normalize_config_paths(config: &mut Schema) -> anyhow::Result<bool> {
    let current_launcher = launcher_dir()?;

    let mut changed = false;

    for legacy_name in LEGACY_APP_FOLDER_NAMES {
        let legacy_launcher = xdg_data_dir(legacy_name)?;

        changed |= rewrite_legacy_path(
            &mut config.game.wine.prefix,
            &legacy_launcher,
            &current_launcher,
        );
        changed |= rewrite_legacy_path(
            &mut config.game.wine.builds,
            &legacy_launcher,
            &current_launcher,
        );
        changed |= rewrite_legacy_path(
            &mut config.game.dxvk.builds,
            &legacy_launcher,
            &current_launcher,
        );
        changed |= rewrite_legacy_path(
            &mut config.game.path.global,
            &legacy_launcher,
            &current_launcher,
        );
        changed |= rewrite_legacy_path(
            &mut config.game.path.china,
            &legacy_launcher,
            &current_launcher,
        );
        changed |= rewrite_legacy_path(
            &mut config.components.path,
            &legacy_launcher,
            &current_launcher,
        );
        changed |= rewrite_legacy_path(
            &mut config.game.enhancements.fps_unlocker.path,
            &legacy_launcher,
            &current_launcher,
        );

        if let Some(temp) = &mut config.launcher.temp {
            changed |= rewrite_legacy_path(temp, &legacy_launcher, &current_launcher);
        }
    }

    let default_game_dir = crate::factory_game::default_game_dir();

    if crate::factory_game::is_default_genshin_path(&config.game.path.global) {
        config.game.path.global = default_game_dir.clone();
        changed = true;
    }

    if crate::factory_game::is_default_genshin_path(&config.game.path.china) {
        config.game.path.china = default_game_dir;
        changed = true;
    }

    if config
        .game
        .wine
        .selected
        .as_deref()
        .is_none_or(|selected| selected.starts_with("spritz-wine-cachyos"))
    {
        config.game.wine.selected = Some(crate::factory_game::PREFERRED_WINE_VERSION.to_string());
        changed = true;
    }

    Ok(changed)
}

fn load_config() -> anyhow::Result<Schema> {
    let mut config = Config::get()?;

    if normalize_config_paths(&mut config)? {
        Config::update_raw(config.clone())?;
    }

    Ok(config)
}

lazy_static::lazy_static! {
    /// Config loaded on the app's start. Use `Config::get()` to get up to date config instead.
    /// This one is used to prepare some launcher UI components on start
    pub static ref CONFIG: Schema = load_config().expect("Failed to load config");

    pub static ref GAME: Game = Game::new(CONFIG.game.path.for_edition(CONFIG.launcher.edition), CONFIG.launcher.edition);

    /// Path to launcher folder. Standard is `$HOME/.local/share/a-factory-game-launcher`
    pub static ref LAUNCHER_FOLDER: PathBuf = launcher_dir().expect("Failed to get launcher folder");

    /// Path to launcher's cache folder. Standard is `$HOME/.cache/a-factory-game-launcher`
    pub static ref CACHE_FOLDER: PathBuf = cache_dir().expect("Failed to get launcher's cache folder");

    /// Path to `debug.log` file. Standard is `$HOME/.local/share/a-factory-game-launcher/debug.log`
    pub static ref DEBUG_FILE: PathBuf = LAUNCHER_FOLDER.join("debug.log");

    /// Path to `background` file. Standard is `$HOME/.local/share/a-factory-game-launcher/background`
    pub static ref BACKGROUND_FILE: PathBuf = LAUNCHER_FOLDER.join("background");

    /// Path to `background-overlat` file. Standard is `$HOME/.local/share/a-factory-game-launcher/background-overlay`
    pub static ref BACKGROUND_OVERLAY_FILE: PathBuf = LAUNCHER_FOLDER.join("background-overlay");

    /// Path to the processed `background` file. Standard is `$HOME/.cache/a-factory-game-launcher/background`
    pub static ref PROCESSED_BACKGROUND_FILE: PathBuf = CACHE_FOLDER.join("background");

    /// Path to the processed `background-overlay` file. Standard is `$HOME/.cache/a-factory-game-launcher/background-overlay`
    pub static ref PROCESSED_BACKGROUND_OVERLAY_FILE: PathBuf = CACHE_FOLDER.join("background-overlay");

    /// Path to the processed `background-video` file. Standard is `$HOME/.cache/a-factory-game-launcher/background-video`
    pub static ref BACKGROUND_VIDEO_FILE: PathBuf = CACHE_FOLDER.join("background-video");

    /// Path to `.keep-background` file. Used to mark launcher that it shouldn't update background picture
    ///
    /// Standard is `$HOME/.local/share/a-factory-game-launcher/.keep-background`
    pub static ref KEEP_BACKGROUND_FILE: PathBuf = LAUNCHER_FOLDER.join(".keep-background");

    /// Path to `.first-run` file. Used to mark launcher that it should run FirstRun window
    ///
    /// Standard is `$HOME/.local/share/a-factory-game-launcher/.first-run`
    pub static ref FIRST_RUN_FILE: PathBuf = LAUNCHER_FOLDER.join(".first-run");

    /// Global app's css
    static ref GLOBAL_CSS: String = format!("
        progressbar > text {{
            margin-bottom: 4px;
        }}

        window.classic-style {{
            background: url(\"file://{}\"), url(\"file://{}\");
            background-repeat: no-repeat, no-repeat;
            background-size: cover, cover;
        }}

        .background-overlay {{
            background: url(\"file://{}\");
            background-repeat: no-repeat;
            background-size: cover;
        }}

        window.classic-style progressbar {{
            background-color: #00000020;
            border-radius: 16px;
            padding: 8px 16px;
        }}

        window.classic-style progressbar:hover {{
            background-color: #00000060;
            color: #ffffff;
            transition-duration: 0.5s;
            transition-timing-function: linear;
        }}

        .round-bin {{
            border-radius: 24px;
        }}
        ",
        PROCESSED_BACKGROUND_OVERLAY_FILE.to_string_lossy(),
        PROCESSED_BACKGROUND_FILE.to_string_lossy(),
        PROCESSED_BACKGROUND_OVERLAY_FILE.to_string_lossy(),
        );
}

fn main() -> anyhow::Result<()> {
    ensure_sdk_paths()?;

    // Setup custom panic handler
    human_panic::setup_panic!(human_panic::metadata!());

    // Create launcher folder if it doesn't exist.
    if !LAUNCHER_FOLDER.exists() {
        // check if the location is a symlink. [Path::exists] resolves the symlink and
        // returns whether its *target* exists or not.
        if LAUNCHER_FOLDER.is_symlink() {
            eprintln!(
                "{} is a broken symlink, meaning the directory it is pointing to does not exist, cannot proceed.",
                LAUNCHER_FOLDER.display()
            );
            anyhow::bail!("Launcher folder is a broken symlink");
        }

        std::fs::create_dir_all(LAUNCHER_FOLDER.as_path())
            .expect("Failed to create launcher folder");

        // This one is kinda critical but well, I can't do anything about it
        std::fs::write(FIRST_RUN_FILE.as_path(), "").expect("Failed to create .first-run file");

        // Set initial launcher language based on system language
        // CONFIG is initialized lazily so it will contain following changes as well
        let mut config = Config::get().expect("Failed to get config");

        config.launcher.language = i18n::format_lang(i18n::get_default_lang());

        Config::update_raw(config).expect("Failed to update config");
    }

    // Create cache folder if it doesn't exist.
    if !CACHE_FOLDER.exists() {
        if CACHE_FOLDER.is_symlink() {
            eprintln!(
                "{} is a broken symlink, meaning the directory it is pointing to does not exist, cannot proceed.",
                CACHE_FOLDER.display()
            );
            anyhow::bail!("Cache folder is a broken symlink");
        }

        std::fs::create_dir_all(CACHE_FOLDER.as_path()).expect("Failed to create cache folder");
    }

    // Force debug output
    let mut force_debug = 0;

    // Run the game
    let mut run_game = false;

    // Force run the game
    let mut just_run_game = false;

    // Force disable verbose tracing output in stdout
    let mut no_verbose_tracing = false;

    let args = std::env::args().collect::<Vec<_>>();
    let mut gtk_args = Vec::new();

    // Parse arguments
    for i in 0..args.len() {
        match args[i].as_str() {
            "--debug" => force_debug += 1,
            "--run-game" => run_game = true,
            "--just-run-game" => just_run_game = true,
            "--no-verbose-tracing" => no_verbose_tracing = true,

            "--session" => {
                // Switch active session prior running the app
                if let Some(session) = args.get(i + 1) {
                    Sessions::set_current(session.to_owned())?;
                }
            }

            arg => gtk_args.push(arg.to_string()),
        }
    }

    // Prepare stdout logger
    let stdout = tracing_subscriber::fmt::layer()
        .pretty()
        .with_filter({
            if force_debug >= 2 {
                LevelFilter::TRACE
            } else if APP_DEBUG || force_debug >= 1 {
                LevelFilter::DEBUG
            } else {
                LevelFilter::WARN
            }
        })
        .with_filter(filter_fn(move |metadata| {
            !metadata.target().contains("rustls")
                && !metadata.target().contains("reqwest")
                && !metadata.target().contains("h2")
                && !metadata.target().contains("hyper_util")
                && !no_verbose_tracing
        }));

    // Prepare debug file logger
    let file = std::fs::File::create(DEBUG_FILE.as_path())?;

    let debug_log = tracing_subscriber::fmt::layer()
        .pretty()
        .with_ansi(false)
        .with_writer(std::sync::Arc::new(file))
        .with_filter({
            if force_debug >= 2 {
                LevelFilter::TRACE
            } else {
                LevelFilter::DEBUG
            }
        })
        .with_filter(filter_fn(|metadata| {
            !metadata.target().contains("rustls")
                && !metadata.target().contains("reqwest")
                && !metadata.target().contains("h2")
                && !metadata.target().contains("hyper_util")
        }));

    tracing_subscriber::registry()
        .with(stdout)
        .with(debug_log)
        .init();

    tracing::info!("Starting application ({APP_VERSION})");

    adw::init().expect("Libadwaita initialization failed");

    // Register and include resources
    gtk::gio::resources_register_include!("resources.gresource")
        .expect("Failed to register resources");

    // Set icons search path
    gtk::IconTheme::for_display(&gtk::gdk::Display::default().unwrap())
        .add_resource_path(&format!("{APP_RESOURCE_PATH}/icons"));

    // Set global css
    relm4::set_global_css(&GLOBAL_CSS);

    // Set application's title
    gtk::glib::set_application_name("A Factory Game Launcher");
    gtk::glib::set_program_name(Some("A Factory Game Launcher"));

    // Set UI language
    let lang = CONFIG
        .launcher
        .language
        .parse()
        .expect("Wrong language format used in config");

    i18n::set_lang(lang).expect("Failed to set launcher language");

    tracing::info!("Set UI language to {}", i18n::get_lang());

    // Run FirstRun window if .first-run file persist
    if FIRST_RUN_FILE.exists() {
        // Create the app
        let app = RelmApp::new(APP_ID).with_args(gtk_args);

        // Show first run window
        app.run::<FirstRunApp>(());
    }
    // Run the app if everything's ready
    else {
        if run_game || just_run_game {
            let state =
                LauncherState::get_from_config(|_| {}).expect("Failed to get launcher state");

            match state {
                LauncherState::Launch => {
                    anime_launcher_sdk::genshin::game::run().expect("Failed to run the game");

                    return Ok(());
                }

                LauncherState::PredownloadAvailable { .. } if just_run_game => {
                    anime_launcher_sdk::genshin::game::run().expect("Failed to run the game");

                    return Ok(());
                }

                _ => (),
            }
        }

        // Create the app
        let app = RelmApp::new(APP_ID).with_args(gtk_args);

        // Show main window
        app.run::<App>(());
    }

    Ok(())
}
