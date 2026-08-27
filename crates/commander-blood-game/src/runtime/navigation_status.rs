//! Concrete flat-memory owner for the recovered bridge navigation status.

use anyhow::{Context, Result};

use crate::native::bloodprg::{
    GameLifecycleState, NavigationStatusContext, NavigationStatusLabels, NavigationStatusLocation,
    NavigationStatusOutcome, NavigationStatusRegion, NavigationStatusSource, NavigationStatusState,
    NavigationStatusText, TextPresentationState, update_navigation_status,
};

use super::ModernGameServices;
use super::navigation_chart::RuntimeNavigationStatusSnapshot;

pub(super) const NAVIGATION_STATUS_ENTITY_INDEX: usize = 31;
const CARRIAGE_RETURN: u8 = b'\r';

/// Persistent semantic mode for the bridge location-and-roster hover.
#[derive(Default)]
pub(super) struct RuntimeNavigationStatus {
    state: NavigationStatusState,
}

impl RuntimeNavigationStatus {
    /// Advance one exact late-frame status gate and publish its subtitle surface.
    pub(super) fn update(
        &mut self,
        services: &mut ModernGameServices<'_>,
        lifecycle: &mut GameLifecycleState,
        snapshot: &RuntimeNavigationStatusSnapshot,
    ) -> Result<NavigationStatusOutcome> {
        let hover_entity = *services
            .runtime()
            .bridge_sprite_entities()
            .get(NAVIGATION_STATUS_ENTITY_INDEX)
            .context("bridge navigation-status entity is absent")?;
        let pointer = services.input().pointer_sample().position;
        let display_visible = services.text_presentation().subtitle_word_list_mode;
        self.state.hover.visible = display_visible;
        if display_visible {
            self.state.hover.pending = false;
        }

        let sources = snapshot
            .sources
            .iter()
            .map(|source| NavigationStatusSource {
                kind: source.kind,
                active: source.active,
                life_support_visits: source.life_support_visits,
                location: source.location,
                name: source.name.as_ref(),
            })
            .collect::<Vec<_>>();
        let outcome = update_navigation_status(
            NavigationStatusContext {
                transition_pending: lifecycle.navigation_transition_pending,
                camera_view_active: services.bridge_camera_view_active(),
                presentation_active: lifecycle.presentation.active,
                hover_entity_enabled: hover_entity.flags.is_active(),
                hover_region: NavigationStatusRegion::new(
                    [hover_entity.draw_position.x, hover_entity.draw_position.y],
                    [hover_entity.extent.width, hover_entity.extent.height],
                ),
                pointer: pointer.map(|coordinate| coordinate as u16),
                location: NavigationStatusLocation {
                    kind: snapshot.location_kind,
                    name: snapshot.location_name.as_ref(),
                },
                sources: &sources,
                ark_location: Some(snapshot.ark_location),
                labels: NavigationStatusLabels {
                    planet: snapshot.labels.planet.as_ref(),
                    ship: snapshot.labels.ship.as_ref(),
                    black_hole: snapshot.labels.black_hole.as_ref(),
                    life_support: snapshot.labels.life_support.as_ref(),
                },
            },
            &mut self.state,
        );

        match outcome {
            NavigationStatusOutcome::PointerOutside => clear_status_display(
                services.text_presentation_mut(),
                &mut lifecycle.presentation.subtitle_word_list_mode,
            ),
            NavigationStatusOutcome::Composed { .. } => {
                publish_status_display(
                    &self.state,
                    services.text_presentation_mut(),
                    &mut lifecycle.presentation.subtitle_word_list_mode,
                )?;
                // DS and GS address the same game-data byte in the shipped
                // runtime. OR-ing pending and then incrementing it changes
                // mode 1 into visible mode 2 after composition.
                self.state.hover.pending = false;
                self.state.hover.visible = true;
            }
            NavigationStatusOutcome::TransitionPending
            | NavigationStatusOutcome::CameraViewActive
            | NavigationStatusOutcome::PresentationActive
            | NavigationStatusOutcome::HoverEntityDisabled
            | NavigationStatusOutcome::AlreadyVisible => {}
        }
        Ok(outcome)
    }
}

fn clear_status_display(presentation: &mut TextPresentationState, lifecycle_mode: &mut bool) {
    presentation.subtitle_word_list_mode = false;
    *lifecycle_mode = false;
}

fn publish_status_display(
    state: &NavigationStatusState,
    presentation: &mut TextPresentationState,
    lifecycle_mode: &mut bool,
) -> Result<()> {
    let text = state
        .text
        .as_ref()
        .context("composed navigation status has no text")?;
    presentation.subtitle_text = serialize_status_text(text);
    presentation.subtitle_reveal_cursor = None;
    presentation.subtitle_word_list_mode = true;
    *lifecycle_mode = true;
    Ok(())
}

fn serialize_status_text(text: &NavigationStatusText) -> Box<[u8]> {
    let byte_count = text
        .lines()
        .iter()
        .map(|line| line.len().saturating_add(1))
        .sum();
    let mut bytes = Vec::with_capacity(byte_count);
    for line in text.lines() {
        bytes.extend_from_slice(line);
        bytes.push(CARRIAGE_RETURN);
    }
    bytes.into_boxed_slice()
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::script::ScriptObjectKind;

    use super::*;
    use crate::native::bloodprg::{
        GamePresentationScheduler, NavigationStatusLocationKind, NavigationStatusSource,
    };

    const LOCATION: u16 = 1;
    const ARK: u16 = 2;

    #[test]
    fn composed_status_publishes_cr_lines_and_visible_mode() {
        let mut status = NavigationStatusState::default();
        let sources = [NavigationStatusSource {
            kind: ScriptObjectKind::Actor,
            active: true,
            life_support_visits: 1,
            location: LOCATION,
            name: b"IZWALITO",
        }];
        let outcome = update_navigation_status(
            NavigationStatusContext {
                transition_pending: false,
                camera_view_active: false,
                presentation_active: false,
                hover_entity_enabled: true,
                hover_region: NavigationStatusRegion::new([10, 20], [30, 40]),
                pointer: [10, 20],
                location: NavigationStatusLocation {
                    kind: NavigationStatusLocationKind::Planet,
                    name: b"PTERRA",
                },
                sources: &sources,
                ark_location: ARK,
                labels: NavigationStatusLabels {
                    planet: b"PLANET: ",
                    ship: b"SHIP: ",
                    black_hole: b"BLACK HOLE: ",
                    life_support: b"LIFE SUPPORT:",
                },
            },
            &mut status,
        );
        assert_eq!(
            outcome,
            NavigationStatusOutcome::Composed {
                life_support_count: 1
            }
        );

        let mut presentation = TextPresentationState::default();
        let mut lifecycle_mode = false;
        publish_status_display(&status, &mut presentation, &mut lifecycle_mode).unwrap();
        assert_eq!(
            presentation.subtitle_text.as_ref(),
            b"PLANET: PTERRA\rLIFE SUPPORT:\rIZWALITO\r\r"
        );
        assert_eq!(presentation.subtitle_reveal_cursor, None);
        assert!(presentation.subtitle_word_list_mode);
        assert!(lifecycle_mode);
    }

    #[test]
    fn pointer_exit_clears_only_the_shared_status_mode() {
        let mut presentation = TextPresentationState {
            subtitle_display_active: true,
            subtitle_word_list_mode: true,
            subtitle_text: Box::from(b"DIALOGUE\r".as_slice()),
            ..TextPresentationState::default()
        };
        let mut lifecycle = GamePresentationScheduler {
            subtitle_display_active: true,
            subtitle_word_list_mode: true,
            ..GamePresentationScheduler::default()
        };

        clear_status_display(&mut presentation, &mut lifecycle.subtitle_word_list_mode);

        assert!(!presentation.subtitle_word_list_mode);
        assert!(!lifecycle.subtitle_word_list_mode);
        assert!(presentation.subtitle_display_active);
        assert!(lifecycle.subtitle_display_active);
        assert_eq!(presentation.subtitle_text.as_ref(), b"DIALOGUE\r");
    }
}
