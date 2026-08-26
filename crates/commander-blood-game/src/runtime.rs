//! Original-data discovery and flat runtime bootstrap.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_formats::archive::{BloodArchive, BloodResourceName};
use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};
use commander_blood_formats::palette::decode_bloodprg_default_palette;

use crate::assets::{OriginalResourceSource, OriginalResourceStore};
use crate::native::bloodprg::{
    ORIGINAL_SCRIPT_PROFILE_COUNT, OriginalResourceCache, OriginalResourceCatalog,
    OriginalScriptProfileCatalog, ResourceLoadStatus, ScriptProfileId, ScriptProfileManager,
    StartupWritableResourceCatalog,
};

/// Name of the original packed resource archive.
pub const ORIGINAL_ARCHIVE_FILENAME: &str = "BLOOD.DAT";
/// Name of the original native executable used as a serialized-data source.
pub const ORIGINAL_EXECUTABLE_FILENAME: &str = "BLOODPRG.EXE";
/// Name of the original title artwork.
pub const ORIGINAL_TITLE_FILENAME: &str = "BLOOD.LBM";
/// Name of the original bridge panorama archive.
pub const ORIGINAL_BRIDGE_PANORAMA_FILENAME: &str = "TB.BIG";

const DATA_ROOT_ENVIRONMENT_VARIABLE: &str = "CBLOOD_DATA";
const KNOWN_DATA_ROOTS: [&str; 3] = [
    "commander-blood-audio/_tmp_iso",
    "output/_tmp_iso",
    "accuracy/cblood_install/cblood",
];

/// Required original files resolved below one explicit data root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalGameDataPaths {
    root: PathBuf,
    archive: PathBuf,
    executable: PathBuf,
    title: PathBuf,
    bridge_panorama: PathBuf,
}

impl OriginalGameDataPaths {
    /// Resolve and validate every required companion below `root`.
    pub fn from_root(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        if !root.is_dir() {
            bail!(
                "Commander Blood data root is not a directory: {}",
                root.display()
            );
        }

        let paths = Self {
            archive: root.join(ORIGINAL_ARCHIVE_FILENAME),
            executable: root.join(ORIGINAL_EXECUTABLE_FILENAME),
            title: root.join(ORIGINAL_TITLE_FILENAME),
            bridge_panorama: root.join(ORIGINAL_BRIDGE_PANORAMA_FILENAME),
            root,
        };
        for path in [
            &paths.archive,
            &paths.executable,
            &paths.title,
            &paths.bridge_panorama,
        ] {
            if !path.is_file() {
                bail!(
                    "required Commander Blood data file is missing: {}",
                    path.display()
                );
            }
        }
        Ok(paths)
    }

    /// Discover the original data root from an explicit path, `CBLOOD_DATA`, or repository roots.
    pub fn discover(explicit_root: Option<&Path>) -> Result<Self> {
        if let Some(root) = explicit_root {
            return Self::from_root(root);
        }

        let mut candidates = Vec::new();
        if let Some(root) = std::env::var_os(DATA_ROOT_ENVIRONMENT_VARIABLE) {
            candidates.push(PathBuf::from(root));
        }
        candidates.extend(KNOWN_DATA_ROOTS.map(PathBuf::from));

        for root in candidates {
            if let Ok(paths) = Self::from_root(root) {
                return Ok(paths);
            }
        }
        bail!(
            "complete Commander Blood data set not found; pass --data PATH or set {DATA_ROOT_ENVIRONMENT_VARIABLE}"
        )
    }

    /// Root containing the original files and writable loose resources.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Packed `BLOOD.DAT` path.
    pub fn archive(&self) -> &Path {
        &self.archive
    }

    /// Original `BLOODPRG.EXE` path.
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    /// Original `BLOOD.LBM` title path.
    pub fn title(&self) -> &Path {
        &self.title
    }

    /// Original `TB.BIG` bridge panorama path.
    pub fn bridge_panorama(&self) -> &Path {
        &self.bridge_panorama
    }
}

/// Structural metrics from decoding one complete shipped BloodScript profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptProfileValidation {
    /// Zero-based playable profile identity.
    pub profile: ScriptProfileId,
    /// Framed executable COD instruction count.
    pub code_token_count: usize,
    /// Framed BAS dialogue token count.
    pub dialogue_token_count: usize,
    /// Interned DIC word count.
    pub dictionary_word_count: usize,
    /// DEB symbol count.
    pub directory_entry_count: usize,
}

/// Decoded original resources needed to construct the modern game runtime.
pub struct OriginalGameData {
    paths: OriginalGameDataPaths,
    executable: Box<[u8]>,
    resource_store: OriginalResourceStore,
    resource_catalog: OriginalResourceCatalog,
    script_profile_catalog: OriginalScriptProfileCatalog,
    writable_resource_catalog: StartupWritableResourceCatalog,
    default_palette: [[u8; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
    archive_entry_count: usize,
}

impl fmt::Debug for OriginalGameData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OriginalGameData")
            .field("paths", &self.paths)
            .field("executable_byte_count", &self.executable.len())
            .field("archive_entry_count", &self.archive_entry_count)
            .field("resource_count", &self.resource_catalog.len())
            .field(
                "writable_resource_count",
                &self.writable_resource_catalog.len(),
            )
            .finish_non_exhaustive()
    }
}

impl OriginalGameData {
    /// Read and validate the complete original game-data set.
    pub fn load(paths: OriginalGameDataPaths) -> Result<Self> {
        let executable = std::fs::read(paths.executable())
            .with_context(|| format!("reading {}", paths.executable().display()))?
            .into_boxed_slice();
        let resource_catalog = OriginalResourceCatalog::decode_bloodprg(&executable)
            .context("decoding original resource catalog")?;
        let script_profile_catalog = OriginalScriptProfileCatalog::decode_bloodprg(&executable)
            .context("decoding BloodScript profile catalog")?;
        let writable_resource_catalog =
            StartupWritableResourceCatalog::decode_bloodprg(&executable)
                .context("decoding startup writable-resource catalog")?;
        let default_palette = decode_bloodprg_default_palette(&executable)
            .context("decoding original default palette")?;

        let archive_bytes = std::fs::read(paths.archive())
            .with_context(|| format!("reading {}", paths.archive().display()))?
            .into_boxed_slice();
        let archive = BloodArchive::decode(archive_bytes)
            .with_context(|| format!("decoding {}", paths.archive().display()))?;
        let archive_entry_count = archive.entries().len();
        let resource_store =
            OriginalResourceStore::new(paths.root().to_owned(), Some(archive), [], false);

        Ok(Self {
            paths,
            executable,
            resource_store,
            resource_catalog,
            script_profile_catalog,
            writable_resource_catalog,
            default_palette,
            archive_entry_count,
        })
    }

    /// Paths owning this loaded data set.
    pub const fn paths(&self) -> &OriginalGameDataPaths {
        &self.paths
    }

    /// Original executable bytes retained as serialized tables, not executable memory.
    pub const fn executable(&self) -> &[u8] {
        &self.executable
    }

    /// Archive-or-loose resource service used by translated game systems.
    pub const fn resource_store(&self) -> &OriginalResourceStore {
        &self.resource_store
    }

    /// Stable resource-ID catalog decoded from the executable.
    pub const fn resource_catalog(&self) -> &OriginalResourceCatalog {
        &self.resource_catalog
    }

    /// Five authored BloodScript profile resource matrices.
    pub const fn script_profile_catalog(&self) -> &OriginalScriptProfileCatalog {
        &self.script_profile_catalog
    }

    /// Startup resources copied to the writable data root when absent.
    pub const fn writable_resource_catalog(&self) -> &StartupWritableResourceCatalog {
        &self.writable_resource_catalog
    }

    /// Default 256-color palette expanded from six-bit VGA components.
    pub const fn default_palette(&self) -> &[[u8; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT] {
        &self.default_palette
    }

    /// Number of original archive directory entries visible to native lookup.
    pub const fn archive_entry_count(&self) -> usize {
        self.archive_entry_count
    }

    /// Decode every playable BloodScript profile through the archive-backed resource service.
    pub fn validate_script_profiles(&self) -> Result<Vec<ScriptProfileValidation>> {
        let mut validations = Vec::with_capacity(ORIGINAL_SCRIPT_PROFILE_COUNT);
        for profile in ScriptProfileId::all() {
            let resources = self.script_profile_catalog.profile(profile);
            for resource in resources.all() {
                let name = self.resource_catalog.name(resource).with_context(|| {
                    format!(
                        "profile {} references unknown resource {}",
                        profile.value(),
                        resource.value()
                    )
                })?;
                if self.resource_store.source(name) != OriginalResourceSource::EmbeddedArchive {
                    bail!(
                        "profile {} resource {} ({}) is not present in BLOOD.DAT",
                        profile.value(),
                        resource.value(),
                        String::from_utf8_lossy(name.as_bytes())
                    );
                }
            }

            let mut manager = ScriptProfileManager::new(self.script_profile_catalog.clone());
            let mut cache = OriginalResourceCache::new();
            let outcome = manager
                .select(
                    profile,
                    &mut cache,
                    &self.resource_store,
                    &self.resource_catalog,
                )
                .with_context(|| format!("loading BloodScript profile {}", profile.value()))?;
            if outcome.resource_statuses
                != [ResourceLoadStatus::LoadedNow;
                    crate::native::bloodprg::SCRIPT_PROFILE_RESOURCE_COUNT]
            {
                bail!(
                    "profile {} did not load five fresh resources",
                    profile.value()
                );
            }
            let loaded = manager
                .current()
                .context("profile manager did not retain the selected profile")?;
            validations.push(ScriptProfileValidation {
                profile,
                code_token_count: loaded.code().tokens().len(),
                dialogue_token_count: loaded.dialogue().tokens().len(),
                dictionary_word_count: loaded.dictionary().len(),
                directory_entry_count: loaded.directory().entries().len(),
            });
        }
        Ok(validations)
    }

    /// Load a named original archive member through native-compatible name folding.
    pub fn load_named_resource(&self, name: impl AsRef<[u8]>) -> Result<Box<[u8]>> {
        let name = BloodResourceName::new(name).context("validating original resource name")?;
        self.resource_store.load(&name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANU3_RESOURCE_NAME: &[u8] = b"MANU3.XDB";

    #[test]
    fn discovers_and_bootstraps_the_complete_original_data_set() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        let data = OriginalGameData::load(paths).unwrap();

        assert_eq!(
            data.resource_catalog().len(),
            crate::native::bloodprg::ORIGINAL_RESOURCE_COUNT
        );
        assert_eq!(
            data.writable_resource_catalog().len(),
            crate::native::bloodprg::STARTUP_WRITABLE_RESOURCE_COUNT
        );
        assert!(data.archive_entry_count() > crate::native::bloodprg::ORIGINAL_RESOURCE_COUNT);
        assert_eq!(
            data.validate_script_profiles().unwrap().len(),
            ORIGINAL_SCRIPT_PROFILE_COUNT
        );
        assert!(
            !data
                .load_named_resource(MANU3_RESOURCE_NAME)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_an_incomplete_explicit_data_root() {
        let missing = std::env::temp_dir().join(format!(
            "commander-blood-missing-data-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&missing).unwrap();
        let error = OriginalGameDataPaths::from_root(&missing).unwrap_err();
        assert!(error.to_string().contains(ORIGINAL_ARCHIVE_FILENAME));
        std::fs::remove_dir_all(missing).unwrap();
    }
}
