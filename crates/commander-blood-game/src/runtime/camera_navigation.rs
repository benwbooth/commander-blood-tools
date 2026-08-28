//! Flat runtime owner for bridge camera-navigation activation.

use anyhow::{Context, Result};
use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};
use commander_blood_formats::script::ScriptObjectKind;

use crate::native::bloodprg::{
    CameraNavigationLocation, CameraNavigationOutcome, CameraNavigationRegionPoll,
    CameraNavigationSlot, CameraNavigationState, GameLifecycleState, Manu3AnimationSelector,
    NavActorSlot, PointerButton, PresentationHitRectangle, PrimaryPointerSample,
    STATUS_REGION_POLL_ATTEMPTS, StatusRegionPollBackend, poll_status_region,
    update_camera_navigation,
};

use super::{ModernGameServices, RuntimePaletteTransitionConfig};

const ACCESS_COUNTER_WORD_INDEX: usize = 10;
const STATUS_REGION_ENTITY_INDEX: usize = 31;
const SHIP_DESTINATION_ENTRY_FLAGS: u16 = 5;
const HUD_INITIALIZATION_PENDING: u8 = 1;

/// Persistent semantic state for native `camera_nav_update`.
#[derive(Default)]
pub(super) struct RuntimeCameraNavigation {
    state: CameraNavigationState,
}

impl RuntimeCameraNavigation {
    /// Advance destination activation against live profile, pointer, and palette state.
    pub(super) fn update(
        &mut self,
        services: &mut ModernGameServices<'_>,
        lifecycle: &mut GameLifecycleState,
        actor_slot: &mut NavActorSlot,
    ) -> Result<CameraNavigationOutcome> {
        let (target, kind) = services.current_arche_navigation_target()?;
        let access_count = if matches!(
            kind,
            ScriptObjectKind::CelestialBody | ScriptObjectKind::NavigationEntity
        ) {
            let profile = services
                .runtime()
                .current_profile()
                .context("camera navigation requires a loaded BloodScript profile")?;
            let field = profile
                .state()
                .object_word(target, ACCESS_COUNTER_WORD_INDEX)
                .with_context(|| format!("navigation target {target:?} has no access word"))?;
            profile.state().word(field).with_context(|| {
                format!("navigation target {target:?} access word is unreadable")
            })?
        } else {
            u16::MIN
        };
        let mut location = CameraNavigationLocation { kind, access_count };
        let mut slot = CameraNavigationSlot {
            locked: actor_slot.flags.locked,
            ready: actor_slot.flags.auto_seek,
        };

        self.state
            .set_camera_view_active(services.bridge_camera_view_active());
        self.state
            .set_approach_active(services.runtime().camera_approach().transition_pending);
        let live_palette = flatten_palette(services.runtime().live_palette());
        let entity = services.runtime().bridge_sprite_entities()[STATUS_REGION_ENTITY_INDEX];
        let pointer = services.input().pointer_sample();
        let mut poll = RuntimeDestinationRegionPoll {
            enabled: entity.flags.is_active(),
            region: PresentationHitRectangle::new(
                [entity.draw_position.x as i16, entity.draw_position.y as i16],
                [entity.extent.width as i16, entity.extent.height as i16],
            ),
            pointer: PrimaryPointerSample {
                primary_pressed: pointer.buttons.contains(PointerButton::Primary),
                position: pointer.position,
            },
        };
        let outcome = update_camera_navigation(
            &mut self.state,
            &mut location,
            &mut slot,
            &live_palette,
            &mut poll,
        );
        actor_slot.flags.auto_seek = slot.ready;

        if matches!(
            outcome,
            CameraNavigationOutcome::DestinationUnavailable
                | CameraNavigationOutcome::TransitionStarted
        ) {
            services.request_manu3_animation(Manu3AnimationSelector::CameraDestinationOrRightChart);
        }

        match outcome {
            CameraNavigationOutcome::DestinationUnavailable => {
                lifecycle.navigation_rebuild_pending |= self.state.redraw_requested();
            }
            CameraNavigationOutcome::TransitionStarted => {
                let transition = self
                    .state
                    .palette_transition()
                    .context("camera navigation started without a palette transition")?;
                services
                    .palette_transition_mut()
                    .configure(RuntimePaletteTransitionConfig {
                        source: expand_palette(&transition.source),
                        target: expand_palette(&transition.target),
                        initial_percent: u16::from(transition.percent),
                        increment: u16::from(transition.increment),
                        colors: transition.first_color..=transition.last_color,
                    })
                    .context("configuring camera-navigation palette transition")?;
                services.runtime_mut().front_buffer_mut().clear(u8::MIN);
                let ship = services.ship_presentation_state_mut();
                ship.flags = SHIP_DESTINATION_ENTRY_FLAGS;
                ship.hud_initialization_pending = HUD_INITIALIZATION_PENDING;
                ship.scene_dispatch_blocked = self.state.scene_dispatch_blocked();
                ship.depth_offset = self.state.ship_depth_offset();
                ship.depth_opening_flags = self.state.depth_opening() as u8;
                lifecycle.set_presentation_interface_active(self.state.ui_active());
                lifecycle.presentation.dialogue_hold_complete = self.state.dialogue_hold_complete();
                services.request_ship_hud_reinitialization()?;
            }
            _ => {}
        }
        Ok(outcome)
    }
}

struct RuntimeDestinationRegionPoll {
    enabled: bool,
    region: PresentationHitRectangle,
    pointer: PrimaryPointerSample,
}

impl CameraNavigationRegionPoll for RuntimeDestinationRegionPoll {
    fn poll_destination_region(
        &mut self,
        _location: &mut CameraNavigationLocation,
        _slot: &mut CameraNavigationSlot,
    ) -> bool {
        poll_status_region(self.region, self).is_some_and(|hit| {
            usize::from(hit.attempts_remaining) + 1 == STATUS_REGION_POLL_ATTEMPTS
        })
    }
}

impl StatusRegionPollBackend for RuntimeDestinationRegionPoll {
    fn status_region_enabled(&mut self, _attempts_remaining: u8) -> bool {
        self.enabled
    }

    fn primary_pointer_sample(&mut self, _attempts_remaining: u8) -> PrimaryPointerSample {
        self.pointer
    }
}

fn flatten_palette(
    palette: &[[u8; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
) -> [u8; PALETTE_ENTRY_COUNT * RGB_COMPONENT_COUNT] {
    let mut bytes = [u8::MIN; PALETTE_ENTRY_COUNT * RGB_COMPONENT_COUNT];
    for (destination, source) in bytes
        .chunks_exact_mut(RGB_COMPONENT_COUNT)
        .zip(palette.iter())
    {
        destination.copy_from_slice(source);
    }
    bytes
}

fn expand_palette(
    bytes: &[u8; PALETTE_ENTRY_COUNT * RGB_COMPONENT_COUNT],
) -> [[u8; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT] {
    std::array::from_fn(|index| {
        let start = index * RGB_COMPONENT_COUNT;
        bytes[start..start + RGB_COMPONENT_COUNT]
            .try_into()
            .expect("palette component chunk has fixed length")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destination_poll_accepts_only_the_native_first_attempt_result() {
        let mut poll = RuntimeDestinationRegionPoll {
            enabled: true,
            region: PresentationHitRectangle::new([10, 20], [30, 40]),
            pointer: PrimaryPointerSample {
                primary_pressed: true,
                position: [10, 20],
            },
        };
        let mut location = CameraNavigationLocation {
            kind: ScriptObjectKind::CelestialBody,
            access_count: 1,
        };
        let mut slot = CameraNavigationSlot::default();

        assert!(poll.poll_destination_region(&mut location, &mut slot));
        poll.pointer.primary_pressed = false;
        assert!(!poll.poll_destination_region(&mut location, &mut slot));
    }

    #[test]
    fn palette_conversion_preserves_every_component() {
        let palette = std::array::from_fn(|color| {
            std::array::from_fn(|component| (color * RGB_COMPONENT_COUNT + component) as u8)
        });
        assert_eq!(expand_palette(&flatten_palette(&palette)), palette);
    }
}
