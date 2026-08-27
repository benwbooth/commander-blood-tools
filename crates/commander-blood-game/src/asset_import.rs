//! One-time conversion from the packed DOS installation to a loose asset store.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_formats::archive::{BloodArchive, BloodResourceName};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(crate) const ASSET_MANIFEST_FILENAME: &str = "manifest.json";
pub(crate) const IMPORTED_ASSET_DIRECTORY_NAME: &str = "assets-v1";
pub(crate) const IMPORTED_ASSET_SCHEMA_VERSION: u32 = 1;
pub(crate) const RESOURCE_DIRECTORY_NAME: &str = "resources";

const COMPANION_DIRECTORY_NAME: &str = "companions";
const ORIGINAL_ARCHIVE_FILENAME: &str = "BLOOD.DAT";
const TEMPORARY_IMPORT_INFIX: &str = "import";
const REPLACED_IMPORT_INFIX: &str = "replaced";
const SHA256_BYTE_COUNT: usize = 32;
const REQUIRED_COMPANION_FILENAMES: [&str; 5] = [
    "BLOODPRG.EXE",
    "BLOOD.LBM",
    "TB.BIG",
    "DESCRIPT.DES",
    "BLOOD.SAV",
];

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
    pub(crate) source_archive_sha256: String,
    pub(crate) source_archive_byte_count: u64,
    pub(crate) source_archive_entry_count: usize,
    pub(crate) resources: Vec<ImportedAssetEntry>,
    pub(crate) companions: Vec<ImportedCompanionEntry>,
}

impl ImportedAssetManifest {
    pub(crate) fn load(root: &Path) -> Result<Self> {
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
        manifest.validate(root, false)?;
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
        for required in REQUIRED_COMPANION_FILENAMES {
            if !companion_names
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(required))
            {
                bail!("imported asset manifest is missing companion {required}");
            }
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
    if let Ok(manifest) = ImportedAssetManifest::load(destination_root) {
        return Ok(AssetImportOutcome::Reused {
            resource_count: manifest.resources.len(),
        });
    }
    if !source_root.is_dir() {
        bail!(
            "Commander Blood import source is not a directory: {}",
            source_root.display()
        );
    }

    let archive_path = case_insensitive_child(source_root, ORIGINAL_ARCHIVE_FILENAME)?
        .with_context(|| {
            format!(
                "Commander Blood import source has no {ORIGINAL_ARCHIVE_FILENAME}: {}",
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

    let mut companions = Vec::with_capacity(REQUIRED_COMPANION_FILENAMES.len());
    for filename in REQUIRED_COMPANION_FILENAMES {
        let source = case_insensitive_child(source_root, filename)?.with_context(|| {
            format!(
                "Commander Blood import source is missing required companion {filename}: {}",
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
        let archive = archive_bytes(&[
            (r"SQ\INTRO.HNM", b"video"),
            (r"SN\TB.SND", b"sound"),
            ("DUP.DAT", b"archive wins"),
        ]);
        std::fs::write(source.join(ORIGINAL_ARCHIVE_FILENAME), archive).unwrap();
        for filename in REQUIRED_COMPANION_FILENAMES {
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
            REQUIRED_COMPANION_FILENAMES.len()
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
