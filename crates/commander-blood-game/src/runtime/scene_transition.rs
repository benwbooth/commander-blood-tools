//! Flat runtime host for contact-driven bridge scene transitions.

use anyhow::{Context, Result, bail};
use commander_blood_formats::script::{ScriptObjectId, ScriptObjectKind, ScriptState};

use crate::native::bloodprg::{
    BRIDGE_SPRITE_ENTITY_COUNT, BridgeSceneInput, BridgeSpriteEntity, GameLifecycleState,
    GameSceneLink, PbmDecodeOptions, PbmPaletteUpdate, PbmTransparency,
    PresentationWordChoicePhase, SceneImageBand, SceneImageLoadOptions, SceneTransitionHost,
    SceneTransitionLine, SceneTransitionOutcome, SceneTransitionPalettes, SceneTransitionPhase,
    SceneTransitionRecordKind, SceneTransitionRecordSource, SceneTransitionState,
    ScriptPresentationScanState, decode_pbm_image, fill_back_buffer_band, update_scene_transition,
};

use super::game_lifecycle::bridge_steering_interaction;
use super::{
    ModernGameServices, RuntimePaletteTransitionConfig, RuntimePaletteTransitionSurface,
    RuntimePlatformHost,
};

pub(super) const SCENE_TRANSITION_IMAGE_RESOURCE: &[u8] = b"FRIGO.FD";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RuntimeSceneRecord {
    object: ScriptObjectId,
    kind: SceneTransitionRecordKind,
}

impl RuntimeSceneRecord {
    const fn new(object: ScriptObjectId, kind: ScriptObjectKind) -> Self {
        Self {
            object,
            kind: if matches!(kind, ScriptObjectKind::Actor) {
                SceneTransitionRecordKind::Presentation
            } else {
                SceneTransitionRecordKind::Other
            },
        }
    }
}

/// Persistent scene-transition state with typed script-record ownership.
#[derive(Default)]
pub struct RuntimeSceneTransition {
    state: SceneTransitionState,
    palettes: SceneTransitionPalettes,
    current_record: Option<RuntimeSceneRecord>,
    deferred_record: Option<RuntimeSceneRecord>,
}

impl RuntimeSceneTransition {
    /// Native contact pointers refer to VAR byte offsets, not DEB directory IDs.
    pub(super) fn record_bindings(
        &self,
        state: &ScriptState,
    ) -> Result<[Option<(usize, ScriptObjectKind)>; 2]> {
        let binding =
            |record: Option<RuntimeSceneRecord>| -> Result<Option<(usize, ScriptObjectKind)>> {
                record
                    .map(|record| {
                        let object = state
                            .object(record.object)
                            .context("contact record is absent before profile handoff")?;
                        Ok((object.source_offset(), object.kind))
                    })
                    .transpose()
            };
        Ok([
            binding(self.current_record)?,
            binding(self.deferred_record)?,
        ])
    }

    pub(super) fn rebind_records(
        &mut self,
        bindings: [Option<(usize, ScriptObjectKind)>; 2],
        state: &ScriptState,
    ) -> Result<()> {
        let resolve =
            |binding: Option<(usize, ScriptObjectKind)>| -> Result<Option<RuntimeSceneRecord>> {
                binding
                    .map(|(offset, kind)| {
                        let object = state
                            .objects()
                            .iter()
                            .find(|object| object.source_offset() == offset && object.kind == kind)
                            .context("contact record is absent after profile handoff")?;
                        Ok(RuntimeSceneRecord::new(object.id, object.kind))
                    })
                    .transpose()
            };
        let current = resolve(bindings[0])?;
        let deferred = resolve(bindings[1])?;
        self.current_record = current;
        self.deferred_record = deferred;
        Ok(())
    }

    /// Borrow the recovered coordinator state for diagnostics and tests.
    pub const fn state(&self) -> &SceneTransitionState {
        &self.state
    }

    /// Arm the contact selected by the bridge console as a typed scene record.
    pub fn begin(
        &mut self,
        current_record: Option<(ScriptObjectId, ScriptObjectKind)>,
        deferred_record: (ScriptObjectId, ScriptObjectKind),
    ) -> Result<()> {
        if self.state.phase != SceneTransitionPhase::Inactive {
            bail!("a scene transition is already active");
        }
        let deferred_record = RuntimeSceneRecord::new(deferred_record.0, deferred_record.1);
        self.current_record = Some(
            current_record
                .map(|(object, kind)| RuntimeSceneRecord::new(object, kind))
                .unwrap_or(deferred_record),
        );
        self.deferred_record = Some(deferred_record);
        self.state.begin();
        Ok(())
    }

    /// Advance one recovered scene-transition frame over canonical runtime state.
    pub fn update<'window>(
        &mut self,
        services: &mut ModernGameServices<'window>,
        lifecycle: &mut GameLifecycleState,
        scene_link: GameSceneLink,
        platform: &mut RuntimePlatformHost<'window>,
    ) -> Result<SceneTransitionOutcome> {
        if self.state.phase == SceneTransitionPhase::Inactive {
            lifecycle.profile_change_blockers.render_update_active = false;
            return Ok(SceneTransitionOutcome::Inactive);
        }

        import_shared_scene_state(&mut self.state, lifecycle);

        if self.state.bridge_blocked
            && services.latest_presentation_started()
                == self.deferred_record.map(|record| record.object)
        {
            self.state.bridge_blocked = false;
        }

        self.palettes.live = *services.runtime().live_palette();
        let shared_palette_percent = services.palette_transition().state().percent;
        self.palettes.transition.percent = u8::try_from(shared_palette_percent).with_context(|| {
            format!(
                "scene-transition palette percentage {shared_palette_percent} exceeds byte state"
            )
        })?;

        services.prepare_scene_transition_presentation(lifecycle);
        let mut presentation = services.presentation_scan_state().clone();
        let mut text = services.text_presentation().clone();
        lifecycle.presentation.request_flags = text.request_flags;
        let mut entities = std::mem::replace(
            services.runtime_mut().bridge_sprite_entities_mut(),
            [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT],
        );

        let (result, dispatch_palette_percent, scene_dispatched) = {
            let mut host = RuntimeSceneTransitionHost {
                services,
                lifecycle,
                platform,
                current_record: self
                    .current_record
                    .context("active scene transition has no current record")?,
                deferred_record: self
                    .deferred_record
                    .context("active scene transition has no deferred record")?,
                dispatch_palette_percent: shared_palette_percent,
                scene_dispatched: false,
            };
            let result = update_scene_transition(
                &mut self.state,
                &mut presentation,
                &mut text,
                &mut self.palettes,
                &mut entities,
                &scene_link,
                &mut host,
            );
            (result, host.dispatch_palette_percent, host.scene_dispatched)
        };
        *services.runtime_mut().bridge_sprite_entities_mut() = entities;
        let outcome = result.map_err(|error| anyhow::anyhow!("{error}"))?;

        if scene_dispatched && outcome != SceneTransitionOutcome::PaletteRestoreStarted {
            self.palettes.transition.percent = u8::try_from(dispatch_palette_percent)
                .context("scene dispatcher produced a palette percentage above 255")?;
            services
                .palette_transition_mut()
                .set_progress_percent(dispatch_palette_percent);
        }
        self.install_palette_transition(services, outcome)?;
        *services.runtime_mut().live_palette_mut() = self.palettes.live;

        reconcile_scene_request_flags(&mut text, lifecycle, scene_dispatched, outcome);
        services.commit_scene_transition_presentation(presentation, text, lifecycle)?;
        if outcome == SceneTransitionOutcome::DeferredRecordArmed {
            services.defer_ship_actor_presentation(
                self.deferred_record
                    .expect("active scene transition retains its deferred record")
                    .object,
            );
        }
        if outcome == SceneTransitionOutcome::CleanedUp {
            services.reset_presentation_word_choice()?;
        }
        if matches!(
            outcome,
            SceneTransitionOutcome::ImageLoaded
                | SceneTransitionOutcome::DeferredRecordArmed
                | SceneTransitionOutcome::CleanedUp
        ) && let Some(selector) = self.state.manu3_animation
        {
            services.request_manu3_animation(selector);
        }
        export_shared_scene_state(&self.state, lifecycle);
        lifecycle.navigation_rebuild_pending |= self.take_redraw_request();
        publish_scene_transition_ui(lifecycle, outcome);
        lifecycle.profile_change_blockers.render_update_active =
            self.state.phase != SceneTransitionPhase::Inactive;

        if self.state.phase == SceneTransitionPhase::Inactive {
            self.current_record = None;
            self.deferred_record = None;
        }
        Ok(outcome)
    }

    fn take_redraw_request(&mut self) -> bool {
        std::mem::take(&mut self.state.redraw_pending)
    }

    fn install_palette_transition(
        &self,
        services: &mut ModernGameServices<'_>,
        outcome: SceneTransitionOutcome,
    ) -> Result<()> {
        let presentation_image_loaded = outcome == SceneTransitionOutcome::ImageLoaded
            && self
                .deferred_record
                .is_some_and(|record| record.kind == SceneTransitionRecordKind::Presentation);
        if !presentation_image_loaded && outcome != SceneTransitionOutcome::PaletteRestoreStarted {
            return Ok(());
        }

        let first = u8::try_from(self.palettes.transition.first_color)
            .context("scene-transition first palette color exceeds the indexed palette")?;
        let last = u8::try_from(self.palettes.transition.last_color)
            .context("scene-transition last palette color exceeds the indexed palette")?;
        services
            .palette_transition_mut()
            .configure(RuntimePaletteTransitionConfig {
                surface: if presentation_image_loaded {
                    RuntimePaletteTransitionSurface::PresentationFrame
                } else {
                    RuntimePaletteTransitionSurface::GameFrame
                },
                source: self.palettes.source,
                target: self.palettes.target,
                initial_percent: u16::from(self.palettes.transition.percent),
                increment: u16::from(self.palettes.transition.increment),
                colors: first..=last,
            })
            .context("configuring the contact scene palette transition")
    }
}

fn import_shared_scene_state(state: &mut SceneTransitionState, lifecycle: &GameLifecycleState) {
    state.active_line = lifecycle
        .presentation
        .active_line
        .map(SceneTransitionLine::from_number);
    state.scene_gate_active = lifecycle.presentation.scene_gate_active;
}

fn export_shared_scene_state(state: &SceneTransitionState, lifecycle: &mut GameLifecycleState) {
    lifecycle.presentation.active_line = state.active_line.map(SceneTransitionLine::number);
    lifecycle.presentation.scene_gate_active = state.scene_gate_active;
}

fn reconcile_scene_request_flags(
    text: &mut crate::native::bloodprg::TextPresentationState,
    lifecycle: &GameLifecycleState,
    scene_dispatched: bool,
    outcome: SceneTransitionOutcome,
) {
    if scene_dispatched && outcome != SceneTransitionOutcome::CleanedUp {
        text.request_flags = lifecycle.presentation.request_flags;
    }
}

fn publish_scene_transition_ui(
    lifecycle: &mut GameLifecycleState,
    outcome: SceneTransitionOutcome,
) {
    match outcome {
        SceneTransitionOutcome::Initialized => lifecycle.set_low_ui_state_word(u16::MIN),
        SceneTransitionOutcome::CleanedUp => lifecycle.set_low_ui_state_word(1),
        _ => {}
    }
}

struct RuntimeSceneTransitionHost<'services, 'window, 'lifecycle, 'platform> {
    services: &'services mut ModernGameServices<'window>,
    lifecycle: &'lifecycle mut GameLifecycleState,
    platform: &'platform mut RuntimePlatformHost<'window>,
    current_record: RuntimeSceneRecord,
    deferred_record: RuntimeSceneRecord,
    dispatch_palette_percent: u16,
    scene_dispatched: bool,
}

impl RuntimeSceneTransitionHost<'_, '_, '_, '_> {
    const fn record(&self, source: SceneTransitionRecordSource) -> RuntimeSceneRecord {
        match source {
            SceneTransitionRecordSource::Current => self.current_record,
            SceneTransitionRecordSource::Deferred => self.deferred_record,
        }
    }
}

impl SceneTransitionHost for RuntimeSceneTransitionHost<'_, '_, '_, '_> {
    type SceneLink = GameSceneLink;
    type Error = anyhow::Error;

    fn scene_record_kind(&self, source: SceneTransitionRecordSource) -> SceneTransitionRecordKind {
        self.record(source).kind
    }

    fn lookup_scene_description(
        &mut self,
        source: SceneTransitionRecordSource,
        presentation_interface_active: bool,
        text: &mut crate::native::bloodprg::TextPresentationState,
    ) -> Result<()> {
        let record = self.record(source);
        self.lifecycle
            .set_presentation_interface_active(presentation_interface_active);
        self.services
            .apply_scene_transition_description(record.object, presentation_interface_active, text)
            .context("applying the selected contact DESCRIPT record")
    }

    fn dispatch_scene_line(
        &mut self,
        link: &Self::SceneLink,
        state: &mut SceneTransitionState,
        presentation: &mut ScriptPresentationScanState,
        live_palette: &mut crate::native::bloodprg::IndexedGamePalette,
    ) -> Result<()> {
        self.scene_dispatched = true;
        let related = self.record(state.record_source).object;
        *self.services.runtime_mut().live_palette_mut() = *live_palette;
        let result = self.services.dispatch_scene_transition(
            *link,
            state,
            presentation,
            self.lifecycle,
            related,
            &mut self.dispatch_palette_percent,
        );
        *live_palette = *self.services.runtime().live_palette();
        result?;
        Ok(())
    }

    fn load_scene_image(
        &mut self,
        options: SceneImageLoadOptions,
        live_palette: &mut crate::native::bloodprg::IndexedGamePalette,
    ) -> Result<()> {
        let encoded = self
            .services
            .runtime()
            .data()
            .load_named_resource(SCENE_TRANSITION_IMAGE_RESOURCE)
            .context("loading FRIGO.FD for a contact scene transition")?;
        let decode_options = PbmDecodeOptions {
            palette_update: if options.refresh_palette {
                PbmPaletteUpdate::SceneColors
            } else {
                PbmPaletteUpdate::Preserve
            },
            transparency: if options.transparent_zero {
                PbmTransparency::TransparentZero
            } else {
                PbmTransparency::Opaque
            },
        };
        let (_front, back) = self.services.runtime_mut().presentation_buffers_mut();
        decode_pbm_image(&encoded, back, live_palette, decode_options)
            .context("decoding FRIGO.FD into the retained contact background")?;
        self.services
            .stage_presentation_scene_palette(live_palette)?;
        Ok(())
    }

    fn present_scene_image(&mut self) -> Result<()> {
        self.services.runtime_mut().restore_back_buffer();
        Ok(())
    }

    fn clear_scene_image_band(&mut self, band: SceneImageBand) -> Result<()> {
        let (_front, back) = self.services.runtime_mut().presentation_buffers_mut();
        fill_back_buffer_band(
            back,
            usize::from(band.first_row),
            usize::from(band.last_row),
            band.color,
        )
        .context("clearing the retained contact scene band")
    }

    fn update_bridge(
        &mut self,
        _state: &mut SceneTransitionState,
        _presentation: &mut ScriptPresentationScanState,
    ) -> Result<()> {
        let pointer = self.services.input().pointer_sample();
        let retained_word_choice_owner =
            self.services.presentation_word_choice_phase()? != PresentationWordChoicePhase::Closed;
        self.services.update_bridge_steering(BridgeSceneInput {
            horizontal_delta: self.platform.take_bridge_horizontal_delta(),
            pointer_buttons: pointer.buttons.bits(),
            interaction: bridge_steering_interaction(
                self.lifecycle,
                retained_word_choice_owner,
                self.services.presentation_screen_state()?.active(),
            ),
        })?;
        Ok(())
    }

    fn run_alien_overlay(&mut self, _presentation: &mut ScriptPresentationScanState) -> Result<()> {
        self.services
            .run_runtime_alien_overlay_cycle(self.lifecycle, self.platform)
            .map(|_| ())
    }

    fn initialize_ship_hud(
        &mut self,
        live_palette: &mut crate::native::bloodprg::IndexedGamePalette,
    ) -> Result<()> {
        *self.services.runtime_mut().live_palette_mut() = *live_palette;
        self.services.snapshot_navigation_hud_palette_and_camera()?;
        *live_palette = *self.services.runtime().live_palette();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};

    use super::*;
    use crate::native::bloodprg::{IndexedGamePalette, PresentationRequestFlags, ScriptProfileId};
    use crate::runtime::{OriginalGameData, OriginalGameDataPaths, OriginalGameRuntime};

    const UNCHANGED_CONSOLE_COMPONENT: u8 = 17;
    const FIRST_CONSOLE_COLOR: usize = 192;

    #[test]
    fn begin_classifies_typed_records_and_rejects_reentry() {
        let Some(runtime) = loaded_initial_profile() else {
            return;
        };
        let profile = runtime.current_profile().unwrap();
        let actor = profile
            .state()
            .objects()
            .iter()
            .find(|record| record.kind == ScriptObjectKind::Actor)
            .unwrap();
        let other = profile
            .state()
            .objects()
            .iter()
            .find(|record| record.kind != ScriptObjectKind::Actor)
            .unwrap();
        let mut transition = RuntimeSceneTransition::default();

        transition
            .begin(Some((other.id, other.kind)), (actor.id, actor.kind))
            .unwrap();

        assert_eq!(transition.state.phase, SceneTransitionPhase::Initialize);
        assert_eq!(
            transition.current_record.unwrap().kind,
            SceneTransitionRecordKind::Other
        );
        assert_eq!(
            transition.deferred_record.unwrap().kind,
            SceneTransitionRecordKind::Presentation
        );
        assert!(transition.begin(None, (actor.id, actor.kind)).is_err());
    }

    #[test]
    fn shipped_transition_image_preserves_the_bridge_console_palette() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        let data = OriginalGameData::load(paths).unwrap();
        let encoded = data
            .load_named_resource(SCENE_TRANSITION_IMAGE_RESOURCE)
            .unwrap();
        let mut framebuffer = vec![u8::MIN; crate::runtime::LOGICAL_FRAMEBUFFER_PIXEL_COUNT];
        let mut palette: IndexedGamePalette =
            [[UNCHANGED_CONSOLE_COMPONENT; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];

        let outcome = decode_pbm_image(
            &encoded,
            &mut framebuffer,
            &mut palette,
            PbmDecodeOptions {
                palette_update: PbmPaletteUpdate::SceneColors,
                transparency: PbmTransparency::Opaque,
            },
        )
        .unwrap();

        assert!(outcome.palette_changed);
        assert!(framebuffer.iter().any(|pixel| *pixel != u8::MIN));
        assert!(
            palette[..FIRST_CONSOLE_COLOR]
                .iter()
                .any(|color| *color != [UNCHANGED_CONSOLE_COMPONENT; RGB_COMPONENT_COUNT])
        );
        assert!(
            palette[FIRST_CONSOLE_COLOR..]
                .iter()
                .all(|color| *color == [UNCHANGED_CONSOLE_COMPONENT; RGB_COMPONENT_COUNT])
        );
    }

    #[test]
    fn completed_scene_redraw_is_published_once() {
        let mut transition = RuntimeSceneTransition::default();
        transition.state.redraw_pending = true;

        assert!(transition.take_redraw_request());
        assert!(!transition.take_redraw_request());
    }

    #[test]
    fn scene_transition_imports_and_exports_the_shared_dialogue_line() {
        const SCRUTER_JO_VOICE_SELECTOR: i8 = 20;

        let dialogue_line =
            crate::native::bloodprg::presentation_line_for_text_selector(SCRUTER_JO_VOICE_SELECTOR);
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.presentation.active_line = Some(dialogue_line);
        lifecycle.presentation.scene_gate_active = true;
        let mut state = SceneTransitionState {
            active_line: Some(SceneTransitionLine::Complete),
            scene_gate_active: false,
            ..SceneTransitionState::default()
        };

        import_shared_scene_state(&mut state, &lifecycle);

        assert_eq!(
            state.active_line,
            Some(SceneTransitionLine::Other(dialogue_line))
        );
        assert!(state.scene_gate_active);

        state.active_line = Some(SceneTransitionLine::AlienOverlayEntry);
        state.scene_gate_active = false;
        export_shared_scene_state(&state, &mut lifecycle);

        assert_eq!(
            lifecycle.presentation.active_line,
            Some(SceneTransitionLine::AlienOverlayEntry.number())
        );
        assert!(!lifecycle.presentation.scene_gate_active);
    }

    #[test]
    fn scene_transition_applies_native_full_ui_word_assignments() {
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.set_low_ui_state_word(15);

        publish_scene_transition_ui(&mut lifecycle, SceneTransitionOutcome::Initialized);
        assert_eq!(lifecycle.low_ui_state_word(), u16::MIN);

        lifecycle.set_low_ui_state_word(15);
        publish_scene_transition_ui(&mut lifecycle, SceneTransitionOutcome::CleanedUp);
        assert_eq!(lifecycle.low_ui_state_word(), 1);
    }

    #[test]
    fn scene_cleanup_keeps_the_native_request_bit_clear() {
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.presentation.request_flags = PresentationRequestFlags::decode(11);
        let mut text = crate::native::bloodprg::TextPresentationState {
            request_flags: PresentationRequestFlags::decode(8),
            ..crate::native::bloodprg::TextPresentationState::default()
        };

        reconcile_scene_request_flags(
            &mut text,
            &lifecycle,
            true,
            SceneTransitionOutcome::CleanedUp,
        );

        assert_eq!(text.request_flags.bits(), 8);
    }

    fn loaded_initial_profile() -> Option<OriginalGameRuntime> {
        let paths = OriginalGameDataPaths::discover(None).ok()?;
        let data = OriginalGameData::load(paths).ok()?;
        let mut runtime = OriginalGameRuntime::new(data);
        runtime.load_profile(ScriptProfileId::INITIAL).ok()?;
        Some(runtime)
    }
}
