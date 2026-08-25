//! Typed loading of original Commander Blood artwork.

use std::collections::BTreeSet;
use std::ops::RangeInclusive;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_formats::archive::{BloodArchive, BloodResourceName};

const RGBA_COMPONENT_COUNT: usize = 4;
const OPAQUE_ALPHA: u8 = 255;
const TITLE_FILENAME: &str = "BLOOD.LBM";
const EXECUTABLE_FILENAME: &str = "BLOODPRG.EXE";
const BRIDGE_PANORAMA_FILENAME: &str = "TB.BIG";

/// Storage selected for one original game resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalResourceSource {
    /// Member bytes validated inside `BLOOD.DAT`.
    EmbeddedArchive,
    /// Standalone file below the configured game-data root.
    LooseFile,
}

/// Unified resource loader replacing DOS files and expanded-memory backends.
#[derive(Clone, Debug)]
pub struct OriginalResourceStore {
    loose_root: PathBuf,
    archive: Option<BloodArchive>,
    loose_names: BTreeSet<BloodResourceName>,
    force_loose: bool,
}

impl OriginalResourceStore {
    /// Construct a loader from decoded archive data and exact-case loose names.
    pub fn new(
        loose_root: PathBuf,
        archive: Option<BloodArchive>,
        loose_names: impl IntoIterator<Item = BloodResourceName>,
        force_loose: bool,
    ) -> Self {
        Self {
            loose_root,
            archive,
            loose_names: loose_names.into_iter().collect(),
            force_loose,
        }
    }

    /// Select the source that can satisfy one resource request.
    ///
    /// This translates `resource_source_select` at BLOODPRG file offset
    /// `0x002693`. The authored loose-name allowlist remains case-sensitive;
    /// archive lookup uses the executable's distinct DOS byte folding.
    pub fn source(&self, name: &BloodResourceName) -> OriginalResourceSource {
        if self.force_loose || self.loose_names.contains(name) {
            return OriginalResourceSource::LooseFile;
        }
        if self
            .archive
            .as_ref()
            .is_some_and(|archive| archive.member(name).is_some())
        {
            OriginalResourceSource::EmbeddedArchive
        } else {
            OriginalResourceSource::LooseFile
        }
    }

    /// Return the byte count of a resolvable resource.
    ///
    /// This is the typed host equivalent of `resource_name_lookup` at
    /// BLOODPRG file offset `0x0028CA`.
    pub fn resource_len(&self, name: &BloodResourceName) -> Result<usize> {
        match self.source(name) {
            OriginalResourceSource::EmbeddedArchive => Ok(self
                .archive
                .as_ref()
                .and_then(|archive| archive.member(name))
                .expect("source selection validated the archive member")
                .len()),
            OriginalResourceSource::LooseFile => {
                let path = self.loose_path(name)?;
                let byte_count = std::fs::metadata(&path)
                    .with_context(|| format!("reading resource metadata {}", path.display()))?
                    .len();
                usize::try_from(byte_count)
                    .with_context(|| format!("resource is too large: {}", path.display()))
            }
        }
    }

    /// Load one resource into a single owned byte allocation.
    ///
    /// This translates `resource_file_load` at BLOODPRG file offset
    /// `0x002ABB`. XMS, EMS, chunk cursors, and address wrapping are obsolete;
    /// consumers receive the exact member or loose-file bytes directly.
    pub fn load(&self, name: &BloodResourceName) -> Result<Box<[u8]>> {
        match self.source(name) {
            OriginalResourceSource::EmbeddedArchive => Ok(Box::from(
                self.archive
                    .as_ref()
                    .and_then(|archive| archive.member(name))
                    .expect("source selection validated the archive member"),
            )),
            OriginalResourceSource::LooseFile => {
                let path = self.loose_path(name)?;
                Ok(std::fs::read(&path)
                    .with_context(|| format!("reading original resource {}", path.display()))?
                    .into_boxed_slice())
            }
        }
    }

    /// Create or truncate one loose resource and write all supplied bytes.
    ///
    /// This translates `file_create_and_write` at BLOODPRG file offset
    /// `0x002B6B`. The configured root replaces drive and current-directory
    /// changes, and Rust's complete-slice write replaces chunk cursor updates.
    pub fn write_loose(&self, name: &BloodResourceName, data: &[u8]) -> Result<usize> {
        let path = self.loose_path(name)?;
        std::fs::write(&path, data)
            .with_context(|| format!("writing original resource {}", path.display()))?;
        Ok(data.len())
    }

    /// Copy a nonempty resource to a loose destination below the data root.
    ///
    /// This translates `startup_resource_file_copy` at BLOODPRG file offset
    /// `0x00280F`. A zero-length source preserves the original skip behavior;
    /// successful copies use the same typed archive-or-loose source policy as
    /// ordinary loads.
    pub fn copy_to_loose(
        &self,
        source: &BloodResourceName,
        destination: &BloodResourceName,
    ) -> Result<bool> {
        let data = self.load(source)?;
        if data.is_empty() {
            return Ok(false);
        }
        self.write_loose(destination, &data)?;
        Ok(true)
    }

    fn loose_path(&self, name: &BloodResourceName) -> Result<PathBuf> {
        let folded = name.archive_lookup_key();
        let host_name = std::str::from_utf8(&folded)
            .expect("validated ASCII resource name")
            .replace('\\', "/");
        let relative = Path::new(&host_name);
        let mut path = self.loose_root.clone();
        for component in relative.components() {
            match component {
                Component::Normal(part) => path.push(part),
                Component::CurDir => {}
                _ => bail!("resource path escapes the game-data root: {host_name}"),
            }
        }
        Ok(path)
    }
}

/// One decoded original frame ready for upload to a modern GPU texture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalFrame {
    /// Source width in pixels.
    pub width: u32,
    /// Source height in pixels.
    pub height: u32,
    /// Row-major red, green, blue, alpha pixels.
    pub rgba: Vec<u8>,
    /// Original row-major palette indices retained for palette animation.
    pub indexed_pixels: Vec<u8>,
    /// Source palette expanded to packed red, green, blue, alpha entries.
    pub palette_rgba: Vec<u8>,
}

impl OriginalFrame {
    /// Decode one original ILBM or PBM image without changing its palette colors.
    pub fn load_lbm(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("reading original image {}", path.display()))?;
        let image = commander_blood_formats::lbm::decode_lbm(&bytes)
            .with_context(|| format!("decoding original LBM image {}", path.display()))?;

        let mut palette_rgba = Vec::with_capacity(image.palette.len() * RGBA_COMPONENT_COUNT);
        for [red, green, blue] in image.palette {
            palette_rgba.extend_from_slice(&[red, green, blue, OPAQUE_ALPHA]);
        }
        let mut frame = Self {
            width: image.width as u32,
            height: image.height as u32,
            rgba: Vec::new(),
            indexed_pixels: image.pixels,
            palette_rgba,
        };
        frame.rebuild_rgba();
        Ok(frame)
    }

    /// Install a recovered palette range and rebuild the current RGBA frame.
    pub fn install_palette_range(
        &mut self,
        palette: &[[u8; 3]; commander_blood_formats::lbm::PALETTE_ENTRY_COUNT],
        range: RangeInclusive<usize>,
    ) {
        for index in range {
            let destination = index * RGBA_COMPONENT_COUNT;
            self.palette_rgba[destination..destination + 3].copy_from_slice(&palette[index]);
            self.palette_rgba[destination + 3] = OPAQUE_ALPHA;
        }
        self.rebuild_rgba();
    }

    fn rebuild_rgba(&mut self) {
        self.rgba.clear();
        self.rgba
            .reserve(self.indexed_pixels.len() * RGBA_COMPONENT_COUNT);
        for palette_index in &self.indexed_pixels {
            let start = usize::from(*palette_index) * RGBA_COMPONENT_COUNT;
            self.rgba
                .extend_from_slice(&self.palette_rgba[start..start + RGBA_COMPONENT_COUNT]);
        }
    }
}

/// Find the original title image from an explicit path or known game-data roots.
pub fn find_title_image(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if path.is_file() {
            return Ok(path.to_owned());
        }
        bail!("title image does not exist: {}", path.display());
    }

    let mut candidates = Vec::new();
    if let Some(root) = std::env::var_os("CBLOOD_DATA") {
        candidates.push(PathBuf::from(root).join(TITLE_FILENAME));
    }
    candidates.extend([
        PathBuf::from("commander-blood-audio/_tmp_iso").join(TITLE_FILENAME),
        PathBuf::from("output/_tmp_iso").join(TITLE_FILENAME),
        PathBuf::from("accuracy/cblood_install/cblood").join(TITLE_FILENAME),
    ]);

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .context("BLOOD.LBM not found; pass --asset PATH or set CBLOOD_DATA")
}

/// Find the original executable used as the authoritative default-palette source.
pub fn find_bloodprg_executable(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if path.is_file() {
            return Ok(path.to_owned());
        }
        bail!("game executable does not exist: {}", path.display());
    }

    let mut candidates = Vec::new();
    if let Some(root) = std::env::var_os("CBLOOD_DATA") {
        candidates.push(PathBuf::from(root).join(EXECUTABLE_FILENAME));
    }
    candidates.extend([
        PathBuf::from("commander-blood-audio/_tmp_iso").join(EXECUTABLE_FILENAME),
        PathBuf::from("output/_tmp_iso").join(EXECUTABLE_FILENAME),
        PathBuf::from("accuracy/cblood_install/cblood").join(EXECUTABLE_FILENAME),
    ]);

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .context("BLOODPRG.EXE not found; pass --bloodprg PATH or set CBLOOD_DATA")
}

/// Find the original `TB.BIG` bridge panorama from an explicit path or data root.
pub fn find_bridge_panorama(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if path.is_file() {
            return Ok(path.to_owned());
        }
        bail!("bridge panorama does not exist: {}", path.display());
    }

    let mut candidates = Vec::new();
    if let Some(root) = std::env::var_os("CBLOOD_DATA") {
        candidates.push(PathBuf::from(root).join(BRIDGE_PANORAMA_FILENAME));
    }
    candidates.extend([
        PathBuf::from("commander-blood-audio/_tmp_iso").join(BRIDGE_PANORAMA_FILENAME),
        PathBuf::from("output/_tmp_iso").join(BRIDGE_PANORAMA_FILENAME),
        PathBuf::from("accuracy/cblood_install/cblood").join(BRIDGE_PANORAMA_FILENAME),
    ]);

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .context("TB.BIG not found; pass --panorama PATH or set CBLOOD_DATA")
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use commander_blood_formats::archive::BloodArchive;
    use serde::Deserialize;

    use super::*;

    const ORIGINAL_TITLE_WIDTH: u32 = 640;
    const ORIGINAL_TITLE_HEIGHT: u32 = 480;
    const MINIMUM_DISTINCT_TITLE_COLORS: usize = 8;
    const ALPHA_COMPONENT_INDEX: usize = RGBA_COMPONENT_COUNT - 1;
    const DIRECTORY_HEADER_SIZE: usize = 2;
    const DIRECTORY_ENTRY_SIZE: usize = 25;
    const RESOURCE_NAME_FIELD_SIZE: usize = 16;
    const BYTE_COUNT_FIELD_OFFSET: usize = RESOURCE_NAME_FIELD_SIZE;
    const FILE_POSITION_FIELD_OFFSET: usize = BYTE_COUNT_FIELD_OFFSET + 4;
    const FILE_POSITION_FIELD_SIZE: usize = 4;
    const FORCE_LOOSE_BIT: u8 = 1;
    const TEST_PAYLOAD_BYTE: u8 = 73;
    static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(u64::MIN);

    #[derive(Deserialize)]
    struct SourceSelectionOracle {
        filename: String,
        force_flag: u8,
        allowlist_entries: Vec<String>,
        route: String,
        archive_hit: bool,
    }

    struct TemporaryResourceRoot(PathBuf);

    impl TemporaryResourceRoot {
        fn create() -> Self {
            let sequence = TEMPORARY_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "commander-blood-resource-test-{}-{sequence}",
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

    fn archive_bytes(records: &[(BloodResourceName, &[u8])]) -> Box<[u8]> {
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
            data[cursor..cursor + name.as_bytes().len()].copy_from_slice(name.as_bytes());
            data[cursor + BYTE_COUNT_FIELD_OFFSET..cursor + FILE_POSITION_FIELD_OFFSET]
                .copy_from_slice(&i32::try_from(payload.len()).unwrap().to_le_bytes());
            data[cursor + FILE_POSITION_FIELD_OFFSET
                ..cursor + FILE_POSITION_FIELD_OFFSET + FILE_POSITION_FIELD_SIZE]
                .copy_from_slice(&i32::try_from(payload_position).unwrap().to_le_bytes());
            data[payload_position..payload_position + payload.len()].copy_from_slice(payload);
            payload_position += payload.len();
        }
        data.into_boxed_slice()
    }

    fn resource_name(name: impl AsRef<[u8]>) -> BloodResourceName {
        BloodResourceName::new(name).unwrap()
    }

    #[test]
    fn converts_the_original_indexed_title_to_rgba() {
        let Ok(path) = find_title_image(None) else {
            return;
        };
        let frame = OriginalFrame::load_lbm(&path).unwrap();
        assert_eq!(
            (frame.width, frame.height),
            (ORIGINAL_TITLE_WIDTH, ORIGINAL_TITLE_HEIGHT)
        );
        assert_eq!(
            frame.rgba.len(),
            ORIGINAL_TITLE_WIDTH as usize * ORIGINAL_TITLE_HEIGHT as usize * RGBA_COMPONENT_COUNT
        );
        assert!(
            frame
                .rgba
                .chunks_exact(RGBA_COMPONENT_COUNT)
                .all(|pixel| pixel[ALPHA_COMPONENT_INDEX] == OPAQUE_ALPHA)
        );
        assert_eq!(
            frame.palette_rgba.len(),
            commander_blood_formats::lbm::PALETTE_ENTRY_COUNT * RGBA_COMPONENT_COUNT
        );
        let distinct_colors: std::collections::BTreeSet<&[u8]> =
            frame.rgba.chunks_exact(RGBA_COMPONENT_COUNT).collect();
        assert!(distinct_colors.len() >= MINIMUM_DISTINCT_TITLE_COLORS);
    }

    #[test]
    fn source_selection_matches_every_native_oracle_vector() {
        let vectors: Vec<SourceSelectionOracle> = serde_json::from_str(include_str!(
            "../../../re/tools/oracle_vectors/func_2693_natural.json"
        ))
        .unwrap();

        for vector in vectors {
            let requested = resource_name(&vector.filename);
            let archive_name = if vector.archive_hit {
                requested.clone()
            } else {
                resource_name("UNRELATED.DAT")
            };
            let archive =
                BloodArchive::decode(archive_bytes(&[(archive_name, &[TEST_PAYLOAD_BYTE])]))
                    .unwrap();
            let store = OriginalResourceStore::new(
                PathBuf::new(),
                Some(archive),
                vector.allowlist_entries.iter().map(resource_name),
                vector.force_flag & FORCE_LOOSE_BIT != u8::MIN,
            );
            let expected = if vector.route == "write" || !vector.archive_hit {
                OriginalResourceSource::LooseFile
            } else {
                OriginalResourceSource::EmbeddedArchive
            };

            assert_eq!(store.source(&requested), expected, "{}", vector.filename);
        }
    }

    #[test]
    fn loads_owned_archive_and_nested_loose_resource_bytes() {
        let root = TemporaryResourceRoot::create();
        let loose_name = resource_name(r"DIR\LOOSE.DAT");
        let loose_payload = b"loose resource";
        let loose_path = root.0.join("DIR/LOOSE.DAT");
        std::fs::create_dir_all(loose_path.parent().unwrap()).unwrap();
        std::fs::write(&loose_path, loose_payload).unwrap();

        let embedded_name = resource_name("EMBED.DAT");
        let embedded_payload = b"embedded resource";
        let archive =
            BloodArchive::decode(archive_bytes(&[(embedded_name.clone(), embedded_payload)]))
                .unwrap();
        let store =
            OriginalResourceStore::new(root.0.clone(), Some(archive), [loose_name.clone()], false);

        assert_eq!(
            store.resource_len(&embedded_name).unwrap(),
            embedded_payload.len()
        );
        assert_eq!(&*store.load(&embedded_name).unwrap(), embedded_payload);
        assert_eq!(
            store.resource_len(&loose_name).unwrap(),
            loose_payload.len()
        );
        assert_eq!(&*store.load(&loose_name).unwrap(), loose_payload);
    }

    #[test]
    fn copies_and_truncates_resources_below_the_explicit_root() {
        let root = TemporaryResourceRoot::create();
        let source_name = resource_name("SOURCE.DAT");
        let destination_name = resource_name("COPIED.DAT");
        let empty_name = resource_name("EMPTY.DAT");
        let empty_destination_name = resource_name("NOFILE.DAT");
        let payload = b"complete copied resource";
        let archive = BloodArchive::decode(archive_bytes(&[
            (source_name.clone(), payload),
            (empty_name.clone(), &[]),
        ]))
        .unwrap();
        let store = OriginalResourceStore::new(root.0.clone(), Some(archive), [], false);

        assert!(
            store
                .copy_to_loose(&source_name, &destination_name)
                .unwrap()
        );
        assert_eq!(std::fs::read(root.0.join("COPIED.DAT")).unwrap(), payload);
        assert!(
            !store
                .copy_to_loose(&empty_name, &empty_destination_name)
                .unwrap()
        );
        assert!(!root.0.join("NOFILE.DAT").exists());

        assert_eq!(
            store.write_loose(&destination_name, &[]).unwrap(),
            usize::MIN
        );
        assert!(std::fs::read(root.0.join("COPIED.DAT")).unwrap().is_empty());
    }
}
