//! TrollLauncher skin library: skins saved independently of accounts, with
//! folders, favorites and per-skin cape choice.
//!
//! Layout inside the settings directory:
//!   skin-library/library.json   metadata
//!   skin-library/skins/<id>.png textures (id = first 16 bytes of SHA-256, hex)

use std::collections::HashMap;
use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use bytes::Bytes;
use chrono::Utc;
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use crate::ErrorKind;
use crate::state::DirectoryInfo;
use crate::util::fetch::REQWEST_CLIENT;

const LIBRARY_DIR: &str = "skin-library";
const LIBRARY_FILE: &str = "library.json";
const MAX_TEXTURE_BYTES: usize = 512 * 1024;

static LIBRARY_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum LibrarySkinVariant {
    Classic,
    Slim,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LibrarySkin {
    pub id: String,
    pub name: String,
    pub variant: LibrarySkinVariant,
    /// Folder name, `None` = no folder.
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    /// `mojang:<cape uuid>`, `optifine`, `elyby` or `None`.
    #[serde(default)]
    pub cape: Option<String>,
    /// Where it came from: `file`, `url`, `mojang:<nick>`, `elyby:<nick>`, `account`.
    #[serde(default)]
    pub source: String,
    pub added: i64,
    /// `data:image/png;base64,…`, filled when the library is read.
    #[serde(default, skip_deserializing)]
    pub texture: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SkinLibrary {
    #[serde(default)]
    pub folders: Vec<String>,
    #[serde(default)]
    pub skins: Vec<LibrarySkin>,
    /// Account (profile UUID) → id of the library skin last put on it.
    #[serde(default)]
    pub equipped: HashMap<String, String>,
}

/// A downloaded texture that is not saved yet.
#[derive(Serialize, Clone, Debug)]
pub struct FetchedSkin {
    pub texture: Bytes,
    pub variant: LibrarySkinVariant,
    pub name: String,
}

/// Cape textures found for a nickname, as data URLs.
#[derive(Serialize, Clone, Debug, Default)]
pub struct ExtraCapes {
    pub optifine: Option<String>,
    pub elyby: Option<String>,
}

fn library_dir() -> crate::Result<PathBuf> {
    DirectoryInfo::global_handle_if_ready()
        .map(|dirs| dirs.settings_dir.join(LIBRARY_DIR))
        .ok_or_else(|| {
            ErrorKind::OtherError("Launcher directories are not ready".into())
                .into()
        })
}

fn texture_path(dir: &std::path::Path, id: &str) -> PathBuf {
    dir.join("skins").join(format!("{id}.png"))
}

fn data_url(bytes: &[u8]) -> String {
    format!("data:image/png;base64,{}", BASE64.encode(bytes))
}

async fn read_library(dir: &std::path::Path) -> SkinLibrary {
    tokio::fs::read(dir.join(LIBRARY_FILE))
        .await
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

async fn write_library(
    dir: &std::path::Path,
    library: &SkinLibrary,
) -> crate::Result<()> {
    let bytes = serde_json::to_vec_pretty(library)?;
    crate::util::io::create_dir_all(dir).await?;
    crate::util::io::write(dir.join(LIBRARY_FILE), bytes).await?;
    Ok(())
}

/// Checks the PNG is a Minecraft skin and guesses its arm model.
fn inspect_skin(bytes: &[u8]) -> crate::Result<LibrarySkinVariant> {
    if bytes.len() > MAX_TEXTURE_BYTES {
        return Err(ErrorKind::OtherError(
            "Файл слишком большой для скина".into(),
        )
        .into());
    }
    let image = image::load_from_memory_with_format(
        bytes,
        image::ImageFormat::Png,
    )
    .map_err(|_| ErrorKind::OtherError("Это не PNG-картинка".into()))?;
    let (width, height) = image.dimensions();
    if width != 64 || (height != 64 && height != 32) {
        return Err(ErrorKind::OtherError(format!(
            "Скин должен быть 64×64 или 64×32, а этот {width}×{height}"
        ))
        .into());
    }
    if height == 32 {
        return Ok(LibrarySkinVariant::Classic);
    }
    // Slim skins leave the outer column of the right arm transparent.
    let slim = (20..32).all(|y| {
        (54..56).all(|x| image.get_pixel(x, y).0[3] == 0)
    });
    Ok(if slim {
        LibrarySkinVariant::Slim
    } else {
        LibrarySkinVariant::Classic
    })
}

fn skin_id(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest[..16].iter().map(|b| format!("{b:02x}")).collect()
}

pub async fn get() -> crate::Result<SkinLibrary> {
    let dir = library_dir()?;
    let _guard = LIBRARY_LOCK.lock().await;
    let mut library = read_library(&dir).await;
    let mut missing = Vec::new();
    for skin in &mut library.skins {
        match tokio::fs::read(texture_path(&dir, &skin.id)).await {
            Ok(bytes) => skin.texture = data_url(&bytes),
            Err(_) => missing.push(skin.id.clone()),
        }
    }
    library.skins.retain(|skin| !missing.contains(&skin.id));
    Ok(library)
}

/// Saves a texture to the library. Adding the same picture twice returns the
/// existing entry (with name/folder unchanged).
pub async fn add(
    texture: Vec<u8>,
    name: String,
    variant: Option<LibrarySkinVariant>,
    folder: Option<String>,
    source: String,
) -> crate::Result<LibrarySkin> {
    let detected = inspect_skin(&texture)?;
    let dir = library_dir()?;
    let _guard = LIBRARY_LOCK.lock().await;
    let mut library = read_library(&dir).await;
    let id = skin_id(&texture);

    if let Some(existing) = library.skins.iter().find(|skin| skin.id == id) {
        let mut existing = existing.clone();
        existing.texture = data_url(&texture);
        return Ok(existing);
    }

    crate::util::io::create_dir_all(dir.join("skins")).await?;
    crate::util::io::write(texture_path(&dir, &id), &texture).await?;
    let name = name.trim();
    let mut skin = LibrarySkin {
        id,
        name: if name.is_empty() {
            "Скин".to_owned()
        } else {
            name.chars().take(64).collect()
        },
        variant: variant.unwrap_or(detected),
        folder: folder.filter(|f| library.folders.contains(f)),
        favorite: false,
        cape: None,
        source,
        added: Utc::now().timestamp(),
        texture: String::new(),
    };
    library.skins.push(skin.clone());
    write_library(&dir, &library).await?;
    skin.texture = data_url(&texture);
    Ok(skin)
}

/// Updates name, model, folder, favorite flag and cape of a saved skin.
pub async fn update(skin: LibrarySkin) -> crate::Result<()> {
    let dir = library_dir()?;
    let _guard = LIBRARY_LOCK.lock().await;
    let mut library = read_library(&dir).await;
    let folders = library.folders.clone();
    let Some(entry) = library.skins.iter_mut().find(|s| s.id == skin.id)
    else {
        return Err(ErrorKind::OtherError("Скин не найден".into()).into());
    };
    let name = skin.name.trim();
    if !name.is_empty() {
        entry.name = name.chars().take(64).collect();
    }
    entry.variant = skin.variant;
    entry.folder = skin.folder.filter(|f| folders.contains(f));
    entry.favorite = skin.favorite;
    entry.cape = skin.cape;
    write_library(&dir, &library).await
}

pub async fn remove(id: &str) -> crate::Result<()> {
    let dir = library_dir()?;
    let _guard = LIBRARY_LOCK.lock().await;
    let mut library = read_library(&dir).await;
    library.skins.retain(|skin| skin.id != id);
    library.equipped.retain(|_, skin| skin != id);
    write_library(&dir, &library).await?;
    let _ = tokio::fs::remove_file(texture_path(&dir, id)).await;
    Ok(())
}

/// Replaces the folder list (also used for renaming: `renamed` maps old → new).
pub async fn set_folders(
    folders: Vec<String>,
    renamed: HashMap<String, String>,
) -> crate::Result<()> {
    let dir = library_dir()?;
    let _guard = LIBRARY_LOCK.lock().await;
    let mut library = read_library(&dir).await;
    let mut cleaned: Vec<String> = Vec::new();
    for folder in folders {
        let folder: String = folder.trim().chars().take(40).collect();
        if !folder.is_empty() && !cleaned.contains(&folder) {
            cleaned.push(folder);
        }
    }
    for skin in &mut library.skins {
        if let Some(folder) = &skin.folder {
            if let Some(new_name) = renamed.get(folder) {
                skin.folder = Some(new_name.clone());
            }
            if !skin.folder.as_ref().is_some_and(|f| cleaned.contains(f)) {
                skin.folder = None;
            }
        }
    }
    library.folders = cleaned;
    write_library(&dir, &library).await
}

pub async fn set_equipped(account: String, id: String) -> crate::Result<()> {
    let dir = library_dir()?;
    let _guard = LIBRARY_LOCK.lock().await;
    let mut library = read_library(&dir).await;
    library.equipped.insert(account, id);
    write_library(&dir, &library).await
}

pub async fn texture(id: &str) -> crate::Result<Bytes> {
    let dir = library_dir()?;
    Ok(Bytes::from(tokio::fs::read(texture_path(&dir, id)).await?))
}

pub async fn export(id: &str, path: PathBuf) -> crate::Result<()> {
    let bytes = texture(id).await?;
    crate::util::io::write(path, &bytes).await?;
    Ok(())
}

async fn download(url: &str) -> crate::Result<Bytes> {
    let response = REQWEST_CLIENT
        .get(url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(ErrorKind::OtherError(format!(
            "Не удалось скачать: {}",
            response.status()
        ))
        .into());
    }
    let bytes = response.bytes().await?;
    if bytes.len() > MAX_TEXTURE_BYTES {
        return Err(ErrorKind::OtherError("Файл слишком большой".into()).into());
    }
    Ok(bytes)
}

#[derive(Deserialize)]
struct TextureEntry {
    url: String,
    #[serde(default)]
    metadata: Option<TextureMetadata>,
}

#[derive(Deserialize)]
struct TextureMetadata {
    #[serde(default)]
    model: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct TextureSet {
    #[serde(default)]
    skin: Option<TextureEntry>,
    #[serde(default)]
    cape: Option<TextureEntry>,
}

#[derive(Deserialize)]
struct MojangTexturesPayload {
    textures: TextureSet,
}

#[derive(Deserialize)]
struct MojangProfileLookup {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct MojangSessionProfile {
    properties: Vec<MojangProperty>,
}

#[derive(Deserialize)]
struct MojangProperty {
    name: String,
    value: String,
}

fn https(url: &str) -> String {
    url.replacen("http://", "https://", 1)
}

async fn mojang_textures(nick: &str) -> crate::Result<(String, TextureSet)> {
    let lookup = REQWEST_CLIENT
        .get(format!(
            "https://api.mojang.com/users/profiles/minecraft/{}",
            urlencoding::encode(nick)
        ))
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await?;
    if !lookup.status().is_success() {
        return Err(ErrorKind::OtherError(format!(
            "Игрок «{nick}» не найден в Minecraft"
        ))
        .into());
    }
    let lookup: MojangProfileLookup = lookup.json().await?;
    let profile: MojangSessionProfile = REQWEST_CLIENT
        .get(format!(
            "https://sessionserver.mojang.com/session/minecraft/profile/{}",
            lookup.id
        ))
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await?
        .json()
        .await?;
    let property = profile
        .properties
        .into_iter()
        .find(|p| p.name == "textures")
        .ok_or_else(|| ErrorKind::OtherError("У игрока нет скина".into()))?;
    let payload: MojangTexturesPayload = serde_json::from_slice(
        &BASE64
            .decode(property.value)
            .map_err(|_| ErrorKind::OtherError("Mojang вернул мусор".into()))?,
    )?;
    Ok((lookup.name, payload.textures))
}

async fn elyby_textures(nick: &str) -> crate::Result<TextureSet> {
    let response = REQWEST_CLIENT
        .get(format!(
            "https://skinsystem.ely.by/textures/{}",
            urlencoding::encode(nick)
        ))
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await?;
    if response.status() != reqwest::StatusCode::OK {
        return Ok(TextureSet {
            skin: None,
            cape: None,
        });
    }
    Ok(response.json().await.unwrap_or(TextureSet {
        skin: None,
        cape: None,
    }))
}

fn model_of(entry: &TextureEntry) -> LibrarySkinVariant {
    if entry
        .metadata
        .as_ref()
        .and_then(|m| m.model.as_deref())
        .is_some_and(|model| model == "slim")
    {
        LibrarySkinVariant::Slim
    } else {
        LibrarySkinVariant::Classic
    }
}

/// Downloads the current skin of a player from Mojang (`mojang`) or Ely.by (`elyby`).
pub async fn fetch_by_nick(
    nick: &str,
    source: &str,
) -> crate::Result<FetchedSkin> {
    let nick = nick.trim();
    if nick.is_empty() {
        return Err(ErrorKind::OtherError("Введи ник".into()).into());
    }
    let (name, set) = if source == "elyby" {
        (nick.to_owned(), elyby_textures(nick).await?)
    } else {
        mojang_textures(nick).await?
    };
    let skin = set.skin.ok_or_else(|| {
        ErrorKind::OtherError(format!("У игрока «{nick}» стандартный скин"))
    })?;
    let texture = download(&https(&skin.url)).await?;
    inspect_skin(&texture)?;
    Ok(FetchedSkin {
        texture,
        variant: model_of(&skin),
        name,
    })
}

pub async fn fetch_url(url: &str) -> crate::Result<FetchedSkin> {
    let parsed = url::Url::parse(url.trim())
        .map_err(|_| ErrorKind::OtherError("Неверная ссылка".into()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ErrorKind::OtherError("Нужна http(s)-ссылка".into()).into());
    }
    let texture = match download(&https(parsed.as_str())).await {
        Ok(texture) => texture,
        Err(_) if parsed.scheme() == "http" => download(parsed.as_str()).await?,
        Err(error) => return Err(error),
    };
    let variant = inspect_skin(&texture)?;
    let name = parsed
        .path_segments()
        .and_then(|mut s| s.next_back().map(str::to_owned))
        .map(|file| file.trim_end_matches(".png").to_owned())
        .filter(|file| !file.is_empty())
        .unwrap_or_else(|| "Скин по ссылке".to_owned());
    Ok(FetchedSkin {
        texture,
        variant,
        name,
    })
}

/// Pads an OptiFine cape (46×22 layout, any scale) to the 64×32 cape layout the
/// renderer expects.
fn normalize_cape(bytes: &[u8]) -> Option<Vec<u8>> {
    let image = image::load_from_memory(bytes).ok()?;
    let (width, height) = image.dimensions();
    let scale = if width % 64 == 0 {
        return Some(bytes.to_vec());
    } else if width % 46 == 0 {
        width / 46
    } else {
        return None;
    };
    let mut canvas = image::RgbaImage::new(64 * scale, 32 * scale);
    image::imageops::overlay(&mut canvas, &image.to_rgba8(), 0, 0);
    let _ = height;
    let mut out = std::io::Cursor::new(Vec::new());
    canvas.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

/// Finds the OptiFine and Ely.by capes for a nickname (missing ones are `None`).
pub async fn extra_capes(nick: &str) -> crate::Result<ExtraCapes> {
    let nick = nick.trim();
    let optifine = async {
        let response = REQWEST_CLIENT
            .get(format!(
                "https://optifine.net/capes/{}.png",
                urlencoding::encode(nick)
            ))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        let bytes = response.bytes().await.ok()?;
        normalize_cape(&bytes).map(|png| data_url(&png))
    };
    let elyby = async {
        let cape = elyby_textures(nick).await.ok()?.cape?;
        let bytes = download(&https(&cape.url)).await.ok()?;
        normalize_cape(&bytes).map(|png| data_url(&png))
    };
    let (optifine, elyby) = tokio::join!(optifine, elyby);
    Ok(ExtraCapes { optifine, elyby })
}
