//! Concrete runtime services assembled for the recovered top-level lifecycle.

use anyhow::{Context, Result, bail};
use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::bloodprg::{BloodprgFontResources, decode_bloodprg_bridge_resources};
use commander_blood_formats::script::ScriptWordId;
use commander_blood_formats::snd::{SndBank, VocPcm};
use sdl3::AudioSubsystem;
use sdl3::video::Window;

use crate::native::bloodprg::{
    BridgeScene, BridgeSceneFrame, BridgeSceneInput, ConfirmDialogOutcome, ConfirmDialogState,
    DescriptRecordApplication, FontPoint, FontVerticalBand, GameFontFace, GameLifecycleState,
    GamePresentationOwner, GameSceneLink, InlineMenuRevealOutcome, InlineMenuTextMetrics,
    InputAction, Manu3HandFrameContext, Manu3HandFrameState, PbmDecodeResult, PointerButtonEdges,
    PointerButtons, PointerSample, PresentationChoiceNumber, PresentationPresentPolicy,
    PresentationResourceId, PresentationResourceSequenceOutcome, PresentationScreenOutcome,
    PresentationScreenState, ScriptClock, ScriptFrameOutcome, ScriptProfileId,
    ScriptProfileLoadOutcome, ShipPresentationState, ShipProjectionResources,
    StartupPreparationOutcome, draw_planar_dialogue_text, measure_game_text_width,
    reveal_inline_menu_step, update_manu3_hand_frame,
};
use crate::native::manu3::animation::CursorPosition;
use crate::native::random::BloodPrng;

use super::{
    LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_PIXEL_COUNT, OriginalGameData,
    OriginalGameRuntime, RuntimeAssetLoadStatus, RuntimeAudioHost, RuntimeConfirmDialog,
    RuntimeInputHost, RuntimePcmClip, RuntimePresentationCatalog, RuntimePresentationHost,
    RuntimePresentationPlayer, RuntimePresentationScreen, RuntimePresentationStepOutcome,
    RuntimeScriptBackend, RuntimeScriptCommand, RuntimeScriptSystem, VGA_BIOS_FONT_8X8,
};

const INITIAL_LOGICAL_POINTER: [i16; 2] = [160, 100];
const MUSIC_RESOURCE_DIRECTORY: &[u8] = b"MU\\";
const DEFAULT_BRIDGE_SOUND_BANK: &[u8] = b"tb.snd";
const FULL_LOGICAL_FONT_BAND: FontVerticalBand = FontVerticalBand {
    top: 0,
    bottom: LOGICAL_FRAMEBUFFER_HEIGHT as i32 - 1,
};
const MENU_WIDTH_PROBE_ORIGIN: FontPoint = FontPoint { x: 10, y: 8 };
const MENU_WIDTH_PROBE_COLOR: u8 = u8::MIN;

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
    loaded_voice: Option<RuntimePcmClip>,
    bridge_scene: Option<BridgeScene>,
    bridge_frame: Option<BridgeSceneFrame>,
    presentation_screen: Option<RuntimePresentationScreen>,
    confirm_dialog: RuntimeConfirmDialog,
    manu3_hand: Manu3HandFrameState,
    ship_presentation: ShipPresentationState,
    random: BloodPrng,
    scripts: RuntimeScriptSystem,
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
        let scripts = RuntimeScriptSystem::new(&data, script_clock);
        let presentation_player = RuntimePresentationPlayer::new(data.presentation_catalog());
        let runtime = OriginalGameRuntime::new(data);
        let presentation_screen = RuntimePresentationScreen::new(*runtime.live_palette())?;
        let presentation = RuntimePresentationHost::new_startup(window, &runtime)?;
        Ok(Self {
            runtime,
            input: RuntimeInputHost::new(INITIAL_LOGICAL_POINTER),
            presentation,
            presentation_player,
            audio: None,
            loaded_voice: None,
            bridge_scene: None,
            bridge_frame: None,
            presentation_screen: Some(presentation_screen),
            confirm_dialog,
            manu3_hand: Manu3HandFrameState::default(),
            ship_presentation: ShipPresentationState::default(),
            random: BloodPrng::default(),
            scripts,
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

    /// Load and validate the authored startup `CARTE.SPR` cache resource.
    pub fn load_initial_cartography_resource(&mut self) -> Result<()> {
        let _ = self.runtime.load_startup_cartography_resource()?;
        Ok(())
    }

    /// Load and validate the default `SN\\TB.SND` resident bridge sound bank.
    pub fn load_default_sound_bank(&mut self) -> Result<()> {
        self.scripts
            .backend_mut()
            .load_resident_sound_bank(DEFAULT_BRIDGE_SOUND_BANK)
            .context("loading default bridge sound bank")?;
        let loaded = self
            .scripts
            .backend()
            .loaded_sound_bank()
            .context("default bridge sound bank was not retained")?;
        SndBank::decode(loaded.encoded_bytes()).context("decoding default bridge sound bank")?;
        Ok(())
    }

    /// Decode and start the navigation music selected by the active DESCRIPT record.
    pub fn restart_navigation_music(&mut self) -> Result<()> {
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
        self.audio_mut()?
            .play_background(RuntimePcmClip::from_voc(&decoded))
    }

    /// Decode and play one authored clip from the currently loaded SND bank.
    pub fn play_loaded_sound_bank_clip(&mut self, clip_index: u8) -> Result<()> {
        let resource = self
            .scripts
            .backend()
            .loaded_sound_bank()
            .context("no DESCRIPT sound bank is loaded")?;
        let bank = SndBank::decode(resource.encoded_bytes()).with_context(|| {
            format!(
                "decoding sound bank {}",
                String::from_utf8_lossy(resource.name())
            )
        })?;
        let clip = bank
            .clip(usize::from(clip_index))
            .with_context(|| format!("sound bank clip {clip_index} is not authored"))?;
        let clip = RuntimePcmClip::from_snd_clip(clip)?;
        self.audio_mut()?.play_foreground(clip)
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

    /// Surface asynchronous SDL audio failures on the game thread.
    pub fn check_audio(&self) -> Result<()> {
        self.audio_ref()?.check_callback()
    }

    /// Current source-sample position of navigation music, when active.
    pub fn navigation_music_position(&self) -> Result<Option<u64>> {
        Ok(self.audio_ref()?.background_position())
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
        self.scripts.load_profile(&mut self.runtime, profile)
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

    /// Enable or disable the bridge's recovered six-choice presentation panel.
    pub fn set_presentation_screen_active(&mut self, active: bool) -> Result<()> {
        self.presentation_screen
            .as_mut()
            .context("presentation screen is already being updated")?
            .state_mut()
            .set_active(active);
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

    /// Apply one selected DESCRIPT record through the live BloodScript text state.
    pub fn apply_presentation_description(
        &mut self,
        name: &[u8],
    ) -> Result<Option<DescriptRecordApplication>> {
        let application = self.scripts.apply_presentation_description(name)?;
        self.synchronize_script_presentations()?;
        Ok(application)
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
        self.synchronize_script_presentations()?;
        self.process_script_commands()?;
        Ok(outcome)
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

    /// Borrow resolved fixed and DESCRIPT-authored presentation metadata.
    pub const fn presentation_catalog(&self) -> &RuntimePresentationCatalog {
        self.presentation_player.catalog()
    }

    /// Borrow the concrete script backend for lifecycle-state updates.
    pub const fn script_backend(&self) -> &RuntimeScriptBackend {
        self.scripts.backend()
    }

    /// Mutably borrow the concrete script backend for lifecycle-state updates.
    pub fn script_backend_mut(&mut self) -> &mut RuntimeScriptBackend {
        self.scripts.backend_mut()
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
        let scene = self
            .bridge_scene
            .as_mut()
            .context("bridge scene has not been initialized")?;
        self.bridge_frame = Some(
            scene
                .render_frame(input)
                .context("rendering bridge scene")?,
        );
        Ok(self
            .bridge_frame
            .as_ref()
            .expect("rendered bridge frame was retained"))
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
    use crate::runtime::OriginalGameDataPaths;

    const TEST_CLOCK_SEED: u8 = 17;
    const TEST_SCRIPT_CLOCK: ScriptClock = ScriptClock {
        hour: 12,
        day: 2,
        month: 1,
    };

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
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        if std::env::var_os("WAYLAND_DISPLAY").is_none() && std::env::var_os("DISPLAY").is_none() {
            return;
        }
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
        let bridge_frame = services
            .render_bridge_frame(BridgeSceneInput::default())
            .unwrap();
        assert!(!bridge_frame.starfield.plotted.is_empty());
        services.submit_indexed_frame().unwrap();
        services.present_current_bridge_frame().unwrap();

        assert_eq!(services.presented_frame_count(), 2);
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
