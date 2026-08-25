//! Typed bridge navigation-status hover and text composition.

use commander_blood_formats::script::ScriptObjectKind;

const DISPLAY_GENERATION_STEP: u8 = 1;

/// Unsigned entity bounds used by the bridge navigation-status hover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationStatusRegion {
    origin: [u16; 2],
    extent: [u16; 2],
}

impl NavigationStatusRegion {
    /// Build a region from an unsigned entity origin and extent.
    pub const fn new(origin: [u16; 2], extent: [u16; 2]) -> Self {
        Self { origin, extent }
    }

    /// Return the unsigned entity origin.
    pub const fn origin(self) -> [u16; 2] {
        self.origin
    }

    /// Return the unsigned entity extent.
    pub const fn extent(self) -> [u16; 2] {
        self.extent
    }

    /// Test inclusive bounds using the original wrapping edge calculation.
    pub const fn contains(self, point: [u16; 2]) -> bool {
        coordinate_inside(point[0], self.origin[0], self.extent[0])
            && coordinate_inside(point[1], self.origin[1], self.extent[1])
    }
}

const fn coordinate_inside(point: u16, origin: u16, extent: u16) -> bool {
    point >= origin && point <= origin.wrapping_add(extent)
}

/// Status-title category selected from the decoded navigation object kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationStatusLocationKind {
    /// Planet or another ordinary celestial location.
    Planet,
    /// Ship navigation entity.
    Ship,
    /// Black-hole destination.
    BlackHole,
}

/// Current location shown by the bridge status hover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationStatusLocation<'a> {
    /// Semantic title category.
    pub kind: NavigationStatusLocationKind,
    /// Original game-font bytes for the authored location name.
    pub name: &'a [u8],
}

/// One descendant considered for the life-support roster.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationStatusSource<'a, LocationId> {
    /// Decoded source-object kind.
    pub kind: ScriptObjectKind,
    /// Whether the source object is active.
    pub active: bool,
    /// Number of recorded life-support visits.
    pub life_support_visits: u16,
    /// Current typed location relationship.
    pub location: LocationId,
    /// Original game-font bytes for the source name.
    pub name: &'a [u8],
}

/// Authored labels decoded from the original game resources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationStatusLabels<'a> {
    /// Prefix for an ordinary planet/location.
    pub planet: &'a [u8],
    /// Prefix for a ship location.
    pub ship: &'a [u8],
    /// Prefix for a black-hole location.
    pub black_hole: &'a [u8],
    /// Life-support roster heading.
    pub life_support: &'a [u8],
}

/// Read-only bridge and world inputs for one status update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationStatusContext<'a, LocationId> {
    /// A navigation transition currently suppresses hover status.
    pub transition_pending: bool,
    /// Camera view currently suppresses hover status.
    pub camera_view_active: bool,
    /// Another presentation currently suppresses hover status.
    pub presentation_active: bool,
    /// The status-hover entity is enabled.
    pub hover_entity_enabled: bool,
    /// Unsigned bounds carried by the hover entity.
    pub hover_region: NavigationStatusRegion,
    /// Current unsigned logical pointer position.
    pub pointer: [u16; 2],
    /// Current location resolved through Arche's typed relationship.
    pub location: NavigationStatusLocation<'a>,
    /// Typed descendants returned by navigation-source traversal.
    pub sources: &'a [NavigationStatusSource<'a, LocationId>],
    /// Typed location identity representing the Ark.
    pub ark_location: LocationId,
    /// Authored status labels.
    pub labels: NavigationStatusLabels<'a>,
}

/// Semantic bridge hover mode replacing packed status bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavigationStatusHoverMode {
    /// The pointer is currently inside the status entity.
    pub pending: bool,
    /// Existing status text is already visible and must not be recomposed.
    pub visible: bool,
}

/// Owned logical status lines in original game-font bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NavigationStatusText {
    lines: Vec<Box<[u8]>>,
}

impl NavigationStatusText {
    /// Return the title, heading, eligible names, and final blank line.
    pub fn lines(&self) -> &[Box<[u8]>] {
        &self.lines
    }
}

/// Mutable bridge navigation-status state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NavigationStatusState {
    /// Semantic hover state.
    pub hover: NavigationStatusHoverMode,
    /// Wrapping revision incremented whenever status text is recomposed.
    pub display_generation: u8,
    /// Currently composed logical status text.
    pub text: Option<NavigationStatusText>,
    /// Character reveal cursor into the current text.
    pub reveal_cursor: usize,
}

/// Terminal path taken by one navigation-status update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationStatusOutcome {
    /// A navigation transition suppresses status work.
    TransitionPending,
    /// Camera view suppresses status work.
    CameraViewActive,
    /// Another presentation suppresses status work.
    PresentationActive,
    /// The hover entity is disabled.
    HoverEntityDisabled,
    /// The pointer is outside the hover entity.
    PointerOutside,
    /// Existing visible status text was retained.
    AlreadyVisible,
    /// New status text was composed.
    Composed {
        /// Number of eligible life-support names appended.
        life_support_count: usize,
    },
}

/// Update bridge navigation hover state and compose location status text.
///
/// This translates `nav_state_gate` at BLOODPRG routine offset `0x0082E8`.
/// Decoded relationships and object kinds replace the record arena and offset
/// list. Logical owned lines replace the fixed NUL-terminated text buffer, and
/// independent semantic hover/display state replaces segment-owned aliases.
pub fn update_navigation_status<LocationId: PartialEq>(
    context: NavigationStatusContext<'_, LocationId>,
    state: &mut NavigationStatusState,
) -> NavigationStatusOutcome {
    if context.transition_pending {
        return NavigationStatusOutcome::TransitionPending;
    }
    if context.camera_view_active {
        return NavigationStatusOutcome::CameraViewActive;
    }
    if context.presentation_active {
        return NavigationStatusOutcome::PresentationActive;
    }
    if !context.hover_entity_enabled {
        return NavigationStatusOutcome::HoverEntityDisabled;
    }
    if !context.hover_region.contains(context.pointer) {
        state.hover = NavigationStatusHoverMode::default();
        return NavigationStatusOutcome::PointerOutside;
    }

    state.hover.pending = true;
    if state.hover.visible {
        return NavigationStatusOutcome::AlreadyVisible;
    }

    let title = match context.location.kind {
        NavigationStatusLocationKind::Planet => context.labels.planet,
        NavigationStatusLocationKind::Ship => context.labels.ship,
        NavigationStatusLocationKind::BlackHole => context.labels.black_hole,
    };
    let mut title_line = Vec::with_capacity(title.len() + context.location.name.len());
    title_line.extend_from_slice(title);
    title_line.extend_from_slice(context.location.name);

    let eligible_sources = context.sources.iter().filter(|source| {
        source.kind == ScriptObjectKind::Actor
            && source.active
            && source.life_support_visits != u16::MIN
            && source.location != context.ark_location
    });
    let mut lines = vec![
        title_line.into_boxed_slice(),
        context.labels.life_support.into(),
    ];
    let initial_line_count = lines.len();
    lines.extend(eligible_sources.map(|source| source.name.into()));
    let life_support_count = lines.len() - initial_line_count;
    lines.push(Box::default());

    state.text = Some(NavigationStatusText { lines });
    state.display_generation = state
        .display_generation
        .wrapping_add(DISPLAY_GENERATION_STEP);
    state.reveal_cursor = usize::MIN;
    NavigationStatusOutcome::Composed { life_support_count }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 13;
    const ARK_LOCATION: u16 = 30_583;
    const OTHER_LOCATION: u16 = 4_660;
    const SECOND_OTHER_LOCATION: u16 = 22_136;

    #[derive(Deserialize)]
    struct StatusOracle {
        name: String,
        terminal_path: String,
        transition: u8,
        camera: u8,
        presentation: u8,
        entity_flags: u16,
        rect: [u16; 4],
        mouse: [u16; 2],
        hit: Option<bool>,
        data_mode_before: u8,
        data_mode_after: u8,
        game_mode_before: u8,
        game_mode_after: u8,
        source_offsets: Vec<u16>,
        text_after: Option<String>,
        calls: Vec<serde_json::Value>,
    }

    #[test]
    fn status_gate_matches_every_original_semantic_vector() {
        let vectors: Vec<StatusOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_82e8_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let region = NavigationStatusRegion::new(
                [vector.rect[0], vector.rect[1]],
                [vector.rect[2], vector.rect[3]],
            );
            if let Some(hit) = vector.hit {
                assert_eq!(region.contains(vector.mouse), hit, "{}", vector.name);
            }

            let sources = sources_for(&vector);
            assert_eq!(
                sources.len(),
                vector.source_offsets.len(),
                "{}",
                vector.name
            );
            let initial_text = NavigationStatusText {
                lines: vec![Box::from(&b"PRESERVED"[..])],
            };
            let mut state = NavigationStatusState {
                hover: decode_hover_mode(vector.data_mode_before),
                display_generation: vector.game_mode_before,
                text: Some(initial_text.clone()),
                reveal_cursor: 37,
            };
            let outcome = update_navigation_status(
                NavigationStatusContext {
                    transition_pending: vector.transition & 1 != u8::MIN,
                    camera_view_active: vector.camera & 1 != u8::MIN,
                    presentation_active: vector.presentation & 1 != u8::MIN,
                    hover_entity_enabled: vector.entity_flags & 1 != u16::MIN,
                    hover_region: region,
                    pointer: vector.mouse,
                    location: NavigationStatusLocation {
                        kind: location_kind(&vector.name),
                        name: b"GAIA",
                    },
                    sources: &sources,
                    ark_location: ARK_LOCATION,
                    labels: NavigationStatusLabels {
                        planet: b"PLANET: ",
                        ship: b"SHIP: ",
                        black_hole: b"BLACK HOLE: ",
                        life_support: b"LIFE SUPPORT:",
                    },
                },
                &mut state,
            );

            assert_eq!(
                encode_hover_mode(state.hover),
                vector.data_mode_after & 3,
                "{}",
                vector.name
            );
            assert_eq!(
                state.display_generation, vector.game_mode_after,
                "{}",
                vector.name
            );
            assert!(
                outcome_matches_path(outcome, &vector.terminal_path),
                "{}: {outcome:?} did not match {}",
                vector.name,
                vector.terminal_path
            );

            if let Some(expected_text) = vector.text_after {
                assert_eq!(
                    original_text_bytes(state.text.as_ref().unwrap()),
                    expected_text.replace("\\r", "\r").as_bytes(),
                    "{}",
                    vector.name
                );
                assert_eq!(state.reveal_cursor, usize::MIN, "{}", vector.name);
                assert_eq!(vector.calls.len(), 1, "{}", vector.name);
            } else {
                assert_eq!(state.text, Some(initial_text), "{}", vector.name);
                assert_eq!(state.reveal_cursor, 37, "{}", vector.name);
                assert!(vector.calls.is_empty(), "{}", vector.name);
            }
        }
    }

    fn sources_for(vector: &StatusOracle) -> Vec<NavigationStatusSource<'static, u16>> {
        let all = [
            source(
                ScriptObjectKind::Actor,
                true,
                7,
                OTHER_LOCATION,
                b"ELIGIBLE",
            ),
            source(
                ScriptObjectKind::WorldState,
                true,
                7,
                OTHER_LOCATION,
                b"WRONGKIND",
            ),
            source(
                ScriptObjectKind::Actor,
                false,
                7,
                OTHER_LOCATION,
                b"INACTIVE",
            ),
            source(
                ScriptObjectKind::Actor,
                true,
                u16::MIN,
                OTHER_LOCATION,
                b"UNSEEN",
            ),
            source(ScriptObjectKind::Actor, true, 7, ARK_LOCATION, b"ONARK"),
            source(
                ScriptObjectKind::Actor,
                true,
                9,
                SECOND_OTHER_LOCATION,
                b"ACTIVEHIGH",
            ),
        ];
        all.into_iter().take(vector.source_offsets.len()).collect()
    }

    const fn source(
        kind: ScriptObjectKind,
        active: bool,
        life_support_visits: u16,
        location: u16,
        name: &'static [u8],
    ) -> NavigationStatusSource<'static, u16> {
        NavigationStatusSource {
            kind,
            active,
            life_support_visits,
            location,
            name,
        }
    }

    fn location_kind(name: &str) -> NavigationStatusLocationKind {
        match name {
            "exact_ship_kind_selects_ship_title" => NavigationStatusLocationKind::Ship,
            "black_hole_bit_overrides_ship_title" => NavigationStatusLocationKind::BlackHole,
            _ => NavigationStatusLocationKind::Planet,
        }
    }

    fn original_text_bytes(text: &NavigationStatusText) -> Vec<u8> {
        let mut bytes = Vec::new();
        for line in text.lines() {
            bytes.extend_from_slice(line);
            bytes.push(b'\r');
        }
        bytes.push(u8::MIN);
        bytes
    }

    const fn decode_hover_mode(mode: u8) -> NavigationStatusHoverMode {
        NavigationStatusHoverMode {
            pending: mode & 1 != u8::MIN,
            visible: mode & 2 != u8::MIN,
        }
    }

    const fn encode_hover_mode(mode: NavigationStatusHoverMode) -> u8 {
        mode.pending as u8 | (mode.visible as u8) << 1
    }

    fn outcome_matches_path(outcome: NavigationStatusOutcome, path: &str) -> bool {
        match outcome {
            NavigationStatusOutcome::TransitionPending => path == "transition_gate",
            NavigationStatusOutcome::CameraViewActive => path == "camera_gate",
            NavigationStatusOutcome::PresentationActive => path == "presentation_gate",
            NavigationStatusOutcome::HoverEntityDisabled => path == "entity_disabled",
            NavigationStatusOutcome::PointerOutside => {
                matches!(path, "x_below" | "x_above" | "y_below" | "y_above")
            }
            NavigationStatusOutcome::AlreadyVisible => path == "already_visible",
            NavigationStatusOutcome::Composed { .. } => path == "compose",
        }
    }
}
