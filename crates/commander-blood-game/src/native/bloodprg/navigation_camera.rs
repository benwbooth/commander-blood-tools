//! Navigation-chart camera transition and interaction coordinator.

use std::fmt;

use super::{
    LocationInfoPanelState, LocationPanelPhase, LocationPanelRect, LocationPanelRects,
    LocationPanelTransitionProgress, NavigationWipeSpan,
};

const FIRST_TRANSITION_STEP: u8 = 8;
const SCREEN_WIDTH: u16 = 320;
const SCREEN_HEIGHT: u16 = 200;
const SCREEN_CENTER_Y: u16 = 110;
const PRIMARY_CHART_ENTITY: u16 = 0;
const ARCHE_CHART_ENTITY: u16 = 1;
const FIRST_SECONDARY_CHART_ENTITY: u16 = 5;
const FINAL_CHART_ENTITY: u16 = 31;
const SECONDARY_CHART_ENTITY_CAPACITY: usize = 26;
const CHART_RESOURCE_ID: u16 = 44;
const ARCHE_FRAME: u16 = 6;
const SECONDARY_FRAME_OFFSET: u16 = 3;
const SECONDARY_MARKER_OFFSET: u16 = 3;
const ARCHE_X_OFFSET: u16 = 16;
const ARCHE_Y_OFFSET: u16 = 13;
const BLACK_HOLE_ARCH_X_ADJUSTMENT: u16 = 5;
const BLACK_HOLE_ARCH_Y_ADJUSTMENT: u16 = 2;
const SHIP_ARCH_X_ADJUSTMENT: u16 = 3;
const POINTER_PANEL_EXTENT: i16 = 4;
const LABEL_Y_BIAS: u16 = 10;
const LABEL_COLOR: u8 = 239;
const HAND_SIDE_BOUNDARY: u16 = 160;

/// Independent object-kind bits needed by chart frame and Arche placement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavigationChartObjectKind {
    /// Object uses the ship marker family.
    pub ship: bool,
    /// Object uses the black-hole marker family.
    pub black_hole: bool,
}

impl NavigationChartObjectKind {
    const fn marker_frame(self) -> u16 {
        if self.black_hole {
            1
        } else if self.ship {
            2
        } else {
            0
        }
    }
}

/// One typed object displayed or selected on the navigation chart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationChartObject<ObjectId> {
    /// Stable world-object identity.
    pub id: ObjectId,
    /// Marker-kind flags.
    pub kind: NavigationChartObjectKind,
    /// Authored game-font name.
    pub name: Box<[u8]>,
    /// Original logical marker position.
    pub marker: [u16; 2],
    /// Whether the native access counter requests the extra chart marker.
    pub show_secondary_marker: bool,
}

/// Arche marker and current-location data used by chart presentation.
#[derive(Clone, Copy, Debug)]
pub struct NavigationChartArche<'a, ObjectId> {
    /// Arche's original logical marker position.
    pub marker: [u16; 2],
    /// Stable identity of the current location.
    pub current_location: &'a ObjectId,
    /// Current location's independent kind bits.
    pub current_location_kind: NavigationChartObjectKind,
}

/// One chart-entity population request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationChartEntityDraw {
    /// Destination entity slot.
    pub entity: u16,
    /// Typed chart sprite resource identifier.
    pub resource: u16,
    /// Original logical marker origin.
    pub position: [u16; 2],
    /// Sprite frame within the chart resource.
    pub frame: u16,
}

/// One rectangular span copied from the back buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationChartCopySpan {
    /// Left edge.
    pub x: u16,
    /// Row.
    pub y: u16,
    /// Width.
    pub width: u16,
}

/// Direction of the eight-step navigation-chart wipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationChartWipeDirection {
    /// Camera closes into the chart and builds its entities.
    Closing,
    /// Camera opens back toward the interactive panorama.
    Opening,
}

/// Animated hand selected by a chart click.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationChartHand {
    /// Neutral hand selector.
    Neutral,
    /// Left-side click animation.
    Left,
    /// Right-side click animation.
    Right,
}

/// Current and requested hand-animation selectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationChartHandState {
    /// Current animation.
    pub current: NavigationChartHand,
    /// Requested animation.
    pub requested: NavigationChartHand,
}

impl Default for NavigationChartHandState {
    fn default() -> Self {
        Self {
            current: NavigationChartHand::Neutral,
            requested: NavigationChartHand::Neutral,
        }
    }
}

/// Pointer edges consumed by navigation-chart interaction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavigationChartInputState {
    /// Current original logical pointer position.
    pub pointer: [u16; 2],
    /// Primary press edge.
    pub primary_pressed: bool,
    /// General pending mouse-press latch.
    pub press_pending: bool,
}

/// Mutable navigation-chart camera and panel state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationCameraState<ObjectId> {
    /// Remaining transition step; zero is interactive state.
    pub transition_step: u8,
    /// Whether the navigation camera is active.
    pub active: bool,
    /// Closing wipe has reached its final frame.
    pub wipe_complete: bool,
    /// Navigation-chart UI bit exposed to the surrounding bridge.
    pub ui_active: bool,
    /// Number of chart objects built on the first closing frame.
    pub chart_object_count: usize,
    /// Number of extra marker entities after entity five.
    pub secondary_marker_count: usize,
    /// Native lower entity-state mask interpreted semantically.
    pub entity_state_mask: u8,
    /// PBM palette refresh is allowed during panorama loads.
    pub palette_refresh_enabled: bool,
    /// Hand animation state.
    pub hand: NavigationChartHandState,
    /// Pointer and press latches.
    pub input: NavigationChartInputState,
    /// Location-information panel lifecycle.
    pub panel: LocationInfoPanelState<ObjectId>,
    /// Current and target panel rectangles initialized by a chart click.
    pub panel_rects: LocationPanelRects,
}

impl<ObjectId> Default for NavigationCameraState<ObjectId> {
    fn default() -> Self {
        Self {
            transition_step: u8::MIN,
            active: false,
            wipe_complete: false,
            ui_active: false,
            chart_object_count: usize::MIN,
            secondary_marker_count: usize::MIN,
            entity_state_mask: u8::MIN,
            palette_refresh_enabled: true,
            hand: NavigationChartHandState::default(),
            input: NavigationChartInputState::default(),
            panel: LocationInfoPanelState::default(),
            panel_rects: LocationPanelRects::default(),
        }
    }
}

/// Entity visibility state emitted during interactive chart polling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationChartEntityState {
    /// Entity slot.
    pub entity: u16,
    /// Chart entities remain visible while interactive.
    pub visible: bool,
    /// Native active flag after applying the chart state mask.
    pub active: bool,
}

/// Host operations required by navigation-chart camera coordination.
pub trait NavigationCameraHost<ObjectId, ComparisonExtent> {
    /// World-query or rendering failure.
    type Error;

    /// Copy the native planar chart background into the modern work surface.
    fn copy_chart_background_to_work_surface(&mut self);

    /// Build current chart objects in original traversal order.
    fn chart_objects(&mut self) -> Result<Vec<NavigationChartObject<ObjectId>>, Self::Error>;

    /// Populate one chart entity.
    fn populate_chart_entity(&mut self, draw: NavigationChartEntityDraw);

    /// Render the primary chart entity into the work surface.
    fn render_primary_chart_entity(&mut self);

    /// Toggle one entity's transition state.
    fn transition_chart_entity(&mut self, entity: u16);

    /// Build typed wipe spans for the selected endpoint.
    fn build_wipe_spans(
        &mut self,
        endpoint: [u16; 2],
    ) -> Result<Box<[NavigationWipeSpan]>, Self::Error>;

    /// Copy one back-buffer span.
    fn copy_back_buffer_span(&mut self, span: NavigationChartCopySpan);

    /// Publish dirty rectangles after a transition frame.
    fn publish_transition_dirty_rects(&mut self);

    /// Load the bridge panorama into the flat work surface.
    fn load_bridge_panorama(&mut self);

    /// Snapshot ship HUD colors and reset its camera.
    fn snapshot_ship_hud_and_reset_camera(&mut self);

    /// Present the restored panorama frame.
    fn present_restored_panorama(&mut self);

    /// Run the location-information panel with inherited extent context.
    fn update_location_panel(
        &mut self,
        panel: &mut LocationInfoPanelState<ObjectId>,
        comparison_extent: &ComparisonExtent,
    ) -> Result<(), Self::Error>;

    /// Commit one interactive entity state.
    fn set_chart_entity_state(&mut self, state: NavigationChartEntityState);

    /// Pick the first object under the current pointer.
    fn pick_chart_object(&mut self)
    -> Result<Option<NavigationChartObject<ObjectId>>, Self::Error>;

    /// Measure a hover label with the native dual-font rule.
    fn measure_hover_label(&mut self, text: &[u8]) -> u16;

    /// Draw the clamped hover label.
    fn draw_hover_label(&mut self, text: &[u8], position: [u16; 2], color: u8);
}

/// Read-only camera inputs for one update.
#[derive(Clone, Copy, Debug)]
pub struct NavigationCameraContext<'a, ObjectId, ComparisonExtent> {
    /// Arche marker and current world location.
    pub arche: NavigationChartArche<'a, ObjectId>,
    /// Eight authored radial-wipe endpoints.
    pub wipe_endpoints: &'a [[u16; 2]],
    /// Inherited panel entity extent context.
    pub comparison_extent: &'a ComparisonExtent,
}

/// Observable terminal path of one navigation-camera update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationCameraOutcome {
    /// Camera and transition are inactive.
    Inactive,
    /// Interactive camera is waiting for the closing wipe to finish.
    WaitingForWipe,
    /// Existing selected location delegated to the panel.
    LocationPanel,
    /// Interactive poll found no chart object.
    NoObjectPicked,
    /// Hover label was drawn.
    HoverLabel {
        /// Clamped logical position.
        position: [u16; 2],
    },
    /// Click selected the current location and therefore opened no panel.
    CurrentLocation,
    /// Click initialized a new location panel.
    LocationPanelOpened,
    /// One radial transition frame was generated.
    TransitionFrame {
        /// Opening or closing direction.
        direction: NavigationChartWipeDirection,
        /// Whether this was transition step eight.
        first_frame: bool,
        /// Number of emitted back-buffer copies.
        copy_count: usize,
    },
}

/// Invalid typed camera state or host failure.
#[derive(Debug)]
pub enum NavigationCameraError<HostError> {
    /// Host operation failed.
    Host(HostError),
    /// Nonzero transition step exceeds the authored eight-frame table.
    InvalidTransitionStep {
        /// Supplied step.
        step: u8,
    },
    /// The decoded endpoint table is too short.
    MissingWipeEndpoint {
        /// Required zero-based endpoint index.
        index: usize,
    },
    /// Extra marker entities would collide with the reserved final entity.
    TooManySecondaryMarkers {
        /// Requested marker count.
        count: usize,
    },
}

impl<HostError: fmt::Display> fmt::Display for NavigationCameraError<HostError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(error) => error.fmt(formatter),
            Self::InvalidTransitionStep { step } => {
                write!(formatter, "invalid navigation transition step {step}")
            }
            Self::MissingWipeEndpoint { index } => {
                write!(formatter, "missing navigation wipe endpoint {index}")
            }
            Self::TooManySecondaryMarkers { count } => {
                write!(
                    formatter,
                    "{count} navigation secondary markers exceed entity capacity"
                )
            }
        }
    }
}

impl<HostError> std::error::Error for NavigationCameraError<HostError> where
    HostError: std::error::Error + 'static
{
}

/// Update navigation-chart transition or interactive behavior.
///
/// This translates `nav_camera_state_check` at BLOODPRG routine offset
/// `0x008CCE`. Typed world identities, chart objects, entity requests, wipe
/// endpoints, bounded spans, and panel state replace record offsets, near/far
/// pointers, sentinel lists, segment-swapped framebuffers, and packed entity
/// bytes. The original first-frame order, marker geometry, wipe copies, hover
/// clamping, hand selection, and panel initialization are retained.
pub fn update_navigation_camera<ObjectId, ComparisonExtent, Host>(
    context: NavigationCameraContext<'_, ObjectId, ComparisonExtent>,
    state: &mut NavigationCameraState<ObjectId>,
    host: &mut Host,
) -> Result<NavigationCameraOutcome, NavigationCameraError<Host::Error>>
where
    ObjectId: Clone + PartialEq,
    Host: NavigationCameraHost<ObjectId, ComparisonExtent>,
{
    if state.transition_step == u8::MIN {
        return update_interactive_camera(context, state, host);
    }
    if state.transition_step > FIRST_TRANSITION_STEP {
        return Err(NavigationCameraError::InvalidTransitionStep {
            step: state.transition_step,
        });
    }

    state.panel.selected_location = None;
    let direction = if state.active {
        NavigationChartWipeDirection::Opening
    } else {
        NavigationChartWipeDirection::Closing
    };
    let first_frame = state.transition_step == FIRST_TRANSITION_STEP;
    if first_frame {
        match direction {
            NavigationChartWipeDirection::Closing => build_chart_entities(&context, state, host)?,
            NavigationChartWipeDirection::Opening => {
                validate_secondary_marker_count(state.secondary_marker_count)?;
                restore_panorama(state, host);
            }
        }
    }

    let endpoint_index = match direction {
        NavigationChartWipeDirection::Closing => usize::from(state.transition_step - 1),
        NavigationChartWipeDirection::Opening => usize::from(9 - state.transition_step),
    };
    let endpoint = *context.wipe_endpoints.get(endpoint_index).ok_or(
        NavigationCameraError::MissingWipeEndpoint {
            index: endpoint_index,
        },
    )?;
    if direction == NavigationChartWipeDirection::Closing {
        state.wipe_complete = state.transition_step == 1;
    }
    let spans = host
        .build_wipe_spans(endpoint)
        .map_err(NavigationCameraError::Host)?;
    let copy_count = emit_wipe_copies(direction, endpoint[1], &spans, host);
    state.transition_step = state.transition_step.wrapping_sub(1);
    host.publish_transition_dirty_rects();

    Ok(NavigationCameraOutcome::TransitionFrame {
        direction,
        first_frame,
        copy_count,
    })
}

fn update_interactive_camera<ObjectId, ComparisonExtent, Host>(
    context: NavigationCameraContext<'_, ObjectId, ComparisonExtent>,
    state: &mut NavigationCameraState<ObjectId>,
    host: &mut Host,
) -> Result<NavigationCameraOutcome, NavigationCameraError<Host::Error>>
where
    ObjectId: Clone + PartialEq,
    Host: NavigationCameraHost<ObjectId, ComparisonExtent>,
{
    if !state.active {
        return Ok(NavigationCameraOutcome::Inactive);
    }
    if !state.wipe_complete {
        return Ok(NavigationCameraOutcome::WaitingForWipe);
    }

    state.ui_active = true;
    if state.panel.selected_location.is_some() {
        host.update_location_panel(&mut state.panel, context.comparison_extent)
            .map_err(NavigationCameraError::Host)?;
        return Ok(NavigationCameraOutcome::LocationPanel);
    }

    validate_secondary_marker_count(state.secondary_marker_count)?;
    publish_interactive_entity_states(state, host);
    let Some(picked) = host
        .pick_chart_object()
        .map_err(NavigationCameraError::Host)?
    else {
        return Ok(NavigationCameraOutcome::NoObjectPicked);
    };

    if !state.input.primary_pressed {
        let width = host.measure_hover_label(&picked.name);
        let position = [
            clamp_wrapped_signed(state.input.pointer[0].wrapping_sub(width)),
            clamp_wrapped_signed(state.input.pointer[1].wrapping_sub(LABEL_Y_BIAS)),
        ];
        host.draw_hover_label(&picked.name, position, LABEL_COLOR);
        return Ok(NavigationCameraOutcome::HoverLabel { position });
    }

    state.hand.current = NavigationChartHand::Neutral;
    state.hand.requested = if state.input.pointer[0] > HAND_SIDE_BOUNDARY {
        NavigationChartHand::Right
    } else {
        NavigationChartHand::Left
    };
    state.input.primary_pressed = false;
    state.input.press_pending = false;
    if picked.id == *context.arche.current_location {
        return Ok(NavigationCameraOutcome::CurrentLocation);
    }

    let picked_id = picked.id;
    state.panel.selected_location = Some(picked_id.clone());
    state.panel.deferred_record_link = Some(picked_id);
    state.panel.phase = LocationPanelPhase::Opening;
    state.panel.active = true;
    state.panel.geometry.scale_step = u8::MIN;
    state.panel.transition = LocationPanelTransitionProgress {
        current: u8::MIN,
        total: FIRST_TRANSITION_STEP,
    };
    state.panel_rects.current = LocationPanelRect {
        x: state.input.pointer[0] as i16,
        y: state.input.pointer[1] as i16,
        width: POINTER_PANEL_EXTENT,
        height: POINTER_PANEL_EXTENT,
    };
    state.panel.geometry.layout.current = state.input.pointer;
    state.panel.geometry.layout.target = [
        state.panel_rects.target.x as u16,
        state.panel_rects.target.y as u16,
    ];
    host.transition_chart_entity(ARCHE_CHART_ENTITY);
    transition_secondary_entities(state.secondary_marker_count, host);
    Ok(NavigationCameraOutcome::LocationPanelOpened)
}

fn publish_interactive_entity_states<ObjectId, ComparisonExtent, Host>(
    state: &NavigationCameraState<ObjectId>,
    host: &mut Host,
) where
    Host: NavigationCameraHost<ObjectId, ComparisonExtent>,
{
    let secondary_active = state.entity_state_mask & 1 != u8::MIN;
    for index in 0..state.secondary_marker_count {
        host.set_chart_entity_state(NavigationChartEntityState {
            entity: FIRST_SECONDARY_CHART_ENTITY.wrapping_add(index as u16),
            visible: true,
            active: secondary_active,
        });
    }
    host.set_chart_entity_state(NavigationChartEntityState {
        entity: ARCHE_CHART_ENTITY,
        visible: true,
        active: state.entity_state_mask & 7 == u8::MIN,
    });
}

fn build_chart_entities<ObjectId, ComparisonExtent, Host>(
    context: &NavigationCameraContext<'_, ObjectId, ComparisonExtent>,
    state: &mut NavigationCameraState<ObjectId>,
    host: &mut Host,
) -> Result<(), NavigationCameraError<Host::Error>>
where
    ObjectId: Clone,
    Host: NavigationCameraHost<ObjectId, ComparisonExtent>,
{
    host.copy_chart_background_to_work_surface();
    let objects = host.chart_objects().map_err(NavigationCameraError::Host)?;
    if !objects.is_empty() {
        state.chart_object_count = objects.len();
        state.secondary_marker_count = usize::MIN;
        for object in objects {
            let frame = object.kind.marker_frame();
            host.populate_chart_entity(NavigationChartEntityDraw {
                entity: PRIMARY_CHART_ENTITY,
                resource: CHART_RESOURCE_ID,
                position: object.marker,
                frame,
            });
            if object.show_secondary_marker {
                let entity = FIRST_SECONDARY_CHART_ENTITY
                    .checked_add(state.secondary_marker_count as u16)
                    .filter(|entity| *entity < FINAL_CHART_ENTITY)
                    .ok_or(NavigationCameraError::TooManySecondaryMarkers {
                        count: state.secondary_marker_count + 1,
                    })?;
                state.secondary_marker_count += 1;
                host.populate_chart_entity(NavigationChartEntityDraw {
                    entity,
                    resource: CHART_RESOURCE_ID,
                    position: [
                        object.marker[0].wrapping_sub(SECONDARY_MARKER_OFFSET),
                        object.marker[1].wrapping_sub(SECONDARY_MARKER_OFFSET),
                    ],
                    frame: frame.wrapping_add(SECONDARY_FRAME_OFFSET),
                });
            }
            host.render_primary_chart_entity();
        }
        host.transition_chart_entity(PRIMARY_CHART_ENTITY);
    }

    let mut position = [
        clamp_wrapped_signed(context.arche.marker[0].wrapping_sub(ARCHE_X_OFFSET)),
        clamp_wrapped_signed(context.arche.marker[1].wrapping_sub(ARCHE_Y_OFFSET)),
    ];
    if context.arche.current_location_kind.black_hole {
        position[0] = position[0].wrapping_add(BLACK_HOLE_ARCH_X_ADJUSTMENT);
        position[1] = position[1].wrapping_add(BLACK_HOLE_ARCH_Y_ADJUSTMENT);
    }
    if context.arche.current_location_kind.ship {
        position[0] = position[0].wrapping_add(SHIP_ARCH_X_ADJUSTMENT);
    }
    host.populate_chart_entity(NavigationChartEntityDraw {
        entity: ARCHE_CHART_ENTITY,
        resource: CHART_RESOURCE_ID,
        position,
        frame: ARCHE_FRAME,
    });
    host.transition_chart_entity(ARCHE_CHART_ENTITY);
    transition_secondary_entities(state.secondary_marker_count, host);
    host.transition_chart_entity(FINAL_CHART_ENTITY);
    Ok(())
}

fn restore_panorama<ObjectId, ComparisonExtent, Host>(
    state: &mut NavigationCameraState<ObjectId>,
    host: &mut Host,
) where
    Host: NavigationCameraHost<ObjectId, ComparisonExtent>,
{
    state.wipe_complete = false;
    host.transition_chart_entity(ARCHE_CHART_ENTITY);
    transition_secondary_entities(state.secondary_marker_count, host);
    state.palette_refresh_enabled = false;
    host.load_bridge_panorama();
    host.snapshot_ship_hud_and_reset_camera();
    host.present_restored_panorama();
}

fn transition_secondary_entities<ObjectId, ComparisonExtent, Host>(count: usize, host: &mut Host)
where
    Host: NavigationCameraHost<ObjectId, ComparisonExtent>,
{
    for index in 0..count {
        host.transition_chart_entity(FIRST_SECONDARY_CHART_ENTITY.wrapping_add(index as u16));
    }
}

fn validate_secondary_marker_count<HostError>(
    count: usize,
) -> Result<(), NavigationCameraError<HostError>> {
    if count > SECONDARY_CHART_ENTITY_CAPACITY {
        return Err(NavigationCameraError::TooManySecondaryMarkers { count });
    }
    Ok(())
}

fn emit_wipe_copies<ObjectId, ComparisonExtent, Host>(
    direction: NavigationChartWipeDirection,
    endpoint_y: u16,
    spans: &[NavigationWipeSpan],
    host: &mut Host,
) -> usize
where
    Host: NavigationCameraHost<ObjectId, ComparisonExtent>,
{
    let mut copies = Vec::new();
    match direction {
        NavigationChartWipeDirection::Closing if endpoint_y < SCREEN_CENTER_Y => {
            copies.extend((SCREEN_CENTER_Y..SCREEN_HEIGHT).map(full_row));
            let mut row = endpoint_y;
            for span in spans {
                copies.extend(outside_span(row, span.left, span.width));
                row = row.wrapping_add(1);
            }
        }
        NavigationChartWipeDirection::Closing => {
            let mut row = SCREEN_CENTER_Y;
            for span in spans {
                copies.push(NavigationChartCopySpan {
                    x: span.left,
                    y: row,
                    width: span.width,
                });
                row = row.wrapping_add(1);
            }
            copies.extend((row..SCREEN_HEIGHT).map(full_row));
        }
        NavigationChartWipeDirection::Opening if endpoint_y < SCREEN_CENTER_Y => {
            if endpoint_y > 1 {
                copies.extend((1..endpoint_y).rev().map(full_row));
            }
            let mut row = endpoint_y;
            for span in spans {
                copies.push(NavigationChartCopySpan {
                    x: span.left,
                    y: row,
                    width: span.width,
                });
                row = row.wrapping_add(1);
            }
        }
        NavigationChartWipeDirection::Opening => {
            copies.extend((1..SCREEN_CENTER_Y).rev().map(full_row));
            let mut row = SCREEN_CENTER_Y;
            for span in spans {
                copies.extend(outside_span(row, span.left, span.width));
                row = row.wrapping_add(1);
            }
        }
    }

    let count = copies.len();
    for copy in copies {
        host.copy_back_buffer_span(copy);
    }
    count
}

const fn full_row(y: u16) -> NavigationChartCopySpan {
    NavigationChartCopySpan {
        x: u16::MIN,
        y,
        width: SCREEN_WIDTH,
    }
}

fn outside_span(row: u16, left: u16, width: u16) -> [NavigationChartCopySpan; 2] {
    let right = left.wrapping_add(width);
    [
        NavigationChartCopySpan {
            x: u16::MIN,
            y: row,
            width: left,
        },
        NavigationChartCopySpan {
            x: right,
            y: row,
            width: SCREEN_WIDTH.wrapping_sub(right),
        },
    ]
}

const fn clamp_wrapped_signed(value: u16) -> u16 {
    if value as i16 >= 0 { value } else { u16::MIN }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::*;

    const CURRENT_LOCATION: u16 = 8_448;
    const PICKED_LOCATION: u16 = 4_096;
    const INITIAL_DEFERRED_LOCATION: u16 = 27_242;
    const TRANSITION_ENDPOINT_COUNT: usize = 8;

    #[derive(Deserialize)]
    struct CameraVector {
        name: String,
        state_before: u8,
        state_after: u8,
        active: u8,
        calls: Vec<OracleCall>,
        copy_count: usize,
        copy_head: Vec<[u16; 3]>,
        copy_tail: Vec<[u16; 3]>,
        copy_sha256: String,
    }

    #[derive(Deserialize)]
    struct OracleCall {
        name: String,
        endpoint: Option<[u16; 2]>,
        entity: Option<u16>,
        resource: Option<u16>,
        position: Option<[u16; 2]>,
        frame: Option<u16>,
        range: Option<[u16; 2]>,
        text: Option<String>,
        color: Option<u8>,
        result: Option<u16>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Event {
        Vga,
        List(usize),
        Populate(NavigationChartEntityDraw),
        Render,
        Transition(u16),
        Wipe([u16; 2]),
        Dirty,
        Panorama,
        Reset,
        Flip,
        Panel,
        Pick(Option<u16>),
        Width(Box<[u8]>),
        Text {
            text: Box<[u8]>,
            position: [u16; 2],
            color: u8,
        },
    }

    struct OracleHost {
        events: Vec<Event>,
        copies: Vec<NavigationChartCopySpan>,
        entity_states: Vec<NavigationChartEntityState>,
        chart_objects: Vec<NavigationChartObject<u16>>,
        picked: Option<NavigationChartObject<u16>>,
        wipe_spans: Box<[NavigationWipeSpan]>,
        label_width: u16,
    }

    impl NavigationCameraHost<u16, ()> for OracleHost {
        type Error = std::convert::Infallible;

        fn copy_chart_background_to_work_surface(&mut self) {
            self.events.push(Event::Vga);
        }

        fn chart_objects(&mut self) -> Result<Vec<NavigationChartObject<u16>>, Self::Error> {
            self.events.push(Event::List(self.chart_objects.len()));
            Ok(std::mem::take(&mut self.chart_objects))
        }

        fn populate_chart_entity(&mut self, draw: NavigationChartEntityDraw) {
            self.events.push(Event::Populate(draw));
        }

        fn render_primary_chart_entity(&mut self) {
            self.events.push(Event::Render);
        }

        fn transition_chart_entity(&mut self, entity: u16) {
            self.events.push(Event::Transition(entity));
        }

        fn build_wipe_spans(
            &mut self,
            endpoint: [u16; 2],
        ) -> Result<Box<[NavigationWipeSpan]>, Self::Error> {
            self.events.push(Event::Wipe(endpoint));
            Ok(std::mem::take(&mut self.wipe_spans))
        }

        fn copy_back_buffer_span(&mut self, span: NavigationChartCopySpan) {
            self.copies.push(span);
        }

        fn publish_transition_dirty_rects(&mut self) {
            self.events.push(Event::Dirty);
        }

        fn load_bridge_panorama(&mut self) {
            self.events.push(Event::Panorama);
        }

        fn snapshot_ship_hud_and_reset_camera(&mut self) {
            self.events.push(Event::Reset);
        }

        fn present_restored_panorama(&mut self) {
            self.events.push(Event::Flip);
        }

        fn update_location_panel(
            &mut self,
            _panel: &mut LocationInfoPanelState<u16>,
            _comparison_extent: &(),
        ) -> Result<(), Self::Error> {
            self.events.push(Event::Panel);
            Ok(())
        }

        fn set_chart_entity_state(&mut self, state: NavigationChartEntityState) {
            self.entity_states.push(state);
        }

        fn pick_chart_object(&mut self) -> Result<Option<NavigationChartObject<u16>>, Self::Error> {
            self.events
                .push(Event::Pick(self.picked.as_ref().map(|object| object.id)));
            Ok(self.picked.take())
        }

        fn measure_hover_label(&mut self, text: &[u8]) -> u16 {
            self.events.push(Event::Width(Box::from(text)));
            self.label_width
        }

        fn draw_hover_label(&mut self, text: &[u8], position: [u16; 2], color: u8) {
            self.events.push(Event::Text {
                text: Box::from(text),
                position,
                color,
            });
        }
    }

    #[test]
    fn camera_update_matches_every_original_vector() {
        let vectors: Vec<CameraVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_8cce_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 12);

        for vector in vectors {
            let endpoint = vector
                .calls
                .iter()
                .find_map(|call| call.endpoint)
                .unwrap_or([160, 110]);
            let mut endpoints = [[160, 110]; TRANSITION_ENDPOINT_COUNT];
            if vector.state_before != u8::MIN {
                let index = if vector.active & 1 != u8::MIN {
                    usize::from(9 - vector.state_before)
                } else {
                    usize::from(vector.state_before - 1)
                };
                endpoints[index] = endpoint;
            }
            let mut state = state_for(&vector);
            let context = NavigationCameraContext {
                arche: NavigationChartArche {
                    marker: [12, 10],
                    current_location: &CURRENT_LOCATION,
                    current_location_kind: NavigationChartObjectKind {
                        ship: true,
                        black_hole: true,
                    },
                },
                wipe_endpoints: &endpoints,
                comparison_extent: &(),
            };
            let mut host = host_for(&vector);

            let outcome = update_navigation_camera(context, &mut state, &mut host).unwrap();

            assert_event_trace(&vector, &host.events);
            assert_copy_trace(&vector, &host.copies);
            assert_state(&vector, &state, &host, outcome);
            assert_call_details(&vector, &host.events);
        }
    }

    #[test]
    fn oversized_flat_secondary_marker_state_is_rejected_before_entity_updates() {
        let mut state = NavigationCameraState {
            active: true,
            wipe_complete: true,
            secondary_marker_count: SECONDARY_CHART_ENTITY_CAPACITY + 1,
            ..NavigationCameraState::default()
        };
        let endpoints = [[160, 110]; TRANSITION_ENDPOINT_COUNT];
        let context = NavigationCameraContext {
            arche: NavigationChartArche {
                marker: [12, 10],
                current_location: &CURRENT_LOCATION,
                current_location_kind: NavigationChartObjectKind::default(),
            },
            wipe_endpoints: &endpoints,
            comparison_extent: &(),
        };
        let mut host = OracleHost {
            events: Vec::new(),
            copies: Vec::new(),
            entity_states: Vec::new(),
            chart_objects: Vec::new(),
            picked: None,
            wipe_spans: Box::default(),
            label_width: u16::MIN,
        };

        assert!(matches!(
            update_navigation_camera(context, &mut state, &mut host),
            Err(NavigationCameraError::TooManySecondaryMarkers { count })
                if count == SECONDARY_CHART_ENTITY_CAPACITY + 1
        ));
        assert!(host.events.is_empty());
        assert!(host.entity_states.is_empty());
    }

    fn state_for(vector: &CameraVector) -> NavigationCameraState<u16> {
        let mut state = NavigationCameraState {
            transition_step: vector.state_before,
            active: vector.active & 1 != u8::MIN,
            wipe_complete: !vector.name.contains("waits_for_wipe"),
            ui_active: false,
            chart_object_count: 41,
            secondary_marker_count: 2,
            entity_state_mask: 2,
            palette_refresh_enabled: true,
            hand: NavigationChartHandState {
                current: NavigationChartHand::Right,
                requested: NavigationChartHand::Left,
            },
            input: NavigationChartInputState {
                pointer: if vector.name.contains("new_right") {
                    [200, 80]
                } else if vector.name.contains("hover") {
                    [12, 7]
                } else {
                    [100, 70]
                },
                primary_pressed: vector.name.contains("click_"),
                press_pending: true,
            },
            panel: LocationInfoPanelState {
                selected_location: vector
                    .name
                    .contains("selected_panel")
                    .then_some(PICKED_LOCATION),
                deferred_record_link: Some(INITIAL_DEFERRED_LOCATION),
                ..LocationInfoPanelState::default()
            },
            panel_rects: LocationPanelRects {
                target: LocationPanelRect {
                    x: 110,
                    y: 25,
                    width: 96,
                    height: 70,
                },
                current: LocationPanelRect {
                    x: 123,
                    y: 77,
                    width: 9,
                    height: 9,
                },
            },
        };
        if vector.name.contains("closing_first_frame") {
            state.chart_object_count = 0;
        }
        state
    }

    fn host_for(vector: &CameraVector) -> OracleHost {
        let picked = if vector.name.contains("hover") || vector.name.contains("new_right") {
            Some(chart_object(
                PICKED_LOCATION,
                NavigationChartObjectKind {
                    ship: false,
                    black_hole: true,
                },
                b"ALPHA",
                [50, 60],
                true,
            ))
        } else if vector.name.contains("click_current") {
            Some(chart_object(
                CURRENT_LOCATION,
                NavigationChartObjectKind::default(),
                b"CURRENT",
                [12, 10],
                false,
            ))
        } else {
            None
        };
        OracleHost {
            events: Vec::new(),
            copies: Vec::new(),
            entity_states: Vec::new(),
            chart_objects: if vector.name.contains("builds_chart") {
                vec![
                    chart_object(
                        PICKED_LOCATION,
                        NavigationChartObjectKind {
                            ship: false,
                            black_hole: true,
                        },
                        b"ALPHA",
                        [50, 60],
                        true,
                    ),
                    chart_object(
                        4_352,
                        NavigationChartObjectKind {
                            ship: true,
                            black_hole: false,
                        },
                        b"BETA",
                        [100, 70],
                        false,
                    ),
                ]
            } else {
                Vec::new()
            },
            picked,
            wipe_spans: spans_for(&vector.name).into_boxed_slice(),
            label_width: 30,
        }
    }

    fn chart_object(
        id: u16,
        kind: NavigationChartObjectKind,
        name: &[u8],
        marker: [u16; 2],
        show_secondary_marker: bool,
    ) -> NavigationChartObject<u16> {
        NavigationChartObject {
            id,
            kind,
            name: Box::from(name),
            marker,
            show_secondary_marker,
        }
    }

    fn spans_for(name: &str) -> Vec<NavigationWipeSpan> {
        let values: &[(u16, u16)] = match name {
            "closing_completion_copies_outside_center" => &[(40, 60)],
            "closing_upper_half_copies_inside_then_tail" => &[(70, 180), (80, 160)],
            "opening_lower_half_reveals_rows_in_reverse" => &[(90, 140), (80, 160)],
            "opening_upper_half_copies_outside_center" => &[(65, 190)],
            "closing_first_frame_builds_chart_entities" => &[(75, 170)],
            "opening_first_frame_restores_panorama_buffer" => &[(100, 120)],
            _ => &[],
        };
        values
            .iter()
            .map(|(left, width)| NavigationWipeSpan {
                left: *left,
                width: *width,
            })
            .collect()
    }

    fn event_name(event: &Event) -> &'static str {
        match event {
            Event::Vga => "vga",
            Event::List(_) => "list",
            Event::Populate(_) => "populate",
            Event::Render => "render",
            Event::Transition(_) => "transition",
            Event::Wipe(_) => "wipe",
            Event::Dirty => "dirty",
            Event::Panorama => "panorama",
            Event::Reset => "reset",
            Event::Flip => "flip",
            Event::Panel => "panel",
            Event::Pick(_) => "pick",
            Event::Width(_) => "width",
            Event::Text { .. } => "text",
        }
    }

    fn assert_event_trace(vector: &CameraVector, events: &[Event]) {
        let expected: Vec<&str> = vector.calls.iter().map(|call| call.name.as_str()).collect();
        let actual: Vec<&str> = events.iter().map(event_name).collect();
        assert_eq!(actual, expected, "{}", vector.name);
    }

    fn assert_copy_trace(vector: &CameraVector, copies: &[NavigationChartCopySpan]) {
        assert_eq!(copies.len(), vector.copy_count, "{}", vector.name);
        let as_rows: Vec<[u16; 3]> = copies
            .iter()
            .map(|copy| [copy.x, copy.y, copy.width])
            .collect();
        assert_eq!(
            &as_rows[..as_rows.len().min(4)],
            vector.copy_head.as_slice(),
            "{}",
            vector.name
        );
        assert_eq!(
            &as_rows[as_rows.len().saturating_sub(4)..],
            vector.copy_tail.as_slice(),
            "{}",
            vector.name
        );
        let bytes: Vec<u8> = as_rows
            .iter()
            .flat_map(|row| row.iter().flat_map(|value| value.to_le_bytes()))
            .collect();
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            vector.copy_sha256,
            "{}",
            vector.name
        );
    }

    fn assert_state(
        vector: &CameraVector,
        state: &NavigationCameraState<u16>,
        host: &OracleHost,
        outcome: NavigationCameraOutcome,
    ) {
        assert_eq!(state.transition_step, vector.state_after, "{}", vector.name);
        let interactive = vector.state_before == u8::MIN
            && vector.active & 1 != u8::MIN
            && !vector.name.contains("waits_for_wipe");
        assert_eq!(state.ui_active, interactive, "{}", vector.name);

        if vector.state_before != u8::MIN {
            assert!(state.panel.selected_location.is_none(), "{}", vector.name);
        }
        if vector.name.contains("click_") {
            assert_eq!(
                state.hand.current,
                NavigationChartHand::Neutral,
                "{}",
                vector.name
            );
            assert_eq!(
                state.hand.requested,
                if vector.name.contains("new_right") {
                    NavigationChartHand::Right
                } else {
                    NavigationChartHand::Left
                },
                "{}",
                vector.name
            );
            assert!(!state.input.primary_pressed, "{}", vector.name);
            assert!(!state.input.press_pending, "{}", vector.name);
        }
        if vector.name.contains("new_right") {
            assert_eq!(state.panel.selected_location, Some(PICKED_LOCATION));
            assert_eq!(state.panel.deferred_record_link, Some(PICKED_LOCATION));
            assert_eq!(state.panel.phase, LocationPanelPhase::Opening);
            assert!(state.panel.active);
            assert_eq!(state.panel.geometry.scale_step, u8::MIN);
            assert_eq!(
                state.panel.transition,
                LocationPanelTransitionProgress {
                    current: u8::MIN,
                    total: FIRST_TRANSITION_STEP,
                }
            );
            assert_eq!(
                state.panel_rects.current,
                LocationPanelRect {
                    x: 200,
                    y: 80,
                    width: POINTER_PANEL_EXTENT,
                    height: POINTER_PANEL_EXTENT,
                }
            );
        }
        if vector.name.contains("builds_chart") {
            assert_eq!(state.chart_object_count, 2);
            assert_eq!(state.secondary_marker_count, 1);
        }
        if vector.name.contains("restores_panorama") {
            assert!(!state.wipe_complete);
            assert!(!state.palette_refresh_enabled);
        }
        if vector.name == "closing_completion_copies_outside_center" {
            assert!(state.wipe_complete);
        }

        let expected_outcome = match vector.name.as_str() {
            "inactive" => NavigationCameraOutcome::Inactive,
            "interactive_waits_for_wipe" => NavigationCameraOutcome::WaitingForWipe,
            "selected_panel_forwards_inherited_extent" => NavigationCameraOutcome::LocationPanel,
            "hover_draws_clamped_object_label" => {
                NavigationCameraOutcome::HoverLabel { position: [0, 0] }
            }
            "click_current_location_only_updates_hand_and_input" => {
                NavigationCameraOutcome::CurrentLocation
            }
            "click_new_right_location_starts_panel" => NavigationCameraOutcome::LocationPanelOpened,
            _ => NavigationCameraOutcome::TransitionFrame {
                direction: if vector.active & 1 != u8::MIN {
                    NavigationChartWipeDirection::Opening
                } else {
                    NavigationChartWipeDirection::Closing
                },
                first_frame: vector.state_before == FIRST_TRANSITION_STEP,
                copy_count: vector.copy_count,
            },
        };
        assert_eq!(outcome, expected_outcome, "{}", vector.name);

        let expected_entity_states = if matches!(
            vector.name.as_str(),
            "hover_draws_clamped_object_label"
                | "click_current_location_only_updates_hand_and_input"
                | "click_new_right_location_starts_panel"
        ) {
            vec![
                NavigationChartEntityState {
                    entity: 5,
                    visible: true,
                    active: false,
                },
                NavigationChartEntityState {
                    entity: 6,
                    visible: true,
                    active: false,
                },
                NavigationChartEntityState {
                    entity: 1,
                    visible: true,
                    active: false,
                },
            ]
        } else {
            Vec::new()
        };
        assert_eq!(
            host.entity_states, expected_entity_states,
            "{}",
            vector.name
        );
    }

    fn assert_call_details(vector: &CameraVector, events: &[Event]) {
        let expected_populates: Vec<NavigationChartEntityDraw> = vector
            .calls
            .iter()
            .filter(|call| call.name == "populate")
            .map(|call| NavigationChartEntityDraw {
                entity: call.entity.unwrap(),
                resource: call.resource.unwrap(),
                position: call.position.unwrap(),
                frame: call.frame.unwrap(),
            })
            .collect();
        let actual_populates: Vec<NavigationChartEntityDraw> = events
            .iter()
            .filter_map(|event| match event {
                Event::Populate(draw) => Some(*draw),
                _ => None,
            })
            .collect();
        assert_eq!(actual_populates, expected_populates, "{}", vector.name);

        let expected_transitions: Vec<u16> = vector
            .calls
            .iter()
            .filter_map(|call| {
                if call.name == "transition" {
                    Some(call.entity.unwrap())
                } else {
                    None
                }
            })
            .collect();
        let actual_transitions: Vec<u16> = events
            .iter()
            .filter_map(|event| match event {
                Event::Transition(entity) => Some(*entity),
                _ => None,
            })
            .collect();
        assert_eq!(actual_transitions, expected_transitions, "{}", vector.name);

        for event in events {
            match event {
                Event::List(count) => {
                    let expected = vector
                        .calls
                        .iter()
                        .find(|call| call.name == "list")
                        .and_then(|call| call.result)
                        .unwrap();
                    assert_eq!(*count, usize::from(expected), "{}", vector.name);
                }
                Event::Wipe(endpoint) => {
                    let expected = vector.calls.iter().find_map(|call| call.endpoint).unwrap();
                    assert_eq!(*endpoint, expected, "{}", vector.name);
                }
                Event::Width(text) => {
                    let expected = vector
                        .calls
                        .iter()
                        .find(|call| call.name == "width")
                        .and_then(|call| call.text.as_deref())
                        .unwrap();
                    assert_eq!(text.as_ref(), expected.as_bytes(), "{}", vector.name);
                }
                Event::Text {
                    text,
                    position,
                    color,
                } => {
                    let expected = vector
                        .calls
                        .iter()
                        .find(|call| call.name == "text")
                        .unwrap();
                    assert_eq!(
                        text.as_ref(),
                        expected.text.as_deref().unwrap().as_bytes(),
                        "{}",
                        vector.name
                    );
                    assert_eq!(*position, expected.position.unwrap(), "{}", vector.name);
                    assert_eq!(*color, expected.color.unwrap(), "{}", vector.name);
                }
                Event::Render => {
                    let expected_ranges: Vec<[u16; 2]> =
                        vector.calls.iter().filter_map(|call| call.range).collect();
                    assert!(expected_ranges.contains(&[0, 0]), "{}", vector.name);
                }
                Event::Pick(result) => {
                    let expected = vector
                        .calls
                        .iter()
                        .find(|call| call.name == "pick")
                        .and_then(|call| call.result);
                    assert_eq!(*result, expected, "{}", vector.name);
                }
                Event::Vga
                | Event::Populate(_)
                | Event::Transition(_)
                | Event::Dirty
                | Event::Panorama
                | Event::Reset
                | Event::Flip
                | Event::Panel => {}
            }
        }
    }
}
