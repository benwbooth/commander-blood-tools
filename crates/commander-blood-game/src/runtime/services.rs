//! Concrete runtime services assembled for the recovered top-level lifecycle.

use std::ops::Range;

use anyhow::{Context, Result, bail};
use commander_blood_formats::alien::AlienAsset;
use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::bloodprg::{BloodprgFontResources, decode_bloodprg_bridge_resources};
use commander_blood_formats::descript::DescriptBackgroundSlot;
use commander_blood_formats::instruction::ScriptTextWord;
use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};
use commander_blood_formats::script::{ScriptObjectId, ScriptObjectKind, ScriptWordId};
use sdl3::AudioSubsystem;
use sdl3::video::Window;

use crate::native::alien::AlienSceneFrame;
use crate::native::bloodprg::{
    AudioClipRequest, AudioEventContext, AudioEventState, AudioPlaybackBanks,
    BRIDGE_CONSOLE_TINT_FIRST, BRIDGE_DARK_PALETTE_ADJUSTMENT, BRIDGE_SPRITE_ENTITY_COUNT,
    BridgeActorPresentationState, BridgePageBackend, BridgePageState, BridgePageTarget,
    BridgePaletteAdjustment, BridgeScene, BridgeSceneFrame, BridgeSceneInput,
    BridgeScreenInitializationBackend, BridgeScreenInitializationState, BridgeSpriteCommitOutcome,
    BridgeSpriteRasterOutcome, BridgeSteeringInteraction, BridgeSteeringOutcome,
    CameraPageFlipOutcome, CdAudioPreparationOutcome, CdAudioState, ChoiceListConfig,
    ChoiceListFrame, ChoiceListHandAnimation, ChoiceListHandRequest, ChoiceListPointer,
    ChoiceListState, ConfirmDialogOutcome, ConfirmDialogState, DescriptMusicSelectionOutcome,
    DescriptRecordApplication, DirtyRegionCopyOutcome, FontPoint, FontVerticalBand, GameFontFace,
    GameLifecycleState, GamePresentationOwner, GameSceneLink, IndexedGamePalette,
    InlineMenuRevealOutcome, InlineMenuTextMetrics, InputAction, InputCancellationOutcome,
    InputCancellationState, LoadedSoundBank, Manu3AnimationSelector, Manu3HandFrameContext,
    Manu3HandFrameState, NAV_ACTOR_SLOT_COUNT, NameAreaEffectOutcome, NavActorSlot,
    NavActorSlotUpdateOutcome, OriginalSaveGame, PbmDecodeResult, PointerButtonEdges,
    PointerButtons, PointerSample, PresentationBridgeMode, PresentationChoiceNumber,
    PresentationHitAreas, PresentationHitRectangle, PresentationHitSelection,
    PresentationHoverOutcome, PresentationHoverState, PresentationPresentPolicy,
    PresentationQueueClockGates, PresentationQueueServiceOutcome, PresentationResourceId,
    PresentationResourceSequenceOutcome, PresentationSceneDispatchOutcome,
    PresentationScreenOutcome, PresentationScreenState, PresentationWordChoiceOutcome,
    RasterRectOutcome, SCENE_PALETTE_CLEAR_COLOR_COUNT, SHIP_CAMERA_RESET, SaveLoadMenuPhase,
    SceneTransitionState, ScriptActionRuntimeState, ScriptActionState, ScriptClock,
    ScriptFrameOutcome, ScriptObjectFlag, ScriptPresentationEntity, ScriptPresentationScanState,
    ScriptProfileId, ScriptProfileLoadOutcome, ScriptShipNavigationMode, ScriptTravelActionPhase,
    ShipDepthTransitionOutcome, ShipHudInitializationContext, ShipPresentationOutcome,
    ShipPresentationState, ShipProjectionResources, ShipTargetSelectionState, ShipViewEntityId,
    SoundBankUsage, SpeakerGateAction, StartupPreparationOutcome, TextPresentationState,
    clear_scene_palette_entries, draw_planar_dialogue_text, fill_display_band,
    increment_object_access_counters, initialize_bridge_screen, load_sound_bank,
    measure_game_text_width, object_has_flag, objects_at_arche_position, play_cd_audio_track_two,
    prepare_cd_audio, presentable_navigation_objects, process_audio_events, render_bridge_page,
    reveal_inline_menu_step, stop_cd_audio, update_manu3_hand_frame,
    update_presentation_bridge_mode, update_presentation_hover,
};
use crate::native::manu3::animation::CursorPosition;
use crate::native::random::BloodPrng;

use super::bridge_actors::RuntimeBridgeActors;
use super::bridge_console::RuntimeBridgeConsole;
use super::camera_navigation::RuntimeCameraNavigation;
use super::choice_list::{
    RuntimeChoiceListStyle, draw_choice_list_rows, prepare_choice_list_frame,
};
use super::input::INITIAL_LOGICAL_POINTER;
use super::navigation_chart::RuntimeNavigationChart;
use super::navigation_status::RuntimeNavigationStatus;
use super::presentation::RuntimeBridgeComposition;
use super::presentation_screen::RuntimeSceneTransitionDispatchContext;
use super::ship_presentation::update_runtime_ship_presentation as run_runtime_ship_presentation;
use super::ship_target::ship_hud_arche_link;
use super::{
    LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_PIXEL_COUNT, LOGICAL_FRAMEBUFFER_WIDTH,
    OriginalGameData, OriginalGameRuntime, RuntimeAlienOverlayCycle, RuntimeAssetLoadStatus,
    RuntimeAudioHost, RuntimeConfirmDialog, RuntimeInputHost, RuntimePaletteTransition,
    RuntimePaletteTransitionConfig, RuntimePaletteTransitionOutcome, RuntimePlatformHost,
    RuntimePresentationCatalog, RuntimePresentationHost, RuntimePresentationPlayer,
    RuntimePresentationQueueMetrics, RuntimePresentationScreen, RuntimePresentationStepOutcome,
    RuntimePresentationWordChoice, RuntimeSaveLoad, RuntimeSceneTransition, RuntimeScriptBackend,
    RuntimeScriptCommand, RuntimeScriptSystem, RuntimeShipHud, RuntimeShipNavigation,
    RuntimeShipTargetSelection, RuntimeShipTargetSelector, RuntimeSubtitleReveal,
    VGA_BIOS_FONT_8X8, initialize_and_restore_original_save_game,
};

const MUSIC_RESOURCE_DIRECTORY: &[u8] = b"MU\\";
const SOUND_BANK_RESOURCE_DIRECTORY: &[u8] = b"SN\\";
const DEFAULT_BRIDGE_SOUND_BANK: &[u8] = b"tb.snd";
const RADIO_SOUND_BANK: &[u8] = b"radio.snd";
const FULL_LOGICAL_FONT_BAND: FontVerticalBand = FontVerticalBand {
    top: 0,
    bottom: LOGICAL_FRAMEBUFFER_HEIGHT as i32 - 1,
};
const MENU_WIDTH_PROBE_ORIGIN: FontPoint = FontPoint { x: 10, y: 8 };
const MENU_WIDTH_PROBE_COLOR: u8 = u8::MIN;
const CHOICE_LIST_SELECTION_SOUND_CLIP: u8 = u8::MIN;
const NO_BRIDGE_HORIZONTAL_MOTION: i32 = 0;
const SHIP_HUD_PALETTE_TRANSITION_INCREMENT: u16 = 10;
const NAVIGATION_BACKGROUND_SLOT: u8 = 1;
const SHIP_NAVIGATION_ACTIVE_FLAGS: u16 = 9;
const SHIP_PRESENTATION_ACTIVE_FLAG: u16 = 1;
const SHIP_PRESENTATION_HUD_FLAG: u16 = 8;
const BRIDGE_REDRAW_REQUESTED: u8 = 1;
const SHIP_NAVIGATION_STATUS_LINE: u16 = 3;
const NAVIGATION_PALETTE_TRANSITION_INCREMENT: u16 = 10;
const FIRST_SHIP_PROJECTION_ENTITY: u16 = 21;
const AFTER_LAST_SHIP_PROJECTION_ENTITY: u16 = BRIDGE_SPRITE_ENTITY_COUNT as u16;
const FIRST_TRANSITION_ENTITY: u16 = 20;
const NAME_AREA_EFFECT_ENTITY_INDEX: usize = 2;
const NAME_AREA_PALETTE_FIRST: usize = 224;
const NAME_AREA_PALETTE_AFTER_LAST: usize = 240;
const RECOVERED_PRESENTATION_MODE_BLOCKED: bool = false;
const PRIMARY_PRESENTATION_ACTOR_SLOT: usize = 0;
const SECONDARY_PRESENTATION_ACTOR_SLOT: usize = 2;
const DISABLED_PRESENTATION_HIT_RECT: PresentationHitRectangle =
    PresentationHitRectangle::new([-1; 2], [-1; 2]);
const BRIDGE_ACTOR_PALETTE_COLOR_COUNT: usize = 192;
const CAMERA_PAGE_SHIP_ACTIVE_RESULT: u16 = 21;
const CAMERA_PAGE_TOGGLE_BIT: u16 = 2;
const INPUT_CANCEL_SHIP_BLOCK: u16 = 4;
const SCRIPT2_PROFILE_VALUE: u8 = 1;
const SCRIPT2_PTERRA_UNLOCK_STATE_OFFSET: u16 = 0x12C2;
const SCRIPT2_INIT_PROCEDURE_NAME: &[u8] = b"init";
const PTERRA_OBJECT_NAME: &[u8] = b"Pterra";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RandomDrawAudit {
    startup_point_cloud: u64,
    script: u64,
    audio: u64,
    name_area_effect: u64,
    presentation_noise: u64,
}

const fn random_draw_delta(before: u8, after: u8) -> u64 {
    after.wrapping_sub(before) as u64
}

/// Owned flat services that concrete `GameLifecycleHost` methods delegate to.
///
/// This type deliberately exposes only operations backed by translated logic
/// and a real host implementation. Audio, VM coordination, and save handling
/// are added here only when their complete services can be wired without a
/// placeholder path.
pub struct ModernGameServices<'window> {
    runtime: OriginalGameRuntime,
    input: RuntimeInputHost,
    presentation: RuntimePresentationHost<'window>,
    presentation_player: RuntimePresentationPlayer,
    audio: Option<RuntimeAudioHost>,
    resident_sound_bank: Option<LoadedSoundBank>,
    audio_events: AudioEventState,
    bridge_scene: Option<BridgeScene>,
    bridge_frame: Option<BridgeSceneFrame>,
    bridge_screen: BridgeScreenInitializationState,
    bridge_presentation_mode: Option<PresentationBridgeMode>,
    presentation_hover: PresentationHoverState<BridgeActorPresentationState>,
    nav_actor_slots: [NavActorSlot; NAV_ACTOR_SLOT_COUNT],
    bridge_actors: Option<RuntimeBridgeActors>,
    bridge_console: Option<RuntimeBridgeConsole>,
    camera_navigation: Option<RuntimeCameraNavigation>,
    navigation_chart: Option<RuntimeNavigationChart>,
    navigation_status: Option<RuntimeNavigationStatus>,
    presentation_screen: Option<RuntimePresentationScreen>,
    presentation_word_choice: Option<RuntimePresentationWordChoice>,
    save_load: Option<RuntimeSaveLoad>,
    ship_hud: Option<RuntimeShipHud>,
    ship_navigation: Option<RuntimeShipNavigation>,
    scene_transition: Option<RuntimeSceneTransition>,
    alien_overlay: Option<RuntimeAlienOverlayCycle>,
    ship_target_selector: Option<RuntimeShipTargetSelector>,
    choice_list_style: RuntimeChoiceListStyle,
    subtitle_reveal: Option<RuntimeSubtitleReveal>,
    palette_transition: RuntimePaletteTransition,
    bridge_palette: IndexedGamePalette,
    confirm_dialog: RuntimeConfirmDialog,
    manu3_hand: Manu3HandFrameState,
    manu3_previous_animation: Manu3AnimationSelector,
    ship_presentation: ShipPresentationState,
    random: BloodPrng,
    random_draws: RandomDrawAudit,
    game_timer_tick: u16,
    scripts: RuntimeScriptSystem,
    script_finale_shutdown_pending: bool,
    alien_overlay_mouse_idle_reset_pending: bool,
    cd_audio: CdAudioState,
    main_viewport_configured: bool,
}

fn synchronize_selected_ship_target(
    selected_target: Option<ScriptObjectId>,
    action: &mut ScriptActionState,
    ship: &mut ShipPresentationState,
) {
    let Some(target) = selected_target else {
        return;
    };

    action.current_ship_target = Some(target);
    action.ship_navigation_mode = ScriptShipNavigationMode::Active;
    ship.flags = SHIP_NAVIGATION_ACTIVE_FLAGS;
    ship.bridge_redraw_pending = u8::MIN;
    ship.active_line = SHIP_NAVIGATION_STATUS_LINE;
}

fn publish_presentation_screen_modal_ui(
    lifecycle: &mut GameLifecycleState,
    redraw_requested: bool,
) {
    lifecycle.set_modal_ui_busy(redraw_requested);
}

fn latch_script_finale_completion(
    shutdown_pending: &mut bool,
    finale_requested: bool,
    active_line_before: Option<u16>,
    active_line_after: Option<u16>,
) {
    *shutdown_pending |=
        finale_requested && active_line_before.is_some() && active_line_after.is_none();
}

impl<'window> ModernGameServices<'window> {
    /// Allocate flat game state and an artwork-only loading renderer.
    pub fn new(
        window: &'window Window,
        data: OriginalGameData,
        script_clock: ScriptClock,
    ) -> Result<Self> {
        let confirm_dialog = RuntimeConfirmDialog::new(*data.confirm_dialog_regions());
        let initial_text_speed_step = data.bridge_menu_text().initial_text_speed_step();
        let bridge_console = RuntimeBridgeConsole::new(initial_text_speed_step);
        let scripts = RuntimeScriptSystem::new(&data, script_clock);
        let presentation_player = RuntimePresentationPlayer::new(data.presentation_catalog());
        let startup_palette = *data.default_vga_palette();
        let runtime = OriginalGameRuntime::new(data);
        let bridge_palette = startup_palette;
        let presentation_screen = RuntimePresentationScreen::new(startup_palette)?;
        let presentation = RuntimePresentationHost::new_startup(window, &runtime)?;
        Ok(Self {
            runtime,
            input: RuntimeInputHost::new(INITIAL_LOGICAL_POINTER),
            presentation,
            presentation_player,
            audio: None,
            resident_sound_bank: None,
            audio_events: AudioEventState {
                playback_enabled: false,
                menu_words_pending: false,
                dialogue_armed: false,
                voice_reaction_requested: false,
                voice_cooldown: u8::MIN,
                dialogue_delay: u16::MIN,
                dialogue_seed: u16::MIN,
                last_clip: u16::MIN,
            },
            bridge_scene: None,
            bridge_frame: None,
            bridge_screen: BridgeScreenInitializationState::default(),
            bridge_presentation_mode: None,
            presentation_hover: PresentationHoverState::new(
                false,
                BridgeActorPresentationState::Unchanged,
                BridgeActorPresentationState::Unchanged,
            ),
            nav_actor_slots: [NavActorSlot::default(); NAV_ACTOR_SLOT_COUNT],
            bridge_actors: Some(RuntimeBridgeActors::default()),
            bridge_console: Some(bridge_console),
            camera_navigation: Some(RuntimeCameraNavigation::default()),
            navigation_chart: Some(RuntimeNavigationChart::default()),
            navigation_status: Some(RuntimeNavigationStatus::default()),
            presentation_screen: Some(presentation_screen),
            presentation_word_choice: Some(RuntimePresentationWordChoice::default()),
            save_load: Some(RuntimeSaveLoad::default()),
            ship_hud: Some(RuntimeShipHud::default()),
            ship_navigation: Some(RuntimeShipNavigation::default()),
            scene_transition: Some(RuntimeSceneTransition::default()),
            alien_overlay: Some(RuntimeAlienOverlayCycle::default()),
            ship_target_selector: Some(RuntimeShipTargetSelector::default()),
            choice_list_style: RuntimeChoiceListStyle::default(),
            subtitle_reveal: Some(RuntimeSubtitleReveal::new(initial_text_speed_step)),
            palette_transition: RuntimePaletteTransition::default(),
            bridge_palette,
            confirm_dialog,
            manu3_hand: Manu3HandFrameState::default(),
            manu3_previous_animation: Manu3AnimationSelector::Neutral,
            ship_presentation: ShipPresentationState::default(),
            random: BloodPrng::default(),
            random_draws: RandomDrawAudit::default(),
            game_timer_tick: u16::MIN,
            scripts,
            script_finale_shutdown_pending: false,
            alien_overlay_mouse_idle_reset_pending: false,
            cd_audio: CdAudioState::default(),
            main_viewport_configured: false,
        })
    }

    /// Draw and present `LOADING`, then populate missing writable resources.
    pub fn prepare_startup_resources(&mut self) -> Result<StartupPreparationOutcome> {
        let runtime = &mut self.runtime;
        let presentation = &mut self.presentation;
        runtime.prepare_startup_resources(&VGA_BIOS_FONT_8X8, |frame, palette| {
            presentation.submit_frame(frame, palette)?;
            presentation.present_artwork(&[])
        })
    }

    /// Decode the authored MANU3 overlay exactly once.
    pub fn load_manu3_overlay(&mut self) -> Result<RuntimeAssetLoadStatus> {
        self.runtime.load_manu3()
    }

    /// Recreate GPU resources for bridge rendering and MANU3 composition.
    pub fn initialize_logical_viewport(&mut self) -> Result<()> {
        if self.runtime.manu3().is_none() {
            bail!("MANU3 must be loaded before the main logical viewport");
        }
        self.presentation
            .configure_main_game(&self.runtime)
            .context("configuring main logical viewport")?;
        self.main_viewport_configured = true;
        Ok(())
    }

    /// Decode the complete bridge panorama archive into owned flat storage.
    pub fn open_bridge_panorama(&mut self) -> Result<RuntimeAssetLoadStatus> {
        self.runtime.open_bridge_panorama()
    }

    /// Construct the live bridge and consume its exact startup PRNG sequence.
    pub fn initialize_bridge_scene(&mut self, packed_clock_seed: u8) -> Result<()> {
        if self.bridge_scene.is_some() {
            bail!("bridge scene is already initialized");
        }
        self.runtime
            .rebuild_bridge_sprite_remap_tables()
            .context("building bridge sprite remap tables")?;
        let resources = decode_bloodprg_bridge_resources(self.runtime.data().executable())
            .context("decoding bridge projection resources")?;
        let nav_actor_slots = resources
            .nav_actor_records
            .map(NavActorSlot::from_executable_record);
        let panorama = self
            .runtime
            .take_bridge_panorama()
            .context("bridge panorama must be opened before scene initialization")?;
        self.random.seed_from_clock_register(packed_clock_seed);
        let random_counter_before = self.random.counter;
        let bridge_scene = BridgeScene::new(
            panorama,
            ShipProjectionResources::from(resources),
            &mut self.random,
        )
        .context("constructing live bridge scene")?;
        self.random_draws.startup_point_cloud +=
            random_draw_delta(random_counter_before, self.random.counter);
        self.nav_actor_slots = nav_actor_slots;
        self.bridge_scene = Some(bridge_scene);
        Ok(())
    }

    /// Mark one inclusive camera-transition entity range dirty.
    pub(super) fn mark_camera_transition_entities(
        &mut self,
        first: u16,
        last: u16,
    ) -> Result<usize> {
        let end = last
            .checked_add(1)
            .context("camera-transition entity range ends at u16::MAX")?;
        self.runtime.mark_ship_entity_geometry_dirty(first..end)
    }

    /// Clear the recovered navigation band before projection work.
    pub(super) fn clear_camera_projection_band(&mut self, color: u8) -> Result<()> {
        self.runtime.clear_ship_projection_band(color)
    }

    /// Apply the camera coordinator's flat pose and build its projection matrix.
    pub(super) fn build_camera_projection_matrix(
        &mut self,
        camera: [i16; 3],
        projection_angle: u16,
    ) -> Result<()> {
        let scene = self
            .bridge_scene
            .as_mut()
            .context("camera projection requires an initialized bridge scene")?;
        scene.set_camera_approach_pose(camera, projection_angle);
        scene
            .build_camera_projection_matrix()
            .context("building the camera-transition projection matrix")
    }

    /// Project the bridge point cloud through the prepared camera matrix.
    pub(super) fn project_camera_point_cloud(&mut self) -> Result<()> {
        self.bridge_scene
            .as_mut()
            .context("camera projection requires an initialized bridge scene")?
            .project_camera_point_cloud()
            .context("projecting the camera-transition point cloud")
    }

    /// Project ship entities through the prepared camera matrix.
    pub(super) fn project_camera_object_sprites(&mut self) -> Result<()> {
        let Self {
            runtime,
            bridge_scene,
            ..
        } = self;
        bridge_scene
            .as_mut()
            .context("camera projection requires an initialized bridge scene")?
            .project_camera_object_sprites(runtime.bridge_sprite_entities_mut())
            .context("projecting camera-transition ship objects")
    }

    /// Synchronize a non-rendering camera phase with the live bridge scene.
    pub(super) fn set_camera_approach_pose(
        &mut self,
        camera: [i16; 3],
        projection_angle: u16,
    ) -> Result<()> {
        self.bridge_scene
            .as_mut()
            .context("camera transition requires an initialized bridge scene")?
            .set_camera_approach_pose(camera, projection_angle);
        Ok(())
    }

    /// Capture the HUD palette and restore both camera-state representations.
    pub(super) fn snapshot_camera_transition_hud(&mut self) -> Result<[i16; 3]> {
        self.snapshot_navigation_hud_palette_and_camera()?;
        Ok(SHIP_CAMERA_RESET)
    }

    /// Decode `CHART.FD` and restore it into the current presentation frame.
    pub fn initialize_back_buffer(&mut self) -> Result<PbmDecodeResult> {
        let result = self.runtime.initialize_back_buffer()?;
        self.runtime.restore_back_buffer();
        Ok(result)
    }

    /// Load the exact writable `BLOOD.SAV` directory prepared during startup.
    pub fn load_save_slots(&mut self) -> Result<RuntimeAssetLoadStatus> {
        self.runtime.load_save_slot_directory()
    }

    /// Open and resume the default SDL3 playback stream exactly once.
    pub fn initialize_audio(&mut self, audio: &AudioSubsystem) -> Result<()> {
        if self.audio.is_some() {
            bail!("runtime audio is already initialized");
        }
        self.audio = Some(RuntimeAudioHost::open(audio)?);
        Ok(())
    }

    /// Prepare optional physical-track metadata when a modern source is available.
    ///
    /// The extracted DOS release does not include CD-DA track two, so startup
    /// currently selects the native unavailable path. A future track asset can
    /// supply typed metadata here without reintroducing MSCDEX request blocks.
    pub fn prepare_optional_cd_audio(&mut self) -> CdAudioPreparationOutcome {
        prepare_cd_audio(&mut self.cd_audio, None)
    }

    /// Load and validate the authored startup `CARTE.SPR` cache resource.
    pub fn load_initial_cartography_resource(&mut self) -> Result<()> {
        let _ = self.runtime.load_startup_cartography_resource()?;
        Ok(())
    }

    /// Load and validate the default `SN\\TB.SND` resident bridge sound bank.
    pub fn load_default_sound_bank(&mut self) -> Result<()> {
        self.load_resident_sound_bank_resource(DEFAULT_BRIDGE_SOUND_BANK)
            .context("loading default bridge sound bank")
    }

    /// Load and validate the authored `SN\\RADIO.SND` streamed radio bank.
    pub fn load_radio_sound_bank(&mut self) -> Result<()> {
        self.scripts
            .backend_mut()
            .load_streamed_sound_bank(RADIO_SOUND_BANK)
            .context("loading radio bridge sound bank")?;
        self.scripts
            .backend()
            .streamed_sound_bank()
            .context("radio bridge sound bank was not retained")
            .map(|_| ())
    }

    /// Load one effects bank into the persistent resident slot used by low clip indices.
    pub(super) fn load_resident_sound_bank_resource(&mut self, bank_name: &[u8]) -> Result<()> {
        let resource_name = prefixed_resource_name(SOUND_BANK_RESOURCE_DIRECTORY, bank_name)?;
        let encoded = self
            .runtime
            .data()
            .resource_store()
            .load(&resource_name)
            .with_context(|| {
                format!(
                    "loading resident sound bank {}",
                    String::from_utf8_lossy(resource_name.as_bytes())
                )
            })?;
        self.resident_sound_bank = load_sound_bank(
            self.audio_is_initialized(),
            SoundBankUsage::ResidentEffects,
            &encoded,
        )
        .context("decoding resident sound bank")?;
        if self.resident_sound_bank.is_none() {
            bail!("resident sound bank was loaded before SDL audio initialization");
        }
        Ok(())
    }

    /// Snapshot the current resident bank while a synchronous alien overlay replaces it.
    pub(super) fn resident_sound_bank(&self) -> Result<&LoadedSoundBank> {
        self.resident_sound_bank
            .as_ref()
            .context("no resident effects sound bank is loaded")
    }

    /// Restore a resident effects bank captured before a temporary overlay.
    pub(super) fn restore_resident_sound_bank(&mut self, bank: LoadedSoundBank) {
        self.resident_sound_bank = Some(bank);
    }

    /// Decode and retain navigation music selected by the active DESCRIPT record.
    pub fn load_navigation_music(&mut self) -> Result<()> {
        if !self.navigation_music_enabled()? {
            return self.check_audio();
        }
        let music_name = self
            .scripts
            .backend()
            .assets()
            .music()
            .context("no navigation music is selected")?
            .as_bytes();
        let resource_name = prefixed_resource_name(MUSIC_RESOURCE_DIRECTORY, music_name)?;
        let normalized = self
            .runtime
            .data()
            .normalized_media()
            .load_voc(&resource_name)
            .with_context(|| {
                format!(
                    "loading normalized music resource {}",
                    String::from_utf8_lossy(resource_name.as_bytes())
                )
            })?;
        let wait_prompt = self
            .audio_mut()?
            .load_background_pcm_stream(
                &normalized.samples,
                normalized.sample_rate_hz,
                normalized.sample_rate_code,
            )
            .context("staging normalized navigation music stream")?;
        if let Some(wait_prompt) = wait_prompt {
            self.draw_audio_stream_wait_prompt(wait_prompt)?;
        }
        Ok(())
    }

    /// Start the retained navigation music as a looping background source.
    pub fn start_loaded_navigation_music(&mut self) -> Result<()> {
        self.audio_mut()?.start_background_stream()
    }

    /// Decode and start the navigation music selected by the active DESCRIPT record.
    pub fn restart_navigation_music(&mut self) -> Result<()> {
        if !self.navigation_music_enabled()? {
            return self.check_audio();
        }
        self.load_navigation_music()?;
        self.start_loaded_navigation_music()
    }

    /// Stop only the looping navigation source before replacing its resource.
    pub fn stop_navigation_music(&mut self) -> Result<()> {
        self.audio_mut()?.stop_background()
    }

    /// Return the persistent streamed-audio enable latch selected by the options menu.
    pub fn navigation_music_enabled(&self) -> Result<bool> {
        Ok(self.audio_ref()?.background_channel_active())
    }

    /// Enable or disable every native VOC stream gate without conflating it with playback state.
    pub fn set_navigation_music_enabled(&mut self, enabled: bool) -> Result<()> {
        self.audio_mut()?.set_background_channel_active(enabled)
    }

    fn draw_audio_stream_wait_prompt(&mut self, prompt: &[u8]) -> Result<()> {
        let mut subtitle = self
            .subtitle_reveal
            .take()
            .context("subtitle reveal is already being updated")?;
        let outcome = subtitle.draw_stream_wait_prompt(
            &mut self.runtime,
            self.scripts.text_presentation_mut(),
            prompt,
        );
        self.subtitle_reveal = Some(subtitle);
        outcome.map(|_| ())
    }

    /// Start retained music, or keep the current navigation stream running.
    pub fn ensure_navigation_music(&mut self) -> Result<()> {
        if !self.navigation_music_enabled()? {
            return self.check_audio();
        }
        if self.navigation_music_position()?.is_some() {
            return self.check_audio();
        }
        if !self.audio_ref()?.background_stream_pending() {
            self.load_navigation_music()?;
        }
        self.start_loaded_navigation_music()
    }

    /// Decode and play one authored clip from the currently loaded SND bank.
    pub fn play_loaded_sound_bank_clip(&mut self, clip_index: u8) -> Result<()> {
        self.play_resident_sound_bank_clip(u16::from(clip_index))
    }

    fn play_resident_sound_bank_clip(&mut self, clip_index: u16) -> Result<()> {
        self.play_sound_request(AudioClipRequest::VoiceReaction {
            bank_index: clip_index,
        })
    }

    /// Start optional physical-track-two playback for an alien encounter.
    ///
    /// Extracted DOS data normally has no CD-DA track, which preserves the
    /// original disabled MSCDEX gate. A prepared track must be bound to a real
    /// modern audio source before this method will accept its play command.
    pub(super) fn start_encounter_cd_audio(&mut self) -> Result<()> {
        if play_cd_audio_track_two(&mut self.cd_audio).is_some() {
            bail!("encounter CD track metadata is prepared without a playback source");
        }
        Ok(())
    }

    /// Stop optional encounter CD playback when a physical-track source exists.
    pub(super) fn stop_encounter_cd_audio(&mut self) -> Result<()> {
        if stop_cd_audio(&mut self.cd_audio).is_some() {
            bail!("encounter CD track metadata is prepared without a playback source");
        }
        Ok(())
    }

    /// Decode one authored Creative Voice resource into the shared native stream owner.
    pub fn load_streamed_voice_resource(&mut self, path: &[u8]) -> Result<()> {
        if !self.navigation_music_enabled()? {
            return self.check_audio();
        }
        let resource_name =
            BloodResourceName::new(path).context("validating voice resource path")?;
        let normalized = self
            .runtime
            .data()
            .normalized_media()
            .load_voc(&resource_name)
            .with_context(|| {
                format!(
                    "loading normalized voice resource {}",
                    String::from_utf8_lossy(resource_name.as_bytes())
                )
            })?;
        let wait_prompt = self.audio_mut()?.load_background_pcm_stream(
            &normalized.samples,
            normalized.sample_rate_hz,
            normalized.sample_rate_code,
        )?;
        if let Some(wait_prompt) = wait_prompt {
            self.draw_audio_stream_wait_prompt(wait_prompt)?;
        }
        Ok(())
    }

    /// Start the voice retained by [`Self::load_streamed_voice_resource`].
    pub fn start_loaded_streamed_voice(&mut self) -> Result<()> {
        if !self.navigation_music_enabled()? {
            return self.check_audio();
        }
        self.audio_mut()?.start_background_stream()
    }

    /// Replace the complete live indexed palette with black.
    pub fn clear_live_palette(&mut self) {
        self.runtime.live_palette_mut().fill([u8::MIN; 3]);
    }

    /// Stop all modern audio and clear samples already queued in SDL.
    pub fn stop_audio(&mut self) -> Result<()> {
        self.audio_mut()?.stop_all()
    }

    /// Stop native digital playback without changing the independent PC-speaker gate.
    pub fn stop_digital_audio(&mut self) -> Result<()> {
        self.audio_mut()?.stop_digital()
    }

    /// Release a streamed voice that was loaded but never started.
    pub fn discard_loaded_voice(&mut self) -> bool {
        self.audio
            .as_mut()
            .is_some_and(RuntimeAudioHost::discard_pending_background_stream)
    }

    /// Release decoded navigation music that has not yet entered SDL playback.
    pub fn discard_loaded_music(&mut self) -> bool {
        self.audio
            .as_mut()
            .is_some_and(RuntimeAudioHost::discard_pending_background_stream)
    }

    /// Advance the recovered music double-buffer lifecycle by at most one page.
    pub fn refill_navigation_music(&mut self) -> Result<()> {
        self.audio_mut()?.refill_background_stream().map(|_| ())
    }

    /// Surface asynchronous SDL audio failures on the game thread.
    pub fn check_audio(&self) -> Result<()> {
        self.audio_ref()?.check_callback()
    }

    /// Return native timer counters owned by the recovered audio-event selector.
    pub const fn audio_event_timer_counters(&self) -> (u16, u16) {
        (
            self.audio_events.voice_cooldown as u16,
            self.audio_events.dialogue_delay,
        )
    }

    /// Publish interrupt-timer decrements back to the recovered audio-event selector.
    pub fn synchronize_audio_event_timers(
        &mut self,
        voice_cooldown: u16,
        dialogue_delay: u16,
    ) -> Result<()> {
        self.audio_events.voice_cooldown = u8::try_from(voice_cooldown)
            .context("voice chatter cooldown exceeds its native byte range")?;
        self.audio_events.dialogue_delay = dialogue_delay;
        Ok(())
    }

    /// Select and play the original deterministic dialogue and chatter events.
    pub fn process_runtime_audio_events(
        &mut self,
        dialogue_suppressed: bool,
    ) -> Result<Box<[AudioClipRequest]>> {
        self.check_audio()?;

        let (menu_words_pending, dialogue_armed, voice_reaction_requested, menu_tokens) = {
            let text = self.scripts.text_presentation_mut();
            (
                std::mem::take(&mut text.dialogue_chatter_seed_pending),
                text.dialogue_chatter_active,
                text.subtitle_voice_trigger,
                text.menu_words.clone(),
            )
        };
        self.audio_events.playback_enabled = self.audio_is_initialized();
        self.audio_events.menu_words_pending |= menu_words_pending;
        self.audio_events.dialogue_armed = dialogue_armed;
        self.audio_events.voice_reaction_requested = voice_reaction_requested;

        let menu_words = self.resolve_audio_menu_words(&menu_tokens)?;
        let (clip_count, delay_base, delay_limit) = self
            .scripts
            .backend()
            .streamed_sound_bank()
            .map(|loaded| {
                let header = loaded.bank.header();
                (
                    header.clip_count,
                    header.dialogue_delay_base,
                    header.dialogue_delay_limit,
                )
            })
            .unwrap_or((u16::MIN, u8::MIN, u8::MIN));
        if self.audio_events.dialogue_armed
            && self.audio_events.dialogue_delay == u16::MIN
            && clip_count == u16::MIN
        {
            bail!("dialogue chatter is armed without a streamed DESCRIPT sound bank");
        }

        let requests = {
            let audio_events = &mut self.audio_events;
            let random = &mut self.random;
            let audio_random_draws = &mut self.random_draws.audio;
            process_audio_events(
                audio_events,
                AudioEventContext {
                    dialogue_suppressed,
                    menu_words: &menu_words,
                    streamed_dialogue_clip_count: clip_count,
                    dialogue_delay_base: delay_base,
                    dialogue_delay_limit: delay_limit,
                },
                |upper_bound| {
                    *audio_random_draws += 1;
                    random.next(upper_bound)
                },
            )
            .context("selecting dialogue and chatter audio events")?
        };
        self.scripts.text_presentation_mut().dialogue_chatter_active =
            self.audio_events.dialogue_armed;

        for request in requests.iter().copied() {
            match request {
                AudioClipRequest::StreamedDialogue { index } => {
                    self.play_streamed_dialogue_clip(index)?;
                }
                AudioClipRequest::VoiceReaction { bank_index } => {
                    self.play_resident_sound_bank_clip(bank_index)?;
                }
            }
        }
        Ok(requests)
    }

    fn resolve_audio_menu_words(&self, words: &[ScriptTextWord]) -> Result<Vec<Box<[u8]>>> {
        if words.is_empty() {
            return Ok(Vec::new());
        }
        let dictionary = self
            .runtime
            .current_profile()
            .context("dialogue chatter words require a loaded BloodScript profile")?
            .dictionary();
        words
            .iter()
            .take_while(|word| matches!(word, ScriptTextWord::Dictionary(_)))
            .map(|word| match word {
                ScriptTextWord::Dictionary(word) => {
                    dictionary.word(*word).map(Box::from).with_context(|| {
                        format!("resolving dialogue chatter dictionary word {word:?}")
                    })
                }
                ScriptTextWord::SectionSeparator => unreachable!("section separators terminate"),
            })
            .collect()
    }

    fn play_streamed_dialogue_clip(&mut self, clip_index: u16) -> Result<()> {
        self.play_sound_request(AudioClipRequest::StreamedDialogue { index: clip_index })
    }

    fn play_sound_request(&mut self, request: AudioClipRequest) -> Result<()> {
        let resident = self
            .resident_sound_bank
            .as_ref()
            .context("no resident effects sound bank is loaded")?;
        let streamed = match request {
            AudioClipRequest::VoiceReaction { .. } => self
                .scripts
                .backend()
                .streamed_sound_bank()
                .map_or(&resident.bank, |loaded| &loaded.bank),
            AudioClipRequest::StreamedDialogue { .. } => {
                &self
                    .scripts
                    .backend()
                    .streamed_sound_bank()
                    .context("no streamed DESCRIPT sound bank is loaded")?
                    .bank
            }
        };
        let audio = self
            .audio
            .as_mut()
            .context("runtime audio has not been initialized")?;
        audio.play_sound_request(
            request,
            AudioPlaybackBanks {
                resident_effects: &resident.bank,
                streamed_dialogue: streamed,
            },
        )
    }

    /// Current source-sample position of navigation music, when active.
    pub fn navigation_music_position(&self) -> Result<Option<u64>> {
        Ok(self.audio_ref()?.background_position())
    }

    /// Report whether SDL audio has completed startup initialization.
    pub const fn audio_is_initialized(&self) -> bool {
        self.audio.is_some()
    }

    /// Apply a PC-speaker gate transition through the SDL square-wave replacement.
    pub fn apply_speaker_gate(&mut self, action: SpeakerGateAction) -> Result<()> {
        self.audio_mut()?.apply_speaker_gate(action)
    }

    /// Load one complete BloodScript profile and bind its concrete runtime services.
    pub fn load_script_profile(
        &mut self,
        profile: ScriptProfileId,
    ) -> Result<ScriptProfileLoadOutcome> {
        let outcome = self.scripts.load_profile(&mut self.runtime, profile)?;
        self.ship_hud = Some(RuntimeShipHud::default());
        self.ship_navigation = Some(RuntimeShipNavigation::default());
        self.scene_transition = Some(RuntimeSceneTransition::default());
        self.ship_target_selector = Some(RuntimeShipTargetSelector::default());
        Ok(outcome)
    }

    /// Reconstruct every profile-derived record store from synchronized VAR state.
    pub fn rebuild_script_record_state(&mut self) -> Result<()> {
        let profile = self
            .runtime
            .current_profile_mut()
            .context("record-state reconstruction requires a loaded BloodScript profile")?;
        let synchronized = profile
            .synchronized_state()
            .context("synchronizing typed records into BloodScript VAR state")?;
        profile
            .replace_state(synchronized)
            .context("reconstructing typed records from BloodScript VAR state")
    }

    /// Increment all in-play navigation access counters after a profile change.
    pub fn refresh_object_access_counters(&mut self) -> Result<usize> {
        let profile = self
            .runtime
            .current_profile_mut()
            .context("object-access refresh requires a loaded BloodScript profile")?;
        Ok(increment_object_access_counters(profile.state_mut()))
    }

    /// Capture the active typed profile in the original `GAME*.SAV` format.
    pub fn capture_original_save_game(&self) -> Result<OriginalSaveGame> {
        OriginalSaveGame::capture(
            self.runtime
                .current_profile()
                .context("cannot save without a loaded BloodScript profile")?,
        )
        .map_err(Into::into)
    }

    /// Restore one original save through profile selection, initialization, and HUD rebuild.
    pub fn restore_original_save_game(
        &mut self,
        data: &[u8],
        state: &mut GameLifecycleState,
    ) -> Result<()> {
        let profile = OriginalSaveGame::decode_profile(data)
            .context("decoding the saved BloodScript profile")?;
        self.load_script_profile(profile)?;
        initialize_and_restore_original_save_game(
            &mut self.scripts,
            &mut self.runtime,
            state,
            data,
        )?;
        self.reset_ship_hud()
            .context("rebuilding the ship HUD after save restoration")?;
        state.navigation_rebuild_pending = true;
        state.navigation_transition_pending = false;
        Ok(())
    }

    /// Advance the complete recovered save/load menu against concrete runtime services.
    pub fn update_runtime_save_load(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> Result<crate::native::bloodprg::SaveLoadMenuOutcome> {
        let mut save_load = self
            .save_load
            .take()
            .context("save/load update is reentrant")?;
        let outcome = save_load.update(self, state);
        self.save_load = Some(save_load);
        outcome
    }

    /// Route one translated input action to save/load when it owns the UI.
    pub fn queue_save_load_input(&mut self, action: InputAction) -> Result<bool> {
        Ok(self
            .save_load
            .as_mut()
            .context("save/load is already being updated")?
            .queue_input(action))
    }

    /// Open the ordinary save-slot editor.
    pub fn request_save_menu(&mut self) -> Result<()> {
        self.save_load
            .as_mut()
            .context("save/load is already being updated")?
            .request_save();
        Ok(())
    }

    /// Open the ordinary load-slot selector.
    pub fn request_load_menu(&mut self) -> Result<()> {
        self.save_load
            .as_mut()
            .context("save/load is already being updated")?
            .request_load();
        Ok(())
    }

    /// Request an immediate save to the reserved tenth slot.
    pub fn request_quick_save(&mut self) -> Result<()> {
        self.save_load
            .as_mut()
            .context("save/load is already being updated")?
            .request_quick_save();
        Ok(())
    }

    /// Borrow the persistent save/load adapter state.
    pub fn runtime_save_load(&self) -> Result<&RuntimeSaveLoad> {
        self.save_load
            .as_ref()
            .context("save/load is already being updated")
    }

    /// Advance the recovered ship HUD against concrete flat runtime services.
    pub fn update_runtime_ship_hud(
        &mut self,
        scene_link: GameSceneLink,
        state: &mut GameLifecycleState,
    ) -> Result<crate::native::bloodprg::ShipHudCoordinatorOutcome> {
        let mut ship_hud = self
            .ship_hud
            .take()
            .context("ship HUD update is reentrant")?;
        let outcome = ship_hud.update(self, scene_link, state);
        self.ship_hud = Some(ship_hud);
        outcome
    }

    /// Force the concrete HUD adapter through first-time setup on its next frame.
    pub fn request_ship_hud_reinitialization(&mut self) -> Result<()> {
        self.ship_hud
            .as_mut()
            .context("ship HUD is already being updated")?
            .request_reinitialization();
        Ok(())
    }

    /// Borrow the persistent recovered ship HUD adapter state.
    pub fn runtime_ship_hud(&self) -> Result<&RuntimeShipHud> {
        self.ship_hud
            .as_ref()
            .context("ship HUD is already being updated")
    }

    /// Advance the recovered ship-navigation coordinator against concrete runtime services.
    pub fn update_runtime_ship_navigation(
        &mut self,
        state: &mut GameLifecycleState,
        platform: &mut RuntimePlatformHost<'window>,
    ) -> Result<crate::native::bloodprg::ShipNavigationOutcome> {
        let mut navigation = self
            .ship_navigation
            .take()
            .context("ship navigation update is reentrant")?;
        let outcome = navigation.update(self, state, platform);
        self.ship_navigation = Some(navigation);
        outcome
    }

    /// Borrow the persistent flat ship-navigation adapter state.
    pub fn runtime_ship_navigation(&self) -> Result<&RuntimeShipNavigation> {
        self.ship_navigation
            .as_ref()
            .context("ship navigation is already being updated")
    }

    /// Arm a contact-driven scene transition from one decoded script object.
    pub fn request_scene_transition(&mut self, target: ScriptObjectId) -> Result<()> {
        let profile = self
            .runtime
            .current_profile()
            .context("a scene transition requires a loaded BloodScript profile")?;
        let target_kind = profile
            .state()
            .object(target)
            .with_context(|| format!("scene-transition target {target:?} is absent"))?
            .kind;
        let current = profile
            .active_actor_presentation_related()
            .and_then(|object| {
                profile
                    .state()
                    .object(object)
                    .map(|record| (object, record.kind))
            });
        let mut transition = self
            .scene_transition
            .take()
            .context("scene transition is already being updated")?;
        let outcome = transition.begin(current, (target, target_kind));
        self.scene_transition = Some(transition);
        outcome
    }

    /// Advance the persistent contact scene transition by one game frame.
    pub fn update_runtime_scene_transition(
        &mut self,
        scene_link: GameSceneLink,
        state: &mut GameLifecycleState,
        platform: &mut RuntimePlatformHost<'window>,
    ) -> Result<crate::native::bloodprg::SceneTransitionOutcome> {
        let mut transition = self
            .scene_transition
            .take()
            .context("scene transition is reentrant")?;
        let outcome = transition.update(self, state, scene_link, platform);
        self.scene_transition = Some(transition);
        outcome
    }

    /// Borrow the persistent contact scene-transition adapter.
    pub fn runtime_scene_transition(&self) -> Result<&RuntimeSceneTransition> {
        self.scene_transition
            .as_ref()
            .context("scene transition is already being updated")
    }

    /// Run the recovered synchronous alien-overlay coordinator to completion.
    pub fn run_runtime_alien_overlay_cycle(
        &mut self,
        state: &mut GameLifecycleState,
        platform: &mut RuntimePlatformHost<'window>,
    ) -> Result<crate::native::bloodprg::AlienOverlayCycleOutcome> {
        let mut overlay = self
            .alien_overlay
            .take()
            .context("alien-overlay cycle is reentrant")?;
        let outcome = overlay.run(self, state, platform);
        self.alien_overlay = Some(overlay);
        outcome
    }

    /// Borrow persistent round-robin and shared overlay state.
    pub fn runtime_alien_overlay(&self) -> Result<&RuntimeAlienOverlayCycle> {
        self.alien_overlay
            .as_ref()
            .context("alien-overlay cycle is already running")
    }

    /// Publish the bridge globals restored by BLOODPRG after an XDB returns.
    pub(super) fn publish_alien_overlay_bridge_restoration(&mut self) {
        self.bridge_screen.palette_dirty = true;
        self.alien_overlay_mouse_idle_reset_pending = true;
    }

    /// Consume the native full mouse-idle counter reset at its timer owner.
    pub(super) fn take_alien_overlay_mouse_idle_reset_request(&mut self) -> bool {
        std::mem::take(&mut self.alien_overlay_mouse_idle_reset_pending)
    }

    /// Advance the recovered top-level ship presentation state machine.
    pub fn update_runtime_ship_presentation(
        &mut self,
        scene_link: GameSceneLink,
        state: &mut GameLifecycleState,
        platform: &mut RuntimePlatformHost<'window>,
    ) -> Result<ShipPresentationOutcome> {
        run_runtime_ship_presentation(self, scene_link, state, platform)
    }

    /// Advance and draw the recovered ship-HUD target selector.
    pub fn update_ship_target_selection(
        &mut self,
        state: &mut ShipTargetSelectionState<ScriptObjectId>,
        presentable_targets: &[ScriptObjectId],
    ) -> Result<RuntimeShipTargetSelection> {
        let pointer = self.input.pointer_sample();
        let current_hand_animation = self.manu3_hand.current_animation;
        let mut selector = self
            .ship_target_selector
            .take()
            .context("ship target selector update is reentrant")?;
        let outcome = selector.update(
            &mut self.runtime,
            ChoiceListPointer {
                position: pointer.position,
                primary_pressed: pointer
                    .buttons
                    .contains(crate::native::bloodprg::PointerButton::Primary),
            },
            current_hand_animation,
            state,
            presentable_targets,
        );
        let hand_requests = selector.take_hand_requests();
        self.ship_target_selector = Some(selector);
        self.apply_choice_list_hand_requests(hand_requests);
        let outcome = outcome?;
        if outcome.selection_sound_requested {
            self.play_loaded_sound_bank_clip(CHOICE_LIST_SELECTION_SOUND_CLIP)?;
        }
        Ok(outcome)
    }

    /// Return the last values written to the shared native list-layout globals.
    pub(super) const fn choice_list_style(&self) -> RuntimeChoiceListStyle {
        self.choice_list_style
    }

    /// Publish the shared width-mode write performed by save/load initialization.
    pub(super) fn set_choice_list_preserve_individual_widths(&mut self, preserve: bool) {
        self.choice_list_style.preserve_individual_widths = preserve;
    }

    /// Publish the shared list-layout values written by bridge-console activation.
    pub(super) fn activate_bridge_console_list_style(&mut self) {
        self.choice_list_style = RuntimeChoiceListStyle::BRIDGE_CONSOLE;
    }

    /// Publish the values written by first-time ship HUD initialization.
    pub(super) fn activate_ship_target_list_style(&mut self) {
        self.choice_list_style = RuntimeChoiceListStyle::SHIP_TARGET;
    }

    /// Publish the values written when a dialogue word choice first opens.
    pub(super) fn activate_presentation_word_choice_style(&mut self) {
        self.choice_list_style = RuntimeChoiceListStyle::PRESENTATION_WORD_CHOICE;
    }

    /// Resolve the active profile inputs consumed by first-time ship-HUD setup.
    pub fn ship_hud_initialization_context(
        &self,
    ) -> Result<ShipHudInitializationContext<ScriptObjectId>> {
        let profile = self
            .runtime
            .current_profile()
            .context("ship HUD initialization requires a loaded BloodScript profile")?;
        let arche = profile
            .builtins()
            .archetype
            .context("loaded BloodScript profile has no Arche object")?;
        let (arche_link, linked_record_is_direct_target) =
            ship_hud_arche_link(profile.state(), arche)?;
        Ok(ShipHudInitializationContext {
            arche,
            arche_link,
            linked_record_is_direct_target,
        })
    }

    /// Rebuild the active object census sharing Arche's navigation position.
    pub fn ship_objects_at_arche_position(&self) -> Result<Vec<ScriptObjectId>> {
        let profile = self
            .runtime
            .current_profile()
            .context("ship HUD VM processing requires a loaded BloodScript profile")?;
        let arche = profile
            .builtins()
            .archetype
            .context("loaded BloodScript profile has no Arche object")?;
        objects_at_arche_position(profile.state(), arche)
            .map_err(|error| anyhow::anyhow!("rebuilding ship HUD VM state: {error:?}"))
    }

    /// Build the target-first presentable list below one typed navigation root.
    pub fn presentable_ship_targets(&self, root: ScriptObjectId) -> Result<Vec<ScriptObjectId>> {
        let profile = self
            .runtime
            .current_profile()
            .context("ship target traversal requires a loaded BloodScript profile")?;
        let arche = profile
            .builtins()
            .archetype
            .context("loaded BloodScript profile has no Arche object")?;
        presentable_navigation_objects(profile.state(), root, arche)
            .map_err(|error| anyhow::anyhow!("building presentable ship targets: {error:?}"))
    }

    /// Apply one selected target's DESCRIPT record and report a changed music source.
    pub fn apply_ship_target_description(&mut self, target: ScriptObjectId) -> Result<bool> {
        let application = self
            .scripts
            .apply_object_description(target)?
            .with_context(|| format!("ship target {target:?} has no DESCRIPT record"))?;
        self.synchronize_script_presentations()?;
        Ok(matches!(
            application.music_selection(),
            Some(DescriptMusicSelectionOutcome::Changed)
        ))
    }

    /// Publish the selected target as the complete deferred C1 navigation action.
    pub fn defer_ship_navigation_target(&mut self, target: ScriptObjectId) {
        self.scripts.defer_navigation_target(target);
    }

    /// Write the selected ship-HUD target to the native `orxx` C1 action slot.
    pub fn queue_ship_hud_navigation_target(&mut self, target: ScriptObjectId) -> Result<()> {
        self.scripts.action_state_mut().current_ship_target = Some(target);
        self.scripts
            .queue_ship_hud_navigation_target(&mut self.runtime, target)
    }

    /// Publish the actionable C3 record emitted by a bridge-console command.
    pub fn defer_ship_presentation_queue(&mut self, target: ScriptObjectId) {
        self.scripts.defer_presentation_queue(target);
    }

    /// Publish the complete non-actionable C4 record emitted by navigation presentation.
    pub fn defer_ship_actor_presentation(&mut self, target: ScriptObjectId) {
        self.scripts.defer_actor_presentation(target);
    }

    /// Publish the complete deferred C6 action emitted by black-hole presentation.
    pub(super) fn defer_ship_travel_target(&mut self, target: ScriptObjectId) {
        self.scripts.defer_travel_target(target);
    }

    /// Return the C3 owner waiting for radio actor 4 to complete its line.
    pub(super) const fn pending_ship_presentation_owner(&self) -> Option<ScriptObjectId> {
        self.scripts.action_state().pending_presentation_owner
    }

    /// Clear the C3 owner after radio actor 4 promotes it to a C4 record.
    pub(super) fn clear_pending_ship_presentation_owner(&mut self) {
        self.scripts.action_state_mut().pending_presentation_owner = None;
    }

    /// Synchronize the exact slot-4 active-only gate consumed by C6 travel.
    pub(super) fn set_ship_travel_actor_ready(&mut self, ready: bool) {
        self.scripts.action_state_mut().travel_actor_busy = ready;
    }

    /// Consume C6's write to the flag byte aliased by bridge actor slot 4.
    pub(super) fn take_ship_travel_actor_clear_requested(&mut self) -> bool {
        std::mem::take(&mut self.scripts.action_state_mut().travel_actor_clear_requested)
    }

    /// Reset C6 travel to its first phase before publishing a new record.
    pub(super) fn reset_ship_travel_phase(&mut self) {
        let action = self.scripts.action_state_mut();
        action.travel_phase = Default::default();
        action.travel_actor_clear_requested = false;
    }

    /// Return whether the camera page currently replaces the ordinary bridge.
    pub(super) const fn bridge_camera_view_active(&self) -> bool {
        self.scripts.action_state().camera_view_active
    }

    /// Publish camera-page ownership after actor 5 toggles it.
    pub(super) fn set_bridge_camera_view_active(&mut self, active: bool) {
        self.scripts.action_state_mut().camera_view_active = active;
    }

    /// Resolve Arche's exact current navigation link and decoded object kind.
    pub(super) fn current_arche_navigation_target(
        &self,
    ) -> Result<(ScriptObjectId, ScriptObjectKind)> {
        let profile = self
            .runtime
            .current_profile()
            .context("bridge actors require a loaded BloodScript profile")?;
        let arche = profile
            .builtins()
            .archetype
            .context("loaded BloodScript profile has no Arche object")?;
        let (target, _) = ship_hud_arche_link(profile.state(), arche)?;
        let kind = profile
            .state()
            .object(target)
            .with_context(|| format!("Arche navigation target {target:?} is absent"))?
            .kind;
        Ok((target, kind))
    }

    /// Return the current typed ship target selected by script or HUD state.
    pub fn current_ship_navigation_target(&self) -> Result<ScriptObjectId> {
        if let Some(target) = self.scripts.action_state().current_ship_target.or_else(|| {
            self.ship_hud
                .as_ref()
                .and_then(RuntimeShipHud::coordinator)
                .map(|state| state.current_target)
        }) {
            return Ok(target);
        }
        self.current_arche_navigation_target()
            .map(|(target, _kind)| target)
    }

    /// Return the scene row currently committed by the C1 action dispatcher.
    pub fn ship_navigation_scene_vertical_offset(&self) -> u16 {
        self.scripts.action_state().scene_vertical_offset
    }

    /// Publish the navigation scene row back to the shared C1 action state.
    pub fn set_ship_navigation_scene_vertical_offset(&mut self, vertical_offset: u16) {
        self.scripts.action_state_mut().scene_vertical_offset = vertical_offset;
    }

    /// Return whether the presentation dispatcher retains a reusable scene image.
    pub fn ship_navigation_scene_image_cached(&self) -> Result<bool> {
        Ok(self
            .presentation_screen
            .as_ref()
            .context("presentation screen is already being updated")?
            .scene_state()
            .loaded_scene_image
            .is_some())
    }

    /// Return the dialogue chooser phase shared with navigation teardown.
    pub fn presentation_word_choice_phase(
        &self,
    ) -> Result<crate::native::bloodprg::PresentationWordChoicePhase> {
        Ok(self
            .presentation_word_choice
            .as_ref()
            .context("presentation word choice is already being updated")?
            .state()
            .phase)
    }

    /// Return whether the concrete ship HUD has completed its one-time setup.
    pub fn ship_hud_initialized(&self) -> Result<bool> {
        Ok(self
            .ship_hud
            .as_ref()
            .context("ship HUD is already being updated")?
            .coordinator()
            .is_some_and(|state| state.initialized))
    }

    /// Decode cached DESCRIPT background slot one into the retained navigation frame.
    pub fn stage_ship_navigation_background(&mut self) -> Result<PbmDecodeResult> {
        let slot = DescriptBackgroundSlot::decode(NAVIGATION_BACKGROUND_SLOT)
            .expect("navigation background slot is in the authored range");
        let encoded = self
            .scripts
            .backend()
            .backgrounds()
            .get(slot)
            .context("navigation DESCRIPT background slot one is not loaded")?
            .encoded_image()
            .to_vec();
        let result = self.runtime.stage_navigation_background(&encoded)?;
        let palette = *self.runtime.live_palette();
        self.presentation_screen
            .as_mut()
            .context("presentation screen is already being updated")?
            .stage_navigation_palette(&palette);
        Ok(result)
    }

    pub(super) fn stage_presentation_scene_palette(
        &mut self,
        palette: &IndexedGamePalette,
    ) -> Result<()> {
        self.presentation_screen
            .as_mut()
            .context("presentation screen is already being updated")?
            .stage_navigation_palette(palette);
        Ok(())
    }

    /// Clear rows 35 through 164 before decoding the navigation background.
    pub fn clear_ship_navigation_band(&mut self) -> Result<()> {
        self.presentation_screen
            .as_mut()
            .context("presentation screen is already being updated")?
            .invalidate_scene_image();
        self.runtime.clear_navigation_background_band();
        Ok(())
    }

    /// Return whether presentation completion requested an interactive alien overlay.
    pub fn alien_overlay_trigger_pending(&self) -> Result<bool> {
        Ok(self
            .presentation_screen
            .as_ref()
            .context("presentation screen is already being updated")?
            .scene_state()
            .temporary_sound_trigger)
    }

    pub(super) fn alien_overlay_flags(&self) -> Result<(bool, bool)> {
        Ok(self
            .presentation_screen
            .as_ref()
            .context("presentation screen is already being updated")?
            .alien_overlay_flags())
    }

    pub(super) fn set_alien_overlay_flags(&mut self, armed: bool, pending: bool) -> Result<()> {
        self.presentation_screen
            .as_mut()
            .context("presentation screen is already being updated")?
            .set_alien_overlay_flags(armed, pending);
        Ok(())
    }

    pub(super) fn read_alien_timing_scale(&self) -> Result<u16> {
        let profile = self
            .runtime
            .current_profile()
            .context("alien overlay requires a loaded BloodScript profile")?;
        let source_offset = profile
            .builtins()
            .video_state_offset
            .context("loaded BloodScript profile has no vbio timing word")?;
        let field = profile
            .state()
            .resolve_word_source_offset(source_offset)
            .context("vbio timing offset does not resolve to an aligned VAR word")?;
        profile
            .state()
            .word(field)
            .context("vbio timing word is outside the loaded VAR state")
    }

    pub(super) fn write_alien_timing_scale(&mut self, timing_scale: u16) -> Result<()> {
        let profile = self
            .runtime
            .current_profile_mut()
            .context("alien overlay lost its loaded BloodScript profile")?;
        let source_offset = profile
            .builtins()
            .video_state_offset
            .context("loaded BloodScript profile has no vbio timing word")?;
        let field = profile
            .state()
            .resolve_word_source_offset(source_offset)
            .context("vbio timing offset does not resolve to an aligned VAR word")?;
        if !profile.state_mut().set_word(field, timing_scale) {
            bail!("failed to restore the vbio timing word");
        }
        Ok(())
    }

    pub(super) fn begin_alien_overlay(&mut self, asset: &AlienAsset) -> Result<()> {
        self.ensure_main_viewport()?;
        self.presentation.begin_alien_overlay(asset)
    }

    pub(super) fn present_alien_overlay_frame(&mut self, frame: &AlienSceneFrame) -> Result<()> {
        self.ensure_main_viewport()?;
        self.presentation.present_alien_overlay_frame(frame)
    }

    pub(super) fn finish_alien_overlay(&mut self) -> bool {
        self.presentation.finish_alien_overlay()
    }

    pub(super) fn clear_alien_overlay_transition_frame(&mut self) -> Result<()> {
        fill_display_band(
            self.runtime.front_buffer_mut().pixels_mut(),
            usize::MIN,
            LOGICAL_FRAMEBUFFER_HEIGHT,
            u8::MIN,
        )
        .context("clearing the display after an alien overlay")
    }

    pub(super) fn restore_sequence_back_buffer(&mut self) -> Result<()> {
        self.runtime.restore_sequence_back_buffer().map(|_| ())
    }

    pub(super) fn reload_current_scene_image(&mut self) -> Result<()> {
        let slot = self
            .presentation_screen
            .as_ref()
            .context("presentation screen is already being updated")?
            .loaded_scene_image()
            .context("alien overlay returned without a retained scene image")?;
        let encoded = self
            .scripts
            .backend()
            .backgrounds()
            .get(slot)
            .with_context(|| format!("DESCRIPT background slot {slot:?} is not loaded"))?
            .encoded_image()
            .to_vec();
        self.runtime.reload_scene_back_buffer(&encoded).map(|_| ())
    }

    /// Clear only the 192 scene colors, preserving the bridge console palette tail.
    pub fn clear_navigation_scene_palette(&mut self) {
        clear_scene_palette_entries(self.runtime.live_palette_mut());
    }

    /// Decode CHART.FD into the retained background without presenting it early.
    pub fn initialize_navigation_back_buffer(&mut self) -> Result<PbmDecodeResult> {
        self.runtime.initialize_back_buffer()
    }

    /// Capture the HUD palette and restore the modern bridge camera origin.
    pub fn snapshot_navigation_hud_palette_and_camera(&mut self) -> Result<()> {
        self.runtime.snapshot_ship_hud_palette();
        self.bridge_scene
            .as_mut()
            .context("navigation reset requires an initialized bridge scene")?
            .reset_camera();
        Ok(())
    }

    /// Flatten the first 192 live VGA colors copied by bridge actor 2.
    pub(super) fn bridge_actor_live_palette(
        &self,
    ) -> [u8; BRIDGE_ACTOR_PALETTE_COLOR_COUNT * RGB_COMPONENT_COUNT] {
        let mut bytes = [u8::MIN; BRIDGE_ACTOR_PALETTE_COLOR_COUNT * RGB_COMPONENT_COUNT];
        for (destination, color) in bytes.chunks_exact_mut(RGB_COMPONENT_COUNT).zip(
            self.runtime
                .live_palette()
                .iter()
                .take(BRIDGE_ACTOR_PALETTE_COLOR_COUNT),
        ) {
            destination.copy_from_slice(color);
        }
        bytes
    }

    /// Commit actor 2's 192-color snapshot to the retained bridge palette.
    pub(super) fn apply_bridge_actor_palette(
        &mut self,
        bytes: &[u8; BRIDGE_ACTOR_PALETTE_COLOR_COUNT * RGB_COMPONENT_COUNT],
    ) {
        for (color, source) in self
            .bridge_palette
            .iter_mut()
            .take(BRIDGE_ACTOR_PALETTE_COLOR_COUNT)
            .zip(bytes.chunks_exact(RGB_COMPONENT_COUNT))
        {
            color.copy_from_slice(source);
        }
    }

    /// Configure the original bridge-panorama-to-black full-palette transition.
    pub fn configure_navigation_bridge_palette_transition(&mut self) -> Result<()> {
        self.palette_transition
            .configure(RuntimePaletteTransitionConfig {
                source: self.bridge_palette,
                target: [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
                initial_percent: u16::MIN,
                increment: NAVIGATION_PALETTE_TRANSITION_INCREMENT,
                colors: u8::MIN..=u8::MAX,
            })
            .context("configuring the navigation bridge palette transition")
    }

    /// Reset the dialogue word-choice owner at a recovered shared-state boundary.
    pub fn reset_presentation_word_choice(&mut self) -> Result<()> {
        self.presentation_word_choice
            .as_mut()
            .context("presentation word choice is already being updated")?
            .reset();
        Ok(())
    }

    /// Export countdowns written by frame systems into the canonical game timer.
    pub fn export_game_timer_state(
        &self,
        timer: &mut crate::native::bloodprg::GameTimerState,
    ) -> Result<()> {
        self.subtitle_reveal
            .as_ref()
            .context("subtitle reveal is already being updated")?
            .export_timer_state(timer);
        Ok(())
    }

    /// Publish the canonical timer word and countdowns after one frame of ticks.
    pub fn import_game_timer_state(
        &mut self,
        timer: &crate::native::bloodprg::GameTimerState,
    ) -> Result<()> {
        self.game_timer_tick = timer.tick;
        self.subtitle_reveal
            .as_mut()
            .context("subtitle reveal is already being updated")?
            .import_timer_state(timer);
        Ok(())
    }

    /// Read the wrapping low timer word formerly shared at native address `0x0B29`.
    pub const fn game_timer_tick(&self) -> u16 {
        self.game_timer_tick
    }

    /// Mark the script-side ship interface inactive after full navigation teardown.
    pub fn finish_ship_navigation_reset(&mut self) {
        let action = self.scripts.action_state_mut();
        action.ship_navigation_mode = ScriptShipNavigationMode::Inactive;
    }

    /// Clear the retained ship-HUD setup surface.
    pub fn clear_ship_hud_back_buffer(&mut self) {
        self.runtime.clear_back_buffer();
    }

    /// Apply the bridge seek target and panorama frame written by HUD setup.
    pub fn initialize_ship_hud_bridge_view(
        &mut self,
        seek_target_arc: u16,
        view_frame: u16,
    ) -> Result<()> {
        self.bridge_scene
            .as_mut()
            .context("ship HUD setup requires an initialized bridge scene")?
            .initialize_hud_view(seek_target_arc, view_frame);
        Ok(())
    }

    /// Advance bridge steering while the ship target list owns interaction.
    pub fn render_ship_hud_bridge_frame(&mut self) -> Result<()> {
        let pointer = self.input.pointer_sample();
        self.render_bridge_frame(BridgeSceneInput {
            horizontal_delta: NO_BRIDGE_HORIZONTAL_MOTION,
            pointer_buttons: pointer.buttons.bits(),
            interaction: BridgeSteeringInteraction::MenuEngaged,
        })?;
        Ok(())
    }

    /// Publish the full clip and commit one half-open ship entity range.
    pub fn commit_ship_entities(
        &mut self,
        entities: Range<u16>,
    ) -> Result<BridgeSpriteCommitOutcome> {
        self.runtime.commit_ship_entity_geometry(entities)
    }

    /// Restore current ship dirty rectangles from the retained secondary surface.
    pub fn copy_ship_dirty_regions(&mut self) -> Result<DirtyRegionCopyOutcome> {
        self.runtime.copy_ship_dirty_regions()
    }

    /// Configure the original black-to-HUD-tail palette transition.
    pub fn configure_ship_hud_palette_transition(&mut self) -> Result<()> {
        let source = *self.runtime.live_palette();
        let mut target = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        target[SCENE_PALETTE_CLEAR_COLOR_COUNT] =
            self.runtime.ship_hud().palette_snapshot[usize::MIN];
        let last = u8::try_from(SCENE_PALETTE_CLEAR_COLOR_COUNT)
            .context("ship HUD palette boundary exceeds an indexed palette")?;
        self.palette_transition
            .configure(RuntimePaletteTransitionConfig {
                source,
                target,
                initial_percent: u16::MIN,
                increment: SHIP_HUD_PALETTE_TRANSITION_INCREMENT,
                colors: u8::MIN..=last,
            })
            .context("configuring the ship HUD palette transition")
    }

    /// Synchronize palette progress changed inside the recovered HUD coordinator.
    pub fn synchronize_ship_hud_palette_progress(&mut self, percent: u16, increment: u16) {
        self.palette_transition.set_progress_percent(percent);
        self.palette_transition.set_increment(increment);
        self.ship_presentation.transition_percent = percent;
    }

    /// Return the six BloodScript sequence records in presentation-choice order.
    pub fn presentation_sequence_records(
        &self,
    ) -> Result<[Option<Box<[u8]>>; PresentationChoiceNumber::COUNT]> {
        let profile = self
            .runtime
            .current_profile()
            .context("presentation choices require a loaded BloodScript profile")?;
        Ok(profile
            .sequence_slots()
            .ordered_names()
            .map(|name| name.map(Box::from)))
    }

    /// Lay out, interact with, and draw one recovered bridge choice list.
    pub fn update_choice_list(
        &mut self,
        labels: &[&[u8]],
        config: ChoiceListConfig<'_>,
        state: &mut ChoiceListState,
        primary_pointer_pressed: bool,
    ) -> Result<ChoiceListFrame> {
        let pointer = self.input.pointer_sample().position;
        let current_hand_animation = self.manu3_hand.current_animation;
        let (frame, hand_requests) = prepare_choice_list_frame(
            &mut self.runtime,
            labels,
            config,
            state,
            ChoiceListPointer {
                position: pointer,
                primary_pressed: primary_pointer_pressed,
            },
            current_hand_animation,
        )?;
        self.apply_choice_list_hand_requests(hand_requests);
        if frame.selected_item.is_some() || frame.cancelled {
            self.play_loaded_sound_bank_clip(CHOICE_LIST_SELECTION_SOUND_CLIP)?;
        }
        let fonts = self.runtime.data().font_resources().clone();
        draw_choice_list_rows(
            &mut self.runtime,
            &fonts,
            labels,
            config.cancel_label,
            &frame,
        )?;
        Ok(frame)
    }

    /// Apply ordered selector writes emitted by the shared list widget.
    pub(super) fn apply_choice_list_hand_requests(
        &mut self,
        requests: impl IntoIterator<Item = ChoiceListHandRequest>,
    ) {
        for request in requests {
            let selector = match request.animation {
                ChoiceListHandAnimation::Idle => Manu3AnimationSelector::BridgeActive,
                ChoiceListHandAnimation::Hover => Manu3AnimationSelector::ChoiceListHover,
                ChoiceListHandAnimation::Active => Manu3AnimationSelector::ChoiceListActive,
            };
            if request.restart_current {
                self.restart_manu3_animation(selector);
            } else {
                self.request_manu3_animation(selector);
            }
        }
    }

    /// Advance the BloodScript dialogue choice gate and publish its concept.
    pub fn update_lifecycle_word_choice(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> Result<PresentationWordChoiceOutcome> {
        let mut word_choice = self
            .presentation_word_choice
            .take()
            .context("presentation word-choice update is reentrant")?;
        let outcome = word_choice.update(self, state);
        self.presentation_word_choice = Some(word_choice);
        outcome
    }

    /// Advance and draw the executable-authored progressive subtitle surface.
    pub fn update_lifecycle_subtitles(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> Result<crate::native::bloodprg::SubtitleRevealOutcome> {
        self.scripts.prepare_lifecycle_text_frame(state);
        let mut subtitle = self
            .subtitle_reveal
            .take()
            .context("subtitle reveal update is reentrant")?;
        subtitle.import_lifecycle_state(&state.presentation, self.ship_presentation.hud_active());
        let outcome = subtitle.update(&mut self.runtime, self.scripts.text_presentation_mut());
        self.subtitle_reveal = Some(subtitle);
        let outcome = outcome?;
        self.scripts.finish_lifecycle_text_frame(state)?;
        Ok(outcome)
    }

    /// Advance the shared palette fade and reproduce its input-latch upload gate.
    pub fn update_lifecycle_palette_transition(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> Result<RuntimePaletteTransitionOutcome> {
        let outcome = self
            .palette_transition
            .update(self.runtime.live_palette_mut(), state)
            .context("advancing the recovered palette transition")?;
        self.ship_presentation.transition_percent = self.palette_transition.state().percent;
        Ok(outcome)
    }

    /// Mutably borrow the transition owner used by scene and HUD coordinators.
    pub fn palette_transition_mut(&mut self) -> &mut RuntimePaletteTransition {
        &mut self.palette_transition
    }

    /// Borrow the transition owner used by scene and HUD coordinators.
    pub const fn palette_transition(&self) -> &RuntimePaletteTransition {
        &self.palette_transition
    }

    /// Enable or disable the bridge's recovered six-choice presentation panel.
    pub fn set_presentation_screen_active(&mut self, active: bool) -> Result<()> {
        self.presentation_screen
            .as_mut()
            .context("presentation screen is already being updated")?
            .state_mut()
            .set_active(active);
        Ok(())
    }

    /// Apply the panel actor's write to the presentation renderer's shared phase.
    pub(super) fn begin_presentation_panel_close_if_open(&mut self) -> Result<bool> {
        Ok(self
            .presentation_screen
            .as_mut()
            .context("presentation screen is already being updated")?
            .state_mut()
            .begin_actor_close_if_open())
    }

    /// Synchronize the shared bridge-actor redraw bit with the panel owner.
    pub(super) fn set_bridge_actor_redraw_requested(&mut self, requested: bool) -> Result<()> {
        self.presentation_screen
            .as_mut()
            .context("presentation screen is already being updated")?
            .state_mut()
            .set_redraw_requested(requested);
        Ok(())
    }

    /// Finish the C2 scene queue through the state owners used by modern frames.
    pub(super) fn finish_bridge_actor_scene_presentation(
        &mut self,
        lifecycle: &mut GameLifecycleState,
    ) {
        self.finish_presentation_sequence();
        if self.ship_presentation.flags & SHIP_PRESENTATION_HUD_FLAG != u16::MIN {
            self.ship_presentation.bridge_redraw_pending = BRIDGE_REDRAW_REQUESTED;
        }
        lifecycle.presentation.active_line = None;
        lifecycle.presentation.c2_presentation_gate = false;
        lifecycle
            .presentation
            .request_flags
            .clear_secondary_request();
        self.scripts.finish_actor_scene_presentation();
    }

    /// Rebuild the retained bridge surface and arm the startup reverse panel.
    pub fn initialize_bridge_screen(
        &mut self,
        startup_presentation_mode: bool,
        ship_active: bool,
    ) -> Result<()> {
        let transition_pending = self.runtime.bridge_frame_state().transition_pending();
        self.initialize_bridge_screen_with_transition(
            startup_presentation_mode,
            ship_active,
            transition_pending,
        )
    }

    /// Rebuild bridge flags against the coordinator's transferred transition state.
    pub(super) fn initialize_bridge_screen_with_transition(
        &mut self,
        startup_presentation_mode: bool,
        ship_active: bool,
        transition_pending: bool,
    ) -> Result<()> {
        let panorama_frame = self.bridge_view_frame()? as u16;
        let mut screen_state = std::mem::take(&mut self.bridge_screen);
        screen_state.screen_rebuild_pending = true;
        screen_state.reverse_presentation_active = startup_presentation_mode;
        let mut panorama_palette = self.bridge_palette;
        let mut live_palette = *self.runtime.live_palette();
        let mut actor_slots = std::mem::take(&mut self.nav_actor_slots);
        let outcome = {
            let mut backend = RuntimeBridgeScreenBackend {
                services: self,
                ship_active,
                palette_refresh_in_progress: false,
            };
            initialize_bridge_screen(
                transition_pending,
                panorama_frame,
                &mut screen_state,
                &mut panorama_palette,
                &mut live_palette,
                &mut actor_slots,
                &mut backend,
            )
        };
        self.ship_presentation.depth_offset = screen_state.ship_depth_offset;
        self.bridge_screen = screen_state;
        self.bridge_palette = panorama_palette;
        *self.runtime.live_palette_mut() = live_palette;
        self.nav_actor_slots = actor_slots;
        outcome.context("running the recovered bridge screen initializer")?;
        self.bridge_actors
            .as_mut()
            .context("bridge actor state is already being updated")?
            .reset_bridge_screen_latches();
        if startup_presentation_mode {
            let screen = self
                .presentation_screen
                .as_mut()
                .context("presentation screen is already being updated")?
                .state_mut();
            // Native screen_flags_init preserves the actor slots in reverse mode;
            // handler 3 activates the panel only after its authored line completes.
            screen.arm_startup_reverse();
        }
        Ok(())
    }

    /// Borrow the exact bridge-screen state retained between rebuilds.
    pub const fn bridge_screen_state(&self) -> &BridgeScreenInitializationState {
        &self.bridge_screen
    }

    /// Borrow the bridge presentation state published to its frame coordinator.
    pub fn presentation_screen_state(&self) -> Result<&PresentationScreenState> {
        Ok(self
            .presentation_screen
            .as_ref()
            .context("presentation screen is already being updated")?
            .state())
    }

    /// Apply Escape through the recovered presentation-resource cancellation path.
    pub fn cancel_lifecycle_presentation(
        &mut self,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<InputCancellationOutcome> {
        let cursor = self.presentation_player.cancellation_cursor();
        let mut cancellation = InputCancellationState {
            presentation_active: lifecycle.presentation.c2_presentation_gate
                && self.presentation_player.source_open_or_draining()
                && cursor.is_some(),
            dialogue_ready: self.ship_presentation.dialogue_phase_ready & 1 != u8::MIN,
            ship_active: self.ship_presentation.flags & INPUT_CANCEL_SHIP_BLOCK != u16::MIN,
            active_line: usize::from(
                lifecycle
                    .presentation
                    .active_line
                    .unwrap_or(self.ship_presentation.active_line),
            ),
            resources: cursor.unwrap_or_default(),
            scene_palette: *self.runtime.live_palette(),
            palette_dirty: false,
        };
        let outcome = self
            .input
            .cancel_presentation(&mut cancellation, &mut self.presentation_player);
        lifecycle.pause_hud_active = self.input.dispatch_state().paused;

        if outcome == InputCancellationOutcome::CancelledPresentation {
            self.presentation_player
                .apply_cancellation_cursor(cancellation.resources)?;
            self.ship_presentation.dialogue_phase_ready = u8::from(cancellation.dialogue_ready);
            *self.runtime.live_palette_mut() = cancellation.scene_palette;
            self.presentation_screen
                .as_mut()
                .context("presentation screen is already being updated")?
                .synchronize_scene_palette(cancellation.scene_palette);
            self.palette_transition.request_visual_color_update();
        }
        Ok(outcome)
    }

    /// Advance the bridge presentation panel from live script and pointer state.
    pub fn update_presentation_screen(
        &mut self,
        queued_scene_link: &GameSceneLink,
        primary_pointer_pressed: bool,
    ) -> Result<PresentationScreenOutcome> {
        let active_record_related = self
            .runtime
            .current_profile()
            .and_then(|profile| profile.active_actor_presentation_related());
        let scruter_jo_record = self
            .runtime
            .current_profile()
            .and_then(|profile| profile.builtins().scruter_jo);
        let mut screen = self
            .presentation_screen
            .take()
            .context("presentation screen update is reentrant")?;
        let active_line_before = screen.scene_state().presentation.active_line;
        let finale_requested = self.scripts.sequence_presentation().finale_requested;
        screen
            .state_mut()
            .set_primary_pressed(primary_pointer_pressed);
        let outcome = screen.update(
            self,
            queued_scene_link,
            active_record_related,
            scruter_jo_record,
        );
        latch_script_finale_completion(
            &mut self.script_finale_shutdown_pending,
            finale_requested,
            active_line_before,
            screen.scene_state().presentation.active_line,
        );
        self.presentation_screen = Some(screen);
        if matches!(&outcome, Ok(PresentationScreenOutcome::Initialized)) {
            self.set_previous_manu3_animation(Manu3AnimationSelector::PresentationPanel);
            self.presentation_hover
                .set_previous_actor_state(BridgeActorPresentationState::PresentationPanel);
        }
        outcome
    }

    /// Transfer one-shot panel outputs into the owning lifecycle state.
    pub fn consume_presentation_screen_outputs(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> Result<()> {
        let screen = self
            .presentation_screen
            .as_mut()
            .context("presentation screen is already being updated")?
            .state_mut();
        let screen_rebuild_pending = screen.take_screen_rebuild_pending();
        let completion_audio_pending = screen.take_completion_audio_pending();
        let choice_animation_requested = screen.take_choice_change_animation_requested();
        let startup_mode_completed = screen.take_reverse_resource_variant_restored();

        publish_presentation_screen_modal_ui(state, screen.redraw_requested());
        state.navigation_rebuild_pending |= screen_rebuild_pending;
        state.presentation.completion_audio_pending |= completion_audio_pending;
        if choice_animation_requested {
            self.manu3_hand
                .restart(Manu3AnimationSelector::PresentationChoice);
        }
        if startup_mode_completed {
            state.presentation_mode = false;
        }
        Ok(())
    }

    /// Apply one selected DESCRIPT record through the live BloodScript text state.
    pub fn apply_presentation_description(
        &mut self,
        name: &[u8],
    ) -> Result<Option<DescriptRecordApplication>> {
        let application = self.scripts.apply_presentation_description(name)?;
        self.synchronize_script_presentations()?;
        Ok(application)
    }

    pub(super) fn apply_scene_transition_description(
        &mut self,
        object: ScriptObjectId,
        presentation_interface_active: bool,
        text: &mut TextPresentationState,
    ) -> Result<()> {
        self.scripts
            .apply_object_description_to_text(object, presentation_interface_active, text)?
            .with_context(|| {
                format!("scene-transition object {object:?} has no DESCRIPT record")
            })?;
        self.synchronize_script_presentations()
    }

    /// Queue one recovered MANU3 animation selector for the frame-tail dispatcher.
    pub fn request_manu3_animation(&mut self, selector: Manu3AnimationSelector) {
        self.manu3_hand.request(selector);
    }

    /// Restart one recovered MANU3 animation by clearing the aliased current selector first.
    pub fn restart_manu3_animation(&mut self, selector: Manu3AnimationSelector) {
        self.manu3_hand.restart(selector);
    }

    /// Replace the selector restored when bridge hover or ship navigation ends.
    pub fn set_previous_manu3_animation(&mut self, selector: Manu3AnimationSelector) {
        self.manu3_previous_animation = selector;
    }

    /// Return the selector corresponding to native `presentation_mode_previous_state`.
    pub const fn previous_manu3_animation(&self) -> Manu3AnimationSelector {
        self.manu3_previous_animation
    }

    /// Restore the previous bridge selector through the shared request word.
    pub fn restore_previous_manu3_animation(&mut self) {
        self.manu3_hand.request(self.manu3_previous_animation);
    }

    /// Advance the recovered hand dispatcher and render its requested 3D frame.
    pub fn update_manu3_hand(&mut self, context: Manu3HandFrameContext) -> Result<bool> {
        let Some(request) = update_manu3_hand_frame(&mut self.manu3_hand, context) else {
            return Ok(false);
        };
        self.runtime
            .manu3_mut()
            .context("MANU3 hand update requires the decoded model")?
            .render_frame(request)
            .context("rendering recovered MANU3 hand frame")?;
        Ok(true)
    }

    /// Advance MANU3 from current lifecycle gates and the sampled logical pointer.
    pub fn update_lifecycle_manu3(&mut self, state: &GameLifecycleState) -> Result<bool> {
        let pointer = self.input.pointer_sample().position;
        self.update_manu3_hand(Manu3HandFrameContext {
            presentation_mode_active: state.presentation_mode,
            hud_refresh_active: state.pause_hud_active,
            ship_scene_dispatch_blocked: self.ship_presentation.scene_dispatch_blocked,
            presentation_request_pending: state
                .presentation
                .request_flags
                .secondary_request_pending(),
            cursor: CursorPosition {
                x: pointer[0],
                y: pointer[1],
            },
        })
    }

    /// Reproject the current MANU3 pose for a visual-only host refresh.
    ///
    /// This does not advance the recovered animation selector, phase, or tween
    /// records. The next simulation frame remains the only place where the C
    /// frame-tail MANU3 dispatcher changes game state.
    pub fn reproject_manu3_for_pointer(&mut self, pointer: [i16; 2]) -> Result<bool> {
        let Some(model) = self.runtime.manu3_mut() else {
            return Ok(false);
        };
        model
            .reproject_frame(CursorPosition {
                x: pointer[0],
                y: pointer[1],
            })
            .context("reprojecting the current MANU3 pose")?;
        Ok(true)
    }

    /// Borrow the canonical flat state shared by ship presentation and MANU3.
    pub const fn ship_presentation_state(&self) -> &ShipPresentationState {
        &self.ship_presentation
    }

    /// Mutably borrow ship presentation state for its exact frame coordinator.
    pub fn ship_presentation_state_mut(&mut self) -> &mut ShipPresentationState {
        &mut self.ship_presentation
    }

    /// Capture the current indexed display as the ship-depth source frame.
    pub fn capture_ship_depth_source(&mut self) {
        self.runtime.capture_ship_depth_source();
    }

    /// Advance the canonical recovered ship-depth state by one frame.
    pub fn advance_ship_depth(&mut self) -> ShipDepthTransitionOutcome {
        self.ship_presentation.advance_depth_transition()
    }

    /// Apply the current ship-depth bands to the flat indexed framebuffer.
    pub fn compose_ship_depth_bands(&mut self) -> Result<bool> {
        self.ship_presentation.transition_percent = self.palette_transition.state().percent;
        let increment = self.palette_transition.state().increment;
        let Some(layout) = self.ship_presentation.prepare_depth_band(increment) else {
            return Ok(false);
        };
        self.palette_transition
            .set_progress_percent(self.ship_presentation.transition_percent);
        self.runtime
            .compose_ship_depth_bands(layout)
            .context("composing recovered ship-depth bands")?;
        Ok(true)
    }

    /// Advance one fixed ship-view entity selected by the top-level coordinator.
    pub fn transition_ship_view_entity(&mut self, entity: ShipViewEntityId) -> Result<bool> {
        self.runtime.transition_ship_view_entity(entity)
    }

    /// Dispatch the ship's active authored line through the shared scene player.
    pub fn dispatch_ship_scene(
        &mut self,
        scene_link: GameSceneLink,
    ) -> Result<PresentationSceneDispatchOutcome> {
        let vertical_offset = self.ship_navigation_scene_vertical_offset();
        let active_record_related = self
            .runtime
            .current_profile()
            .and_then(|profile| profile.active_actor_presentation_related());
        let scruter_jo_record = self
            .runtime
            .current_profile()
            .and_then(|profile| profile.builtins().scruter_jo);
        let mut screen = self
            .presentation_screen
            .take()
            .context("ship scene dispatch is reentrant")?;
        screen.set_ship_scene_vertical_offset(vertical_offset);
        let mut ship = std::mem::take(&mut self.ship_presentation);
        let outcome = screen.dispatch_ship_scene(
            self,
            &mut ship,
            scene_link,
            active_record_related,
            scruter_jo_record,
        );
        if matches!(
            &outcome,
            Ok(PresentationSceneDispatchOutcome::PresentationFinished)
        ) && self.scripts.sequence_presentation().finale_requested
        {
            self.script_finale_shutdown_pending = true;
        }
        self.ship_presentation = ship;
        self.presentation_screen = Some(screen);
        outcome
    }

    /// Return the shared frame-readiness flag written by the latest scene dispatch.
    pub(super) fn presentation_scene_frame_presented(&self) -> Result<bool> {
        Ok(self
            .presentation_screen
            .as_ref()
            .context("presentation screen is already being updated")?
            .scene_state()
            .frame_presented)
    }

    /// Take a frame-readiness write emitted while updating the presentation panel.
    pub(super) fn take_presentation_scene_frame_output(&mut self) -> Result<Option<bool>> {
        Ok(self
            .presentation_screen
            .as_mut()
            .context("presentation screen is already being updated")?
            .take_scene_frame_presented_output())
    }

    pub(super) fn dispatch_scene_transition(
        &mut self,
        scene_link: GameSceneLink,
        transition: &mut SceneTransitionState,
        presentation: &mut ScriptPresentationScanState,
        lifecycle: &mut GameLifecycleState,
        active_record_related: ScriptObjectId,
        palette_transition_percent: &mut u16,
    ) -> Result<PresentationSceneDispatchOutcome> {
        let scruter_jo_record = self
            .runtime
            .current_profile()
            .and_then(|profile| profile.builtins().scruter_jo);
        let mut screen = self
            .presentation_screen
            .take()
            .context("scene-transition dispatch is reentrant")?;
        let outcome = screen.dispatch_scene_transition(
            self,
            RuntimeSceneTransitionDispatchContext {
                scene_link,
                transition,
                presentation,
                lifecycle,
                active_record_related,
                scruter_jo_record,
                palette_transition_percent,
            },
        );
        if matches!(
            &outcome,
            Ok(PresentationSceneDispatchOutcome::PresentationFinished)
        ) && self.scripts.sequence_presentation().finale_requested
        {
            self.script_finale_shutdown_pending = true;
        }
        self.presentation_screen = Some(screen);
        outcome
    }

    /// Clear the recovered full-screen travel redraw surface.
    pub fn clear_ship_travel_display(&mut self) {
        self.runtime.clear_ship_travel_display();
    }

    /// Reveal and draw one exact frame of the current BloodScript inline menu.
    pub fn reveal_inline_menu(
        &mut self,
        owner_matches: bool,
        word_delay: u16,
    ) -> Result<InlineMenuRevealOutcome> {
        let dictionary = self
            .runtime
            .current_profile()
            .context("inline menu rendering requires a loaded BloodScript profile")?
            .dictionary()
            .clone();
        let fonts = self.runtime.data().font_resources().clone();
        let mut metrics = RuntimeInlineMenuMetrics::new(&fonts);
        let outcome = reveal_inline_menu_step(
            self.scripts.text_presentation_mut(),
            &dictionary,
            owner_matches,
            word_delay,
            &mut metrics,
        )
        .context("advancing the recovered inline menu reveal")?;
        metrics.finish()?;

        if let InlineMenuRevealOutcome::Frame(frame) = &outcome {
            for placement in &frame.placements {
                let text = dictionary.word(placement.word).with_context(|| {
                    format!(
                        "inline menu word {} is absent from the loaded dictionary",
                        placement.word.index()
                    )
                })?;
                draw_planar_dialogue_text(
                    self.runtime.front_buffer_mut().pixels_mut(),
                    &fonts,
                    text,
                    FontPoint {
                        x: i32::from(placement.position[0]),
                        y: i32::from(placement.position[1]),
                    },
                    FULL_LOGICAL_FONT_BAND,
                    placement.color,
                )
                .context("drawing an inline menu word")?;
            }
        }
        Ok(outcome)
    }

    /// Reveal the live inline menu and publish its hold state to the lifecycle.
    pub fn reveal_lifecycle_inline_menu(
        &mut self,
        state: &mut GameLifecycleState,
        word_delay: u16,
    ) -> Result<InlineMenuRevealOutcome> {
        self.scripts.prepare_lifecycle_text_frame(state);
        let owner_matches = state.presentation.owner == Some(GamePresentationOwner::DeferredMenu);
        let outcome = self.reveal_inline_menu(owner_matches, word_delay)?;
        self.scripts.finish_lifecycle_text_frame(state)?;
        Ok(outcome)
    }

    /// Return the recovered text-speed value shared by menu and subtitle timing.
    pub fn dialogue_word_delay(&self) -> Result<u16> {
        Ok(self
            .subtitle_reveal
            .as_ref()
            .context("subtitle reveal is already being updated")?
            .state()
            .text_speed_step)
    }

    /// Apply a player-selected text-speed step to future subtitle and menu timing.
    pub fn set_dialogue_word_delay(&mut self, step: u16) -> Result<()> {
        self.subtitle_reveal
            .as_mut()
            .context("subtitle reveal is already being updated")?
            .set_text_speed_step(step);
        Ok(())
    }

    /// Consume the next value from the game's persistent recovered PRNG.
    pub fn next_random(&mut self, modulus: u16) -> u16 {
        self.random_draws.presentation_noise += 1;
        self.random.next(modulus)
    }

    /// Borrow the persistent MANU3 selector and presentation-delay state.
    pub const fn manu3_hand_state(&self) -> &Manu3HandFrameState {
        &self.manu3_hand
    }

    /// Mutably borrow the persistent MANU3 selector and presentation-delay state.
    pub fn manu3_hand_state_mut(&mut self) -> &mut Manu3HandFrameState {
        &mut self.manu3_hand
    }

    /// Publish the local civil clock sampled immediately before a BloodScript pass.
    pub fn set_script_clock(&mut self, clock: ScriptClock) {
        self.scripts.backend_mut().set_clock(clock);
    }

    /// Execute one complete translated COD/BAS/presentation frame.
    pub fn execute_script_frame(&mut self, enabled: bool) -> Result<ScriptFrameOutcome> {
        self.synchronize_script_action_runtime_state(u16::MIN)?;
        self.scripts
            .prepare_ship_presentation_state(&self.ship_presentation);
        self.scripts.import_random_state(self.random);
        let random_counter_before = self.random.counter;
        let outcome = self.scripts.execute_frame(&mut self.runtime, enabled);
        self.random = self.scripts.random_state();
        self.random_draws.script += random_draw_delta(random_counter_before, self.random.counter);
        let outcome = outcome?;
        self.scripts
            .finish_ship_presentation_state(&mut self.ship_presentation);
        Ok(outcome)
    }

    /// Execute one translated script frame and apply every ordered host command it emitted.
    pub fn execute_and_apply_script_frame(&mut self, enabled: bool) -> Result<ScriptFrameOutcome> {
        let outcome = self.execute_script_frame(enabled)?;
        self.publish_script_presentation_status_change()?;
        self.synchronize_script_action_effects(None);
        self.synchronize_script_presentations()?;
        self.process_script_commands()?;
        Ok(outcome)
    }

    /// Execute one translated script frame with main-loop state exchange and effects.
    pub fn execute_and_apply_lifecycle_script_frame(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> Result<ScriptFrameOutcome> {
        let execution_enabled = state.vm_execution_enabled;
        self.synchronize_script_action_runtime_state(state.clip_playback_state)?;
        self.scripts
            .prepare_ship_presentation_state(&self.ship_presentation);
        self.scripts.import_random_state(self.random);
        let random_counter_before = self.random.counter;
        let outcome =
            self.scripts
                .execute_lifecycle_frame(&mut self.runtime, state, execution_enabled);
        self.random = self.scripts.random_state();
        self.random_draws.script += random_draw_delta(random_counter_before, self.random.counter);
        let outcome = outcome?;
        self.scripts
            .finish_ship_presentation_state(&mut self.ship_presentation);
        self.publish_script_presentation_status_change()?;
        self.synchronize_script_action_effects(Some(state));
        self.synchronize_script_presentations()?;
        self.process_script_commands()?;
        Ok(outcome)
    }

    fn synchronize_script_action_runtime_state(&mut self, clip_playback_state: u16) -> Result<()> {
        let ship_navigation_active =
            self.ship_presentation.flags & SHIP_PRESENTATION_ACTIVE_FLAG != u16::MIN;
        let camera_view_transition_steps = self
            .bridge_actors
            .as_ref()
            .context("bridge actor state is already being updated")?
            .camera_transition_step();
        let loaded_scene_vertical_offset = self
            .scripts
            .backend()
            .assets()
            .location_scene_top_row()
            .unwrap_or(u16::MIN);
        self.scripts.action_state_mut().ship_navigation_mode = if ship_navigation_active {
            ScriptShipNavigationMode::Active
        } else {
            ScriptShipNavigationMode::Inactive
        };
        let voc_playback_enabled = self.audio_is_initialized();
        self.scripts
            .backend_mut()
            .set_action_runtime_state(ScriptActionRuntimeState {
                camera_approach_phase: self.runtime.camera_approach().phase,
                camera_view_transition_steps,
                ship_navigation_active,
                loaded_scene_vertical_offset,
                clip_playback_state,
                voc_playback_enabled,
            });
        Ok(())
    }

    /// Drain A8's request to clear the low byte of the native mouse-idle timer alias.
    pub fn take_script_mouse_idle_low_byte_clear_request(&mut self) -> bool {
        self.scripts.take_mouse_idle_low_byte_clear_request()
    }

    /// Drain the finale completion sampled into the native next-frame shutdown gate.
    pub fn take_script_finale_shutdown_request(&mut self) -> bool {
        std::mem::take(&mut self.script_finale_shutdown_pending)
    }

    fn publish_script_presentation_status_change(&mut self) -> Result<()> {
        if self.scripts.take_presentation_palette_dirty() {
            self.palette_transition.request_visual_color_update();
        }
        let (presentation_started, changed, clear_bridge_console) = self
            .scripts
            .last_presentation_outcome()
            .map(|outcome| {
                (
                    outcome.presentation_started.is_some(),
                    outcome.presentation_started.is_some() || outcome.presentation_ended,
                    outcome.bridge_console_selection_cleared,
                )
            })
            .unwrap_or_default();
        if presentation_started {
            self.reset_presentation_word_choice()?;
        }
        if clear_bridge_console {
            self.bridge_console
                .as_mut()
                .expect("bridge console update is not reentrant")
                .clear_selected_item_alias();
        }
        if changed {
            self.request_manu3_animation(Manu3AnimationSelector::BridgeActive);
        }
        Ok(())
    }

    /// Publish one-frame BloodScript action effects into canonical ship and lifecycle state.
    fn synchronize_script_action_effects(
        &mut self,
        mut lifecycle: Option<&mut GameLifecycleState>,
    ) {
        if let Some(state) = lifecycle.as_deref_mut() {
            state
                .profile_change_blockers
                .navigation_actor_transition_active = matches!(
                self.scripts.action_state().travel_phase,
                ScriptTravelActionPhase::WaitingForCamera
                    | ScriptTravelActionPhase::WaitingForPresentation
            );
        }
        let selected_target = self.scripts.take_selected_ship_target();
        synchronize_selected_ship_target(
            selected_target,
            self.scripts.action_state_mut(),
            &mut self.ship_presentation,
        );
        let effects = self.scripts.take_action_effects(lifecycle.is_some());
        if effects.ship_hud_refresh_requested {
            self.ship_presentation.hud_initialization_pending = 1;
        }
        if effects.screen_rebuild_requested {
            lifecycle
                .as_deref_mut()
                .expect("action effects retain screen rebuilds without a lifecycle")
                .navigation_rebuild_pending = true;
        }
        if let Some(playback_state) = effects.clip_playback_state_reload {
            lifecycle
                .as_deref_mut()
                .expect("action effects retain clip playback reloads without a lifecycle")
                .clip_playback_state = playback_state;
        }
        if effects.speaker_pulse_requested {
            lifecycle
                .as_deref_mut()
                .expect("action effects retain speaker pulses without a lifecycle")
                .speaker_pulse_requested = true;
        }
    }

    /// Copy DESCRIPT and A8 name selections into the flat presentation catalog.
    pub fn synchronize_script_presentations(&mut self) -> Result<()> {
        self.presentation_player
            .apply_descript_assets(self.scripts.backend().assets())?;
        let basename = &self.scripts.sequence_presentation().sequence_basename;
        if !basename.is_empty() {
            self.presentation_player
                .select_script_sequence_video(basename)?;
        }
        Ok(())
    }

    /// Select one authored clip from the active DESCRIPT sequence record.
    pub fn select_descript_sequence_video(&mut self, basename: &[u8]) -> Result<()> {
        self.presentation_player
            .select_descript_sequence_video(basename)
    }

    /// Select the current hyperspace clip for presentation line six.
    pub fn select_hyperspace_video(&mut self, basename: &[u8]) -> Result<()> {
        self.presentation_player.select_hyperspace_video(basename)
    }

    /// Resolve and bootstrap one authored presentation line.
    pub fn load_presentation_sequence(
        &mut self,
        line: PresentationResourceId,
        source: crate::native::bloodprg::PresentationSceneSource,
        policy: PresentationPresentPolicy,
        timer_tick: u16,
        render_snapshot_suppressed: bool,
    ) -> Result<Option<PresentationResourceSequenceOutcome>> {
        let outcome = self.presentation_player.load(
            &mut self.runtime,
            line,
            source,
            policy,
            timer_tick,
            render_snapshot_suppressed,
        )?;
        if outcome
            .as_ref()
            .is_some_and(|outcome| outcome.resource_switch.palette.copied_render_snapshot)
        {
            self.synchronize_presentation_transition_source()?;
        }
        Ok(outcome)
    }

    /// Advance the active presentation queue with explicit clock samples.
    pub fn service_presentation_sequence(
        &mut self,
        link_target: u16,
        timer_tick: u16,
        render_snapshot_suppressed: bool,
        secondary_presentation_mode: bool,
    ) -> Result<RuntimePresentationStepOutcome> {
        let audio_position = self
            .audio_ref()?
            .background_stream_remaining()
            .unwrap_or(u16::MIN);
        let clock_gates = PresentationQueueClockGates {
            primary_mode: self.bridge_screen.reverse_presentation_active,
            secondary_mode: secondary_presentation_mode,
            voice_playback: self.navigation_music_enabled()?,
        };
        let outcome = self.presentation_player.service_frame_from_link_target(
            &mut self.runtime,
            link_target,
            audio_position,
            timer_tick,
            clock_gates,
            render_snapshot_suppressed,
        )?;
        if matches!(
            &outcome.queue,
            PresentationQueueServiceOutcome::Active {
                palette: Some(palette),
                ..
            } if palette.copied_render_snapshot
        ) {
            self.synchronize_presentation_transition_source()?;
        }
        Ok(outcome)
    }

    fn synchronize_presentation_transition_source(&mut self) -> Result<()> {
        let palette = self
            .presentation_player
            .render_palette_snapshot()
            .context("presentation palette update has no active stream")?;
        self.palette_transition
            .synchronize_presentation_source(&palette);
        Ok(())
    }

    /// Return whether a presentation stream is retained and still draining.
    pub fn presentation_stream_active(&self) -> bool {
        self.presentation_player.source_open_or_draining()
    }

    /// Return the active presentation queue counters shared with subtitle cues.
    pub fn presentation_queue_metrics(&self) -> Result<Option<RuntimePresentationQueueMetrics>> {
        self.presentation_player.queue_metrics()
    }

    /// Number of HNM frames retired by the currently selected presentation stream.
    pub fn presentation_decoded_frame_count(&self) -> u64 {
        self.presentation_player.decoded_frame_count()
    }

    /// Release the retained presentation source after completion or cancellation.
    pub fn finish_presentation_sequence(&mut self) -> bool {
        self.presentation_player.finish().is_some()
    }

    /// Release every presentation owner retained across lifecycle frames.
    pub fn finish_runtime_presentations(&mut self) -> Result<()> {
        self.finish_presentation_sequence();
        self.presentation_screen
            .as_mut()
            .context("presentation screen is already being updated")?
            .state_mut()
            .set_active(false);
        self.presentation_word_choice
            .as_mut()
            .context("presentation word choice is already being updated")?
            .reset();
        Ok(())
    }

    /// Borrow resolved fixed and DESCRIPT-authored presentation metadata.
    pub const fn presentation_catalog(&self) -> &RuntimePresentationCatalog {
        self.presentation_player.catalog()
    }

    /// Borrow the concrete script backend for lifecycle-state updates.
    pub const fn script_backend(&self) -> &RuntimeScriptBackend {
        self.scripts.backend()
    }

    pub(super) const fn presentation_scan_state(&self) -> &ScriptPresentationScanState {
        self.scripts.presentation_scan_state()
    }

    /// Refresh main-loop presentation writes before a scene transition clones script state.
    pub(super) fn prepare_scene_transition_presentation(&mut self, lifecycle: &GameLifecycleState) {
        self.scripts.prepare_lifecycle_frame(lifecycle);
    }

    pub(super) fn latest_presentation_started(&self) -> Option<ScriptObjectId> {
        self.scripts
            .last_presentation_outcome()
            .and_then(|outcome| outcome.presentation_started)
    }

    pub(super) fn commit_scene_transition_presentation(
        &mut self,
        presentation: ScriptPresentationScanState,
        text: TextPresentationState,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<()> {
        *self.scripts.presentation_scan_state_mut() = presentation;
        *self.scripts.text_presentation_mut() = text;
        self.scripts.finish_lifecycle_frame(lifecycle)
    }

    /// Mutably borrow the concrete script backend for lifecycle-state updates.
    pub fn script_backend_mut(&mut self) -> &mut RuntimeScriptBackend {
        self.scripts.backend_mut()
    }

    /// Borrow the live subtitle, menu, and word-choice state produced by BloodScript.
    pub const fn text_presentation(&self) -> &TextPresentationState {
        self.scripts.text_presentation()
    }

    /// Mutably borrow the shared subtitle and inline-menu presentation state.
    pub fn text_presentation_mut(&mut self) -> &mut TextPresentationState {
        self.scripts.text_presentation_mut()
    }

    /// Publish a completed word choice to BloodScript and refresh lifecycle gates.
    pub fn complete_word_choice(
        &mut self,
        concept: ScriptWordId,
        state: &mut GameLifecycleState,
    ) -> Result<()> {
        self.scripts
            .complete_lifecycle_word_choice(&mut self.runtime, state, concept)
    }

    /// Drain ordered renderer, audio, camera, and HUD commands from BloodScript.
    pub fn take_script_commands(&mut self) -> Vec<RuntimeScriptCommand> {
        self.scripts.take_commands()
    }

    /// Apply all pending BloodScript side effects to concrete flat runtime services.
    pub fn process_script_commands(&mut self) -> Result<usize> {
        let commands = self.take_script_commands();
        let command_count = commands.len();
        for command in commands {
            match command {
                RuntimeScriptCommand::RestartNameAreaEffect => {
                    let sprite_name = self
                        .scripts
                        .backend()
                        .assets()
                        .character_sprite()
                        .context("name-area restart has no DESCRIPT character sprite")?
                        .as_bytes()
                        .to_vec();
                    self.runtime.load_name_area_sprite(&sprite_name)?;
                    self.runtime.restart_name_area_effect();
                    self.bridge_screen.palette_dirty = true;
                }
                RuntimeScriptCommand::TransitionPresentationEntity(entity) => {
                    if entity == ScriptPresentationEntity::NameAreaEffect {
                        self.runtime.stop_name_area_effect();
                    }
                    self.runtime.transition_presentation_entity(entity)?;
                }
                RuntimeScriptCommand::RestartNavigationMusic => {
                    self.restart_navigation_music()?;
                }
                RuntimeScriptCommand::PlayRadioClip { clip_index } => {
                    self.play_loaded_sound_bank_clip(clip_index)?;
                }
                RuntimeScriptCommand::StartCameraTransition { steps } => {
                    self.bridge_actors
                        .as_mut()
                        .context("bridge actor state is already being updated")?
                        .set_camera_transition_step(steps);
                }
                RuntimeScriptCommand::ResetShipHud => {
                    self.reset_ship_hud()?;
                }
            }
        }
        Ok(command_count)
    }

    /// Transition the bridge panel entity used by the six-choice presentation screen.
    pub fn transition_presentation_panel_entity(&mut self) -> Result<bool> {
        self.runtime.transition_presentation_panel_entity()
    }

    /// Restore ship artwork, HUD palette, and the flat bridge camera origin.
    pub fn reset_ship_hud(&mut self) -> Result<()> {
        self.runtime.reset_ship_hud()?;
        self.bridge_scene
            .as_mut()
            .context("ship HUD reset requires an initialized bridge scene")?
            .reset_camera();
        Ok(())
    }

    /// Draw and immediately present the pause HUD when the recovered gate is active.
    pub fn refresh_pause_hud(&mut self, active: bool) -> Result<bool> {
        if self.runtime.draw_pause_hud(active)?.is_none() {
            return Ok(false);
        }
        self.presentation.submit_indexed_frame(&self.runtime)?;
        let bridge_frame = self
            .bridge_frame
            .as_ref()
            .context("pause HUD requires a rendered bridge frame")?;
        self.presentation.present_frame(
            &self.runtime,
            bridge_frame,
            RuntimeBridgeComposition::BridgeSceneWithIndexedOverlay,
        )?;
        Ok(true)
    }

    /// Dispatch one queued SDL key and synchronize lifecycle pause and exit state.
    pub fn dispatch_lifecycle_input(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> Option<InputAction> {
        self.input.dispatch_lifecycle_input(state)
    }

    /// Sample one host pointer position into the original logical surface.
    pub fn poll_lifecycle_pointer(
        &mut self,
        output_size: [f32; 2],
        host_position: [f32; 2],
        buttons: PointerButtons,
    ) -> PointerSample {
        self.input.poll_pointer(output_size, host_position, buttons)
    }

    /// Publish a platform-owned pointer already in original logical coordinates.
    pub fn publish_lifecycle_logical_pointer(
        &mut self,
        position: [i16; 2],
        buttons: PointerButtons,
    ) -> PointerSample {
        self.input.publish_logical_pointer(position, buttons)
    }

    /// Move newly detected SDL pointer edges into the lifecycle latches.
    pub fn update_lifecycle_pointer_buttons(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> PointerButtonEdges {
        self.input.transfer_lifecycle_pointer_edges(state)
    }

    /// Advance and draw the executable-authored navigation confirmation modal.
    pub fn update_confirm_dialog(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> Result<ConfirmDialogOutcome> {
        let pointer_position = self.input.pointer_sample().position;
        let outcome = self
            .confirm_dialog
            .update(&mut self.runtime, state, pointer_position)?;
        match outcome {
            ConfirmDialogOutcome::Inactive => {}
            ConfirmDialogOutcome::Cancelled(_) => {
                self.request_manu3_animation(Manu3AnimationSelector::BlackHoleOrLeftChart)
            }
            ConfirmDialogOutcome::AwaitingChoice(_) | ConfirmDialogOutcome::Confirmed(_) => {
                self.request_manu3_animation(Manu3AnimationSelector::BridgeActive);
            }
        }
        Ok(outcome)
    }

    /// Borrow navigation state shared with the confirmation-dialog coordinator.
    pub const fn confirm_dialog_state(&self) -> &ConfirmDialogState {
        self.confirm_dialog.state()
    }

    /// Mutably borrow navigation state shared with the confirmation-dialog coordinator.
    pub fn confirm_dialog_state_mut(&mut self) -> &mut ConfirmDialogState {
        self.confirm_dialog.state_mut()
    }

    /// Reconfigure the wgpu surface after a nonzero SDL pixel-size event.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.presentation.resize(width, height);
    }

    /// Upload the complete current indexed frame and live VGA palette.
    pub fn submit_indexed_frame(&mut self) -> Result<()> {
        self.presentation.submit_indexed_frame(&self.runtime)
    }

    /// Present current indexed artwork and the optional MANU3 overlay.
    pub fn present_artwork(&mut self) -> Result<()> {
        self.ensure_main_viewport()?;
        let triangles = self
            .runtime
            .manu3()
            .map(|model| model.render_triangles())
            .unwrap_or(&[]);
        self.presentation.present_artwork(triangles)
    }

    /// Advance the translated bridge steering, panorama, and point-cloud frame.
    pub fn render_bridge_frame(&mut self, input: BridgeSceneInput) -> Result<&BridgeSceneFrame> {
        self.update_bridge_steering(input)?;
        self.update_bridge_presentation_mode_bits()?;
        self.render_current_bridge_frame()?;
        self.update_bridge_presentation_hover();
        Ok(self
            .bridge_frame
            .as_ref()
            .expect("rendered bridge frame was retained"))
    }

    /// Advance only the recovered steering state without drawing a frame.
    pub(super) fn update_bridge_steering(
        &mut self,
        input: BridgeSceneInput,
    ) -> Result<BridgeSteeringOutcome> {
        let (outcome, cursor_x) = {
            let scene = self
                .bridge_scene
                .as_mut()
                .context("bridge scene has not been initialized")?;
            let outcome = scene.update_steering(input);
            (outcome, scene.steering().cursor_ring_position as i16)
        };
        let pointer = bridge_pointer_sample(self.input.pointer_sample(), cursor_x);
        self.input
            .publish_logical_pointer(pointer.position, pointer.buttons);
        Ok(outcome)
    }

    /// Select the authored presentation band for the current panorama frame.
    pub(super) fn update_bridge_presentation_mode_bits(&mut self) -> Result<()> {
        let view_frame = self.bridge_view_frame()?;
        // The executable's independent bit-one gate has no recovered writer.
        update_presentation_bridge_mode(
            view_frame,
            RECOVERED_PRESENTATION_MODE_BLOCKED,
            &mut self.bridge_presentation_mode,
        );
        Ok(())
    }

    /// Run the recovered page flip and preserve its low-byte camera toggle bit.
    pub(super) fn flip_bridge_camera_page(
        &mut self,
        ship_active: bool,
    ) -> Result<CameraPageFlipOutcome> {
        let panorama_frame = self.bridge_view_frame()? as u16;
        let mut page_state = BridgePageState {
            palette_dirty: self.bridge_screen.palette_dirty,
            transparent_zero: self.bridge_screen.transparent_zero,
            dirty_copy_requested: self.bridge_screen.dirty_copy_requested,
        };
        let palette_refresh_in_progress = self.bridge_screen.palette_refresh_in_progress;
        {
            let mut backend = RuntimeBridgeScreenBackend {
                services: self,
                ship_active,
                palette_refresh_in_progress,
            };
            render_bridge_page(ship_active, panorama_frame, &mut page_state, &mut backend)
                .context("flipping the bridge camera page")?;
        }
        self.bridge_screen.palette_dirty = page_state.palette_dirty;
        self.bridge_screen.transparent_zero = page_state.transparent_zero;
        self.bridge_screen.dirty_copy_requested = page_state.dirty_copy_requested;

        let native_result = if ship_active {
            CAMERA_PAGE_SHIP_ACTIVE_RESULT
        } else {
            panorama_frame
        };
        Ok(if native_result & CAMERA_PAGE_TOGGLE_BIT != u16::MIN {
            CameraPageFlipOutcome::ToggleCameraView
        } else {
            CameraPageFlipOutcome::KeepCurrentView
        })
    }

    /// Presentation band selected from the current authored panorama frame.
    pub const fn bridge_presentation_mode(&self) -> Option<PresentationBridgeMode> {
        self.bridge_presentation_mode
    }

    /// Persistent hover ownership selected from panorama-authored actor boxes.
    pub const fn presentation_hover(
        &self,
    ) -> &PresentationHoverState<BridgeActorPresentationState> {
        &self.presentation_hover
    }

    /// Update presentation hover through the recovered inclusive hit test.
    pub fn update_bridge_presentation_hover(&mut self) -> PresentationHoverOutcome {
        let selection = match self.bridge_presentation_mode {
            Some(PresentationBridgeMode::Outer) => Some(PresentationHitSelection::Primary),
            Some(PresentationBridgeMode::SecondBand) => Some(PresentationHitSelection::Secondary),
            Some(PresentationBridgeMode::FirstBand | PresentationBridgeMode::ThirdBand) | None => {
                None
            }
        };
        let primary = self.nav_actor_slots[PRIMARY_PRESENTATION_ACTOR_SLOT]
            .hit_region
            .unwrap_or(DISABLED_PRESENTATION_HIT_RECT);
        let secondary = self.nav_actor_slots[SECONDARY_PRESENTATION_ACTOR_SLOT]
            .hit_region
            .unwrap_or(DISABLED_PRESENTATION_HIT_RECT);
        let outcome = update_presentation_hover(
            selection,
            PresentationHitAreas::new(primary, secondary),
            self.input.pointer_sample().position,
            BridgeActorPresentationState::PresentationHover,
            &mut self.presentation_hover,
        );
        match outcome {
            PresentationHoverOutcome::Activated => {
                self.request_manu3_animation(Manu3AnimationSelector::PresentationHover);
            }
            PresentationHoverOutcome::Deactivated => self.restore_previous_manu3_animation(),
            PresentationHoverOutcome::Disabled
            | PresentationHoverOutcome::RemainedInside
            | PresentationHoverOutcome::RemainedOutside => {}
        }
        outcome
    }

    /// Advance all six executable-authored bridge actor slots in native order.
    pub fn update_runtime_bridge_actors(
        &mut self,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<NavActorSlotUpdateOutcome> {
        let mut actors = self
            .bridge_actors
            .take()
            .context("bridge actor update is reentrant")?;
        let mut slots = std::mem::take(&mut self.nav_actor_slots);
        let outcome = actors.update(self, lifecycle, &mut slots);
        self.nav_actor_slots = slots;
        self.bridge_actors = Some(actors);
        outcome.context("updating recovered bridge actors")
    }

    /// Return the recovered panel-actor completion latch.
    pub(super) fn bridge_actor_completion_latched(&self) -> Result<bool> {
        Ok(self
            .bridge_actors
            .as_ref()
            .context("bridge actor state is already being updated")?
            .completion_latched())
    }

    /// Return the camera actor's current chart-transition countdown.
    pub(super) fn bridge_actor_camera_transition_step(&self) -> Result<u8> {
        Ok(self
            .bridge_actors
            .as_ref()
            .context("bridge actor state is already being updated")?
            .camera_transition_step())
    }

    /// Whether C6 reached the bridge coordinator's early scene-dispatch phase.
    pub(super) fn bridge_scene_dispatch_pending(&self) -> bool {
        matches!(
            self.scripts.action_state().travel_phase,
            ScriptTravelActionPhase::WaitingForPresentation
        )
    }

    /// Advance the exact destination-region camera-navigation state machine.
    pub(super) fn update_runtime_camera_navigation(
        &mut self,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<crate::native::bloodprg::CameraNavigationOutcome> {
        let mut navigation = self
            .camera_navigation
            .take()
            .context("camera navigation update is reentrant")?;
        let mut slot = self.nav_actor_slots[3];
        let outcome = navigation.update(self, lifecycle, &mut slot);
        self.nav_actor_slots[3] = slot;
        self.camera_navigation = Some(navigation);
        outcome
    }

    /// Advance the recovered chart wipe, interaction, and location panel.
    #[cfg(test)]
    pub(super) fn update_runtime_navigation_chart(
        &mut self,
        lifecycle: &mut GameLifecycleState,
        navigation_animation_phase: u8,
    ) -> Result<crate::native::bloodprg::NavigationCameraOutcome> {
        let comparison_extent = self
            .runtime
            .bridge_sprite_source_extent(usize::MIN)
            .context("reading navigation chart comparison extent")?;
        self.update_runtime_navigation_chart_with_comparison(
            lifecycle,
            navigation_animation_phase,
            comparison_extent,
        )
    }

    /// Advance the navigation chart against the coordinator's typed extent.
    pub(super) fn update_runtime_navigation_chart_with_comparison(
        &mut self,
        lifecycle: &mut GameLifecycleState,
        navigation_animation_phase: u8,
        comparison_extent: crate::native::bloodprg::BridgeSpriteExtent,
    ) -> Result<crate::native::bloodprg::NavigationCameraOutcome> {
        let transition_step = self
            .bridge_actors
            .as_ref()
            .context("bridge actor state is already being updated")?
            .camera_transition_step();
        let mut chart = self
            .navigation_chart
            .take()
            .context("navigation chart update is reentrant")?;
        let outcome = chart.update(
            self,
            lifecycle,
            transition_step,
            navigation_animation_phase,
            comparison_extent,
        );
        let remaining = chart.transition_step();
        let panel_active = chart.location_panel_active();
        self.navigation_chart = Some(chart);
        let actors = self
            .bridge_actors
            .as_mut()
            .context("bridge actor state disappeared during navigation update")?;
        actors.set_camera_transition_step(remaining);
        actors.set_location_panel_active(panel_active);
        outcome.context("updating recovered navigation chart")
    }

    /// Compose or clear the executable-authored bridge location status hover.
    pub(super) fn update_runtime_navigation_status(
        &mut self,
        lifecycle: &mut GameLifecycleState,
    ) -> Result<crate::native::bloodprg::NavigationStatusOutcome> {
        let snapshot = self
            .navigation_chart
            .as_ref()
            .context("navigation chart is already being updated")?
            .status_snapshot()
            .context("navigation status requires a decoded chart world")?;
        let mut status = self
            .navigation_status
            .take()
            .context("navigation status update is reentrant")?;
        let outcome = status.update(self, lifecycle, &snapshot);
        self.navigation_status = Some(status);
        outcome.context("updating recovered navigation status")
    }

    /// Advance the executable-authored character-name palette noise.
    pub(super) fn advance_bridge_name_area_effect(&mut self) -> Result<NameAreaEffectOutcome> {
        let Self {
            runtime,
            random,
            random_draws,
            bridge_frame,
            ..
        } = self;
        let sprite_layer = &mut bridge_frame
            .as_mut()
            .context("name-area effect requires a rendered bridge frame")?
            .actor_sprite_pixels;
        let outcome = runtime.advance_name_area_effect_on(sprite_layer, &mut |modulus| {
            let modulus = u16::try_from(modulus).unwrap_or(u16::MAX);
            random_draws.name_area_effect += 1;
            usize::from(random.next(modulus))
        })?;
        overlay_nonzero_indices(runtime.front_buffer_mut().pixels_mut(), sprite_layer);
        Ok(outcome)
    }

    /// Apply the final fixed bridge-console tint after actor completion.
    pub(super) fn remap_bridge_completion_region(&mut self) -> Result<RasterRectOutcome> {
        self.runtime.remap_bridge_completion_region()
    }

    /// Prepare a bridge frame from current steering without consuming host input.
    pub fn render_current_bridge_frame(&mut self) -> Result<&BridgeSceneFrame> {
        self.render_current_bridge_frame_with_palette_refresh(
            self.bridge_screen.palette_refresh_in_progress,
        )
    }

    pub(super) fn render_current_bridge_frame_with_palette_refresh(
        &mut self,
        refresh_live_palette: bool,
    ) -> Result<&BridgeSceneFrame> {
        self.render_current_bridge_frame_to_target(refresh_live_palette, BridgePageTarget::Primary)
    }

    fn render_current_bridge_frame_to_target(
        &mut self,
        refresh_live_palette: bool,
        target: BridgePageTarget,
    ) -> Result<&BridgeSceneFrame> {
        {
            let Self {
                runtime,
                bridge_scene,
                bridge_frame,
                bridge_palette,
                nav_actor_slots,
                ..
            } = self;
            let scene = bridge_scene
                .as_mut()
                .context("bridge scene has not been initialized")?;
            let mut live_palette = *runtime.live_palette();
            let frame = scene
                .render_current_frame_with_palette(
                    runtime.bridge_sprite_entities_mut(),
                    refresh_live_palette,
                    bridge_palette,
                    &mut live_palette,
                )
                .context("rendering current bridge scene")?;
            *runtime.live_palette_mut() = live_palette;
            for (slot, orb_box) in nav_actor_slots
                .iter_mut()
                .zip(frame.station_orb_boxes.iter().copied())
            {
                slot.hit_region = orb_box.map(|orb_box| {
                    PresentationHitRectangle::new(
                        orb_box.origin.map(|coordinate| coordinate as i16),
                        orb_box.size.map(|extent| extent as i16),
                    )
                });
            }
            *bridge_frame = Some(frame);
        }
        self.rasterize_bridge_frame_sprite_range(
            FIRST_SHIP_PROJECTION_ENTITY..AFTER_LAST_SHIP_PROJECTION_ENTITY,
        )?;
        let mut indexed_bridge_base = vec![u8::MIN; LOGICAL_FRAMEBUFFER_PIXEL_COUNT];
        self.compose_current_bridge_work_surface(&mut indexed_bridge_base)?;
        let (front, back) = self.runtime.presentation_buffers_mut();
        selected_bridge_page_mut(front, back, target).copy_from_slice(&indexed_bridge_base);
        Ok(self
            .bridge_frame
            .as_ref()
            .expect("rendered bridge frame was retained"))
    }

    /// Flatten the current modern bridge layers into one logical indexed page.
    ///
    /// This is used only as the source page for the recovered camera wipe. The
    /// order matches the bridge base pass: stars, projected objects, then
    /// panorama. The current indexed page is intentionally excluded because it
    /// contains the chart being replaced by this opening wipe.
    pub(super) fn compose_current_bridge_work_surface(&self, destination: &mut [u8]) -> Result<()> {
        if destination.len() != LOGICAL_FRAMEBUFFER_PIXEL_COUNT {
            bail!(
                "bridge work surface has {} pixels; expected {}",
                destination.len(),
                LOGICAL_FRAMEBUFFER_PIXEL_COUNT
            );
        }
        let frame = self
            .bridge_frame
            .as_ref()
            .context("bridge work-surface composition requires a rendered frame")?;
        destination.fill(u8::MIN);
        for star in &frame.starfield.plotted {
            destination[star.framebuffer_index] = star.palette_index;
        }
        overlay_nonzero_indices(destination, &frame.object_sprite_pixels);
        overlay_nonzero_indices(destination, &frame.panorama_pixels);
        Ok(())
    }

    /// Composite one recovered bridge sprite range into the retained GPU layer.
    pub(super) fn rasterize_bridge_frame_sprite_range(
        &mut self,
        entities: Range<u16>,
    ) -> Result<BridgeSpriteRasterOutcome> {
        let frame = self
            .bridge_frame
            .as_mut()
            .context("bridge sprite rasterization requires a rendered frame")?;
        let actor_end = entities.end.min(FIRST_TRANSITION_ENTITY);
        let projection_start = entities.start.max(FIRST_TRANSITION_ENTITY);
        let mut draw_requests = Vec::new();
        let mut selected_blitter_after = None;
        let mut rasterized_request_count = usize::MIN;
        if projection_start < entities.end {
            let projected = self.runtime.rasterize_ship_entity_range(
                projection_start..entities.end,
                &mut frame.object_sprite_pixels,
            )?;
            draw_requests.extend(projected.dispatch.draw_requests);
            selected_blitter_after = projected.dispatch.selected_blitter_after;
            rasterized_request_count += projected.rasterized_request_count;
        }
        if entities.start < actor_end {
            let actors = self.runtime.rasterize_ship_entity_range(
                entities.start..actor_end,
                &mut frame.actor_sprite_pixels,
            )?;
            draw_requests.extend(actors.dispatch.draw_requests);
            selected_blitter_after = actors
                .dispatch
                .selected_blitter_after
                .or(selected_blitter_after);
            rasterized_request_count += actors.rasterized_request_count;
            overlay_nonzero_indices(
                self.runtime.front_buffer_mut().pixels_mut(),
                &frame.actor_sprite_pixels,
            );
        }
        Ok(BridgeSpriteRasterOutcome {
            dispatch: crate::native::bloodprg::BridgeSpriteRenderOutcome {
                draw_requests: draw_requests.into_boxed_slice(),
                selected_blitter_after,
            },
            rasterized_request_count,
        })
    }

    /// Return the current logical panorama frame used by bridge hit testing.
    pub fn bridge_view_frame(&self) -> Result<i16> {
        let frame = self
            .bridge_scene
            .as_ref()
            .context("bridge scene has not been initialized")?
            .steering()
            .view_frame;
        i16::try_from(frame).context("bridge panorama frame exceeds the signed native range")
    }

    /// Request the recovered automatic panorama seek used when a console row opens.
    pub fn request_bridge_seek(&mut self, target_arc: u16) -> Result<()> {
        self.bridge_scene
            .as_mut()
            .context("bridge scene has not been initialized")?
            .request_seek(target_arc);
        Ok(())
    }

    /// Report whether the bridge is still completing an automatic console seek.
    pub fn bridge_seek_requested(&self) -> Result<bool> {
        Ok(self
            .bridge_scene
            .as_ref()
            .context("bridge scene has not been initialized")?
            .seek_requested())
    }

    /// Return the current automatic-seek arc retained by the bridge scene.
    pub(super) fn bridge_seek_target_arc(&self) -> Result<u16> {
        Ok(self
            .bridge_scene
            .as_ref()
            .context("bridge scene has not been initialized")?
            .seek_target_arc())
    }

    /// Advance the recovered bridge-console dispatcher and its active submenu.
    pub fn update_runtime_bridge_console(&mut self, state: &mut GameLifecycleState) -> Result<()> {
        let mut console = self
            .bridge_console
            .take()
            .context("bridge console update is reentrant")?;
        let outcome = console.update(self, state);
        self.bridge_console = Some(console);
        outcome
    }

    /// Return whether a bridge-console row retains native selection ownership.
    pub(super) fn bridge_console_item_selected(&self) -> bool {
        self.bridge_console
            .as_ref()
            .is_some_and(RuntimeBridgeConsole::selected_item_active)
    }

    /// Advance the text-speed choice at its recovered main-loop position.
    pub fn update_runtime_presentation_choice(
        &mut self,
        state: &mut GameLifecycleState,
    ) -> Result<()> {
        let mut console = self
            .bridge_console
            .take()
            .context("presentation choice update is reentrant")?;
        let outcome = console.update_presentation_choice(self, state);
        self.bridge_console = Some(console);
        outcome
    }

    /// Present one translated bridge scene frame and optional MANU3 overlay.
    pub fn present_bridge_frame(&mut self, bridge_frame: &BridgeSceneFrame) -> Result<()> {
        self.ensure_main_viewport()?;
        let composition = select_bridge_composition(
            self.presentation_player.has_stream(),
            self.presentation_screen
                .as_ref()
                .is_some_and(|screen| screen.state().active()),
            false,
            bridge_frame.steering.view_changed,
        );
        self.presentation
            .present_frame(&self.runtime, bridge_frame, composition)
    }

    /// Present the most recently generated bridge frame.
    pub fn present_current_bridge_frame(&mut self, indexed_ui_active: bool) -> Result<()> {
        self.ensure_main_viewport()?;
        let frame = self
            .bridge_frame
            .as_ref()
            .context("no rendered bridge frame is ready")?;
        let composition = select_bridge_composition(
            self.presentation_player.has_stream(),
            self.presentation_screen
                .as_ref()
                .is_some_and(|screen| screen.state().active()),
            indexed_ui_active,
            frame.steering.view_changed,
        );
        self.presentation
            .present_frame(&self.runtime, frame, composition)
    }

    /// Drop the live bridge and its owned panorama during shutdown.
    pub fn close_bridge_scene(&mut self) -> bool {
        self.bridge_frame = None;
        self.bridge_scene.take().is_some()
    }

    /// Current authored frame in the 180-frame bridge panorama ring.
    pub(super) fn current_bridge_view_frame(&self) -> Option<u16> {
        self.bridge_scene
            .as_ref()
            .map(|scene| scene.steering().view_frame)
    }

    /// Capture action-aligned flat runtime state for original-game comparison.
    pub(super) fn semantic_trace_snapshot(
        &self,
        lifecycle: &GameLifecycleState,
    ) -> Result<serde_json::Value> {
        let profile = self.runtime.current_profile();
        let profile_id = profile.map(|profile| u16::from(profile.id().value()));
        let state_array_hash = profile
            .map(|profile| {
                profile
                    .synchronized_state()
                    .map(|state| fnv1a64(&state.encode()))
                    .map_err(|error| anyhow::anyhow!("synchronizing trace state: {error:?}"))
            })
            .transpose()?;
        let character_slots_hash =
            profile.map(|profile| fnv1a64(&profile.sequence_slots().encode_save_block()));
        let assets = profile
            .map(|profile| {
                profile
                    .resources()
                    .all()
                    .into_iter()
                    .filter_map(|resource| self.runtime.data().resource_catalog().name(resource))
                    .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let text = self.text_presentation();
        let subtitle_end = text
            .subtitle_reveal_cursor
            .unwrap_or(text.subtitle_text.len())
            .min(text.subtitle_text.len());
        let subtitle = String::from_utf8_lossy(&text.subtitle_text[..subtitle_end])
            .replace('\r', " ")
            .trim()
            .to_owned();
        let pointer = self.input.pointer_sample();
        let pointer_edges = self.input.pointer_button_edges();
        let previous_buttons = self.input.previous_pointer_buttons();
        let current_bridge_frame = self.current_bridge_view_frame().unwrap_or(u16::MIN);
        let bridge_seek_requested = self.bridge_seek_requested().unwrap_or(false);
        let bridge_seek_target = self.bridge_seek_target_arc().unwrap_or(u16::MIN);
        let radio_slot = &self.nav_actor_slots[1];
        let panel_slot = &self.nav_actor_slots[2];
        let presentation_screen = self
            .presentation_screen
            .as_ref()
            .map(|screen| screen.state());
        let presentation_screen_active =
            presentation_screen.is_some_and(PresentationScreenState::active);
        let waiting_for_input = lifecycle.presentation.word_choice_active
            && !lifecycle.primary_pointer_pressed
            && lifecycle.pointer_press_pending == u8::MIN
            && !lifecycle.navigation_target_selected;
        let selector_word_choices = profile
            .map(|profile| {
                profile
                    .selector_state()
                    .pending_presentation_words()
                    .iter()
                    .filter_map(|word| profile.dictionary().word(*word))
                    .map(|label| String::from_utf8_lossy(label).into_owned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let rendered_word_choices = self
            .presentation_word_choice
            .as_ref()
            .map(|choice| {
                choice
                    .state()
                    .choices
                    .iter()
                    .map(|choice| String::from_utf8_lossy(&choice.label).into_owned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let bridge_console = self
            .bridge_console
            .as_ref()
            .map(|console| console.semantic_trace_snapshot(self.runtime.data().bridge_menu_text()));
        let script2 = profile
            .filter(|profile| profile.id().value() == SCRIPT2_PROFILE_VALUE)
            .map(|profile| {
                let state = profile.state();
                let unlock = state
                    .resolve_word_source_offset(SCRIPT2_PTERRA_UNLOCK_STATE_OFFSET)
                    .and_then(|field| state.word(field));
                let pterra = profile.directory().find_active_object(PTERRA_OBJECT_NAME);
                let pterra_in_play = pterra
                    .and_then(|object| object_has_flag(state, object, ScriptObjectFlag::InPlay));
                let init = profile
                    .directory()
                    .procedures()
                    .find_map(|(procedure, entry)| {
                        (entry.name() == SCRIPT2_INIT_PROCEDURE_NAME).then_some(procedure)
                    });
                let init_enabled = init
                    .map(|procedure| profile.procedures().is_enabled(procedure))
                    .transpose()
                    .map_err(|error| anyhow::anyhow!("reading SCRIPT2 init state: {error}"))?;
                Ok::<_, anyhow::Error>(serde_json::json!({
                    "globals_a0": unlock,
                    "init_enabled": init_enabled,
                    "pterra_record": pterra.map(ScriptObjectId::index),
                    "pterra_in_play": pterra_in_play,
                }))
            })
            .transpose()?;
        let navigation_chart = self
            .navigation_chart
            .as_ref()
            .map(RuntimeNavigationChart::semantic_trace_snapshot);
        let navigation_target = self
            .scripts
            .action_state()
            .current_ship_target
            .map(|record| {
                let name = profile
                    .and_then(|profile| profile.directory().object(record))
                    .map(|entry| String::from_utf8_lossy(entry.name()).into_owned());
                serde_json::json!({
                    "record": record.index(),
                    "name": name,
                })
            });
        let navigation = serde_json::json!({
            "chart": navigation_chart,
            "target": navigation_target,
            "ship_mode": format!("{:?}", self.scripts.action_state().ship_navigation_mode),
            "travel_phase": format!("{:?}", self.scripts.action_state().travel_phase),
        });
        let save_load = self.save_load.as_ref().map(|save_load| {
            let state = save_load.state();
            let phase = match state.phase {
                SaveLoadMenuPhase::Ready => "ready",
                SaveLoadMenuPhase::LayoutPending => "layout_pending",
                SaveLoadMenuPhase::Transitioning => "transitioning",
            };
            serde_json::json!({
                "active": state.is_active(),
                "save_requested": state.requests.save,
                "load_requested": state.requests.load,
                "quick_save_requested": state.requests.quick_save,
                "phase": phase,
                "ui_flags": state.ui_flags,
                "selected_slot": state.selected_slot,
                "active_slot": state.active_slot,
                "name_length": state.name_length,
                "redraw_pending": state.redraw_pending,
                "palette_dirty": state.palette_dirty,
            })
        });
        let pending_presentation_owner = self.pending_ship_presentation_owner().map(|record| {
            let name = profile
                .and_then(|profile| profile.directory().object(record))
                .map(|entry| String::from_utf8_lossy(entry.name()).into_owned());
            serde_json::json!({
                "record": record.index(),
                "name": name,
            })
        });
        let active_actor_presentation = profile
            .and_then(|profile| profile.active_actor_presentation_related())
            .map(|record| {
                let name = profile
                    .and_then(|profile| profile.directory().object(record))
                    .map(|entry| String::from_utf8_lossy(entry.name()).into_owned());
                serde_json::json!({
                    "record": record.index(),
                    "name": name,
                })
            });
        let screen_hash = fnv1a64(self.runtime.front_buffer().pixels());
        let palette_bytes: Vec<_> = self
            .runtime
            .live_palette()
            .iter()
            .flatten()
            .copied()
            .collect();
        let bridge_layers = self.bridge_frame.as_ref().map(|frame| {
            serde_json::json!({
                "panorama_hash": fnv1a64(&frame.panorama_pixels),
                "object_sprite_hash": fnv1a64(&frame.object_sprite_pixels),
                "actor_sprite_hash": fnv1a64(&frame.actor_sprite_pixels),
                "actor_sprite_metrics": indexed_layer_metrics(&frame.actor_sprite_pixels),
            })
        });
        let portrait = self.runtime.bridge_sprite_entities()[NAME_AREA_EFFECT_ENTITY_INDEX];
        let portrait_source = portrait.frame.map(|frame| match frame.source {
            crate::native::bloodprg::BridgeSpriteFrameSource::CachedResource {
                resource,
                byte_offset,
            } => serde_json::json!({
                "kind": "cached",
                "resource": resource.value(),
                "byte_offset": byte_offset,
                "frame_index": frame.frame_index,
            }),
            crate::native::bloodprg::BridgeSpriteFrameSource::RetainedFramebuffer => {
                serde_json::json!({
                    "kind": "retained_framebuffer",
                    "frame_index": frame.frame_index,
                })
            }
        });
        let portrait_palette = self.runtime.live_palette()
            [NAME_AREA_PALETTE_FIRST..NAME_AREA_PALETTE_AFTER_LAST]
            .to_vec();
        let name_area_effect = self.runtime.name_area_effect();
        let name_area_operation = match name_area_effect.operation {
            commander_blood_formats::name_area_effect::NameAreaEffectOperation::CollapseToFirst => {
                0
            }
            commander_blood_formats::name_area_effect::NameAreaEffectOperation::CollapseToLast => 1,
            commander_blood_formats::name_area_effect::NameAreaEffectOperation::CycleForward => 2,
            commander_blood_formats::name_area_effect::NameAreaEffectOperation::FadeBackward => 3,
        };
        let text_state = serde_json::json!({
            "dialogue_word_delay": self.dialogue_word_delay()?,
            "start_locked": lifecycle.presentation.start_locked,
            "hold_ready": lifecycle.presentation.hold_ready,
            "dialogue_hold_complete": lifecycle.presentation.dialogue_hold_complete,
            "word_buffer_nonempty": lifecycle.presentation.word_buffer_nonempty,
            "text_menu_pending": lifecycle.presentation.text_menu_pending,
            "scene_gate_active": lifecycle.presentation.scene_gate_active,
            "sequence_active": lifecycle.presentation.sequence_active,
            "menu_word_count": text.menu_word_count,
            "dialogue_chatter_active": text.dialogue_chatter_active,
            "dialogue_chatter_seed_pending": text.dialogue_chatter_seed_pending,
            "subtitle_voice_trigger": text.subtitle_voice_trigger,
            "owned_hold_ready": text.hold_ready,
            "owned_dialogue_hold_complete": text.dialogue_hold_complete,
            "owned_dialogue_hold_countdown": text.dialogue_hold_countdown,
        });

        Ok(serde_json::json!({
            "vm": {
                "resource_profile": profile_id,
                "profile_request": lifecycle.pending_profile.map(|profile| i16::from(profile.value())).unwrap_or(-1),
                "execution_enabled": u8::from(lifecycle.vm_execution_enabled),
                "resource_handles": assets,
                "active_line": lifecycle.presentation.active_line,
                "displayed_line": self.ship_presentation.active_line,
            },
            "presentation": {
                "ui_flags": lifecycle.low_ui_state_word(),
                "actor_transition": u8::from(lifecycle.profile_change_blockers.navigation_actor_transition_active),
                "bridge_frame": current_bridge_frame as i16,
                "bridge_seek_requested": bridge_seek_requested,
                "bridge_seek_target": bridge_seek_target,
                "ship_flags": self.ship_presentation.flags,
                "ship_ui_state": self.ship_presentation.ui_state,
                "mode": u8::from(lifecycle.presentation_mode),
                "box_mode": u8::from(presentation_screen_active),
                "screen_phase": presentation_screen
                    .map(|screen| screen.phase().executable_value())
                    .unwrap_or(u16::MIN),
                "screen_reverse": presentation_screen.is_some_and(PresentationScreenState::reverse),
                "navigation_rebuild_pending": lifecycle.navigation_rebuild_pending,
                "word_choice_active": u8::from(lifecycle.presentation.word_choice_active),
                "selector_word_choices": selector_word_choices,
                "rendered_word_choices": rendered_word_choices,
                "pending_presentation_owner": pending_presentation_owner,
                "active_actor_presentation": active_actor_presentation,
                "nav_target_selection": u8::from(lifecycle.navigation_target_selected),
                "active": u8::from(lifecycle.presentation.active),
                "defer": u8::from(lifecycle.presentation.menu_deferred),
                "text_state": text_state,
                "text_wait": u8::from(lifecycle.presentation.word_choice_active) * 2,
                "text_display_active": u8::from(lifecycle.presentation.subtitle_display_active),
                "request_flags": lifecycle.presentation.request_flags.bits(),
                "screen_active": presentation_screen_active,
                "manu3_requested": self.manu3_hand.requested_animation,
                "manu3_current": self.manu3_hand.current_animation,
                "radio_slot": {
                    "active": radio_slot.flags.active,
                    "auto_seek": radio_slot.flags.auto_seek,
                    "locked": radio_slot.flags.locked,
                    "present": radio_slot.flags.active,
                    "ready": radio_slot.flags.auto_seek,
                    "loaded": radio_slot.flags.clear_mouse_before_hit,
                    "frame": radio_slot.line.frame,
                    "terminal_frame": radio_slot.line.terminal_frame,
                },
                "panel_slot": {
                    "active": panel_slot.flags.active,
                    "auto_seek": panel_slot.flags.auto_seek,
                    "locked": panel_slot.flags.locked,
                    "loaded": panel_slot.flags.clear_mouse_before_hit,
                    "frame": panel_slot.line.frame,
                    "terminal_frame": panel_slot.line.terminal_frame,
                },
                "portrait_entity": {
                    "flags": portrait.flags.bits(),
                    "source": portrait_source,
                    "source_extent": [portrait.source_extent.width, portrait.source_extent.height],
                    "draw_position": [portrait.draw_position.x, portrait.draw_position.y],
                    "extent": [portrait.extent.width, portrait.extent.height],
                    "committed_position": [
                        portrait.committed_draw_position.x,
                        portrait.committed_draw_position.y,
                    ],
                    "committed_extent": [
                        portrait.committed_extent.width,
                        portrait.committed_extent.height,
                    ],
                    "palette": portrait_palette,
                },
                "waiting_for_input": waiting_for_input,
            },
            "bridge_console": bridge_console,
            "script2": script2,
            "navigation": navigation,
            "save_load": save_load,
            "input": {
                "mouse_x": pointer.position[0],
                "mouse_y": pointer.position[1],
                "buttons": pointer.buttons.bits(),
                "previous_buttons": previous_buttons.bits(),
                "primary_pressed": u8::from(pointer_edges.primary_pressed || lifecycle.primary_pointer_pressed),
                "press_pending": lifecycle.pointer_press_pending,
            },
            "audio": {
                "driver_pending": u8::from(self.audio.is_none()),
                "stream_mode": u8::from(self.presentation_stream_active()),
                "stream_channel": u8::MIN,
                "dialogue_delay": self.audio_events.dialogue_delay,
                "dialogue_hold": lifecycle.presentation.dialogue_hold_countdown,
                "timer_tick": self.game_timer_tick,
                "clip_playback_state": lifecycle.clip_playback_state,
                "last_clip": self.audio_events.last_clip,
                "streamed_clip_count": u16::MIN,
                "events": [],
            },
            "subtitle": subtitle,
            "persistent": {
                "state_array_hash": state_array_hash,
                "character_slots_hash": character_slots_hash,
                "record_block": null,
                "record_hash": state_array_hash,
            },
            "random": {
                "seed": self.random.seed,
                "mix_low": self.random.mix_low,
                "mix_high": self.random.mix_high,
                "counter": self.random.counter,
                "draws": {
                    "startup_point_cloud": self.random_draws.startup_point_cloud,
                    "script": self.random_draws.script,
                    "audio": self.random_draws.audio,
                    "name_area_effect": self.random_draws.name_area_effect,
                    "presentation_noise": self.random_draws.presentation_noise,
                },
            },
            "name_area_effect": {
                "active": name_area_effect.active,
                "restart_requested": name_area_effect.restart_requested,
                "sequence_index": name_area_effect.active.then_some(name_area_effect.sequence_index),
                "frame_index": name_area_effect.active.then_some(name_area_effect.frame_index),
                "operation": name_area_operation,
                "frames_remaining": name_area_effect.frames_remaining,
                "frame_cursor": null,
            },
            "assets": assets,
            "video": {
                "screen_hash": screen_hash,
                "palette_hash": fnv1a64(&palette_bytes),
                "bridge_layers": bridge_layers,
            },
        }))
    }

    /// Core owned game state used by translated script and scene systems.
    pub const fn runtime(&self) -> &OriginalGameRuntime {
        &self.runtime
    }

    /// Mutable core state for translated script and scene updates.
    pub fn runtime_mut(&mut self) -> &mut OriginalGameRuntime {
        &mut self.runtime
    }

    /// SDL input queue, latches, and logical pointer sampler.
    pub const fn input(&self) -> &RuntimeInputHost {
        &self.input
    }

    /// Mutable SDL input service used by the event pump.
    pub fn input_mut(&mut self) -> &mut RuntimeInputHost {
        &mut self.input
    }

    /// Number of frames published by the wgpu presentation service.
    pub const fn presented_frame_count(&self) -> u64 {
        self.presentation.presented_frame_count()
    }

    fn ensure_main_viewport(&self) -> Result<()> {
        if !self.main_viewport_configured {
            bail!("main logical viewport has not been configured");
        }
        Ok(())
    }

    fn audio_ref(&self) -> Result<&RuntimeAudioHost> {
        self.audio
            .as_ref()
            .context("runtime audio has not been initialized")
    }

    fn audio_mut(&mut self) -> Result<&mut RuntimeAudioHost> {
        self.audio
            .as_mut()
            .context("runtime audio has not been initialized")
    }
}

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn indexed_layer_metrics(pixels: &[u8]) -> serde_json::Value {
    let mut nonzero_count = usize::MIN;
    let mut minimum = [usize::MAX; 2];
    let mut maximum = [usize::MIN; 2];
    let mut used = [false; 256];
    for (index, palette_index) in pixels.iter().copied().enumerate() {
        if palette_index == u8::MIN {
            continue;
        }
        let x = index % LOGICAL_FRAMEBUFFER_WIDTH;
        let y = index / LOGICAL_FRAMEBUFFER_WIDTH;
        nonzero_count += 1;
        minimum[0] = minimum[0].min(x);
        minimum[1] = minimum[1].min(y);
        maximum[0] = maximum[0].max(x);
        maximum[1] = maximum[1].max(y);
        used[usize::from(palette_index)] = true;
    }
    let palette_indices = used
        .iter()
        .enumerate()
        .filter_map(|(index, used)| used.then_some(index))
        .collect::<Vec<_>>();
    serde_json::json!({
        "nonzero_count": nonzero_count,
        "bounds": (nonzero_count != usize::MIN).then_some([minimum, maximum]),
        "palette_indices": palette_indices,
    })
}

fn overlay_nonzero_indices(destination: &mut [u8], source: &[u8]) {
    for (destination, source) in destination.iter_mut().zip(source.iter().copied()) {
        if source != u8::MIN {
            *destination = source;
        }
    }
}

const fn select_bridge_composition(
    presentation_stream_active: bool,
    presentation_panel_active: bool,
    indexed_ui_active: bool,
    bridge_view_changed: bool,
) -> RuntimeBridgeComposition {
    if presentation_panel_active {
        RuntimeBridgeComposition::IndexedFramebuffer
    } else if presentation_stream_active {
        RuntimeBridgeComposition::IndexedFramebuffer
    } else if indexed_ui_active {
        RuntimeBridgeComposition::BridgeSceneWithIndexedOverlay
    } else if bridge_view_changed {
        RuntimeBridgeComposition::BridgeScene
    } else {
        RuntimeBridgeComposition::BridgeScene
    }
}

struct RuntimeBridgeScreenBackend<'services, 'window> {
    services: &'services mut ModernGameServices<'window>,
    ship_active: bool,
    palette_refresh_in_progress: bool,
}

impl RuntimeBridgeScreenBackend<'_, '_> {
    fn ensure_panorama_frame(&mut self, frame: u16, refresh_live_palette: bool) -> Result<()> {
        let current_frame = self
            .services
            .bridge_frame
            .as_ref()
            .map(|bridge_frame| bridge_frame.panorama_frame);
        if current_frame != Some(usize::from(frame)) {
            self.services
                .render_current_bridge_frame_with_palette_refresh(refresh_live_palette)?;
        }
        let actual_frame = self
            .services
            .bridge_frame
            .as_ref()
            .context("bridge frame was not retained after panorama preparation")?
            .panorama_frame;
        if actual_frame != usize::from(frame) {
            bail!(
                "bridge screen requested panorama frame {frame}, but steering prepared {actual_frame}"
            );
        }
        Ok(())
    }

    fn compose_panorama_page(
        &mut self,
        target: BridgePageTarget,
        transparent_zero: bool,
    ) -> Result<()> {
        let panorama = &self
            .services
            .bridge_frame
            .as_ref()
            .context("bridge frame was not retained for page composition")?
            .panorama_pixels;
        let (front, back) = self.services.runtime.presentation_buffers_mut();
        let destination = selected_bridge_page_mut(front, back, target);
        compose_bridge_page(destination, panorama, transparent_zero)
    }
}

impl BridgeScreenInitializationBackend for RuntimeBridgeScreenBackend<'_, '_> {
    type Error = anyhow::Error;

    fn prepare_page(&mut self, state: &mut BridgeScreenInitializationState) -> Result<()> {
        self.palette_refresh_in_progress = state.palette_refresh_in_progress;
        let panorama_frame = self.services.bridge_view_frame()? as u16;
        let mut page_state = BridgePageState {
            palette_dirty: state.palette_dirty,
            transparent_zero: state.transparent_zero,
            dirty_copy_requested: state.dirty_copy_requested,
        };
        let ship_active = self.ship_active;
        render_bridge_page(ship_active, panorama_frame, &mut page_state, self)
            .context("running the recovered bridge page coordinator")?;
        state.palette_dirty = page_state.palette_dirty;
        state.transparent_zero = page_state.transparent_zero;
        state.dirty_copy_requested = page_state.dirty_copy_requested;
        Ok(())
    }

    fn load_panorama_frame(
        &mut self,
        frame: u16,
        panorama_palette: &mut IndexedGamePalette,
        state: &mut BridgeScreenInitializationState,
    ) -> Result<()> {
        self.ensure_panorama_frame(frame, state.palette_refresh_in_progress)?;
        self.compose_panorama_page(BridgePageTarget::Primary, state.transparent_zero)?;
        *panorama_palette = self.services.bridge_palette;
        Ok(())
    }

    fn clear_secondary_page(&mut self, _state: &mut BridgeScreenInitializationState) -> Result<()> {
        self.services.runtime.clear_back_buffer();
        self.services.bridge_frame = None;
        Ok(())
    }

    fn populate_bridge_background(
        &mut self,
        _panorama_palette: &mut IndexedGamePalette,
        _state: &mut BridgeScreenInitializationState,
    ) -> Result<()> {
        self.services
            .runtime
            .activate_retained_bridge_background()
            .context("populating the bridge background during screen initialization")
    }

    fn mark_presentation_entity_dirty(
        &mut self,
        _state: &mut BridgeScreenInitializationState,
    ) -> Result<()> {
        self.services
            .runtime
            .transition_presentation_entity(ScriptPresentationEntity::DialogueOverlay)
            .map(|_| ())
            .context("transitioning the bridge presentation entity during screen initialization")
    }

    fn build_palette_adjustment(
        &mut self,
        adjustment: BridgePaletteAdjustment,
        _state: &mut BridgeScreenInitializationState,
    ) -> Result<()> {
        if adjustment != BRIDGE_DARK_PALETTE_ADJUSTMENT {
            bail!("bridge screen requested an unsupported palette adjustment: {adjustment:?}");
        }
        self.services.runtime.rebuild_bridge_dark_remap_table()
    }

    fn build_console_tint(
        &mut self,
        first_palette_index: u8,
        _state: &mut BridgeScreenInitializationState,
    ) -> Result<()> {
        if first_palette_index != BRIDGE_CONSOLE_TINT_FIRST {
            bail!(
                "bridge screen requested console tint index {first_palette_index}; expected {BRIDGE_CONSOLE_TINT_FIRST}"
            );
        }
        self.services
            .runtime
            .rebuild_bridge_console_tint_table(first_palette_index)
    }
}

impl BridgePageBackend for RuntimeBridgeScreenBackend<'_, '_> {
    type Error = anyhow::Error;

    fn clear_page(&mut self, target: BridgePageTarget, _state: &BridgePageState) -> Result<()> {
        if target != BridgePageTarget::Secondary {
            bail!("bridge page clear requested unsupported target {target:?}");
        }
        self.services.runtime.clear_back_buffer();
        self.services.bridge_frame = None;
        Ok(())
    }

    fn build_ship_projection(&mut self, _state: &BridgePageState) -> Result<()> {
        self.services
            .bridge_scene
            .as_mut()
            .context("bridge page projection requires an initialized scene")?
            .build_camera_projection_matrix()
            .context("building bridge page ship projection")
    }

    fn project_ship_point_cloud(&mut self, _state: &BridgePageState) -> Result<()> {
        self.services.project_camera_point_cloud()
    }

    fn project_ship_objects(&mut self, _state: &BridgePageState) -> Result<()> {
        self.services.project_camera_object_sprites()
    }

    fn commit_ship_sprites(
        &mut self,
        target: BridgePageTarget,
        _state: &BridgePageState,
    ) -> Result<()> {
        if target != BridgePageTarget::Secondary {
            bail!("bridge sprite commit requested unsupported target {target:?}");
        }
        self.services
            .commit_ship_entities(FIRST_SHIP_PROJECTION_ENTITY..AFTER_LAST_SHIP_PROJECTION_ENTITY)
            .map(|_| ())
    }

    fn render_ship_sprites(
        &mut self,
        target: BridgePageTarget,
        _state: &BridgePageState,
    ) -> Result<()> {
        if target != BridgePageTarget::Secondary {
            bail!("bridge sprite render requested unsupported target {target:?}");
        }
        self.services
            .render_current_bridge_frame_to_target(
                self.palette_refresh_in_progress,
                BridgePageTarget::Secondary,
            )
            .map(|_| ())
    }

    fn load_panorama_frame(
        &mut self,
        target: BridgePageTarget,
        frame: u16,
        state: &BridgePageState,
    ) -> Result<()> {
        if target != BridgePageTarget::Primary {
            bail!("bridge panorama load requested unsupported target {target:?}");
        }
        self.ensure_panorama_frame(frame, self.palette_refresh_in_progress)?;
        self.compose_panorama_page(target, state.transparent_zero)
    }
}

fn compose_bridge_page(
    destination: &mut [u8],
    panorama: &[u8],
    transparent_zero: bool,
) -> Result<()> {
    if destination.len() != panorama.len() {
        bail!(
            "bridge page has {} pixels, but panorama has {}",
            destination.len(),
            panorama.len()
        );
    }
    if transparent_zero {
        overlay_nonzero_indices(destination, panorama);
    } else {
        destination.copy_from_slice(panorama);
    }
    Ok(())
}

fn selected_bridge_page_mut<'page>(
    primary: &'page mut [u8],
    secondary: &'page mut [u8],
    target: BridgePageTarget,
) -> &'page mut [u8] {
    match target {
        BridgePageTarget::Primary => primary,
        BridgePageTarget::Secondary => secondary,
    }
}

fn bridge_pointer_sample(mut pointer: PointerSample, cursor_x: i16) -> PointerSample {
    pointer.position[0] = cursor_x;
    pointer
}

fn prefixed_resource_name(directory: &[u8], name: &[u8]) -> Result<BloodResourceName> {
    if name.contains(&b'/') || name.contains(&b'\\') {
        return BloodResourceName::new(name).context("validating authored audio resource path");
    }
    let mut path = Vec::with_capacity(directory.len() + name.len());
    path.extend_from_slice(directory);
    path.extend_from_slice(name);
    BloodResourceName::new(path).context("validating prefixed audio resource path")
}

struct RuntimeInlineMenuMetrics<'fonts> {
    fonts: &'fonts BloodprgFontResources,
    scratch: Box<[u8]>,
    error: Option<anyhow::Error>,
}

impl<'fonts> RuntimeInlineMenuMetrics<'fonts> {
    fn new(fonts: &'fonts BloodprgFontResources) -> Self {
        Self {
            fonts,
            scratch: vec![u8::MIN; LOGICAL_FRAMEBUFFER_PIXEL_COUNT].into_boxed_slice(),
            error: None,
        }
    }

    fn record_width(&mut self, result: Result<u16>) -> u16 {
        match result {
            Ok(width) => width,
            Err(error) => {
                if self.error.is_none() {
                    self.error = Some(error);
                }
                u16::MIN
            }
        }
    }

    fn finish(self) -> Result<()> {
        match self.error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl InlineMenuTextMetrics for RuntimeInlineMenuMetrics<'_> {
    fn rendered_width(&mut self, _word: ScriptWordId, text: &[u8]) -> u16 {
        self.scratch.fill(u8::MIN);
        let result = draw_planar_dialogue_text(
            &mut self.scratch,
            self.fonts,
            text,
            MENU_WIDTH_PROBE_ORIGIN,
            FULL_LOGICAL_FONT_BAND,
            MENU_WIDTH_PROBE_COLOR,
        )
        .context("measuring an inline menu word through the recovered draw routine")
        .map(|outcome| outcome.draw_width);
        self.record_width(result)
    }

    fn lookahead_width(&mut self, word: Option<(ScriptWordId, &[u8])>) -> u16 {
        let result = word.map_or(Ok(u16::MIN), |(_word, text)| {
            measure_game_text_width(text, GameFontFace::Main, self.fonts)
                .context("measuring inline menu lookahead")
        });
        self.record_width(result)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use commander_blood_formats::bloodprg::decode_bloodprg_font_resources;
    use commander_blood_formats::instruction::ScriptTimerSlot;
    use commander_blood_formats::script::decode_script_dictionary;

    use super::*;
    use crate::native::bloodprg::{
        BridgeSpriteFrameSource, CREDITS_VOICE_RESOURCE_PATH, ChoiceListRowKind, GameTimerContext,
        GameTimerState, PointerButton, PresentationResourceLine, ResourceId, ScriptDeferredRecord,
        advance_game_timer_tick, update_game_presentation_ownership,
    };
    use crate::runtime::OriginalGameDataPaths;
    use crate::runtime::camera_approach::update_runtime_camera_approach;

    const TEST_CLOCK_SEED: u8 = 17;
    const TEST_SCRIPT_CLOCK: ScriptClock = ScriptClock {
        hour: 12,
        day: 2,
        month: 1,
    };
    const HYPERSPACE_PRESENTATION_LINE: PresentationResourceId = PresentationResourceId::new(6);
    const SCRIPT_RADIO_CLIP_COUNTDOWN: u16 = 2;

    #[test]
    fn authored_finale_latches_only_when_its_active_line_finishes() {
        let mut shutdown_pending = false;
        let finale_line = PresentationResourceLine::Sequence.number();

        latch_script_finale_completion(
            &mut shutdown_pending,
            true,
            Some(finale_line),
            Some(finale_line),
        );
        assert!(!shutdown_pending);
        latch_script_finale_completion(&mut shutdown_pending, false, Some(finale_line), None);
        assert!(!shutdown_pending);
        latch_script_finale_completion(&mut shutdown_pending, true, Some(finale_line), None);
        assert!(shutdown_pending);
    }

    #[test]
    fn presentation_screen_redraw_latch_owns_the_native_modal_ui_bit() {
        let mut lifecycle = GameLifecycleState::default();

        publish_presentation_screen_modal_ui(&mut lifecycle, true);
        assert!(lifecycle.modal_ui_busy());

        publish_presentation_screen_modal_ui(&mut lifecycle, false);
        assert!(!lifecycle.modal_ui_busy());
    }

    #[test]
    fn bridge_composition_preserves_indexed_ui_while_the_bridge_moves() {
        assert_eq!(
            select_bridge_composition(true, false, false, false),
            RuntimeBridgeComposition::IndexedFramebuffer
        );
        assert_eq!(
            select_bridge_composition(true, false, false, true),
            RuntimeBridgeComposition::IndexedFramebuffer
        );
        assert_eq!(
            select_bridge_composition(false, true, false, false),
            RuntimeBridgeComposition::IndexedFramebuffer
        );
        assert_eq!(
            select_bridge_composition(true, true, false, true),
            RuntimeBridgeComposition::IndexedFramebuffer
        );
        assert_eq!(
            select_bridge_composition(false, false, true, true),
            RuntimeBridgeComposition::BridgeSceneWithIndexedOverlay
        );
        assert_eq!(
            select_bridge_composition(false, false, true, false),
            RuntimeBridgeComposition::BridgeSceneWithIndexedOverlay
        );
        assert_eq!(
            select_bridge_composition(false, false, false, false),
            RuntimeBridgeComposition::BridgeScene
        );
    }

    #[test]
    fn missing_c1_target_does_not_activate_navigation_or_line_three() {
        let mut action = ScriptActionState::default();
        let mut ship = ShipPresentationState {
            flags: 1,
            bridge_redraw_pending: 1,
            active_line: 4,
            ..ShipPresentationState::default()
        };
        let expected_action = action.clone();
        let expected_ship = ship;

        synchronize_selected_ship_target(None, &mut action, &mut ship);

        assert_eq!(action, expected_action);
        assert_eq!(ship, expected_ship);
    }

    const MAXIMUM_CAMERA_TRANSITION_FRAMES: usize = 2_048;
    const TEST_OUTPUT_SIZE: [f32; 2] = [640.0, 480.0];
    const LOGICAL_TEST_OUTPUT_SIZE: [f32; 2] = [320.0, 200.0];
    const TEST_LOGICAL_TO_HOST_SCALE: [f32; 2] = [2.0, 2.4];
    const FIRST_ADVANCED_PRESENTATION_FRAME: u16 = 1;
    const OBJECT_ACCESS_COUNTER_BYTE_OFFSET: usize = 20;
    const VISITED_DESTINATION_COUNT: u8 = 1;
    const LOCATION_PANEL_OPENING_SETTLE_FRAMES: usize = 9;
    const LOCATION_PANEL_CLOSING_SETTLE_FRAMES: usize = 8;
    const POINTER_PRESS_PENDING: u8 = 1;

    #[test]
    fn bridge_page_composition_preserves_chart_pixels_only_in_transparent_mode() {
        let panorama = [u8::MIN, 7, u8::MIN, 9];
        let mut transparent_page = [1, 2, 3, 4];
        compose_bridge_page(&mut transparent_page, &panorama, true).unwrap();
        assert_eq!(transparent_page, [1, 7, 3, 9]);

        let mut replacing_page = [1, 2, 3, 4];
        compose_bridge_page(&mut replacing_page, &panorama, false).unwrap();
        assert_eq!(replacing_page, panorama);
    }

    #[test]
    fn bridge_page_targets_preserve_the_original_primary_secondary_mapping() {
        let mut primary = [1, 2];
        let mut secondary = [3, 4];

        selected_bridge_page_mut(&mut primary, &mut secondary, BridgePageTarget::Secondary)
            .copy_from_slice(&[5, 6]);
        assert_eq!(primary, [1, 2]);
        assert_eq!(secondary, [5, 6]);

        selected_bridge_page_mut(&mut primary, &mut secondary, BridgePageTarget::Primary)
            .copy_from_slice(&[7, 8]);
        assert_eq!(primary, [7, 8]);
        assert_eq!(secondary, [5, 6]);
    }

    #[test]
    fn recovered_steering_x_replaces_raw_x_before_bridge_interactions() {
        let buttons = PointerButtons::from_bits(PointerButton::Primary as u16);
        let pointer = PointerSample {
            position: [301, 87],
            buttons,
        };

        let synchronized = bridge_pointer_sample(pointer, 142);

        assert_eq!(synchronized.position, [142, 87]);
        assert_eq!(synchronized.buttons, buttons);
    }

    const STATUS_TEST_ORIGIN: [u16; 2] = [40, 50];
    const STATUS_TEST_EXTENT: [u16; 2] = [30, 20];
    const STATUS_TEST_ACTIVE_FLAGS: u16 = 1;
    const PTERRA_NAME: &[u8] = b"Pterra";
    const ARK_NAME: &[u8] = b"Ark";
    const PTERRA_BACKGROUND_NAME: &[u8] = b"pterra1f.lbm";
    const ALTERNATE_SCRIPT_PROFILE_VALUE: u8 = 1;
    const TARGET_SELECTOR_SETTLE_FRAME_LIMIT: usize = 16;
    const PHONE_BRIDGE_TARGET_ARC: u16 = 90;
    const PHONE_BRIDGE_VIEW_FRAME: i16 = 45;
    const STARTUP_PHONE_TIMER_SLOT: u8 = 22;
    const STARTUP_PHONE_TIMER_VALUE: u16 = 5;
    const TIMER_TICKS_PER_GAME_FRAME: usize = 8;
    const STARTUP_PHONE_FRAME_LIMIT: usize = 160;
    const IZWALITO_NAME: &[u8] = b"Izwalito";
    const IZWALITO_SPRITE: &[u8] = b"izwalito.spr";
    const IZWALITO_IDLE_PRESENTATION_LINE: PresentationResourceId = PresentationResourceId::new(8);
    const IZWALITO_IDLE_VIDEO: &[u8] = b"PE\\aaisw.hnm";
    const AUTHORED_RADIO_TERMINAL_FRAME: u16 = 11;
    const FIRST_BRIDGE_ENTITY: u16 = 0;
    const AFTER_LAST_BRIDGE_ENTITY: u16 = BRIDGE_SPRITE_ENTITY_COUNT as u16;
    const FIRST_ACTOR_ENTITY: u16 = 1;
    const AFTER_LAST_ACTOR_ENTITY: u16 = 20;
    const NAME_AREA_EFFECT_ENTITY_INDEX: usize = 2;
    const NAME_AREA_EFFECT_RESOURCE: ResourceId = ResourceId::new(7);
    const NAME_AREA_EFFECT_POSITION: [u16; 2] = [16, 74];
    const NONZERO_SHIP_DEPTH_OFFSET: u16 = 73;
    const RADIO_COMPLETION_FRAME_LIMIT: usize = AUTHORED_RADIO_TERMINAL_FRAME as usize + 2;
    const AUTHORED_ACTOR_RESOURCES: [u16; NAV_ACTOR_SLOT_COUNT] = [17, 13, 15, 16, 19, 18];
    const AUTHORED_ACTOR_TRANSITION_RESOURCES: [Option<u16>; NAV_ACTOR_SLOT_COUNT] =
        [None, None, None, None, Some(21), Some(20)];
    const AUTHORED_ACTOR_TARGET_ARCS: [u16; NAV_ACTOR_SLOT_COUNT] = [0, 90, 180, 270, 0, 0];
    const AUTHORED_ACTOR_DRAW_POSITIONS: [[u16; 2]; NAV_ACTOR_SLOT_COUNT] = [
        [132, 108],
        [110, 42],
        [137, 139],
        [156, 63],
        [17, 104],
        [195, 110],
    ];

    static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(u64::MIN);

    struct TemporaryRoot(std::path::PathBuf);

    impl TemporaryRoot {
        fn create() -> Self {
            let sequence = TEMPORARY_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "commander-blood-services-test-{}-{sequence}",
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
    fn inline_menu_metrics_use_valid_recovered_font_geometry() {
        let fonts =
            decode_bloodprg_font_resources(include_bytes!("../../../../re/bin/BLOODPRG.EXE"))
                .unwrap();
        let dictionary = decode_script_dictionary(b"YES\0").unwrap();
        let (word, text) = dictionary.words().next().unwrap();
        let mut metrics = RuntimeInlineMenuMetrics::new(&fonts);

        let rendered = metrics.rendered_width(word, text);
        let lookahead = metrics.lookahead_width(Some((word, text)));

        assert_ne!(rendered, u16::MIN);
        assert_ne!(lookahead, u16::MIN);
        metrics.finish().unwrap();
    }

    #[test]
    #[ignore = "requires an active desktop and serialized SDL/wgpu ownership"]
    fn real_services_run_the_complete_available_startup_slice() {
        let paths = OriginalGameDataPaths::discover(None)
            .expect("ignored real-services test requires the original game data");
        assert!(
            std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some(),
            "ignored real-services test requires an active desktop"
        );
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let audio = sdl.audio().unwrap();
        let window = video
            .window("Commander Blood service test", 640, 480)
            .position_centered()
            .metal_view()
            .build()
            .unwrap();
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut services = ModernGameServices::new(&window, data, TEST_SCRIPT_CLOCK).unwrap();

        let startup = services.prepare_startup_resources().unwrap();
        assert!(startup.write_directory_created);
        assert_eq!(
            services.load_manu3_overlay().unwrap(),
            RuntimeAssetLoadStatus::LoadedNow
        );
        services.initialize_logical_viewport().unwrap();
        assert_eq!(
            services.open_bridge_panorama().unwrap(),
            RuntimeAssetLoadStatus::LoadedNow
        );
        assert_eq!(
            services.load_save_slots().unwrap(),
            RuntimeAssetLoadStatus::LoadedNow
        );
        services.initialize_audio(&audio).unwrap();
        services.load_initial_cartography_resource().unwrap();
        services.initialize_bridge_scene(TEST_CLOCK_SEED).unwrap();
        assert_eq!(
            services.bridge_view_frame().unwrap(),
            crate::native::bloodprg::INITIAL_BRIDGE_VIEW_FRAME as i16
        );
        assert_eq!(
            services
                .nav_actor_slots
                .map(|slot| slot.line.resource.get()),
            AUTHORED_ACTOR_RESOURCES
        );
        assert_eq!(
            services.nav_actor_slots.map(|slot| {
                slot.line
                    .transition_resource
                    .map(PresentationResourceId::get)
            }),
            AUTHORED_ACTOR_TRANSITION_RESOURCES
        );
        assert_eq!(
            services.nav_actor_slots.map(|slot| slot.target_arc),
            AUTHORED_ACTOR_TARGET_ARCS
        );
        assert_eq!(
            services.nav_actor_slots.map(|slot| slot.line.position),
            AUTHORED_ACTOR_DRAW_POSITIONS
        );
        assert_eq!(
            services.nav_actor_slots.map(|slot| slot.hit_region),
            [
                None,
                None,
                None,
                None,
                Some(PresentationHitRectangle::new([18, 111], [96, 47])),
                Some(PresentationHitRectangle::new([215, 112], [75, 27])),
            ]
        );
        services.load_default_sound_bank().unwrap();
        services.initialize_back_buffer().unwrap();
        services
            .load_script_profile(ScriptProfileId::new(u8::MIN).unwrap())
            .unwrap();
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.vm_execution_enabled = true;
        lifecycle.set_presentation_interface_active(true);
        let script = services
            .execute_and_apply_lifecycle_script_frame(&mut lifecycle)
            .unwrap();
        assert_ne!(
            script.end,
            crate::native::bloodprg::ScriptFrameEnd::ExecutionDisabled
        );
        services.rebuild_script_record_state().unwrap();
        services.refresh_object_access_counters().unwrap();
        services.reset_ship_hud().unwrap();
        services
            .input_mut()
            .poll_pointer([320.0, 200.0], [160.0, 100.0], PointerButtons::NONE);
        assert!(services.update_lifecycle_manu3(&lifecycle).unwrap());
        let centered_hand = services.runtime().manu3().unwrap().projection_center();
        assert!(
            !services
                .runtime()
                .manu3()
                .unwrap()
                .render_triangles()
                .is_empty(),
            "the production MANU3 path produced no visible hand triangles"
        );
        services
            .input_mut()
            .poll_pointer([320.0, 200.0], [240.0, 100.0], PointerButtons::NONE);
        assert!(services.update_lifecycle_manu3(&lifecycle).unwrap());
        assert_ne!(
            services.runtime().manu3().unwrap().projection_center(),
            centered_hand,
            "the production hand did not follow logical pointer motion"
        );

        services.ship_presentation_state_mut().depth_offset = NONZERO_SHIP_DEPTH_OFFSET;
        services.initialize_bridge_screen(false, false).unwrap();
        assert_eq!(services.ship_presentation_state().depth_offset, u16::MIN);
        assert!(!services.bridge_screen_state().screen_rebuild_pending);
        assert!(!services.bridge_screen_state().palette_refresh_in_progress);
        assert!(services.bridge_screen_state().palette_dirty);
        assert!(services.bridge_screen_state().clip_snapshot_ready);
        let resident_bank = services.resident_sound_bank().unwrap().clone();
        services
            .script_backend_mut()
            .load_streamed_sound_bank(DEFAULT_BRIDGE_SOUND_BANK)
            .unwrap();
        let dictionary_word = services
            .runtime()
            .current_profile()
            .unwrap()
            .dictionary()
            .words()
            .next()
            .unwrap()
            .0;
        services.audio_events = AudioEventState {
            playback_enabled: true,
            menu_words_pending: false,
            dialogue_armed: false,
            voice_reaction_requested: false,
            voice_cooldown: u8::MIN,
            dialogue_delay: u16::MIN,
            dialogue_seed: u16::MIN,
            last_clip: u16::MIN,
        };
        {
            let text = services.scripts.text_presentation_mut();
            text.dialogue_chatter_seed_pending = true;
            text.dialogue_chatter_active = false;
            text.subtitle_voice_trigger = false;
            text.menu_words = Box::new([ScriptTextWord::Dictionary(dictionary_word)]);
        }
        assert!(
            services
                .process_runtime_audio_events(true)
                .unwrap()
                .is_empty()
        );
        assert!(services.audio_events.menu_words_pending);
        assert!(
            services
                .process_runtime_audio_events(false)
                .unwrap()
                .is_empty()
        );
        let dialogue_requests = services.process_runtime_audio_events(false).unwrap();
        assert!(matches!(
            dialogue_requests.as_ref(),
            [AudioClipRequest::StreamedDialogue { .. }]
        ));
        assert_eq!(services.resident_sound_bank().unwrap(), &resident_bank);
        let horn = services
            .runtime()
            .current_profile()
            .unwrap()
            .builtins()
            .horn
            .unwrap();
        let izwalito = services
            .runtime()
            .current_profile()
            .unwrap()
            .directory()
            .find_active_object(IZWALITO_NAME)
            .expect("SCRIPT1 must contain Izwalito");
        services
            .request_bridge_seek(PHONE_BRIDGE_TARGET_ARC)
            .unwrap();
        for _ in usize::MIN..MAXIMUM_CAMERA_TRANSITION_FRAMES {
            services
                .render_bridge_frame(BridgeSceneInput {
                    interaction: BridgeSteeringInteraction::MenuEngaged,
                    ..BridgeSceneInput::default()
                })
                .unwrap();
            if !services.bridge_seek_requested().unwrap() {
                break;
            }
        }
        assert!(!services.bridge_seek_requested().unwrap());
        assert_eq!(
            services.bridge_view_frame().unwrap(),
            PHONE_BRIDGE_VIEW_FRAME
        );
        let (station_index, station_orb, plotted_star_count) = {
            let bridge_frame = services
                .render_bridge_frame(BridgeSceneInput::default())
                .unwrap();
            (
                bridge_frame.metadata.station.index(),
                bridge_frame.metadata.orb_box,
                bridge_frame.starfield.plotted.len(),
            )
        };
        assert_eq!(
            services
                .update_runtime_camera_navigation(&mut lifecycle)
                .unwrap(),
            crate::native::bloodprg::CameraNavigationOutcome::UnsupportedLocation
        );
        assert_eq!(
            services
                .update_runtime_bridge_actors(&mut lifecycle)
                .unwrap(),
            NavActorSlotUpdateOutcome::Updated
        );
        let chart_candidates = services
            .runtime()
            .current_profile()
            .unwrap()
            .state()
            .objects()
            .iter()
            .filter(|object| {
                matches!(
                    object.kind,
                    ScriptObjectKind::CelestialBody
                        | ScriptObjectKind::NavigationEntity
                        | ScriptObjectKind::BlackHole
                )
            })
            .map(|object| object.id)
            .collect::<Vec<_>>();
        for (index, object) in chart_candidates.iter().enumerate() {
            assert!(crate::native::bloodprg::set_object_flag(
                services
                    .runtime_mut()
                    .current_profile_mut()
                    .unwrap()
                    .state_mut(),
                *object,
                crate::native::bloodprg::ScriptObjectFlag::InPlay,
                true,
            ));
            if index != usize::MIN {
                let state = services
                    .runtime_mut()
                    .current_profile_mut()
                    .unwrap()
                    .state_mut();
                let counter = state
                    .object_byte(*object, OBJECT_ACCESS_COUNTER_BYTE_OFFSET)
                    .expect("chart destination must contain its access counter");
                assert!(state.set_byte(counter, VISITED_DESTINATION_COUNT));
            }
        }
        crate::native::bloodprg::ScriptExecutionBackend::start_camera_transition(
            services.scripts.backend_mut(),
            crate::native::bloodprg::CAMERA_VIEW_TRANSITION_STEPS,
        )
        .unwrap();
        assert_eq!(services.process_script_commands().unwrap(), 1);
        assert_eq!(
            services.bridge_actor_camera_transition_step().unwrap(),
            crate::native::bloodprg::CAMERA_VIEW_TRANSITION_STEPS
        );
        for _ in usize::MIN..usize::from(crate::native::bloodprg::CAMERA_VIEW_TRANSITION_STEPS) {
            assert!(matches!(
                services
                    .update_runtime_navigation_chart(&mut lifecycle, u8::MIN)
                    .unwrap(),
                crate::native::bloodprg::NavigationCameraOutcome::TransitionFrame {
                    direction: crate::native::bloodprg::NavigationChartWipeDirection::Closing,
                    ..
                }
            ));
        }
        assert_eq!(
            services
                .navigation_chart
                .as_ref()
                .unwrap()
                .chart_object_count(),
            chart_candidates.len()
        );
        assert_eq!(
            services
                .update_runtime_navigation_chart(&mut lifecycle, u8::MIN)
                .unwrap(),
            crate::native::bloodprg::NavigationCameraOutcome::Inactive
        );
        let panel_marker = {
            let profile = services.runtime().current_profile().unwrap();
            let arche = profile.builtins().archetype.unwrap();
            let current = services.current_arche_navigation_target().unwrap().0;
            let target = crate::native::bloodprg::navigation_chart_objects(profile.state())
                .into_iter()
                .find(|object| {
                    *object != current
                        && profile
                            .state()
                            .object(*object)
                            .is_some_and(|record| record.kind == ScriptObjectKind::CelestialBody)
                        && profile.directory().object(*object).is_some_and(|entry| {
                            services
                                .runtime()
                                .data()
                                .world_artwork_layout()
                                .iter()
                                .any(|artwork| artwork.name() == entry.name())
                        })
                })
                .expect("profile must contain an authored planet panel");
            let marker = crate::native::bloodprg::resolve_navigation_position(
                profile.state(),
                target,
                arche,
                u16::MIN,
            )
            .unwrap();
            profile.state().word_pair(marker).unwrap()
        };
        services.set_bridge_camera_view_active(true);
        services.input_mut().poll_pointer(
            LOGICAL_TEST_OUTPUT_SIZE,
            panel_marker.map(f32::from),
            PointerButtons::from_bits(PointerButton::Primary as u16),
        );
        lifecycle.primary_pointer_pressed = true;
        lifecycle.pointer_press_pending = POINTER_PRESS_PENDING;
        assert_eq!(
            services
                .update_runtime_navigation_chart(&mut lifecycle, u8::MIN)
                .unwrap(),
            crate::native::bloodprg::NavigationCameraOutcome::LocationPanelOpened
        );
        services.input_mut().poll_pointer(
            LOGICAL_TEST_OUTPUT_SIZE,
            panel_marker.map(f32::from),
            PointerButtons::default(),
        );
        lifecycle.primary_pointer_pressed = false;
        lifecycle.pointer_press_pending = u8::MIN;
        for _ in usize::MIN..LOCATION_PANEL_OPENING_SETTLE_FRAMES {
            assert_eq!(
                services
                    .update_runtime_navigation_chart(&mut lifecycle, u8::MIN)
                    .unwrap(),
                crate::native::bloodprg::NavigationCameraOutcome::LocationPanel
            );
        }
        assert!(
            services
                .navigation_chart
                .as_ref()
                .unwrap()
                .location_panel_active()
        );
        lifecycle.primary_pointer_pressed = true;
        lifecycle.pointer_press_pending = POINTER_PRESS_PENDING;
        assert_eq!(
            services
                .update_runtime_navigation_chart(&mut lifecycle, u8::MIN)
                .unwrap(),
            crate::native::bloodprg::NavigationCameraOutcome::LocationPanel
        );
        lifecycle.primary_pointer_pressed = false;
        lifecycle.pointer_press_pending = u8::MIN;
        for _ in usize::MIN..LOCATION_PANEL_CLOSING_SETTLE_FRAMES {
            services
                .update_runtime_navigation_chart(&mut lifecycle, u8::MIN)
                .unwrap();
        }
        assert!(
            !services
                .navigation_chart
                .as_ref()
                .unwrap()
                .location_panel_active()
        );
        let closed_chart = services.runtime().front_buffer().pixels().to_vec();
        services
            .bridge_actors
            .as_mut()
            .unwrap()
            .set_camera_transition_step(crate::native::bloodprg::CAMERA_VIEW_TRANSITION_STEPS);
        for _ in usize::MIN..usize::from(crate::native::bloodprg::CAMERA_VIEW_TRANSITION_STEPS) {
            assert!(matches!(
                services
                    .update_runtime_navigation_chart(&mut lifecycle, u8::MIN)
                    .unwrap(),
                crate::native::bloodprg::NavigationCameraOutcome::TransitionFrame {
                    direction: crate::native::bloodprg::NavigationChartWipeDirection::Opening,
                    ..
                }
            ));
        }
        let opening_source = services.navigation_chart.as_ref().unwrap().work_surface();
        assert_ne!(opening_source, closed_chart);
        assert!(opening_source.iter().any(|pixel| *pixel != u8::MIN));
        services.set_bridge_camera_view_active(false);
        lifecycle.navigation_transition_pending = false;
        lifecycle.presentation.active = false;
        lifecycle.presentation.subtitle_word_list_mode = false;
        {
            let text = services.text_presentation_mut();
            text.subtitle_display_active = false;
            text.subtitle_word_list_mode = false;
        }
        let status_entity_index = crate::runtime::navigation_status::NAVIGATION_STATUS_ENTITY_INDEX;
        let original_status_entity =
            services.runtime().bridge_sprite_entities()[status_entity_index];
        {
            let status_entity =
                &mut services.runtime_mut().bridge_sprite_entities_mut()[status_entity_index];
            status_entity.flags =
                crate::native::bloodprg::BridgeSpriteFlags::from_bits(STATUS_TEST_ACTIVE_FLAGS);
            status_entity.draw_position = crate::native::bloodprg::BridgeSpritePosition {
                x: STATUS_TEST_ORIGIN[0],
                y: STATUS_TEST_ORIGIN[1],
            };
            status_entity.extent = crate::native::bloodprg::BridgeSpriteExtent {
                width: STATUS_TEST_EXTENT[0],
                height: STATUS_TEST_EXTENT[1],
            };
        }
        let status_entity = services.runtime().bridge_sprite_entities()[status_entity_index];
        services.input_mut().poll_pointer(
            TEST_OUTPUT_SIZE,
            [
                f32::from(status_entity.draw_position.x) * TEST_LOGICAL_TO_HOST_SCALE[0],
                f32::from(status_entity.draw_position.y) * TEST_LOGICAL_TO_HOST_SCALE[1],
            ],
            PointerButtons::default(),
        );
        let status_outcome = services
            .update_runtime_navigation_status(&mut lifecycle)
            .unwrap();
        assert!(
            matches!(
                status_outcome,
                crate::native::bloodprg::NavigationStatusOutcome::Composed { .. }
            ),
            "unexpected navigation status outcome: {status_outcome:?}"
        );
        let status_text = services.text_presentation().subtitle_text.clone();
        assert!(status_text.ends_with(b"\r\r"));
        assert!(services.text_presentation().subtitle_word_list_mode);
        assert!(lifecycle.presentation.subtitle_word_list_mode);

        let status_right = status_entity
            .draw_position
            .x
            .wrapping_add(status_entity.extent.width);
        let outside_x = if status_entity.draw_position.x != u16::MIN {
            status_entity.draw_position.x - 1
        } else {
            status_right.saturating_add(1)
        };
        services.input_mut().poll_pointer(
            TEST_OUTPUT_SIZE,
            [
                f32::from(outside_x) * TEST_LOGICAL_TO_HOST_SCALE[0],
                f32::from(status_entity.draw_position.y) * TEST_LOGICAL_TO_HOST_SCALE[1],
            ],
            PointerButtons::default(),
        );
        assert_eq!(
            services
                .update_runtime_navigation_status(&mut lifecycle)
                .unwrap(),
            crate::native::bloodprg::NavigationStatusOutcome::PointerOutside
        );
        assert_eq!(services.text_presentation().subtitle_text, status_text);
        assert!(!services.text_presentation().subtitle_word_list_mode);
        assert!(!lifecycle.presentation.subtitle_word_list_mode);
        services.runtime_mut().bridge_sprite_entities_mut()[status_entity_index] =
            original_status_entity;
        assert_ne!(plotted_star_count, usize::MIN);
        assert_eq!(
            services.bridge_presentation_mode(),
            Some(PresentationBridgeMode::FirstBand)
        );
        assert!(!services.presentation_hover().active());
        assert_eq!(
            services.nav_actor_slots[station_index].hit_region,
            station_orb.map(|orb_box| PresentationHitRectangle::new(
                orb_box.origin.map(|coordinate| coordinate as i16),
                orb_box.size.map(|extent| extent as i16),
            ))
        );
        let actor_hit_region = services.nav_actor_slots[station_index]
            .hit_region
            .expect("the initial bridge station must have an actor hit region");
        let phone_timer = ScriptTimerSlot::decode(STARTUP_PHONE_TIMER_SLOT).unwrap();
        assert_eq!(
            services
                .runtime()
                .current_profile()
                .unwrap()
                .runtime()
                .timer(phone_timer),
            STARTUP_PHONE_TIMER_VALUE
        );
        lifecycle.vm_execution_enabled = true;
        lifecycle.set_presentation_interface_active(true);
        let mut timer = GameTimerState::default();
        timer.start();
        for _ in usize::MIN..STARTUP_PHONE_FRAME_LIMIT {
            for _ in usize::MIN..TIMER_TICKS_PER_GAME_FRAME {
                advance_game_timer_tick(
                    &mut timer,
                    services
                        .runtime_mut()
                        .current_profile_mut()
                        .unwrap()
                        .runtime_mut(),
                    GameTimerContext::default(),
                );
            }
            services
                .execute_and_apply_lifecycle_script_frame(&mut lifecycle)
                .unwrap();
            if services.pending_ship_presentation_owner() == Some(izwalito) {
                break;
            }
        }
        assert_eq!(
            services.pending_ship_presentation_owner(),
            Some(izwalito),
            "SCRIPT1 never queued the startup Izwalito phone presentation"
        );
        let actor_pointer = services.input_mut().poll_pointer(
            TEST_OUTPUT_SIZE,
            [
                f32::from(actor_hit_region.origin()[0]) * TEST_LOGICAL_TO_HOST_SCALE[0],
                f32::from(actor_hit_region.origin()[1]) * TEST_LOGICAL_TO_HOST_SCALE[1],
            ],
            PointerButtons::from_bits(PointerButton::Primary as u16),
        );
        assert!(actor_hit_region.contains(actor_pointer.position));
        assert!(
            services.nav_actor_slots[station_index].flags.active,
            "station {station_index} has inactive actor flags {:?}",
            services.nav_actor_slots[station_index].flags
        );
        lifecycle.primary_pointer_pressed = true;
        assert_eq!(
            services
                .update_runtime_bridge_actors(&mut lifecycle)
                .unwrap(),
            NavActorSlotUpdateOutcome::Updated
        );
        assert!(services.nav_actor_slots[station_index].flags.auto_seek);
        if services.bridge_seek_requested().unwrap() {
            services
                .render_bridge_frame(BridgeSceneInput {
                    interaction: BridgeSteeringInteraction::MenuEngaged,
                    ..BridgeSceneInput::default()
                })
                .unwrap();
            assert!(!services.bridge_seek_requested().unwrap());
            assert_eq!(
                services
                    .update_runtime_bridge_actors(&mut lifecycle)
                    .unwrap(),
                NavActorSlotUpdateOutcome::Updated
            );
        }
        assert!(
            services.nav_actor_slots[station_index]
                .flags
                .clear_mouse_before_hit
        );
        assert_eq!(
            services.nav_actor_slots[station_index].line.terminal_frame,
            AUTHORED_RADIO_TERMINAL_FRAME
        );
        assert_eq!(
            services.nav_actor_slots[station_index].line.resource.get(),
            AUTHORED_ACTOR_RESOURCES[station_index]
        );
        assert_eq!(
            services.nav_actor_slots[station_index].line.position,
            AUTHORED_ACTOR_DRAW_POSITIONS[station_index]
        );
        assert_eq!(
            services.nav_actor_slots[station_index].line.frame,
            FIRST_ADVANCED_PRESENTATION_FRAME
        );
        assert!(
            lifecycle.modal_ui_busy(),
            "answering the radio must publish native UI bit 2 in the same actor frame (screen redraw: {})",
            services
                .presentation_screen_state()
                .unwrap()
                .redraw_requested()
        );
        assert_eq!(
            services.manu3_hand_state().requested_animation,
            Manu3AnimationSelector::RadioOrb.value(),
            "answering the radio must preserve the DS:0x0A32 hand-animation alias"
        );
        assert!(services.update_lifecycle_manu3(&lifecycle).unwrap());
        assert_eq!(
            services.manu3_hand_state().current_animation,
            Manu3AnimationSelector::RadioOrb.value()
        );
        let word_delay = services.dialogue_word_delay().unwrap();
        services
            .reveal_lifecycle_inline_menu(&mut lifecycle, word_delay)
            .unwrap();
        assert!(
            lifecycle.modal_ui_busy(),
            "late inline-menu export cleared native UI bit 2 while the radio actor owned it"
        );
        services.update_lifecycle_subtitles(&mut lifecycle).unwrap();
        assert!(
            lifecycle.modal_ui_busy(),
            "late subtitle export cleared native UI bit 2 while the radio actor owned it"
        );
        for _ in usize::MIN..RADIO_COMPLETION_FRAME_LIMIT {
            services
                .execute_and_apply_lifecycle_script_frame(&mut lifecycle)
                .unwrap();
            assert!(
                lifecycle.modal_ui_busy(),
                "the interleaved BloodScript pass cleared native UI bit 2 while the radio actor still owned its line"
            );
            services
                .update_runtime_bridge_actors(&mut lifecycle)
                .unwrap();
            if services.pending_ship_presentation_owner().is_none() {
                break;
            }
        }
        assert!(services.pending_ship_presentation_owner().is_none());
        assert_eq!(
            services.presentation_scan_state().deferred,
            crate::native::bloodprg::ScriptDeferredRecord::Complete {
                record: crate::native::bloodprg::ScriptActionRecord::ActorPresentation(izwalito),
                actionable: true,
            }
        );
        let mut phone_lifecycle = lifecycle.clone();
        phone_lifecycle.vm_execution_enabled = true;
        phone_lifecycle.set_presentation_interface_active(true);
        let active_line_before_phone = phone_lifecycle.presentation.active_line;
        let bridge_before_phone = services
            .bridge_frame
            .as_ref()
            .unwrap()
            .actor_sprite_pixels
            .clone();
        services
            .execute_and_apply_lifecycle_script_frame(&mut phone_lifecycle)
            .unwrap();
        services
            .execute_and_apply_lifecycle_script_frame(&mut phone_lifecycle)
            .unwrap();
        let mut phone_scene_link = GameSceneLink::Initial;
        update_game_presentation_ownership(&mut phone_lifecycle, &mut phone_scene_link);
        assert_eq!(
            services.script_backend().active_description_object(),
            Some(izwalito)
        );
        assert_eq!(
            services
                .runtime()
                .current_profile()
                .unwrap()
                .active_actor_presentation_related(),
            Some(izwalito)
        );
        assert_eq!(
            services
                .presentation_catalog()
                .resource_name(IZWALITO_IDLE_PRESENTATION_LINE)
                .expect("Izwalito DESCRIPT must select an idle video")
                .as_bytes(),
            IZWALITO_IDLE_VIDEO
        );
        assert_eq!(
            services
                .script_backend()
                .assets()
                .character_sprite()
                .expect("Izwalito DESCRIPT must select its portrait sprite")
                .as_bytes(),
            IZWALITO_SPRITE
        );
        assert!(
            services
                .script_backend()
                .assets()
                .encoded_idle_video()
                .is_none(),
            "text-only bridge dialogue must not load the idle HNM"
        );
        assert_eq!(
            phone_lifecycle.presentation.active_line, active_line_before_phone,
            "text-only bridge dialogue must preserve the existing presentation line"
        );
        assert!(phone_lifecycle.presentation.active);
        assert!(!phone_lifecycle.presentation.scene_gate_active);
        assert!(!phone_lifecycle.presentation.ship_active);
        assert_eq!(
            services.ship_presentation_state().flags & SHIP_PRESENTATION_ACTIVE_FLAG,
            u16::MIN
        );
        assert!(!services.presentation_stream_active());
        assert!(!services.presentation_screen_state().unwrap().active());
        let portrait = services.runtime().bridge_sprite_entities()[NAME_AREA_EFFECT_ENTITY_INDEX];
        assert_eq!(
            portrait.draw_position,
            crate::native::bloodprg::BridgeSpritePosition {
                x: NAME_AREA_EFFECT_POSITION[0],
                y: NAME_AREA_EFFECT_POSITION[1],
            }
        );
        assert!(matches!(
            portrait.frame.map(|frame| frame.source),
            Some(BridgeSpriteFrameSource::CachedResource {
                resource: NAME_AREA_EFFECT_RESOURCE,
                ..
            })
        ));
        services
            .commit_ship_entities(FIRST_BRIDGE_ENTITY..AFTER_LAST_BRIDGE_ENTITY)
            .unwrap();
        services
            .render_bridge_frame(BridgeSceneInput {
                interaction: BridgeSteeringInteraction::MenuEngaged,
                ..BridgeSceneInput::default()
            })
            .unwrap();
        services
            .rasterize_bridge_frame_sprite_range(FIRST_ACTOR_ENTITY..AFTER_LAST_ACTOR_ENTITY)
            .unwrap();
        let phone_portrait_before_effect = services
            .bridge_frame
            .as_ref()
            .unwrap()
            .actor_sprite_pixels
            .clone();
        assert!(matches!(
            services.advance_bridge_name_area_effect().unwrap(),
            NameAreaEffectOutcome::Rendered { .. }
        ));
        assert!(
            services
                .bridge_frame
                .as_ref()
                .unwrap()
                .actor_sprite_pixels
                .iter()
                .zip(&phone_portrait_before_effect)
                .any(|(after, before)| after != before),
            "Izwalito portrait did not advance the native name-area animation"
        );
        assert!(
            services
                .bridge_frame
                .as_ref()
                .unwrap()
                .actor_sprite_pixels
                .iter()
                .zip(&bridge_before_phone)
                .any(|(after, before)| after != before),
            "Izwalito portrait did not update the bridge sprite layer"
        );
        assert_eq!(
            select_bridge_composition(
                services.presentation_stream_active(),
                services.presentation_screen_state().unwrap().active(),
                phone_lifecycle.presentation.active,
                false,
            ),
            RuntimeBridgeComposition::BridgeSceneWithIndexedOverlay,
            "text-only dialogue must overlay indexed subtitles on the bridge portrait"
        );
        assert_eq!(
            services
                .cancel_lifecycle_presentation(&mut phone_lifecycle)
                .unwrap(),
            InputCancellationOutcome::ForwardedToText,
            "the native Escape handler reserves character lines 8 through 40 for dialogue input"
        );
        assert!(!services.presentation_stream_active());
        services.finish_bridge_actor_scene_presentation(&mut phone_lifecycle);
        services.clear_pending_ship_presentation_owner();
        lifecycle.set_modal_ui_busy(false);
        services.input_mut().poll_pointer(
            [320.0, 200.0],
            [200.0, 80.0],
            PointerButtons::from_bits(PointerButton::Primary as u16),
        );
        lifecycle.primary_pointer_pressed = true;
        services
            .update_runtime_bridge_console(&mut lifecycle)
            .unwrap();
        assert!(services.bridge_seek_requested().unwrap());
        services
            .render_bridge_frame(BridgeSceneInput {
                pointer_buttons: PointerButton::Primary as u16,
                interaction: BridgeSteeringInteraction::MenuEngaged,
                ..BridgeSceneInput::default()
            })
            .unwrap();
        services
            .update_runtime_bridge_console(&mut lifecycle)
            .unwrap();
        assert!(!services.bridge_seek_requested().unwrap());
        assert!(!lifecycle.profile_change_blockers.navigation_choice_active);
        assert_eq!(
            services.presentation_scan_state().deferred,
            crate::native::bloodprg::ScriptDeferredRecord::Complete {
                record: crate::native::bloodprg::ScriptActionRecord::PresentationQueue(horn),
                actionable: true,
            }
        );
        services.submit_indexed_frame().unwrap();
        services.present_current_bridge_frame(false).unwrap();

        assert_eq!(services.presented_frame_count(), 2);
        services.runtime_mut().start_camera_transition();
        let mut hyperspace_queued = false;
        let mut camera_transition_completed = false;
        for _ in usize::MIN..MAXIMUM_CAMERA_TRANSITION_FRAMES {
            let outcome = update_runtime_camera_approach(
                &mut services,
                GameSceneLink::Initial,
                &mut lifecycle,
            )
            .unwrap();
            services
                .render_bridge_frame(BridgeSceneInput {
                    interaction: BridgeSteeringInteraction::MenuEngaged,
                    ..BridgeSceneInput::default()
                })
                .unwrap();
            match outcome {
                Some(crate::native::bloodprg::CameraApproachOutcome::HyperspaceQueued) => {
                    hyperspace_queued = true;
                }
                Some(crate::native::bloodprg::CameraApproachOutcome::TransitionCompleted) => {
                    camera_transition_completed = true;
                    break;
                }
                _ => {}
            }
        }
        assert!(hyperspace_queued);
        assert!(camera_transition_completed);
        assert_eq!(
            services
                .presentation_catalog()
                .resource_name(HYPERSPACE_PRESENTATION_LINE)
                .unwrap()
                .as_bytes(),
            b"SQ\\hyper_00.hnm"
        );
        assert!(!lifecycle.modal_ui_busy());
        assert!(
            !lifecycle
                .profile_change_blockers
                .navigation_actor_transition_active
        );
        services.set_presentation_screen_active(true).unwrap();
        assert_eq!(
            services.presentation_screen_state().unwrap().phase(),
            crate::native::bloodprg::PresentationPanelPhase::Begin
        );
        services
            .update_runtime_bridge_console(&mut lifecycle)
            .unwrap();
        assert_eq!(
            services.presentation_screen_state().unwrap().phase(),
            crate::native::bloodprg::PresentationPanelPhase::Begin
        );
        assert_eq!(
            services
                .update_presentation_screen(&GameSceneLink::Initial, false)
                .unwrap(),
            PresentationScreenOutcome::Initialized
        );
        assert_eq!(
            services.presentation_screen_state().unwrap().phase(),
            crate::native::bloodprg::PresentationPanelPhase::Opening(
                crate::native::bloodprg::PresentationPanelStep::One
            )
        );
        services
            .update_runtime_presentation_choice(&mut lifecycle)
            .unwrap();
        assert_eq!(
            services.presentation_screen_state().unwrap().phase(),
            crate::native::bloodprg::PresentationPanelPhase::Opening(
                crate::native::bloodprg::PresentationPanelStep::One
            )
        );
        assert!(
            services
                .runtime()
                .front_buffer()
                .pixels()
                .iter()
                .any(|pixel| *pixel != u8::MIN)
        );
        services.set_presentation_screen_active(false).unwrap();
        lifecycle.set_presentation_interface_active(true);
        lifecycle.frame_presented = true;
        lifecycle.presentation.c2_presentation_gate = false;
        assert_eq!(
            crate::runtime::bridge_frame::run_runtime_bridge_frame(
                &mut services,
                &mut lifecycle,
                GameSceneLink::Initial,
                BridgeSceneInput::default(),
                u8::MIN,
            )
            .unwrap(),
            crate::native::bloodprg::BridgeFrameOutcome::Presented
        );
        assert!(
            services
                .bridge_frame
                .as_ref()
                .unwrap()
                .panorama_pixels
                .iter()
                .any(|pixel| *pixel != u8::MIN)
        );
        exercise_script_action_effect_bridge(&mut services);
        exercise_pterra_hud_transition(&mut services);
        exercise_streamed_credits_voice(&mut services);
        assert!(services.close_bridge_scene());
    }

    fn exercise_script_action_effect_bridge(services: &mut ModernGameServices<'_>) {
        let original_action = services.scripts.action_state().clone();
        let original_ship = services.ship_presentation;
        {
            let action = services.scripts.action_state_mut();
            action.ship_navigation_mode = ScriptShipNavigationMode::Active;
            action.travel_phase = ScriptTravelActionPhase::WaitingForCamera;
            action.ship_hud_refresh_requested = true;
            action.screen_rebuild_requested = true;
            action.clip_playback_state_reload = Some(SCRIPT_RADIO_CLIP_COUNTDOWN);
            action.speaker_pulse_requested = true;
        }
        let mut lifecycle = GameLifecycleState::default();

        services.synchronize_script_action_effects(Some(&mut lifecycle));

        assert_eq!(services.ship_presentation.hud_initialization_pending, 1);
        assert_eq!(
            services.ship_presentation.active_line,
            original_ship.active_line
        );
        assert_eq!(lifecycle.presentation.active_line, None);
        assert!(lifecycle.navigation_rebuild_pending);
        assert_eq!(lifecycle.clip_playback_state, SCRIPT_RADIO_CLIP_COUNTDOWN);
        assert!(lifecycle.speaker_pulse_requested);
        assert!(
            lifecycle
                .profile_change_blockers
                .navigation_actor_transition_active
        );

        services.ship_presentation.hud_initialization_pending = u8::MIN;
        services.ship_presentation.active_line = u16::MIN;
        lifecycle.presentation.active_line = None;
        lifecycle.navigation_rebuild_pending = false;
        lifecycle.clip_playback_state = u16::MIN;
        lifecycle.speaker_pulse_requested = false;
        services.scripts.action_state_mut().travel_phase =
            ScriptTravelActionPhase::WaitingForPresentation;
        services.synchronize_script_action_effects(Some(&mut lifecycle));

        assert_eq!(
            services.ship_presentation.hud_initialization_pending,
            u8::MIN
        );
        assert_eq!(services.ship_presentation.active_line, u16::MIN);
        assert_eq!(lifecycle.presentation.active_line, None);
        assert!(!lifecycle.navigation_rebuild_pending);
        assert_eq!(lifecycle.clip_playback_state, u16::MIN);
        assert!(!lifecycle.speaker_pulse_requested);
        assert!(
            lifecycle
                .profile_change_blockers
                .navigation_actor_transition_active
        );
        services.scripts.action_state_mut().travel_phase = ScriptTravelActionPhase::WaitingForActor;
        services.synchronize_script_action_effects(Some(&mut lifecycle));
        assert!(
            !lifecycle
                .profile_change_blockers
                .navigation_actor_transition_active
        );
        *services.scripts.action_state_mut() = original_action;
        services.ship_presentation = original_ship;
    }

    fn exercise_pterra_hud_transition(services: &mut ModernGameServices<'_>) {
        services
            .load_script_profile(ScriptProfileId::new(ALTERNATE_SCRIPT_PROFILE_VALUE).unwrap())
            .unwrap();
        services
            .load_script_profile(ScriptProfileId::INITIAL)
            .unwrap();
        services.ship_presentation = ShipPresentationState {
            flags: SHIP_NAVIGATION_ACTIVE_FLAGS,
            ..ShipPresentationState::default()
        };
        let (arche, ark, pterra) = {
            let profile = services.runtime().current_profile().unwrap();
            (
                profile.builtins().archetype.unwrap(),
                profile.builtins().ark.unwrap(),
                profile.directory().find_active_object(PTERRA_NAME).unwrap(),
            )
        };
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.vm_execution_enabled = true;
        services.runtime_mut().start_camera_transition();
        for _ in usize::MIN..MAXIMUM_CAMERA_TRANSITION_FRAMES {
            update_runtime_camera_approach(services, GameSceneLink::Initial, &mut lifecycle)
                .unwrap();
            if services.runtime().camera_approach().phase >= 4 {
                break;
            }
        }
        assert_eq!(services.runtime().camera_approach().phase, 4);
        services.defer_ship_navigation_target(pterra);
        services
            .execute_and_apply_lifecycle_script_frame(&mut lifecycle)
            .unwrap();
        services
            .execute_and_apply_lifecycle_script_frame(&mut lifecycle)
            .unwrap();
        assert_eq!(services.current_ship_navigation_target().unwrap(), pterra);
        assert_eq!(
            ship_hud_arche_link(services.runtime().current_profile().unwrap().state(), arche,)
                .unwrap()
                .0,
            pterra
        );
        assert_eq!(
            services
                .update_runtime_ship_hud(GameSceneLink::Initial, &mut lifecycle)
                .unwrap(),
            crate::native::bloodprg::ShipHudCoordinatorOutcome::TextInactive
        );
        let loaded_scene_top_row = services
            .scripts
            .backend()
            .assets()
            .location_scene_top_row()
            .unwrap();
        let hud_scene_state = services.runtime_ship_hud().unwrap().coordinator().unwrap();
        assert_eq!(
            hud_scene_state.resource_vertical_offset,
            loaded_scene_top_row
        );
        assert_eq!(
            services.ship_navigation_scene_vertical_offset(),
            loaded_scene_top_row
        );
        assert_eq!(
            lifecycle.presentation.active_line,
            hud_scene_state.active_line
        );
        assert_eq!(
            lifecycle.presentation.c2_presentation_gate,
            hud_scene_state.presentation_gate & SHIP_PRESENTATION_ACTIVE_FLAG != u16::MIN
        );
        assert_eq!(lifecycle.frame_presented, hud_scene_state.frame_presented);
        let targets = services
            .runtime_ship_hud()
            .unwrap()
            .coordinator()
            .unwrap()
            .presentable_targets
            .clone();
        let target_names = targets
            .iter()
            .map(|target| {
                services
                    .runtime()
                    .current_profile()
                    .unwrap()
                    .directory()
                    .object(*target)
                    .unwrap()
                    .name()
                    .to_vec()
            })
            .collect::<Vec<_>>();
        assert_eq!(target_names, [ARK_NAME.to_vec()]);
        assert_eq!(
            services
                .runtime_ship_hud()
                .unwrap()
                .coordinator()
                .unwrap()
                .current_target,
            pterra
        );
        let ark_index = targets.iter().position(|target| *target == ark).unwrap();

        let background_slot = DescriptBackgroundSlot::decode(NAVIGATION_BACKGROUND_SLOT).unwrap();
        let background = services
            .scripts
            .backend()
            .backgrounds()
            .get(background_slot)
            .unwrap();
        assert_eq!(background.source_name(), PTERRA_BACKGROUND_NAME);
        assert!(!background.encoded_image().is_empty());

        {
            let text = services.text_presentation_mut();
            text.subtitle_display_active = true;
            text.subtitle_reveal_cursor = Some(text.subtitle_text.len());
        }
        lifecycle.presentation.subtitle_display_active = true;
        for _ in usize::MIN..TARGET_SELECTOR_SETTLE_FRAME_LIMIT {
            services
                .update_runtime_ship_hud(GameSceneLink::Initial, &mut lifecycle)
                .unwrap();
        }
        let ark_row = services
            .ship_target_selector
            .as_ref()
            .unwrap()
            .last_frame()
            .unwrap()
            .rows
            .iter()
            .find_map(|row| {
                matches!(row.kind, ChoiceListRowKind::Item(index) if index == ark_index)
                    .then_some(row.position)
            })
            .unwrap();
        services.input_mut().poll_pointer(
            LOGICAL_TEST_OUTPUT_SIZE,
            ark_row.map(f32::from),
            PointerButtons::from_bits(PointerButton::Primary as u16),
        );
        assert_eq!(
            services
                .update_runtime_ship_hud(GameSceneLink::Initial, &mut lifecycle)
                .unwrap(),
            crate::native::bloodprg::ShipHudCoordinatorOutcome::TargetQueued
        );
        assert_eq!(
            services.presentation_scan_state().deferred,
            ScriptDeferredRecord::Empty
        );

        services
            .execute_and_apply_lifecycle_script_frame(&mut lifecycle)
            .unwrap();
        assert_eq!(services.current_ship_navigation_target().unwrap(), ark);
        assert_eq!(
            services.scripts.action_state().ship_navigation_mode,
            ScriptShipNavigationMode::Active
        );
        assert!(
            services
                .runtime()
                .back_buffer()
                .pixels()
                .iter()
                .any(|pixel| *pixel != u8::MIN)
        );
    }

    fn exercise_streamed_credits_voice(services: &mut ModernGameServices<'_>) {
        services.stop_digital_audio().unwrap();
        services
            .load_streamed_voice_resource(CREDITS_VOICE_RESOURCE_PATH.as_bytes())
            .unwrap();
        services.start_loaded_streamed_voice().unwrap();
        assert!(
            services
                .audio_ref()
                .unwrap()
                .background_stream_remaining()
                .is_some()
        );
        services.refill_navigation_music().unwrap();
        services.stop_digital_audio().unwrap();
    }
}
