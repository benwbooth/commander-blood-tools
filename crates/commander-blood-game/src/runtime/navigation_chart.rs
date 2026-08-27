//! Concrete flat-memory host for the recovered navigation chart and location panel.

use anyhow::{Context, Result, anyhow, bail};
use commander_blood_formats::bloodprg::BLOODPRG_NAVIGATION_WIPE_ENDPOINT_COUNT;
use commander_blood_formats::script::{
    ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateObjectReference, ScriptStateWordPair,
};

use crate::native::bloodprg::{
    BridgeSpriteExtent, BridgeSpritePosition, FontPoint, GameLifecycleState, LoadedScriptProfile,
    LocationInfoPanelContext, LocationInfoPanelHost, LocationInfoPanelState, LocationPanelArtwork,
    LocationPanelInterpolation, LocationPanelLocation, LocationPanelRect, LocationPanelRects,
    LocationPanelSource, LocationPanelSpriteRange, LocationPanelTextDraw,
    LocationPanelTransitionProgress, NavigationCameraContext, NavigationCameraHost,
    NavigationCameraOutcome, NavigationCameraState, NavigationChartArche, NavigationChartCopySpan,
    NavigationChartEntityDraw, NavigationChartEntityState, NavigationChartHand,
    NavigationChartMarkerEndpoint, NavigationChartObject, NavigationChartObjectKind,
    NavigationChartPickObject, NavigationChartPickOutcome, NavigationChartPickState,
    NavigationStatusLabels, NavigationStatusLocationKind, RasterPoint, RasterRectOutcome,
    ResourceId, ScriptFieldSelector, ScriptObjectFlag, build_navigation_wipe_spans,
    copy_work_surface_span, navigation_chart_objects, navigation_source_objects, object_has_flag,
    pick_navigation_chart_object, resolve_navigation_position, script_field_offset,
    update_location_info_panel, update_location_panel_geometry, update_navigation_camera,
};

use super::{LOGICAL_FRAMEBUFFER_PIXEL_COUNT, ModernGameServices, OriginalGameRuntime};

const OBJECT_ACCESS_COUNTER_BYTE_OFFSET: usize = 20;
const WORD_BYTE_COUNT: usize = size_of::<u16>();
const CHART_PRIMARY_ENTITY: usize = 0;
const AFTER_CHART_PRIMARY_ENTITY: u16 = 1;
const LOCATION_PANEL_ENTITY: usize = 0;
const AFTER_LOCATION_PANEL_ENTITY: u16 = 1;
const AFTER_LOCATION_PANEL_TRANSITION_ENTITY: u16 = 2;
const NEUTRAL_HAND_SELECTOR: u16 = 0;
const LEFT_HAND_SELECTOR: u16 = 11;
const RIGHT_HAND_SELECTOR: u16 = 12;
const LOCATION_PANEL_TARGET_RECT: LocationPanelRect = LocationPanelRect {
    x: 110,
    y: 25,
    width: 96,
    height: 70,
};

/// Persistent state and flat scratch pages for `nav_camera_state_check`.
pub(super) struct RuntimeNavigationChart {
    state: NavigationCameraState<ScriptObjectId>,
    pick_state: NavigationChartPickState,
    work_surface: Box<[u8]>,
    staging_surface: Box<[u8]>,
    pending_spans: Vec<NavigationChartCopySpan>,
    status_snapshot: Option<RuntimeNavigationStatusSnapshot>,
}

impl Default for RuntimeNavigationChart {
    fn default() -> Self {
        let mut state = NavigationCameraState::default();
        state.panel_rects.target = LOCATION_PANEL_TARGET_RECT;
        Self {
            state,
            pick_state: NavigationChartPickState {
                marker_extent: [u16::MIN; 2],
                marker_endpoint: NavigationChartMarkerEndpoint::Near,
            },
            work_surface: vec![u8::MIN; LOGICAL_FRAMEBUFFER_PIXEL_COUNT].into_boxed_slice(),
            staging_surface: vec![u8::MIN; LOGICAL_FRAMEBUFFER_PIXEL_COUNT].into_boxed_slice(),
            pending_spans: Vec::new(),
            status_snapshot: None,
        }
    }
}

impl RuntimeNavigationChart {
    /// Remaining camera-actor transition steps after the latest chart frame.
    pub(super) const fn transition_step(&self) -> u8 {
        self.state.transition_step
    }

    /// Whether the location-information panel currently owns chart interaction.
    pub(super) fn location_panel_active(&self) -> bool {
        self.state.panel.active || self.state.panel.selected_location.is_some()
    }

    /// Return the current typed location and roster captured during chart work.
    pub(super) fn status_snapshot(&self) -> Option<RuntimeNavigationStatusSnapshot> {
        self.status_snapshot.clone()
    }

    /// Number of in-play destinations captured on the first closing frame.
    #[cfg(test)]
    pub(super) const fn chart_object_count(&self) -> usize {
        self.state.chart_object_count
    }

    /// Current flat source page retained by the real-data lifecycle test.
    #[cfg(test)]
    pub(super) fn work_surface(&self) -> &[u8] {
        &self.work_surface
    }

    /// Advance one exact chart transition, hover, click, or location-panel frame.
    pub(super) fn update(
        &mut self,
        services: &mut ModernGameServices<'_>,
        lifecycle: &mut GameLifecycleState,
        transition_step: u8,
        navigation_animation_phase: u8,
        comparison_extent: BridgeSpriteExtent,
    ) -> Result<NavigationCameraOutcome> {
        let world = RuntimeNavigationWorld::decode(services.runtime())?;
        self.status_snapshot = Some(world.status_snapshot());
        let pointer = services.input().pointer_sample();
        self.state.transition_step = transition_step;
        self.state.active = services.bridge_camera_view_active();
        self.state.entity_state_mask = navigation_animation_phase;
        self.state.input.pointer = pointer.position.map(|coordinate| coordinate as u16);
        self.state.input.primary_pressed = lifecycle.primary_pointer_pressed;
        self.state.input.press_pending = lifecycle.pointer_press_pending != u8::MIN;
        let hand_before = self.state.hand;

        let context = NavigationCameraContext {
            arche: NavigationChartArche {
                marker: world.arche_marker,
                current_location: &world.current_location,
                current_location_kind: world.current_location_kind,
            },
            wipe_endpoints: &world.wipe_endpoints,
            comparison_extent: &comparison_extent,
        };
        let mut backend = RuntimeNavigationChartBackend {
            services,
            world: &world,
            work_surface: &mut self.work_surface,
            staging_surface: &mut self.staging_surface,
            pending_spans: &mut self.pending_spans,
            pick_state: &mut self.pick_state,
            panel_rects: self.state.panel_rects,
            pointer: self.state.input.pointer,
            primary_pressed: self.state.input.primary_pressed,
            callback_error: None,
        };
        let outcome = update_navigation_camera(context, &mut self.state, &mut backend)
            .map_err(|error| anyhow!(error))?;
        backend.finish_callbacks()?;

        lifecycle.primary_pointer_pressed = self.state.input.primary_pressed;
        if !self.state.input.press_pending {
            lifecycle.pointer_press_pending = u8::MIN;
        }
        lifecycle.set_presentation_interface_active(self.state.ui_active);
        if self.state.hand != hand_before {
            let hand = services.manu3_hand_state_mut();
            hand.current_animation = hand_selector(self.state.hand.current);
            hand.requested_animation = hand_selector(self.state.hand.requested);
        }
        Ok(outcome)
    }
}

#[derive(Clone)]
struct RuntimeNavigationLocation {
    id: ScriptObjectId,
    kind: NavigationStatusLocationKind,
    name: Box<[u8]>,
    sources: Vec<RuntimeNavigationSource>,
}

#[derive(Clone)]
struct RuntimeNavigationSource {
    kind: ScriptObjectKind,
    active: bool,
    life_support_visits: u16,
    location: Option<ScriptObjectId>,
    name: Box<[u8]>,
}

struct RuntimeNavigationArtwork {
    name: Box<[u8]>,
    resource: ResourceId,
}

#[derive(Clone)]
struct RuntimeNavigationLabels {
    planet: Box<[u8]>,
    ship: Box<[u8]>,
    black_hole: Box<[u8]>,
    life_support: Box<[u8]>,
}

struct RuntimeNavigationWorld {
    arche_marker: [u16; 2],
    arche_endpoint_context: u16,
    current_location: ScriptObjectId,
    current_location_kind: NavigationChartObjectKind,
    chart_objects: Vec<NavigationChartObject<ScriptObjectId>>,
    pick_objects: Vec<NavigationChartPickObject<ScriptObjectId, u16>>,
    locations: Vec<RuntimeNavigationLocation>,
    artwork: Vec<RuntimeNavigationArtwork>,
    labels: RuntimeNavigationLabels,
    status_snapshot: RuntimeNavigationStatusSnapshot,
    wipe_endpoints: [[u16; 2]; BLOODPRG_NAVIGATION_WIPE_ENDPOINT_COUNT],
}

/// Owned location data shared with the late bridge navigation-status pass.
#[derive(Clone)]
pub(super) struct RuntimeNavigationStatusSnapshot {
    pub(super) location_kind: NavigationStatusLocationKind,
    pub(super) location_name: Box<[u8]>,
    pub(super) sources: Vec<RuntimeNavigationStatusSource>,
    pub(super) ark_location: ScriptObjectId,
    pub(super) labels: RuntimeNavigationStatusLabels,
}

/// One typed descendant considered by the navigation-status roster filter.
#[derive(Clone)]
pub(super) struct RuntimeNavigationStatusSource {
    pub(super) kind: ScriptObjectKind,
    pub(super) active: bool,
    pub(super) life_support_visits: u16,
    pub(super) location: Option<ScriptObjectId>,
    pub(super) name: Box<[u8]>,
}

/// Owned executable-authored labels used by the navigation-status composer.
#[derive(Clone)]
pub(super) struct RuntimeNavigationStatusLabels {
    pub(super) planet: Box<[u8]>,
    pub(super) ship: Box<[u8]>,
    pub(super) black_hole: Box<[u8]>,
    pub(super) life_support: Box<[u8]>,
}

impl RuntimeNavigationWorld {
    fn decode(runtime: &OriginalGameRuntime) -> Result<Self> {
        let profile = runtime
            .current_profile()
            .context("navigation chart requires a loaded BloodScript profile")?;
        let arche = profile
            .builtins()
            .archetype
            .context("loaded BloodScript profile has no Arche object")?;
        let ark_location = profile
            .builtins()
            .ark
            .context("loaded BloodScript profile has no Ark object")?;
        let current_location = super::ship_target::ship_hud_arche_link(profile.state(), arche)?.0;
        let current_location_object_kind = object_kind(profile, current_location)?;
        let current_location_kind = chart_kind(current_location_object_kind);
        let arche_marker = read_position(
            profile.state(),
            resolve_navigation_position(profile.state(), arche, arche, u16::MIN)
                .context("resolving Arche's chart marker")?,
        )?;
        let arche_endpoint_context =
            read_object_word(profile, arche, ScriptFieldSelector::BLACK_HOLE_RELATION)?;

        let mut chart_objects = Vec::new();
        let mut pick_objects = Vec::new();
        let mut locations = Vec::new();
        for object in navigation_chart_objects(profile.state()) {
            let object_kind = object_kind(profile, object)?;
            let kind = chart_kind(object_kind);
            let comparison = if kind.black_hole {
                read_object_word(profile, object, ScriptFieldSelector::BLACK_HOLE_COMPARISON)?
            } else {
                arche_endpoint_context
            };
            let near_marker = read_position(
                profile.state(),
                resolve_navigation_position(profile.state(), object, arche, comparison)
                    .with_context(|| format!("resolving near marker for {object:?}"))?,
            )?;
            let far_marker = if kind.black_hole {
                read_position(
                    profile.state(),
                    resolve_navigation_position(
                        profile.state(),
                        object,
                        arche,
                        comparison.wrapping_add(1),
                    )
                    .with_context(|| format!("resolving far marker for {object:?}"))?,
                )?
            } else {
                near_marker
            };
            let name: Box<[u8]> = profile
                .directory()
                .object(object)
                .with_context(|| format!("navigation object {object:?} has no directory entry"))?
                .name()
                .into();
            let show_secondary_marker = object_access_counter(profile.state(), object)? == u16::MIN;
            chart_objects.push(NavigationChartObject {
                id: object,
                kind,
                name: name.clone(),
                marker: near_marker,
                show_secondary_marker,
            });
            pick_objects.push(NavigationChartPickObject {
                record: object,
                is_ship: kind.ship,
                is_black_hole: kind.black_hole,
                endpoint_context: comparison,
                near_marker,
                far_marker,
            });
            locations.push(RuntimeNavigationLocation {
                id: object,
                kind: status_kind(object_kind),
                name,
                sources: location_sources(profile, object)?,
            });
        }
        let hidden_locations = profile
            .state()
            .objects()
            .iter()
            .filter(|object| {
                chart_kind_supported(object.kind)
                    && !locations.iter().any(|location| location.id == object.id)
            })
            .map(|object| (object.id, object.kind))
            .collect::<Vec<_>>();
        for (object, kind) in hidden_locations {
            let name = profile
                .directory()
                .object(object)
                .with_context(|| format!("navigation location {object:?} has no directory entry"))?
                .name()
                .into();
            locations.push(RuntimeNavigationLocation {
                id: object,
                kind: status_kind(kind),
                name,
                sources: location_sources(profile, object)?,
            });
        }

        let navigation = runtime.data().navigation_resources();
        let decoded_labels = navigation.labels();
        let labels = RuntimeNavigationLabels {
            planet: decoded_labels.planet().into(),
            ship: decoded_labels.ship().into(),
            black_hole: decoded_labels.black_hole().into(),
            life_support: decoded_labels.life_support().into(),
        };
        let status_snapshot = RuntimeNavigationStatusSnapshot {
            location_kind: status_kind(current_location_object_kind),
            location_name: profile
                .directory()
                .object(current_location)
                .with_context(|| {
                    format!("current navigation location {current_location:?} has no name")
                })?
                .name()
                .into(),
            sources: location_sources(profile, current_location)?
                .into_iter()
                .map(|source| RuntimeNavigationStatusSource {
                    kind: source.kind,
                    active: source.active,
                    life_support_visits: source.life_support_visits,
                    location: source.location,
                    name: source.name,
                })
                .collect(),
            ark_location,
            labels: RuntimeNavigationStatusLabels {
                planet: labels.planet.clone(),
                ship: labels.ship.clone(),
                black_hole: labels.black_hole.clone(),
                life_support: labels.life_support.clone(),
            },
        };
        let artwork = runtime
            .data()
            .world_artwork_layout()
            .iter()
            .map(|entry| RuntimeNavigationArtwork {
                name: entry.name().into(),
                resource: ResourceId::new(entry.resource_id),
            })
            .collect();
        Ok(Self {
            arche_marker,
            arche_endpoint_context,
            current_location,
            current_location_kind,
            chart_objects,
            pick_objects,
            locations,
            artwork,
            labels,
            status_snapshot,
            wipe_endpoints: *navigation.wipe_endpoints(),
        })
    }

    fn location(&self, id: ScriptObjectId) -> Result<&RuntimeNavigationLocation> {
        self.locations
            .iter()
            .find(|location| location.id == id)
            .with_context(|| format!("selected navigation location {id:?} is absent"))
    }

    fn status_snapshot(&self) -> RuntimeNavigationStatusSnapshot {
        self.status_snapshot.clone()
    }

    fn chart_object(&self, id: ScriptObjectId) -> Result<NavigationChartObject<ScriptObjectId>> {
        self.chart_objects
            .iter()
            .find(|object| object.id == id)
            .cloned()
            .with_context(|| format!("picked navigation object {id:?} is absent"))
    }
}

struct RuntimeNavigationChartBackend<'state, 'window> {
    services: &'state mut ModernGameServices<'window>,
    world: &'state RuntimeNavigationWorld,
    work_surface: &'state mut Box<[u8]>,
    staging_surface: &'state mut Box<[u8]>,
    pending_spans: &'state mut Vec<NavigationChartCopySpan>,
    pick_state: &'state mut NavigationChartPickState,
    panel_rects: LocationPanelRects,
    pointer: [u16; 2],
    primary_pressed: bool,
    callback_error: Option<anyhow::Error>,
}

impl RuntimeNavigationChartBackend<'_, '_> {
    fn record_callback<T>(&mut self, result: Result<T>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                if self.callback_error.is_none() {
                    self.callback_error = Some(error);
                }
                None
            }
        }
    }

    fn finish_callbacks(&mut self) -> Result<()> {
        self.callback_error.take().map_or(Ok(()), Err)
    }
}

impl NavigationCameraHost<ScriptObjectId, BridgeSpriteExtent>
    for RuntimeNavigationChartBackend<'_, '_>
{
    type Error = anyhow::Error;

    fn copy_chart_background_to_work_surface(&mut self) {
        self.work_surface
            .copy_from_slice(self.services.runtime().back_buffer().pixels());
    }

    fn chart_objects(&mut self) -> Result<Vec<NavigationChartObject<ScriptObjectId>>> {
        Ok(self.world.chart_objects.clone())
    }

    fn populate_chart_entity(&mut self, draw: NavigationChartEntityDraw) {
        let result = self
            .services
            .runtime_mut()
            .populate_cached_bridge_sprite(
                usize::from(draw.entity),
                ResourceId::new(draw.resource),
                BridgeSpritePosition {
                    x: draw.position[0],
                    y: draw.position[1],
                },
                usize::from(draw.frame),
            )
            .and_then(|populated| {
                if populated {
                    Ok(())
                } else {
                    bail!(
                        "navigation resource {} has no frame {}",
                        draw.resource,
                        draw.frame
                    )
                }
            });
        self.record_callback(result);
    }

    fn render_primary_chart_entity(&mut self) {
        let result = self
            .services
            .runtime_mut()
            .rasterize_ship_entity_range(
                CHART_PRIMARY_ENTITY as u16..AFTER_CHART_PRIMARY_ENTITY,
                self.work_surface,
            )
            .map(|_| ());
        self.record_callback(result);
    }

    fn transition_chart_entity(&mut self, entity: u16) {
        let result = self
            .services
            .runtime_mut()
            .transition_bridge_sprite(usize::from(entity))
            .map(|_| ());
        self.record_callback(result);
    }

    fn build_wipe_spans(
        &mut self,
        endpoint: [u16; 2],
    ) -> Result<Box<[crate::native::bloodprg::NavigationWipeSpan]>> {
        build_navigation_wipe_spans(endpoint).map_err(Into::into)
    }

    fn copy_back_buffer_span(&mut self, span: NavigationChartCopySpan) {
        let result = copy_work_surface_span(
            self.work_surface,
            self.staging_surface,
            usize::from(span.x),
            usize::from(span.y),
            usize::from(span.width),
        )
        .context("staging a navigation wipe span");
        if self.record_callback(result).is_some() {
            self.pending_spans.push(span);
        }
    }

    fn publish_transition_dirty_rects(&mut self) {
        for span in std::mem::take(self.pending_spans) {
            let result = self.services.runtime_mut().publish_work_surface_span(
                self.staging_surface,
                span.x,
                span.y,
                span.width,
            );
            self.record_callback(result);
        }
    }

    fn load_bridge_panorama(&mut self) {
        let result = self
            .services
            .render_current_bridge_frame_with_palette_refresh(false)
            .map(|_| ());
        self.record_callback(result);
    }

    fn snapshot_ship_hud_and_reset_camera(&mut self) {
        let result = self.services.snapshot_navigation_hud_palette_and_camera();
        self.record_callback(result);
    }

    fn present_restored_panorama(&mut self) {
        let result = match self
            .services
            .render_current_bridge_frame_with_palette_refresh(false)
        {
            Ok(_) => self
                .services
                .compose_current_bridge_work_surface(self.work_surface),
            Err(error) => Err(error),
        };
        self.record_callback(result);
    }

    fn update_location_panel(
        &mut self,
        panel: &mut LocationInfoPanelState<ScriptObjectId>,
        comparison_extent: &BridgeSpriteExtent,
    ) -> Result<()> {
        let selected_id = panel
            .selected_location
            .context("location panel has no selected object")?;
        let selected = self.world.location(selected_id)?;
        let artwork = self
            .world
            .artwork
            .iter()
            .map(|entry| LocationPanelArtwork {
                location_name: entry.name.as_ref(),
                resource_id: entry.resource,
            })
            .collect::<Vec<_>>();
        let context = LocationInfoPanelContext {
            selected: &LocationPanelLocation {
                id: selected.id,
                kind: selected.kind,
                name: &selected.name,
            },
            artwork: &artwork,
            labels: navigation_labels(&self.world.labels),
            rects: self.panel_rects,
            pointer: self.pointer,
            primary_pressed: self.primary_pressed,
            comparison_extent,
        };
        let mut backend = RuntimeLocationPanelBackend {
            services: self.services,
            sources: &selected.sources,
            callback_error: None,
        };
        update_location_info_panel(context, panel, &mut backend)?;
        backend.finish_callbacks()
    }

    fn set_chart_entity_state(&mut self, state: NavigationChartEntityState) {
        let result = self
            .services
            .runtime_mut()
            .publish_navigation_sprite_state(usize::from(state.entity), state.active);
        self.record_callback(result);
    }

    fn pick_chart_object(&mut self) -> Result<Option<NavigationChartObject<ScriptObjectId>>> {
        match pick_navigation_chart_object(
            &self.world.pick_objects,
            &self.world.arche_endpoint_context,
            self.pointer,
            self.pick_state,
        ) {
            NavigationChartPickOutcome::None => Ok(None),
            NavigationChartPickOutcome::Picked { record, .. } => {
                self.world.chart_object(record).map(Some)
            }
        }
    }

    fn measure_hover_label(&mut self, text: &[u8]) -> u16 {
        let result = self.services.runtime().measure_main_font_line(text);
        self.record_callback(result).unwrap_or(u16::MIN)
    }

    fn draw_hover_label(&mut self, text: &[u8], position: [u16; 2], color: u8) {
        let result = self
            .services
            .runtime_mut()
            .draw_main_font_line(
                text,
                FontPoint {
                    x: i32::from(position[0]),
                    y: i32::from(position[1]),
                },
                color,
            )
            .map(|_| ());
        self.record_callback(result);
    }
}

struct RuntimeLocationPanelBackend<'state, 'window> {
    services: &'state mut ModernGameServices<'window>,
    sources: &'state [RuntimeNavigationSource],
    callback_error: Option<anyhow::Error>,
}

impl RuntimeLocationPanelBackend<'_, '_> {
    fn record_callback<T>(&mut self, result: Result<T>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                if self.callback_error.is_none() {
                    self.callback_error = Some(error);
                }
                None
            }
        }
    }

    fn finish_callbacks(&mut self) -> Result<()> {
        self.callback_error.take().map_or(Ok(()), Err)
    }

    fn remap_rect(&mut self, rect: LocationPanelRect) -> Result<()> {
        let outcome = self.services.runtime_mut().remap_bridge_dark_region(
            RasterPoint {
                x: i32::from(rect.x),
                y: i32::from(rect.y),
            },
            rect.width as u16,
            rect.height as u16,
        )?;
        if outcome == RasterRectOutcome::Rejected {
            bail!("navigation panel rectangle {rect:?} is outside the logical display");
        }
        Ok(())
    }
}

impl LocationInfoPanelHost<ResourceId, ScriptObjectId, BridgeSpriteExtent>
    for RuntimeLocationPanelBackend<'_, '_>
{
    type Artwork = ResourceId;
    type Error = anyhow::Error;

    fn load_panel_artwork(&mut self, resource_id: &ResourceId) -> Result<Self::Artwork> {
        self.services
            .runtime_mut()
            .load_cached_palette_sprite(*resource_id)?;
        Ok(*resource_id)
    }

    fn install_panel_artwork(&mut self, artwork: Self::Artwork, pointer: [u16; 2]) -> u16 {
        let result = self
            .services
            .runtime_mut()
            .populate_cached_bridge_sprite(
                LOCATION_PANEL_ENTITY,
                artwork,
                BridgeSpritePosition {
                    x: pointer[0],
                    y: pointer[1],
                },
                usize::MIN,
            )
            .and_then(|populated| {
                if populated {
                    self.services
                        .runtime()
                        .bridge_sprite_source_extent(LOCATION_PANEL_ENTITY)
                        .map(|extent| extent.width)
                } else {
                    bail!(
                        "location-panel resource {} has no first frame",
                        artwork.value()
                    )
                }
            });
        self.record_callback(result).unwrap_or(u16::MIN)
    }

    fn prepare_panel_palette(&mut self) {
        let result = self
            .services
            .runtime_mut()
            .rebuild_bridge_dark_remap_table();
        self.record_callback(result);
    }

    fn update_panel_geometry(
        &mut self,
        geometry: &mut crate::native::bloodprg::LocationPanelGeometryState,
        comparison_extent: &BridgeSpriteExtent,
    ) {
        let source_extent = self
            .services
            .runtime()
            .bridge_sprite_source_extent(LOCATION_PANEL_ENTITY);
        let Some(source_extent) = self.record_callback(source_extent) else {
            return;
        };
        let mut backend = RuntimeLocationPanelGeometryBackend {
            runtime: self.services.runtime_mut(),
            callback_error: &mut self.callback_error,
        };
        update_location_panel_geometry(
            geometry,
            [source_extent.width, source_extent.height],
            comparison_extent,
            &mut backend,
        );
    }

    fn render_panel_sprites(&mut self, range: LocationPanelSpriteRange) {
        let entities = match range {
            LocationPanelSpriteRange::PanelOnly => 0..AFTER_LOCATION_PANEL_ENTITY,
            LocationPanelSpriteRange::PanelAndTransition => {
                0..AFTER_LOCATION_PANEL_TRANSITION_ENTITY
            }
        };
        let result = self
            .services
            .runtime_mut()
            .rasterize_ship_entity_range_to_front(entities)
            .map(|_| ());
        self.record_callback(result);
    }

    fn interpolate_panel(
        &mut self,
        direction: LocationPanelInterpolation,
        rects: LocationPanelRects,
        progress: &mut LocationPanelTransitionProgress,
    ) {
        let result = interpolate_panel_rect(direction, rects, progress)
            .and_then(|rect| rect.map_or(Ok(()), |rect| self.remap_rect(rect)));
        self.record_callback(result);
    }

    fn remap_panel_rect(&mut self, rect: LocationPanelRect) {
        let result = self.remap_rect(rect);
        self.record_callback(result);
    }

    fn draw_panel_text(&mut self, draw: LocationPanelTextDraw<'_>) -> u16 {
        let result = self
            .services
            .runtime_mut()
            .draw_main_font_line(
                draw.text,
                FontPoint {
                    x: i32::from(draw.position[0]),
                    y: i32::from(draw.position[1]),
                },
                draw.color,
            )
            .map(|outcome| outcome.draw_width);
        self.record_callback(result).unwrap_or(u16::MIN)
    }

    fn navigation_sources(
        &mut self,
        _location: &ScriptObjectId,
    ) -> Result<Vec<LocationPanelSource>> {
        Ok(self
            .sources
            .iter()
            .map(|source| LocationPanelSource {
                kind: source.kind,
                active: source.active,
                life_support_visits: source.life_support_visits,
                name: source.name.clone(),
            })
            .collect())
    }

    fn release_panel_entity(&mut self) {
        let result = self
            .services
            .runtime_mut()
            .transition_bridge_sprite(LOCATION_PANEL_ENTITY)
            .map(|_| ());
        self.record_callback(result);
    }
}

struct RuntimeLocationPanelGeometryBackend<'state> {
    runtime: &'state mut OriginalGameRuntime,
    callback_error: &'state mut Option<anyhow::Error>,
}

impl crate::native::bloodprg::LocationPanelGeometryHost<BridgeSpriteExtent>
    for RuntimeLocationPanelGeometryBackend<'_>
{
    fn update_panel_extent(
        &mut self,
        extent: [u16; 2],
        comparison_extent: &BridgeSpriteExtent,
        _source_width: &mut u16,
        _layout: &mut crate::native::bloodprg::LocationPanelLayout,
    ) {
        let result = self.runtime.update_bridge_sprite_extent(
            LOCATION_PANEL_ENTITY,
            BridgeSpriteExtent {
                width: extent[0],
                height: extent[1],
            },
            *comparison_extent,
        );
        record_geometry_callback(self.callback_error, result);
    }

    fn update_panel_position(&mut self, position: [u16; 2]) {
        let result = self.runtime.update_bridge_sprite_position(
            LOCATION_PANEL_ENTITY,
            BridgeSpritePosition {
                x: position[0],
                y: position[1],
            },
        );
        record_geometry_callback(self.callback_error, result);
    }
}

fn record_geometry_callback(error_slot: &mut Option<anyhow::Error>, result: Result<()>) {
    if let Err(error) = result
        && error_slot.is_none()
    {
        *error_slot = Some(error);
    }
}

fn interpolate_panel_rect(
    direction: LocationPanelInterpolation,
    rects: LocationPanelRects,
    progress: &mut LocationPanelTransitionProgress,
) -> Result<Option<LocationPanelRect>> {
    if progress.current == progress.total {
        return Ok(None);
    }
    let total = progress.total as i8;
    if total == 0 {
        bail!("location-panel interpolation has zero total steps");
    }
    progress.current = progress.current.wrapping_add(1);
    let (source, target) = match direction {
        LocationPanelInterpolation::Opening => (rects.target, rects.current),
        LocationPanelInterpolation::Closing => (rects.current, rects.target),
    };
    Ok(Some(LocationPanelRect {
        x: interpolate_rect_field(source.x, target.x, total, progress.current),
        y: interpolate_rect_field(source.y, target.y, total, progress.current),
        width: interpolate_rect_field(source.width, target.width, total, progress.current),
        height: interpolate_rect_field(source.height, target.height, total, progress.current),
    }))
}

fn interpolate_rect_field(source: i16, target: i16, total: i8, current: u8) -> i16 {
    let delta = (source as u16).wrapping_sub(target as u16) as i16;
    let quotient = (delta / i16::from(total)) as i8;
    (target as u16).wrapping_add(i16::from(quotient).wrapping_mul(i16::from(current as i8)) as u16)
        as i16
}

fn navigation_labels(labels: &RuntimeNavigationLabels) -> NavigationStatusLabels<'_> {
    NavigationStatusLabels {
        planet: &labels.planet,
        ship: &labels.ship,
        black_hole: &labels.black_hole,
        life_support: &labels.life_support,
    }
}

fn location_sources(
    profile: &LoadedScriptProfile,
    location: ScriptObjectId,
) -> Result<Vec<RuntimeNavigationSource>> {
    navigation_source_objects(profile.state(), location)
        .context("building location-panel source objects")?
        .into_iter()
        .map(|source| {
            let object = profile
                .state()
                .object(source)
                .with_context(|| format!("navigation source {source:?} is absent"))?;
            let name = profile
                .directory()
                .object(source)
                .with_context(|| format!("navigation source {source:?} has no name"))?
                .name()
                .into();
            Ok(RuntimeNavigationSource {
                kind: object.kind,
                active: object_has_flag(profile.state(), source, ScriptObjectFlag::Active)
                    .unwrap_or(false),
                life_support_visits: read_optional_object_word(
                    profile,
                    source,
                    ScriptFieldSelector::ENCOUNTER_COUNT,
                )
                .unwrap_or(u16::MIN),
                location: read_optional_object_reference(
                    profile,
                    source,
                    ScriptFieldSelector::HOLDER_OR_LOCATION,
                )?,
                name,
            })
        })
        .collect()
}

fn object_kind(profile: &LoadedScriptProfile, object: ScriptObjectId) -> Result<ScriptObjectKind> {
    profile
        .state()
        .object(object)
        .map(|record| record.kind)
        .with_context(|| format!("navigation object {object:?} is absent"))
}

const fn chart_kind(kind: ScriptObjectKind) -> NavigationChartObjectKind {
    NavigationChartObjectKind {
        ship: matches!(kind, ScriptObjectKind::NavigationEntity),
        black_hole: matches!(kind, ScriptObjectKind::BlackHole),
    }
}

const fn chart_kind_supported(kind: ScriptObjectKind) -> bool {
    matches!(
        kind,
        ScriptObjectKind::CelestialBody
            | ScriptObjectKind::NavigationEntity
            | ScriptObjectKind::BlackHole
    )
}

const fn status_kind(kind: ScriptObjectKind) -> NavigationStatusLocationKind {
    match kind {
        ScriptObjectKind::NavigationEntity => NavigationStatusLocationKind::Ship,
        ScriptObjectKind::BlackHole => NavigationStatusLocationKind::BlackHole,
        _ => NavigationStatusLocationKind::Planet,
    }
}

fn object_access_counter(state: &ScriptState, object: ScriptObjectId) -> Result<u16> {
    let bytes = state
        .object(object)
        .with_context(|| format!("navigation object {object:?} is absent"))?
        .bytes();
    let value = bytes
        .get(OBJECT_ACCESS_COUNTER_BYTE_OFFSET..OBJECT_ACCESS_COUNTER_BYTE_OFFSET + WORD_BYTE_COUNT)
        .with_context(|| format!("navigation object {object:?} has no access counter"))?;
    Ok(u16::from_le_bytes(
        value.try_into().expect("word slice has fixed length"),
    ))
}

fn read_object_word(
    profile: &LoadedScriptProfile,
    object: ScriptObjectId,
    selector: ScriptFieldSelector,
) -> Result<u16> {
    read_optional_object_word(profile, object, selector).with_context(|| {
        format!(
            "navigation object {object:?} has no field for selector {}",
            selector.index()
        )
    })
}

fn read_optional_object_word(
    profile: &LoadedScriptProfile,
    object: ScriptObjectId,
    selector: ScriptFieldSelector,
) -> Option<u16> {
    let record = profile.state().object(object)?;
    let offset = script_field_offset(record.kind, selector)?;
    let field = profile
        .state()
        .object_word(object, offset / WORD_BYTE_COUNT)?;
    profile.state().word(field)
}

fn read_optional_object_reference(
    profile: &LoadedScriptProfile,
    object: ScriptObjectId,
    selector: ScriptFieldSelector,
) -> Result<Option<ScriptObjectId>> {
    let record = profile
        .state()
        .object(object)
        .with_context(|| format!("navigation source {object:?} is absent"))?;
    let offset = script_field_offset(record.kind, selector).with_context(|| {
        format!(
            "navigation source {object:?} has no field for selector {}",
            selector.index()
        )
    })?;
    let field = profile
        .state()
        .object_word(object, offset / WORD_BYTE_COUNT)
        .with_context(|| format!("navigation source {object:?} relation is unreadable"))?;
    match profile.state().object_reference(field) {
        Some(ScriptStateObjectReference::Object(location)) => Ok(Some(location)),
        Some(ScriptStateObjectReference::Sentinel) => Ok(None),
        None => bail!("navigation source {object:?} has an invalid location relation"),
    }
}

fn read_position(state: &ScriptState, field: ScriptStateWordPair) -> Result<[u16; 2]> {
    state
        .word_pair(field)
        .context("navigation position pair is unreadable")
}

const fn hand_selector(hand: NavigationChartHand) -> u16 {
    match hand {
        NavigationChartHand::Neutral => NEUTRAL_HAND_SELECTOR,
        NavigationChartHand::Left => LEFT_HAND_SELECTOR,
        NavigationChartHand::Right => RIGHT_HAND_SELECTOR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::bloodprg::{
        ORIGINAL_SCRIPT_PROFILE_COUNT, ScriptProfileId, set_object_flag,
    };
    use crate::runtime::{OriginalGameData, OriginalGameDataPaths};

    #[test]
    fn panel_interpolation_matches_recovered_divide_before_multiply_order() {
        let mut progress = LocationPanelTransitionProgress {
            current: 0,
            total: 8,
        };
        let rects = LocationPanelRects {
            target: LOCATION_PANEL_TARGET_RECT,
            current: LocationPanelRect {
                x: 200,
                y: 80,
                width: 4,
                height: 4,
            },
        };

        let first =
            interpolate_panel_rect(LocationPanelInterpolation::Opening, rects, &mut progress)
                .unwrap()
                .unwrap();
        assert_eq!(
            first,
            LocationPanelRect {
                x: 189,
                y: 74,
                width: 15,
                height: 12
            }
        );
        for _ in 1..8 {
            interpolate_panel_rect(LocationPanelInterpolation::Opening, rects, &mut progress)
                .unwrap()
                .unwrap();
        }
        assert_eq!(progress.current, progress.total);
        assert_eq!(
            interpolate_panel_rect(LocationPanelInterpolation::Opening, rects, &mut progress)
                .unwrap(),
            None
        );
    }

    #[test]
    #[ignore = "requires original Commander Blood data"]
    fn every_shipped_profile_decodes_a_typed_navigation_world() {
        let paths = OriginalGameDataPaths::discover(None).unwrap();
        let data = OriginalGameData::load(paths).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        for profile in 0..ORIGINAL_SCRIPT_PROFILE_COUNT {
            runtime
                .load_profile(ScriptProfileId::new(profile as u8).unwrap())
                .unwrap();
            let chart_candidates = runtime
                .current_profile()
                .unwrap()
                .state()
                .objects()
                .iter()
                .filter(|object| {
                    matches!(
                        object.kind,
                        ScriptObjectKind::CelestialBody
                            | ScriptObjectKind::NavigationEntity
                            | ScriptObjectKind::BlackHole
                    )
                })
                .map(|object| object.id)
                .collect::<Vec<_>>();
            for object in &chart_candidates {
                assert!(set_object_flag(
                    runtime.current_profile_mut().unwrap().state_mut(),
                    *object,
                    ScriptObjectFlag::InPlay,
                    true,
                ));
            }
            let world = RuntimeNavigationWorld::decode(&runtime).unwrap();
            assert_eq!(
                world.chart_objects.len(),
                chart_candidates.len(),
                "profile {profile}"
            );
            assert_eq!(world.chart_objects.len(), world.pick_objects.len());
            assert_eq!(world.chart_objects.len(), world.locations.len());
            assert_eq!(world.artwork.len(), 42, "profile {profile}");
            assert!(
                runtime
                    .current_profile()
                    .unwrap()
                    .state()
                    .object(world.current_location)
                    .is_some(),
                "profile {profile}"
            );
        }
    }
}
