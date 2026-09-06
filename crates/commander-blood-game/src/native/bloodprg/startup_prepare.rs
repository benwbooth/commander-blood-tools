//! Loading-screen and writable-resource preparation from the original catalog.

use std::error::Error;
use std::fmt;

use commander_blood_formats::archive::{BloodArchiveError, BloodResourceName};

use super::IndexedGamePalette;

/// File position of the writable-resource table in `BLOODPRG.EXE`.
pub const BLOODPRG_WRITABLE_RESOURCE_CATALOG_FILE_OFFSET: usize = 0x00D679;
/// Number of fixed-width resource names visited during original startup.
pub const STARTUP_WRITABLE_RESOURCE_COUNT: usize = 125;

const WRITABLE_RESOURCE_NAME_FIELD_SIZE: usize = 16;
const STARTUP_LOADING_TEXT: &[u8] = b"LOADING";
const STARTUP_LOADING_TEXT_POSITION: [u16; 2] = [130, 96];
const STARTUP_LOADING_TEXT_COLOR: u8 = 239;
const STARTUP_LOADING_TEXT_BYTE_LIMIT: usize = 255;
const STARTUP_LOADING_BACKGROUND_COLOR: u8 = 0;

/// Stable index into the original writable-resource table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StartupWritableResourceId(usize);

impl StartupWritableResourceId {
    /// Return the zero-based authored table index.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Exact ordered startup resource catalog decoded from the executable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartupWritableResourceCatalog {
    names: Box<[BloodResourceName]>,
}

impl StartupWritableResourceCatalog {
    /// Decode all 125 names and reject malformed executable data.
    pub fn decode_bloodprg(executable: &[u8]) -> Result<Self, StartupWritableCatalogError> {
        Self::decode_at(
            executable,
            BLOODPRG_WRITABLE_RESOURCE_CATALOG_FILE_OFFSET,
            STARTUP_WRITABLE_RESOURCE_COUNT,
        )
    }

    /// Decode the sequel's 152 names and the terminator tested at file 0x1947.
    pub fn decode_blood2pg(executable: &[u8]) -> Result<Self, StartupWritableCatalogError> {
        let end = 0xFA90 + 152 * WRITABLE_RESOURCE_NAME_FIELD_SIZE;
        match executable.get(end) {
            Some(0) => Self::decode_at(executable, 0xFA90, 152),
            Some(_) => Err(StartupWritableCatalogError::MissingTableTerminator { offset: end }),
            None => Err(StartupWritableCatalogError::ExecutableTooShort {
                required: end + 1,
                actual: executable.len(),
            }),
        }
    }

    fn decode_at(
        executable: &[u8],
        offset: usize,
        count: usize,
    ) -> Result<Self, StartupWritableCatalogError> {
        let required = offset + count * WRITABLE_RESOURCE_NAME_FIELD_SIZE;
        if executable.len() < required {
            return Err(StartupWritableCatalogError::ExecutableTooShort {
                required,
                actual: executable.len(),
            });
        }

        let mut names = Vec::with_capacity(count);
        for resource_index in 0..count {
            let start = offset + resource_index * WRITABLE_RESOURCE_NAME_FIELD_SIZE;
            let field = &executable[start..start + WRITABLE_RESOURCE_NAME_FIELD_SIZE];
            let name_length = field.iter().position(|byte| *byte == u8::MIN).ok_or(
                StartupWritableCatalogError::UnterminatedName {
                    resource: StartupWritableResourceId(resource_index),
                },
            )?;
            let name = BloodResourceName::new(&field[..name_length]).map_err(|source| {
                StartupWritableCatalogError::InvalidName {
                    resource: StartupWritableResourceId(resource_index),
                    source,
                }
            })?;
            names.push(name);
        }
        Ok(Self {
            names: names.into_boxed_slice(),
        })
    }

    /// Number of authored entries, including deliberate duplicate names.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Return whether the decoded catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Iterate over stable identifiers and authored names in native order.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (StartupWritableResourceId, &BloodResourceName)> {
        self.names
            .iter()
            .enumerate()
            .map(|(index, name)| (StartupWritableResourceId(index), name))
    }
}

/// Invalid executable bounds or malformed fixed-width startup name.
#[derive(Debug)]
pub enum StartupWritableCatalogError {
    /// The executable ends before the complete game-specific table.
    ExecutableTooShort {
        /// Minimum required byte count.
        required: usize,
        /// Actual executable byte count.
        actual: usize,
    },
    /// The byte following the sequel's complete table is not NUL.
    MissingTableTerminator {
        /// File offset of the expected terminator.
        offset: usize,
    },
    /// One fixed-width name has no NUL terminator.
    UnterminatedName {
        /// Entry owning the malformed field.
        resource: StartupWritableResourceId,
    },
    /// One extracted name is invalid for archive and loose-file lookup.
    InvalidName {
        /// Entry owning the malformed field.
        resource: StartupWritableResourceId,
        /// Validation failure.
        source: BloodArchiveError,
    },
}

impl fmt::Display for StartupWritableCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid startup writable-resource catalog: {self:?}"
        )
    }
}

impl Error for StartupWritableCatalogError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidName { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Text draw requested by the original loading frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartupLoadingText {
    /// ASCII bytes rendered by the game font.
    pub text: &'static [u8],
    /// Logical upper-left baseline position.
    pub position: [u16; 2],
    /// Indexed palette color.
    pub color: u8,
    /// Maximum source bytes accepted by the original renderer.
    pub byte_limit: usize,
}

/// Modern rendering and explicit-root filesystem operations used by startup.
pub trait StartupPreparationHost {
    /// Fatal adapter failure propagated without inventing resource data.
    type Error;

    /// Publish the bridge panorama palette used by the loading screen.
    fn publish_loading_palette(&mut self, palette: &IndexedGamePalette) -> Result<(), Self::Error>;

    /// Clear the loading framebuffer to one indexed color.
    fn clear_loading_frame(&mut self, color: u8) -> Result<(), Self::Error>;

    /// Draw the authored loading label.
    fn draw_loading_text(&mut self, text: StartupLoadingText) -> Result<(), Self::Error>;

    /// Present the completed loading frame through the modern renderer.
    fn present_loading_frame(&mut self) -> Result<(), Self::Error>;

    /// Ensure the explicit writable root exists.
    ///
    /// Unlike rendering failures, this failure belongs to the recovered DOS
    /// filesystem operation and does not abort the startup catalog walk.
    fn ensure_write_directory(&mut self) -> Result<(), StartupFilesystemFailure>;

    /// Probe one authored name below the explicit writable root.
    fn writable_resource_exists(
        &mut self,
        resource: StartupWritableResourceId,
        name: &BloodResourceName,
    ) -> Result<bool, StartupFilesystemFailure>;

    /// Copy one missing resource from the configured source store to the writable root.
    fn copy_resource_to_writable(
        &mut self,
        resource: StartupWritableResourceId,
        name: &BloodResourceName,
    ) -> Result<StartupResourceCopyOutcome, StartupFilesystemFailure>;
}

/// Recoverable filesystem operation attempted during startup preparation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupFilesystemOperation {
    /// Create or validate the configured writable root.
    CreateWriteDirectory,
    /// Probe for one existing writable resource.
    ProbeWritableResource,
    /// Resolve and open one original source resource.
    OpenSourceResource,
    /// Create or truncate one writable destination resource.
    CreateDestinationResource,
    /// Transfer source bytes into an opened destination.
    CopyResourceData,
}

/// Host-side detail for one nonfatal recovered filesystem failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartupFilesystemFailure {
    /// Operation that failed.
    pub operation: StartupFilesystemOperation,
    /// Stable human-readable host diagnostic.
    pub message: String,
}

impl StartupFilesystemFailure {
    /// Construct a typed failure while retaining the host error text.
    pub fn new(operation: StartupFilesystemOperation, error: impl fmt::Display) -> Self {
        Self {
            operation,
            message: error.to_string(),
        }
    }
}

/// Result of a startup copy attempt that reached the source resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupResourceCopyOutcome {
    /// Nonempty source bytes were written to the destination.
    Copied,
    /// The native zero-length source guard skipped destination creation.
    SkippedEmptySource,
}

/// One recoverable startup failure annotated with its catalog owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartupPreparationDiagnostic {
    /// Catalog entry owning the operation, or `None` for write-root creation.
    pub resource: Option<StartupWritableResourceId>,
    /// Authored resource name, absent only for write-root creation.
    pub name: Option<BloodResourceName>,
    /// Operation and host-side failure detail.
    pub failure: StartupFilesystemFailure,
}

/// Completed startup probe and copy summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartupPreparationOutcome {
    /// Whether write-root creation reported success.
    pub write_directory_created: bool,
    /// Number of authored table entries probed.
    pub probed_resources: usize,
    /// Entries copied because their writable counterpart was absent.
    pub copied_resources: Vec<StartupWritableResourceId>,
    /// Recoverable filesystem failures in native execution order.
    pub diagnostics: Vec<StartupPreparationDiagnostic>,
}

/// Draw the loading screen and prepare every authored writable resource.
///
/// This translates `startup_loading_screen_and_write_directory_prepare` at
/// BLOODPRG file offset `0x0016A7`. The exact 125-entry order and duplicate
/// names are retained. Explicit source and writable roots replace DOS drives,
/// current-directory mutation, fixed path buffers, and the native `SS=DS`
/// separator dependency.
pub fn prepare_startup_writable_resources<Host: StartupPreparationHost>(
    catalog: &StartupWritableResourceCatalog,
    loading_palette: &IndexedGamePalette,
    host: &mut Host,
) -> Result<StartupPreparationOutcome, Host::Error> {
    host.publish_loading_palette(loading_palette)?;
    host.clear_loading_frame(STARTUP_LOADING_BACKGROUND_COLOR)?;
    host.draw_loading_text(StartupLoadingText {
        text: STARTUP_LOADING_TEXT,
        position: STARTUP_LOADING_TEXT_POSITION,
        color: STARTUP_LOADING_TEXT_COLOR,
        byte_limit: STARTUP_LOADING_TEXT_BYTE_LIMIT,
    })?;
    host.present_loading_frame()?;

    let mut diagnostics = Vec::new();
    let write_directory_created = match host.ensure_write_directory() {
        Ok(()) => true,
        Err(failure) => {
            diagnostics.push(StartupPreparationDiagnostic {
                resource: None,
                name: None,
                failure,
            });
            false
        }
    };
    let mut copied_resources = Vec::new();
    for (resource, name) in catalog.iter() {
        let resource_exists = match host.writable_resource_exists(resource, name) {
            Ok(resource_exists) => resource_exists,
            Err(failure) => {
                diagnostics.push(StartupPreparationDiagnostic {
                    resource: Some(resource),
                    name: Some(name.clone()),
                    failure,
                });
                false
            }
        };
        if resource_exists {
            continue;
        }
        match host.copy_resource_to_writable(resource, name) {
            Ok(StartupResourceCopyOutcome::Copied) => copied_resources.push(resource),
            Ok(StartupResourceCopyOutcome::SkippedEmptySource) => {}
            Err(failure) => diagnostics.push(StartupPreparationDiagnostic {
                resource: Some(resource),
                name: Some(name.clone()),
                failure,
            }),
        }
    }
    Ok(StartupPreparationOutcome {
        write_directory_created,
        probed_resources: catalog.len(),
        copied_resources,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::convert::Infallible;

    use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};
    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 5;
    const PALETTE_COMPONENT_MASK: usize = 63;
    const PALETTE_ENTRY_STEP: usize = 17;
    const PALETTE_CASE_STEP: usize = 29;
    const PALETTE_SEED: usize = 3;

    #[derive(Deserialize)]
    struct SequelCatalogOracle {
        names: Vec<Vec<u8>>,
        directory_enter_count: usize,
    }

    fn sequel_catalog_oracle() -> SequelCatalogOracle {
        serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/big_bug_bang_writable_catalog.json"
        ))
        .unwrap()
    }

    fn sequel_catalog_fixture() -> Vec<u8> {
        let mut bytes = vec![0; 0x10411];
        for (index, name) in sequel_catalog_oracle().names.iter().enumerate() {
            let start = 0xFA90 + index * 16;
            bytes[start..start + name.len()].copy_from_slice(name);
        }
        bytes
    }

    #[test]
    fn sequel_writable_catalog_matches_every_native_visit_and_duplicate() {
        let oracle = sequel_catalog_oracle();
        let catalog =
            StartupWritableResourceCatalog::decode_blood2pg(&sequel_catalog_fixture()).unwrap();
        assert_eq!(catalog.len(), 152);
        assert_eq!(
            catalog
                .iter()
                .map(|(_, name)| name.as_bytes().to_vec())
                .collect::<Vec<_>>(),
            oracle.names
        );
        let mut host = OracleHost {
            graphics: Vec::new(),
            mkdir_success: true,
            missing: BTreeSet::new(),
            probed: Vec::new(),
            copied: Vec::new(),
        };
        let outcome =
            prepare_startup_writable_resources(&catalog, &loading_palette(0), &mut host).unwrap();
        assert_eq!(host.probed.len(), oracle.directory_enter_count);
        assert_eq!(
            host.probed
                .iter()
                .map(|(_, name)| name.clone())
                .collect::<Vec<_>>(),
            oracle.names
        );
        assert!(host.copied.is_empty());
        assert!(outcome.diagnostics.is_empty());
        assert_eq!(outcome.probed_resources, 152);
        assert_eq!(oracle.names[6], oracle.names[7]);
        assert_eq!(oracle.names[5], b"blood.sav");
        assert_eq!(oracle.names[151], b"script17.dic");
    }

    #[test]
    fn sequel_writable_catalog_rejects_missing_terminators_and_bad_names() {
        let bytes = sequel_catalog_fixture();
        for length in [0, 0xFA90, bytes.len() - 1] {
            assert!(matches!(
                StartupWritableResourceCatalog::decode_blood2pg(&bytes[..length]),
                Err(StartupWritableCatalogError::ExecutableTooShort {
                    required: 0x10411,
                    ..
                })
            ));
        }
        let mut invalid = bytes.clone();
        invalid[0x10410] = 1;
        assert!(matches!(
            StartupWritableResourceCatalog::decode_blood2pg(&invalid),
            Err(StartupWritableCatalogError::MissingTableTerminator { offset: 0x10410 })
        ));
        let mut invalid = bytes.clone();
        invalid[0xFA90..0xFAA0].fill(b'x');
        assert!(matches!(
            StartupWritableResourceCatalog::decode_blood2pg(&invalid),
            Err(StartupWritableCatalogError::UnterminatedName { .. })
        ));
        let mut invalid = bytes;
        invalid[0xFA90] = 0;
        assert!(matches!(
            StartupWritableResourceCatalog::decode_blood2pg(&invalid),
            Err(StartupWritableCatalogError::InvalidName { .. })
        ));
    }

    struct SequelResourceHost {
        store: crate::assets::OriginalResourceStore,
        graphics: OracleHost,
    }

    impl StartupPreparationHost for SequelResourceHost {
        type Error = Infallible;

        fn publish_loading_palette(
            &mut self,
            palette: &IndexedGamePalette,
        ) -> Result<(), Self::Error> {
            self.graphics.publish_loading_palette(palette)
        }
        fn clear_loading_frame(&mut self, color: u8) -> Result<(), Self::Error> {
            self.graphics.clear_loading_frame(color)
        }
        fn draw_loading_text(&mut self, text: StartupLoadingText) -> Result<(), Self::Error> {
            self.graphics.draw_loading_text(text)
        }
        fn present_loading_frame(&mut self) -> Result<(), Self::Error> {
            self.graphics.present_loading_frame()
        }
        fn ensure_write_directory(&mut self) -> Result<(), StartupFilesystemFailure> {
            std::fs::create_dir_all(self.store.writable_root()).map_err(|error| {
                StartupFilesystemFailure::new(
                    StartupFilesystemOperation::CreateWriteDirectory,
                    error.to_string(),
                )
            })
        }
        fn writable_resource_exists(
            &mut self,
            resource: StartupWritableResourceId,
            name: &BloodResourceName,
        ) -> Result<bool, StartupFilesystemFailure> {
            self.graphics
                .probed
                .push((resource.index(), name.as_bytes().to_vec()));
            self.store.writable_resource_exists(name).map_err(|error| {
                StartupFilesystemFailure::new(
                    StartupFilesystemOperation::ProbeWritableResource,
                    error.to_string(),
                )
            })
        }
        fn copy_resource_to_writable(
            &mut self,
            resource: StartupWritableResourceId,
            name: &BloodResourceName,
        ) -> Result<StartupResourceCopyOutcome, StartupFilesystemFailure> {
            self.graphics
                .copied
                .push((resource.index(), name.as_bytes().to_vec()));
            self.store
                .copy_to_loose(name, name)
                .map(|copied| {
                    if copied {
                        StartupResourceCopyOutcome::Copied
                    } else {
                        StartupResourceCopyOutcome::SkippedEmptySource
                    }
                })
                .map_err(|error| {
                    use crate::assets::OriginalResourceCopyOperation;
                    let operation = match error.operation() {
                        OriginalResourceCopyOperation::OpenSource => {
                            StartupFilesystemOperation::OpenSourceResource
                        }
                        OriginalResourceCopyOperation::CreateDestination => {
                            StartupFilesystemOperation::CreateDestinationResource
                        }
                        OriginalResourceCopyOperation::WriteDestination => {
                            StartupFilesystemOperation::CopyResourceData
                        }
                    };
                    StartupFilesystemFailure::new(operation, error.to_string())
                })
        }
    }

    #[test]
    #[ignore = "requires original Big Bug Bang executable and imported resources"]
    fn sequel_writable_catalog_prepares_real_resources_without_borrowing_a_save() {
        use crate::assets::OriginalResourceStore;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/imported-assets/resources");
        let executable = std::fs::read(root.join("../../disc/BLOOD2PG.EXE")).unwrap();
        let game = crate::game::GameVariant::BigBugBang;
        let catalog = game.decode_writable_resource_catalog(&executable).unwrap();
        assert_eq!(
            catalog
                .iter()
                .map(|(_, name)| name.as_bytes().to_vec())
                .collect::<Vec<_>>(),
            sequel_catalog_oracle().names
        );
        let palette = game.decode_default_vga_palette(&executable).unwrap();
        let source = OriginalResourceStore::new(root.clone(), None, [], true);
        let original = catalog
            .iter()
            .filter(|(id, _)| id.index() != 5)
            .map(|(_, name)| (name.clone(), source.load(name).unwrap()))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(original.len(), 150);
        struct TemporaryRoot(std::path::PathBuf);
        impl Drop for TemporaryRoot {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let path = std::env::temp_dir().join(format!(
            "bbb-startup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        let temporary = TemporaryRoot(path);
        let store =
            OriginalResourceStore::with_writable_root(root, temporary.0.clone(), None, [], true);
        let preserved = BloodResourceName::new(b"script1.var").unwrap();
        store
            .write_loose(&preserved, b"existing player state")
            .unwrap();
        let mut host = SequelResourceHost {
            store,
            graphics: OracleHost {
                graphics: Vec::new(),
                mkdir_success: true,
                missing: BTreeSet::new(),
                probed: Vec::new(),
                copied: Vec::new(),
            },
        };
        let outcome = prepare_startup_writable_resources(&catalog, &palette, &mut host).unwrap();
        assert_eq!(outcome.probed_resources, 152);
        assert_eq!(outcome.copied_resources.len(), 149);
        assert_eq!(outcome.diagnostics.len(), 1);
        assert_eq!(outcome.diagnostics[0].resource.unwrap().index(), 5);
        assert_eq!(
            outcome.diagnostics[0].failure.operation,
            StartupFilesystemOperation::OpenSourceResource
        );
        for (name, bytes) in &original {
            assert_eq!(
                source.load(name).unwrap(),
                *bytes,
                "source changed: {name:?}"
            );
            let expected = if *name == preserved {
                b"existing player state".as_slice()
            } else {
                bytes.as_ref()
            };
            assert_eq!(
                host.store.load_writable(name).unwrap().as_ref(),
                expected,
                "{name:?}"
            );
        }
        let save = BloodResourceName::new(b"blood.sav").unwrap();
        assert!(!host.store.writable_resource_exists(&save).unwrap());
        assert!(source.load(&save).is_err());
        let again = prepare_startup_writable_resources(&catalog, &palette, &mut host).unwrap();
        assert!(again.copied_resources.is_empty());
        assert_eq!(again.diagnostics.len(), 1);
        assert_eq!(again.diagnostics[0].resource.unwrap().index(), 5);
        assert_eq!(std::fs::read_dir(&temporary.0).unwrap().count(), 150);
    }

    #[derive(Deserialize)]
    struct GraphicsCall {
        call: String,
        palette_sha256: Option<String>,
        color: Option<u8>,
        text: Option<String>,
        x: Option<u16>,
        y: Option<u16>,
        color_and_limit: Option<u16>,
    }

    #[derive(Deserialize)]
    struct StartupOracle {
        case: String,
        graphics_calls: Vec<GraphicsCall>,
        mkdir_success: bool,
        find_count: usize,
        find_sequence_sha256: String,
        directory_enter_count: usize,
        missing_indices: Vec<usize>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum RecordedGraphicsCall {
        Palette(String),
        Clear(u8),
        Text(StartupLoadingText),
        Present,
    }

    struct OracleHost {
        graphics: Vec<RecordedGraphicsCall>,
        mkdir_success: bool,
        missing: BTreeSet<usize>,
        probed: Vec<(usize, Vec<u8>)>,
        copied: Vec<(usize, Vec<u8>)>,
    }

    impl StartupPreparationHost for OracleHost {
        type Error = Infallible;

        fn publish_loading_palette(
            &mut self,
            palette: &IndexedGamePalette,
        ) -> Result<(), Self::Error> {
            let mut hasher = Sha256::new();
            for color in palette {
                hasher.update(color);
            }
            self.graphics.push(RecordedGraphicsCall::Palette(format!(
                "{:x}",
                hasher.finalize()
            )));
            Ok(())
        }

        fn clear_loading_frame(&mut self, color: u8) -> Result<(), Self::Error> {
            self.graphics.push(RecordedGraphicsCall::Clear(color));
            Ok(())
        }

        fn draw_loading_text(&mut self, text: StartupLoadingText) -> Result<(), Self::Error> {
            self.graphics.push(RecordedGraphicsCall::Text(text));
            Ok(())
        }

        fn present_loading_frame(&mut self) -> Result<(), Self::Error> {
            self.graphics.push(RecordedGraphicsCall::Present);
            Ok(())
        }

        fn ensure_write_directory(&mut self) -> Result<(), StartupFilesystemFailure> {
            if self.mkdir_success {
                Ok(())
            } else {
                Err(StartupFilesystemFailure::new(
                    StartupFilesystemOperation::CreateWriteDirectory,
                    "oracle mkdir failure",
                ))
            }
        }

        fn writable_resource_exists(
            &mut self,
            resource: StartupWritableResourceId,
            name: &BloodResourceName,
        ) -> Result<bool, StartupFilesystemFailure> {
            self.probed
                .push((resource.index(), name.as_bytes().to_vec()));
            Ok(!self.missing.contains(&resource.index()))
        }

        fn copy_resource_to_writable(
            &mut self,
            resource: StartupWritableResourceId,
            name: &BloodResourceName,
        ) -> Result<StartupResourceCopyOutcome, StartupFilesystemFailure> {
            self.copied
                .push((resource.index(), name.as_bytes().to_vec()));
            Ok(StartupResourceCopyOutcome::Copied)
        }
    }

    #[test]
    fn startup_preparation_matches_every_original_oracle_vector() {
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let catalog = StartupWritableResourceCatalog::decode_bloodprg(executable).unwrap();
        let vectors: Vec<StartupOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_16a7_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);
        assert_eq!(catalog.len(), STARTUP_WRITABLE_RESOURCE_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let palette = loading_palette(case_index);
            let mut host = OracleHost {
                graphics: Vec::new(),
                mkdir_success: vector.mkdir_success,
                missing: vector.missing_indices.iter().copied().collect(),
                probed: Vec::new(),
                copied: Vec::new(),
            };

            let outcome =
                prepare_startup_writable_resources(&catalog, &palette, &mut host).unwrap();

            assert_eq!(
                host.graphics,
                expected_graphics(&vector.graphics_calls),
                "{}",
                vector.case
            );
            assert_eq!(outcome.write_directory_created, vector.mkdir_success);
            assert_eq!(
                outcome.diagnostics.len(),
                usize::from(!vector.mkdir_success)
            );
            assert_eq!(outcome.probed_resources, vector.find_count);
            assert_eq!(host.probed.len(), vector.directory_enter_count);
            assert_eq!(
                resource_sequence_hash(host.probed.iter().map(|(_, name)| name.as_slice())),
                vector.find_sequence_sha256,
                "{}",
                vector.case
            );
            assert_eq!(
                outcome
                    .copied_resources
                    .iter()
                    .map(|resource| resource.index())
                    .collect::<Vec<_>>(),
                vector.missing_indices,
                "{}",
                vector.case
            );
            assert_eq!(
                host.copied
                    .iter()
                    .map(|(index, _name)| *index)
                    .collect::<Vec<_>>(),
                vector.missing_indices,
                "{}",
                vector.case
            );
        }
    }

    #[derive(Default)]
    struct FailureOwnershipHost {
        probed: Vec<usize>,
        copied: Vec<usize>,
    }

    impl StartupPreparationHost for FailureOwnershipHost {
        type Error = Infallible;

        fn publish_loading_palette(
            &mut self,
            _palette: &IndexedGamePalette,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn clear_loading_frame(&mut self, _color: u8) -> Result<(), Self::Error> {
            Ok(())
        }

        fn draw_loading_text(&mut self, _text: StartupLoadingText) -> Result<(), Self::Error> {
            Ok(())
        }

        fn present_loading_frame(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }

        fn ensure_write_directory(&mut self) -> Result<(), StartupFilesystemFailure> {
            Err(StartupFilesystemFailure::new(
                StartupFilesystemOperation::CreateWriteDirectory,
                "injected mkdir failure",
            ))
        }

        fn writable_resource_exists(
            &mut self,
            resource: StartupWritableResourceId,
            _name: &BloodResourceName,
        ) -> Result<bool, StartupFilesystemFailure> {
            self.probed.push(resource.index());
            if resource.index() == 0 {
                Err(StartupFilesystemFailure::new(
                    StartupFilesystemOperation::ProbeWritableResource,
                    "injected stat failure",
                ))
            } else {
                Ok(resource.index() == 4)
            }
        }

        fn copy_resource_to_writable(
            &mut self,
            resource: StartupWritableResourceId,
            _name: &BloodResourceName,
        ) -> Result<StartupResourceCopyOutcome, StartupFilesystemFailure> {
            self.copied.push(resource.index());
            let operation = match resource.index() {
                0 => Some(StartupFilesystemOperation::OpenSourceResource),
                1 => Some(StartupFilesystemOperation::CreateDestinationResource),
                2 => Some(StartupFilesystemOperation::CopyResourceData),
                _ => None,
            };
            match operation {
                Some(operation) => Err(StartupFilesystemFailure::new(
                    operation,
                    "injected copy-stage failure",
                )),
                None => Ok(StartupResourceCopyOutcome::Copied),
            }
        }
    }

    #[test]
    fn filesystem_failures_remain_local_and_later_catalog_entries_are_attempted() {
        let names = [
            b"ZERO.DAT".as_slice(),
            b"ONE.DAT",
            b"TWO.DAT",
            b"THREE.DAT",
            b"FOUR.DAT",
        ]
        .into_iter()
        .map(|name| BloodResourceName::new(name).unwrap())
        .collect::<Vec<_>>()
        .into_boxed_slice();
        let catalog = StartupWritableResourceCatalog { names };
        let mut host = FailureOwnershipHost::default();

        let outcome = prepare_startup_writable_resources(
            &catalog,
            &[[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
            &mut host,
        )
        .unwrap();

        assert!(!outcome.write_directory_created);
        assert_eq!(outcome.probed_resources, catalog.len());
        assert_eq!(host.probed, vec![0, 1, 2, 3, 4]);
        assert_eq!(host.copied, vec![0, 1, 2, 3]);
        assert_eq!(
            outcome
                .copied_resources
                .iter()
                .map(|resource| resource.index())
                .collect::<Vec<_>>(),
            vec![3]
        );
        assert_eq!(
            outcome
                .diagnostics
                .iter()
                .map(|diagnostic| (
                    diagnostic.resource.map(StartupWritableResourceId::index),
                    diagnostic.failure.operation,
                ))
                .collect::<Vec<_>>(),
            vec![
                (None, StartupFilesystemOperation::CreateWriteDirectory),
                (Some(0), StartupFilesystemOperation::ProbeWritableResource),
                (Some(0), StartupFilesystemOperation::OpenSourceResource),
                (
                    Some(1),
                    StartupFilesystemOperation::CreateDestinationResource
                ),
                (Some(2), StartupFilesystemOperation::CopyResourceData),
            ]
        );
        assert_eq!(
            outcome.diagnostics[1].name.as_ref().unwrap().as_bytes(),
            b"ZERO.DAT"
        );
    }

    #[test]
    fn malformed_catalog_bounds_and_terminators_are_rejected() {
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        assert!(matches!(
            StartupWritableResourceCatalog::decode_bloodprg(
                &executable[..BLOODPRG_WRITABLE_RESOURCE_CATALOG_FILE_OFFSET]
            ),
            Err(StartupWritableCatalogError::ExecutableTooShort { .. })
        ));

        let required = BLOODPRG_WRITABLE_RESOURCE_CATALOG_FILE_OFFSET
            + STARTUP_WRITABLE_RESOURCE_COUNT * WRITABLE_RESOURCE_NAME_FIELD_SIZE;
        let mut malformed = executable[..required].to_vec();
        malformed[BLOODPRG_WRITABLE_RESOURCE_CATALOG_FILE_OFFSET
            ..BLOODPRG_WRITABLE_RESOURCE_CATALOG_FILE_OFFSET + WRITABLE_RESOURCE_NAME_FIELD_SIZE]
            .fill(b'X');
        assert!(matches!(
            StartupWritableResourceCatalog::decode_bloodprg(&malformed),
            Err(StartupWritableCatalogError::UnterminatedName { .. })
        ));
    }

    fn loading_palette(case_index: usize) -> IndexedGamePalette {
        let mut palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        for (flat_index, component) in palette.iter_mut().flatten().enumerate() {
            *component =
                ((flat_index * PALETTE_ENTRY_STEP + case_index * PALETTE_CASE_STEP + PALETTE_SEED)
                    & PALETTE_COMPONENT_MASK) as u8;
        }
        palette
    }

    fn expected_graphics(calls: &[GraphicsCall]) -> Vec<RecordedGraphicsCall> {
        calls
            .iter()
            .map(|call| match call.call.as_str() {
                "vga_palette_write" => {
                    RecordedGraphicsCall::Palette(call.palette_sha256.clone().unwrap())
                }
                "blit_fill_row_5221" => RecordedGraphicsCall::Clear(call.color.unwrap()),
                "font8x8_text_draw_display" => {
                    let packed = call.color_and_limit.unwrap();
                    assert_eq!(call.text.as_deref(), Some("LOADING"));
                    RecordedGraphicsCall::Text(StartupLoadingText {
                        text: STARTUP_LOADING_TEXT,
                        position: [call.x.unwrap(), call.y.unwrap()],
                        color: packed as u8,
                        byte_limit: usize::from(packed >> u8::BITS),
                    })
                }
                "chunky_to_planar_framebuffer" => RecordedGraphicsCall::Present,
                other => panic!("unknown startup graphics call {other}"),
            })
            .collect()
    }

    fn resource_sequence_hash<'a>(names: impl Iterator<Item = &'a [u8]>) -> String {
        let mut hasher = Sha256::new();
        for (index, name) in names.enumerate() {
            if index != 0 {
                hasher.update([u8::MIN]);
            }
            hasher.update(name);
        }
        format!("{:x}", hasher.finalize())
    }
}
