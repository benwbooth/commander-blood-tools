//! Owned flat game state assembled from translated systems and original resources.

use std::fmt;
use std::ops::Range;

use anyhow::{Context, Result, bail};
use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::manu3::decode_manu3;
use commander_blood_formats::panorama::BridgePanoramaArchive;

use crate::native::bloodprg::{
    BRIDGE_SPRITE_ENTITY_COUNT, BridgeFrameState, BridgeSpriteClipSnapshotFlags,
    BridgeSpriteCommitOutcome, BridgeSpriteDirtyRegions, BridgeSpriteEntity, BridgeSpritePosition,
    BridgeSpriteRect, CHART_BACK_BUFFER_RESOURCE_PATH, CameraApproachState, DirtyRegionCopyOutcome,
    FontPoint, GameFontDrawOutcome, IndexedGamePalette, LoadedScriptProfile, NameAreaEffectOutcome,
    NameAreaEffectState, OriginalResourceCache, OriginalSaveSlotDirectory,
    PaletteResourceLoadOutcome, PaletteResourceStorage, PaletteResourceTarget, PauseHudRefresh,
    PbmDecodeResult, PresentationChoiceNumber, RasterPoint, RasterRectOutcome, ResourceId,
    ScriptPresentationEntity, ScriptProfileId, ScriptProfileLoadOutcome, ScriptProfileManager,
    ShipDepthBandLayout, ShipHudState, ShipViewArtworkSelection, ShipViewEntityId,
    activate_bridge_sprite_from_resource, advance_bridge_sprite_state, build_pause_hud_refresh,
    commit_bridge_sprite_dirty_range, copy_dirty_regions_to_display, decode_chart_back_buffer,
    draw_presentation_choice_number, draw_small_font_text, fill_framebuffer_rect,
    select_ship_view_artwork, update_name_area_effect,
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
const SAVE_SLOT_DIRECTORY_RESOURCE_NAME: &[u8] = b"BLOOD.SAV";
const STARTUP_CARTOGRAPHY_RESOURCE: ResourceId = ResourceId::new(44);
const STARTUP_CARTOGRAPHY_RESOURCE_NAME: &[u8] = b"carte.spr";
const PAUSE_HUD_CLEAR_COLOR: u8 = u8::MIN;
const DIALOGUE_OVERLAY_ENTITY_INDEX: usize = 4;
const NAME_AREA_EFFECT_ENTITY_INDEX: usize = 2;
const SHIP_VIEW_TRANSITION_ENTITY_INDEX: usize = 31;
const PRESENTATION_PANEL_ENTITY_INDEX: usize = 31;
const LOGICAL_FRAMEBUFFER_HALF_HEIGHT: usize = 100;
const SHIP_TRAVEL_CLEAR_COLOR: u8 = u8::MIN;
const SHIP_DIRTY_SNAPSHOT_PENDING: u16 = 1;
const LOGICAL_DISPLAY_CLIP: BridgeSpriteRect = BridgeSpriteRect {
    left: 0,
    right: LOGICAL_FRAMEBUFFER_WIDTH as i32,
    top: 0,
    bottom: LOGICAL_FRAMEBUFFER_HEIGHT as i32,
};

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
    save_slots: Option<OriginalSaveSlotDirectory>,
    bridge_frame_state: BridgeFrameState,
    bridge_sprite_entities: [BridgeSpriteEntity; BRIDGE_SPRITE_ENTITY_COUNT],
    bridge_dirty_regions: BridgeSpriteDirtyRegions,
    camera_approach: CameraApproachState,
    name_area_effect: NameAreaEffectState,
    ship_hud: ShipHudState,
    world_artwork_layout: Box<[commander_blood_formats::world_art::WorldArtworkLayout]>,
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
            .field("save_slots_loaded", &self.save_slots.is_some())
            .finish_non_exhaustive()
    }
}

impl OriginalGameRuntime {
    /// Allocate the flat runtime around one validated original data set.
    pub fn new(data: OriginalGameData) -> Self {
        let profiles = ScriptProfileManager::new(data.script_profile_catalog().clone());
        let live_palette = *data.default_vga_palette();
        let world_artwork_layout = data.world_artwork_layout().to_vec().into_boxed_slice();
        Self {
            data,
            resource_cache: OriginalResourceCache::new(),
            profiles,
            live_palette,
            front_buffer: IndexedFramebuffer::new(),
            back_buffer: IndexedFramebuffer::new(),
            manu3: None,
            bridge_panorama: None,
            save_slots: None,
            bridge_frame_state: BridgeFrameState::default(),
            bridge_sprite_entities: [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT],
            bridge_dirty_regions: BridgeSpriteDirtyRegions::default(),
            camera_approach: CameraApproachState::default(),
            name_area_effect: NameAreaEffectState::default(),
            ship_hud: ShipHudState::default(),
            world_artwork_layout,
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

    /// Persistent typed bridge-frame coordinator state.
    pub const fn bridge_frame_state(&self) -> &BridgeFrameState {
        &self.bridge_frame_state
    }

    /// Complete flat sprite entity table shared by bridge presentation systems.
    pub fn bridge_sprite_entities(&self) -> &[BridgeSpriteEntity; BRIDGE_SPRITE_ENTITY_COUNT] {
        &self.bridge_sprite_entities
    }

    /// Current ship-camera approach state started by BloodScript travel actions.
    pub const fn camera_approach(&self) -> &CameraApproachState {
        &self.camera_approach
    }

    /// Current character-name palette effect state.
    pub const fn name_area_effect(&self) -> &NameAreaEffectState {
        &self.name_area_effect
    }

    /// Most recent ship-HUD palette snapshot and camera reset.
    pub const fn ship_hud(&self) -> &ShipHudState {
        &self.ship_hud
    }

    /// Request the deterministic first frame of the authored name-area effect.
    pub fn restart_name_area_effect(&mut self) {
        self.name_area_effect.active = true;
        self.name_area_effect.restart_requested = true;
    }

    /// Advance one fixed bridge presentation entity emitted by BloodScript.
    pub fn transition_presentation_entity(
        &mut self,
        entity: ScriptPresentationEntity,
    ) -> Result<bool> {
        let entity_index = match entity {
            ScriptPresentationEntity::DialogueOverlay => DIALOGUE_OVERLAY_ENTITY_INDEX,
            ScriptPresentationEntity::NameAreaEffect => NAME_AREA_EFFECT_ENTITY_INDEX,
        };
        advance_bridge_sprite_state(&mut self.bridge_sprite_entities, entity_index)
            .with_context(|| format!("transitioning bridge presentation entity {entity_index}"))
    }

    /// Mark the bridge presentation panel entity for its recovered opening transition.
    pub fn transition_presentation_panel_entity(&mut self) -> Result<bool> {
        advance_bridge_sprite_state(
            &mut self.bridge_sprite_entities,
            PRESENTATION_PANEL_ENTITY_INDEX,
        )
        .context("transitioning the bridge presentation panel entity")
    }

    /// Advance any fixed ship-view entity selected by the recovered coordinator.
    pub fn transition_ship_view_entity(&mut self, entity: ShipViewEntityId) -> Result<bool> {
        let entity_index = usize::from(entity.value());
        advance_bridge_sprite_state(&mut self.bridge_sprite_entities, entity_index)
            .with_context(|| format!("transitioning ship-view entity {entity_index}"))
    }

    /// Arm the translated camera-approach state machine for a travel action.
    pub fn start_camera_transition(&mut self) {
        self.camera_approach.phase = u8::MIN;
        self.camera_approach.transition_pending = true;
        self.bridge_frame_state.set_transition_pending(true);
    }

    /// Advance one frame of the executable-authored character-name palette effect.
    pub fn advance_name_area_effect(
        &mut self,
        random_index: &mut impl FnMut(usize) -> usize,
    ) -> Result<NameAreaEffectOutcome> {
        update_name_area_effect(
            self.data.name_area_effect_sequences(),
            &mut self.name_area_effect,
            self.front_buffer.pixels_mut(),
            random_index,
        )
        .context("updating the bridge name-area palette effect")
    }

    /// Select current-position artwork, activate its sprite, and reset the ship HUD.
    pub fn reset_ship_hud(&mut self) -> Result<ShipViewArtworkSelection> {
        let Self {
            data,
            resource_cache,
            profiles,
            live_palette,
            bridge_sprite_entities,
            ship_hud,
            world_artwork_layout,
            ..
        } = self;
        advance_bridge_sprite_state(bridge_sprite_entities, SHIP_VIEW_TRANSITION_ENTITY_INDEX)
            .context("transitioning the ship-view artwork entity")?;

        let profile = profiles
            .current()
            .context("ship HUD reset requires a loaded BloodScript profile")?;
        let current = profile
            .builtins()
            .archetype
            .context("loaded BloodScript profile has no arche object")?;
        let selection = select_ship_view_artwork(
            profile.directory(),
            profile.state(),
            current,
            world_artwork_layout,
        )
        .context("selecting ship-view artwork for the current position")?;

        if let (Some(request), Some(placement)) =
            (selection.resource_request, selection.entity_placement)
        {
            let resource = ResourceId::new(request.resource.value());
            let loaded = resource_cache
                .load_palette_resource(
                    data.resource_store(),
                    data.resource_catalog(),
                    resource,
                    PaletteResourceTarget::Direct,
                    live_palette,
                )
                .with_context(|| {
                    format!("loading ship-view artwork resource {}", resource.value())
                })?;
            let PaletteResourceStorage::Direct(bytes) = loaded.storage else {
                bail!("direct ship-view artwork load unexpectedly returned cached storage");
            };
            let activated = activate_bridge_sprite_from_resource(
                bridge_sprite_entities,
                usize::from(placement.entity.value()),
                resource,
                &bytes,
                BridgeSpritePosition {
                    x: placement.position[0] as u16,
                    y: placement.position[1] as u16,
                },
                usize::from(placement.frame),
            )
            .context("activating selected ship-view artwork sprite")?;
            if !activated {
                bail!(
                    "ship-view artwork resource {} has no authored initial frame",
                    resource.value()
                );
            }
        }

        ship_hud.capture_palette_and_reset_camera(live_palette);
        Ok(selection)
    }

    /// Logical frame submitted to the modern renderer.
    pub const fn front_buffer(&self) -> &IndexedFramebuffer {
        &self.front_buffer
    }

    /// Mutably borrow the logical presentation frame.
    pub fn front_buffer_mut(&mut self) -> &mut IndexedFramebuffer {
        &mut self.front_buffer
    }

    /// Draw one compact-font line into the active flat framebuffer.
    pub fn draw_small_font_line(
        &mut self,
        text: &[u8],
        origin: FontPoint,
        color: u8,
    ) -> Result<GameFontDrawOutcome> {
        draw_small_font_text(
            self.front_buffer.pixels_mut(),
            self.data.font_resources(),
            text,
            origin,
            color,
        )
        .context("drawing compact game-font text")
    }

    /// Draw the selected presentation-choice number into the active framebuffer.
    pub fn draw_presentation_choice(&mut self, choice: PresentationChoiceNumber) -> Result<usize> {
        draw_presentation_choice_number(choice, self.front_buffer.pixels_mut())
            .map_err(|error| anyhow::anyhow!("drawing presentation choice mask: {error:?}"))
    }

    /// Logical background frame retained across scene composition.
    pub const fn back_buffer(&self) -> &IndexedFramebuffer {
        &self.back_buffer
    }

    /// Copy the complete retained background into the presentation frame.
    pub fn restore_back_buffer(&mut self) {
        self.front_buffer.copy_from(&self.back_buffer);
    }

    /// Capture the current display as the source page for the ship-depth effect.
    pub fn capture_ship_depth_source(&mut self) {
        self.back_buffer.copy_from(&self.front_buffer);
    }

    /// Clear the retained secondary surface used by first-time ship-HUD setup.
    pub fn clear_back_buffer(&mut self) {
        self.back_buffer.clear(u8::MIN);
    }

    /// Publish the full logical clip and commit the requested ship entity range.
    pub fn commit_ship_entity_geometry(
        &mut self,
        entities: Range<u16>,
    ) -> Result<BridgeSpriteCommitOutcome> {
        if entities.is_empty() {
            bail!("ship entity commit range is empty");
        }
        self.bridge_dirty_regions.snapshot_flags =
            BridgeSpriteClipSnapshotFlags::from_bits(SHIP_DIRTY_SNAPSHOT_PENDING);
        self.bridge_dirty_regions.clip_bounds = LOGICAL_DISPLAY_CLIP;
        commit_bridge_sprite_dirty_range(
            &mut self.bridge_sprite_entities,
            usize::from(entities.start),
            usize::from(entities.end - 1),
            &mut self.bridge_dirty_regions,
        )
        .context("committing ship entity geometry")
    }

    /// Restore every published dirty rectangle from the retained back buffer.
    pub fn copy_ship_dirty_regions(&mut self) -> Result<DirtyRegionCopyOutcome> {
        let Self {
            bridge_dirty_regions,
            front_buffer,
            back_buffer,
            ..
        } = self;
        copy_dirty_regions_to_display(
            true,
            &bridge_dirty_regions.regions,
            back_buffer.pixels(),
            front_buffer.pixels_mut(),
        )
        .context("copying ship dirty regions to the display")
    }

    /// Copy the recovered upper and lower ship-depth bands between flat buffers.
    pub fn compose_ship_depth_bands(&mut self, layout: ShipDepthBandLayout) -> Result<()> {
        copy_ship_depth_bands(&mut self.front_buffer, &self.back_buffer, layout)
    }

    /// Clear the complete display after the recovered travel redraw request.
    pub fn clear_ship_travel_display(&mut self) {
        self.front_buffer.clear(SHIP_TRAVEL_CLEAR_COLOR);
    }

    /// Borrow both flat framebuffers for one translated presentation operation.
    pub(super) fn presentation_buffers_mut(&mut self) -> (&mut [u8], &mut [u8]) {
        (
            self.front_buffer.pixels_mut(),
            self.back_buffer.pixels_mut(),
        )
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

    /// Load the authored `CARTE.SPR` startup resource into the flat cache.
    ///
    /// This is the semantic form of resource ID 44 loaded by `bloodprg_main`
    /// immediately after audio initialization. The palette preamble is applied
    /// to the live palette and the decoded sprite remains cache-owned.
    pub fn load_startup_cartography_resource(&mut self) -> Result<PaletteResourceLoadOutcome> {
        let catalog_name = self
            .data
            .resource_catalog()
            .name(STARTUP_CARTOGRAPHY_RESOURCE)
            .context("startup cartography resource ID is not authored")?;
        if catalog_name.as_bytes() != STARTUP_CARTOGRAPHY_RESOURCE_NAME {
            bail!(
                "startup resource {} is {}, expected {}",
                STARTUP_CARTOGRAPHY_RESOURCE.value(),
                String::from_utf8_lossy(catalog_name.as_bytes()),
                String::from_utf8_lossy(STARTUP_CARTOGRAPHY_RESOURCE_NAME)
            );
        }
        self.resource_cache
            .load_palette_resource(
                self.data.resource_store(),
                self.data.resource_catalog(),
                STARTUP_CARTOGRAPHY_RESOURCE,
                PaletteResourceTarget::Cached,
                &mut self.live_palette,
            )
            .context("loading startup CARTE.SPR resource")
    }

    /// Borrow the cached decoded `CARTE.SPR` payload after startup.
    pub fn startup_cartography_resource(&self) -> Option<&[u8]> {
        self.resource_cache.resolve(STARTUP_CARTOGRAPHY_RESOURCE)
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

    /// Load and decode the exact writable `BLOOD.SAV` slot directory once.
    pub fn load_save_slot_directory(&mut self) -> Result<RuntimeAssetLoadStatus> {
        if self.save_slots.is_some() {
            return Ok(RuntimeAssetLoadStatus::AlreadyLoaded);
        }
        let name = save_slot_directory_resource_name()?;
        let bytes = self
            .data
            .resource_store()
            .load_writable(&name)
            .context("loading writable BLOOD.SAV slot directory")?;
        self.save_slots = Some(
            OriginalSaveSlotDirectory::decode(&bytes)
                .context("decoding writable BLOOD.SAV slot directory")?,
        );
        Ok(RuntimeAssetLoadStatus::LoadedNow)
    }

    /// Borrow the loaded ten-slot save directory.
    pub const fn save_slots(&self) -> Option<&OriginalSaveSlotDirectory> {
        self.save_slots.as_ref()
    }

    /// Mutably borrow the loaded save directory for the translated editor.
    pub fn save_slots_mut(&mut self) -> Option<&mut OriginalSaveSlotDirectory> {
        self.save_slots.as_mut()
    }

    /// Persist the complete loaded slot directory to the writable data root.
    pub fn persist_save_slot_directory(&self) -> Result<usize> {
        let directory = self
            .save_slots
            .as_ref()
            .context("save-slot directory has not been loaded")?;
        self.data
            .resource_store()
            .write_loose(&save_slot_directory_resource_name()?, &directory.encode())
            .context("writing writable BLOOD.SAV slot directory")
    }

    /// Read one validated save filename strictly from the writable data root.
    pub fn load_save_file(&self, filename: &[u8]) -> Result<Option<Box<[u8]>>> {
        let name = BloodResourceName::new(filename).context("validating save filename")?;
        let store = self.data.resource_store();
        if !store.writable_resource_exists(&name)? {
            return Ok(None);
        }
        store
            .load_writable(&name)
            .map(Some)
            .with_context(|| format!("loading writable save {}", save_name(&name)))
    }

    /// Create or replace one validated save file below the writable data root.
    pub fn write_save_file(&self, filename: &[u8], data: &[u8]) -> Result<usize> {
        let name = BloodResourceName::new(filename).context("validating save filename")?;
        self.data
            .resource_store()
            .write_loose(&name, data)
            .with_context(|| format!("writing writable save {}", save_name(&name)))
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

fn copy_ship_depth_bands(
    destination: &mut IndexedFramebuffer,
    source: &IndexedFramebuffer,
    layout: ShipDepthBandLayout,
) -> Result<()> {
    let rows = layout
        .logical_rows()
        .context("ship-depth layout does not describe one flat logical framebuffer")?;
    let row_count = usize::from(rows.row_count);
    if row_count > LOGICAL_FRAMEBUFFER_HALF_HEIGHT {
        bail!("ship-depth band exceeds half of the logical framebuffer");
    }
    let upper_source = framebuffer_row_span(rows.upper_source_start, rows.row_count)?;
    let lower_source = framebuffer_row_span(rows.lower_source_start, rows.row_count)?;
    let upper_destination = framebuffer_row_span(rows.upper_destination_start, rows.row_count)?;
    let lower_destination = framebuffer_row_span(rows.lower_destination_start, rows.row_count)?;

    destination.pixels_mut()[upper_destination].copy_from_slice(&source.pixels()[upper_source]);
    destination.pixels_mut()[lower_destination].copy_from_slice(&source.pixels()[lower_source]);
    Ok(())
}

fn framebuffer_row_span(start: u16, count: u16) -> Result<std::ops::Range<usize>> {
    let start = usize::from(start);
    let count = usize::from(count);
    let end = start
        .checked_add(count)
        .context("logical framebuffer row span overflowed")?;
    if end > LOGICAL_FRAMEBUFFER_HEIGHT {
        bail!("logical framebuffer row span {start}..{end} exceeds the display");
    }
    Ok(start * LOGICAL_FRAMEBUFFER_WIDTH..end * LOGICAL_FRAMEBUFFER_WIDTH)
}

fn save_slot_directory_resource_name() -> Result<BloodResourceName> {
    BloodResourceName::new(SAVE_SLOT_DIRECTORY_RESOURCE_NAME)
        .context("validating BLOOD.SAV resource name")
}

fn save_name(name: &BloodResourceName) -> String {
    String::from_utf8_lossy(name.as_bytes()).into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use commander_blood_formats::script::ScriptObjectKind;

    use crate::native::bloodprg::ResourceLoadStatus;

    use super::super::{OriginalGameData, OriginalGameDataPaths, VGA_BIOS_FONT_8X8};
    use super::*;

    const TEST_BACKGROUND_COLOR: u8 = 73;
    const EXPECTED_PAUSE_LABEL_PIXEL_COUNT: usize = 46;
    const TEST_SAVE_FILENAME: &[u8] = b"game9.sav";
    const MISSING_SAVE_FILENAME: &[u8] = b"game8.sav";
    const TEST_SAVE_BYTES: &[u8] = b"flat save storage";
    const INITIAL_SCRIPT_PROFILE: u8 = 0;
    const CLOSED_SHIP_DEPTH: u16 = u16::MIN;
    const FULLY_OPEN_SHIP_DEPTH: u16 = 65;
    const SHIP_DEPTH_TRANSITION_HOLD: u16 = 10;
    const CLOSED_SHIP_BAND_ROW_COUNT: usize = 35;
    const CLOSED_SHIP_UPPER_SOURCE_START: usize = 65;
    const CLOSED_SHIP_LOWER_DESTINATION_START: usize = 165;
    const SHIP_DEPTH_SOURCE_SPLIT: usize = 100;
    const UNTOUCHED_SHIP_DEPTH_PIXEL: u8 = u8::MAX;
    static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(u64::MIN);

    struct TemporaryRoot(PathBuf);

    impl TemporaryRoot {
        fn create() -> Self {
            let sequence = TEMPORARY_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "commander-blood-save-storage-test-{}-{sequence}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for TemporaryRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

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
    fn ship_depth_bands_copy_captured_rows_without_vga_page_emulation() {
        let mut source = IndexedFramebuffer::new();
        for (row_index, row) in source
            .pixels_mut()
            .chunks_exact_mut(LOGICAL_FRAMEBUFFER_WIDTH)
            .enumerate()
        {
            row.fill(u8::try_from(row_index).unwrap());
        }
        let mut destination = IndexedFramebuffer::new();
        destination.clear(UNTOUCHED_SHIP_DEPTH_PIXEL);
        let mut percent = u16::MIN;
        let closed_layout = crate::native::bloodprg::prepare_ship_depth_band(
            u16::from(true),
            CLOSED_SHIP_DEPTH,
            SHIP_DEPTH_TRANSITION_HOLD,
            &mut percent,
            u16::MIN,
        )
        .unwrap();

        copy_ship_depth_bands(&mut destination, &source, closed_layout).unwrap();

        for (row_index, row) in destination
            .pixels()
            .chunks_exact(LOGICAL_FRAMEBUFFER_WIDTH)
            .enumerate()
        {
            let expected = if row_index < CLOSED_SHIP_BAND_ROW_COUNT {
                u8::try_from(row_index + CLOSED_SHIP_UPPER_SOURCE_START).unwrap()
            } else if row_index >= CLOSED_SHIP_LOWER_DESTINATION_START {
                u8::try_from(
                    SHIP_DEPTH_SOURCE_SPLIT + row_index - CLOSED_SHIP_LOWER_DESTINATION_START,
                )
                .unwrap()
            } else {
                UNTOUCHED_SHIP_DEPTH_PIXEL
            };
            assert!(row.iter().all(|pixel| *pixel == expected));
        }

        destination.clear(UNTOUCHED_SHIP_DEPTH_PIXEL);
        let open_layout = crate::native::bloodprg::prepare_ship_depth_band(
            u16::from(true),
            FULLY_OPEN_SHIP_DEPTH,
            SHIP_DEPTH_TRANSITION_HOLD,
            &mut percent,
            u16::MIN,
        )
        .unwrap();
        copy_ship_depth_bands(&mut destination, &source, open_layout).unwrap();
        assert_eq!(destination, source);
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

    #[test]
    fn startup_cartography_resource_is_palette_decoded_and_cache_owned() {
        let Some(data) = original_game_data() else {
            return;
        };
        let mut runtime = OriginalGameRuntime::new(data);
        assert!(runtime.startup_cartography_resource().is_none());

        let first = runtime.load_startup_cartography_resource().unwrap();
        assert_eq!(
            first.storage,
            PaletteResourceStorage::Cached(ResourceLoadStatus::LoadedNow)
        );
        assert!(!runtime.startup_cartography_resource().unwrap().is_empty());

        let second = runtime.load_startup_cartography_resource().unwrap();
        assert_eq!(
            second.storage,
            PaletteResourceStorage::Cached(ResourceLoadStatus::AlreadyLoaded)
        );
    }

    #[test]
    fn save_storage_uses_the_startup_prepared_writable_root_exclusively() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        runtime
            .prepare_startup_resources(&VGA_BIOS_FONT_8X8, |_frame, _palette| Ok(()))
            .unwrap();
        let directory_name = save_slot_directory_resource_name().unwrap();
        let copied_directory = runtime
            .data()
            .resource_store()
            .load_writable(&directory_name)
            .unwrap();

        assert_eq!(
            runtime.load_save_slot_directory().unwrap(),
            RuntimeAssetLoadStatus::LoadedNow
        );
        assert_eq!(
            runtime.save_slots().unwrap().encode().as_slice(),
            copied_directory.as_ref()
        );
        assert_eq!(
            runtime.load_save_slot_directory().unwrap(),
            RuntimeAssetLoadStatus::AlreadyLoaded
        );
        assert_eq!(
            runtime.persist_save_slot_directory().unwrap(),
            copied_directory.len()
        );

        assert_eq!(
            runtime
                .write_save_file(TEST_SAVE_FILENAME, TEST_SAVE_BYTES)
                .unwrap(),
            TEST_SAVE_BYTES.len()
        );
        assert_eq!(
            runtime
                .load_save_file(TEST_SAVE_FILENAME)
                .unwrap()
                .as_deref(),
            Some(TEST_SAVE_BYTES)
        );
        assert_eq!(runtime.load_save_file(MISSING_SAVE_FILENAME).unwrap(), None);
    }

    #[test]
    fn shipped_destination_selects_and_activates_real_ship_view_artwork() {
        let Some(data) = original_game_data() else {
            return;
        };
        let mut runtime = OriginalGameRuntime::new(data);
        runtime
            .load_profile(ScriptProfileId::new(INITIAL_SCRIPT_PROFILE).unwrap())
            .unwrap();

        let layout_names = runtime
            .data()
            .world_artwork_layout()
            .iter()
            .map(|layout| layout.name().to_vec())
            .collect::<Vec<_>>();
        let (current, destination_position) = {
            let profile = runtime.current_profile().unwrap();
            let current = profile.builtins().archetype.unwrap();
            let destination = profile
                .directory()
                .active_objects()
                .find_map(|(object, entry)| {
                    let state_object = profile.state().object(object)?;
                    (object != current
                        && state_object.kind != ScriptObjectKind::BlackHole
                        && layout_names.iter().any(|name| name == entry.name()))
                    .then_some((object, state_object.kind))
                })
                .expect("profile must contain a direct-position world-artwork object");
            let byte_offset = crate::native::bloodprg::script_field_offset(
                destination.1,
                crate::native::bloodprg::ScriptFieldSelector::NAVIGATION_POSITION,
            )
            .expect("destination must have a navigation position");
            let position_field = profile
                .state()
                .object_word_pair(destination.0, byte_offset / std::mem::size_of::<u16>())
                .unwrap();
            (current, profile.state().word_pair(position_field).unwrap())
        };
        {
            let profile = runtime.current_profile_mut().unwrap();
            let current_kind = profile.state().object(current).unwrap().kind;
            let byte_offset = crate::native::bloodprg::script_field_offset(
                current_kind,
                crate::native::bloodprg::ScriptFieldSelector::NAVIGATION_POSITION,
            )
            .unwrap();
            let position_field = profile
                .state()
                .object_word_pair(current, byte_offset / std::mem::size_of::<u16>())
                .unwrap();
            assert!(
                profile
                    .state_mut()
                    .set_word_pair(position_field, destination_position)
            );
        }

        let selection = runtime.reset_ship_hud().unwrap();
        assert_eq!(
            usize::from(selection.transitioned_entity.value()),
            SHIP_VIEW_TRANSITION_ENTITY_INDEX
        );
        let placement = selection
            .entity_placement
            .expect("destination must select world artwork");
        assert!(selection.resource_request.is_some());
        let entity = &runtime.bridge_sprite_entities()[usize::from(placement.entity.value())];
        assert!(entity.flags.is_visible());
        assert!(entity.frame.is_some());
        assert_eq!(
            entity.draw_position,
            BridgeSpritePosition {
                x: placement.position[0] as u16,
                y: placement.position[1] as u16,
            }
        );
        assert_eq!(
            runtime.ship_hud().camera,
            crate::native::bloodprg::SHIP_CAMERA_RESET
        );
        assert_eq!(
            runtime.ship_hud().palette_snapshot.as_slice(),
            &runtime.live_palette()[crate::native::bloodprg::SHIP_HUD_PALETTE_FIRST
                ..crate::native::bloodprg::SHIP_HUD_PALETTE_FIRST
                    + crate::native::bloodprg::SHIP_HUD_PALETTE_COLOR_COUNT]
        );
    }

    fn original_game_data() -> Option<OriginalGameData> {
        original_data_paths().and_then(|paths| {
            OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).ok()
        })
    }

    fn original_data_paths() -> Option<OriginalGameDataPaths> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        [
            workspace_root.join("output/_tmp_iso"),
            workspace_root.join("commander-blood-audio/_tmp_iso"),
            workspace_root.join("accuracy/cblood_install/cblood"),
        ]
        .into_iter()
        .find_map(|root: PathBuf| OriginalGameDataPaths::from_root(root).ok())
    }
}
