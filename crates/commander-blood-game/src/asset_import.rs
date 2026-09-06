//! One-time conversion from the packed DOS installation to a loose asset store.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_formats::archive::{BloodArchive, BloodResourceName};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::game::GameVariant;

pub(crate) const ASSET_MANIFEST_FILENAME: &str = "manifest.json";
pub(crate) const IMPORTED_ASSET_DIRECTORY_NAME: &str = "assets-v1";
pub(crate) const IMPORTED_ASSET_SCHEMA_VERSION: u32 = 1;
pub(crate) const RESOURCE_DIRECTORY_NAME: &str = "resources";

const COMPANION_DIRECTORY_NAME: &str = "companions";
const ORIGINAL_ARCHIVE_FILENAME: &str = "BLOOD.DAT";
const TEMPORARY_IMPORT_INFIX: &str = "import";
const REPLACED_IMPORT_INFIX: &str = "replaced";
const SHA256_BYTE_COUNT: usize = 32;
const HASH_READ_BUFFER_BYTES: usize = 65536;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ImportedAssetOrigin {
    Archive,
    LooseFile,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ImportedMediaKind {
    HnmVideo,
    SndBank,
    VocAudio,
    NativeData,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ImportedAssetEntry {
    pub(crate) resource_name: String,
    pub(crate) path: String,
    pub(crate) byte_count: u64,
    pub(crate) sha256: String,
    pub(crate) origin: ImportedAssetOrigin,
    pub(crate) media_kind: ImportedMediaKind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ImportedCompanionEntry {
    pub(crate) filename: String,
    pub(crate) path: String,
    pub(crate) byte_count: u64,
    pub(crate) sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ImportedAssetManifest {
    pub(crate) schema_version: u32,
    // Schema-one manifests predate sequel support and contain Commander inputs.
    #[serde(default)]
    pub(crate) game: GameVariant,
    pub(crate) source_archive_sha256: String,
    pub(crate) source_archive_byte_count: u64,
    pub(crate) source_archive_entry_count: usize,
    pub(crate) resources: Vec<ImportedAssetEntry>,
    pub(crate) companions: Vec<ImportedCompanionEntry>,
}

impl ImportedAssetManifest {
    pub(crate) fn load(root: &Path) -> Result<Self> {
        let manifest = Self::read(root)?;
        manifest.validate(root, false)?;
        Ok(manifest)
    }

    fn read(root: &Path) -> Result<Self> {
        let manifest_path = root.join(ASSET_MANIFEST_FILENAME);
        let encoded = std::fs::read(&manifest_path).with_context(|| {
            format!(
                "reading imported asset manifest {}",
                manifest_path.display()
            )
        })?;
        let manifest: Self = serde_json::from_slice(&encoded).with_context(|| {
            format!(
                "decoding imported asset manifest {}",
                manifest_path.display()
            )
        })?;
        Ok(manifest)
    }

    pub(crate) fn validate(&self, root: &Path, verify_hashes: bool) -> Result<()> {
        if self.schema_version != IMPORTED_ASSET_SCHEMA_VERSION {
            bail!(
                "unsupported imported asset schema {}; expected {}",
                self.schema_version,
                IMPORTED_ASSET_SCHEMA_VERSION
            );
        }
        if self.source_archive_sha256.len() != SHA256_BYTE_COUNT * 2
            || !self
                .source_archive_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            bail!("imported asset manifest has an invalid source archive SHA-256");
        }

        let mut resource_names = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for entry in &self.resources {
            BloodResourceName::new(entry.resource_name.as_bytes()).with_context(|| {
                format!("invalid imported resource name {}", entry.resource_name)
            })?;
            if !resource_names.insert(entry.resource_name.as_str()) {
                bail!("duplicate imported resource name {}", entry.resource_name);
            }
            validate_entry(
                root,
                &entry.path,
                entry.byte_count,
                &entry.sha256,
                verify_hashes,
                &mut paths,
            )?;
        }

        let mut companion_names = BTreeSet::new();
        for entry in &self.companions {
            if !companion_names.insert(entry.filename.as_str()) {
                bail!("duplicate imported companion {}", entry.filename);
            }
            validate_entry(
                root,
                &entry.path,
                entry.byte_count,
                &entry.sha256,
                verify_hashes,
                &mut paths,
            )?;
        }
        for required in self.game.required_companions() {
            if !companion_names
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(required))
            {
                bail!("imported asset manifest is missing companion {required}");
            }
        }
        let other_game = match self.game {
            GameVariant::CommanderBlood => GameVariant::BigBugBang,
            GameVariant::BigBugBang => GameVariant::CommanderBlood,
        };
        if companion_names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(other_game.executable_filename()))
        {
            bail!("imported asset manifest mixes native executables from both games");
        }
        Ok(())
    }

    pub(crate) fn resource_names(&self) -> Result<Vec<BloodResourceName>> {
        self.resources
            .iter()
            .map(|entry| {
                BloodResourceName::new(entry.resource_name.as_bytes()).with_context(|| {
                    format!("invalid imported resource name {}", entry.resource_name)
                })
            })
            .collect()
    }

    pub(crate) fn companion_path(&self, root: &Path, filename: &str) -> Result<PathBuf> {
        let entry = self
            .companions
            .iter()
            .find(|entry| entry.filename.eq_ignore_ascii_case(filename))
            .with_context(|| format!("imported asset manifest has no companion {filename}"))?;
        checked_relative_path(root, &entry.path)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AssetImportOutcome {
    Imported { resource_count: usize },
    Reused { resource_count: usize },
}

pub(crate) fn import_original_assets(
    source_root: &Path,
    destination_root: &Path,
) -> Result<AssetImportOutcome> {
    let game = detect_source_game(source_root)?;
    let source_canonical = source_root
        .canonicalize()
        .context("resolving source installation")?;
    let destination_canonical = resolve_destination(destination_root)?;
    if source_canonical.starts_with(&destination_canonical)
        || destination_canonical.starts_with(&source_canonical)
    {
        bail!("asset import source and destination must not overlap");
    }
    let source_executable = case_insensitive_child(source_root, game.executable_filename())?
        .context("detected game executable disappeared")?;
    let source_executable_hash =
        sha256_hex(&std::fs::read(&source_executable).with_context(|| {
            format!("reading source executable {}", source_executable.display())
        })?);
    // Check identity even if files in a different game's cache are damaged.
    // A validation error must not turn into permission to replace that cache.
    if destination_root.join(ASSET_MANIFEST_FILENAME).exists() {
        let manifest = ImportedAssetManifest::read(destination_root)?;
        if manifest.game != game {
            bail!(
                "asset cache {} belongs to {}, but the source is {}; choose a separate destination",
                destination_root.display(),
                manifest.game.title(),
                game.title()
            );
        }
        let executable = manifest
            .companions
            .iter()
            .find(|entry| {
                entry
                    .filename
                    .eq_ignore_ascii_case(game.executable_filename())
            })
            .context("imported game has no executable companion")?;
        if executable.sha256 != source_executable_hash {
            bail!(
                "asset cache {} contains a different {} executable build; choose a separate destination",
                destination_root.display(),
                game.title()
            );
        }
        if let Some(archive) = case_insensitive_child(source_root, ORIGINAL_ARCHIVE_FILENAME)? {
            if sha256_file(&archive)? != manifest.source_archive_sha256 {
                bail!(
                    "asset cache {} contains a different source archive; choose a separate destination",
                    destination_root.display()
                );
            }
        }
        for entry in manifest
            .resources
            .iter()
            .filter(|entry| entry.origin == ImportedAssetOrigin::LooseFile)
        {
            if let Some(source) = case_insensitive_child(source_root, &entry.resource_name)? {
                if sha256_file(&source)? != entry.sha256 {
                    bail!(
                        "asset cache {} differs from source resource {}; choose a separate destination",
                        destination_root.display(),
                        entry.resource_name
                    );
                }
            }
        }
        if manifest.validate(destination_root, false).is_ok() {
            return Ok(AssetImportOutcome::Reused {
                resource_count: manifest.resources.len(),
            });
        }
    }
    if !source_root.is_dir() {
        bail!(
            "game import source is not a directory: {}",
            source_root.display()
        );
    }

    let archive_path = case_insensitive_child(source_root, ORIGINAL_ARCHIVE_FILENAME)?
        .with_context(|| {
            format!(
                "game import source has no {ORIGINAL_ARCHIVE_FILENAME}: {}",
                source_root.display()
            )
        })?;
    let archive_bytes = std::fs::read(&archive_path)
        .with_context(|| format!("reading original archive {}", archive_path.display()))?;
    let source_archive_byte_count = u64::try_from(archive_bytes.len())
        .context("original archive length exceeds the manifest range")?;
    let source_archive_sha256 = sha256_hex(&archive_bytes);
    let archive = BloodArchive::decode(archive_bytes.into_boxed_slice())
        .with_context(|| format!("decoding original archive {}", archive_path.display()))?;
    let source_archive_entry_count = archive.entries().len();

    let temporary_root = temporary_sibling(destination_root, TEMPORARY_IMPORT_INFIX)?;
    if temporary_root.exists() {
        std::fs::remove_dir_all(&temporary_root).with_context(|| {
            format!(
                "removing interrupted asset import {}",
                temporary_root.display()
            )
        })?;
    }
    std::fs::create_dir_all(temporary_root.join(RESOURCE_DIRECTORY_NAME)).with_context(|| {
        format!(
            "creating imported resource root {}",
            temporary_root.display()
        )
    })?;
    std::fs::create_dir_all(temporary_root.join(COMPANION_DIRECTORY_NAME)).with_context(|| {
        format!(
            "creating imported companion root {}",
            temporary_root.display()
        )
    })?;

    let import_result = build_import(
        game,
        source_root,
        &temporary_root,
        &archive,
        source_archive_sha256,
        source_archive_byte_count,
        source_archive_entry_count,
    );
    let manifest = match import_result {
        Ok(manifest) => manifest,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&temporary_root);
            return Err(error);
        }
    };
    replace_directory(&temporary_root, destination_root)?;
    Ok(AssetImportOutcome::Imported {
        resource_count: manifest.resources.len(),
    })
}

fn build_import(
    game: GameVariant,
    source_root: &Path,
    temporary_root: &Path,
    archive: &BloodArchive,
    source_archive_sha256: String,
    source_archive_byte_count: u64,
    source_archive_entry_count: usize,
) -> Result<ImportedAssetManifest> {
    let mut imported_keys = BTreeSet::new();
    let mut resources = Vec::new();
    for entry in archive.entries() {
        let key = entry.name().archive_lookup_key();
        if !imported_keys.insert(key.clone()) {
            continue;
        }
        let resource_name = std::str::from_utf8(&key)
            .expect("validated archive resource name")
            .to_owned();
        let payload = archive
            .member(entry.name())
            .expect("archive entry must resolve its validated payload");
        resources.push(write_resource(
            temporary_root,
            &resource_name,
            payload,
            ImportedAssetOrigin::Archive,
        )?);
    }

    for directory_entry in std::fs::read_dir(source_root)
        .with_context(|| format!("reading original data root {}", source_root.display()))?
    {
        let directory_entry = directory_entry
            .with_context(|| format!("reading original data root {}", source_root.display()))?;
        let file_type = directory_entry.file_type().with_context(|| {
            format!(
                "reading original file type {}",
                directory_entry.path().display()
            )
        })?;
        if !file_type.is_file() {
            continue;
        }
        let Some(filename) = directory_entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if filename.eq_ignore_ascii_case(ORIGINAL_ARCHIVE_FILENAME) {
            continue;
        }
        let Ok(name) = BloodResourceName::new(filename.as_bytes()) else {
            continue;
        };
        let key = name.archive_lookup_key();
        if !imported_keys.insert(key.clone()) {
            continue;
        }
        let bytes = std::fs::read(directory_entry.path()).with_context(|| {
            format!(
                "reading loose original resource {}",
                directory_entry.path().display()
            )
        })?;
        let resource_name = std::str::from_utf8(&key)
            .expect("validated loose resource name")
            .to_owned();
        resources.push(write_resource(
            temporary_root,
            &resource_name,
            &bytes,
            ImportedAssetOrigin::LooseFile,
        )?);
    }
    resources.sort_by(|left, right| left.resource_name.cmp(&right.resource_name));

    let mut companions = Vec::with_capacity(game.required_companions().len());
    for &filename in game.required_companions() {
        let source = case_insensitive_child(source_root, filename)?.with_context(|| {
            format!(
                "game import source is missing required companion {filename}: {}",
                source_root.display()
            )
        })?;
        let bytes = std::fs::read(&source)
            .with_context(|| format!("reading original companion {}", source.display()))?;
        let relative = format!("{COMPANION_DIRECTORY_NAME}/{filename}");
        let destination = checked_relative_path(temporary_root, &relative)?;
        std::fs::write(&destination, &bytes)
            .with_context(|| format!("writing imported companion {}", destination.display()))?;
        companions.push(ImportedCompanionEntry {
            filename: filename.to_owned(),
            path: relative,
            byte_count: u64::try_from(bytes.len())
                .context("companion length exceeds the manifest range")?,
            sha256: sha256_hex(&bytes),
        });
    }

    let manifest = ImportedAssetManifest {
        schema_version: IMPORTED_ASSET_SCHEMA_VERSION,
        game,
        source_archive_sha256,
        source_archive_byte_count,
        source_archive_entry_count,
        resources,
        companions,
    };
    manifest.validate(temporary_root, true)?;
    let manifest_path = temporary_root.join(ASSET_MANIFEST_FILENAME);
    let mut encoded =
        serde_json::to_vec_pretty(&manifest).context("encoding imported asset manifest")?;
    encoded.push(b'\n');
    std::fs::write(&manifest_path, encoded).with_context(|| {
        format!(
            "writing imported asset manifest {}",
            manifest_path.display()
        )
    })?;
    Ok(manifest)
}

/// Identify a source by its main executable name, never by the shared BLOOD.DAT name.
/// This identifies the game, not whether the runtime supports that executable revision.
pub(crate) fn detect_source_game(root: &Path) -> Result<GameVariant> {
    if !root.is_dir() {
        bail!("game import source is not a directory: {}", root.display());
    }
    let commander =
        case_insensitive_child(root, GameVariant::CommanderBlood.executable_filename())?.is_some();
    let sequel =
        case_insensitive_child(root, GameVariant::BigBugBang.executable_filename())?.is_some();
    match (commander, sequel) {
        (true, false) => Ok(GameVariant::CommanderBlood),
        (false, true) => Ok(GameVariant::BigBugBang),
        (true, true) => bail!(
            "game source {} contains both main executables; use separate game directories",
            root.display()
        ),
        (false, false) => bail!(
            "game source {} has neither BLOODPRG.EXE nor BLOOD2PG.EXE",
            root.display()
        ),
    }
}

fn resolve_destination(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return path
            .canonicalize()
            .with_context(|| format!("resolving destination {}", path.display()));
    }
    let name = path
        .file_name()
        .context("asset destination must name a directory")?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(resolve_destination(parent)?.join(name))
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("opening source fingerprint input {}", path.display()))?;
    let mut hash = Sha256::new();
    let mut buffer = [0; HASH_READ_BUFFER_BYTES];
    loop {
        let count = file
            .read(&mut buffer)
            .with_context(|| format!("hashing source input {}", path.display()))?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn write_resource(
    root: &Path,
    resource_name: &str,
    bytes: &[u8],
    origin: ImportedAssetOrigin,
) -> Result<ImportedAssetEntry> {
    let normalized_name = resource_name.replace('\\', "/");
    let relative = format!("{RESOURCE_DIRECTORY_NAME}/{normalized_name}");
    let destination = checked_relative_path(root, &relative)?;
    let parent = destination
        .parent()
        .context("imported resource path has no parent")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("creating imported resource directory {}", parent.display()))?;
    std::fs::write(&destination, bytes)
        .with_context(|| format!("writing imported resource {}", destination.display()))?;
    Ok(ImportedAssetEntry {
        resource_name: resource_name.to_owned(),
        path: relative,
        byte_count: u64::try_from(bytes.len())
            .context("resource length exceeds the manifest range")?,
        sha256: sha256_hex(bytes),
        origin,
        media_kind: media_kind(resource_name),
    })
}

fn validate_entry<'a>(
    root: &Path,
    relative: &'a str,
    expected_byte_count: u64,
    expected_sha256: &str,
    verify_hashes: bool,
    paths: &mut BTreeSet<&'a str>,
) -> Result<()> {
    if !paths.insert(relative) {
        bail!("duplicate imported asset path {relative}");
    }
    if expected_sha256.len() != SHA256_BYTE_COUNT * 2
        || !expected_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("imported asset {relative} has an invalid SHA-256");
    }
    let path = checked_relative_path(root, relative)?;
    let actual_byte_count = std::fs::metadata(&path)
        .with_context(|| format!("reading imported asset metadata {}", path.display()))?
        .len();
    if actual_byte_count != expected_byte_count {
        bail!(
            "imported asset {} has {} bytes; expected {}",
            path.display(),
            actual_byte_count,
            expected_byte_count
        );
    }
    if verify_hashes {
        let bytes = std::fs::read(&path)
            .with_context(|| format!("reading imported asset {}", path.display()))?;
        let actual_sha256 = sha256_hex(&bytes);
        if actual_sha256 != expected_sha256 {
            bail!(
                "imported asset {} has SHA-256 {}; expected {}",
                path.display(),
                actual_sha256,
                expected_sha256
            );
        }
    }
    Ok(())
}

pub(crate) fn checked_relative_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("imported asset path is not a safe relative path: {relative}");
    }
    Ok(root.join(relative_path))
}

fn media_kind(resource_name: &str) -> ImportedMediaKind {
    let extension = Path::new(resource_name)
        .extension()
        .and_then(|extension| extension.to_str());
    match extension {
        Some(extension) if extension.eq_ignore_ascii_case("HNM") => ImportedMediaKind::HnmVideo,
        Some(extension) if extension.eq_ignore_ascii_case("SND") => ImportedMediaKind::SndBank,
        Some(extension) if extension.eq_ignore_ascii_case("VOC") => ImportedMediaKind::VocAudio,
        _ => ImportedMediaKind::NativeData,
    }
}

fn case_insensitive_child(root: &Path, filename: &str) -> Result<Option<PathBuf>> {
    let exact = root.join(filename);
    if exact.is_file() {
        return Ok(Some(exact));
    }
    let mut matched: Option<PathBuf> = None;
    for entry in std::fs::read_dir(root)
        .with_context(|| format!("reading original data root {}", root.display()))?
    {
        let entry =
            entry.with_context(|| format!("reading original data root {}", root.display()))?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        if entry
            .file_name()
            .to_str()
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(filename))
        {
            if let Some(previous) = matched {
                bail!(
                    "ambiguous original companion {filename}: {} and {}",
                    previous.display(),
                    entry.path().display()
                );
            }
            matched = Some(entry.path());
        }
    }
    Ok(matched)
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(SHA256_BYTE_COUNT * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

pub(crate) fn temporary_sibling(destination: &Path, infix: &str) -> Result<PathBuf> {
    let parent = destination
        .parent()
        .with_context(|| format!("asset destination has no parent: {}", destination.display()))?;
    let filename = destination.file_name().with_context(|| {
        format!(
            "asset destination has no filename: {}",
            destination.display()
        )
    })?;
    Ok(parent.join(format!(
        ".{}-{infix}-{}",
        filename.to_string_lossy(),
        std::process::id()
    )))
}

pub(crate) fn replace_directory(temporary: &Path, destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .with_context(|| format!("asset destination has no parent: {}", destination.display()))?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("creating asset cache parent {}", parent.display()))?;
    if !destination.exists() {
        return std::fs::rename(temporary, destination)
            .with_context(|| format!("installing imported asset store {}", destination.display()));
    }

    let replaced = temporary_sibling(destination, REPLACED_IMPORT_INFIX)?;
    if replaced.exists() {
        std::fs::remove_dir_all(&replaced)
            .with_context(|| format!("removing stale asset backup {}", replaced.display()))?;
    }
    std::fs::rename(destination, &replaced)
        .with_context(|| format!("backing up previous asset store {}", destination.display()))?;
    if let Err(error) = std::fs::rename(temporary, destination) {
        let _ = std::fs::rename(&replaced, destination);
        return Err(error)
            .with_context(|| format!("installing imported asset store {}", destination.display()));
    }
    std::fs::remove_dir_all(&replaced)
        .with_context(|| format!("removing previous asset store {}", replaced.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    const DIRECTORY_HEADER_SIZE: usize = 2;
    const DIRECTORY_ENTRY_SIZE: usize = 25;
    const RESOURCE_NAME_FIELD_SIZE: usize = 16;
    const BYTE_COUNT_FIELD_OFFSET: usize = RESOURCE_NAME_FIELD_SIZE;
    const FILE_POSITION_FIELD_OFFSET: usize = BYTE_COUNT_FIELD_OFFSET + 4;
    const FILE_POSITION_FIELD_SIZE: usize = 4;
    static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(u64::MIN);

    struct TemporaryRoot(PathBuf);

    impl TemporaryRoot {
        fn create(label: &str) -> Self {
            let sequence = TEMPORARY_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "commander-blood-{label}-{}-{sequence}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TemporaryRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn archive_bytes(records: &[(&str, &[u8])]) -> Vec<u8> {
        let terminator_size = 1;
        let directory_size =
            DIRECTORY_HEADER_SIZE + records.len() * DIRECTORY_ENTRY_SIZE + terminator_size;
        let payload_size: usize = records.iter().map(|(_name, payload)| payload.len()).sum();
        let mut data = vec![u8::MIN; directory_size + payload_size];
        data[..DIRECTORY_HEADER_SIZE]
            .copy_from_slice(&u16::try_from(records.len()).unwrap().to_le_bytes());
        let mut payload_position = directory_size;
        for (entry, (name, payload)) in records.iter().enumerate() {
            let cursor = DIRECTORY_HEADER_SIZE + entry * DIRECTORY_ENTRY_SIZE;
            data[cursor..cursor + name.len()].copy_from_slice(name.as_bytes());
            data[cursor + BYTE_COUNT_FIELD_OFFSET..cursor + FILE_POSITION_FIELD_OFFSET]
                .copy_from_slice(&i32::try_from(payload.len()).unwrap().to_le_bytes());
            data[cursor + FILE_POSITION_FIELD_OFFSET
                ..cursor + FILE_POSITION_FIELD_OFFSET + FILE_POSITION_FIELD_SIZE]
                .copy_from_slice(&i32::try_from(payload_position).unwrap().to_le_bytes());
            data[payload_position..payload_position + payload.len()].copy_from_slice(payload);
            payload_position += payload.len();
        }
        data
    }

    fn write_source(source: &Path) {
        write_source_for_game(source, GameVariant::CommanderBlood);
    }

    fn write_source_for_game(source: &Path, game: GameVariant) {
        let archive = archive_bytes(&[
            (r"SQ\INTRO.HNM", b"video"),
            (r"SN\TB.SND", b"sound"),
            ("DUP.DAT", b"archive wins"),
        ]);
        std::fs::write(source.join(ORIGINAL_ARCHIVE_FILENAME), archive).unwrap();
        for filename in game.required_companions() {
            std::fs::write(source.join(filename), filename.as_bytes()).unwrap();
        }
        std::fs::write(source.join("SCRIPT1.COD"), b"loose script").unwrap();
        std::fs::write(source.join("dup.dat"), b"loose duplicate").unwrap();
    }

    #[test]
    fn imports_archive_and_loose_resources_into_one_manifested_tree() {
        let source = TemporaryRoot::create("asset-source");
        let cache_parent = TemporaryRoot::create("asset-cache-parent");
        let destination = cache_parent.0.join(IMPORTED_ASSET_DIRECTORY_NAME);
        write_source(&source.0);

        let outcome = import_original_assets(&source.0, &destination).unwrap();
        assert_eq!(outcome, AssetImportOutcome::Imported { resource_count: 9 });
        assert!(!destination.join(ORIGINAL_ARCHIVE_FILENAME).exists());

        let manifest = ImportedAssetManifest::load(&destination).unwrap();
        manifest.validate(&destination, true).unwrap();
        assert_eq!(manifest.source_archive_entry_count, 3);
        assert_eq!(manifest.resources.len(), 9);
        assert_eq!(
            manifest.companions.len(),
            GameVariant::CommanderBlood.required_companions().len()
        );
        assert_eq!(
            std::fs::read(destination.join("resources/DUP.DAT")).unwrap(),
            b"archive wins"
        );
        assert_eq!(
            std::fs::read(destination.join("resources/SQ/INTRO.HNM")).unwrap(),
            b"video"
        );
        assert_eq!(
            std::fs::read(destination.join("resources/SCRIPT1.COD")).unwrap(),
            b"loose script"
        );
    }

    #[test]
    fn reuses_a_complete_store_without_reopening_the_source_archive() {
        let source = TemporaryRoot::create("asset-reuse-source");
        let cache_parent = TemporaryRoot::create("asset-reuse-cache-parent");
        let destination = cache_parent.0.join(IMPORTED_ASSET_DIRECTORY_NAME);
        write_source(&source.0);
        import_original_assets(&source.0, &destination).unwrap();
        std::fs::remove_file(source.0.join(ORIGINAL_ARCHIVE_FILENAME)).unwrap();

        assert_eq!(
            import_original_assets(&source.0, &destination).unwrap(),
            AssetImportOutcome::Reused { resource_count: 9 }
        );
    }

    #[test]
    fn rejects_a_manifest_when_an_imported_file_changes_size() {
        let source = TemporaryRoot::create("asset-corrupt-source");
        let cache_parent = TemporaryRoot::create("asset-corrupt-cache-parent");
        let destination = cache_parent.0.join(IMPORTED_ASSET_DIRECTORY_NAME);
        write_source(&source.0);
        import_original_assets(&source.0, &destination).unwrap();
        std::fs::write(destination.join("resources/SQ/INTRO.HNM"), b"truncated").unwrap();

        assert!(ImportedAssetManifest::load(&destination).is_err());
    }

    #[test]
    fn sequel_import_uses_its_own_executable_and_title_without_invented_saves() {
        let source = TemporaryRoot::create("sequel-source");
        let cache = TemporaryRoot::create("sequel-cache");
        let destination = cache.0.join(IMPORTED_ASSET_DIRECTORY_NAME);
        write_source_for_game(&source.0, GameVariant::BigBugBang);
        import_original_assets(&source.0, &destination).unwrap();
        let manifest = ImportedAssetManifest::load(&destination).unwrap();
        assert_eq!(manifest.game, GameVariant::BigBugBang);
        assert_eq!(manifest.companions.len(), 4);
        assert!(
            manifest
                .companion_path(&destination, "BLOOD2PG.EXE")
                .unwrap()
                .is_file()
        );
        assert!(
            manifest
                .companion_path(&destination, "BLOOD2.LBM")
                .unwrap()
                .is_file()
        );
        assert!(!destination.join("companions/BLOODPRG.EXE").exists());
        assert!(!destination.join("resources/BLOOD.SAV").exists());
        assert!(!destination.join("resources/SCRIPT1.BAS").exists());
        manifest.validate(&destination, true).unwrap();
        let error = crate::runtime::OriginalGameDataPaths::from_root(&destination).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unrecognized Big Bug Bang executable"),
            "{error:#}"
        );
        assert!(!destination.join("media-v1").exists());
    }

    #[test]
    fn requesting_other_game_preserves_existing_cache_in_both_directions() {
        for (old, new) in [
            (GameVariant::CommanderBlood, GameVariant::BigBugBang),
            (GameVariant::BigBugBang, GameVariant::CommanderBlood),
        ] {
            let source = TemporaryRoot::create("other-game-source");
            let other = TemporaryRoot::create("other-game-other-source");
            let cache = TemporaryRoot::create("other-game-cache");
            let destination = cache.0.join(IMPORTED_ASSET_DIRECTORY_NAME);
            write_source_for_game(&source.0, old);
            write_source_for_game(&other.0, new);
            import_original_assets(&source.0, &destination).unwrap();
            let manifest_path = destination.join(ASSET_MANIFEST_FILENAME);
            let before = std::fs::read(&manifest_path).unwrap();
            let error = import_original_assets(&other.0, &destination).unwrap_err();
            assert!(error.to_string().contains("belongs to"), "{error:#}");
            assert_eq!(std::fs::read(manifest_path).unwrap(), before);
            assert_eq!(ImportedAssetManifest::load(&destination).unwrap().game, old);
        }
    }

    #[test]
    fn legacy_manifest_remains_commander_without_reimporting() {
        let source = TemporaryRoot::create("legacy-game-source");
        let cache = TemporaryRoot::create("legacy-game-cache");
        let destination = cache.0.join(IMPORTED_ASSET_DIRECTORY_NAME);
        write_source(&source.0);
        import_original_assets(&source.0, &destination).unwrap();
        let path = destination.join(ASSET_MANIFEST_FILENAME);
        let mut json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        json.as_object_mut().unwrap().remove("game");
        std::fs::write(path, serde_json::to_vec(&json).unwrap()).unwrap();
        assert_eq!(
            ImportedAssetManifest::load(&destination).unwrap().game,
            GameVariant::CommanderBlood
        );
        assert!(matches!(
            import_original_assets(&source.0, &destination).unwrap(),
            AssetImportOutcome::Reused { .. }
        ));
    }

    #[test]
    fn different_executable_build_is_not_silently_reused() {
        let source = TemporaryRoot::create("build-source");
        let cache = TemporaryRoot::create("build-cache");
        let destination = cache.0.join(IMPORTED_ASSET_DIRECTORY_NAME);
        write_source(&source.0);
        import_original_assets(&source.0, &destination).unwrap();
        std::fs::write(source.0.join("BLOODPRG.EXE"), b"another build").unwrap();
        let error = import_original_assets(&source.0, &destination).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("different Commander Blood executable build")
        );
        assert_eq!(
            std::fs::read(destination.join("companions/BLOODPRG.EXE")).unwrap(),
            b"BLOODPRG.EXE"
        );
    }

    #[test]
    fn same_executable_cannot_reuse_different_archive_or_script_content() {
        for resource in [ORIGINAL_ARCHIVE_FILENAME, "SCRIPT1.COD"] {
            let source = TemporaryRoot::create("content-source");
            let cache = TemporaryRoot::create("content-cache");
            let destination = cache.0.join(IMPORTED_ASSET_DIRECTORY_NAME);
            write_source(&source.0);
            import_original_assets(&source.0, &destination).unwrap();
            let path = source.0.join(resource);
            let mut bytes = std::fs::read(&path).unwrap();
            let last = bytes.last_mut().unwrap();
            *last ^= 1;
            std::fs::write(path, bytes).unwrap();
            assert!(import_original_assets(&source.0, &destination).is_err());
            ImportedAssetManifest::load(&destination)
                .unwrap()
                .validate(&destination, true)
                .unwrap();
        }
    }

    #[test]
    fn source_detection_is_case_insensitive_and_rejects_ambiguity() {
        let source = TemporaryRoot::create("identity-source");
        assert!(detect_source_game(&source.0).is_err());
        std::fs::write(source.0.join("blood2pg.exe"), b"sequel").unwrap();
        assert_eq!(
            detect_source_game(&source.0).unwrap(),
            GameVariant::BigBugBang
        );
        std::fs::write(source.0.join("BLOODPRG.EXE"), b"commander").unwrap();
        assert!(
            detect_source_game(&source.0)
                .unwrap_err()
                .to_string()
                .contains("both main executables")
        );
    }

    #[test]
    fn changing_manifest_game_cannot_relabel_a_commander_cache() {
        let source = TemporaryRoot::create("relabel-source");
        let cache = TemporaryRoot::create("relabel-cache");
        let destination = cache.0.join(IMPORTED_ASSET_DIRECTORY_NAME);
        write_source(&source.0);
        import_original_assets(&source.0, &destination).unwrap();
        let mut manifest = ImportedAssetManifest::load(&destination).unwrap();
        manifest.game = GameVariant::BigBugBang;
        assert!(manifest.validate(&destination, false).is_err());
    }

    #[test]
    fn damaged_other_game_cache_is_not_replaced() {
        let commander = TemporaryRoot::create("damaged-commander");
        let sequel = TemporaryRoot::create("damaged-sequel");
        let cache = TemporaryRoot::create("damaged-cache");
        let destination = cache.0.join(IMPORTED_ASSET_DIRECTORY_NAME);
        write_source(&commander.0);
        write_source_for_game(&sequel.0, GameVariant::BigBugBang);
        import_original_assets(&commander.0, &destination).unwrap();
        std::fs::remove_file(destination.join("resources/SQ/INTRO.HNM")).unwrap();
        let before = std::fs::read(destination.join(ASSET_MANIFEST_FILENAME)).unwrap();
        let error = import_original_assets(&sequel.0, &destination).unwrap_err();
        assert!(error.to_string().contains("belongs to"));
        assert_eq!(
            std::fs::read(destination.join(ASSET_MANIFEST_FILENAME)).unwrap(),
            before
        );
    }

    #[test]
    fn import_cannot_replace_or_nest_inside_the_source_installation() {
        let parent = TemporaryRoot::create("overlap-parent");
        let source = parent.0.join("source");
        std::fs::create_dir(&source).unwrap();
        write_source(&source);
        let before = std::fs::read(source.join(ORIGINAL_ARCHIVE_FILENAME)).unwrap();
        for destination in [&source, &source.join("nested/cache"), &parent.0] {
            assert!(
                import_original_assets(&source, destination)
                    .unwrap_err()
                    .to_string()
                    .contains("overlap")
            );
            assert_eq!(
                std::fs::read(source.join(ORIGINAL_ARCHIVE_FILENAME)).unwrap(),
                before
            );
        }
    }

    #[test]
    #[ignore = "requires the original Big Bug Bang disc under output/big-bug-bang/disc"]
    fn original_sequel_import_loads_its_authentic_initial_profile_from_loose_files() {
        use crate::assets::OriginalResourceStore;
        use crate::native::bloodprg::{
            OriginalResourceCache, ScriptProfileId, ScriptProfileManager,
        };

        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output/big-bug-bang/disc");
        let cache = TemporaryRoot::create("real-sequel-import");
        let destination = cache.0.join(IMPORTED_ASSET_DIRECTORY_NAME);
        import_original_assets(&source, &destination).unwrap();
        let manifest = ImportedAssetManifest::load(&destination).unwrap();
        manifest.validate(&destination, true).unwrap();
        let executable = std::fs::read(
            manifest
                .companion_path(&destination, manifest.game.executable_filename())
                .unwrap(),
        )
        .unwrap();
        let resources = manifest.game.decode_resource_catalog(&executable).unwrap();
        let catalog = manifest.game.decode_profile_catalog(&executable).unwrap();
        assert_eq!(resources.len(), 155);
        assert_eq!(catalog.dialect(), manifest.game.script_dialect());
        let store = OriginalResourceStore::with_writable_root(
            destination.join(RESOURCE_DIRECTORY_NAME),
            cache.0.join("writable"),
            None,
            manifest.resource_names().unwrap(),
            true,
        );
        let mut manager = ScriptProfileManager::new(catalog);
        let initial = ScriptProfileId::new_for_dialect(0, manifest.game.script_dialect()).unwrap();
        manager
            .select(
                initial,
                &mut OriginalResourceCache::new(),
                &store,
                &resources,
            )
            .unwrap();
        let profile = manager.current().unwrap();
        assert_eq!(
            profile.dialogue().encoded_bytes(),
            std::fs::read(source.join("SCRIPT1.COD")).unwrap()
        );
        assert_eq!(
            store
                .load(&BloodResourceName::new(b"SCRIPT1.VAR").unwrap())
                .unwrap()
                .as_ref(),
            std::fs::read(source.join("SCRIPT1.VAR")).unwrap()
        );
        assert!(!destination.join("resources/SCRIPT1.BAS").exists());
        assert!(store.archive_entries().is_none());
    }

    #[test]
    fn refuses_to_import_without_every_runtime_companion() {
        let source = TemporaryRoot::create("asset-missing-companion-source");
        let cache_parent = TemporaryRoot::create("asset-missing-companion-cache-parent");
        let destination = cache_parent.0.join(IMPORTED_ASSET_DIRECTORY_NAME);
        write_source(&source.0);
        std::fs::remove_file(source.0.join("BLOOD.SAV")).unwrap();

        let error = import_original_assets(&source.0, &destination).unwrap_err();
        assert!(error.to_string().contains("BLOOD.SAV"));
        assert!(!destination.exists());
    }
}
