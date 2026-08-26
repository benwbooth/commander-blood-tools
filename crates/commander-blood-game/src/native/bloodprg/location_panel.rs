//! Location-information panel lifecycle, artwork, and text presentation.

use commander_blood_formats::script::ScriptObjectKind;

use super::{LocationPanelGeometryState, NavigationStatusLabels, NavigationStatusLocationKind};

const SOURCE_WIDTH_NUMERATOR: u16 = 14;
const SOURCE_WIDTH_SHIFT: u32 = 5;
const PANEL_TEXT_X: u16 = 110;
const PANEL_TITLE_Y: u16 = 25;
const PANEL_TEXT_ROW_HEIGHT: u16 = 10;
const PANEL_NAME_GAP: u16 = 6;
const PANEL_TITLE_COLOR: u8 = 238;
const PANEL_SOURCE_COLOR: u8 = 254;

/// One typed world-art lookup entry used by the location panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationPanelArtwork<'a, ResourceId> {
    /// Authored location name matched byte-for-byte.
    pub location_name: &'a [u8],
    /// Typed resource identifier; the DOS high-bit load flag is unnecessary.
    pub resource_id: ResourceId,
}

/// Selected world object shown in the panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationPanelLocation<'a, LocationId> {
    /// Stable world-object identity.
    pub id: LocationId,
    /// Semantic title category.
    pub kind: NavigationStatusLocationKind,
    /// Authored game-font name bytes.
    pub name: &'a [u8],
}

/// One source object considered for the life-support list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocationPanelSource {
    /// Decoded source-object kind.
    pub kind: ScriptObjectKind,
    /// Whether the object currently participates in world state.
    pub active: bool,
    /// Number of life-support visits recorded by the game.
    pub life_support_visits: u16,
    /// Authored game-font name bytes.
    pub name: Box<[u8]>,
}

/// Signed rectangle retained from the original logical framebuffer geometry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocationPanelRect {
    /// Left edge.
    pub x: i16,
    /// Top edge.
    pub y: i16,
    /// Width.
    pub width: i16,
    /// Height.
    pub height: i16,
}

/// Current and target panel rectangles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocationPanelRects {
    /// Steady-state panel rectangle.
    pub target: LocationPanelRect,
    /// Collapsed transition rectangle.
    pub current: LocationPanelRect,
}

/// Frame-counter state used by panel rectangle interpolation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocationPanelTransitionProgress {
    /// Current interpolation step.
    pub current: u8,
    /// Final interpolation step.
    pub total: u8,
}

impl LocationPanelTransitionProgress {
    const fn is_complete(self) -> bool {
        self.current == self.total
    }
}

/// Semantic panel phase replacing the native transition bit byte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocationPanelPhase {
    /// Panel is fully open and displaying status text.
    #[default]
    Steady,
    /// Panel artwork is expanding into place.
    Opening,
    /// Panel artwork is collapsing before release.
    Closing,
}

/// Mutable panel state shared by its dispatcher and geometry routine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocationInfoPanelState<LocationId> {
    /// Current lifecycle phase.
    pub phase: LocationPanelPhase,
    /// Whether the bridge still considers the panel active.
    pub active: bool,
    /// Scaling and placement inputs consumed by entity geometry.
    pub geometry: LocationPanelGeometryState,
    /// Rectangle interpolation progress.
    pub transition: LocationPanelTransitionProgress,
    /// Selected world object, cleared after closing completes.
    pub selected_location: Option<LocationId>,
    /// Deferred navigation link, cleared with the selected object.
    pub deferred_record_link: Option<LocationId>,
}

impl<LocationId> Default for LocationInfoPanelState<LocationId> {
    fn default() -> Self {
        Self {
            phase: LocationPanelPhase::default(),
            active: false,
            geometry: LocationPanelGeometryState::default(),
            transition: LocationPanelTransitionProgress::default(),
            selected_location: None,
            deferred_record_link: None,
        }
    }
}

/// Sprite range dirtied by one panel frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocationPanelSpriteRange {
    /// Only the panel sprite itself.
    PanelOnly,
    /// Panel sprite and its adjacent transition sprite.
    PanelAndTransition,
}

/// Direction of rectangle interpolation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocationPanelInterpolation {
    /// Expand from the collapsed rectangle to the target rectangle.
    Opening,
    /// Collapse from the target rectangle to the collapsed rectangle.
    Closing,
}

/// One text draw emitted by the steady panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationPanelTextDraw<'a> {
    /// Authored game-font bytes.
    pub text: &'a [u8],
    /// Original logical pixel origin.
    pub position: [u16; 2],
    /// Original indexed-palette color.
    pub color: u8,
}

/// Resource, world-query, geometry, and rendering operations used by the panel.
pub trait LocationInfoPanelHost<ResourceId, LocationId, ComparisonExtent> {
    /// Loaded artwork representation owned by the backend.
    type Artwork;
    /// Resource or world-query failure.
    type Error;

    /// Load the typed panel artwork resource.
    fn load_panel_artwork(
        &mut self,
        resource_id: &ResourceId,
    ) -> Result<Self::Artwork, Self::Error>;

    /// Install artwork on the panel entity and return its source stride.
    fn install_panel_artwork(&mut self, artwork: Self::Artwork, pointer: [u16; 2]) -> u16;

    /// Build the panel's dark palette-remap table.
    fn prepare_panel_palette(&mut self);

    /// Apply the already-translated scale and placement geometry step.
    fn update_panel_geometry(
        &mut self,
        geometry: &mut LocationPanelGeometryState,
        comparison_extent: &ComparisonExtent,
    );

    /// Render the dirty panel sprite range.
    fn render_panel_sprites(&mut self, range: LocationPanelSpriteRange);

    /// Advance rectangle interpolation in the selected direction.
    fn interpolate_panel(
        &mut self,
        direction: LocationPanelInterpolation,
        rects: LocationPanelRects,
        progress: &mut LocationPanelTransitionProgress,
    );

    /// Apply the steady-state palette remap to the panel rectangle.
    fn remap_panel_rect(&mut self, rect: LocationPanelRect);

    /// Draw game-font text and return the resulting width.
    fn draw_panel_text(&mut self, draw: LocationPanelTextDraw<'_>) -> u16;

    /// Resolve current navigation sources for the selected location.
    fn navigation_sources(
        &mut self,
        location: &LocationId,
    ) -> Result<Vec<LocationPanelSource>, Self::Error>;

    /// Release the panel entity after its closing transition.
    fn release_panel_entity(&mut self);
}

/// Read-only values consumed by one panel dispatcher call.
#[derive(Clone, Copy, Debug)]
pub struct LocationInfoPanelContext<'a, ResourceId, LocationId, ComparisonExtent> {
    /// Selected location and display name.
    pub selected: &'a LocationPanelLocation<'a, LocationId>,
    /// Ordered world-art table.
    pub artwork: &'a [LocationPanelArtwork<'a, ResourceId>],
    /// Authored title and life-support labels.
    pub labels: NavigationStatusLabels<'a>,
    /// Current and target interpolation rectangles.
    pub rects: LocationPanelRects,
    /// Current logical mouse position used when installing artwork.
    pub pointer: [u16; 2],
    /// Primary mouse edge that begins closing from steady state.
    pub primary_pressed: bool,
    /// Inherited entity extent context.
    pub comparison_extent: &'a ComparisonExtent,
}

/// Observable lifecycle result of one panel dispatcher call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocationInfoPanelOutcome {
    /// Opening animation remains in progress.
    Opening {
        /// Whether this frame loaded and installed matching artwork.
        artwork_installed: bool,
    },
    /// Steady panel text was drawn.
    Steady {
        /// Whether opening completed earlier in this same call.
        opening_completed: bool,
        /// Number of eligible life-support names drawn.
        life_support_count: usize,
    },
    /// Closing animation remains in progress.
    Closing {
        /// Whether a mouse edge initiated closing in this call.
        initiated: bool,
        /// Whether opening completed before that same mouse edge.
        opening_completed: bool,
    },
    /// Closing completed and the entity and world links were released.
    Closed {
        /// Whether a mouse edge initiated and completed closing in this call.
        initiated: bool,
        /// Whether opening completed before closing began in this call.
        opening_completed: bool,
    },
}

/// Update location-panel artwork, animation, labels, and close lifecycle.
///
/// This translates `location_info_panel_dispatch` at BLOODPRG routine offset
/// `0x009083`. Typed resources and world objects replace record offsets, far
/// strings, the terminated art table, and a stack-owned source list. Explicit
/// phases, rectangles, progress, and renderer operations replace packed state
/// bytes and framebuffer globals while preserving helper order and wrapping
/// scale arithmetic.
pub fn update_location_info_panel<ResourceId, LocationId, ComparisonExtent, Host>(
    context: LocationInfoPanelContext<'_, ResourceId, LocationId, ComparisonExtent>,
    state: &mut LocationInfoPanelState<LocationId>,
    host: &mut Host,
) -> Result<LocationInfoPanelOutcome, Host::Error>
where
    Host: LocationInfoPanelHost<ResourceId, LocationId, ComparisonExtent>,
{
    let mut opening_completed = false;
    if state.phase == LocationPanelPhase::Opening {
        let artwork_installed = if state.geometry.scale_step == u8::MIN {
            install_matching_artwork(&context, state, host)?
        } else {
            false
        };

        state.geometry.scale_step = state.geometry.scale_step.wrapping_add(1);
        host.update_panel_geometry(&mut state.geometry, context.comparison_extent);
        host.render_panel_sprites(LocationPanelSpriteRange::PanelAndTransition);
        let interpolation_complete = state.transition.is_complete();
        host.interpolate_panel(
            LocationPanelInterpolation::Opening,
            context.rects,
            &mut state.transition,
        );
        if !interpolation_complete {
            return Ok(LocationInfoPanelOutcome::Opening { artwork_installed });
        }
        state.phase = LocationPanelPhase::Steady;
        opening_completed = true;
    }

    if state.phase == LocationPanelPhase::Steady {
        if !context.primary_pressed {
            let life_support_count = draw_steady_panel(&context, host)?;
            return Ok(LocationInfoPanelOutcome::Steady {
                opening_completed,
                life_support_count,
            });
        }

        state.active = false;
        state.phase = LocationPanelPhase::Closing;
        state.transition.current = u8::MIN;
        state.geometry.scale_step = state.geometry.scale_step.wrapping_add(1);
        return Ok(close_panel_step(
            context,
            state,
            host,
            true,
            opening_completed,
        ));
    }

    Ok(close_panel_step(
        context,
        state,
        host,
        false,
        opening_completed,
    ))
}

fn install_matching_artwork<ResourceId, LocationId, ComparisonExtent, Host>(
    context: &LocationInfoPanelContext<'_, ResourceId, LocationId, ComparisonExtent>,
    state: &mut LocationInfoPanelState<LocationId>,
    host: &mut Host,
) -> Result<bool, Host::Error>
where
    Host: LocationInfoPanelHost<ResourceId, LocationId, ComparisonExtent>,
{
    let Some(entry) = context
        .artwork
        .iter()
        .find(|entry| entry.location_name == context.selected.name)
    else {
        return Ok(false);
    };

    let artwork = host.load_panel_artwork(&entry.resource_id)?;
    let source_stride = host.install_panel_artwork(artwork, context.pointer);
    state.geometry.source_width = u16::from(source_stride as u8)
        .wrapping_mul(SOURCE_WIDTH_NUMERATOR)
        .wrapping_shr(SOURCE_WIDTH_SHIFT);
    host.prepare_panel_palette();
    Ok(true)
}

fn draw_steady_panel<ResourceId, LocationId, ComparisonExtent, Host>(
    context: &LocationInfoPanelContext<'_, ResourceId, LocationId, ComparisonExtent>,
    host: &mut Host,
) -> Result<usize, Host::Error>
where
    Host: LocationInfoPanelHost<ResourceId, LocationId, ComparisonExtent>,
{
    host.render_panel_sprites(LocationPanelSpriteRange::PanelOnly);
    host.remap_panel_rect(context.rects.target);

    let title = match context.selected.kind {
        NavigationStatusLocationKind::Planet => context.labels.planet,
        NavigationStatusLocationKind::Ship => context.labels.ship,
        NavigationStatusLocationKind::BlackHole => context.labels.black_hole,
    };
    let title_width = host.draw_panel_text(LocationPanelTextDraw {
        text: title,
        position: [PANEL_TEXT_X, PANEL_TITLE_Y],
        color: PANEL_TITLE_COLOR,
    });
    host.draw_panel_text(LocationPanelTextDraw {
        text: context.selected.name,
        position: [
            PANEL_TEXT_X
                .wrapping_add(title_width)
                .wrapping_add(PANEL_NAME_GAP),
            PANEL_TITLE_Y,
        ],
        color: PANEL_TITLE_COLOR,
    });
    host.draw_panel_text(LocationPanelTextDraw {
        text: context.labels.life_support,
        position: [
            PANEL_TEXT_X,
            PANEL_TITLE_Y.wrapping_add(PANEL_TEXT_ROW_HEIGHT),
        ],
        color: PANEL_TITLE_COLOR,
    });

    let sources = host.navigation_sources(&context.selected.id)?;
    let mut text_y = PANEL_TITLE_Y.wrapping_add(PANEL_TEXT_ROW_HEIGHT.wrapping_mul(2));
    let mut life_support_count = usize::MIN;
    for source in sources {
        if source.kind != ScriptObjectKind::Actor
            || !source.active
            || source.life_support_visits == u16::MIN
        {
            continue;
        }
        host.draw_panel_text(LocationPanelTextDraw {
            text: &source.name,
            position: [PANEL_TEXT_X, text_y],
            color: PANEL_SOURCE_COLOR,
        });
        text_y = text_y.wrapping_add(PANEL_TEXT_ROW_HEIGHT);
        life_support_count += 1;
    }
    Ok(life_support_count)
}

fn close_panel_step<ResourceId, LocationId, ComparisonExtent, Host>(
    context: LocationInfoPanelContext<'_, ResourceId, LocationId, ComparisonExtent>,
    state: &mut LocationInfoPanelState<LocationId>,
    host: &mut Host,
    initiated: bool,
    opening_completed: bool,
) -> LocationInfoPanelOutcome
where
    Host: LocationInfoPanelHost<ResourceId, LocationId, ComparisonExtent>,
{
    state.geometry.scale_step = state.geometry.scale_step.wrapping_sub(1);
    host.update_panel_geometry(&mut state.geometry, context.comparison_extent);
    host.render_panel_sprites(LocationPanelSpriteRange::PanelAndTransition);
    let interpolation_complete = state.transition.is_complete();
    host.interpolate_panel(
        LocationPanelInterpolation::Closing,
        context.rects,
        &mut state.transition,
    );
    if !interpolation_complete {
        return LocationInfoPanelOutcome::Closing {
            initiated,
            opening_completed,
        };
    }

    host.release_panel_entity();
    state.phase = LocationPanelPhase::Steady;
    state.selected_location = None;
    state.deferred_record_link = None;
    LocationInfoPanelOutcome::Closed {
        initiated,
        opening_completed,
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const SELECTED_LOCATION: u16 = 6_144;
    const DEFERRED_LOCATION: u16 = 27_242;
    const INITIAL_SOURCE_WIDTH: u16 = 777;
    const MATCHING_RESOURCE: u16 = 94;
    const MATCHING_SOURCE_STRIDE: u16 = 303;
    const TARGET_RECT: LocationPanelRect = LocationPanelRect {
        x: 110,
        y: 25,
        width: 96,
        height: 70,
    };
    const CURRENT_RECT: LocationPanelRect = LocationPanelRect {
        x: 123,
        y: 77,
        width: 4,
        height: 4,
    };

    #[derive(Deserialize)]
    struct PanelVector {
        name: String,
        state_before: u8,
        state_after: u8,
        scale_before: u8,
        scale_after: u8,
        mouse: u8,
        interpolation_complete: bool,
        calls: Vec<OracleCall>,
    }

    #[derive(Deserialize)]
    struct OracleCall {
        name: String,
        text: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum HostEvent {
        Resource(u16),
        Install {
            resource: u16,
            pointer: [u16; 2],
        },
        Palette,
        Geometry,
        Render(LocationPanelSpriteRange),
        Interpolate(LocationPanelInterpolation),
        Remap(LocationPanelRect),
        Text {
            text: Box<[u8]>,
            position: [u16; 2],
            color: u8,
        },
        SourceList(u16),
        Release,
    }

    struct OracleHost {
        events: Vec<HostEvent>,
        sources: Vec<LocationPanelSource>,
    }

    impl LocationInfoPanelHost<u16, u16, ()> for OracleHost {
        type Artwork = u16;
        type Error = std::convert::Infallible;

        fn load_panel_artwork(&mut self, resource_id: &u16) -> Result<u16, Self::Error> {
            self.events.push(HostEvent::Resource(*resource_id));
            Ok(*resource_id)
        }

        fn install_panel_artwork(&mut self, artwork: u16, pointer: [u16; 2]) -> u16 {
            self.events.push(HostEvent::Install {
                resource: artwork,
                pointer,
            });
            MATCHING_SOURCE_STRIDE
        }

        fn prepare_panel_palette(&mut self) {
            self.events.push(HostEvent::Palette);
        }

        fn update_panel_geometry(
            &mut self,
            _geometry: &mut LocationPanelGeometryState,
            _comparison_extent: &(),
        ) {
            self.events.push(HostEvent::Geometry);
        }

        fn render_panel_sprites(&mut self, range: LocationPanelSpriteRange) {
            self.events.push(HostEvent::Render(range));
        }

        fn interpolate_panel(
            &mut self,
            direction: LocationPanelInterpolation,
            rects: LocationPanelRects,
            _progress: &mut LocationPanelTransitionProgress,
        ) {
            assert_eq!(rects, panel_rects());
            self.events.push(HostEvent::Interpolate(direction));
        }

        fn remap_panel_rect(&mut self, rect: LocationPanelRect) {
            self.events.push(HostEvent::Remap(rect));
        }

        fn draw_panel_text(&mut self, draw: LocationPanelTextDraw<'_>) -> u16 {
            self.events.push(HostEvent::Text {
                text: Box::from(draw.text),
                position: draw.position,
                color: draw.color,
            });
            u16::try_from(draw.text.len()).unwrap().wrapping_mul(5)
        }

        fn navigation_sources(
            &mut self,
            location: &u16,
        ) -> Result<Vec<LocationPanelSource>, Self::Error> {
            self.events.push(HostEvent::SourceList(*location));
            Ok(std::mem::take(&mut self.sources))
        }

        fn release_panel_entity(&mut self) {
            self.events.push(HostEvent::Release);
        }
    }

    #[test]
    fn panel_dispatch_matches_every_original_vector() {
        let vectors: Vec<PanelVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_9083_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 11);

        for vector in vectors {
            let selected_name: &[u8] = if vector.name.contains("missing_art") {
                b"MISSING"
            } else {
                b"TARGET"
            };
            let selected = LocationPanelLocation {
                id: SELECTED_LOCATION,
                kind: selected_kind(&vector.name),
                name: selected_name,
            };
            let artwork = [
                LocationPanelArtwork {
                    location_name: b"OTHER",
                    resource_id: 32,
                },
                LocationPanelArtwork {
                    location_name: b"TARGET",
                    resource_id: MATCHING_RESOURCE,
                },
            ];
            let mut state = LocationInfoPanelState {
                phase: decode_phase(vector.state_before),
                active: true,
                geometry: LocationPanelGeometryState {
                    scale_step: vector.scale_before,
                    source_width: INITIAL_SOURCE_WIDTH,
                    ..LocationPanelGeometryState::default()
                },
                transition: LocationPanelTransitionProgress {
                    current: if vector.interpolation_complete { 8 } else { 3 },
                    total: 8,
                },
                selected_location: Some(SELECTED_LOCATION),
                deferred_record_link: Some(DEFERRED_LOCATION),
            };
            let context = LocationInfoPanelContext {
                selected: &selected,
                artwork: &artwork,
                labels: NavigationStatusLabels {
                    planet: b"PLANET: ",
                    ship: b"SHIP: ",
                    black_hole: b"BLACK HOLE: ",
                    life_support: b"LIFE SUPPORT:",
                },
                rects: panel_rects(),
                pointer: [123, 77],
                primary_pressed: vector.mouse & 1 != u8::MIN,
                comparison_extent: &(),
            };
            let mut host = OracleHost {
                events: Vec::new(),
                sources: if vector.name.contains("eligible_life_support") {
                    source_objects()
                } else {
                    Vec::new()
                },
            };

            let outcome = update_location_info_panel(context, &mut state, &mut host).unwrap();

            assert_event_names(&vector, &host.events);
            assert_text_calls(&vector, &host.events);
            assert_state(&vector, &state, outcome);
            assert_event_details(&vector, &host.events);
        }
    }

    fn selected_kind(name: &str) -> NavigationStatusLocationKind {
        if name.contains("black_hole") {
            NavigationStatusLocationKind::BlackHole
        } else if name.contains("ship_title") {
            NavigationStatusLocationKind::Ship
        } else {
            NavigationStatusLocationKind::Planet
        }
    }

    fn decode_phase(bits: u8) -> LocationPanelPhase {
        if bits & 1 != u8::MIN {
            LocationPanelPhase::Opening
        } else if bits & 2 != u8::MIN {
            LocationPanelPhase::Closing
        } else {
            LocationPanelPhase::Steady
        }
    }

    fn panel_rects() -> LocationPanelRects {
        LocationPanelRects {
            target: TARGET_RECT,
            current: CURRENT_RECT,
        }
    }

    fn source_objects() -> Vec<LocationPanelSource> {
        vec![
            source(ScriptObjectKind::Actor, true, 1, b"ELIGIBLE"),
            source(ScriptObjectKind::CelestialBody, true, 1, b"WRONGKIND"),
            source(ScriptObjectKind::Actor, false, 1, b"INACTIVE"),
            source(ScriptObjectKind::Actor, true, 0, b"UNSEEN"),
        ]
    }

    fn source(
        kind: ScriptObjectKind,
        active: bool,
        life_support_visits: u16,
        name: &[u8],
    ) -> LocationPanelSource {
        LocationPanelSource {
            kind,
            active,
            life_support_visits,
            name: Box::from(name),
        }
    }

    fn event_name(event: &HostEvent) -> &'static str {
        match event {
            HostEvent::Resource(_) => "resource",
            HostEvent::Install { .. } => "setter",
            HostEvent::Palette => "palette",
            HostEvent::Geometry => "entity",
            HostEvent::Render(_) => "render",
            HostEvent::Interpolate(_) => "interpolate",
            HostEvent::Remap(_) => "remap",
            HostEvent::Text { .. } => "text",
            HostEvent::SourceList(_) => "source_list",
            HostEvent::Release => "transition",
        }
    }

    fn assert_event_names(vector: &PanelVector, events: &[HostEvent]) {
        let expected: Vec<&str> = vector
            .calls
            .iter()
            .filter(|call| call.name != "compare")
            .map(|call| call.name.as_str())
            .collect();
        let actual: Vec<&str> = events.iter().map(event_name).collect();
        assert_eq!(actual, expected, "{}", vector.name);
    }

    fn assert_text_calls(vector: &PanelVector, events: &[HostEvent]) {
        let expected: Vec<&str> = vector
            .calls
            .iter()
            .filter_map(|call| call.text.as_deref())
            .collect();
        let actual: Vec<&[u8]> = events
            .iter()
            .filter_map(|event| match event {
                HostEvent::Text { text, .. } => Some(text.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(actual.len(), expected.len(), "{}", vector.name);
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert_eq!(actual, expected.as_bytes(), "{}", vector.name);
        }
    }

    fn assert_state(
        vector: &PanelVector,
        state: &LocationInfoPanelState<u16>,
        outcome: LocationInfoPanelOutcome,
    ) {
        assert_eq!(
            state.phase,
            decode_phase(vector.state_after),
            "{}",
            vector.name
        );
        assert_eq!(
            state.geometry.scale_step, vector.scale_after,
            "{}",
            vector.name
        );
        assert_eq!(state.active, vector.mouse & 1 == u8::MIN, "{}", vector.name);

        let closed = vector.name == "closing_completion_releases_entity_and_links";
        assert_eq!(state.selected_location.is_none(), closed, "{}", vector.name);
        assert_eq!(
            state.deferred_record_link.is_none(),
            closed,
            "{}",
            vector.name
        );
        let installed = vector.name == "first_open_frame_scans_to_second_art_entry";
        assert_eq!(
            state.geometry.source_width,
            if installed { 20 } else { INITIAL_SOURCE_WIDTH },
            "{}",
            vector.name
        );
        assert_eq!(
            state.transition.current,
            if vector.mouse & 1 != u8::MIN {
                u8::MIN
            } else if vector.interpolation_complete {
                8
            } else {
                3
            },
            "{}",
            vector.name
        );

        let expected_outcome = match vector.name.as_str() {
            "opening_continues_without_repeating_setup" => LocationInfoPanelOutcome::Opening {
                artwork_installed: false,
            },
            "first_open_frame_scans_to_second_art_entry" => LocationInfoPanelOutcome::Opening {
                artwork_installed: true,
            },
            "first_open_frame_tolerates_missing_art_entry" => LocationInfoPanelOutcome::Opening {
                artwork_installed: false,
            },
            "opening_completion_enters_steady_state" => LocationInfoPanelOutcome::Steady {
                opening_completed: true,
                life_support_count: 0,
            },
            "closing_decrements_scale_and_waits" => LocationInfoPanelOutcome::Closing {
                initiated: false,
                opening_completed: false,
            },
            "closing_completion_releases_entity_and_links" => LocationInfoPanelOutcome::Closed {
                initiated: false,
                opening_completed: false,
            },
            "mouse_close_transition_preserves_current_scale" => LocationInfoPanelOutcome::Closing {
                initiated: true,
                opening_completed: false,
            },
            "steady_planet_draws_only_eligible_life_support_source" => {
                LocationInfoPanelOutcome::Steady {
                    opening_completed: false,
                    life_support_count: 1,
                }
            }
            _ => LocationInfoPanelOutcome::Steady {
                opening_completed: false,
                life_support_count: 0,
            },
        };
        assert_eq!(outcome, expected_outcome, "{}", vector.name);
    }

    fn assert_event_details(vector: &PanelVector, events: &[HostEvent]) {
        let actual_render_ranges: Vec<LocationPanelSpriteRange> = events
            .iter()
            .filter_map(|event| match event {
                HostEvent::Render(range) => Some(*range),
                _ => None,
            })
            .collect();
        let expected_render_ranges = if vector.name == "opening_completion_enters_steady_state" {
            vec![
                LocationPanelSpriteRange::PanelAndTransition,
                LocationPanelSpriteRange::PanelOnly,
            ]
        } else if vector.state_before & 3 != u8::MIN || vector.mouse & 1 != u8::MIN {
            vec![LocationPanelSpriteRange::PanelAndTransition]
        } else {
            vec![LocationPanelSpriteRange::PanelOnly]
        };
        assert_eq!(
            actual_render_ranges, expected_render_ranges,
            "{}",
            vector.name
        );

        for event in events {
            match event {
                HostEvent::Resource(resource) => {
                    assert_eq!(*resource, MATCHING_RESOURCE, "{}", vector.name);
                }
                HostEvent::Install { resource, pointer } => {
                    assert_eq!(*resource, MATCHING_RESOURCE, "{}", vector.name);
                    assert_eq!(*pointer, [123, 77], "{}", vector.name);
                }
                HostEvent::Interpolate(direction) => {
                    let expected =
                        if vector.state_before & 2 != u8::MIN || vector.mouse & 1 != u8::MIN {
                            LocationPanelInterpolation::Closing
                        } else {
                            LocationPanelInterpolation::Opening
                        };
                    assert_eq!(*direction, expected, "{}", vector.name);
                }
                HostEvent::Remap(rect) => assert_eq!(*rect, TARGET_RECT, "{}", vector.name),
                HostEvent::Text {
                    text,
                    position,
                    color,
                } => {
                    let expected = match text.as_ref() {
                        b"PLANET: " | b"SHIP: " | b"BLACK HOLE: " => {
                            ([PANEL_TEXT_X, PANEL_TITLE_Y], PANEL_TITLE_COLOR)
                        }
                        b"TARGET" => {
                            let title_len = match selected_kind(&vector.name) {
                                NavigationStatusLocationKind::Planet => 8,
                                NavigationStatusLocationKind::Ship => 6,
                                NavigationStatusLocationKind::BlackHole => 12,
                            };
                            (
                                [PANEL_TEXT_X + title_len * 5 + PANEL_NAME_GAP, PANEL_TITLE_Y],
                                PANEL_TITLE_COLOR,
                            )
                        }
                        b"LIFE SUPPORT:" => (
                            [PANEL_TEXT_X, PANEL_TITLE_Y + PANEL_TEXT_ROW_HEIGHT],
                            PANEL_TITLE_COLOR,
                        ),
                        b"ELIGIBLE" => (
                            [PANEL_TEXT_X, PANEL_TITLE_Y + PANEL_TEXT_ROW_HEIGHT * 2],
                            PANEL_SOURCE_COLOR,
                        ),
                        _ => continue,
                    };
                    assert_eq!((*position, *color), expected, "{}", vector.name);
                }
                HostEvent::SourceList(location) => {
                    assert_eq!(*location, SELECTED_LOCATION, "{}", vector.name);
                }
                HostEvent::Palette
                | HostEvent::Geometry
                | HostEvent::Render(_)
                | HostEvent::Release => {}
            }
        }
    }
}
