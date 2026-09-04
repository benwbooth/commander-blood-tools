//! Original-data discovery and flat runtime bootstrap.

mod alien_overlay;
mod audio;
mod bios_font;
mod bridge_actors;
mod bridge_console;
mod bridge_frame;
mod camera_approach;
mod camera_navigation;
mod choice_list;
mod confirm_dialog;
mod contact_scenario;
mod game_lifecycle;
mod input;
mod navigation_chart;
mod navigation_status;
mod palette_transition;
mod platform;
mod presentation;
mod presentation_catalog;
mod presentation_player;
mod presentation_run;
mod presentation_scene;
mod presentation_screen;
mod save_load;
mod scenario;
mod scene_transition;
mod script_backend;
mod services;
mod shared_ui;
mod ship_hud;
mod ship_navigation;
mod ship_presentation;
mod ship_target;
mod startup;
mod state;
mod subtitles;
mod video;
mod word_choice;

pub use alien_overlay::{
    RuntimeAlienOverlayCycle, RuntimeAlienOverlayFrameHost, RuntimeAlienOverlayFrameInput,
    RuntimeAlienOverlayOutcome, run_runtime_alien_overlay,
};
pub use audio::{RuntimeAudioHost, RuntimePcmClip, RuntimePcmMixer};
pub use bios_font::VGA_BIOS_FONT_8X8;
pub use confirm_dialog::RuntimeConfirmDialog;
pub use game_lifecycle::RuntimeGameLifecycleHost;
pub use input::{RuntimeInputHost, map_host_pointer_to_logical};
pub use palette_transition::{
    RuntimePaletteTransition, RuntimePaletteTransitionConfig, RuntimePaletteTransitionOutcome,
    RuntimePaletteTransitionSurface,
};
pub use platform::{
    GAME_FRAME_DURATION, PRESENTATION_FRAME_DURATION, RECOVERED_FRAME_BUDGET, RuntimePlatformHost,
};
pub use presentation::RuntimePresentationHost;
pub use presentation_catalog::{RuntimePresentationBackground, RuntimePresentationCatalog};
pub use presentation_player::RuntimePresentationPlayer;
pub use presentation_run::{RuntimePresentationRunOutcome, run_runtime_presentation};
pub use presentation_scene::RuntimePresentationScene;
pub use presentation_screen::RuntimePresentationScreen;
pub use save_load::RuntimeSaveLoad;
pub use scene_transition::RuntimeSceneTransition;
pub use script_backend::{
    LoadedRuntimeResource, RuntimeScriptActionEffects, RuntimeScriptBackend, RuntimeScriptCommand,
    RuntimeScriptSystem, initialize_and_restore_original_save_game,
};
pub use services::ModernGameServices;
pub use ship_hud::RuntimeShipHud;
pub use ship_navigation::RuntimeShipNavigation;
pub use ship_target::{RuntimeShipTargetSelection, RuntimeShipTargetSelector};
pub use state::{
    IndexedFramebuffer, LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_PIXEL_COUNT,
    LOGICAL_FRAMEBUFFER_WIDTH, OriginalGameRuntime, RuntimeAssetLoadStatus,
};
pub use subtitles::RuntimeSubtitleReveal;
pub use video::{
    RuntimePresentationQueueMetrics, RuntimePresentationRequest, RuntimePresentationStepOutcome,
    RuntimePresentationStream,
};
pub use word_choice::RuntimePresentationWordChoice;

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::bloodprg::{
    BloodprgBridgeMenuText, BloodprgConfirmDialogRegions, BloodprgFontResources,
    BloodprgHyperspaceResources, BloodprgNavigationResources, BloodprgPresentationCatalog,
    decode_bloodprg_bridge_menu_text, decode_bloodprg_confirm_dialog_regions,
    decode_bloodprg_font_resources, decode_bloodprg_hyperspace_resources,
    decode_bloodprg_navigation_resources, decode_bloodprg_presentation_catalog,
};
use commander_blood_formats::descript_database::DescriptDatabase;
use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};
use commander_blood_formats::name_area_effect::{
    NameAreaEffectSequence, decode_bloodprg_name_area_effect_sequences,
};
use commander_blood_formats::palette::decode_bloodprg_default_vga_palette;
use commander_blood_formats::world_art::{
    WorldArtworkLayout, decode_bloodprg_world_artwork_layout,
};

use crate::asset_import::{
    ASSET_MANIFEST_FILENAME, AssetImportOutcome, IMPORTED_ASSET_DIRECTORY_NAME,
    ImportedAssetManifest, RESOURCE_DIRECTORY_NAME, import_original_assets,
};
use crate::assets::OriginalResourceStore;
use crate::media_import::NormalizedMediaStore;
use crate::native::bloodprg::{
    ORIGINAL_SCRIPT_PROFILE_COUNT, OriginalResourceCache, OriginalResourceCatalog,
    OriginalScriptProfileCatalog, ResourceLoadStatus, ScriptProfileId, ScriptProfileManager,
    StartupWritableResourceCatalog,
};
use crate::script_rebuild::prepare_verified_script_artifacts;

/// Name of the original packed resource archive.
pub const ORIGINAL_ARCHIVE_FILENAME: &str = "BLOOD.DAT";
/// Name of the original native executable used as a serialized-data source.
pub const ORIGINAL_EXECUTABLE_FILENAME: &str = "BLOODPRG.EXE";
/// Name of the original title artwork.
pub const ORIGINAL_TITLE_FILENAME: &str = "BLOOD.LBM";
/// Name of the original bridge panorama archive.
pub const ORIGINAL_BRIDGE_PANORAMA_FILENAME: &str = "TB.BIG";
/// Name of the authored scene, dialogue, subtitle, and audio catalog.
pub const ORIGINAL_DESCRIPT_FILENAME: &str = "DESCRIPT.DES";
/// Name of the original writable save-slot template copied during startup.
pub const ORIGINAL_SAVE_DIRECTORY_FILENAME: &str = "BLOOD.SAV";

const DATA_ROOT_ENVIRONMENT_VARIABLE: &str = "CBLOOD_DATA";
const ASSET_CACHE_ROOT_ENVIRONMENT_VARIABLE: &str = "CBLOOD_ASSET_CACHE";
const WRITABLE_DATA_ROOT_ENVIRONMENT_VARIABLE: &str = "CBLOOD_WRITE_DATA";
const XDG_DATA_HOME_ENVIRONMENT_VARIABLE: &str = "XDG_DATA_HOME";
const HOME_ENVIRONMENT_VARIABLE: &str = "HOME";
const USER_DATA_DIRECTORY_NAME: &str = "commander-blood";
const KNOWN_DATA_ROOTS: [&str; 3] = [
    "commander-blood-audio/_tmp_iso",
    "output/_tmp_iso",
    "accuracy/cblood_install/cblood",
];

/// Required original files resolved below one explicit data root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalGameDataPaths {
    root: PathBuf,
    resource_root: PathBuf,
    executable: PathBuf,
    title: PathBuf,
    bridge_panorama: PathBuf,
    descript: PathBuf,
    manifest: ImportedAssetManifest,
    media_store: NormalizedMediaStore,
}

impl OriginalGameDataPaths {
    /// Resolve an imported loose store, importing one original installation when needed.
    pub fn from_root(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        if !root.is_dir() {
            bail!(
                "Commander Blood data root is not a directory: {}",
                root.display()
            );
        }

        if root.join(ASSET_MANIFEST_FILENAME).is_file() {
            return Self::from_imported_root(root);
        }
        if !root_file_exists_case_insensitive(&root, ORIGINAL_ARCHIVE_FILENAME)? {
            bail!(
                "Commander Blood data root has neither an imported manifest nor {ORIGINAL_ARCHIVE_FILENAME}: {}",
                root.display()
            );
        }
        let imported_root = discover_asset_cache_root(None)?;
        if ImportedAssetManifest::load(&imported_root).is_err() {
            eprintln!(
                "Importing original Commander Blood assets into {}. This one-time operation may take several minutes.",
                imported_root.display()
            );
        }
        let outcome = import_original_assets(&root, &imported_root).with_context(|| {
            format!(
                "importing original Commander Blood assets from {}",
                root.display()
            )
        })?;
        if let AssetImportOutcome::Imported { resource_count } = outcome {
            eprintln!(
                "Imported {resource_count} loose Commander Blood resources into {}.",
                imported_root.display()
            );
        }
        Self::from_imported_root(imported_root)
    }

    fn from_imported_root(root: PathBuf) -> Result<Self> {
        let manifest = ImportedAssetManifest::load(&root)?;
        let media_store = NormalizedMediaStore::prepare(&root, &manifest)?;

        let paths = Self {
            resource_root: root.join(RESOURCE_DIRECTORY_NAME),
            executable: manifest.companion_path(&root, ORIGINAL_EXECUTABLE_FILENAME)?,
            title: manifest.companion_path(&root, ORIGINAL_TITLE_FILENAME)?,
            bridge_panorama: manifest.companion_path(&root, ORIGINAL_BRIDGE_PANORAMA_FILENAME)?,
            descript: manifest.companion_path(&root, ORIGINAL_DESCRIPT_FILENAME)?,
            manifest,
            media_store,
            root,
        };
        if !paths.resource_root.is_dir() {
            bail!(
                "required imported resource directory is missing: {}",
                paths.resource_root.display()
            );
        }
        for path in [
            &paths.executable,
            &paths.title,
            &paths.bridge_panorama,
            &paths.descript,
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

        let imported_root = discover_asset_cache_root(None)?;
        if let Ok(paths) = Self::from_imported_root(imported_root) {
            return Ok(paths);
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

    /// Read-only root containing the original game files.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Directory containing every imported resource as an ordinary loose file.
    pub fn resource_root(&self) -> &Path {
        &self.resource_root
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

    /// Authored `DESCRIPT.DES` presentation database path.
    pub fn descript(&self) -> &Path {
        &self.descript
    }

    fn manifest(&self) -> &ImportedAssetManifest {
        &self.manifest
    }
}

fn root_file_exists_case_insensitive(root: &Path, filename: &str) -> Result<bool> {
    let exact = root.join(filename);
    if exact.is_file() {
        return Ok(true);
    }
    for entry in std::fs::read_dir(root)
        .with_context(|| format!("reading Commander Blood data root {}", root.display()))?
    {
        let entry = entry
            .with_context(|| format!("reading Commander Blood data root {}", root.display()))?;
        let Some(candidate) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if candidate.eq_ignore_ascii_case(filename) && entry.path().is_file() {
            return Ok(true);
        }
    }
    Ok(false)
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
    descript_database: DescriptDatabase,
    confirm_dialog_regions: BloodprgConfirmDialogRegions,
    bridge_menu_text: BloodprgBridgeMenuText,
    hyperspace_resources: BloodprgHyperspaceResources,
    navigation_resources: BloodprgNavigationResources,
    font_resources: BloodprgFontResources,
    presentation_catalog: BloodprgPresentationCatalog,
    default_vga_palette: [[u8; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
    name_area_effect_sequences: Box<[NameAreaEffectSequence]>,
    world_artwork_layout: Box<[WorldArtworkLayout]>,
    archive_entry_count: usize,
    imported_resource_count: usize,
    normalized_audio_count: usize,
    media_store: NormalizedMediaStore,
}

impl fmt::Debug for OriginalGameData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OriginalGameData")
            .field("paths", &self.paths)
            .field("executable_byte_count", &self.executable.len())
            .field("archive_entry_count", &self.archive_entry_count)
            .field("imported_resource_count", &self.imported_resource_count)
            .field("normalized_audio_count", &self.normalized_audio_count)
            .field("resource_count", &self.resource_catalog.len())
            .field(
                "descript_record_count",
                &self.descript_database.records().len(),
            )
            .field(
                "writable_resource_count",
                &self.writable_resource_catalog.len(),
            )
            .finish_non_exhaustive()
    }
}

impl OriginalGameData {
    /// Read and validate the complete original game-data set using the host's user-data root.
    pub fn load(paths: OriginalGameDataPaths) -> Result<Self> {
        let writable_root = discover_writable_data_root(None)?;
        Self::load_with_writable_root(paths, writable_root)
    }

    /// Read original data while keeping all runtime-owned files below `writable_root`.
    pub fn load_with_writable_root(
        paths: OriginalGameDataPaths,
        writable_root: impl Into<PathBuf>,
    ) -> Result<Self> {
        let writable_root = writable_root.into();
        let verified_scripts = prepare_verified_script_artifacts(
            paths.descript(),
            paths.resource_root(),
            &writable_root,
        )
        .context("checking editable game-script sources")?;
        if verified_scripts.rebuilt_unit_count != 0 {
            eprintln!(
                "Rebuilt and byte-exactly verified {} newer game-script source unit(s).",
                verified_scripts.rebuilt_unit_count
            );
        }
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
        let confirm_dialog_regions = decode_bloodprg_confirm_dialog_regions(&executable)
            .context("decoding confirmation-dialog hit regions")?;
        let bridge_menu_text = decode_bloodprg_bridge_menu_text(&executable)
            .context("decoding bridge options and text-speed labels")?;
        let hyperspace_resources = decode_bloodprg_hyperspace_resources(&executable)
            .context("decoding camera-travel hyperspace resources")?;
        let navigation_resources = decode_bloodprg_navigation_resources(&executable)
            .context("decoding navigation labels and radial-wipe endpoints")?;
        let font_resources = decode_bloodprg_font_resources(&executable)
            .context("decoding original executable font resources")?;
        let presentation_catalog = decode_bloodprg_presentation_catalog(&executable)
            .context("decoding executable presentation-line catalog")?;
        let default_vga_palette = decode_bloodprg_default_vga_palette(&executable)
            .context("decoding original default palette")?;
        let name_area_effect_sequences = decode_bloodprg_name_area_effect_sequences(&executable)
            .context("decoding executable name-area effect sequences")?;
        let world_artwork_layout = decode_bloodprg_world_artwork_layout(&executable)
            .context("decoding executable world-artwork layout")?;
        let descript_bytes = match verified_scripts.descript {
            Some(bytes) => bytes,
            None => std::fs::read(paths.descript())
                .with_context(|| format!("reading {}", paths.descript().display()))?
                .into_boxed_slice(),
        };
        let descript_database = DescriptDatabase::parse(&descript_bytes).map_err(|error| {
            anyhow::anyhow!("decoding {}: {error:?}", paths.descript().display())
        })?;

        let archive_entry_count = paths.manifest().source_archive_entry_count;
        let imported_resource_count = paths.manifest().resources.len();
        let normalized_audio_count = paths.media_store.audio_entry_count();
        let media_store = paths.media_store.clone();
        let resource_names = paths.manifest().resource_names()?;
        let mut resource_store = OriginalResourceStore::with_writable_root(
            paths.resource_root().to_owned(),
            writable_root,
            None,
            resource_names,
            true,
        );
        resource_store.install_verified_overrides(verified_scripts.resources);

        Ok(Self {
            paths,
            executable,
            resource_store,
            resource_catalog,
            script_profile_catalog,
            writable_resource_catalog,
            descript_database,
            confirm_dialog_regions,
            bridge_menu_text,
            hyperspace_resources,
            navigation_resources,
            font_resources,
            presentation_catalog,
            default_vga_palette,
            name_area_effect_sequences,
            world_artwork_layout,
            archive_entry_count,
            imported_resource_count,
            normalized_audio_count,
            media_store,
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

    pub(crate) const fn normalized_media(&self) -> &NormalizedMediaStore {
        &self.media_store
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

    /// Parsed authored scene, dialogue, subtitle, and audio combinations.
    pub const fn descript_database(&self) -> &DescriptDatabase {
        &self.descript_database
    }

    /// Executable-authored logical hit regions for the navigation confirmation modal.
    pub const fn confirm_dialog_regions(&self) -> &BloodprgConfirmDialogRegions {
        &self.confirm_dialog_regions
    }

    /// Exact bridge options, text-speed choices, and shared cancel label.
    pub const fn bridge_menu_text(&self) -> &BloodprgBridgeMenuText {
        &self.bridge_menu_text
    }

    /// Exact eight-clip hyperspace cycle selected by the camera coordinator.
    pub const fn hyperspace_resources(&self) -> &BloodprgHyperspaceResources {
        &self.hyperspace_resources
    }

    /// Exact labels and radial-wipe endpoints used by bridge navigation.
    pub const fn navigation_resources(&self) -> &BloodprgNavigationResources {
        &self.navigation_resources
    }

    /// Exact compact, subtitle, square-cap, and dialogue fonts from the executable.
    pub const fn font_resources(&self) -> &BloodprgFontResources {
        &self.font_resources
    }

    /// Initial streamed-video templates indexed by authored presentation line.
    pub const fn presentation_catalog(&self) -> &BloodprgPresentationCatalog {
        &self.presentation_catalog
    }

    /// Default 256-color palette retaining the native six-bit VGA DAC components.
    pub const fn default_vga_palette(&self) -> &[[u8; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT] {
        &self.default_vga_palette
    }

    /// Authored palette-effect sequences used by the bridge character-name area.
    pub fn name_area_effect_sequences(&self) -> &[NameAreaEffectSequence] {
        &self.name_area_effect_sequences
    }

    /// Immutable starting copy of the executable's world-artwork selection table.
    pub fn world_artwork_layout(&self) -> &[WorldArtworkLayout] {
        &self.world_artwork_layout
    }

    /// Number of original archive directory entries visible to native lookup.
    pub const fn archive_entry_count(&self) -> usize {
        self.archive_entry_count
    }

    /// Number of unique effective resources represented by the imported loose manifest.
    pub const fn imported_resource_count(&self) -> usize {
        self.imported_resource_count
    }

    /// Decode every playable BloodScript profile through the original resource service.
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
                if !self.resource_store.resource_exists(name)? {
                    bail!(
                        "profile {} resource {} ({}) is not resolvable from the original data set",
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

/// Select the flat host directory that owns saves and startup-copied resources.
pub fn discover_writable_data_root(explicit_root: Option<&Path>) -> Result<PathBuf> {
    if let Some(root) = explicit_root {
        return Ok(root.to_owned());
    }
    if let Some(root) = std::env::var_os(WRITABLE_DATA_ROOT_ENVIRONMENT_VARIABLE) {
        return Ok(PathBuf::from(root));
    }
    if let Some(root) = std::env::var_os(XDG_DATA_HOME_ENVIRONMENT_VARIABLE) {
        return Ok(PathBuf::from(root).join(USER_DATA_DIRECTORY_NAME));
    }
    if let Some(home) = std::env::var_os(HOME_ENVIRONMENT_VARIABLE) {
        return Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join(USER_DATA_DIRECTORY_NAME));
    }
    Ok(std::env::current_dir()
        .context("resolving current directory for writable game data")?
        .join(USER_DATA_DIRECTORY_NAME))
}

/// Select the versioned directory that owns immutable imported game assets.
pub fn discover_asset_cache_root(explicit_root: Option<&Path>) -> Result<PathBuf> {
    if let Some(root) = explicit_root {
        return Ok(root.to_owned());
    }
    if let Some(root) = std::env::var_os(ASSET_CACHE_ROOT_ENVIRONMENT_VARIABLE) {
        return Ok(PathBuf::from(root));
    }
    Ok(discover_writable_data_root(None)?.join(IMPORTED_ASSET_DIRECTORY_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGINAL_DESCRIPT_RECORD_COUNT: usize = 145;
    const ORIGINAL_NAME_AREA_EFFECT_SEQUENCE_COUNT: usize = 10;
    const ORIGINAL_WORLD_ARTWORK_LAYOUT_COUNT: usize = 42;

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
            data.descript_database().records().len(),
            ORIGINAL_DESCRIPT_RECORD_COUNT
        );
        assert!(data.descript_database().lookup(b"Scruter_Jo").is_some());
        assert_eq!(
            data.name_area_effect_sequences().len(),
            ORIGINAL_NAME_AREA_EFFECT_SEQUENCE_COUNT
        );
        assert_eq!(
            data.world_artwork_layout().len(),
            ORIGINAL_WORLD_ARTWORK_LAYOUT_COUNT
        );
        assert_eq!(
            data.presentation_catalog().lines().len(),
            commander_blood_formats::bloodprg::BLOODPRG_PRESENTATION_LINE_COUNT
        );
        assert_eq!(data.bridge_menu_text().option_labels()[0].as_ref(), b"TEXT");
        assert_eq!(data.bridge_menu_text().initial_text_speed_step(), 2);
        assert_eq!(
            data.validate_script_profiles().unwrap().len(),
            ORIGINAL_SCRIPT_PROFILE_COUNT
        );
        assert!(
            data.default_vga_palette()
                .iter()
                .flatten()
                .all(|component| *component <= 63)
        );

        let mut runtime = OriginalGameRuntime::new(data);
        assert_eq!(
            runtime.load_manu3().unwrap(),
            RuntimeAssetLoadStatus::LoadedNow
        );
        assert_eq!(
            runtime.load_manu3().unwrap(),
            RuntimeAssetLoadStatus::AlreadyLoaded
        );
        assert_eq!(
            runtime.open_bridge_panorama().unwrap(),
            RuntimeAssetLoadStatus::LoadedNow
        );
        assert!(runtime.bridge_panorama().unwrap().frame_count() > 0);
        runtime.initialize_back_buffer().unwrap();
        assert!(
            runtime
                .back_buffer()
                .pixels()
                .iter()
                .any(|pixel| *pixel != 0)
        );
        let initial_profile = ScriptProfileId::new(0).unwrap();
        runtime.load_profile(initial_profile).unwrap();
        assert_eq!(runtime.current_profile().unwrap().id(), initial_profile);
    }

    #[test]
    fn explicit_writable_root_does_not_replace_the_original_source_root() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        let writable_root = std::env::temp_dir().join(format!(
            "commander-blood-runtime-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&writable_root);
        let data =
            OriginalGameData::load_with_writable_root(paths.clone(), &writable_root).unwrap();

        assert_eq!(data.paths().root(), paths.root());
        assert_eq!(
            data.resource_store().loose_source_root(),
            paths.resource_root()
        );
        assert_eq!(data.resource_store().writable_root(), writable_root);
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

    #[test]
    fn asset_cache_root_is_versioned_below_the_writable_data_root() {
        let writable_root = std::env::temp_dir().join(format!(
            "commander-blood-cache-root-test-{}",
            std::process::id()
        ));
        assert_eq!(
            discover_asset_cache_root(Some(&writable_root)).unwrap(),
            writable_root
        );
    }
}
