//! Live bridge panorama and procedural starfield over owned, flat runtime state.

use std::error::Error;
use std::fmt;

use commander_blood_formats::panorama::{
    BridgePanoramaArchive, BridgePanoramaError, BridgePanoramaFrameMetadata,
    PANORAMA_FRAME_PIXEL_COUNT, PanoramaDecodeMode, SHIPPED_PANORAMA_FRAME_COUNT,
};

use crate::native::random::BloodPrng;

use super::{
    BRIDGE_ARC_UNITS_PER_VIEW_FRAME, BRIDGE_CURSOR_RING_UNIT_COUNT,
    BRIDGE_CURSOR_UNITS_PER_VIEW_FRAME, BRIDGE_LOGICAL_SCREEN_CENTER_X, BridgeSteeringInteraction,
    BridgeSteeringOutcome, BridgeSteeringState, FULL_SHIP_PROJECTION_CLIP, NavActorSeekState,
    SHIP_CAMERA_RESET, SHIP_POINT_CLOUD_COUNT, ShipCameraPosition, ShipPointCloudProjection,
    ShipPointRecord, ShipProjectionAngles, ShipProjectionError, ShipProjectionResources,
    build_ship_projection_matrix, project_ship_point_cloud, randomize_ship_point_cloud,
    update_bridge_steering,
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
    /// Opaque indexed panorama layer; index zero becomes transparent on the GPU.
    pub panorama_pixels: Box<[u8]>,
    /// Exact fixed-point starfield projection generated before panorama compositing.
    pub starfield: ShipPointCloudProjection,
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
    steering: BridgeSteeringState,
    seek: NavActorSeekState,
    presentation_link: u16,
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
            steering,
            seek: NavActorSeekState::default(),
            presentation_link: INITIAL_PRESENTATION_LINK,
        })
    }

    /// Current flat steering state, exposed for higher-level bridge interaction.
    pub fn steering(&self) -> BridgeSteeringState {
        self.steering
    }

    /// Advance steering and generate the exact starfield and panorama layers.
    pub fn render_frame(
        &mut self,
        input: BridgeSceneInput,
    ) -> Result<BridgeSceneFrame, BridgeSceneError> {
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

        let matrix = build_ship_projection_matrix(
            &self.projection_resources.trigonometry,
            ShipProjectionAngles {
                camera_yaw: u16::MIN,
                navigation_heading: self.steering.projection_heading,
                camera_roll: u16::MIN,
            },
        )?;
        let mut star_occupancy = vec![u8::MIN; PANORAMA_FRAME_PIXEL_COUNT];
        let starfield = project_ship_point_cloud(
            &self.point_cloud,
            self.camera,
            matrix,
            FULL_SHIP_PROJECTION_CLIP,
            &mut star_occupancy,
        )?;
        let panorama_frame = usize::from(self.steering.view_frame);
        let mut panorama_pixels = vec![u8::MIN; PANORAMA_FRAME_PIXEL_COUNT].into_boxed_slice();
        let metadata = self.panorama.decode_frame_over(
            panorama_frame,
            &mut panorama_pixels,
            PanoramaDecodeMode::Opaque,
        )?;

        Ok(BridgeSceneFrame {
            panorama_frame,
            metadata,
            panorama_pixels,
            starfield,
            steering,
        })
    }
}

fn native_ring_input(screen_x: u16, frame_angle_bias: u16, horizontal_delta: i32) -> u16 {
    screen_x
        .wrapping_add(frame_angle_bias)
        .wrapping_add(BRIDGE_CURSOR_RING_UNIT_COUNT)
        .wrapping_add(horizontal_delta as u16)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::bloodprg::decode_bloodprg_bridge_resources;
    use commander_blood_formats::panorama::BridgeStation;

    use super::*;

    const TEST_CLOCK_BYTE: u8 = 17;
    const LARGE_POINTER_DELTA: i32 = 160;
    const NO_POINTER_DELTA: i32 = 0;

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

        let centered = scene.render_frame(BridgeSceneInput::default()).unwrap();
        assert_eq!(
            centered.panorama_frame,
            usize::from(INITIAL_BRIDGE_VIEW_FRAME)
        );
        assert_eq!(centered.metadata.station, BridgeStation::Console);
        assert_eq!(centered.panorama_pixels.len(), PANORAMA_FRAME_PIXEL_COUNT);
        assert!(!centered.starfield.plotted.is_empty());
        assert!(!centered.steering.view_changed);
        assert_eq!(
            scene.steering().cursor_ring_position,
            BRIDGE_LOGICAL_SCREEN_CENTER_X
        );

        let steered = scene
            .render_frame(BridgeSceneInput {
                horizontal_delta: LARGE_POINTER_DELTA,
                ..BridgeSceneInput::default()
            })
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
