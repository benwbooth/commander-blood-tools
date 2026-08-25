//! Typed frame coordinator for the shared alien-scene geometry pipeline.

use std::fmt;

use commander_blood_formats::alien::{
    AXIS_COUNT, AlienAsset, AlienBehaviorMethod, AlienTransformData, AlienWaveSelectionData,
    AlienXdbKind,
};

use super::{
    AlienBehaviorError, AlienBehindCameraSignal, AlienCallbackSceneState, AlienCameraAngles,
    AlienCameraControl, AlienCameraStep, AlienCameraTransform, AlienFaceSelection,
    AlienFaceSelectionError, AlienModelPose, AlienMouseSample, AlienPrimaryMeshFrame,
    AlienPrimaryMeshPose, AlienPrimaryProjectionError, AlienProjectionError, AlienRasterError,
    AlienRenderGeometry, AlienSceneNode, AlienScreenCenter, AlienSpecies, AlienStarfieldError,
    AlienStarfieldFrame, AlienWaveError, AlienWaveMethodState, AlienWaveSelection, adjust_state,
    anchor_state, bounds_then_wrap, generate_starfield, prepare_render_geometry, select_faces,
    update_or_initialize_wave, wrap_positions,
};

const INITIAL_VIEW: [i16; AXIS_COUNT] = [1_885, -239, -9_790];
const INITIAL_PITCH: i16 = 0;
const INITIAL_PAN: i16 = 1_656;
const INITIAL_SECONDARY_PAN: i16 = 0;
const INITIAL_DEPTH_VELOCITY: i16 = 0;
const ACTIVE_INTERACTION_SIGNAL: u16 = 1;
const ORIGINAL_SCREEN_CENTER: AlienScreenCenter = AlienScreenCenter { x: 160, y: 100 };

/// Render-facing native output produced in recovered main-loop order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienSceneFrame {
    /// Input and camera-control result for this frame.
    pub camera_step: AlienCameraStep,
    /// Camera-relative primary-mesh projection and face buckets.
    pub primary: AlienPrimaryMeshFrame,
    /// Visible fixed-point starfield points.
    pub starfield: AlienStarfieldFrame,
    /// Hierarchical model face decisions and buckets.
    pub models: AlienFaceSelection,
    /// Owned textured triangles for the primary and behavior-model passes.
    pub geometry: AlienRenderGeometry,
}

/// Mutable native state for one AMER, CROOLIS, or SCRUT scene.
#[derive(Clone, Debug)]
pub struct AlienScene {
    asset: AlienAsset,
    species: AlienSpecies,
    /// Mouse, keyboard, and camera accumulators.
    pub control: AlienCameraControl,
    /// Eased camera matrix and fixed-point position.
    pub camera: AlienCameraTransform,
    /// Primary camera-relative model state.
    pub primary: AlienPrimaryMeshPose,
    /// Behavior-model poses in authored dispatch order.
    pub models: Vec<AlienModelPose>,
    /// Per-model continuation state for authored wave methods.
    wave_states: Vec<Option<AlienWaveMethodState>>,
    /// Model selected by the latest CROOLIS/SCRUT camera-plane signal.
    pub selected_model: Option<usize>,
    /// Shared state published and consumed by translated behavior callbacks.
    pub callback_state: AlienCallbackSceneState,
    /// Original scene-exit word published by the bounds behavior.
    exit_requested: u16,
}

/// Failure in one typed alien-scene frame stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienSceneError {
    /// Primary mesh projection failed.
    Primary(AlienPrimaryProjectionError),
    /// One hierarchy projection failed.
    ModelProjection {
        /// Model that failed.
        model_index: usize,
        /// Underlying projection failure.
        error: AlienProjectionError,
    },
    /// Model face selection failed.
    FaceSelection(AlienFaceSelectionError),
    /// Starfield generation failed.
    Starfield(AlienStarfieldError),
    /// Textured triangle preparation failed.
    Raster(AlienRasterError),
    /// One model's direct behavior method rejected its typed state.
    Behavior {
        /// Model that failed.
        model_index: usize,
        /// Underlying behavior failure.
        error: AlienBehaviorError,
    },
    /// A wave model has no decoded continuation state.
    MissingWaveState {
        /// Model missing its state.
        model_index: usize,
    },
    /// One wave method rejected its typed state.
    Wave {
        /// Model that failed.
        model_index: usize,
        /// Underlying wave failure.
        error: AlienWaveError,
    },
}

impl fmt::Display for AlienSceneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "alien scene frame failed: {self:?}")
    }
}

impl std::error::Error for AlienSceneError {}

impl From<AlienPrimaryProjectionError> for AlienSceneError {
    fn from(error: AlienPrimaryProjectionError) -> Self {
        Self::Primary(error)
    }
}

impl From<AlienFaceSelectionError> for AlienSceneError {
    fn from(error: AlienFaceSelectionError) -> Self {
        Self::FaceSelection(error)
    }
}

impl From<AlienStarfieldError> for AlienSceneError {
    fn from(error: AlienStarfieldError) -> Self {
        Self::Starfield(error)
    }
}

impl From<AlienRasterError> for AlienSceneError {
    fn from(error: AlienRasterError) -> Self {
        Self::Raster(error)
    }
}

impl AlienScene {
    /// Construct scene state from one fully decoded overlay asset.
    pub fn from_asset(asset: AlienAsset) -> Self {
        let species = match asset.kind {
            AlienXdbKind::Amer => AlienSpecies::Amer,
            AlienXdbKind::Croolis => AlienSpecies::Croolis,
            AlienXdbKind::Scrut => AlienSpecies::Scrut,
        };
        let mut position = asset.camera.position;
        for axis in usize::MIN..AXIS_COUNT {
            position[axis] = ((position[axis] as u32 & u32::from(u16::MAX))
                | (u32::from(INITIAL_VIEW[axis] as u16) << u16::BITS))
                as i32;
        }
        let camera = AlienCameraTransform {
            matrix: asset.camera.matrix,
            position,
            view: INITIAL_VIEW,
            transformed_view: asset.camera.transformed_view,
            ..AlienCameraTransform::default()
        };
        let control = AlienCameraControl {
            horizontal_filter: asset.camera.horizontal_filter,
            pitch: INITIAL_PITCH,
            pan: INITIAL_PAN,
            secondary_pan: INITIAL_SECONDARY_PAN,
            depth_velocity: INITIAL_DEPTH_VELOCITY,
            ..AlienCameraControl::default()
        };
        let primary = AlienPrimaryMeshPose::from_model(&asset.primary_model);
        let models = asset
            .models
            .iter()
            .map(AlienModelPose::from_model)
            .collect();
        let wave_states = asset
            .models
            .iter()
            .map(|model| {
                model.wave.map(|state| AlienWaveMethodState {
                    initialized: state.initialized,
                    primary_phase: state.primary_phase,
                    primary_step: state.primary_step,
                    secondary_phase: state.secondary_phase,
                    secondary_step: state.secondary_step,
                })
            })
            .collect();
        let callback_state = AlienCallbackSceneState {
            method_delta: asset.initial_method_delta,
            wave_selection: match asset.wave_scene.selection {
                AlienWaveSelectionData::Disabled => AlienWaveSelection::Disabled,
                AlienWaveSelectionData::Requested => AlienWaveSelection::Requested,
                AlienWaveSelectionData::Selected => AlienWaveSelection::Selected,
            },
            wave_current_sample: asset.wave_scene.current_sample,
            wave_selected_node: asset.wave_scene.selected_node.map(|node| AlienSceneNode {
                model_index: node.model_index,
                node_index: node.node_index,
            }),
            ..AlienCallbackSceneState::default()
        };
        Self {
            asset,
            species,
            control,
            camera,
            primary,
            models,
            wave_states,
            selected_model: None,
            callback_state,
            exit_requested: u16::MIN,
        }
    }

    /// Advance all currently translated native frame stages in original order.
    pub fn step(&mut self, mouse: AlienMouseSample) -> Result<AlienSceneFrame, AlienSceneError> {
        let camera_step = self.control.step(self.species, mouse);
        self.camera.update(
            AlienCameraAngles {
                pitch: self.control.pitch,
                pan: self.control.pan,
                secondary_pan: self.control.secondary_pan,
            },
            self.control.depth_velocity,
            &self.asset.trigonometry,
        );
        let primary = self
            .primary
            .project_and_select(self.camera.matrix, ORIGINAL_SCREEN_CENTER)?;
        let starfield = generate_starfield(
            self.asset.star_seed,
            self.camera.position,
            self.camera.matrix,
            &self.asset.star_shade_table,
        )?;

        let scene_camera = AlienTransformData {
            matrix: self.camera.matrix,
            translation: self.camera.transformed_view,
        };
        for (model_index, (model, pose)) in
            self.asset.models.iter().zip(&mut self.models).enumerate()
        {
            if model.behavior == AlienBehaviorMethod::Wave {
                let state = self.wave_states[model_index]
                    .as_mut()
                    .ok_or(AlienSceneError::MissingWaveState { model_index })?;
                update_or_initialize_wave(
                    self.species,
                    model_index,
                    pose,
                    state,
                    &mut self.callback_state,
                    self.camera.view,
                    &self.asset.trigonometry,
                )
                .map_err(|error| AlienSceneError::Wave { model_index, error })?;
            }
            let behavior_result = match model.behavior {
                AlienBehaviorMethod::WrapPositions => {
                    Some(wrap_positions(&mut pose.nodes, self.camera.view))
                }
                AlienBehaviorMethod::BoundsThenWrap => Some(bounds_then_wrap(
                    &mut pose.nodes,
                    self.camera.view,
                    &mut self.exit_requested,
                )),
                AlienBehaviorMethod::AnchorState => {
                    Some(anchor_state(&mut pose.nodes).map(|node_index| {
                        self.callback_state.active_node = Some(AlienSceneNode {
                            model_index,
                            node_index,
                        });
                    }))
                }
                AlienBehaviorMethod::AdjustState => Some(
                    adjust_state(
                        self.species,
                        &mut pose.nodes,
                        self.callback_state.method_delta,
                    )
                    .map(drop),
                ),
                AlienBehaviorMethod::NoOperation
                | AlienBehaviorMethod::Wave
                | AlienBehaviorMethod::AnimationDispatch
                | AlienBehaviorMethod::RingAnimation
                | AlienBehaviorMethod::PaletteUpdate
                | AlienBehaviorMethod::ApplySampleDelta
                | AlienBehaviorMethod::ApplyScaledSampleDelta
                | AlienBehaviorMethod::Resume => None,
            };
            if let Some(result) = behavior_result {
                result.map_err(|error| AlienSceneError::Behavior { model_index, error })?;
            }
            pose.transform_and_project(
                &model.mesh,
                scene_camera,
                ORIGINAL_SCREEN_CENTER,
                &self.asset.trigonometry,
            )
            .map_err(|error| AlienSceneError::ModelProjection { model_index, error })?;
        }
        let models = select_faces(self.species, &mut self.models)?;
        let geometry = prepare_render_geometry(
            &self.asset.primary_model.mesh,
            &self.primary,
            &primary,
            &self.asset.models,
            &self.models,
            &models,
            &self.asset.raster_reciprocals,
        )?;
        match models.behind_camera {
            AlienBehindCameraSignal::Unchanged => {}
            AlienBehindCameraSignal::General => {
                self.control.interaction_signal = ACTIVE_INTERACTION_SIGNAL;
            }
            AlienBehindCameraSignal::Model(model_index) => {
                self.selected_model = Some(model_index);
                self.control.interaction_signal = ACTIVE_INTERACTION_SIGNAL;
            }
        }

        Ok(AlienSceneFrame {
            camera_step,
            primary,
            starfield,
            models,
            geometry,
        })
    }

    /// Overlay species used by this scene.
    pub fn species(&self) -> AlienSpecies {
        self.species
    }

    /// Decoded authoritative resources retained by the scene.
    pub fn asset(&self) -> &AlienAsset {
        &self.asset
    }

    /// Whether a translated bounds method requested leaving this scene.
    pub fn exit_requested(&self) -> bool {
        self.exit_requested != u16::MIN
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::alien::{AlienXdbKind, decode_alien_xdb};

    use super::*;

    const CENTERED_MOUSE: AlienMouseSample = AlienMouseSample {
        x: 320,
        y: 512,
        buttons: 0,
    };
    const BOUNDS_ANGLE_STEP: u16 = 64;
    const STATE_ANGLE_STEP: u16 = 15;
    const BOUNDS_ANGLE_AXIS: usize = 1;
    const STATE_ANGLE_AXIS: usize = 2;
    const EXPECTED_INITIAL_METHOD_DELTA: i16 = -4;

    fn original_xdb(name: &str) -> Option<PathBuf> {
        [
            Path::new("output/_tmp_dat").join(name),
            Path::new("../../output/_tmp_dat").join(name),
        ]
        .into_iter()
        .find(|path| path.is_file())
    }

    #[test]
    fn every_original_alien_asset_runs_the_translated_frame_pipeline() {
        let cases = [
            (AlienXdbKind::Amer, "amer.xdb"),
            (AlienXdbKind::Croolis, "croolis.xdb"),
            (AlienXdbKind::Scrut, "scrut.xdb"),
        ];
        for (kind, filename) in cases {
            let Some(path) = original_xdb(filename) else {
                continue;
            };
            let data = std::fs::read(path).unwrap();
            let asset = decode_alien_xdb(&data, kind).unwrap();
            let model_count = asset.models.len();
            let mut scene = AlienScene::from_asset(asset);
            let initial_angles = scene
                .models
                .iter()
                .map(|model| model.nodes[0].angles)
                .collect::<Vec<_>>();
            let initial_wave_states = scene.wave_states.clone();
            let expected_wave_node =
                scene
                    .asset
                    .wave_scene
                    .selected_node
                    .map(|node| AlienSceneNode {
                        model_index: node.model_index,
                        node_index: node.node_index,
                    });
            let frame = scene.step(CENTERED_MOUSE).unwrap();
            assert_eq!(frame.models.decisions.len(), model_count);
            assert!(!frame.starfield.stars.is_empty());
            assert_eq!(scene.camera.view, INITIAL_VIEW);
            assert_eq!(
                scene.callback_state.method_delta,
                EXPECTED_INITIAL_METHOD_DELTA
            );

            let mut expected_anchor = None;
            for (model_index, model) in scene.asset.models.iter().enumerate() {
                let angles = scene.models[model_index].nodes[0].angles;
                match model.behavior {
                    AlienBehaviorMethod::BoundsThenWrap => assert_eq!(
                        angles[BOUNDS_ANGLE_AXIS],
                        initial_angles[model_index][BOUNDS_ANGLE_AXIS]
                            .wrapping_add(BOUNDS_ANGLE_STEP)
                    ),
                    AlienBehaviorMethod::AnchorState => {
                        assert_eq!(
                            angles[STATE_ANGLE_AXIS],
                            initial_angles[model_index][STATE_ANGLE_AXIS]
                                .wrapping_sub(STATE_ANGLE_STEP)
                        );
                        expected_anchor = Some(AlienSceneNode {
                            model_index,
                            node_index: usize::MIN,
                        });
                    }
                    AlienBehaviorMethod::AdjustState if kind == AlienXdbKind::Scrut => {
                        assert_eq!(
                            angles[STATE_ANGLE_AXIS],
                            initial_angles[model_index][STATE_ANGLE_AXIS]
                                .wrapping_sub(STATE_ANGLE_STEP)
                        );
                    }
                    AlienBehaviorMethod::AdjustState => assert_eq!(
                        angles[STATE_ANGLE_AXIS],
                        initial_angles[model_index][STATE_ANGLE_AXIS]
                    ),
                    _ => {}
                }
            }
            assert_eq!(scene.callback_state.active_node, expected_anchor);
            assert_eq!(
                scene.callback_state.wave_selection,
                AlienWaveSelection::Disabled
            );
            assert_eq!(scene.callback_state.wave_selected_node, expected_wave_node);
            for (before, after) in initial_wave_states.iter().zip(&scene.wave_states) {
                let (Some(before), Some(after)) = (before, after) else {
                    assert_eq!(before.is_none(), after.is_none());
                    continue;
                };
                assert!(before.initialized);
                assert_eq!(
                    after.primary_phase,
                    before
                        .primary_phase
                        .wrapping_add(before.primary_step as u16)
                );
                assert_eq!(
                    after.secondary_phase,
                    before
                        .secondary_phase
                        .wrapping_add(before.secondary_step as u16)
                );
            }
        }
    }
}
