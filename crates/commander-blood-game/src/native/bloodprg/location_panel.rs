//! Location-information panel lifecycle, artwork, and text presentation.

use commander_blood_formats::script::ScriptObjectKind;

use super::{LocationPanelGeometryState, NavigationStatusLabels, NavigationStatusLocationKind};

const SOURCE_WIDTH_NUMERATOR: u16 = 14;
const SOURCE_WIDTH_SHIFT: u32 = 5;
const PANEL_TEXT_X: u16 = 110;
const SEQUEL_PANEL_TITLE_X: u16 = 108;
const PANEL_TITLE_Y: u16 = 25;
const PANEL_TEXT_ROW_HEIGHT: u16 = 10;
const PANEL_NAME_GAP: u16 = 6;
const PANEL_TITLE_COLOR: u8 = 238;
const PANEL_SOURCE_COLOR: u8 = 254;
const SEQUEL_POPULATION_COLOR: u8 = 98;
const SEQUEL_AGGRESSIVENESS_COLOR: u8 = 252;
const SEQUEL_ENERGY_COLOR: u8 = 254;
const SEQUEL_EVOLUTION_COLOR: u8 = 96;
const SEQUEL_STAT_X: u16 = 205;
const SEQUEL_STAT_HEIGHT: u16 = 8;
const SEQUEL_STAT_DIVISOR: u16 = 20;
const SEQUEL_CANDIDATE_RIGHT: u16 = 250;

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
    /// Whether BBB should scan this object for nested location candidates.
    pub allows_sublocations: bool,
    /// Authored game-font name bytes.
    pub name: &'a [u8],
}

/// Numeric fields displayed for one sequel actor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationPanelActorDetails {
    /// Signed population value formatted as decimal text.
    pub population: i16,
    /// Signed aggressiveness value and raw-word bar source.
    pub aggressiveness: i16,
    /// Signed energy value, clamped at zero for its bar only.
    pub energy: i16,
    /// Signed evolution value and raw-word bar source.
    pub evolution: i16,
}

/// One source object considered for Commander life support or BBB details.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocationPanelSource<LocationId> {
    /// Stable source-object identity.
    pub id: LocationId,
    /// Decoded source-object kind.
    pub kind: ScriptObjectKind,
    /// Whether the object currently participates in world state.
    pub active: bool,
    /// Whether the object passes BBB's location-candidate participation gate.
    pub in_play: bool,
    /// Number of life-support visits recorded by the game.
    pub life_support_visits: u16,
    /// Authored game-font name bytes.
    pub name: Box<[u8]>,
    /// BBB actor statistics when the record carries its detail-display flag.
    pub actor_details: Option<LocationPanelActorDetails>,
}

/// BBB-only labels drawn in the detailed location panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationPanelDetailLabels<'a> {
    /// Selected sublocation title.
    pub location: &'a [u8],
    /// Leader-name label.
    pub leader: &'a [u8],
    /// Population value label.
    pub population: &'a [u8],
    /// Aggressiveness bar label.
    pub aggressiveness: &'a [u8],
    /// Energy bar label.
    pub energy: &'a [u8],
    /// Evolution bar label.
    pub evolution: &'a [u8],
}

/// Game-specific location-panel behavior and required data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocationPanelVariant<'a, LocationId> {
    /// Commander Blood's life-support roster and immediate close step.
    CommanderBlood,
    /// Big Bug Bang's sublocation chooser and actor statistics.
    BigBugBang {
        /// Arche binding excluded from the sublocation candidate list.
        excluded_location: LocationId,
        /// Exact executable-authored detail labels.
        labels: LocationPanelDetailLabels<'a>,
    },
}

/// Mutable pointer input consumed by one location-panel call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocationPanelInput {
    /// Current unsigned logical pointer position.
    pub pointer: [u16; 2],
    /// Primary pointer edge, cleared when the panel handles it.
    pub primary_pressed: bool,
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

/// BBB sublocation display mode replacing its packed choice byte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocationPanelChoiceMode {
    /// Show the selected chart location directly.
    #[default]
    None,
    /// Show the eligible sublocation list.
    CandidateList,
    /// Show one selected sublocation and its actor details.
    LocationDetails,
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
    /// BBB sublocation chooser mode.
    pub choice_mode: LocationPanelChoiceMode,
    /// Number of eligible BBB sublocations found in the latest frame.
    pub candidate_count: usize,
    /// BBB sublocation currently under inspection or the pointer.
    pub hovered_location: Option<LocationId>,
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
            choice_mode: LocationPanelChoiceMode::default(),
            candidate_count: usize::MIN,
            hovered_location: None,
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

/// One solid BBB statistic rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationPanelStatDraw {
    /// Original logical pixel origin.
    pub position: [u16; 2],
    /// Rectangle width after native integer scaling.
    pub width: u16,
    /// Rectangle height.
    pub height: u16,
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
    ) -> Result<Vec<LocationPanelSource<LocationId>>, Self::Error>;

    /// Format one signed native word as decimal game-font bytes.
    fn format_panel_integer(&mut self, value: i16) -> Box<[u8]>;

    /// Draw one solid BBB statistic rectangle.
    fn draw_panel_stat(&mut self, draw: LocationPanelStatDraw);

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
    /// Game-specific steady-panel behavior and data.
    pub variant: LocationPanelVariant<'a, LocationId>,
    /// Current and target interpolation rectangles.
    pub rects: LocationPanelRects,
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
        /// Number of roster, candidate, or detail records drawn.
        displayed_record_count: usize,
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
/// This translates `location_info_panel_dispatch` at BLOODPRG file offset
/// `0x009083` and BLOOD2PG file offset `0x00A5E0`. Typed resources and world
/// objects replace record offsets, far strings, the terminated art table, and a
/// stack-owned source list. Explicit phases, rectangles, progress, and renderer
/// operations replace packed state bytes and framebuffer globals while
/// preserving helper order and wrapping scale arithmetic.
pub fn update_location_info_panel<ResourceId, LocationId, ComparisonExtent, Host>(
    context: LocationInfoPanelContext<'_, ResourceId, LocationId, ComparisonExtent>,
    state: &mut LocationInfoPanelState<LocationId>,
    input: &mut LocationPanelInput,
    host: &mut Host,
) -> Result<LocationInfoPanelOutcome, Host::Error>
where
    LocationId: Clone + PartialEq,
    Host: LocationInfoPanelHost<ResourceId, LocationId, ComparisonExtent>,
{
    let mut opening_completed = false;
    if state.phase == LocationPanelPhase::Opening {
        if matches!(context.variant, LocationPanelVariant::BigBugBang { .. }) {
            state.choice_mode = LocationPanelChoiceMode::None;
            state.candidate_count = usize::MIN;
            state.hovered_location = None;
            input.primary_pressed = false;
        }
        let artwork_installed = if state.geometry.scale_step == u8::MIN {
            install_matching_artwork(&context, state, input.pointer, host)?
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
        if matches!(context.variant, LocationPanelVariant::BigBugBang { .. }) {
            let displayed_record_count = draw_sequel_panel(&context, state, input, host)?;
            return Ok(if state.phase == LocationPanelPhase::Closing {
                LocationInfoPanelOutcome::Closing {
                    initiated: true,
                    opening_completed,
                }
            } else {
                LocationInfoPanelOutcome::Steady {
                    opening_completed,
                    displayed_record_count,
                }
            });
        }

        if !input.primary_pressed {
            let displayed_record_count = draw_commander_panel(&context, host)?;
            return Ok(LocationInfoPanelOutcome::Steady {
                opening_completed,
                displayed_record_count,
            });
        }

        arm_panel_close(state);
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
    pointer: [u16; 2],
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
    let source_stride = host.install_panel_artwork(artwork, pointer);
    state.geometry.source_width = u16::from(source_stride as u8)
        .wrapping_mul(SOURCE_WIDTH_NUMERATOR)
        .wrapping_shr(SOURCE_WIDTH_SHIFT);
    host.prepare_panel_palette();
    Ok(true)
}

fn draw_commander_panel<ResourceId, LocationId, ComparisonExtent, Host>(
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
    let mut displayed_record_count = usize::MIN;
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
        displayed_record_count += 1;
    }
    Ok(displayed_record_count)
}

fn draw_sequel_panel<ResourceId, LocationId, ComparisonExtent, Host>(
    context: &LocationInfoPanelContext<'_, ResourceId, LocationId, ComparisonExtent>,
    state: &mut LocationInfoPanelState<LocationId>,
    input: &mut LocationPanelInput,
    host: &mut Host,
) -> Result<usize, Host::Error>
where
    LocationId: Clone + PartialEq,
    Host: LocationInfoPanelHost<ResourceId, LocationId, ComparisonExtent>,
{
    let LocationPanelVariant::BigBugBang {
        excluded_location,
        labels,
    } = &context.variant
    else {
        unreachable!("sequel panel renderer requires sequel panel data");
    };

    host.render_panel_sprites(LocationPanelSpriteRange::PanelOnly);
    host.remap_panel_rect(context.rects.target);

    let mut candidates = Vec::new();
    if context.selected.allows_sublocations {
        candidates = host
            .navigation_sources(&context.selected.id)?
            .into_iter()
            .filter(|source| {
                source.id != *excluded_location
                    && source.kind == ScriptObjectKind::Location
                    && source.in_play
            })
            .collect();
        state.candidate_count = candidates.len();
    }
    if candidates.len() == 1 {
        state.hovered_location = Some(candidates[0].id.clone());
        state.choice_mode = LocationPanelChoiceMode::LocationDetails;
    }

    let displayed_candidate = if state.choice_mode == LocationPanelChoiceMode::LocationDetails {
        state
            .hovered_location
            .as_ref()
            .and_then(|hovered| candidates.iter().find(|candidate| candidate.id == *hovered))
    } else {
        None
    };
    let (displayed_id, displayed_name) = displayed_candidate
        .map_or((&context.selected.id, context.selected.name), |candidate| {
            (&candidate.id, candidate.name.as_ref())
        });
    let title = if state.choice_mode == LocationPanelChoiceMode::LocationDetails {
        labels.location
    } else {
        location_title(context.selected.kind, context.labels)
    };
    let title_width = host.draw_panel_text(LocationPanelTextDraw {
        text: title,
        position: [SEQUEL_PANEL_TITLE_X, PANEL_TITLE_Y],
        color: PANEL_TITLE_COLOR,
    });
    host.draw_panel_text(LocationPanelTextDraw {
        text: displayed_name,
        position: [
            SEQUEL_PANEL_TITLE_X
                .wrapping_add(title_width)
                .wrapping_add(PANEL_NAME_GAP),
            PANEL_TITLE_Y,
        ],
        color: PANEL_TITLE_COLOR,
    });

    let mut displayed_record_count = usize::MIN;
    let mut text_y = PANEL_TITLE_Y.wrapping_add(PANEL_TEXT_ROW_HEIGHT);
    if context.selected.allows_sublocations
        && state.choice_mode != LocationPanelChoiceMode::LocationDetails
        && candidates.len() > 1
    {
        state.choice_mode = LocationPanelChoiceMode::CandidateList;
        state.hovered_location = None;
        for candidate in &candidates {
            let hovered = input.pointer[0] >= PANEL_TEXT_X
                && input.pointer[0] < SEQUEL_CANDIDATE_RIGHT
                && input.pointer[1] >= text_y
                && input.pointer[1] < text_y.wrapping_add(PANEL_TEXT_ROW_HEIGHT);
            if hovered {
                state.hovered_location = Some(candidate.id.clone());
            }
            host.draw_panel_text(LocationPanelTextDraw {
                text: &candidate.name,
                position: [PANEL_TEXT_X, text_y],
                color: if hovered {
                    PANEL_SOURCE_COLOR
                } else {
                    PANEL_TITLE_COLOR
                },
            });
            text_y = text_y.wrapping_add(PANEL_TEXT_ROW_HEIGHT);
            displayed_record_count += 1;
        }
        if input.primary_pressed && state.hovered_location.is_some() {
            state.choice_mode = LocationPanelChoiceMode::LocationDetails;
            input.primary_pressed = false;
            return Ok(displayed_record_count);
        }
    } else if state.choice_mode != LocationPanelChoiceMode::CandidateList {
        displayed_record_count = draw_sequel_actor_details(displayed_id, labels, host)?;
    }

    if !input.primary_pressed {
        return Ok(displayed_record_count);
    }
    input.primary_pressed = false;
    if state.choice_mode == LocationPanelChoiceMode::LocationDetails && state.candidate_count > 1 {
        state.choice_mode = LocationPanelChoiceMode::CandidateList;
        return Ok(displayed_record_count);
    }
    arm_panel_close(state);
    Ok(displayed_record_count)
}

fn draw_sequel_actor_details<ResourceId, LocationId, ComparisonExtent, Host>(
    location: &LocationId,
    labels: &LocationPanelDetailLabels<'_>,
    host: &mut Host,
) -> Result<usize, Host::Error>
where
    Host: LocationInfoPanelHost<ResourceId, LocationId, ComparisonExtent>,
{
    for source in host.navigation_sources(location)? {
        if source.kind != ScriptObjectKind::Actor || source.actor_details.is_none() {
            continue;
        }
        if !source.active {
            return Ok(usize::MIN);
        }
        let details = source
            .actor_details
            .expect("checked sequel actor has panel details");
        draw_fixed_panel_text(host, labels.leader, [108, 35], PANEL_TITLE_COLOR);
        draw_fixed_panel_text(host, &source.name, [148, 35], PANEL_TITLE_COLOR);
        draw_fixed_panel_text(host, labels.population, [108, 45], SEQUEL_POPULATION_COLOR);
        let population = host.format_panel_integer(details.population);
        draw_fixed_panel_text(host, &population, [210, 45], SEQUEL_POPULATION_COLOR);
        draw_sequel_stat(
            host,
            labels.aggressiveness,
            55,
            SEQUEL_AGGRESSIVENESS_COLOR,
            details.aggressiveness,
            false,
        );
        draw_sequel_stat(
            host,
            labels.energy,
            65,
            SEQUEL_ENERGY_COLOR,
            details.energy,
            true,
        );
        draw_sequel_stat(
            host,
            labels.evolution,
            75,
            SEQUEL_EVOLUTION_COLOR,
            details.evolution,
            false,
        );
        return Ok(1);
    }
    Ok(usize::MIN)
}

fn draw_sequel_stat<ResourceId, LocationId, ComparisonExtent, Host>(
    host: &mut Host,
    label: &[u8],
    y: u16,
    color: u8,
    value: i16,
    clamp_negative: bool,
) where
    Host: LocationInfoPanelHost<ResourceId, LocationId, ComparisonExtent>,
{
    draw_fixed_panel_text(host, label, [108, y], color);
    let _formatted = host.format_panel_integer(value);
    let scaled = if clamp_negative && value.is_negative() {
        u16::MIN
    } else {
        (value as u16) / SEQUEL_STAT_DIVISOR
    };
    host.draw_panel_stat(LocationPanelStatDraw {
        position: [SEQUEL_STAT_X, y],
        width: scaled,
        height: SEQUEL_STAT_HEIGHT,
        color,
    });
}

fn draw_fixed_panel_text<ResourceId, LocationId, ComparisonExtent, Host>(
    host: &mut Host,
    text: &[u8],
    position: [u16; 2],
    color: u8,
) where
    Host: LocationInfoPanelHost<ResourceId, LocationId, ComparisonExtent>,
{
    host.draw_panel_text(LocationPanelTextDraw {
        text,
        position,
        color,
    });
}

const fn location_title(
    kind: NavigationStatusLocationKind,
    labels: NavigationStatusLabels<'_>,
) -> &[u8] {
    match kind {
        NavigationStatusLocationKind::Planet => labels.planet,
        NavigationStatusLocationKind::Ship => labels.ship,
        NavigationStatusLocationKind::BlackHole => labels.black_hole,
    }
}

fn arm_panel_close<LocationId>(state: &mut LocationInfoPanelState<LocationId>) {
    state.active = false;
    state.phase = LocationPanelPhase::Closing;
    state.transition.current = u8::MIN;
    state.geometry.scale_step = state.geometry.scale_step.wrapping_add(1);
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
    use std::collections::BTreeMap;

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
        Integer(i16),
        Stat(LocationPanelStatDraw),
        Release,
    }

    struct OracleHost {
        events: Vec<HostEvent>,
        sources: BTreeMap<u16, Vec<LocationPanelSource<u16>>>,
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
        ) -> Result<Vec<LocationPanelSource<u16>>, Self::Error> {
            self.events.push(HostEvent::SourceList(*location));
            Ok(self.sources.get(location).cloned().unwrap_or_default())
        }

        fn format_panel_integer(&mut self, value: i16) -> Box<[u8]> {
            self.events.push(HostEvent::Integer(value));
            value.to_string().into_bytes().into_boxed_slice()
        }

        fn draw_panel_stat(&mut self, draw: LocationPanelStatDraw) {
            self.events.push(HostEvent::Stat(draw));
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
                allows_sublocations: false,
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
                choice_mode: LocationPanelChoiceMode::None,
                candidate_count: 0,
                hovered_location: None,
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
                variant: LocationPanelVariant::CommanderBlood,
                rects: panel_rects(),
                comparison_extent: &(),
            };
            let mut input = LocationPanelInput {
                pointer: [123, 77],
                primary_pressed: vector.mouse & 1 != u8::MIN,
            };
            let mut host = OracleHost {
                events: Vec::new(),
                sources: BTreeMap::from([(
                    SELECTED_LOCATION,
                    if vector.name.contains("eligible_life_support") {
                        source_objects()
                    } else {
                        Vec::new()
                    },
                )]),
            };

            let outcome =
                update_location_info_panel(context, &mut state, &mut input, &mut host).unwrap();

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

    fn source_objects() -> Vec<LocationPanelSource<u16>> {
        vec![
            source(1, ScriptObjectKind::Actor, true, 1, b"ELIGIBLE"),
            source(2, ScriptObjectKind::CelestialBody, true, 1, b"WRONGKIND"),
            source(3, ScriptObjectKind::Actor, false, 1, b"INACTIVE"),
            source(4, ScriptObjectKind::Actor, true, 0, b"UNSEEN"),
        ]
    }

    fn source(
        id: u16,
        kind: ScriptObjectKind,
        active: bool,
        life_support_visits: u16,
        name: &[u8],
    ) -> LocationPanelSource<u16> {
        LocationPanelSource {
            id,
            kind,
            active,
            in_play: false,
            life_support_visits,
            name: Box::from(name),
            actor_details: None,
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
            HostEvent::Integer(_) => "integer",
            HostEvent::Stat(_) => "stat",
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
                displayed_record_count: 0,
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
                    displayed_record_count: 1,
                }
            }
            _ => LocationInfoPanelOutcome::Steady {
                opening_completed: false,
                displayed_record_count: 0,
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
                | HostEvent::Integer(_)
                | HostEvent::Stat(_)
                | HostEvent::Release => {}
            }
        }
    }

    #[derive(Deserialize)]
    struct SequelPanelVector {
        name: String,
        phase_before: u8,
        phase_after: u8,
        scale_before: u8,
        scale_after: u8,
        primary_before: bool,
        primary_after: bool,
        interpolation_complete: bool,
        choice_mode_after: u8,
        candidate_count_after: usize,
        hovered_location_after: u16,
        panel_active_after: bool,
        selected_after: u16,
        deferred_after: u16,
        calls: Vec<SequelOracleCall>,
    }

    #[derive(Deserialize)]
    struct SequelOracleCall {
        name: String,
        text: Option<String>,
        position: Option<[u16; 2]>,
        color: Option<u16>,
        value: Option<u16>,
        extent: Option<u16>,
    }

    const EXCLUDED_LOCATION: u16 = 0x1900;
    const LOCATION_A: u16 = 0x2000;
    const LOCATION_B: u16 = 0x2100;
    const WRONG_KIND: u16 = 0x2200;
    const INACTIVE_LOCATION: u16 = 0x2300;
    const DETAIL_WRONG: u16 = 0x3000;
    const DETAIL_ACTOR: u16 = 0x3100;
    const DETAIL_INACTIVE: u16 = 0x3200;
    const DETAIL_NEGATIVE: u16 = 0x3300;

    #[test]
    fn sequel_panel_dispatch_matches_every_original_vector() {
        let vectors = include_str!(
            "../../../../../re/tools/oracle_vectors/big_bug_bang_location_panel.jsonl"
        )
        .lines()
        .map(|line| serde_json::from_str::<SequelPanelVector>(line).unwrap())
        .collect::<Vec<_>>();
        assert_eq!(vectors.len(), 20);

        for vector in vectors {
            let selected_name: &[u8] = if vector.name.contains("missing_art") {
                b"MISSING"
            } else {
                b"TARGET"
            };
            let (kind, allows_sublocations) = sequel_selected_kind(&vector.name);
            let selected = LocationPanelLocation {
                id: SELECTED_LOCATION,
                kind,
                allows_sublocations,
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
            let details_mode = vector.name.starts_with("selected_location_");
            let mut state = LocationInfoPanelState {
                phase: decode_phase(vector.phase_before),
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
                choice_mode: if details_mode {
                    LocationPanelChoiceMode::LocationDetails
                } else {
                    LocationPanelChoiceMode::None
                },
                candidate_count: 0,
                hovered_location: details_mode.then_some(LOCATION_B),
            };
            let context = LocationInfoPanelContext {
                selected: &selected,
                artwork: &artwork,
                labels: NavigationStatusLabels {
                    planet: b"PLANETE: ",
                    ship: b"VAISSEAU: ",
                    black_hole: b"TROU NOIR: ",
                    life_support: b"VIE PRESENTE:",
                },
                variant: LocationPanelVariant::BigBugBang {
                    excluded_location: EXCLUDED_LOCATION,
                    labels: sequel_detail_labels(),
                },
                rects: panel_rects(),
                comparison_extent: &(),
            };
            let mut input = LocationPanelInput {
                pointer: if vector.name.starts_with("candidate_") {
                    [120, 47]
                } else {
                    [300, 190]
                },
                primary_pressed: vector.primary_before,
            };
            let mut host = OracleHost {
                events: Vec::new(),
                sources: sequel_sources(&vector.name),
            };

            let _outcome =
                update_location_info_panel(context, &mut state, &mut input, &mut host).unwrap();

            assert_sequel_events(&vector, &host.events);
            assert_eq!(
                state.phase,
                decode_phase(vector.phase_after),
                "{}",
                vector.name
            );
            assert_eq!(
                state.geometry.scale_step, vector.scale_after,
                "{}",
                vector.name
            );
            assert_eq!(state.active, vector.panel_active_after, "{}", vector.name);
            assert_eq!(
                input.primary_pressed, vector.primary_after,
                "{}",
                vector.name
            );
            assert_eq!(
                state.choice_mode,
                decode_choice_mode(vector.choice_mode_after),
                "{}",
                vector.name
            );
            assert_eq!(
                state.candidate_count, vector.candidate_count_after,
                "{}",
                vector.name
            );
            assert_eq!(
                state.hovered_location,
                (vector.hovered_location_after != 0).then_some(vector.hovered_location_after),
                "{}",
                vector.name
            );
            assert_eq!(
                state.selected_location,
                (vector.selected_after != 0).then_some(vector.selected_after),
                "{}",
                vector.name
            );
            assert_eq!(
                state.deferred_record_link,
                (vector.deferred_after != 0).then_some(vector.deferred_after),
                "{}",
                vector.name
            );
        }
    }

    fn sequel_selected_kind(name: &str) -> (NavigationStatusLocationKind, bool) {
        if name.contains("black_hole") {
            (NavigationStatusLocationKind::BlackHole, false)
        } else if name.contains("ship_title") {
            (NavigationStatusLocationKind::Ship, false)
        } else {
            (
                NavigationStatusLocationKind::Planet,
                name.contains("planet_")
                    || name.starts_with("candidate_")
                    || name.starts_with("selected_location_")
                    || name.starts_with("single_")
                    || name.starts_with("inactive_presentable_")
                    || name.starts_with("signed_statistics_"),
            )
        }
    }

    const fn sequel_detail_labels() -> LocationPanelDetailLabels<'static> {
        LocationPanelDetailLabels {
            location: b"LIEU:",
            leader: b"CHEF",
            population: b"POPULATION:",
            aggressiveness: b"AGRESSIVITE:",
            energy: b"ENERGIE:",
            evolution: b"EVOLUTION:",
        }
    }

    fn decode_choice_mode(value: u8) -> LocationPanelChoiceMode {
        match value {
            0 => LocationPanelChoiceMode::None,
            1 => LocationPanelChoiceMode::CandidateList,
            2 => LocationPanelChoiceMode::LocationDetails,
            _ => panic!("unexpected original choice mode {value}"),
        }
    }

    fn sequel_sources(name: &str) -> BTreeMap<u16, Vec<LocationPanelSource<u16>>> {
        let details = vec![
            sequel_source(
                DETAIL_WRONG,
                ScriptObjectKind::Actor,
                true,
                false,
                b"WRONGDETAIL",
                None,
            ),
            sequel_source(
                DETAIL_ACTOR,
                ScriptObjectKind::Actor,
                true,
                false,
                b"LEADER",
                Some(LocationPanelActorDetails {
                    population: 1234,
                    aggressiveness: 99,
                    energy: 401,
                    evolution: 78,
                }),
            ),
        ];
        let candidates = vec![
            sequel_source(
                EXCLUDED_LOCATION,
                ScriptObjectKind::Location,
                true,
                true,
                b"ARCHE",
                None,
            ),
            sequel_source(
                WRONG_KIND,
                ScriptObjectKind::Actor,
                false,
                true,
                b"WRONGKIND",
                None,
            ),
            sequel_source(
                INACTIVE_LOCATION,
                ScriptObjectKind::Location,
                false,
                false,
                b"INACTIVE",
                None,
            ),
            sequel_source(
                LOCATION_A,
                ScriptObjectKind::Location,
                false,
                true,
                b"FIRST",
                None,
            ),
            sequel_source(
                LOCATION_B,
                ScriptObjectKind::Location,
                false,
                true,
                b"SECOND",
                None,
            ),
        ];
        if name.contains("multiple_locations") || name.starts_with("candidate_") {
            return BTreeMap::from([(SELECTED_LOCATION, candidates)]);
        }
        if name.starts_with("selected_location_") {
            return BTreeMap::from([(SELECTED_LOCATION, candidates), (LOCATION_B, details)]);
        }
        if name.starts_with("single_") {
            return BTreeMap::from([
                (
                    SELECTED_LOCATION,
                    vec![sequel_source(
                        LOCATION_A,
                        ScriptObjectKind::Location,
                        false,
                        true,
                        b"FIRST",
                        None,
                    )],
                ),
                (LOCATION_A, details),
            ]);
        }
        if name.starts_with("inactive_presentable_") {
            return BTreeMap::from([(
                SELECTED_LOCATION,
                vec![
                    sequel_source(
                        DETAIL_INACTIVE,
                        ScriptObjectKind::Actor,
                        false,
                        false,
                        b"INACTIVELEADER",
                        Some(LocationPanelActorDetails {
                            population: 0,
                            aggressiveness: 0,
                            energy: 0,
                            evolution: 0,
                        }),
                    ),
                    details[1].clone(),
                ],
            )]);
        }
        if name.starts_with("signed_statistics_") {
            return BTreeMap::from([(
                SELECTED_LOCATION,
                vec![sequel_source(
                    DETAIL_NEGATIVE,
                    ScriptObjectKind::Actor,
                    true,
                    false,
                    b"NEGATIVE",
                    Some(LocationPanelActorDetails {
                        population: -25,
                        aggressiveness: -25,
                        energy: -25,
                        evolution: -25,
                    }),
                )],
            )]);
        }
        if name.contains("eligible_actor_details") {
            return BTreeMap::from([(SELECTED_LOCATION, details)]);
        }
        BTreeMap::new()
    }

    fn sequel_source(
        id: u16,
        kind: ScriptObjectKind,
        active: bool,
        in_play: bool,
        name: &[u8],
        actor_details: Option<LocationPanelActorDetails>,
    ) -> LocationPanelSource<u16> {
        LocationPanelSource {
            id,
            kind,
            active,
            in_play,
            life_support_visits: 0,
            name: Box::from(name),
            actor_details,
        }
    }

    fn assert_sequel_events(vector: &SequelPanelVector, events: &[HostEvent]) {
        let expected = vector
            .calls
            .iter()
            .filter(|call| call.name != "compare")
            .collect::<Vec<_>>();
        assert_eq!(events.len(), expected.len(), "{}", vector.name);
        for (actual, expected) in events.iter().zip(expected) {
            assert_eq!(event_name(actual), expected.name, "{}", vector.name);
            match actual {
                HostEvent::Text {
                    text,
                    position,
                    color,
                } => {
                    assert_eq!(
                        text.as_ref(),
                        expected.text.as_deref().unwrap().as_bytes(),
                        "{}",
                        vector.name
                    );
                    assert_eq!(Some(*position), expected.position, "{}", vector.name);
                    assert_eq!(Some(u16::from(*color)), expected.color, "{}", vector.name);
                }
                HostEvent::Integer(value) => {
                    assert_eq!(Some(*value as u16), expected.value, "{}", vector.name);
                }
                HostEvent::Stat(draw) => {
                    assert_eq!(Some(draw.position), expected.position, "{}", vector.name);
                    assert_eq!(Some(draw.width), expected.value, "{}", vector.name);
                    assert_eq!(Some(draw.height), expected.extent, "{}", vector.name);
                    assert_eq!(
                        Some(u16::from(draw.color)),
                        expected.color,
                        "{}",
                        vector.name
                    );
                }
                _ => {}
            }
        }
    }
}
