//! Owned flat game state assembled from translated systems and original resources.

use std::fmt;

use anyhow::{Context, Result, bail};
use commander_blood_formats::manu3::decode_manu3;
use commander_blood_formats::panorama::BridgePanoramaArchive;

use crate::native::bloodprg::{
    BridgeSpriteRect, CHART_BACK_BUFFER_RESOURCE_PATH, FontPoint, IndexedGamePalette,
    LoadedScriptProfile, OriginalResourceCache, PauseHudRefresh, PbmDecodeResult, RasterPoint,
    RasterRectOutcome, ScriptProfileId, ScriptProfileLoadOutcome, ScriptProfileManager,
    build_pause_hud_refresh, decode_chart_back_buffer, draw_small_font_text, fill_framebuffer_rect,
};
use crate::native::manu3::model::Manu3Model;

use super::OriginalGameData;

/// Width of the original logical game surface.
pub const LOGICAL_FRAMEBUFFER_WIDTH: usize = 320;
/// Height of the original logical game surface.
pub const LOGICAL_FRAMEBUFFER_HEIGHT: usize = 200;
/// Number of palette indices in one complete logical game frame.
pub const LOGICAL_FRAMEBUFFER_PIXEL_COUNT: usize =
    LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT;

const MANU3_RESOURCE_NAME: &[u8] = b"MANU3.XDB";
const PAUSE_HUD_CLEAR_COLOR: u8 = u8::MIN;

/// One owned 320 by 200 row-major indexed framebuffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexedFramebuffer {
    pixels: Box<[u8]>,
}

impl IndexedFramebuffer {
    /// Allocate a black logical framebuffer.
    pub fn new() -> Self {
        Self {
            pixels: vec![u8::MIN; LOGICAL_FRAMEBUFFER_PIXEL_COUNT].into_boxed_slice(),
        }
    }

    /// Borrow all row-major palette indices.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Mutably borrow all row-major palette indices.
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// Replace every pixel with one palette index.
    pub fn clear(&mut self, palette_index: u8) {
        self.pixels.fill(palette_index);
    }

    /// Copy a complete logical frame without address or pitch conversion.
    pub fn copy_from(&mut self, source: &Self) {
        self.pixels.copy_from_slice(&source.pixels);
    }
}

impl Default for IndexedFramebuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether an optional runtime asset was decoded during the current request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeAssetLoadStatus {
    /// The asset was read and decoded during this call.
    LoadedNow,
    /// The existing decoded asset remains active.
    AlreadyLoaded,
}

/// Core owned state used by the future SDL lifecycle host.
pub struct OriginalGameRuntime {
    data: OriginalGameData,
    resource_cache: OriginalResourceCache,
    profiles: ScriptProfileManager,
    live_palette: IndexedGamePalette,
    front_buffer: IndexedFramebuffer,
    back_buffer: IndexedFramebuffer,
    manu3: Option<Manu3Model>,
    bridge_panorama: Option<BridgePanoramaArchive>,
}

impl fmt::Debug for OriginalGameRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OriginalGameRuntime")
            .field("data", &self.data)
            .field(
                "loaded_profile",
                &self.profiles.current().map(LoadedScriptProfile::id),
            )
            .field("manu3_loaded", &self.manu3.is_some())
            .field("bridge_panorama_loaded", &self.bridge_panorama.is_some())
            .finish_non_exhaustive()
    }
}

impl OriginalGameRuntime {
    /// Allocate the flat runtime around one validated original data set.
    pub fn new(data: OriginalGameData) -> Self {
        let profiles = ScriptProfileManager::new(data.script_profile_catalog().clone());
        let live_palette = *data.default_vga_palette();
        Self {
            data,
            resource_cache: OriginalResourceCache::new(),
            profiles,
            live_palette,
            front_buffer: IndexedFramebuffer::new(),
            back_buffer: IndexedFramebuffer::new(),
            manu3: None,
            bridge_panorama: None,
        }
    }

    /// Validated source data and executable-derived catalogs.
    pub const fn data(&self) -> &OriginalGameData {
        &self.data
    }

    /// Palette currently applied by translated drawing and presentation systems.
    pub const fn live_palette(&self) -> &IndexedGamePalette {
        &self.live_palette
    }

    /// Mutably borrow the live native six-bit palette.
    pub fn live_palette_mut(&mut self) -> &mut IndexedGamePalette {
        &mut self.live_palette
    }

    /// Logical frame submitted to the modern renderer.
    pub const fn front_buffer(&self) -> &IndexedFramebuffer {
        &self.front_buffer
    }

    /// Mutably borrow the logical presentation frame.
    pub fn front_buffer_mut(&mut self) -> &mut IndexedFramebuffer {
        &mut self.front_buffer
    }

    /// Logical background frame retained across scene composition.
    pub const fn back_buffer(&self) -> &IndexedFramebuffer {
        &self.back_buffer
    }

    /// Copy the complete retained background into the presentation frame.
    pub fn restore_back_buffer(&mut self) {
        self.front_buffer.copy_from(&self.back_buffer);
    }

    /// Decode the original MANU3 model from `BLOOD.DAT` once.
    pub fn load_manu3(&mut self) -> Result<RuntimeAssetLoadStatus> {
        if self.manu3.is_some() {
            return Ok(RuntimeAssetLoadStatus::AlreadyLoaded);
        }
        let bytes = self
            .data
            .load_named_resource(MANU3_RESOURCE_NAME)
            .context("loading MANU3.XDB from original resources")?;
        let asset = decode_manu3(&bytes).context("decoding MANU3.XDB")?;
        self.manu3 = Some(Manu3Model::from_asset(asset).context("constructing MANU3 model")?);
        Ok(RuntimeAssetLoadStatus::LoadedNow)
    }

    /// Borrow the decoded MANU3 runtime model.
    pub const fn manu3(&self) -> Option<&Manu3Model> {
        self.manu3.as_ref()
    }

    /// Mutably borrow the decoded MANU3 runtime model.
    pub fn manu3_mut(&mut self) -> Option<&mut Manu3Model> {
        self.manu3.as_mut()
    }

    /// Decode the complete bridge panorama archive once.
    pub fn open_bridge_panorama(&mut self) -> Result<RuntimeAssetLoadStatus> {
        if self.bridge_panorama.is_some() {
            return Ok(RuntimeAssetLoadStatus::AlreadyLoaded);
        }
        let path = self.data.paths().bridge_panorama();
        let bytes = std::fs::read(path)
            .with_context(|| format!("reading bridge panorama {}", path.display()))?
            .into_boxed_slice();
        self.bridge_panorama = Some(
            BridgePanoramaArchive::decode(bytes)
                .with_context(|| format!("decoding bridge panorama {}", path.display()))?,
        );
        Ok(RuntimeAssetLoadStatus::LoadedNow)
    }

    /// Borrow the decoded bridge panorama archive.
    pub const fn bridge_panorama(&self) -> Option<&BridgePanoramaArchive> {
        self.bridge_panorama.as_ref()
    }

    /// Transfer the decoded panorama to the live bridge scene.
    pub fn take_bridge_panorama(&mut self) -> Option<BridgePanoramaArchive> {
        self.bridge_panorama.take()
    }

    /// Decode `CHART.FD` into the retained logical background.
    pub fn initialize_back_buffer(&mut self) -> Result<PbmDecodeResult> {
        let bytes = self
            .data
            .load_named_resource(CHART_BACK_BUFFER_RESOURCE_PATH.as_bytes())
            .context("loading CHART.FD")?;
        decode_chart_back_buffer(
            &bytes,
            self.back_buffer.pixels_mut(),
            &mut self.live_palette,
        )
        .context("decoding CHART.FD")
    }

    /// Draw the exact pause clear rectangle and compact-font label into the front buffer.
    pub fn draw_pause_hud(&mut self, active: bool) -> Result<Option<PauseHudRefresh>> {
        let Some(refresh) = build_pause_hud_refresh(u8::from(active)) else {
            return Ok(None);
        };
        let display_clip = BridgeSpriteRect {
            left: i32::from(u16::MIN),
            right: i32::try_from(LOGICAL_FRAMEBUFFER_WIDTH)
                .context("logical framebuffer width exceeds i32")?,
            top: i32::from(u16::MIN),
            bottom: i32::try_from(LOGICAL_FRAMEBUFFER_HEIGHT)
                .context("logical framebuffer height exceeds i32")?,
        };
        let clear = refresh.clear_region;
        let clear_outcome = fill_framebuffer_rect(
            self.front_buffer.pixels_mut(),
            display_clip,
            RasterPoint {
                x: i32::from(clear.x),
                y: i32::from(clear.y),
            },
            clear.width,
            clear.height,
            PAUSE_HUD_CLEAR_COLOR,
        )
        .context("clearing the translated pause HUD region")?;
        if clear_outcome == RasterRectOutcome::Rejected {
            bail!("authored pause HUD rectangle was rejected by the logical display clip");
        }
        draw_small_font_text(
            self.front_buffer.pixels_mut(),
            self.data.font_resources(),
            refresh.text,
            FontPoint {
                x: i32::from(refresh.text_position[0]),
                y: i32::from(refresh.text_position[1]),
            },
            refresh.text_palette_index,
        )
        .context("drawing the translated pause HUD label")?;
        Ok(Some(refresh))
    }

    /// Load and bind one complete playable BloodScript profile.
    pub fn load_profile(&mut self, profile: ScriptProfileId) -> Result<ScriptProfileLoadOutcome> {
        self.profiles
            .select(
                profile,
                &mut self.resource_cache,
                self.data.resource_store(),
                self.data.resource_catalog(),
            )
            .with_context(|| format!("loading BloodScript profile {}", profile.value()))
    }

    /// Borrow the currently loaded playable profile.
    pub const fn current_profile(&self) -> Option<&LoadedScriptProfile> {
        self.profiles.current()
    }

    /// Mutably borrow the currently loaded playable profile for VM execution.
    pub fn current_profile_mut(&mut self) -> Option<&mut LoadedScriptProfile> {
        self.profiles.current_mut()
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::super::{OriginalGameData, OriginalGameDataPaths};
    use super::*;

    const TEST_BACKGROUND_COLOR: u8 = 73;
    const EXPECTED_PAUSE_LABEL_PIXEL_COUNT: usize = 46;

    #[test]
    fn framebuffer_has_stable_logical_dimensions_and_complete_copy_semantics() {
        let mut source = IndexedFramebuffer::new();
        let mut destination = IndexedFramebuffer::new();
        source.clear(TEST_BACKGROUND_COLOR);
        destination.copy_from(&source);

        assert_eq!(destination.pixels().len(), LOGICAL_FRAMEBUFFER_PIXEL_COUNT);
        assert!(
            destination
                .pixels()
                .iter()
                .all(|pixel| *pixel == TEST_BACKGROUND_COLOR)
        );
    }

    #[test]
    fn pause_hud_composes_exact_geometry_with_the_executable_font() {
        let Some(data) = original_game_data() else {
            return;
        };
        let mut runtime = OriginalGameRuntime::new(data);
        runtime.front_buffer_mut().clear(TEST_BACKGROUND_COLOR);
        let unchanged = runtime.front_buffer().clone();

        assert_eq!(runtime.draw_pause_hud(false).unwrap(), None);
        assert_eq!(runtime.front_buffer(), &unchanged);

        let refresh = runtime.draw_pause_hud(true).unwrap().unwrap();
        assert_eq!(refresh, build_pause_hud_refresh(u8::from(true)).unwrap());
        let clear = refresh.clear_region;
        let mut label_pixel_count = usize::MIN;
        for y in usize::MIN..LOGICAL_FRAMEBUFFER_HEIGHT {
            for x in usize::MIN..LOGICAL_FRAMEBUFFER_WIDTH {
                let pixel = runtime.front_buffer().pixels()[y * LOGICAL_FRAMEBUFFER_WIDTH + x];
                let inside = x >= usize::from(clear.x)
                    && x < usize::from(clear.x + clear.width)
                    && y >= usize::from(clear.y)
                    && y < usize::from(clear.y + clear.height);
                if inside {
                    assert!(
                        pixel == PAUSE_HUD_CLEAR_COLOR || pixel == refresh.text_palette_index,
                        "unexpected pause pixel {pixel} at {x},{y}"
                    );
                    label_pixel_count += usize::from(pixel == refresh.text_palette_index);
                } else {
                    assert_eq!(pixel, TEST_BACKGROUND_COLOR, "outside pixel at {x},{y}");
                }
            }
        }
        assert_eq!(label_pixel_count, EXPECTED_PAUSE_LABEL_PIXEL_COUNT);
    }

    fn original_game_data() -> Option<OriginalGameData> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        [
            workspace_root.join("output/_tmp_iso"),
            workspace_root.join("commander-blood-audio/_tmp_iso"),
            workspace_root.join("accuracy/cblood_install/cblood"),
        ]
        .into_iter()
        .find_map(|root: PathBuf| OriginalGameDataPaths::from_root(root).ok())
        .and_then(|paths| {
            OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).ok()
        })
    }
}
