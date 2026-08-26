//! Typed original-resource catalog and runtime cache.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use commander_blood_formats::archive::{BloodArchiveError, BloodResourceName};

use crate::assets::OriginalResourceStore;

use super::IndexedGamePalette;
use super::presentation_resource::{PaletteBlockDecodeError, decode_palette_blocks};

/// File position of the fixed-width resource-name table in `BLOODPRG.EXE`.
pub const BLOODPRG_RESOURCE_CATALOG_FILE_OFFSET: usize = 0x00CDF4;
/// Number of resource names authored in the original executable.
pub const ORIGINAL_RESOURCE_COUNT: usize = 95;
/// Paragraph-size rounding applied to original resource allocations.
pub const ORIGINAL_RESOURCE_ALLOCATION_ALIGNMENT: usize = 16;

const RESOURCE_NAME_FIELD_SIZE: usize = 16;
const RESOURCE_FILE_HEADER_SIZE: usize = 2;
const RESOURCE_PALETTE_PREAMBLE_FLAG: u16 = 2;

/// Stable zero-based identifier from the original resource catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(u16);

impl ResourceId {
    /// Construct an identifier; catalog lookup validates whether it is authored.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Return the original numeric identifier.
    pub const fn value(self) -> u16 {
        self.0
    }

    const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Ordered mapping from original resource identifiers to validated filenames.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalResourceCatalog {
    names: Box<[BloodResourceName]>,
}

impl OriginalResourceCatalog {
    /// Construct a catalog from an already validated authored sequence.
    pub fn new(names: impl IntoIterator<Item = BloodResourceName>) -> Self {
        Self {
            names: names.into_iter().collect(),
        }
    }

    /// Decode all 95 fixed-width names from the original executable image.
    ///
    /// The file position is serialized-format evidence only. Runtime lookups use
    /// [`ResourceId`] and ordinary owned collections.
    pub fn decode_bloodprg(executable: &[u8]) -> Result<Self, ResourceCacheError> {
        let required = BLOODPRG_RESOURCE_CATALOG_FILE_OFFSET
            + ORIGINAL_RESOURCE_COUNT * RESOURCE_NAME_FIELD_SIZE;
        if executable.len() < required {
            return Err(ResourceCacheError::ExecutableTooShort {
                required,
                actual: executable.len(),
            });
        }

        let mut names = Vec::with_capacity(ORIGINAL_RESOURCE_COUNT);
        for resource_index in 0..ORIGINAL_RESOURCE_COUNT {
            let start =
                BLOODPRG_RESOURCE_CATALOG_FILE_OFFSET + resource_index * RESOURCE_NAME_FIELD_SIZE;
            let field = &executable[start..start + RESOURCE_NAME_FIELD_SIZE];
            let name_length = field.iter().position(|byte| *byte == u8::MIN).ok_or(
                ResourceCacheError::UnterminatedCatalogName {
                    resource: ResourceId::new(resource_index as u16),
                },
            )?;
            let resource = ResourceId::new(resource_index as u16);
            let name = BloodResourceName::new(&field[..name_length])
                .map_err(|source| ResourceCacheError::InvalidCatalogName { resource, source })?;
            names.push(name);
        }
        Ok(Self::new(names))
    }

    /// Return the number of authored names in this catalog.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Return whether the catalog contains no authored names.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Resolve an identifier to its authored filename.
    pub fn name(&self, resource: ResourceId) -> Option<&BloodResourceName> {
        self.names.get(resource.index())
    }
}

/// Whether a cache load read new bytes or reused the existing owned resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceLoadStatus {
    /// The requested bytes were loaded and inserted during this call.
    LoadedNow,
    /// The requested identifier was already resident.
    AlreadyLoaded,
}

/// Destination policy for a palette-prefixed resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteResourceTarget {
    /// Retain the processed bytes in the resource cache.
    Cached,
    /// Return independently owned processed bytes to the caller.
    Direct,
}

/// Result storage selected by [`OriginalResourceCache::load_palette_resource`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteResourceStorage {
    /// The resource is available through its catalog identifier.
    Cached(ResourceLoadStatus),
    /// The caller owns the processed bytes without a cache entry.
    Direct(Box<[u8]>),
}

/// Observable result of loading one palette-prefixed resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteResourceLoadOutcome {
    /// Where the processed bytes were retained.
    pub storage: PaletteResourceStorage,
    /// Whether the file requested a live-palette update.
    pub palette_changed: bool,
}

#[derive(Clone, Debug)]
struct CachedResource {
    bytes: Box<[u8]>,
    allocation_byte_count: usize,
}

/// Owned runtime resources indexed by their original stable identifiers.
#[derive(Clone, Debug, Default)]
pub struct OriginalResourceCache {
    entries: BTreeMap<ResourceId, CachedResource>,
}

impl OriginalResourceCache {
    /// Construct an empty resource cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load one catalog resource or report that its existing bytes were reused.
    ///
    /// This translates `resource_load_by_id` at BLOODPRG file offset
    /// `0x00287B`. A single owned byte allocation replaces the native allocator,
    /// while zero-length files retain the original failure result.
    pub fn load_by_id(
        &mut self,
        store: &OriginalResourceStore,
        catalog: &OriginalResourceCatalog,
        resource: ResourceId,
    ) -> Result<ResourceLoadStatus, ResourceCacheError> {
        let name = catalog_name(catalog, resource)?;
        let byte_count = store
            .resource_len(name)
            .map_err(|source| ResourceCacheError::ResourceRead { resource, source })?;
        if byte_count == usize::MIN {
            return Err(ResourceCacheError::EmptyResource(resource));
        }
        if self.entries.contains_key(&resource) {
            return Ok(ResourceLoadStatus::AlreadyLoaded);
        }

        let bytes = store
            .load(name)
            .map_err(|source| ResourceCacheError::ResourceRead { resource, source })?;
        if bytes.is_empty() {
            return Err(ResourceCacheError::EmptyResource(resource));
        }
        self.insert(resource, bytes)?;
        Ok(ResourceLoadStatus::LoadedNow)
    }

    /// Load and process the palette preamble used by original sprite resources.
    ///
    /// This translates `resource_named_file_load` at BLOODPRG file offset
    /// `0x003FC7`. A typed target replaces the high bit formerly packed into the
    /// numeric identifier. Palette blocks are applied before a cache hit is
    /// reported, matching the native ordering.
    pub fn load_palette_resource(
        &mut self,
        store: &OriginalResourceStore,
        catalog: &OriginalResourceCatalog,
        resource: ResourceId,
        target: PaletteResourceTarget,
        live_palette: &mut IndexedGamePalette,
    ) -> Result<PaletteResourceLoadOutcome, ResourceCacheError> {
        let name = catalog_name(catalog, resource)?;
        let source = store
            .load(name)
            .map_err(|source| ResourceCacheError::ResourceRead { resource, source })?;
        let (bytes, updated_palette, palette_changed) =
            decode_palette_resource(resource, &source, live_palette)?;

        let storage = match target {
            PaletteResourceTarget::Cached => {
                if self.entries.contains_key(&resource) {
                    PaletteResourceStorage::Cached(ResourceLoadStatus::AlreadyLoaded)
                } else {
                    self.insert(resource, bytes)?;
                    PaletteResourceStorage::Cached(ResourceLoadStatus::LoadedNow)
                }
            }
            PaletteResourceTarget::Direct => PaletteResourceStorage::Direct(bytes),
        };
        *live_palette = updated_palette;
        Ok(PaletteResourceLoadOutcome {
            storage,
            palette_changed,
        })
    }

    /// Release a loaded identifier and return whether an entry existed.
    ///
    /// This is the flat-data behavior of `resource_release` at BLOODPRG file
    /// offset `0x005288`. Removing an owned map entry also replaces the native
    /// `resource_free_inner` pool compaction at `0x00529C`.
    pub fn release(&mut self, resource: ResourceId) -> bool {
        self.entries.remove(&resource).is_some()
    }

    /// Borrow the exact loaded file bytes for an identifier.
    ///
    /// This translates `resource_handle_resolve` at BLOODPRG file offset
    /// `0x005320`; callers receive a checked slice instead of machine state.
    pub fn resolve(&self, resource: ResourceId) -> Option<&[u8]> {
        self.entries.get(&resource).map(|entry| &*entry.bytes)
    }

    /// Return the original allocator's 16-byte-rounded size metadata.
    ///
    /// This translates `resource_get_field4` at BLOODPRG file offset
    /// `0x00533C`. Save-game code queries this value, so the observable rounding
    /// remains even though Rust stores the exact file bytes independently.
    pub fn allocation_byte_count(&self, resource: ResourceId) -> Option<usize> {
        self.entries
            .get(&resource)
            .map(|entry| entry.allocation_byte_count)
    }

    /// Return whether an identifier currently has owned bytes.
    pub fn is_loaded(&self, resource: ResourceId) -> bool {
        self.entries.contains_key(&resource)
    }

    fn insert(&mut self, resource: ResourceId, bytes: Box<[u8]>) -> Result<(), ResourceCacheError> {
        let allocation_byte_count = rounded_allocation_byte_count(bytes.len())?;
        self.entries.insert(
            resource,
            CachedResource {
                bytes,
                allocation_byte_count,
            },
        );
        Ok(())
    }
}

/// Invalid catalog data, resource request, or palette-prefixed payload.
#[derive(Debug)]
pub enum ResourceCacheError {
    /// The executable ends before the complete authored resource-name table.
    ExecutableTooShort {
        /// Minimum byte count required by the table.
        required: usize,
        /// Actual executable byte count.
        actual: usize,
    },
    /// One fixed-width catalog field did not contain a terminator.
    UnterminatedCatalogName {
        /// Identifier owning the malformed field.
        resource: ResourceId,
    },
    /// One catalog field was not a valid original resource name.
    InvalidCatalogName {
        /// Identifier owning the malformed field.
        resource: ResourceId,
        /// Name validation failure.
        source: BloodArchiveError,
    },
    /// The requested identifier is outside the decoded catalog.
    UnknownResource(ResourceId),
    /// The requested original file contained no bytes.
    EmptyResource(ResourceId),
    /// Loading or measuring the selected host resource failed.
    ResourceRead {
        /// Identifier whose file operation failed.
        resource: ResourceId,
        /// Underlying host or archive error.
        source: anyhow::Error,
    },
    /// Rounding the loaded byte count exceeded the host collection limit.
    ResourceTooLarge(usize),
    /// A palette-prefixed file does not contain its initial word.
    TruncatedResourceHeader {
        /// Identifier owning the malformed file.
        resource: ResourceId,
        /// Available bytes.
        available: usize,
    },
    /// A palette block header ended before its complete word.
    TruncatedPaletteBlockHeader {
        /// Identifier owning the malformed file.
        resource: ResourceId,
        /// Byte position of the incomplete header.
        position: usize,
    },
    /// A palette block extends beyond the 256-color live palette.
    PaletteBlockOutOfRange {
        /// Identifier owning the malformed file.
        resource: ResourceId,
        /// First requested color.
        first_color: usize,
        /// Requested color count.
        color_count: usize,
    },
    /// A palette block contains fewer RGB bytes than its header requests.
    TruncatedPaletteBlock {
        /// Identifier owning the malformed file.
        resource: ResourceId,
        /// Requested component byte count.
        required: usize,
        /// Available component byte count.
        available: usize,
    },
}

impl fmt::Display for ResourceCacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid Commander Blood resource operation: {self:?}"
        )
    }
}

impl Error for ResourceCacheError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidCatalogName { source, .. } => Some(source),
            Self::ResourceRead { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

fn catalog_name(
    catalog: &OriginalResourceCatalog,
    resource: ResourceId,
) -> Result<&BloodResourceName, ResourceCacheError> {
    catalog
        .name(resource)
        .ok_or(ResourceCacheError::UnknownResource(resource))
}

fn rounded_allocation_byte_count(byte_count: usize) -> Result<usize, ResourceCacheError> {
    byte_count
        .checked_add(ORIGINAL_RESOURCE_ALLOCATION_ALIGNMENT - 1)
        .map(|value| {
            value / ORIGINAL_RESOURCE_ALLOCATION_ALIGNMENT * ORIGINAL_RESOURCE_ALLOCATION_ALIGNMENT
        })
        .ok_or(ResourceCacheError::ResourceTooLarge(byte_count))
}

fn decode_palette_resource(
    resource: ResourceId,
    source: &[u8],
    live_palette: &IndexedGamePalette,
) -> Result<(Box<[u8]>, IndexedGamePalette, bool), ResourceCacheError> {
    let header_bytes = source.get(..RESOURCE_FILE_HEADER_SIZE).ok_or(
        ResourceCacheError::TruncatedResourceHeader {
            resource,
            available: source.len(),
        },
    )?;
    let header = u16::from_le_bytes(
        header_bytes
            .try_into()
            .expect("validated two-byte resource header"),
    );
    if header & RESOURCE_PALETTE_PREAMBLE_FLAG == u16::MIN {
        return Ok((Box::from(source), *live_palette, false));
    }

    let mut palette = *live_palette;
    let cursor = decode_palette_blocks(source, RESOURCE_FILE_HEADER_SIZE, &mut palette).map_err(
        |source| match source {
            PaletteBlockDecodeError::TruncatedHeader { position } => {
                ResourceCacheError::TruncatedPaletteBlockHeader { resource, position }
            }
            PaletteBlockDecodeError::ColorsOutOfRange {
                first_color,
                color_count,
            } => ResourceCacheError::PaletteBlockOutOfRange {
                resource,
                first_color,
                color_count,
            },
            PaletteBlockDecodeError::TruncatedComponents {
                required,
                available,
            } => ResourceCacheError::TruncatedPaletteBlock {
                resource,
                required,
                available,
            },
        },
    )?;

    let mut bytes = Vec::with_capacity(RESOURCE_FILE_HEADER_SIZE + source.len() - cursor);
    bytes.extend_from_slice(header_bytes);
    bytes.extend_from_slice(&source[cursor..]);
    Ok((bytes.into_boxed_slice(), palette, true))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};
    use serde::Deserialize;

    use super::*;

    const FIRST_RESOURCE_ID: ResourceId = ResourceId::new(0);
    const VENUSIA_RESOURCE_ID: ResourceId = ResourceId::new(25);
    const PTERRA_RESOURCE_ID: ResourceId = ResourceId::new(35);
    const FINAL_RESOURCE_ID: ResourceId = ResourceId::new(94);
    const CACHE_TEST_RESOURCE_ID: ResourceId = ResourceId::new(1);
    const CACHE_TEST_PAYLOAD_LENGTH: usize = 17;
    const CACHE_TEST_ALLOCATION_LENGTH: usize = 32;
    const PALETTE_TEST_RESOURCE_ID: ResourceId = ResourceId::new(2);
    const PALETTE_TEST_FIRST_COLOR: usize = 2;
    const PALETTE_TEST_COLOR_COUNT: usize = 2;
    const PALETTE_TEST_HEADER: u16 = 0x5372;
    const PALETTE_TEST_BLOCK_TERMINATOR: u16 = u16::MAX;
    const PALETTE_TEST_PAYLOAD: &[u8] = b"sprite-body";
    const NAMED_RESOURCE_ORACLE_VECTOR_COUNT: usize = 8;
    const NAMED_RESOURCE_ORACLE_SUCCESS_COUNT: usize = 6;
    const NAMED_RESOURCE_ORACLE_HOST_FAILURE_COUNT: usize = 2;
    const ORIGINAL_DIRECT_RESOURCE_FLAG: u16 = 0x8000;
    static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(u64::MIN);

    #[derive(Deserialize)]
    struct NamedResourceOracle {
        name: String,
        resource_id: u16,
        catalog_resource_id: u16,
        mode: String,
        success: bool,
        allocation_status: Option<i16>,
        source_hex: String,
        palette_before_hex: String,
        palette_after_hex: String,
        processed_resource_hex: String,
    }

    struct TemporaryResourceRoot(PathBuf);

    impl TemporaryResourceRoot {
        fn create() -> Self {
            let sequence = TEMPORARY_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "commander-blood-cache-test-{}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TemporaryResourceRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn resource_name(name: impl AsRef<[u8]>) -> BloodResourceName {
        BloodResourceName::new(name).unwrap()
    }

    fn test_catalog() -> OriginalResourceCatalog {
        OriginalResourceCatalog::new([
            resource_name("UNUSED.DAT"),
            resource_name("CACHE.DAT"),
            resource_name("PALETTE.DAT"),
        ])
    }

    fn loose_store(root: &TemporaryResourceRoot) -> OriginalResourceStore {
        OriginalResourceStore::new(root.0.clone(), None, [], true)
    }

    fn decode_hex(encoded: &str) -> Vec<u8> {
        encoded
            .as_bytes()
            .chunks_exact(RESOURCE_FILE_HEADER_SIZE)
            .map(|digits| {
                let digits = std::str::from_utf8(digits).unwrap();
                u8::from_str_radix(digits, 16).unwrap()
            })
            .collect()
    }

    fn palette_from_hex(encoded: &str) -> IndexedGamePalette {
        let bytes = decode_hex(encoded);
        assert_eq!(bytes.len(), PALETTE_ENTRY_COUNT * RGB_COMPONENT_COUNT);
        let mut palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        for (destination, source) in palette
            .iter_mut()
            .zip(bytes.chunks_exact(RGB_COMPONENT_COUNT))
        {
            destination.copy_from_slice(source);
        }
        palette
    }

    fn catalog_with_resource(
        resource: ResourceId,
        name: &BloodResourceName,
    ) -> OriginalResourceCatalog {
        let mut names = vec![resource_name("UNUSED.DAT"); resource.index() + 1];
        names[resource.index()] = name.clone();
        OriginalResourceCatalog::new(names)
    }

    fn palette_resource(block_colors: &[u8]) -> Box<[u8]> {
        let mut source = Vec::new();
        source.extend_from_slice(&PALETTE_TEST_HEADER.to_le_bytes());
        source.extend_from_slice(&[
            PALETTE_TEST_FIRST_COLOR as u8,
            PALETTE_TEST_COLOR_COUNT as u8,
        ]);
        source.extend_from_slice(block_colors);
        source.extend_from_slice(&PALETTE_TEST_BLOCK_TERMINATOR.to_le_bytes());
        source.extend_from_slice(PALETTE_TEST_PAYLOAD);
        source.into_boxed_slice()
    }

    #[test]
    fn executable_catalog_recovers_every_authored_resource_name() {
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let catalog = OriginalResourceCatalog::decode_bloodprg(executable).unwrap();

        assert_eq!(catalog.len(), ORIGINAL_RESOURCE_COUNT);
        assert_eq!(
            catalog.name(FIRST_RESOURCE_ID).unwrap().as_bytes(),
            b"fupcom.spr"
        );
        assert_eq!(
            catalog.name(VENUSIA_RESOURCE_ID).unwrap().as_bytes(),
            b"venusia.ext"
        );
        assert_eq!(
            catalog.name(PTERRA_RESOURCE_ID).unwrap().as_bytes(),
            b"pterra.ext"
        );
        assert_eq!(
            catalog.name(FINAL_RESOURCE_ID).unwrap().as_bytes(),
            b"ondoya.ext"
        );
    }

    #[test]
    fn palette_loader_matches_every_applicable_native_oracle_vector() {
        let vectors: Vec<NamedResourceOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_3fc7_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), NAMED_RESOURCE_ORACLE_VECTOR_COUNT);

        let mut successes = usize::MIN;
        let mut host_failures = usize::MIN;
        for vector in vectors {
            let root = TemporaryResourceRoot::create();
            let store = loose_store(&root);
            let name = resource_name("RESOURCE.DAT");
            let resource = ResourceId::new(vector.catalog_resource_id);
            let catalog = catalog_with_resource(resource, &name);
            let source = decode_hex(&vector.source_hex);
            let expected_bytes = decode_hex(&vector.processed_resource_hex);
            let mut palette = palette_from_hex(&vector.palette_before_hex);
            let palette_before = palette;
            let mut cache = OriginalResourceCache::new();

            assert_eq!(
                vector.resource_id & !ORIGINAL_DIRECT_RESOURCE_FLAG,
                vector.catalog_resource_id,
                "{}",
                vector.name
            );
            if !vector.success {
                let result = cache.load_palette_resource(
                    &store,
                    &catalog,
                    resource,
                    PaletteResourceTarget::Cached,
                    &mut palette,
                );
                assert!(result.is_err(), "{}", vector.name);
                assert_eq!(palette, palette_before, "{}", vector.name);
                assert!(!cache.is_loaded(resource), "{}", vector.name);
                host_failures += 1;
                continue;
            }

            std::fs::write(root.0.join("RESOURCE.DAT"), &source).unwrap();
            if vector.allocation_status == Some(1) {
                assert_eq!(
                    cache.load_by_id(&store, &catalog, resource).unwrap(),
                    ResourceLoadStatus::LoadedNow,
                    "{}",
                    vector.name
                );
            }
            let target = if vector.mode == "direct" {
                PaletteResourceTarget::Direct
            } else {
                PaletteResourceTarget::Cached
            };
            let outcome = cache
                .load_palette_resource(&store, &catalog, resource, target, &mut palette)
                .unwrap();

            assert_eq!(
                palette,
                palette_from_hex(&vector.palette_after_hex),
                "{}",
                vector.name
            );
            match outcome.storage {
                PaletteResourceStorage::Direct(bytes) => {
                    assert_eq!(vector.mode, "direct", "{}", vector.name);
                    assert_eq!(&*bytes, &expected_bytes, "{}", vector.name);
                    assert!(!cache.is_loaded(resource), "{}", vector.name);
                }
                PaletteResourceStorage::Cached(status) => {
                    assert_eq!(vector.mode, "allocated", "{}", vector.name);
                    let expected_status = if vector.allocation_status == Some(1) {
                        ResourceLoadStatus::AlreadyLoaded
                    } else {
                        ResourceLoadStatus::LoadedNow
                    };
                    assert_eq!(status, expected_status, "{}", vector.name);
                    assert_eq!(
                        cache.resolve(resource),
                        Some(expected_bytes.as_slice()),
                        "{}",
                        vector.name
                    );
                }
            }
            successes += 1;
        }

        assert_eq!(successes, NAMED_RESOURCE_ORACLE_SUCCESS_COUNT);
        assert_eq!(host_failures, NAMED_RESOURCE_ORACLE_HOST_FAILURE_COUNT);
    }

    #[test]
    fn ordinary_load_resolve_size_reuse_and_release_are_owned_operations() {
        let root = TemporaryResourceRoot::create();
        let store = loose_store(&root);
        let catalog = test_catalog();
        let payload = vec![0x5A; CACHE_TEST_PAYLOAD_LENGTH];
        std::fs::write(root.0.join("CACHE.DAT"), &payload).unwrap();
        let mut cache = OriginalResourceCache::new();

        assert_eq!(
            cache
                .load_by_id(&store, &catalog, CACHE_TEST_RESOURCE_ID)
                .unwrap(),
            ResourceLoadStatus::LoadedNow
        );
        assert_eq!(cache.resolve(CACHE_TEST_RESOURCE_ID), Some(&payload[..]));
        assert_eq!(
            cache.allocation_byte_count(CACHE_TEST_RESOURCE_ID),
            Some(CACHE_TEST_ALLOCATION_LENGTH)
        );
        assert_eq!(
            cache
                .load_by_id(&store, &catalog, CACHE_TEST_RESOURCE_ID)
                .unwrap(),
            ResourceLoadStatus::AlreadyLoaded
        );
        assert!(cache.release(CACHE_TEST_RESOURCE_ID));
        assert!(!cache.release(CACHE_TEST_RESOURCE_ID));
        assert_eq!(cache.resolve(CACHE_TEST_RESOURCE_ID), None);
    }

    #[test]
    fn palette_resource_updates_colors_and_strips_only_the_preamble() {
        let root = TemporaryResourceRoot::create();
        let store = loose_store(&root);
        let catalog = test_catalog();
        let block_colors = [1, 2, 3, 4, 5, 6];
        std::fs::write(root.0.join("PALETTE.DAT"), palette_resource(&block_colors)).unwrap();
        let mut cache = OriginalResourceCache::new();
        let mut palette = [[0x3C; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];

        let outcome = cache
            .load_palette_resource(
                &store,
                &catalog,
                PALETTE_TEST_RESOURCE_ID,
                PaletteResourceTarget::Cached,
                &mut palette,
            )
            .unwrap();

        assert_eq!(
            outcome,
            PaletteResourceLoadOutcome {
                storage: PaletteResourceStorage::Cached(ResourceLoadStatus::LoadedNow),
                palette_changed: true,
            }
        );
        assert_eq!(
            &palette[PALETTE_TEST_FIRST_COLOR..PALETTE_TEST_FIRST_COLOR + PALETTE_TEST_COLOR_COUNT],
            &[[1, 2, 3], [4, 5, 6]]
        );
        let mut expected = PALETTE_TEST_HEADER.to_le_bytes().to_vec();
        expected.extend_from_slice(PALETTE_TEST_PAYLOAD);
        assert_eq!(cache.resolve(PALETTE_TEST_RESOURCE_ID), Some(&expected[..]));
    }

    #[test]
    fn direct_palette_resource_does_not_create_a_cache_entry() {
        let root = TemporaryResourceRoot::create();
        let store = loose_store(&root);
        let catalog = test_catalog();
        let block_colors = [7, 8, 9, 10, 11, 12];
        std::fs::write(root.0.join("PALETTE.DAT"), palette_resource(&block_colors)).unwrap();
        let mut cache = OriginalResourceCache::new();
        let mut palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];

        let outcome = cache
            .load_palette_resource(
                &store,
                &catalog,
                PALETTE_TEST_RESOURCE_ID,
                PaletteResourceTarget::Direct,
                &mut palette,
            )
            .unwrap();

        let PaletteResourceStorage::Direct(bytes) = outcome.storage else {
            panic!("direct target must return owned bytes");
        };
        assert_eq!(
            &bytes[..RESOURCE_FILE_HEADER_SIZE],
            &PALETTE_TEST_HEADER.to_le_bytes()
        );
        assert_eq!(&bytes[RESOURCE_FILE_HEADER_SIZE..], PALETTE_TEST_PAYLOAD);
        assert!(!cache.is_loaded(PALETTE_TEST_RESOURCE_ID));
    }

    #[test]
    fn malformed_palette_resource_is_transactional() {
        let root = TemporaryResourceRoot::create();
        let store = loose_store(&root);
        let catalog = test_catalog();
        let malformed = [
            PALETTE_TEST_HEADER.to_le_bytes().as_slice(),
            &[
                PALETTE_TEST_FIRST_COLOR as u8,
                PALETTE_TEST_COLOR_COUNT as u8,
            ],
            &[1, 2, 3],
        ]
        .concat();
        std::fs::write(root.0.join("PALETTE.DAT"), malformed).unwrap();
        let mut cache = OriginalResourceCache::new();
        let mut palette = [[0x2A; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        let before = palette;

        assert!(
            cache
                .load_palette_resource(
                    &store,
                    &catalog,
                    PALETTE_TEST_RESOURCE_ID,
                    PaletteResourceTarget::Cached,
                    &mut palette,
                )
                .is_err()
        );
        assert_eq!(palette, before);
        assert!(!cache.is_loaded(PALETTE_TEST_RESOURCE_ID));
    }
}
