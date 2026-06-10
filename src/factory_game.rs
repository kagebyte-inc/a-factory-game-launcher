use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::Context;
use anime_launcher_sdk::config::ConfigExt;
use anime_launcher_sdk::anime_game_core::reqwest::blocking::Client;
use anime_launcher_sdk::anime_game_core::reqwest::header::{CONTENT_LENGTH, RANGE};
use md5::Digest;
use serde::{Deserialize, Serialize};

use crate::CONFIG;

pub const LAUNCHER_BOOTSTRAP_API: &str =
    "https://launcher.gryphline.com/api/launcher/get_latest_launcher";
pub const LAUNCHER_APP_CODE: &str = "TiaytKBUIEdoEwRT";
pub const GAME_APP_CODE: &str = "YDUTE5gscDZ229CW";
pub const CHANNEL: &str = "6";
pub const SUB_CHANNEL: &str = "9999";
pub const PACKAGE_SUB_CHANNEL: &str = "6";
pub const REGION: &str = "sg";
pub const REMOTE_TARGET_APP: &str = concat!("End", "Field");
pub const TRACKING: &str = "GRYPHLINKDownloader";
pub const LAUNCHER_VERSION: &str = "1.4.0";
pub const LAUNCHER_VERSION_DETAIL: &str = "1.4.0.1407";
pub const GAME_VERSION: &str = "1.3.3";
pub const GAME_DIR_NAME: &str = "Factory Game";
pub const GAME_EXECUTABLE: &str = concat!("End", "field.exe");
pub const PREFERRED_WINE_VERSION: &str = "wine-dwproton-wow64-11.0-3";
pub const VERSION_MARKER: &str = ".factory-game-version";
pub const CHECKSUM_CACHE: &str = ".factory-game-checksums.json";
pub const SEVEN_ZIP_VERSION: &str = "26.01";
pub const SEVEN_ZIP_EXTRA_URL: &str = "https://www.7-zip.org/a/7z2601-extra.7z";
pub const GAME_UUID: &str = "cd2d0f009b9c3997c1e4d319ed1a0eae";
pub const GAME_FILES_MD5: &str = "088b578e66a1cb3ffe3219cc9ca7a25d";
pub const PACKAGE_TOTAL_SIZE: u64 = 114_517_544_273;
pub const PACKAGE_FILE_LIST_URL: &str = "https://beyond.hg-cdn.com/YDUTE5gscDZ229CW/1.3/update/6/6/Windows/1.3.3_BGlQHu2HgMlqTuqG/files";
pub const PACKAGE_PACK_BASE_URL: &str = "https://beyond.hg-cdn.com/YDUTE5gscDZ229CW/1.3/update/6/6/Windows/1.3.3_BGlQHu2HgMlqTuqG/packs/Beyond_Release_v1d3-Rel-os-7123154-1_prod_obt_official.zip.";
pub const PACKAGE_PACK_COUNT: usize = 49;
pub const PACKAGE_PACK_SIZE: u64 = 1_073_741_824;
pub const PACKAGE_LAST_PACK_SIZE: u64 = 172_264_000;

pub const PACKAGE_PACK_MD5: [&str; PACKAGE_PACK_COUNT] = [
    "2f9c207dabf7ebaef5000f77916a6b98",
    "3ddc5e4bd8c9d1cb0c1c198245c8eb2f",
    "4b0769003bea4cb76b5455a72a5d6fb3",
    "57f3f635354e312281028f9b5486d12c",
    "2d3ca0c7e8de05e8cad4217175357fbb",
    "3b36ff26d0e0744945ab5e274bf1f2c3",
    "e0ea6ef515cf824d125cbbaee75e74e8",
    "33a479edcb0873cf0b930fc6d0ad8124",
    "a18b37f7d60b78caf08b1a4d720e04d2",
    "e3a7ea1768566a38f30547d793610fc4",
    "2071e617140317e03ea57dc1456e5041",
    "67f06cc4be227bff1f4230250e3ded2b",
    "f0955e00a18916109be58e09d4566912",
    "60746fc96727d42858115288d2e9def2",
    "1ce86d321f91edd8cf116364d91c74e2",
    "92ef7c5b737fcd92dd76a91834ed7cd0",
    "6b7556f07cc36276f4d084b53262732a",
    "e597a9fbe6cc7f1ee04ab4ce273b431c",
    "009b4b2e3e3b3c1f7720f84ac1d1c6b9",
    "229e9ed243ea6580cbe442a68b6c47ba",
    "b4d1449175f4f0485eabe5237d16a213",
    "8106fbce5133f68429abf1c0a830f8d7",
    "53a4ee9c9e3ae66f966232885f407ebd",
    "11d830a2244db3a73000b40da7d527fe",
    "02a1a2f38202288bb363f911d615c09f",
    "07993bdd2b7df00d539094a6b7a44a5d",
    "89607a4a4d72a0a16f57ff9accd7ee2f",
    "37fc8067a5b78cbb5e042738d20494d7",
    "9a934c99a5c5fad0fbe41ab313785353",
    "a30a7459a6a8a1e44f3de1df6fe8ae2d",
    "dcc6fdbaf30d29da9512e8326bf625de",
    "ac0f4b1255339f6392bb81bc7710a813",
    "0f03b90b07f092da05c9b3f46a10872e",
    "0d03fe19e83ac42c5047c10aa9f35cc4",
    "cf66a649c31a16339f683135c5ce5278",
    "03dfd034e4680cf5f325f0fdd2e1eddb",
    "e85a1681973622cd7ec6b0907690e1ae",
    "906c0ba24c1698448a2c5c2a63e0d3c2",
    "98b01ad8a1aa6ccfc73e2eec53634138",
    "8568271e384274216e8ff48723255907",
    "afe5d090ce46f2f1b8123ab26926a23f",
    "aefd536e1f616bd03eba1c431addd11c",
    "3f13a03d945df76118d6fe55a9241d02",
    "59567a136d54f42a77252a47b4999273",
    "0de54e3c1aa99cc72e441b83aeb40609",
    "c01a7fd401fe9fc373f85379d30f9e91",
    "de9d0895ef6873ce9e36fd1cdfc4a280",
    "87b786d19b1b134a9db14f09c52cd6ed",
    "89da08a94b98f85c9bf193f3fd1c495d",
];

#[derive(Debug, Clone, Deserialize)]
pub struct LatestLauncher {
    pub action: u8,
    pub version: String,
    pub request_version: String,
    pub exe_url: String,
    pub exe_size: String,
}

#[derive(Debug, Clone, Copy)]
pub struct KnownPackage {
    pub version: &'static str,
    pub file_list_url: &'static str,
    pub pack_base_url: &'static str,
    pub pack_count: usize,
    pub pack_size: u64,
    pub last_pack_size: u64,
    pub total_size: u64,
    pub game_files_md5: &'static str,
    pub pack_md5: &'static [&'static str],
}

#[derive(Debug, Clone)]
pub enum InstallUpdate {
    Downloading {
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    Verifying {
        current: usize,
        total: usize,
    },
    Unpacking,
    UnpackingProgress {
        percent: u8,
    },
    Debug {
        message: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ChecksumCache {
    packs: HashMap<String, CachedChecksum>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedChecksum {
    size: u64,
    modified: u64,
    md5: String,
}

impl LatestLauncher {
    pub fn exe_size_bytes(&self) -> anyhow::Result<u64> {
        self.exe_size
            .parse()
            .context("Failed to parse GRYPHLINK launcher size")
    }
}

impl KnownPackage {
    pub fn pack_url(&self, index: usize) -> Option<String> {
        (1..=self.pack_count)
            .contains(&index)
            .then(|| format!("{}{:03}", self.pack_base_url, index))
    }

    pub fn pack_size(&self, index: usize) -> Option<u64> {
        match index {
            1..=48 => Some(self.pack_size),
            49 => Some(self.last_pack_size),
            _ => None,
        }
    }

    pub fn download_size(&self) -> u64 {
        (self.pack_count as u64 - 1) * self.pack_size + self.last_pack_size
    }
}

pub fn latest_launcher_url() -> String {
    format!(
        "{LAUNCHER_BOOTSTRAP_API}?appcode={LAUNCHER_APP_CODE}&channel={CHANNEL}&sub_channel={SUB_CHANNEL}&ua=&ta={REMOTE_TARGET_APP}&tracking={TRACKING}"
    )
}

pub fn get_latest_launcher(client: &Client) -> anyhow::Result<LatestLauncher> {
    client
        .get(latest_launcher_url())
        .send()
        .context("Failed to request latest GRYPHLINK launcher")?
        .error_for_status()
        .context("GRYPHLINK launcher bootstrap returned an error")?
        .json()
        .context("Failed to decode latest GRYPHLINK launcher response")
}

pub fn known_full_package() -> KnownPackage {
    KnownPackage {
        version: GAME_VERSION,
        file_list_url: PACKAGE_FILE_LIST_URL,
        pack_base_url: PACKAGE_PACK_BASE_URL,
        pack_count: PACKAGE_PACK_COUNT,
        pack_size: PACKAGE_PACK_SIZE,
        last_pack_size: PACKAGE_LAST_PACK_SIZE,
        total_size: PACKAGE_TOTAL_SIZE,
        game_files_md5: GAME_FILES_MD5,
        pack_md5: &PACKAGE_PACK_MD5,
    }
}

pub fn default_game_dir() -> PathBuf {
    crate::LAUNCHER_FOLDER.join(GAME_DIR_NAME)
}

pub fn configured_game_dir() -> PathBuf {
    CONFIG
        .game
        .path
        .for_edition(CONFIG.launcher.edition)
        .to_path_buf()
}

pub fn is_default_genshin_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "Genshin Impact" || name == "YuanShen")
}

pub fn installed_version_in(game_dir: &Path) -> Option<String> {
    std::fs::read_to_string(game_dir.join(VERSION_MARKER))
        .ok()
        .map(|version| version.trim().to_string())
        .filter(|version| !version.is_empty())
}

pub fn is_installed_in(game_dir: &Path) -> bool {
    game_dir.join(GAME_EXECUTABLE).exists() && installed_version_in(game_dir).is_some()
}

pub fn is_installed() -> bool {
    is_installed_in(&configured_game_dir())
}

pub fn preferred_wine_version(
    components_path: &Path,
) -> anyhow::Result<Option<anime_launcher_sdk::components::wine::Version>> {
    anime_launcher_sdk::components::wine::Version::find_in(components_path, PREFERRED_WINE_VERSION)
}

pub fn install_to<F>(game_dir: &Path, temp_dir: &Path, mut update: F) -> anyhow::Result<()>
where
    F: FnMut(InstallUpdate),
{
    let package = known_full_package();
    let download_dir = temp_dir.join("factory-game-download").join(package.version);

    update(InstallUpdate::Debug {
        message: format!("download dir: {}", download_dir.display()),
    });
    update(InstallUpdate::Debug {
        message: format!("game dir: {}", game_dir.display()),
    });

    std::fs::create_dir_all(&download_dir).context("Failed to create factory game download dir")?;
    std::fs::create_dir_all(game_dir).context("Failed to create factory game dir")?;

    let client = Client::new();
    let total_bytes = package.download_size();
    let mut downloaded_bytes = 0;
    let mut checksum_cache = load_checksum_cache(&download_dir);
    let mut checksum_cache_dirty = false;

    update(InstallUpdate::Verifying {
        current: 0,
        total: package.pack_count,
    });

    for index in 1..=package.pack_count {
        let pack_path = pack_path(&download_dir, index);
        let expected_size = package
            .pack_size(index)
            .expect("pack index is within known package");

        update(InstallUpdate::Debug {
            message: format!("checking pack {index:03}"),
        });
        update(InstallUpdate::Verifying {
            current: index - 1,
            total: package.pack_count,
        });

        let validation = validate_pack(
            &pack_path,
            index,
            expected_size,
            package.pack_md5[index - 1],
            &mut checksum_cache,
        )?;
        checksum_cache_dirty |= validation.cache_updated;

        update(InstallUpdate::Debug {
            message: validation.message,
        });

        if validation.valid {
            update(InstallUpdate::Debug {
                message: format!("pack {index:03} is valid"),
            });

            if checksum_cache_dirty {
                save_checksum_cache(&download_dir, &checksum_cache)?;
                checksum_cache_dirty = false;
            }

            downloaded_bytes += expected_size;

            update(InstallUpdate::Verifying {
                current: index,
                total: package.pack_count,
            });

            continue;
        }

        let existing_size = pack_path.metadata().map(|meta| meta.len()).unwrap_or(0);
        if existing_size > expected_size {
            update(InstallUpdate::Debug {
                message: format!(
                    "pack {index:03} is oversized ({existing_size} > {expected_size}), restarting"
                ),
            });
            std::fs::remove_file(&pack_path).with_context(|| {
                format!(
                    "Failed to remove oversized factory game pack {}",
                    pack_path.display()
                )
            })?;
        } else if existing_size > 0 {
            update(InstallUpdate::Debug {
                message: format!("resuming pack {index:03} from {existing_size} bytes"),
            });
        } else {
            update(InstallUpdate::Debug {
                message: format!("downloading pack {index:03}"),
            });
        }

        download_pack(
            &client,
            package.pack_url(index).expect("pack index is valid"),
            &pack_path,
            expected_size,
            downloaded_bytes,
            total_bytes,
            &mut update,
        )
        .with_context(|| format!("Failed to download factory game pack {index:03}"))?;

        update(InstallUpdate::Verifying {
            current: index - 1,
            total: package.pack_count,
        });

        let validation = validate_pack(
            &pack_path,
            index,
            expected_size,
            package.pack_md5[index - 1],
            &mut checksum_cache,
        )?;
        checksum_cache_dirty |= validation.cache_updated;

        update(InstallUpdate::Debug {
            message: validation.message,
        });

        if !validation.valid {
            let _ = std::fs::remove_file(&pack_path);
            anyhow::bail!("Factory game pack {index:03} failed MD5 verification");
        }

        if checksum_cache_dirty {
            save_checksum_cache(&download_dir, &checksum_cache)?;
            checksum_cache_dirty = false;
        }

        update(InstallUpdate::Debug {
            message: format!("pack {index:03} downloaded and verified"),
        });
        downloaded_bytes += expected_size;

        update(InstallUpdate::Verifying {
            current: index,
            total: package.pack_count,
        });
    }

    update(InstallUpdate::Unpacking);
    if checksum_cache_dirty {
        save_checksum_cache(&download_dir, &checksum_cache)?;
    }
    unpack_package(&download_dir, game_dir, &mut update)?;

    std::fs::write(
        game_dir.join(VERSION_MARKER),
        format!("{}\n", package.version),
    )
    .context("Failed to write factory game version marker")?;

    Ok(())
}

pub fn launch() -> anyhow::Result<bool> {
    let config = anime_launcher_sdk::genshin::config::Config::get()?;
    let game_dir = config
        .game
        .path
        .for_edition(config.launcher.edition)
        .to_path_buf();

    if !is_installed_in(&game_dir) {
        anyhow::bail!("factory game is not installed");
    }

    let Some(wine) = config.get_selected_wine()? else {
        anyhow::bail!("Couldn't find wine executable");
    };

    let features = wine.features(&config.components.path)?.unwrap_or_default();
    let wine_dir = config.game.wine.builds.join(&wine.name);
    let wine_binary = wine_dir.join(wine.files.wine64.as_ref().unwrap_or(&wine.files.wine));

    let run_command = features
        .command
        .as_ref()
        .map(|command| replace_keywords(command, &wine_dir, &config.game.wine.prefix, &game_dir))
        .unwrap_or_else(|| format!("'{}'", wine_binary.to_string_lossy()));

    let mut command = Command::new("bash");
    command.arg("-c");
    command.arg(format!("{run_command} {GAME_EXECUTABLE}"));
    command.current_dir(&game_dir);
    command.env("WINEARCH", "win64");
    command.env("WINEPREFIX", &config.game.wine.prefix);

    for (key, value) in features.env {
        command.env(
            key,
            replace_keywords(value, &wine_dir, &config.game.wine.prefix, &game_dir),
        );
    }

    if let Ok(Some(dxvk)) = config.get_selected_dxvk() {
        if let Ok(Some(features)) = dxvk.features(&config.components.path) {
            for (key, value) in features.env {
                command.env(
                    key,
                    replace_keywords(value, &wine_dir, &config.game.wine.prefix, &game_dir),
                );
            }
        }
    }

    command.envs(&config.game.environment);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    tracing::info!("Running factory game from {}", game_dir.display());

    let output = command.output().context("Failed to launch factory game")?;

    let game_log = crate::LAUNCHER_FOLDER.join("game.log");
    let mut log = File::create(game_log).context("Failed to create game.log")?;
    log.write_all(&output.stdout)?;
    log.write_all(&output.stderr)?;

    if !output.status.success() {
        anyhow::bail!("factory game exited with status {}", output.status);
    }

    Ok(false)
}

fn replace_keywords(
    value: impl ToString,
    wine_dir: &Path,
    prefix: &Path,
    game_dir: &Path,
) -> String {
    value
        .to_string()
        .replace("%build%", &wine_dir.to_string_lossy())
        .replace("%prefix%", &prefix.to_string_lossy())
        .replace("%temp%", &std::env::temp_dir().to_string_lossy())
        .replace("%launcher%", &crate::LAUNCHER_FOLDER.to_string_lossy())
        .replace("%game%", &game_dir.to_string_lossy())
}

fn pack_path(download_dir: &Path, index: usize) -> PathBuf {
    download_dir.join(format!(
        "Beyond_Release_v1d3-Rel-os-7123154-1_prod_obt_official.zip.{index:03}"
    ))
}

#[derive(Debug, Clone)]
struct PackValidation {
    valid: bool,
    cache_updated: bool,
    message: String,
}

fn load_checksum_cache(download_dir: &Path) -> ChecksumCache {
    std::fs::read(download_dir.join(CHECKSUM_CACHE))
        .ok()
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default()
}

fn save_checksum_cache(download_dir: &Path, cache: &ChecksumCache) -> anyhow::Result<()> {
    let path = download_dir.join(CHECKSUM_CACHE);
    let data = serde_json::to_vec_pretty(cache)?;

    std::fs::write(&path, data)
        .with_context(|| format!("Failed to write checksum cache {}", path.display()))
}

fn validate_pack(
    path: &Path,
    index: usize,
    expected_size: u64,
    expected_md5: &str,
    cache: &mut ChecksumCache,
) -> anyhow::Result<PackValidation> {
    let Ok(metadata) = path.metadata() else {
        return Ok(PackValidation {
            valid: false,
            cache_updated: false,
            message: format!("pack {index:03}: not downloaded yet"),
        });
    };

    if metadata.len() != expected_size {
        cache.packs.remove(&index.to_string());

        return Ok(PackValidation {
            valid: false,
            cache_updated: true,
            message: format!(
                "pack {index:03}: size mismatch, have {}, want {expected_size}",
                metadata.len()
            ),
        });
    }

    let modified = modified_secs(&metadata).unwrap_or(0);
    let cache_key = index.to_string();

    if let Some(cached) = cache.packs.get(&cache_key) {
        if cached.size == metadata.len() && cached.modified == modified {
            return Ok(PackValidation {
                valid: cached.md5.eq_ignore_ascii_case(expected_md5),
                cache_updated: false,
                message: format!(
                    "pack {index:03}: checksum cache hit ({})",
                    if cached.md5.eq_ignore_ascii_case(expected_md5) {
                        "valid"
                    } else {
                        "invalid"
                    }
                ),
            });
        }
    }

    let md5 = md5_file(path)?;
    let valid = md5.eq_ignore_ascii_case(expected_md5);

    if valid {
        cache.packs.insert(
            cache_key,
            CachedChecksum {
                size: metadata.len(),
                modified,
                md5,
            },
        );
    } else {
        cache.packs.remove(&cache_key);
    }

    Ok(PackValidation {
        valid,
        cache_updated: true,
        message: format!(
            "pack {index:03}: checksum cache miss, calculated md5 ({})",
            if valid { "valid" } else { "invalid" }
        ),
    })
}

fn modified_secs(metadata: &std::fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

fn md5_file(path: &Path) -> anyhow::Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let mut hasher = md5::Md5::new();
    let mut buffer = [0; 1024 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        if read == 0 {
            break;
        }

        md5::Digest::update(&mut hasher, &buffer[..read]);
    }

    Ok(format!("{:x}", md5::Digest::finalize(hasher)))
}

fn download_pack<F>(
    client: &Client,
    url: String,
    path: &Path,
    expected_size: u64,
    already_downloaded: u64,
    total_bytes: u64,
    update: &mut F,
) -> anyhow::Result<()>
where
    F: FnMut(InstallUpdate),
{
    let mut start = path.metadata().map(|meta| meta.len()).unwrap_or(0);
    let mut request = client.get(url);

    if start > 0 && start < expected_size {
        request = request.header(RANGE, format!("bytes={start}-"));
    } else if start >= expected_size {
        std::fs::remove_file(path)
            .with_context(|| format!("Failed to remove {}", path.display()))?;
        start = 0;
    }

    let mut response = request
        .send()
        .context("Failed to send factory game pack request")?
        .error_for_status()
        .context("Factory game CDN returned an error")?;

    let content_length = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());

    if start > 0 && content_length == Some(expected_size) {
        start = 0;
    }

    let mut file = if start > 0 {
        OpenOptions::new()
            .append(true)
            .open(path)
            .with_context(|| format!("Failed to append {}", path.display()))?
    } else {
        File::create(path).with_context(|| format!("Failed to create {}", path.display()))?
    };

    let mut current = start;
    let mut buffer = [0; 1024 * 1024];

    loop {
        let read = response
            .read(&mut buffer)
            .context("Failed to read factory game pack response")?;

        if read == 0 {
            break;
        }

        file.write_all(&buffer[..read])
            .with_context(|| format!("Failed to write {}", path.display()))?;
        current += read as u64;

        update(InstallUpdate::Downloading {
            downloaded_bytes: already_downloaded + current,
            total_bytes,
        });
    }

    Ok(())
}

fn unpack_package<F>(download_dir: &Path, game_dir: &Path, update: &mut F) -> anyhow::Result<()>
where
    F: FnMut(InstallUpdate),
{
    let first_pack = pack_path(download_dir, 1);
    let archive_size = first_pack.metadata().map(|meta| meta.len()).unwrap_or(0);
    let windows_7zip = ensure_windows_7zip(update)?;
    let config = anime_launcher_sdk::genshin::config::Config::get()?;
    let Some(wine) = config.get_selected_wine()? else {
        anyhow::bail!("Couldn't find wine executable for Windows 7-Zip");
    };
    let wine_binary = config
        .game
        .wine
        .builds
        .join(&wine.name)
        .join(wine.files.wine64.as_ref().unwrap_or(&wine.files.wine));
    let windows_archive = wine_path(&first_pack);
    let windows_output_arg = format!("-o{}", wine_path(game_dir));

    update(InstallUpdate::Debug {
        message: format!("7z archive: {}", first_pack.display()),
    });
    update(InstallUpdate::Debug {
        message: format!("7z output: {}", game_dir.display()),
    });
    update(InstallUpdate::Debug {
        message: format!("7z first part size: {archive_size} bytes"),
    });
    update(InstallUpdate::Debug {
        message: format!("7z exe: {}", windows_7zip.display()),
    });
    update(InstallUpdate::Debug {
        message: format!("wine: {}", wine_binary.display()),
    });
    update(InstallUpdate::Debug {
        message: format!(
            "7z command: wine '{}' x '{}' '{}' -y -bsp1",
            windows_7zip.display(),
            windows_archive,
            windows_output_arg
        ),
    });

    let mut child = Command::new(&wine_binary)
        .arg(&windows_7zip)
        .arg("x")
        .arg(&windows_archive)
        .arg(&windows_output_arg)
        .arg("-y")
        .arg("-bsp1")
        .env("WINEPREFIX", &config.game.wine.prefix)
        .env("WINEARCH", "win64")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to run Windows 7-Zip through Wine")?;

    update(InstallUpdate::Debug {
        message: format!("7z pid: {}", child.id()),
    });

    let (tx, rx) = mpsc::channel();

    if let Some(stdout) = child.stdout.take() {
        spawn_7z_reader("stdout", stdout, tx.clone());
    }

    if let Some(stderr) = child.stderr.take() {
        spawn_7z_reader("stderr", stderr, tx);
    }

    let mut last_percent = None;
    let started = Instant::now();
    let mut last_heartbeat = Instant::now();
    let mut last_debug_line = String::new();
    let mut stderr_tail = Vec::<String>::new();

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(SevenZipEvent::Line { stream, line }) => {
                if let Some(percent) = parse_7z_percent(line.as_bytes()) {
                    if last_percent != Some(percent) {
                        update(InstallUpdate::UnpackingProgress { percent });
                        last_percent = Some(percent);
                    }
                }

                if line != last_debug_line {
                    update(InstallUpdate::Debug {
                        message: format!("7z {stream}: {line}"),
                    });
                    last_debug_line = line.clone();
                }

                if stream == "stderr" {
                    stderr_tail.push(line);

                    if stderr_tail.len() > 20 {
                        stderr_tail.remove(0);
                    }
                }
            }

            Ok(SevenZipEvent::ReaderFinished { stream }) => {
                update(InstallUpdate::Debug {
                    message: format!("7z {stream}: stream closed"),
                });
            }

            Err(mpsc::RecvTimeoutError::Timeout) => {}

            Err(mpsc::RecvTimeoutError::Disconnected) => {}
        }

        if let Some(status) = child.try_wait().context("Failed to poll 7z")? {
            update(InstallUpdate::Debug {
                message: format!(
                    "7z exited after {}s with {status}",
                    started.elapsed().as_secs()
                ),
            });

            if !status.success() {
                anyhow::bail!(
                    "7z failed to unpack factory game: {}",
                    stderr_tail.join("\n")
                );
            }

            break;
        }

        if last_heartbeat.elapsed() >= Duration::from_secs(5) {
            update(InstallUpdate::Debug {
                message: format!(
                    "7z still running: elapsed={}s, last_progress={}%",
                    started.elapsed().as_secs(),
                    last_percent.unwrap_or(0)
                ),
            });
            last_heartbeat = Instant::now();
        }
    }

    update(InstallUpdate::UnpackingProgress { percent: 100 });
    update(InstallUpdate::Debug {
        message: "7z finished successfully".to_string(),
    });

    Ok(())
}

fn ensure_windows_7zip<F>(update: &mut F) -> anyhow::Result<PathBuf>
where
    F: FnMut(InstallUpdate),
{
    let config = anime_launcher_sdk::genshin::config::Config::get()?;
    let target_dir = config
        .game
        .wine
        .prefix
        .join("drive_c")
        .join("factory-game-tools")
        .join("7zip");
    let target_exe = target_dir.join("7za.exe");

    if target_exe.exists() {
        update(InstallUpdate::Debug {
            message: format!("Windows 7-Zip already installed: {}", target_exe.display()),
        });

        return Ok(target_exe);
    }

    update(InstallUpdate::Debug {
        message: format!("installing Windows 7-Zip {SEVEN_ZIP_VERSION} into Wine prefix"),
    });

    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("Failed to create {}", target_dir.display()))?;

    let tools_dir = crate::CACHE_FOLDER.join("tools").join("7zip");
    std::fs::create_dir_all(&tools_dir)
        .with_context(|| format!("Failed to create {}", tools_dir.display()))?;

    let archive = tools_dir.join("7z2601-extra.7z");

    if !archive.exists() || archive.metadata().map(|meta| meta.len()).unwrap_or(0) == 0 {
        update(InstallUpdate::Debug {
            message: format!("downloading Windows 7-Zip from {SEVEN_ZIP_EXTRA_URL}"),
        });
        download_file(SEVEN_ZIP_EXTRA_URL, &archive)?;
    } else {
        update(InstallUpdate::Debug {
            message: format!("using cached Windows 7-Zip archive: {}", archive.display()),
        });
    }

    let extract_dir = tools_dir.join("7z2601-extra");
    if !extract_dir.join("x64").join("7za.exe").exists() {
        std::fs::create_dir_all(&extract_dir)
            .with_context(|| format!("Failed to create {}", extract_dir.display()))?;

        update(InstallUpdate::Debug {
            message: format!(
                "extracting Windows 7-Zip extra archive to {}",
                extract_dir.display()
            ),
        });

        let output = Command::new("7z")
            .arg("x")
            .arg(&archive)
            .arg(format!("-o{}", extract_dir.display()))
            .arg("-y")
            .output()
            .context("Failed to extract Windows 7-Zip extra archive with system 7z")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to extract Windows 7-Zip extra archive: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    for file in ["7za.exe", "7za.dll", "7zxa.dll"] {
        let from = extract_dir.join("x64").join(file);
        let to = target_dir.join(file);

        std::fs::copy(&from, &to).with_context(|| {
            format!(
                "Failed to install Windows 7-Zip file {} to {}",
                from.display(),
                to.display()
            )
        })?;
    }

    update(InstallUpdate::Debug {
        message: format!("Windows 7-Zip installed: {}", target_exe.display()),
    });

    Ok(target_exe)
}

fn download_file(url: &str, path: &Path) -> anyhow::Result<()> {
    let mut response = Client::new()
        .get(url)
        .send()
        .with_context(|| format!("Failed to download {url}"))?
        .error_for_status()
        .with_context(|| format!("7-Zip download returned an error: {url}"))?;

    let mut file =
        File::create(path).with_context(|| format!("Failed to create {}", path.display()))?;
    let mut buffer = [0; 128 * 1024];

    loop {
        let read = response
            .read(&mut buffer)
            .with_context(|| format!("Failed to read {url}"))?;

        if read == 0 {
            break;
        }

        file.write_all(&buffer[..read])
            .with_context(|| format!("Failed to write {}", path.display()))?;
    }

    Ok(())
}

fn wine_path(path: &Path) -> String {
    let mut result = String::from("Z:");

    for component in path.components() {
        match component {
            std::path::Component::RootDir => result.push('\\'),
            std::path::Component::Normal(value) => {
                if !result.ends_with('\\') {
                    result.push('\\');
                }

                result.push_str(&value.to_string_lossy());
            }
            _ => (),
        }
    }

    result
}

enum SevenZipEvent {
    Line { stream: &'static str, line: String },
    ReaderFinished { stream: &'static str },
}

fn spawn_7z_reader<R>(stream: &'static str, mut reader: R, tx: mpsc::Sender<SevenZipEvent>)
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut line = Vec::new();
        let mut byte = [0; 1];

        loop {
            match reader.read(&mut byte) {
                Ok(0) => break,
                Ok(_) => {
                    if byte[0] == b'\r' || byte[0] == b'\n' {
                        if let Some(line) = clean_7z_line(&line) {
                            let _ = tx.send(SevenZipEvent::Line { stream, line });
                        }

                        line.clear();
                    } else {
                        line.push(byte[0]);
                    }
                }
                Err(_) => break,
            }
        }

        if let Some(line) = clean_7z_line(&line) {
            let _ = tx.send(SevenZipEvent::Line { stream, line });
        }

        let _ = tx.send(SevenZipEvent::ReaderFinished { stream });
    });
}

fn parse_7z_percent(output: &[u8]) -> Option<u8> {
    let output = String::from_utf8_lossy(output);

    output
        .match_indices('%')
        .filter_map(|(index, _)| parse_percent_prefix(&output[..index]))
        .last()
}

fn parse_percent_prefix(value: &str) -> Option<u8> {
    let mut digits = String::new();

    for ch in value.trim_end().chars().rev() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else if !digits.is_empty() {
            break;
        }
    }

    if digits.is_empty() {
        return None;
    }

    let digits = digits.chars().rev().collect::<String>();
    let percent = digits.parse::<u8>().ok()?;

    (percent <= 100).then_some(percent)
}

fn clean_7z_line(output: &[u8]) -> Option<String> {
    let line = String::from_utf8_lossy(output)
        .replace('\r', "")
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    (!line.is_empty()).then_some(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_known_pack_urls() {
        let package = known_full_package();

        assert_eq!(
            package.pack_url(1).as_deref(),
            Some(
                "https://beyond.hg-cdn.com/YDUTE5gscDZ229CW/1.3/update/6/6/Windows/1.3.3_BGlQHu2HgMlqTuqG/packs/Beyond_Release_v1d3-Rel-os-7123154-1_prod_obt_official.zip.001"
            )
        );
        assert!(package.pack_url(0).is_none());
        assert!(package.pack_url(50).is_none());
    }

    #[test]
    fn keeps_official_bootstrap_query() {
        assert_eq!(
            latest_launcher_url(),
            format!(
                "https://launcher.gryphline.com/api/launcher/get_latest_launcher?appcode={LAUNCHER_APP_CODE}&channel={CHANNEL}&sub_channel={SUB_CHANNEL}&ua=&ta={REMOTE_TARGET_APP}&tracking={TRACKING}"
            )
        );
    }

    #[test]
    fn knows_compressed_download_size() {
        let package = known_full_package();

        assert_eq!(
            package.download_size(),
            48 * PACKAGE_PACK_SIZE + PACKAGE_LAST_PACK_SIZE
        );
    }

    #[test]
    fn detects_old_default_game_paths() {
        assert!(is_default_genshin_path(Path::new(
            "/tmp/a-factory-game-launcher/Genshin Impact"
        )));
        assert!(is_default_genshin_path(Path::new(
            "/tmp/a-factory-game-launcher/YuanShen"
        )));
        assert!(!is_default_genshin_path(Path::new(
            "/tmp/a-factory-game-launcher/Factory Game"
        )));
    }

    #[test]
    fn parses_7z_progress_lines() {
        assert_eq!(parse_7z_percent(b"  0% 12 - file.bin\r"), Some(0));
        assert_eq!(parse_7z_percent(b" 42% 123 - file.bin\r"), Some(42));
        assert_eq!(parse_7z_percent(b"Everything is Ok\n100%\r"), Some(100));
        assert_eq!(parse_7z_percent(b"No percent here"), None);
    }

    #[test]
    fn cleans_7z_lines_for_debug_log() {
        assert_eq!(
            clean_7z_line(b" 42% 123 - file.bin\r").as_deref(),
            Some("42% 123 - file.bin")
        );
        assert_eq!(clean_7z_line(b"\r\n").as_deref(), None);
    }

    #[test]
    fn caches_pack_checksums() {
        let dir =
            std::env::temp_dir().join(format!("factory-game-cache-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let path = dir.join("pack.001");
        std::fs::write(&path, b"abc").unwrap();

        let mut cache = ChecksumCache::default();
        let first =
            validate_pack(&path, 1, 3, "900150983cd24fb0d6963f7d28e17f72", &mut cache).unwrap();

        assert!(first.valid);
        assert!(first.cache_updated);
        assert!(first.message.contains("cache miss"));

        save_checksum_cache(&dir, &cache).unwrap();

        let mut cache = load_checksum_cache(&dir);
        let second =
            validate_pack(&path, 1, 3, "900150983cd24fb0d6963f7d28e17f72", &mut cache).unwrap();

        assert!(second.valid);
        assert!(!second.cache_updated);
        assert!(second.message.contains("cache hit"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn converts_unix_paths_for_wine() {
        assert_eq!(
            wine_path(Path::new("/home/au/game/file.zip.001")),
            r"Z:\home\au\game\file.zip.001"
        );
    }
}
