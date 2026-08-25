//! Bridge presentation hover selection over typed logical rectangles.

/// One rectangle whose right and bottom edges are inclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationHitRectangle {
    origin: [i16; 2],
    extent: [i16; 2],
}

impl PresentationHitRectangle {
    /// Build a logical hit rectangle from its upper-left origin and dimensions.
    pub const fn new(origin: [i16; 2], extent: [i16; 2]) -> Self {
        Self { origin, extent }
    }

    /// Return the upper-left logical coordinate.
    pub const fn origin(self) -> [i16; 2] {
        self.origin
    }

    /// Return the recovered signed dimensions.
    pub const fn extent(self) -> [i16; 2] {
        self.extent
    }

    /// Test the original inclusive bounds with its wrapping coordinate math.
    pub const fn contains(self, point: [i16; 2]) -> bool {
        coordinate_inside(point[0], self.origin[0], self.extent[0])
            && coordinate_inside(point[1], self.origin[1], self.extent[1])
    }
}

/// The two bridge regions selectable by presentation mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationHitAreas {
    primary: PresentationHitRectangle,
    secondary: PresentationHitRectangle,
}

impl PresentationHitAreas {
    /// Build the primary and secondary bridge hit areas.
    pub const fn new(
        primary: PresentationHitRectangle,
        secondary: PresentationHitRectangle,
    ) -> Self {
        Self { primary, secondary }
    }

    const fn selected(self, selection: PresentationHitSelection) -> PresentationHitRectangle {
        match selection {
            PresentationHitSelection::Primary => self.primary,
            PresentationHitSelection::Secondary => self.secondary,
        }
    }
}

/// Bridge hit area enabled for the current presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationHitSelection {
    /// The first authored actor region.
    Primary,
    /// The second authored actor region.
    Secondary,
}

/// Semantic hover ownership and actor state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationHoverState<ActorState> {
    active: bool,
    actor_state: ActorState,
    previous_actor_state: ActorState,
}

impl<ActorState> PresentationHoverState<ActorState> {
    /// Build hover state from explicit semantic actor states.
    pub const fn new(
        active: bool,
        actor_state: ActorState,
        previous_actor_state: ActorState,
    ) -> Self {
        Self {
            active,
            actor_state,
            previous_actor_state,
        }
    }

    /// Return whether the presentation hover currently owns actor state.
    pub const fn active(&self) -> bool {
        self.active
    }

    /// Return the actor state currently published to the bridge.
    pub const fn actor_state(&self) -> &ActorState {
        &self.actor_state
    }

    /// Replace the actor state restored when the hover ends.
    pub fn set_previous_actor_state(&mut self, previous_actor_state: ActorState) {
        self.previous_actor_state = previous_actor_state;
    }
}

/// State transition taken by one presentation hover update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationHoverOutcome {
    /// No presentation hit area is enabled; state was untouched.
    Disabled,
    /// The point remained inside an already active hover area.
    RemainedInside,
    /// Entering the selected rectangle activated presentation hover.
    Activated,
    /// The point remained outside while presentation hover was inactive.
    RemainedOutside,
    /// Leaving the selected rectangle restored the previous actor state.
    Deactivated,
}

/// Update bridge presentation hover for the selected logical rectangle.
///
/// This translates `presentation_mode_dispatch` at BLOODPRG routine offset
/// `0x0078D0`. A semantic optional selection replaces the native UI mode bits,
/// and `active` replaces the activity flag byte while preserving exact bounds.
pub fn update_presentation_hover<ActorState: Clone>(
    selection: Option<PresentationHitSelection>,
    hit_areas: PresentationHitAreas,
    point: [i16; 2],
    hovering_actor_state: ActorState,
    state: &mut PresentationHoverState<ActorState>,
) -> PresentationHoverOutcome {
    let Some(selection) = selection else {
        return PresentationHoverOutcome::Disabled;
    };

    if hit_areas.selected(selection).contains(point) {
        if state.active {
            PresentationHoverOutcome::RemainedInside
        } else {
            state.active = true;
            state.actor_state = hovering_actor_state;
            PresentationHoverOutcome::Activated
        }
    } else if state.active {
        state.active = false;
        state.actor_state = state.previous_actor_state.clone();
        PresentationHoverOutcome::Deactivated
    } else {
        PresentationHoverOutcome::RemainedOutside
    }
}

const fn coordinate_inside(point: i16, origin: i16, extent: i16) -> bool {
    point >= origin && point.wrapping_sub(extent) <= origin
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 20;
    const HOVERING_ACTOR_STATE: u16 = 9;
    const DEFAULT_PREVIOUS_ACTOR_STATE: u16 = 23;

    #[derive(Deserialize)]
    struct HoverOracle {
        name: String,
        ui: u8,
        selected_rect: String,
        point: [i16; 2],
        inside: Option<bool>,
        mode_before: u8,
        mode_after: u8,
        presentation_before: u16,
        presentation_after: u16,
    }

    #[test]
    fn hover_update_matches_every_original_semantic_vector() {
        let vectors: Vec<HoverOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_78d0_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let selection = selection_from_original_ui(vector.ui);
            assert_eq!(
                selected_rect_name(selection),
                vector.selected_rect,
                "{}",
                vector.name,
            );
            let hit_areas = oracle_hit_areas(&vector.name);
            if let (Some(selection), Some(expected_inside)) = (selection, vector.inside) {
                assert_eq!(
                    hit_areas.selected(selection).contains(vector.point),
                    expected_inside,
                    "{}",
                    vector.name,
                );
            }

            let active_before = vector.mode_before & 1 != u8::MIN;
            let previous_actor_state = if active_before && vector.inside == Some(false) {
                vector.presentation_after
            } else {
                DEFAULT_PREVIOUS_ACTOR_STATE
            };
            let mut state = PresentationHoverState::new(
                active_before,
                vector.presentation_before,
                previous_actor_state,
            );
            let outcome = update_presentation_hover(
                selection,
                hit_areas,
                vector.point,
                HOVERING_ACTOR_STATE,
                &mut state,
            );

            assert_eq!(
                outcome,
                expected_outcome(selection, vector.inside, active_before),
                "{}",
                vector.name,
            );
            assert_eq!(
                state.active(),
                vector.mode_after & 1 != u8::MIN,
                "{}",
                vector.name,
            );
            assert_eq!(
                *state.actor_state(),
                vector.presentation_after,
                "{}",
                vector.name,
            );
        }
    }

    #[test]
    fn prior_actor_state_can_change_between_hover_sessions() {
        let mut state = PresentationHoverState::new(false, 3_u8, 1_u8);
        state.set_previous_actor_state(7);
        assert_eq!(*state.actor_state(), 3);
    }

    fn selection_from_original_ui(ui: u8) -> Option<PresentationHitSelection> {
        const PRESENTATION_AREA_GATE: u8 = 0x50;
        const SECONDARY_AREA: u8 = 0x40;

        if ui & PRESENTATION_AREA_GATE == u8::MIN {
            None
        } else if ui & SECONDARY_AREA != u8::MIN {
            Some(PresentationHitSelection::Secondary)
        } else {
            Some(PresentationHitSelection::Primary)
        }
    }

    fn selected_rect_name(selection: Option<PresentationHitSelection>) -> &'static str {
        match selection {
            None => "none",
            Some(PresentationHitSelection::Primary) => "first",
            Some(PresentationHitSelection::Secondary) => "second",
        }
    }

    fn oracle_hit_areas(name: &str) -> PresentationHitAreas {
        let primary = match name {
            "signed_subtract_wrap_outside" | "signed_min_plus_one_inside" => {
                PresentationHitRectangle::new([i16::MIN, -10], [1, 20])
            }
            "signed_vertical_subtract_wrap_outside" => {
                PresentationHitRectangle::new([-10, i16::MIN], [20, 1])
            }
            _ => PresentationHitRectangle::new([100, 60], [30, 20]),
        };
        PresentationHitAreas::new(
            primary,
            PresentationHitRectangle::new([-120, -50], [40, 30]),
        )
    }

    fn expected_outcome(
        selection: Option<PresentationHitSelection>,
        inside: Option<bool>,
        active_before: bool,
    ) -> PresentationHoverOutcome {
        match (selection, inside, active_before) {
            (None, _, _) => PresentationHoverOutcome::Disabled,
            (Some(_), Some(true), false) => PresentationHoverOutcome::Activated,
            (Some(_), Some(true), true) => PresentationHoverOutcome::RemainedInside,
            (Some(_), Some(false), false) => PresentationHoverOutcome::RemainedOutside,
            (Some(_), Some(false), true) => PresentationHoverOutcome::Deactivated,
            (Some(_), None, _) => unreachable!(),
        }
    }
}
