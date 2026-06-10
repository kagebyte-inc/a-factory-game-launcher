use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anime_launcher_sdk::anime_game_core::installer::downloader::Downloader;
use anime_launcher_sdk::anime_game_core::reqwest::blocking::Client;
use anyhow::{Context, anyhow};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};

use anime_launcher_sdk::is_available;

const GRYPHLINE_BATCH_PROXY_API: &str = "https://launcher.gryphline.com/api/proxy/web/batch_proxy";
const PLATFORM: &str = "Windows";
const SOURCE: &str = "launcher";

pub fn download_background(with_video: bool, index: u8) -> anyhow::Result<()> {
    tracing::debug!("Downloading background picture");

    let backgrounds = get_background_info_multiple()?;
    #[allow(unused_parens, reason = "Clarity in the `find` condition")]
    let info = backgrounds
        .iter()
        .skip(index as usize)
        .find(|bginfo| (!with_video || matches!(bginfo, BackgroundSpec::Video { .. })))
        .or_else(|| backgrounds.get(index as usize))
        .or(backgrounds.first())
        .ok_or(anyhow!(
            "Failed to get background information: no backgrounds in the API"
        ))?;

    let regenerate_image = info.download(with_video)?;

    if regenerate_image {
        if gtk_webp_image_supported() {
            std::fs::copy(&*crate::BACKGROUND_FILE, &*crate::PROCESSED_BACKGROUND_FILE)
                .context("Copying background file")?;
            if matches!(info, BackgroundSpec::Video { .. }) {
                if crate::BACKGROUND_OVERLAY_FILE.exists() {
                    std::fs::copy(
                        &*crate::BACKGROUND_OVERLAY_FILE,
                        &*crate::PROCESSED_BACKGROUND_OVERLAY_FILE,
                    )
                    .context("Copying background overlay file")?;
                }
            }
        } else {
            tracing::info!("WebP GDK Pixbuf Loader is not installed, converting images to PNG");
            info.convert_and_copy()?;
        }
    } else {
        tracing::debug!("Not re-generating the background image, already latest")
    }

    if matches!(info, BackgroundSpec::Normal { .. }) {
        // Remove the overlay and video file if it's normal variant.
        // Ignore error, if file is already missing for example.
        let _ = std::fs::remove_file(&*crate::PROCESSED_BACKGROUND_OVERLAY_FILE);
        let _ = std::fs::remove_file(&*crate::BACKGROUND_VIDEO_FILE);
    }

    Ok(())
}

#[cached::proc_macro::cached(result)]
pub fn get_background_info_multiple() -> anyhow::Result<Vec<BackgroundSpec>> {
    let response = fetch_launcher_backgrounds()?;
    BackgroundSpec::from_batch_proxy_response(&response)
}

#[cached::proc_macro::cached(result)]
pub fn get_background_info(index: u8) -> anyhow::Result<BackgroundSpec> {
    let backgrounds = get_background_info_multiple()?;

    backgrounds
        .get(index as usize)
        .or_else(|| backgrounds.first())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("The API did not provide any backgrounds"))
}

pub fn get_uri() -> String {
    GRYPHLINE_BATCH_PROXY_API.to_owned()
}

#[derive(Debug, Clone)]
pub enum BackgroundSpec {
    Normal {
        background: Background,
    },
    Video {
        background: Background,
        video: Background,
        overlay: Option<Background>,
    },
}

impl BackgroundSpec {
    fn from_batch_proxy_response(response: &BatchProxyResponse) -> anyhow::Result<Vec<Self>> {
        let main_image = response
            .proxy_rsps
            .iter()
            .find(|rsp| rsp.kind == "get_main_bg_image")
            .and_then(|rsp| rsp.get_main_bg_image_rsp.as_ref())
            .and_then(|rsp| rsp.main_bg_image.as_ref())
            .filter(|image| !image.url.is_empty());

        let fallback_banner = response
            .proxy_rsps
            .iter()
            .find(|rsp| rsp.kind == "get_banner")
            .and_then(|rsp| rsp.get_banner_rsp.as_ref())
            .and_then(|rsp| rsp.banners.iter().find(|banner| !banner.url.is_empty()));

        let background = if let Some(main_image) = main_image {
            Background::from_uri_with_hash(main_image.url.clone(), main_image.md5.clone())?
        } else if let Some(fallback_banner) = fallback_banner {
            Background::from_uri_with_hash(
                fallback_banner.url.clone(),
                fallback_banner.md5.clone(),
            )?
        } else {
            anyhow::bail!("GRYPHLINK API did not provide main background or banners");
        };

        if let Some(video_url) =
            main_image.and_then(|image| image.video_url.as_deref().filter(|url| !url.is_empty()))
        {
            Ok(vec![Self::Video {
                background,
                video: Background::from_uri(video_url.to_string())?,
                overlay: None,
            }])
        } else {
            Ok(vec![Self::Normal { background }])
        }
    }

    fn background(&self) -> &Background {
        match self {
            Self::Normal { background } | Self::Video { background, .. } => background,
        }
    }

    /// Returns true if the background needs to be re-generated
    fn download(&self, with_video: bool) -> anyhow::Result<bool> {
        let mut regenerate_image = false;

        regenerate_image |= self.background().download(&crate::BACKGROUND_FILE)?;

        if let Self::Video { video, overlay, .. } = self {
            if let Some(overlay) = overlay {
                regenerate_image |= overlay.download(&crate::BACKGROUND_OVERLAY_FILE)?;
            } else {
                let _ = std::fs::remove_file(&*crate::BACKGROUND_OVERLAY_FILE);
                let _ = std::fs::remove_file(&*crate::PROCESSED_BACKGROUND_OVERLAY_FILE);
            }
            if with_video {
                regenerate_image |= video.download(&crate::BACKGROUND_VIDEO_FILE)?;
            }
        }

        Ok(regenerate_image)
    }

    fn convert_and_copy(&self) -> anyhow::Result<()> {
        finalize_file(
            self.background(),
            &crate::BACKGROUND_FILE,
            &crate::PROCESSED_BACKGROUND_FILE,
        )?;
        if let Self::Video { overlay, .. } = self {
            if let Some(overlay) = overlay {
                finalize_file(
                    overlay,
                    &crate::BACKGROUND_OVERLAY_FILE,
                    &crate::PROCESSED_BACKGROUND_OVERLAY_FILE,
                )?;
            }
        }
        Ok(())
    }
}

fn finalize_file(bg_info: &Background, from: &Path, to: &Path) -> anyhow::Result<()> {
    if bg_info.uri.ends_with(".webp") {
        convert_image(from, to).context(format!("Converting image {to:?}"))?;
    }

    // If it failed to re-code the file - just copy it
    // Will happen with HSR because devs apparently named
    // their background image ".webp" while it's JPEG
    if !to.exists() {
        std::fs::copy(from, to).context(format!("Copying {to:?}"))?;
    }

    Ok(())
}

fn convert_image(from: &Path, to: &Path) -> anyhow::Result<()> {
    if is_available("dwebp") {
        Command::new("dwebp")
            .arg(from)
            .arg("-o")
            .arg(to)
            .spawn()?
            .wait()?;
    } else if is_available("magick") {
        Command::new("magick")
            .arg(from)
            .arg(format!("PNG:{}", to.display()))
            .spawn()?
            .wait()?;
    } else {
        tracing::warn!("Could not find `dwebp` or `magick` to convert the image file.");
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Background {
    pub uri: String,
    pub hash: Option<String>,
}

impl Background {
    fn from_uri(uri: String) -> anyhow::Result<Self> {
        Self::from_uri_with_hash(uri, None)
    }

    fn from_uri_with_hash(uri: String, hash: Option<String>) -> anyhow::Result<Self> {
        anyhow::ensure!(
            !uri.is_empty(),
            "GRYPHLINK API returned an empty background URL"
        );

        Ok(Self {
            hash: hash.or_else(|| get_img_hash_from_uri(&uri)),
            uri,
        })
    }

    /// Return true if the background needs to be re-generated
    fn download(&self, path: &Path) -> anyhow::Result<bool> {
        if !check_img_file(path, self.hash.as_deref(), &self.uri)? {
            download_img_file(path, &self.uri)?;
            write_img_uri_cache(path, &self.uri)?;
            return Ok(true);
        }
        Ok(false)
    }
}

/// Returns true if image exists and is correct
fn check_img_file(path: &Path, expected_hash: Option<&str>, uri: &str) -> anyhow::Result<bool> {
    if path.exists() {
        if let Some(expected_hash) = expected_hash {
            let hash = Md5::digest(std::fs::read(path)?);

            if format!("{hash:x}").eq_ignore_ascii_case(expected_hash) {
                tracing::debug!("Background picture {path:?} already downloaded. Skipping");

                return Ok(true);
            }
        } else if std::fs::read_to_string(img_uri_cache_path(path))
            .map(|cached_uri| cached_uri.trim() == uri)
            .unwrap_or(false)
        {
            tracing::debug!("Background picture {path:?} already downloaded. Skipping");

            return Ok(true);
        }
    }

    Ok(false)
}

fn get_img_hash_from_uri(uri: &str) -> Option<String> {
    let hash = uri
        .split('/')
        .next_back()
        .unwrap_or_default()
        .split(['_', '.'])
        .next()
        .unwrap_or_default();

    (hash.len() == 32 && hash.chars().all(|ch| ch.is_ascii_hexdigit())).then(|| hash.to_owned())
}

#[cached::proc_macro::once()]
fn gtk_webp_image_supported() -> bool {
    let supported_pixbuf_formats = gtk::gdk_pixbuf::Pixbuf::formats();
    supported_pixbuf_formats.into_iter().any(|format| {
        format
            .name()
            .map(|name| name.eq_ignore_ascii_case("webp"))
            .unwrap_or(false)
            || format
                .extensions()
                .iter()
                .any(|ext| ext.eq_ignore_ascii_case("webp"))
    })
}

fn download_img_file(path: &Path, uri: &str) -> anyhow::Result<()> {
    let mut downloader = Downloader::new(uri)?;

    downloader.continue_downloading = false;

    if let Err(err) = downloader.download(path, |_, _| {}) {
        anyhow::bail!(err);
    }

    Ok(())
}

fn img_uri_cache_path(path: &Path) -> std::path::PathBuf {
    path.with_extension("url")
}

fn write_img_uri_cache(path: &Path, uri: &str) -> anyhow::Result<()> {
    std::fs::write(img_uri_cache_path(path), uri).context("Writing background URL cache")
}

fn fetch_launcher_backgrounds() -> anyhow::Result<BatchProxyResponse> {
    let lang = crate::i18n::format_lang(crate::i18n::get_lang());
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .context("Failed to create GRYPHLINK API client")?;

    client
        .post(GRYPHLINE_BATCH_PROXY_API)
        .json(&BatchProxyRequest::main_background(lang))
        .send()
        .context("Failed to request GRYPHLINK launcher background")?
        .error_for_status()
        .context("GRYPHLINK launcher background API returned an error")?
        .json()
        .context("Failed to decode GRYPHLINK launcher background response")
}

#[derive(Debug, Serialize)]
struct BatchProxyRequest {
    proxy_reqs: Vec<ProxyRequest>,
}

impl BatchProxyRequest {
    fn main_background(language: String) -> Self {
        Self {
            proxy_reqs: vec![
                ProxyRequest {
                    kind: "get_main_bg_image",
                    get_main_bg_image_req: Some(MainBackgroundRequest::new(language.clone())),
                    get_banner_req: None,
                },
                ProxyRequest {
                    kind: "get_banner",
                    get_main_bg_image_req: None,
                    get_banner_req: Some(MainBackgroundRequest::new(language)),
                },
            ],
        }
    }
}

#[derive(Debug, Serialize)]
struct ProxyRequest {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    get_main_bg_image_req: Option<MainBackgroundRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    get_banner_req: Option<MainBackgroundRequest>,
}

#[derive(Debug, Serialize)]
struct MainBackgroundRequest {
    appcode: &'static str,
    language: String,
    channel: &'static str,
    sub_channel: &'static str,
    platform: &'static str,
    source: &'static str,
}

impl MainBackgroundRequest {
    fn new(language: String) -> Self {
        Self {
            appcode: crate::factory_game::GAME_APP_CODE,
            language,
            channel: crate::factory_game::CHANNEL,
            sub_channel: crate::factory_game::SUB_CHANNEL,
            platform: PLATFORM,
            source: SOURCE,
        }
    }
}

#[derive(Debug, Deserialize)]
struct BatchProxyResponse {
    proxy_rsps: Vec<ProxyResponse>,
}

#[derive(Debug, Deserialize)]
struct ProxyResponse {
    kind: String,
    get_main_bg_image_rsp: Option<MainBackgroundResponse>,
    get_banner_rsp: Option<BannerResponse>,
}

#[derive(Debug, Deserialize)]
struct MainBackgroundResponse {
    main_bg_image: Option<MainBackgroundImage>,
}

#[derive(Debug, Deserialize)]
struct MainBackgroundImage {
    url: String,
    md5: Option<String>,
    video_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BannerResponse {
    banners: Vec<BannerImage>,
}

#[derive(Debug, Deserialize)]
struct BannerImage {
    url: String,
    md5: Option<String>,
}
