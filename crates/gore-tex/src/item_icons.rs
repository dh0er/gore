//! Generation-bound, immutable PNG cache for every item icon used by Save Editor.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use image::{ImageDecoder as _, ImageEncoder as _};
use serde::{Deserialize, Serialize};

use crate::container::{
    InstalledTextureComposite, OpenTexturePreviewBatch, VerifiedLegacySidecarKind,
};
use crate::error::{Result, TexError};
use crate::index::OpenTextureGeneration;

pub const ITEM_ICON_CACHE_SCHEMA: u32 = 1;

const CACHE_DIRECTORY_PREFIX: &str = "item-icons-v1-";
const MANIFEST_FILE_NAME: &str = "manifest.json";
const MAX_ITEMS: usize = 4_096;
const MAX_ITEM_ID_BYTES: usize = 512;
const MAX_ICON_ID_BYTES: usize = 256;
const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
// Shipped item icons are 256x256 and around 35-85 KiB. These bounds retain
// generous headroom for a later game build without accepting general-purpose
// multi-megapixel textures or a multi-gigabyte cache through this narrow path.
const MAX_ICON_DIMENSION: u32 = 512;
const MAX_ICON_PNG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_PREVIEW_USMAP_BYTES: u64 = 64 * 1024 * 1024;
const MAX_DECODED_ICON_BYTES: u64 = MAX_ICON_DIMENSION as u64 * MAX_ICON_DIMENSION as u64 * 4;
// The current ~678 unique 256x256 icons decode to about 170 MiB and encode to
// well below 100 MiB. Bound both cumulative proof work and published disk use.
const MAX_CACHE_PNG_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CACHE_DECODED_BYTES: u64 = 512 * 1024 * 1024;
static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemIconSpec {
    pub item_id: String,
    pub icon_id: String,
}

impl ItemIconSpec {
    pub fn new(item_id: impl Into<String>, icon_id: impl Into<String>) -> Self {
        Self {
            item_id: item_id.into(),
            icon_id: icon_id.into(),
        }
    }
}

/// The on-disk contract consumed by Save Editor. Every image path is relative
/// to the directory containing `manifest.json`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemIconManifest {
    pub schema: u32,
    pub build_id: String,
    pub item_count: usize,
    pub items: BTreeMap<String, String>,
    pub files: BTreeMap<String, ItemIconFileSeal>,
}

/// Complete bounded proof for one generated PNG. Reuse still decodes and
/// hashes the file; these values are comparisons, not a header-only shortcut.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemIconFileSeal {
    pub width: u32,
    pub height: u32,
    pub byte_length: u64,
    pub decoded_byte_length: u64,
    pub png_blake3: String,
    pub rgba_blake3: String,
}

/// Prepare (or reuse) the complete icon cache for one installed game generation.
/// The game installation is read-only; only the shared per-user `gore` directory
/// receives an immutable generation directory.
pub fn prepare_item_icon_cache(game_root: &Path, items: &[ItemIconSpec]) -> Result<PathBuf> {
    let cache_root = gore_loc::paths::shared_data_dir();
    std::fs::create_dir_all(&cache_root)?;
    let utoc = crate::paths::main_container(game_root)?;
    let usmap = crate::paths::usmap(game_root)?;
    let mut source = InstalledItemIconSource::open(&utoc, &usmap, &cache_root)?;
    prepare_item_icon_cache_with_source(&cache_root, items, &mut source)
}

struct PreparedCatalog {
    item_to_png: BTreeMap<String, String>,
    icon_to_png: BTreeMap<String, String>,
    digest: String,
}

struct ExpectedItemIconManifest {
    build_id: String,
    item_count: usize,
    items: BTreeMap<String, String>,
}

#[derive(Default)]
struct CacheBudget {
    png_bytes: u64,
    decoded_bytes: u64,
}

impl CacheBudget {
    fn admit(&mut self, seal: &ItemIconFileSeal) -> bool {
        let Some(png_bytes) = self.png_bytes.checked_add(seal.byte_length) else {
            return false;
        };
        let Some(decoded_bytes) = self.decoded_bytes.checked_add(seal.decoded_byte_length) else {
            return false;
        };
        if png_bytes > MAX_CACHE_PNG_BYTES || decoded_bytes > MAX_CACHE_DECODED_BYTES {
            return false;
        }
        self.png_bytes = png_bytes;
        self.decoded_bytes = decoded_bytes;
        true
    }
}

impl PreparedCatalog {
    fn from_specs(specs: &[ItemIconSpec]) -> Result<Self> {
        if specs.is_empty() || specs.len() > MAX_ITEMS {
            return Err(invalid_data("item icon catalog has an invalid item count"));
        }

        let mut item_to_icon = BTreeMap::new();
        for spec in specs {
            validate_item_id(&spec.item_id)?;
            validate_icon_id(&spec.icon_id)?;
            if item_to_icon
                .insert(spec.item_id.clone(), spec.icon_id.clone())
                .is_some()
            {
                return Err(invalid_data(
                    "item icon catalog contains a duplicate item id",
                ));
            }
        }

        let mut icon_to_png = BTreeMap::new();
        for icon_id in item_to_icon.values() {
            icon_to_png
                .entry(icon_id.clone())
                .or_insert_with(|| icon_relative_path(icon_id));
        }
        let item_to_png = item_to_icon
            .iter()
            .map(|(item_id, icon_id)| {
                (
                    item_id.clone(),
                    icon_to_png
                        .get(icon_id)
                        .expect("every validated icon has a cache path")
                        .clone(),
                )
            })
            .collect();

        let mut hasher = blake3::Hasher::new();
        hasher.update(b"gore-tex.item-icon-catalog.v1\0");
        hasher.update(&(item_to_icon.len() as u64).to_le_bytes());
        for (item_id, icon_id) in &item_to_icon {
            hash_string(&mut hasher, item_id);
            hash_string(&mut hasher, icon_id);
        }

        Ok(Self {
            item_to_png,
            icon_to_png,
            digest: hasher.finalize().to_hex().to_string(),
        })
    }
}

fn validate_item_id(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAX_ITEM_ID_BYTES
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(invalid_data(
            "item icon catalog contains an invalid item id",
        ));
    }
    Ok(())
}

fn validate_icon_id(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAX_ICON_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(invalid_data(
            "item icon catalog contains an invalid icon id",
        ));
    }
    Ok(())
}

fn hash_string(hasher: &mut blake3::Hasher, value: &str) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn icon_relative_path(icon_id: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"gore-tex.item-icon-file.v1\0");
    hash_string(&mut hasher, icon_id);
    format!("images/{}.png", hasher.finalize().to_hex())
}

fn generation_directory(cache_root: &Path, build_id: &str, catalog_digest: &str) -> PathBuf {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"gore-tex.item-icon-cache-generation.v1\0");
    hash_string(&mut hasher, build_id);
    hash_string(&mut hasher, catalog_digest);
    cache_root.join(format!(
        "{CACHE_DIRECTORY_PREFIX}{}",
        hasher.finalize().to_hex()
    ))
}

trait ItemIconSource {
    fn current_build_id(&mut self) -> Result<String>;
    fn write_png(&mut self, asset_path: &str, output: &Path) -> Result<()>;
}

struct InstalledItemIconSource {
    previews: OpenTexturePreviewBatch,
    generation: OpenTextureGeneration,
    initial_build_id: Option<String>,
    usmap_bytes: Option<Vec<u8>>,
}

impl InstalledItemIconSource {
    fn open(utoc: &Path, usmap: &Path, cache_root: &Path) -> Result<Self> {
        // This is the one and only composite open for the entire batch.
        let composite = InstalledTextureComposite::open(utoc)?;
        let generation = OpenTextureGeneration::capture(&composite, usmap, cache_root)?;
        let initial_build_id = Some(generation.captured_build_id().to_string());
        let previews = OpenTexturePreviewBatch::from_composite(composite)?;
        Ok(Self {
            previews,
            generation,
            initial_build_id,
            usmap_bytes: None,
        })
    }

    fn ensure_mapping_loaded(&mut self) -> Result<()> {
        if self.usmap_bytes.is_none() {
            self.usmap_bytes = Some(
                self.generation
                    .read_mapping_bounded(MAX_PREVIEW_USMAP_BYTES)?,
            );
        }
        Ok(())
    }
}

impl ItemIconSource for InstalledItemIconSource {
    fn current_build_id(&mut self) -> Result<String> {
        match self.initial_build_id.take() {
            Some(build_id) => Ok(build_id),
            None => self.generation.current_build_id(),
        }
    }

    fn write_png(&mut self, asset_path: &str, output: &Path) -> Result<()> {
        // Load the bounded mapping exactly once, then reuse it for every decode.
        self.ensure_mapping_loaded()?;
        let usmap_bytes = self
            .usmap_bytes
            .as_deref()
            .expect("mapping was initialized");
        let mut converted = self.previews.unpack(asset_path)?;
        let ubulk = converted
            .sidecars
            .iter_mut()
            .find(|sidecar| sidecar.kind == VerifiedLegacySidecarKind::Bulk)
            .map(|sidecar| std::mem::take(&mut sidecar.bytes))
            .unwrap_or_default();
        let mut info =
            crate::decode::parse(&converted.uasset, &converted.uexp, &ubulk, usmap_bytes)?;
        let decoded_byte_length = bounded_decoded_byte_length(info.width, info.height)
            .ok_or_else(|| invalid_data("item icon dimensions exceed their preview limit"))?;
        let pixels = crate::decode::to_rgba8(&info)?;
        info.mip0.clear();
        info.decoded_rgba = None;
        drop(converted);
        drop(ubulk);

        let expected_pixels = usize::try_from(decoded_byte_length / 4)
            .map_err(|_| invalid_data("item icon dimensions overflow"))?;
        if pixels.len() != expected_pixels {
            return Err(invalid_data(
                "decoded item icon pixel count does not match its dimensions",
            ));
        }
        let rgba_len = pixels
            .len()
            .checked_mul(4)
            .ok_or_else(|| invalid_data("decoded item icon byte count overflows"))?;
        if u64::try_from(rgba_len).ok() != Some(decoded_byte_length) {
            return Err(invalid_data(
                "decoded item icon byte count exceeds its preview limit",
            ));
        }
        let mut rgba = Vec::with_capacity(rgba_len);
        for pixel in pixels {
            rgba.extend_from_slice(&[
                (pixel >> 16) as u8,
                (pixel >> 8) as u8,
                pixel as u8,
                (pixel >> 24) as u8,
            ]);
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)?;
        image::codecs::png::PngEncoder::new(&mut file)
            .write_image(
                &rgba,
                info.width,
                info.height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|error| TexError::PngEncode(error.to_string()))?;
        file.flush()?;
        file.sync_all()?;
        // `rgba`, `info`, and all converted package bytes leave scope here,
        // before the next icon starts.
        Ok(())
    }
}

fn bounded_decoded_byte_length(width: u32, height: u32) -> Option<u64> {
    if width == 0 || height == 0 || width > MAX_ICON_DIMENSION || height > MAX_ICON_DIMENSION {
        return None;
    }
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|bytes| *bytes <= MAX_DECODED_ICON_BYTES)
}

fn prepare_item_icon_cache_with_source(
    cache_root: &Path,
    items: &[ItemIconSpec],
    source: &mut dyn ItemIconSource,
) -> Result<PathBuf> {
    let catalog = PreparedCatalog::from_specs(items)?;
    let build_id_before = source.current_build_id()?;
    validate_build_id(&build_id_before)?;
    let expected = ExpectedItemIconManifest {
        build_id: build_id_before.clone(),
        item_count: catalog.item_to_png.len(),
        items: catalog.item_to_png.clone(),
    };
    let final_directory = generation_directory(cache_root, &build_id_before, &catalog.digest);
    let manifest_path = final_directory.join(MANIFEST_FILE_NAME);

    std::fs::create_dir_all(cache_root)?;
    // Validation, corrupt-generation quarantine, rebuilding, and publication
    // are one cross-process critical section. A crashed process releases the OS
    // lock; the small sentinel remains for the next acquisition.
    let _generation_lock = GenerationLock::acquire(cache_root, &final_directory)?;
    remove_stale_staging_directories(cache_root, &final_directory)?;
    if complete_cache_matches(&final_directory, &expected)? {
        let build_id_after = source.current_build_id()?;
        if build_id_after != build_id_before {
            return Err(TexError::GenerationChanged);
        }
        return Ok(manifest_path);
    }
    match std::fs::symlink_metadata(&final_directory) {
        Ok(_) => {
            quarantine_incomplete_generation(cache_root, &final_directory)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    let mut staging = StagingDirectory::create(cache_root, &final_directory)?;
    let images_directory = staging.path().join("images");
    std::fs::create_dir(&images_directory)?;

    let mut budget = CacheBudget::default();
    let mut files = BTreeMap::new();
    for (icon_id, relative_png) in &catalog.icon_to_png {
        let output = staging.path().join(relative_png);
        let canonical = format!("/Game/UI/Textures/ItemIcons/T_ItemIcon_{icon_id}");
        match source.write_png(&canonical, &output) {
            Ok(()) => {}
            Err(TexError::AssetNotFound(_)) => {
                let lower_i = format!("/Game/UI/Textures/ItemIcons/T_Itemicon_{icon_id}");
                source.write_png(&lower_i, &output)?;
            }
            Err(error) => return Err(error),
        }
        let output_length = std::fs::metadata(&output)?.len();
        let seal = inspect_cached_png(&output, output_length)?.ok_or_else(|| {
            invalid_data("generated item icon is not a complete bounded RGBA PNG")
        })?;
        if !budget.admit(&seal) {
            return Err(invalid_data(
                "generated item icon cache exceeds its cumulative byte limits",
            ));
        }
        files.insert(relative_png.clone(), seal);
    }

    let manifest = ItemIconManifest {
        schema: ITEM_ICON_CACHE_SCHEMA,
        build_id: expected.build_id.clone(),
        item_count: expected.item_count,
        items: expected.items.clone(),
        files,
    };
    write_manifest(staging.path(), &manifest)?;
    let build_id_after = source.current_build_id()?;
    if build_id_after != build_id_before {
        return Err(TexError::GenerationChanged);
    }

    match std::fs::rename(staging.path(), &final_directory) {
        Ok(()) => {
            staging.disarm();
            Ok(manifest_path)
        }
        Err(publication_error) => {
            // Another process may have won the same immutable generation race.
            // Accept it only after validating the complete byte contract.
            if complete_cache_matches(&final_directory, &expected)? {
                Ok(manifest_path)
            } else {
                Err(publication_error.into())
            }
        }
    }
}

fn validate_build_id(build_id: &str) -> Result<()> {
    if build_id.is_empty()
        || build_id.len() > 512
        || build_id.trim() != build_id
        || build_id.chars().any(char::is_control)
    {
        return Err(invalid_data("installed texture build id is invalid"));
    }
    Ok(())
}

fn write_manifest(directory: &Path, manifest: &ItemIconManifest) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|error| {
        TexError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        ))
    })?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(invalid_data("item icon manifest exceeds its byte limit"));
    }
    let path = directory.join(MANIFEST_FILE_NAME);
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(&bytes)?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}

fn complete_cache_matches(directory: &Path, expected: &ExpectedItemIconManifest) -> Result<bool> {
    let directory_metadata = match std::fs::symlink_metadata(directory) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if !directory_metadata.file_type().is_dir() || directory_metadata.file_type().is_symlink() {
        return Ok(false);
    }

    let manifest_path = directory.join(MANIFEST_FILE_NAME);
    let manifest_metadata = match std::fs::symlink_metadata(&manifest_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if !manifest_metadata.file_type().is_file()
        || manifest_metadata.file_type().is_symlink()
        || manifest_metadata.len() > MAX_MANIFEST_BYTES
    {
        return Ok(false);
    }
    let mut manifest_file = match File::open(&manifest_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let metadata = manifest_file.metadata()?;
    if !metadata.file_type().is_file()
        || metadata.len() != manifest_metadata.len()
        || metadata.len() > MAX_MANIFEST_BYTES
    {
        return Ok(false);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    (&mut manifest_file)
        .take(MAX_MANIFEST_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != metadata.len() {
        return Ok(false);
    }
    let manifest: ItemIconManifest = match serde_json::from_slice(&bytes) {
        Ok(manifest) => manifest,
        Err(_) => return Ok(false),
    };
    if manifest.schema != ITEM_ICON_CACHE_SCHEMA
        || manifest.build_id != expected.build_id
        || manifest.item_count != expected.item_count
        || manifest.items != expected.items
    {
        return Ok(false);
    }

    let unique_paths: BTreeSet<_> = manifest.items.values().map(String::as_str).collect();
    let sealed_paths: BTreeSet<_> = manifest.files.keys().map(String::as_str).collect();
    if sealed_paths != unique_paths {
        return Ok(false);
    }
    let mut budget = CacheBudget::default();
    for relative in unique_paths {
        if !relative.starts_with("images/")
            || relative.contains('\\')
            || relative.contains("..")
            || !relative.ends_with(".png")
        {
            return Ok(false);
        }
        let path = directory.join(relative);
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        if !metadata.file_type().is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() == 0
            || metadata.len() > MAX_ICON_PNG_BYTES
        {
            return Ok(false);
        }
        let expected_seal = manifest
            .files
            .get(relative)
            .expect("file-key equality was checked above");
        if metadata.len() != expected_seal.byte_length {
            return Ok(false);
        }
        if !cached_png_matches_seal(&path, expected_seal)? {
            return Ok(false);
        }
        if !budget.admit(expected_seal) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Validate a previously sealed PNG without decoding its complete RGBA payload
/// again. The initial generation decoded every pixel before recording both
/// hashes. On reuse, an exact PNG hash plus the bounded decoder metadata proves
/// that those same bytes are present; any accidental byte change fails before
/// Flutter can consume the file.
fn cached_png_matches_seal(path: &Path, expected: &ItemIconFileSeal) -> Result<bool> {
    const PNG_IEND: [u8; 12] = [0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82];
    if expected.byte_length == 0
        || expected.byte_length > MAX_ICON_PNG_BYTES
        || expected.png_blake3.len() != 64
        || expected.rgba_blake3.len() != 64
    {
        return Ok(false);
    }
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.file_type().is_file() || metadata.len() != expected.byte_length {
        return Ok(false);
    }
    let capacity = usize::try_from(expected.byte_length)
        .map_err(|_| invalid_data("cached item icon length is unsupported"))?;
    let mut png = Vec::with_capacity(capacity);
    (&mut file)
        .take(MAX_ICON_PNG_BYTES.saturating_add(1))
        .read_to_end(&mut png)?;
    if png.len() != capacity
        || !png.ends_with(&PNG_IEND)
        || blake3::hash(&png).to_hex().as_str() != expected.png_blake3.as_str()
    {
        return Ok(false);
    }

    let decoder = match image::codecs::png::PngDecoder::new(Cursor::new(png.as_slice())) {
        Ok(decoder) => decoder,
        Err(_) => return Ok(false),
    };
    let (width, height) = decoder.dimensions();
    let Some(decoded_byte_length) = bounded_decoded_byte_length(width, height) else {
        return Ok(false);
    };
    Ok(decoder.color_type() == image::ColorType::Rgba8
        && width == expected.width
        && height == expected.height
        && decoded_byte_length == expected.decoded_byte_length)
}

fn inspect_cached_png(path: &Path, expected_len: u64) -> Result<Option<ItemIconFileSeal>> {
    const PNG_IEND: [u8; 12] = [0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82];
    if expected_len == 0 || expected_len > MAX_ICON_PNG_BYTES {
        return Ok(None);
    }
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.file_type().is_file() || metadata.len() != expected_len {
        return Ok(None);
    }
    let capacity = usize::try_from(expected_len)
        .map_err(|_| invalid_data("cached item icon length is unsupported"))?;
    let mut png = Vec::with_capacity(capacity);
    (&mut file)
        .take(MAX_ICON_PNG_BYTES.saturating_add(1))
        .read_to_end(&mut png)?;
    if png.len() != capacity || !png.ends_with(&PNG_IEND) {
        return Ok(None);
    }

    let mut decoder = match image::codecs::png::PngDecoder::new(Cursor::new(png.as_slice())) {
        Ok(decoder) => decoder,
        Err(_) => return Ok(None),
    };
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_ICON_DIMENSION);
    limits.max_image_height = Some(MAX_ICON_DIMENSION);
    limits.max_alloc = Some(MAX_DECODED_ICON_BYTES.saturating_mul(4));
    if decoder.set_limits(limits).is_err()
        || decoder.color_type() != image::ColorType::Rgba8
        || decoder.is_apng().unwrap_or(true)
    {
        return Ok(None);
    }
    let (width, height) = decoder.dimensions();
    let Some(decoded_byte_length) = bounded_decoded_byte_length(width, height) else {
        return Ok(None);
    };
    if decoder.total_bytes() != decoded_byte_length {
        return Ok(None);
    }
    let decoded_len = usize::try_from(decoded_byte_length)
        .map_err(|_| invalid_data("cached item icon decode length is unsupported"))?;
    let mut rgba = vec![0_u8; decoded_len];
    if decoder.read_image(&mut rgba).is_err() {
        return Ok(None);
    }

    Ok(Some(ItemIconFileSeal {
        width,
        height,
        byte_length: expected_len,
        decoded_byte_length,
        png_blake3: blake3::hash(&png).to_hex().to_string(),
        rgba_blake3: blake3::hash(&rgba).to_hex().to_string(),
    }))
}

struct GenerationLock {
    _file: File,
}

impl GenerationLock {
    fn acquire(cache_root: &Path, generation: &Path) -> Result<Self> {
        let generation_name = owned_generation_name(cache_root, generation)?;
        let lock_path = cache_root.join(format!(".{generation_name}.lock"));
        let file = open_generation_lock(&lock_path, true)?;
        let held_identity = generation_lock_identity(&file)?;
        lock_generation_file(&file)?;

        // Reopen after the potentially blocking acquire. A replacement of the
        // sentinel name while we waited must not grant a second process an
        // independent lock for the same generation.
        let named = open_generation_lock(&lock_path, false)?;
        if generation_lock_identity(&named)? != held_identity {
            return Err(invalid_data(
                "item icon cache lock changed while acquiring it",
            ));
        }
        Ok(Self { _file: file })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GenerationLockIdentity {
    volume_or_device: u64,
    file: u64,
}

#[cfg(windows)]
fn open_generation_lock(path: &Path, create: bool) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt as _;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create(create)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    options.open(path)
}

#[cfg(unix)]
fn open_generation_lock(path: &Path, create: bool) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create(create)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    options.open(path)
}

#[cfg(not(any(windows, unix)))]
fn open_generation_lock(path: &Path, create: bool) -> std::io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(create)
        .open(path)
}

#[cfg(windows)]
fn generation_lock_identity(file: &File) -> Result<GenerationLockIdentity> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_REPARSE_POINT,
    };

    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: `file` owns a live handle and `information` has the Win32 ABI layout.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if information.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0
    {
        return Err(invalid_data(
            "item icon cache lock is not a plain regular file",
        ));
    }
    Ok(GenerationLockIdentity {
        volume_or_device: u64::from(information.dwVolumeSerialNumber),
        file: (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow),
    })
}

#[cfg(unix)]
fn generation_lock_identity(file: &File) -> Result<GenerationLockIdentity> {
    use std::os::unix::fs::MetadataExt as _;

    let metadata = file.metadata()?;
    if !metadata.file_type().is_file() {
        return Err(invalid_data(
            "item icon cache lock is not a plain regular file",
        ));
    }
    Ok(GenerationLockIdentity {
        volume_or_device: metadata.dev(),
        file: metadata.ino(),
    })
}

#[cfg(not(any(windows, unix)))]
fn generation_lock_identity(_file: &File) -> Result<GenerationLockIdentity> {
    Err(invalid_data(
        "item icon cache locking is unsupported on this platform",
    ))
}

#[cfg(windows)]
fn lock_generation_file(file: &File) -> Result<()> {
    use std::os::windows::io::AsRawHandle as _;
    use windows_sys::Win32::Storage::FileSystem::{LockFileEx, LOCKFILE_EXCLUSIVE_LOCK};
    use windows_sys::Win32::System::IO::OVERLAPPED;

    let mut overlapped = OVERLAPPED::default();
    // SAFETY: `file` remains alive in `GenerationLock`; this is one synchronous
    // byte-range lock at offset zero and `overlapped` lives through the call.
    if unsafe {
        LockFileEx(
            file.as_raw_handle(),
            LOCKFILE_EXCLUSIVE_LOCK,
            0,
            1,
            0,
            &mut overlapped,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[cfg(unix)]
fn lock_generation_file(file: &File) -> Result<()> {
    use std::os::fd::AsRawFd as _;

    loop {
        // SAFETY: `file` owns this descriptor for the returned lock guard.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } == 0 {
            return Ok(());
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::Interrupted {
            return Err(error.into());
        }
    }
}

#[cfg(not(any(windows, unix)))]
fn lock_generation_file(_file: &File) -> Result<()> {
    Err(invalid_data(
        "item icon cache locking is unsupported on this platform",
    ))
}

fn owned_generation_name<'a>(cache_root: &Path, generation: &'a Path) -> Result<&'a str> {
    if generation.parent() != Some(cache_root) {
        return Err(invalid_data(
            "item icon cache generation escaped its cache root",
        ));
    }
    generation
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| {
            name.strip_prefix(CACHE_DIRECTORY_PREFIX)
                .is_some_and(|key| {
                    key.len() == 64 && key.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
        })
        .ok_or_else(|| invalid_data("item icon cache generation name is invalid"))
}

/// Preserve a broken cache for diagnostics/recovery, then free the exact
/// generation name for an atomic rebuild. Only a direct, non-symlink directory
/// at the generation path derived by this module is eligible; files, links, and
/// every unrelated sibling fail closed and are never moved or deleted.
fn quarantine_incomplete_generation(cache_root: &Path, generation: &Path) -> Result<PathBuf> {
    let name = owned_generation_name(cache_root, generation)?;
    let metadata = std::fs::symlink_metadata(generation)?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(invalid_data(
            "invalid item icon cache path is not an owned generation directory",
        ));
    }

    for _ in 0..128 {
        let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let quarantine = cache_root.join(format!(
            ".{name}.quarantine-{}-{sequence}",
            std::process::id()
        ));
        if std::fs::symlink_metadata(&quarantine).is_ok() {
            continue;
        }
        match std::fs::rename(generation, &quarantine) {
            Ok(()) => return Ok(quarantine),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(invalid_data(
        "could not allocate an item icon cache quarantine path",
    ))
}

struct StagingDirectory {
    path: PathBuf,
    armed: bool,
}

impl StagingDirectory {
    fn create(cache_root: &Path, final_directory: &Path) -> Result<Self> {
        let final_name = final_directory
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid_data("item icon generation has no cache name"))?;
        for _ in 0..128 {
            let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = cache_root.join(format!(
                ".{final_name}.tmp-{}-{sequence}",
                std::process::id()
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path, armed: true }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(invalid_data(
            "could not allocate a unique item icon staging directory",
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// A terminated editor can leave a partially decoded staging directory behind.
/// Once this process owns the exact generation lock, no live writer for that
/// generation can still be using one, so remove only the direct, module-owned
/// `.<generation>.tmp-<pid>-<sequence>` directories before reuse or rebuilding.
fn remove_stale_staging_directories(cache_root: &Path, generation: &Path) -> Result<()> {
    let generation_name = owned_generation_name(cache_root, generation)?;
    let prefix = format!(".{generation_name}.tmp-");
    for entry in std::fs::read_dir(cache_root)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(suffix) = name.strip_prefix(&prefix) else {
            continue;
        };
        let Some((pid, sequence)) = suffix.split_once('-') else {
            continue;
        };
        if pid.is_empty()
            || sequence.is_empty()
            || !pid.bytes().all(|byte| byte.is_ascii_digit())
            || !sequence.bytes().all(|byte| byte.is_ascii_digit())
        {
            continue;
        }

        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
            return Err(invalid_data(
                "stale item icon staging path is not an owned directory",
            ));
        }
        std::fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn invalid_data(message: &'static str) -> TexError {
    TexError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER_ONLY_RGBA8_PNG: [u8; 33] = [
        137, 80, 78, 71, 13, 10, 26, 10, // signature
        0, 0, 0, 13, b'I', b'H', b'D', b'R', // IHDR length + type
        0, 0, 0, 1, 0, 0, 0, 1, // 1x1
        8, 6, 0, 0, 0, // RGBA8, standard compression/filter, no interlace
        0, 0, 0, 0, // deliberately incomplete/invalid CRC; no IDAT/IEND
    ];

    fn write_test_png(path: &Path, width: u32, height: u32, pixel: [u8; 4]) -> Result<()> {
        let pixel_count = usize::try_from(u64::from(width) * u64::from(height)).unwrap();
        let mut rgba = Vec::with_capacity(pixel_count * 4);
        for _ in 0..pixel_count {
            rgba.extend_from_slice(&pixel);
        }
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        image::codecs::png::PngEncoder::new(&mut file)
            .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
            .map_err(|error| TexError::PngEncode(error.to_string()))?;
        file.flush()?;
        Ok(())
    }

    struct FakeSource {
        build_ids: Vec<String>,
        build_index: usize,
        calls: Vec<String>,
        missing: BTreeSet<String>,
        error: Option<(String, &'static str)>,
    }

    impl FakeSource {
        fn stable(build_id: &str) -> Self {
            Self {
                build_ids: vec![build_id.to_string(), build_id.to_string()],
                build_index: 0,
                calls: Vec::new(),
                missing: BTreeSet::new(),
                error: None,
            }
        }
    }

    impl ItemIconSource for FakeSource {
        fn current_build_id(&mut self) -> Result<String> {
            let value = self
                .build_ids
                .get(self.build_index)
                .or_else(|| self.build_ids.last())
                .expect("fake supplies at least one build id")
                .clone();
            self.build_index += 1;
            Ok(value)
        }

        fn write_png(&mut self, asset_path: &str, output: &Path) -> Result<()> {
            self.calls.push(asset_path.to_string());
            if self.missing.contains(asset_path) {
                return Err(TexError::AssetNotFound(asset_path.to_string()));
            }
            if self
                .error
                .as_ref()
                .is_some_and(|(path, _)| path == asset_path)
            {
                let (_, format) = self.error.as_ref().expect("checked above");
                return Err(TexError::UnsupportedFormat((*format).to_string()));
            }
            write_test_png(output, 1, 1, [0x12, 0x34, 0x56, 0xff])
        }
    }

    fn specs() -> Vec<ItemIconSpec> {
        vec![
            ItemIconSpec::new("ItMi_One", "Shared"),
            ItemIconSpec::new("ItMi_Two", "Shared"),
            ItemIconSpec::new("ItFo_Three", "Food"),
        ]
    }

    #[test]
    fn prepares_every_item_deduplicates_icons_and_uses_exact_case_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let mut source = FakeSource::stable("build-a");
        source
            .missing
            .insert("/Game/UI/Textures/ItemIcons/T_ItemIcon_Food".to_string());

        let manifest_path =
            prepare_item_icon_cache_with_source(temp.path(), &specs(), &mut source).unwrap();
        assert_eq!(
            source.calls,
            vec![
                "/Game/UI/Textures/ItemIcons/T_ItemIcon_Food",
                "/Game/UI/Textures/ItemIcons/T_Itemicon_Food",
                "/Game/UI/Textures/ItemIcons/T_ItemIcon_Shared",
            ]
        );

        let manifest: ItemIconManifest =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        assert_eq!(manifest.schema, ITEM_ICON_CACHE_SCHEMA);
        assert_eq!(manifest.build_id, "build-a");
        assert_eq!(manifest.item_count, 3);
        assert_eq!(manifest.items.len(), 3);
        assert_eq!(manifest.files.len(), 2);
        assert!(manifest.files.values().all(|seal| {
            seal.width == 1
                && seal.height == 1
                && seal.decoded_byte_length == 4
                && seal.png_blake3.len() == 64
                && seal.rgba_blake3.len() == 64
        }));
        assert_eq!(manifest.items["ItMi_One"], manifest.items["ItMi_Two"]);
        assert_ne!(manifest.items["ItMi_One"], manifest.items["ItFo_Three"]);
        for relative in manifest.items.values() {
            assert!(manifest_path.parent().unwrap().join(relative).is_file());
        }
    }

    #[test]
    fn non_missing_error_does_not_try_case_fallback_or_publish() {
        let temp = tempfile::tempdir().unwrap();
        let mut source = FakeSource::stable("build-a");
        source.error = Some((
            "/Game/UI/Textures/ItemIcons/T_ItemIcon_Food".to_string(),
            "PF_BOGUS",
        ));

        let error =
            prepare_item_icon_cache_with_source(temp.path(), &specs(), &mut source).unwrap_err();
        assert!(matches!(error, TexError::UnsupportedFormat(_)));
        assert_eq!(
            source.calls,
            vec!["/Game/UI/Textures/ItemIcons/T_ItemIcon_Food"]
        );
        assert!(published_generations(temp.path()).is_empty());
    }

    #[test]
    fn generation_drift_discards_staging_and_never_publishes() {
        let temp = tempfile::tempdir().unwrap();
        let mut source = FakeSource::stable("build-before");
        source.build_ids[1] = "build-after".to_string();

        let error =
            prepare_item_icon_cache_with_source(temp.path(), &specs(), &mut source).unwrap_err();
        assert!(matches!(error, TexError::GenerationChanged));
        assert!(published_generations(temp.path()).is_empty());
        assert!(std::fs::read_dir(temp.path()).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".tmp-")
        }));
    }

    #[test]
    fn complete_generation_is_reused_without_extracting_again() {
        let temp = tempfile::tempdir().unwrap();
        let mut first = FakeSource::stable("build-a");
        let first_path =
            prepare_item_icon_cache_with_source(temp.path(), &specs(), &mut first).unwrap();
        assert!(!first.calls.is_empty());

        let generation_name = first_path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy();
        let stale = temp.path().join(format!(".{generation_name}.tmp-999-7"));
        std::fs::create_dir_all(stale.join("images")).unwrap();
        std::fs::write(stale.join("images/stale.png"), b"partial").unwrap();
        let unrelated = temp.path().join(".item-icons-v1-unrelated.tmp-999-7");
        std::fs::create_dir(&unrelated).unwrap();

        let mut second = FakeSource::stable("build-a");
        second.error = Some((
            "/Game/UI/Textures/ItemIcons/T_ItemIcon_Food".to_string(),
            "must-not-run",
        ));
        let reused =
            prepare_item_icon_cache_with_source(temp.path(), &specs(), &mut second).unwrap();
        assert_eq!(reused, first_path);
        assert!(second.calls.is_empty());
        assert_eq!(second.build_index, 2);
        assert!(!stale.exists());
        assert!(unrelated.exists());
    }

    #[test]
    fn incomplete_existing_generation_is_quarantined_then_rebuilt() {
        let temp = tempfile::tempdir().unwrap();
        let catalog = PreparedCatalog::from_specs(&specs()).unwrap();
        let directory = generation_directory(temp.path(), "build-a", &catalog.digest);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join(MANIFEST_FILE_NAME), b"{}").unwrap();
        let mut source = FakeSource::stable("build-a");

        let repaired =
            prepare_item_icon_cache_with_source(temp.path(), &specs(), &mut source).unwrap();
        assert_eq!(repaired, directory.join(MANIFEST_FILE_NAME));
        assert!(!source.calls.is_empty());
        let quarantine = std::fs::read_dir(temp.path())
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().contains(".quarantine-"))
            })
            .expect("invalid generation is preserved in quarantine");
        assert_eq!(
            std::fs::read(quarantine.join(MANIFEST_FILE_NAME)).unwrap(),
            b"{}"
        );
    }

    #[test]
    fn header_only_png_is_not_reused() {
        let temp = tempfile::tempdir().unwrap();
        let mut first = FakeSource::stable("build-a");
        let manifest_path =
            prepare_item_icon_cache_with_source(temp.path(), &specs(), &mut first).unwrap();
        let manifest: ItemIconManifest =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        let damaged = manifest_path
            .parent()
            .unwrap()
            .join(&manifest.items["ItMi_One"]);
        std::fs::write(damaged, HEADER_ONLY_RGBA8_PNG).unwrap();

        let mut second = FakeSource::stable("build-a");
        let repaired =
            prepare_item_icon_cache_with_source(temp.path(), &specs(), &mut second).unwrap();
        assert_eq!(repaired, manifest_path);
        assert!(!second.calls.is_empty(), "damaged PNG must force a rebuild");
        let repaired_manifest: ItemIconManifest =
            serde_json::from_slice(&std::fs::read(&repaired).unwrap()).unwrap();
        let repaired_png = repaired
            .parent()
            .unwrap()
            .join(&repaired_manifest.items["ItMi_One"]);
        let repaired_len = std::fs::metadata(&repaired_png).unwrap().len();
        assert!(inspect_cached_png(&repaired_png, repaired_len)
            .unwrap()
            .is_some());
    }

    #[test]
    fn valid_png_with_different_pixels_fails_the_manifest_hash_and_is_repaired() {
        let temp = tempfile::tempdir().unwrap();
        let mut first = FakeSource::stable("build-a");
        let manifest_path =
            prepare_item_icon_cache_with_source(temp.path(), &specs(), &mut first).unwrap();
        let manifest: ItemIconManifest =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        let damaged = manifest_path
            .parent()
            .unwrap()
            .join(&manifest.items["ItMi_One"]);
        let original_len = std::fs::metadata(&damaged).unwrap().len();
        let replacement = temp.path().join("different-valid.png");
        write_test_png(&replacement, 1, 1, [0xaa, 0xbb, 0xcc, 0xff]).unwrap();
        assert_eq!(std::fs::metadata(&replacement).unwrap().len(), original_len);
        std::fs::copy(&replacement, &damaged).unwrap();

        let mut second = FakeSource::stable("build-a");
        let repaired =
            prepare_item_icon_cache_with_source(temp.path(), &specs(), &mut second).unwrap();
        assert_eq!(repaired, manifest_path);
        assert!(
            !second.calls.is_empty(),
            "valid but changed bytes must rebuild"
        );
    }

    #[test]
    fn png_validation_enforces_dimensions_file_size_and_cumulative_budgets() {
        let temp = tempfile::tempdir().unwrap();
        let too_wide = temp.path().join("too-wide.png");
        write_test_png(&too_wide, MAX_ICON_DIMENSION + 1, 1, [0, 0, 0, 0xff]).unwrap();
        let too_wide_len = std::fs::metadata(&too_wide).unwrap().len();
        assert!(inspect_cached_png(&too_wide, too_wide_len)
            .unwrap()
            .is_none());

        let too_large = temp.path().join("too-large.png");
        let file = File::create(&too_large).unwrap();
        file.set_len(MAX_ICON_PNG_BYTES + 1).unwrap();
        drop(file);
        assert!(inspect_cached_png(&too_large, MAX_ICON_PNG_BYTES + 1)
            .unwrap()
            .is_none());

        let mut budget = CacheBudget::default();
        let exact_limit = ItemIconFileSeal {
            width: 1,
            height: 1,
            byte_length: MAX_CACHE_PNG_BYTES,
            decoded_byte_length: MAX_CACHE_DECODED_BYTES,
            png_blake3: "0".repeat(64),
            rgba_blake3: "0".repeat(64),
        };
        assert!(budget.admit(&exact_limit));
        let one_more = ItemIconFileSeal {
            byte_length: 1,
            decoded_byte_length: 1,
            ..exact_limit
        };
        assert!(!budget.admit(&one_more));
    }

    #[test]
    fn generation_lock_serializes_corrupt_cache_repair() {
        use std::sync::mpsc;
        use std::time::Duration;

        let temp = tempfile::tempdir().unwrap();
        let catalog = PreparedCatalog::from_specs(&specs()).unwrap();
        let generation = generation_directory(temp.path(), "build-a", &catalog.digest);
        let first = GenerationLock::acquire(temp.path(), &generation).unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let root = temp.path().to_path_buf();
        let generation_for_thread = generation.clone();
        let waiter = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            let _second = GenerationLock::acquire(&root, &generation_for_thread).unwrap();
            acquired_tx.send(()).unwrap();
        });
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(acquired_rx
            .recv_timeout(Duration::from_millis(100))
            .is_err());
        drop(first);
        acquired_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        waiter.join().unwrap();
    }

    #[test]
    fn catalog_validation_is_bounded_and_rejects_unsafe_icon_suffixes() {
        let bad = vec![ItemIconSpec::new("ItMi_Bad", "../Other")];
        let error = PreparedCatalog::from_specs(&bad).err().unwrap();
        assert!(error.to_string().contains("invalid icon id"));

        let duplicate = vec![
            ItemIconSpec::new("ItMi_Same", "A"),
            ItemIconSpec::new("ItMi_Same", "B"),
        ];
        let error = PreparedCatalog::from_specs(&duplicate).err().unwrap();
        assert!(error.to_string().contains("duplicate item id"));
    }

    fn published_generations(root: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(root)
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(CACHE_DIRECTORY_PREFIX))
            })
            .collect()
    }
}
