//! Live bridge panorama and procedural starfield over owned, flat runtime state.

use std::error::Error;
use std::fmt;

use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};
use commander_blood_formats::panorama::{
    BridgePanoramaArchive, BridgePanoramaError, BridgePanoramaFrameMetadata,
    PANORAMA_FRAME_PIXEL_COUNT, PanoramaDecodeMode, SHIPPED_PANORAMA_FRAME_COUNT,
};

use crate::native::random::BloodPrng;

use super::{
    BRIDGE_ARC_UNITS_PER_VIEW_FRAME, BRIDGE_CURSOR_RING_UNIT_COUNT,
    BRIDGE_CURSOR_UNITS_PER_VIEW_FRAME, BRIDGE_LOGICAL_SCREEN_CENTER_X, BridgePanoramaLoadTarget,
    BridgeSpriteEntity, BridgeStationOrbBoxes, BridgeSteeringInteraction, BridgeSteeringOutcome,
    BridgeSteeringState, FULL_SHIP_PROJECTION_CLIP, IndexedGamePalette, NavActorSeekState,
    SHIP_CAMERA_RESET, SHIP_POINT_CLOUD_COUNT, SHIP_TRIGONOMETRY_SAMPLE_COUNT, ShipCameraPosition,
    ShipObjectSpriteProjection, ShipPointCloudProjection, ShipPointRecord, ShipProjectionAngles,
    ShipProjectionError, ShipProjectionMatrix, ShipProjectionResources,
    build_ship_projection_matrix, load_bridge_panorama_frame,
    project_ship_object_sprites_against_source_extent, project_ship_point_cloud,
    randomize_ship_point_cloud, update_bridge_steering,
};

/// Authored golden-console rest frame used when entering the bridge hub.
pub const INITIAL_BRIDGE_VIEW_FRAME: u16 = 45;

const INITIAL_PRESENTATION_LINK: u16 = INITIAL_BRIDGE_VIEW_FRAME * BRIDGE_ARC_UNITS_PER_VIEW_FRAME;

/// Host input consumed by one bridge scene frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BridgeSceneInput {
    /// Relative horizontal SDL motion accumulated since the preceding frame.
    pub horizontal_delta: i32,
    /// Current semantic pointer-button bits.
    pub pointer_buttons: u16,
    /// Whether a bridge menu currently constrains free camera steering.
    pub interaction: BridgeSteeringInteraction,
}

/// One complete bridge frame ready for modern GPU presentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeSceneFrame {
    /// Current authored panorama frame number.
    pub panorama_frame: usize,
    /// Typed station and eye-orb metadata decoded with the panorama pixels.
    pub metadata: BridgePanoramaFrameMetadata,
    /// Current station hit boxes published into the first four actor slots.
    pub station_orb_boxes: BridgeStationOrbBoxes,
    /// Opaque indexed panorama layer; index zero becomes transparent on the GPU.
    pub panorama_pixels: Box<[u8]>,
    /// Exact fixed-point starfield projection generated before panorama compositing.
    pub starfield: ShipPointCloudProjection,
    /// Perspective projections applied to visible navigation sprite entities.
    pub object_sprites: Box<[ShipObjectSpriteProjection]>,
    /// Fresh transparent indexed layer rasterized from projected ship objects.
    pub object_sprite_pixels: Box<[u8]>,
    /// Observable steering result for bridge interaction and presentation routing.
    pub steering: BridgeSteeringOutcome,
}

/// Invalid bridge resources or runtime projection input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeSceneError {
    /// `TB.BIG` does not contain the complete authored panorama ring.
    InvalidPanoramaFrameCount {
        /// Number of decoded archive entries.
        actual: usize,
        /// Number of entries consumed by bridge steering.
        expected: usize,
    },
    /// A decoded panorama frame was malformed.
    Panorama(BridgePanoramaError),
    /// Typed projection state could not produce a frame.
    Projection(ShipProjectionError),
    /// Camera projection stages were called out of recovered order.
    ProjectionStageUnavailable(&'static str),
}

impl fmt::Display for BridgeSceneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid bridge scene: {self:?}")
    }
}

impl Error for BridgeSceneError {}

impl From<BridgePanoramaError> for BridgeSceneError {
    fn from(error: BridgePanoramaError) -> Self {
        Self::Panorama(error)
    }
}

impl From<ShipProjectionError> for BridgeSceneError {
    fn from(error: ShipProjectionError) -> Self {
        Self::Projection(error)
    }
}

/// Persistent modern bridge scene state.
#[derive(Clone, Debug)]
pub struct BridgeScene {
    panorama: BridgePanoramaArchive,
    projection_resources: ShipProjectionResources,
    point_cloud: Box<[ShipPointRecord]>,
    camera: ShipCameraPosition,
    camera_yaw: u16,
    prepared_projection: Option<PreparedBridgeProjection>,
    steering: BridgeSteeringState,
    seek: NavActorSeekState,
    presentation_link: u16,
    last_steering: BridgeSteeringOutcome,
}

#[derive(Clone, Debug)]
struct PreparedBridgeProjection {
    navigation_heading: u16,
    matrix: ShipProjectionMatrix,
    starfield: Option<ShipPointCloudProjection>,
    object_sprites: Option<Box<[ShipObjectSpriteProjection]>>,
}

impl BridgeScene {
    /// Build the bridge from decoded resources and randomize its point cloud once.
    ///
    /// The caller owns the shared game PRNG. This constructor consumes the exact
    /// 3,000 startup draws while retaining only ordinary point records afterward.
    pub fn new(
        panorama: BridgePanoramaArchive,
        projection_resources: ShipProjectionResources,
        random: &mut BloodPrng,
    ) -> Result<Self, BridgeSceneError> {
        if panorama.frame_count() != SHIPPED_PANORAMA_FRAME_COUNT {
            return Err(BridgeSceneError::InvalidPanoramaFrameCount {
                actual: panorama.frame_count(),
                expected: SHIPPED_PANORAMA_FRAME_COUNT,
            });
        }

        let mut point_cloud =
            vec![ShipPointRecord::default(); SHIP_POINT_CLOUD_COUNT].into_boxed_slice();
        randomize_ship_point_cloud(&mut point_cloud, random)?;
        let frame_angle_bias = INITIAL_BRIDGE_VIEW_FRAME
            .wrapping_mul(BRIDGE_CURSOR_UNITS_PER_VIEW_FRAME)
            .wrapping_sub(BRIDGE_LOGICAL_SCREEN_CENTER_X);
        let steering = BridgeSteeringState {
            view_frame: INITIAL_BRIDGE_VIEW_FRAME,
            cursor_ring_position: BRIDGE_LOGICAL_SCREEN_CENTER_X,
            cursor_arc: INITIAL_PRESENTATION_LINK,
            cursor_drag_reference: BRIDGE_LOGICAL_SCREEN_CENTER_X,
            pointer_buttons: u16::MIN,
            seek_initial_distance: u16::MIN,
            turn_direction: None,
            projection_heading: INITIAL_BRIDGE_VIEW_FRAME,
            frame_angle_bias,
        };

        Ok(Self {
            panorama,
            projection_resources,
            point_cloud,
            camera: ShipCameraPosition {
                position: SHIP_CAMERA_RESET.map(|component| component as u16),
            },
            camera_yaw: u16::MIN,
            prepared_projection: None,
            steering,
            seek: NavActorSeekState::default(),
            presentation_link: INITIAL_PRESENTATION_LINK,
            last_steering: BridgeSteeringOutcome {
                view_changed: false,
                presentation_link: INITIAL_PRESENTATION_LINK,
            },
        })
    }

    /// Current flat steering state, exposed for higher-level bridge interaction.
    pub fn steering(&self) -> BridgeSteeringState {
        self.steering
    }

    /// Request automatic steering toward one native bridge arc.
    pub fn request_seek(&mut self, target_arc: u16) {
        self.seek.target_arc = target_arc;
        self.seek.requested = true;
    }

    /// Report whether an automatic bridge seek remains pending.
    pub const fn seek_requested(&self) -> bool {
        self.seek.requested
    }

    /// Restore the authored camera origin used when rebuilding the ship HUD.
    pub fn reset_camera(&mut self) {
        self.set_camera_approach_pose(SHIP_CAMERA_RESET, u16::MIN);
    }

    /// Apply the flat signed camera coordinates and yaw owned by camera travel.
    pub fn set_camera_approach_pose(&mut self, camera: [i16; 3], camera_yaw: u16) {
        let camera = ShipCameraPosition {
            position: camera.map(|component| component as u16),
        };
        if self.camera != camera || self.camera_yaw != camera_yaw {
            self.camera = camera;
            self.camera_yaw = camera_yaw;
            self.prepared_projection = None;
        }
    }

    /// Build and retain the projection matrix requested by camera travel.
    pub fn build_camera_projection_matrix(&mut self) -> Result<(), BridgeSceneError> {
        let matrix = build_ship_projection_matrix(
            &self.projection_resources.trigonometry,
            ShipProjectionAngles {
                camera_yaw: camera_yaw_table_index(self.camera_yaw),
                navigation_heading: self.steering.projection_heading,
                camera_roll: u16::MIN,
            },
        )?;
        self.prepared_projection = Some(PreparedBridgeProjection {
            navigation_heading: self.steering.projection_heading,
            matrix,
            starfield: None,
            object_sprites: None,
        });
        Ok(())
    }

    /// Project the persistent starfield through the retained camera matrix.
    pub fn project_camera_point_cloud(&mut self) -> Result<(), BridgeSceneError> {
        let prepared = self.prepared_projection.as_mut().ok_or(
            BridgeSceneError::ProjectionStageUnavailable("camera matrix"),
        )?;
        let mut occupancy = vec![u8::MIN; PANORAMA_FRAME_PIXEL_COUNT];
        prepared.starfield = Some(project_ship_point_cloud(
            &self.point_cloud,
            self.camera,
            prepared.matrix,
            FULL_SHIP_PROJECTION_CLIP,
            &mut occupancy,
        )?);
        Ok(())
    }

    /// Project ship object entities through the retained camera matrix.
    pub fn project_camera_object_sprites(
        &mut self,
        sprite_entities: &mut [BridgeSpriteEntity],
    ) -> Result<(), BridgeSceneError> {
        let prepared = self.prepared_projection.as_mut().ok_or(
            BridgeSceneError::ProjectionStageUnavailable("camera matrix"),
        )?;
        prepared.object_sprites = Some(project_ship_object_sprites_against_source_extent(
            &self.projection_resources.object_anchors,
            self.camera,
            prepared.matrix,
            sprite_entities,
        )?);
        Ok(())
    }

    /// Apply the bridge globals written when the recovered ship HUD opens.
    pub fn initialize_hud_view(&mut self, seek_target_arc: u16, view_frame: u16) {
        self.seek.target_arc = seek_target_arc;
        self.steering.view_frame = view_frame;
    }

    /// Advance only the recovered bridge steering state.
    pub fn update_steering(&mut self, input: BridgeSceneInput) -> BridgeSteeringOutcome {
        self.steering.cursor_ring_position = native_ring_input(
            self.steering.cursor_ring_position,
            self.steering.frame_angle_bias,
            input.horizontal_delta,
        );
        self.steering.pointer_buttons = input.pointer_buttons;
        let steering = update_bridge_steering(
            &mut self.steering,
            &mut self.seek,
            input.interaction,
            self.presentation_link,
        );
        self.presentation_link = steering.presentation_link;
        self.last_steering = steering;
        steering
    }

    /// Generate layers for the current steering state without consuming input.
    pub fn render_current_frame(
        &mut self,
        sprite_entities: &mut [BridgeSpriteEntity],
    ) -> Result<BridgeSceneFrame, BridgeSceneError> {
        let panorama_palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        let mut live_palette = panorama_palette;
        self.render_current_frame_with_palette(
            sprite_entities,
            false,
            &panorama_palette,
            &mut live_palette,
        )
    }

    /// Generate the current layers through the recovered panorama loader.
    pub fn render_current_frame_with_palette(
        &mut self,
        sprite_entities: &mut [BridgeSpriteEntity],
        refresh_live_palette: bool,
        panorama_palette: &IndexedGamePalette,
        live_palette: &mut IndexedGamePalette,
    ) -> Result<BridgeSceneFrame, BridgeSceneError> {
        let steering = self.last_steering;

        let prepared = self
            .prepared_projection
            .take()
            .filter(|prepared| prepared.navigation_heading == self.steering.projection_heading);
        let matrix = if let Some(prepared) = prepared.as_ref() {
            prepared.matrix
        } else {
            build_ship_projection_matrix(
                &self.projection_resources.trigonometry,
                ShipProjectionAngles {
                    camera_yaw: camera_yaw_table_index(self.camera_yaw),
                    navigation_heading: self.steering.projection_heading,
                    camera_roll: u16::MIN,
                },
            )?
        };
        let starfield = if let Some(starfield) = prepared
            .as_ref()
            .and_then(|prepared| prepared.starfield.as_ref())
        {
            starfield.clone()
        } else {
            let mut star_occupancy = vec![u8::MIN; PANORAMA_FRAME_PIXEL_COUNT];
            project_ship_point_cloud(
                &self.point_cloud,
                self.camera,
                matrix,
                FULL_SHIP_PROJECTION_CLIP,
                &mut star_occupancy,
            )?
        };
        let object_sprites =
            if let Some(object_sprites) = prepared.and_then(|prepared| prepared.object_sprites) {
                object_sprites
            } else {
                project_ship_object_sprites_against_source_extent(
                    &self.projection_resources.object_anchors,
                    self.camera,
                    matrix,
                    sprite_entities,
                )?
            };
        let panorama_frame = usize::from(self.steering.view_frame);
        let mut panorama_pixels = vec![u8::MIN; PANORAMA_FRAME_PIXEL_COUNT].into_boxed_slice();
        let mut station_orb_boxes = BridgeStationOrbBoxes::default();
        let metadata = load_bridge_panorama_frame(
            &self.panorama,
            panorama_frame,
            PanoramaDecodeMode::Opaque,
            BridgePanoramaLoadTarget::new(
                &mut panorama_pixels,
                &mut station_orb_boxes,
                refresh_live_palette,
                panorama_palette,
                live_palette,
            ),
        )?;

        Ok(BridgeSceneFrame {
            panorama_frame,
            metadata,
            station_orb_boxes,
            panorama_pixels,
            starfield,
            object_sprites,
            object_sprite_pixels: vec![u8::MIN; PANORAMA_FRAME_PIXEL_COUNT].into_boxed_slice(),
            steering,
        })
    }

    /// Advance steering and generate the exact starfield and panorama layers.
    pub fn render_frame(
        &mut self,
        input: BridgeSceneInput,
        sprite_entities: &mut [BridgeSpriteEntity],
    ) -> Result<BridgeSceneFrame, BridgeSceneError> {
        self.update_steering(input);
        self.render_current_frame(sprite_entities)
    }
}

fn native_ring_input(screen_x: u16, frame_angle_bias: u16, horizontal_delta: i32) -> u16 {
    screen_x
        .wrapping_add(frame_angle_bias)
        .wrapping_add(BRIDGE_CURSOR_RING_UNIT_COUNT)
        .wrapping_add(horizontal_delta as u16)
}

fn camera_yaw_table_index(camera_yaw: u16) -> u16 {
    // The executable stores a duplicate angle-zero sample immediately after
    // the 180 logical entries. Camera travel deliberately publishes 180 as its
    // wrap sentinel, so map only that value to the equivalent owned sample.
    if camera_yaw == SHIP_TRIGONOMETRY_SAMPLE_COUNT as u16 {
        u16::MIN
    } else {
        camera_yaw
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::bloodprg::decode_bloodprg_bridge_resources;
    use commander_blood_formats::panorama::BridgeStation;

    use super::super::{BRIDGE_SPRITE_ENTITY_COUNT, BridgeSpriteExtent, BridgeSpriteFlags};
    use super::*;

    const TEST_CLOCK_BYTE: u8 = 17;
    const LARGE_POINTER_DELTA: i32 = 160;
    const NO_POINTER_DELTA: i32 = 0;
    const FIRST_NAVIGATION_ENTITY_INDEX: usize = 21;
    const ACTIVE_VISIBLE_SPRITE_FLAGS: u16 = 129;
    const TEST_SPRITE_WIDTH: u16 = 40;
    const TEST_SPRITE_HEIGHT: u16 = 24;

    #[test]
    fn ring_adapter_reconstructs_the_native_absolute_input_without_cursor_warping() {
        let frame_angle_bias = INITIAL_BRIDGE_VIEW_FRAME
            .wrapping_mul(BRIDGE_CURSOR_UNITS_PER_VIEW_FRAME)
            .wrapping_sub(BRIDGE_LOGICAL_SCREEN_CENTER_X);
        assert_eq!(
            native_ring_input(
                BRIDGE_LOGICAL_SCREEN_CENTER_X,
                frame_angle_bias,
                NO_POINTER_DELTA
            ),
            INITIAL_BRIDGE_VIEW_FRAME * BRIDGE_CURSOR_UNITS_PER_VIEW_FRAME
                + BRIDGE_CURSOR_RING_UNIT_COUNT
        );
        assert_eq!(
            native_ring_input(
                BRIDGE_LOGICAL_SCREEN_CENTER_X,
                frame_angle_bias,
                LARGE_POINTER_DELTA
            ),
            INITIAL_BRIDGE_VIEW_FRAME * BRIDGE_CURSOR_UNITS_PER_VIEW_FRAME
                + BRIDGE_CURSOR_RING_UNIT_COUNT
                + LARGE_POINTER_DELTA as u16
        );
        assert_eq!(
            camera_yaw_table_index(SHIP_TRIGONOMETRY_SAMPLE_COUNT as u16),
            u16::MIN
        );
        assert_eq!(
            camera_yaw_table_index(SHIP_TRIGONOMETRY_SAMPLE_COUNT as u16 + 1),
            SHIP_TRIGONOMETRY_SAMPLE_COUNT as u16 + 1
        );
    }

    #[test]
    fn original_resources_run_through_the_flat_live_bridge_pipeline() {
        let Some(executable_path) = original_file("BLOODPRG.EXE") else {
            return;
        };
        let Some(panorama_path) = original_file("TB.BIG") else {
            return;
        };
        let executable = std::fs::read(executable_path).unwrap();
        let resources = decode_bloodprg_bridge_resources(&executable).unwrap();
        let panorama =
            BridgePanoramaArchive::decode(std::fs::read(panorama_path).unwrap().into_boxed_slice())
                .unwrap();
        let mut random = BloodPrng::default();
        random.seed_from_clock_register(TEST_CLOCK_BYTE);
        let mut scene = BridgeScene::new(panorama, resources.into(), &mut random).unwrap();
        let mut sprite_entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
        sprite_entities[FIRST_NAVIGATION_ENTITY_INDEX] = BridgeSpriteEntity {
            flags: BridgeSpriteFlags::from_bits(ACTIVE_VISIBLE_SPRITE_FLAGS),
            source_extent: BridgeSpriteExtent {
                width: TEST_SPRITE_WIDTH,
                height: TEST_SPRITE_HEIGHT,
            },
            ..BridgeSpriteEntity::default()
        };

        assert_eq!(
            scene.project_camera_point_cloud(),
            Err(BridgeSceneError::ProjectionStageUnavailable(
                "camera matrix"
            ))
        );

        let centered = scene
            .render_frame(BridgeSceneInput::default(), &mut sprite_entities)
            .unwrap();
        assert_eq!(
            centered.panorama_frame,
            usize::from(INITIAL_BRIDGE_VIEW_FRAME)
        );
        assert_eq!(centered.metadata.station, BridgeStation::Console);
        assert_eq!(
            centered.station_orb_boxes[centered.metadata.station.index()],
            centered.metadata.orb_box
        );
        assert_eq!(centered.station_orb_boxes.iter().flatten().count(), 1);
        assert_eq!(centered.panorama_pixels.len(), PANORAMA_FRAME_PIXEL_COUNT);
        assert_eq!(
            centered.object_sprite_pixels.len(),
            PANORAMA_FRAME_PIXEL_COUNT
        );
        assert!(!centered.starfield.plotted.is_empty());
        assert_eq!(centered.object_sprites.len(), 1);
        assert!(!centered.steering.view_changed);
        scene.set_camera_approach_pose([9_500, 12_000, 1_500], 23);
        scene.build_camera_projection_matrix().unwrap();
        scene.project_camera_point_cloud().unwrap();
        scene
            .project_camera_object_sprites(&mut sprite_entities)
            .unwrap();
        let camera_frame = scene
            .render_frame(
                BridgeSceneInput {
                    interaction: BridgeSteeringInteraction::MenuEngaged,
                    ..BridgeSceneInput::default()
                },
                &mut sprite_entities,
            )
            .unwrap();
        assert!(!camera_frame.starfield.plotted.is_empty());
        assert_eq!(camera_frame.object_sprites.len(), 1);
        assert_eq!(
            scene.steering().cursor_ring_position,
            BRIDGE_LOGICAL_SCREEN_CENTER_X
        );

        let steered = scene
            .render_frame(
                BridgeSceneInput {
                    horizontal_delta: LARGE_POINTER_DELTA,
                    ..BridgeSceneInput::default()
                },
                &mut sprite_entities,
            )
            .unwrap();
        assert!(steered.steering.view_changed);
        assert_ne!(steered.panorama_frame, centered.panorama_frame);
    }

    fn original_file(filename: &str) -> Option<PathBuf> {
        [
            Path::new("output/_tmp_iso").join(filename),
            Path::new("../../output/_tmp_iso").join(filename),
            Path::new("commander-blood-audio/_tmp_iso").join(filename),
            Path::new("../../commander-blood-audio/_tmp_iso").join(filename),
        ]
        .into_iter()
        .find(|path| path.is_file())
    }
}
