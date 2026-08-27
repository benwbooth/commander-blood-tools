//! Concrete runtime services assembled for the recovered top-level lifecycle.

use std::ops::Range;

use anyhow::{Context, Result, bail};
use commander_blood_formats::alien::AlienAsset;
use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::bloodprg::{BloodprgFontResources, decode_bloodprg_bridge_resources};
use commander_blood_formats::descript::DescriptBackgroundSlot;
use commander_blood_formats::instruction::ScriptTextWord;
use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};
use commander_blood_formats::script::{ScriptObjectId, ScriptWordId};
use commander_blood_formats::snd::VocPcm;
use sdl3::AudioSubsystem;
use sdl3::video::Window;

use crate::native::alien::AlienSceneFrame;
use crate::native::bloodprg::{
    AudioClipRequest, AudioEventContext, AudioEventState, BridgeScene, BridgeSceneFrame,
    BridgeSceneInput, BridgeSpriteCommitOutcome, BridgeSteeringInteraction,
    CdAudioPreparationOutcome, CdAudioState, ChoiceListConfig, ChoiceListFrame, ChoiceListPointer,
    ChoiceListState, ConfirmDialogOutcome, ConfirmDialogState, DescriptMusicSelectionOutcome,
    DescriptRecordApplication, DirtyRegionCopyOutcome, FontPoint, FontVerticalBand, GameFontFace,
    GameLifecycleState, GamePresentationOwner, GameSceneLink, IndexedGamePalette,
    InlineMenuRevealOutcome, InlineMenuTextMetrics, InputAction, LoadedSoundBank,
    Manu3HandFrameContext, Manu3HandFrameState, NAV_ACTOR_SLOT_COUNT, NavActorSlot,
    OriginalSaveGame, PbmDecodeResult, PointerButtonEdges, PointerButtons, PointerSample,
    PresentationChoiceNumber, PresentationPresentPolicy, PresentationResourceId,
    PresentationResourceSequenceOutcome, PresentationSceneDispatchOutcome,
    PresentationScreenOutcome, PresentationScreenState, PresentationWordChoiceOutcome,
    SCENE_PALETTE_CLEAR_COLOR_COUNT, SHIP_CAMERA_RESET, SceneTransitionState, ScriptClock,
    ScriptFrameOutcome, ScriptPresentationScanState, ScriptProfileId, ScriptProfileLoadOutcome,
    ScriptShipNavigationMode, ShipDepthTransitionOutcome, ShipHudInitializationContext,
    ShipPresentationOutcome, ShipPresentationState, ShipProjectionResources,
    ShipTargetSelectionState, ShipViewEntityId, SoundBankUsage, StartupPreparationOutcome,
    TextPresentationState, clear_scene_palette_entries, deactivate_nav_actor_slots,
    draw_planar_dialogue_text, fill_display_band, increment_object_access_counters,
    load_sound_bank, measure_game_text_width, objects_at_arche_position,
    original_save_state_block_byte_count, play_cd_audio_track_two, prepare_cd_audio,
    presentable_navigation_objects, process_audio_events, reveal_inline_menu_step, stop_cd_audio,
    update_manu3_hand_frame,
};
use crate::native::manu3::animation::CursorPosition;
use crate::native::random::BloodPrng;

use super::bridge_console::RuntimeBridgeConsole;
use super::choice_list::{
    RuntimeChoiceListStyle, draw_choice_list_rows, prepare_choice_list_frame,
};
use super::presentation_screen::RuntimeSceneTransitionDispatchContext;
use super::ship_presentation::update_runtime_ship_presentation as run_runtime_ship_presentation;
use super::ship_target::ship_hud_arche_link;
use super::{
    LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_PIXEL_COUNT, OriginalGameData,
    OriginalGameRuntime, RuntimeAlienOverlayCycle, RuntimeAssetLoadStatus, RuntimeAudioHost,
    RuntimeConfirmDialog, RuntimeInputHost, RuntimePaletteTransition,
    RuntimePaletteTransitionConfig, RuntimePaletteTransitionOutcome, RuntimePcmClip,
    RuntimePlatformHost, RuntimePresentationCatalog, RuntimePresentationHost,
    RuntimePresentationPlayer, RuntimePresentationScreen, RuntimePresentationStepOutcome,
    RuntimePresentationWordChoice, RuntimeSaveLoad, RuntimeSceneTransition, RuntimeScriptBackend,
    RuntimeScriptCommand, RuntimeScriptSystem, RuntimeShipHud, RuntimeShipNavigation,
    RuntimeShipTargetSelection, RuntimeShipTargetSelector, RuntimeSubtitleReveal,
    VGA_BIOS_FONT_8X8,
};

const INITIAL_LOGICAL_POINTER: [i16; 2] = [160, 100];
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
const SHIP_NAVIGATION_STATUS_LINE: u16 = 3;
const NAVIGATION_PALETTE_TRANSITION_INCREMENT: u16 = 10;
const PRESENTATION_CHOICE_MANU3_ANIMATION: u16 = 14;

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
    loaded_music: Option<RuntimePcmClip>,
    loaded_voice: Option<RuntimePcmClip>,
    bridge_scene: Option<BridgeScene>,
    bridge_frame: Option<BridgeSceneFrame>,
    nav_actor_slots: [NavActorSlot; NAV_ACTOR_SLOT_COUNT],
    bridge_console: Option<RuntimeBridgeConsole>,
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
    ship_presentation: ShipPresentationState,
    random: BloodPrng,
    scripts: RuntimeScriptSystem,
    cd_audio: CdAudioState,
    main_viewport_configured: bool,
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
        let runtime = OriginalGameRuntime::new(data);
        let bridge_palette = *runtime.live_palette();
        let presentation_screen = RuntimePresentationScreen::new(*runtime.live_palette())?;
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
            loaded_music: None,
            loaded_voice: None,
            bridge_scene: None,
            bridge_frame: None,
            nav_actor_slots: [NavActorSlot::default(); NAV_ACTOR_SLOT_COUNT],
            bridge_console: Some(bridge_console),
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
            ship_presentation: ShipPresentationState::default(),
            random: BloodPrng::default(),
            scripts,
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
        self.bridge_palette = *self.runtime.live_palette();
        self.runtime
            .rebuild_bridge_sprite_remap_tables()
            .context("building bridge sprite remap tables")?;
        let resources = decode_bloodprg_bridge_resources(self.runtime.data().executable())
            .context("decoding bridge projection resources")?;
        let panorama = self
            .runtime
            .take_bridge_panorama()
            .context("bridge panorama must be opened before scene initialization")?;
        self.random.seed_from_clock_register(packed_clock_seed);
        self.bridge_scene = Some(
            BridgeScene::new(
                panorama,
                ShipProjectionResources::from(resources),
                &mut self.random,
            )
            .context("constructing live bridge scene")?,
        );
        Ok(())
    }

    /// Rebuild the concrete bridge resources touched by camera travel setup.
    pub(super) fn initialize_camera_transition_screen(&mut self) -> Result<()> {
        let Self {
            runtime,
            bridge_palette,
            nav_actor_slots,
            ..
        } = self;
        *runtime.live_palette_mut() = *bridge_palette;
        runtime
            .rebuild_bridge_sprite_remap_tables()
            .context("rebuilding camera-transition sprite remaps")?;
        runtime
            .activate_retained_bridge_background()
            .context("activating the camera-transition bridge background")?;
        deactivate_nav_actor_slots(nav_actor_slots);
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
        let music_name = self
            .scripts
            .backend()
            .assets()
            .music()
            .context("no navigation music is selected")?
            .as_bytes();
        let resource_name = prefixed_resource_name(MUSIC_RESOURCE_DIRECTORY, music_name)?;
        let encoded = self
            .runtime
            .data()
            .resource_store()
            .load(&resource_name)
            .with_context(|| {
                format!(
                    "loading music resource {}",
                    String::from_utf8_lossy(resource_name.as_bytes())
                )
            })?;
        let decoded = VocPcm::decode(&encoded).with_context(|| {
            format!(
                "decoding music resource {}",
                String::from_utf8_lossy(resource_name.as_bytes())
            )
        })?;
        self.loaded_music = Some(RuntimePcmClip::from_voc(&decoded));
        Ok(())
    }

    /// Start the retained navigation music as a looping background source.
    pub fn start_loaded_navigation_music(&mut self) -> Result<()> {
        let clip = self
            .loaded_music
            .take()
            .context("no decoded navigation music is waiting to start")?;
        self.audio_mut()?.play_background(clip)
    }

    /// Decode and start the navigation music selected by the active DESCRIPT record.
    pub fn restart_navigation_music(&mut self) -> Result<()> {
        self.load_navigation_music()?;
        self.start_loaded_navigation_music()
    }

    /// Stop only the looping navigation source before replacing its resource.
    pub fn stop_navigation_music(&mut self) -> Result<()> {
        self.audio_mut()?.stop_background()
    }

    /// Start retained music, or keep the current navigation stream running.
    pub fn ensure_navigation_music(&mut self) -> Result<()> {
        if self.navigation_music_position()?.is_some() {
            return self.check_audio();
        }
        if self.loaded_music.is_none() {
            self.load_navigation_music()?;
        }
        self.start_loaded_navigation_music()
    }

    /// Decode and play one authored clip from the currently loaded SND bank.
    pub fn play_loaded_sound_bank_clip(&mut self, clip_index: u8) -> Result<()> {
        self.play_resident_sound_bank_clip(u16::from(clip_index))
    }

    fn play_resident_sound_bank_clip(&mut self, clip_index: u16) -> Result<()> {
        let clip = self
            .resident_sound_bank()
            .context("playing a resident sound effect")?
            .bank
            .clip(usize::from(clip_index))
            .with_context(|| format!("resident sound bank clip {clip_index} is not authored"))?;
        let clip = RuntimePcmClip::from_snd_clip(clip)?;
        self.audio_mut()?.play_foreground(clip)
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

    /// Decode and retain one authored Creative Voice resource for a later start call.
    pub fn load_voice_resource(&mut self, path: &[u8]) -> Result<()> {
        let resource_name =
            BloodResourceName::new(path).context("validating voice resource path")?;
        let encoded = self
            .runtime
            .data()
            .resource_store()
            .load(&resource_name)
            .with_context(|| {
                format!(
                    "loading voice resource {}",
                    String::from_utf8_lossy(resource_name.as_bytes())
                )
            })?;
        let decoded = VocPcm::decode(&encoded).with_context(|| {
            format!(
                "decoding voice resource {}",
                String::from_utf8_lossy(resource_name.as_bytes())
            )
        })?;
        self.loaded_voice = Some(RuntimePcmClip::from_voc(&decoded));
        Ok(())
    }

    /// Start the previously decoded voice clip over any active background music.
    pub fn start_loaded_voice(&mut self) -> Result<()> {
        let clip = self
            .loaded_voice
            .take()
            .context("no decoded voice resource is waiting to start")?;
        self.audio_mut()?.play_foreground(clip)
    }

    /// Replace the complete live indexed palette with black.
    pub fn clear_live_palette(&mut self) {
        self.runtime.live_palette_mut().fill([u8::MIN; 3]);
    }

    /// Stop all modern audio and clear samples already queued in SDL.
    pub fn stop_audio(&mut self) -> Result<()> {
        self.audio_mut()?.stop_all()
    }

    /// Release a decoded voice clip that was loaded but never started.
    pub fn discard_loaded_voice(&mut self) -> bool {
        self.loaded_voice.take().is_some()
    }

    /// Release decoded navigation music that has not yet entered SDL playback.
    pub fn discard_loaded_music(&mut self) -> bool {
        self.loaded_music.take().is_some()
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
    pub fn process_runtime_audio_events(&mut self) -> Result<Box<[AudioClipRequest]>> {
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
            process_audio_events(
                audio_events,
                AudioEventContext {
                    dialogue_suppressed: false,
                    menu_words: &menu_words,
                    streamed_dialogue_clip_count: clip_count,
                    dialogue_delay_base: delay_base,
                    dialogue_delay_limit: delay_limit,
                },
                |upper_bound| random.next(upper_bound),
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
        let clip = self
            .scripts
            .backend()
            .streamed_sound_bank()
            .context("no streamed DESCRIPT sound bank is loaded")?
            .bank
            .clip(usize::from(clip_index))
            .with_context(|| format!("streamed dialogue clip {clip_index} is not authored"))?;
        let clip = RuntimePcmClip::from_snd_clip(clip)?;
        self.audio_mut()?.play_foreground(clip)
    }

    /// Current source-sample position of navigation music, when active.
    pub fn navigation_music_position(&self) -> Result<Option<u64>> {
        Ok(self.audio_ref()?.background_position())
    }

    /// Report whether SDL audio has completed startup initialization.
    pub const fn audio_is_initialized(&self) -> bool {
        self.audio.is_some()
    }

    /// Current source-sample position of the foreground voice or effect.
    pub fn foreground_audio_position(&self) -> Result<Option<u64>> {
        Ok(self.audio_ref()?.foreground_position())
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
        state.pending_profile = None;
        state.vm_execution_enabled = true;
        self.execute_and_apply_lifecycle_script_frame(state)
            .context("initializing the saved BloodScript profile")?;

        let state_byte_count = original_save_state_block_byte_count(
            self.runtime
                .current_profile()
                .context("saved profile initialization did not retain a profile")?,
        )
        .context("resolving the saved profile state allocation")?;
        let save = OriginalSaveGame::decode(data, state_byte_count)
            .context("decoding the complete original save image")?;
        save.restore_into(
            self.runtime
                .current_profile_mut()
                .context("saved profile disappeared before state restoration")?,
        )
        .context("restoring the original save blocks")?;
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
        state: &mut GameLifecycleState,
    ) -> Result<crate::native::bloodprg::ShipHudCoordinatorOutcome> {
        let mut ship_hud = self
            .ship_hud
            .take()
            .context("ship HUD update is reentrant")?;
        let outcome = ship_hud.update(self, state);
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
        let current = self
            .scripts
            .backend()
            .active_description_object()
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
            state,
            presentable_targets,
        );
        self.ship_target_selector = Some(selector);
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
            scene_top_row: self
                .scripts
                .backend()
                .assets()
                .location_scene_top_row()
                .unwrap_or(u16::MIN),
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

    /// Publish the complete non-actionable C4 record emitted by navigation presentation.
    pub fn defer_ship_actor_presentation(&mut self, target: ScriptObjectId) {
        self.scripts.defer_actor_presentation(target);
    }

    /// Return the current typed ship target selected by script or HUD state.
    pub fn current_ship_navigation_target(&self) -> Result<ScriptObjectId> {
        self.scripts
            .action_state()
            .current_ship_target
            .or_else(|| {
                self.ship_hud
                    .as_ref()
                    .and_then(RuntimeShipHud::coordinator)
                    .map(|state| state.current_target)
            })
            .context("ship navigation has no current target")
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

    /// Reset the dialogue word-choice owner when navigation returns to the bridge.
    pub fn reset_navigation_word_choice(&mut self) -> Result<()> {
        self.presentation_word_choice
            .as_mut()
            .context("presentation word choice is already being updated")?
            .reset();
        Ok(())
    }

    /// Mark the script-side ship interface inactive after full navigation teardown.
    pub fn finish_ship_navigation_reset(&mut self) {
        let action = self.scripts.action_state_mut();
        action.ship_navigation_mode = ScriptShipNavigationMode::Inactive;
        action.bridge_redraw_pending = false;
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
        let frame = prepare_choice_list_frame(
            &mut self.runtime,
            labels,
            config,
            state,
            ChoiceListPointer {
                position: pointer,
                primary_pressed: primary_pointer_pressed,
            },
        )?;
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
        let mut subtitle = self
            .subtitle_reveal
            .take()
            .context("subtitle reveal update is reentrant")?;
        subtitle.import_lifecycle_state(&state.presentation, self.ship_presentation.hud_active());
        let outcome = subtitle.update(&mut self.runtime, self.scripts.text_presentation_mut());
        self.subtitle_reveal = Some(subtitle);
        let outcome = outcome?;
        self.scripts.finish_lifecycle_frame(state)?;
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

    /// Rebuild the retained bridge surface and arm the startup reverse panel.
    pub fn initialize_bridge_screen(&mut self, startup_presentation_mode: bool) -> Result<()> {
        self.runtime
            .activate_retained_bridge_background()
            .context("activating retained bridge artwork during screen rebuild")?;
        if startup_presentation_mode {
            let screen = self
                .presentation_screen
                .as_mut()
                .context("presentation screen is already being updated")?
                .state_mut();
            screen.set_active(true);
            screen.set_reverse(true);
        }
        Ok(())
    }

    /// Borrow the bridge presentation state published to its frame coordinator.
    pub fn presentation_screen_state(&self) -> Result<&PresentationScreenState> {
        Ok(self
            .presentation_screen
            .as_ref()
            .context("presentation screen is already being updated")?
            .state())
    }

    /// Advance the bridge presentation panel from live script and pointer state.
    pub fn update_presentation_screen(
        &mut self,
        queued_scene_link: &GameSceneLink,
        primary_pointer_pressed: bool,
    ) -> Result<PresentationScreenOutcome> {
        let active_record_related = self.scripts.backend().active_description_object();
        let scruter_jo_record = self
            .runtime
            .current_profile()
            .and_then(|profile| profile.builtins().scruter_jo);
        let mut screen = self
            .presentation_screen
            .take()
            .context("presentation screen update is reentrant")?;
        screen
            .state_mut()
            .set_primary_pressed(primary_pointer_pressed);
        let outcome = screen.update(
            self,
            queued_scene_link,
            active_record_related,
            scruter_jo_record,
        );
        self.presentation_screen = Some(screen);
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

        state.navigation_rebuild_pending |= screen_rebuild_pending;
        state.presentation.completion_audio_pending |= completion_audio_pending;
        if choice_animation_requested {
            self.manu3_hand.requested_animation = PRESENTATION_CHOICE_MANU3_ANIMATION;
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
        text: &mut TextPresentationState,
    ) -> Result<()> {
        self.scripts
            .apply_object_description_to_text(object, text)?
            .with_context(|| {
                format!("scene-transition object {object:?} has no DESCRIPT record")
            })?;
        self.synchronize_script_presentations()
    }

    /// Queue one recovered MANU3 animation selector for the frame-tail dispatcher.
    pub fn request_manu3_animation(&mut self, selector: u16) {
        self.manu3_hand.requested_animation = selector;
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
    pub fn dispatch_ship_scene(&mut self) -> Result<PresentationSceneDispatchOutcome> {
        let active_record_related = self.scripts.backend().active_description_object();
        let scruter_jo_record = self
            .runtime
            .current_profile()
            .and_then(|profile| profile.builtins().scruter_jo);
        let mut screen = self
            .presentation_screen
            .take()
            .context("ship scene dispatch is reentrant")?;
        let mut ship = std::mem::take(&mut self.ship_presentation);
        let outcome =
            screen.dispatch_ship_scene(self, &mut ship, active_record_related, scruter_jo_record);
        self.ship_presentation = ship;
        self.presentation_screen = Some(screen);
        outcome
    }

    pub(super) fn dispatch_scene_transition(
        &mut self,
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
                transition,
                presentation,
                lifecycle,
                active_record_related,
                scruter_jo_record,
                palette_transition_percent,
            },
        );
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
        let owner_matches = state.presentation.owner == Some(GamePresentationOwner::DeferredMenu);
        let outcome = self.reveal_inline_menu(owner_matches, word_delay)?;
        self.scripts.finish_lifecycle_frame(state)?;
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

    /// Execute one complete translated COD/BAS/presentation frame.
    pub fn execute_script_frame(&mut self, enabled: bool) -> Result<ScriptFrameOutcome> {
        self.scripts.execute_frame(&mut self.runtime, enabled)
    }

    /// Execute one translated script frame and apply every ordered host command it emitted.
    pub fn execute_and_apply_script_frame(&mut self, enabled: bool) -> Result<ScriptFrameOutcome> {
        let outcome = self.execute_script_frame(enabled)?;
        self.synchronize_script_ship_state();
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
        let outcome =
            self.scripts
                .execute_lifecycle_frame(&mut self.runtime, state, execution_enabled)?;
        self.synchronize_script_ship_state();
        self.synchronize_script_presentations()?;
        self.process_script_commands()?;
        Ok(outcome)
    }

    /// Publish one-frame BloodScript target changes into the canonical ship FSM state.
    pub fn synchronize_script_ship_state(&mut self) {
        let presentation_target = self
            .scripts
            .last_presentation_outcome()
            .and_then(|outcome| outcome.presentation_started);
        let selected_target = self.scripts.take_selected_ship_target();
        let Some(target) = selected_target.or(presentation_target) else {
            return;
        };

        let action = self.scripts.action_state_mut();
        action.current_ship_target = Some(target);
        action.ship_navigation_mode = ScriptShipNavigationMode::Active;
        action.bridge_redraw_pending = false;
        self.ship_presentation.flags = SHIP_NAVIGATION_ACTIVE_FLAGS;
        self.ship_presentation.bridge_redraw_pending = u8::MIN;
        if selected_target.is_some() {
            self.ship_presentation.active_line = SHIP_NAVIGATION_STATUS_LINE;
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
        policy: PresentationPresentPolicy,
        timer_tick: u16,
    ) -> Result<PresentationResourceSequenceOutcome> {
        self.presentation_player
            .load(&mut self.runtime, line, policy, timer_tick)
    }

    /// Advance the active presentation queue with explicit clock samples.
    pub fn service_presentation_sequence(
        &mut self,
        audio_position: u16,
        timer_tick: u16,
    ) -> Result<RuntimePresentationStepOutcome> {
        self.presentation_player
            .service_frame(&mut self.runtime, audio_position, timer_tick)
    }

    /// Return whether a presentation stream is retained and still draining.
    pub fn presentation_stream_active(&self) -> bool {
        self.presentation_player.has_stream() && !self.presentation_player.is_finished()
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
            .complete_word_choice(&mut self.runtime, concept)?;
        self.scripts.finish_lifecycle_frame(state)
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
                    self.runtime.restart_name_area_effect();
                }
                RuntimeScriptCommand::TransitionPresentationEntity(entity) => {
                    self.runtime.transition_presentation_entity(entity)?;
                }
                RuntimeScriptCommand::RestartNavigationMusic => {
                    self.restart_navigation_music()?;
                }
                RuntimeScriptCommand::PlayRadioClip { clip_index } => {
                    self.play_loaded_sound_bank_clip(clip_index)?;
                }
                RuntimeScriptCommand::StartCameraTransition => {
                    self.runtime.start_camera_transition();
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
        self.presentation
            .present_frame(&self.runtime, self.bridge_frame.as_ref())?;
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
        self.confirm_dialog
            .update(&mut self.runtime, state, pointer_position)
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
        let Self {
            runtime,
            bridge_scene,
            bridge_frame,
            ..
        } = self;
        let scene = bridge_scene
            .as_mut()
            .context("bridge scene has not been initialized")?;
        let mut frame = scene
            .render_frame(input, runtime.bridge_sprite_entities_mut())
            .context("rendering bridge scene")?;
        frame.object_sprite_pixels = runtime
            .rasterize_ship_object_layer()
            .context("rendering bridge ship-object layer")?
            .into_pixels();
        *bridge_frame = Some(frame);
        Ok(bridge_frame
            .as_ref()
            .expect("rendered bridge frame was retained"))
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

    /// Present one translated bridge scene frame and optional MANU3 overlay.
    pub fn present_bridge_frame(&mut self, bridge_frame: &BridgeSceneFrame) -> Result<()> {
        self.ensure_main_viewport()?;
        self.presentation
            .present_frame(&self.runtime, Some(bridge_frame))
    }

    /// Present the most recently generated bridge frame.
    pub fn present_current_bridge_frame(&mut self) -> Result<()> {
        self.ensure_main_viewport()?;
        let frame = self
            .bridge_frame
            .as_ref()
            .context("no rendered bridge frame is ready")?;
        self.presentation.present_frame(&self.runtime, Some(frame))
    }

    /// Drop the live bridge and its owned panorama during shutdown.
    pub fn close_bridge_scene(&mut self) -> bool {
        self.bridge_frame = None;
        self.bridge_scene.take().is_some()
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
    use commander_blood_formats::script::decode_script_dictionary;

    use super::*;
    use crate::native::bloodprg::PointerButton;
    use crate::runtime::OriginalGameDataPaths;
    use crate::runtime::camera_approach::update_runtime_camera_approach;

    const TEST_CLOCK_SEED: u8 = 17;
    const TEST_SCRIPT_CLOCK: ScriptClock = ScriptClock {
        hour: 12,
        day: 2,
        month: 1,
    };
    const HYPERSPACE_PRESENTATION_LINE: PresentationResourceId = PresentationResourceId::new(6);
    const MAXIMUM_CAMERA_TRANSITION_FRAMES: usize = 2_048;

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
        services.load_default_sound_bank().unwrap();
        services.initialize_back_buffer().unwrap();
        services
            .load_script_profile(ScriptProfileId::new(u8::MIN).unwrap())
            .unwrap();
        let script = services.execute_script_frame(true).unwrap();
        assert_ne!(
            script.end,
            crate::native::bloodprg::ScriptFrameEnd::ExecutionDisabled
        );
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
        assert!(services.process_runtime_audio_events().unwrap().is_empty());
        let dialogue_requests = services.process_runtime_audio_events().unwrap();
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
        let bridge_frame = services
            .render_bridge_frame(BridgeSceneInput::default())
            .unwrap();
        assert!(!bridge_frame.starfield.plotted.is_empty());
        services.input_mut().poll_pointer(
            [320.0, 200.0],
            [200.0, 80.0],
            PointerButtons::from_bits(PointerButton::Primary as u16),
        );
        let mut lifecycle = GameLifecycleState::default();
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
                record: crate::native::bloodprg::ScriptActionRecord::ActorPresentation(horn),
                actionable: false,
            }
        );
        services.submit_indexed_frame().unwrap();
        services.present_current_bridge_frame().unwrap();

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
        assert!(services.close_bridge_scene());
        assert!(
            services
                .runtime()
                .front_buffer()
                .pixels()
                .iter()
                .any(|pixel| *pixel != u8::MIN)
        );
    }
}
