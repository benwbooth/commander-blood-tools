//! Typed frame coordinator for the shared alien-scene geometry pipeline.

use std::fmt;

use commander_blood_formats::alien::{
    AXIS_COUNT, AlienAsset, AlienBehaviorMethod, AlienRingInitialCallbackData,
    AlienRingLifecycleData, AlienTransformData, AlienTrigonometryPair, AlienWaveSelectionData,
    AlienXdbKind, TRIGONOMETRY_ENTRY_COUNT,
};

use super::{
    AlienBehaviorError, AlienBehindCameraSignal, AlienCallbackSceneState, AlienCameraAngles,
    AlienCameraControl, AlienCameraStep, AlienCameraTransform, AlienFaceSelection,
    AlienFaceSelectionError, AlienModelPose, AlienMouseSample, AlienPaletteAnimationState,
    AlienPaletteError, AlienPaletteInput, AlienPrimaryMeshFrame, AlienPrimaryMeshPose,
    AlienPrimaryProjectionError, AlienProjectionError, AlienRasterError, AlienRenderGeometry,
    AlienRingAnimationState, AlienRingCallback, AlienRingCallbacks, AlienRingEntry, AlienRingError,
    AlienRingLifecycle, AlienRingNodeState, AlienRingResumeState, AlienRingSharedState,
    AlienSceneNode, AlienScreenCenter, AlienSelectionUpdate, AlienSpecies, AlienStarfieldError,
    AlienStarfieldFrame, AlienWaveCallbackUpdate, AlienWaveError, AlienWaveMethodState,
    AlienWaveSelection, adjust_state, anchor_state, begin_resume_clear, bounds_then_wrap,
    capture_resume_state, clear_next_ring_entry, continue_wave_steering, generate_starfield,
    prepare_render_geometry, restart_initial_course, select_faces, update_follow_course,
    update_initial_course, update_or_initialize_ring, update_or_initialize_wave,
    update_palette_animation, update_wave_callback, update_wave_camera, update_wave_finish,
    update_wave_motion, update_wave_return, update_wave_selection, wrap_positions,
};

const INITIAL_VIEW: [i16; AXIS_COUNT] = [1_885, -239, -9_790];
const INITIAL_PITCH: i16 = 0;
const INITIAL_PAN: i16 = 1_656;
const INITIAL_SECONDARY_PAN: i16 = 0;
const INITIAL_DEPTH_VELOCITY: i16 = 0;
const ACTIVE_INTERACTION_SIGNAL: u16 = 1;
const ORIGINAL_SCREEN_CENTER: AlienScreenCenter = AlienScreenCenter { x: 160, y: 100 };

struct AlienSceneRingCallbacks<'a> {
    model_index: usize,
    scene: &'a mut AlienCallbackSceneState,
    resume: &'a mut AlienRingResumeState,
    random_state: &'a mut u16,
    camera_view: [i16; AXIS_COUNT],
    camera_pan: u16,
    trigonometry: &'a [AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
}

impl AlienSceneRingCallbacks<'_> {
    fn invoke_wave_selection(
        &mut self,
        species: AlienSpecies,
        node_index: usize,
        pose: &mut AlienModelPose,
        animation: &mut AlienRingAnimationState,
    ) -> Result<(), AlienRingError> {
        match update_wave_selection(
            species,
            self.model_index,
            node_index,
            pose,
            animation,
            self.scene,
        )? {
            AlienSelectionUpdate::MotionContinuationRequested => continue_wave_steering(
                node_index,
                pose,
                animation,
                self.camera_view,
                self.trigonometry,
            )?,
            AlienSelectionUpdate::CameraUpdateRequested => {
                update_wave_camera(node_index, self.camera_pan, pose, animation)?;
            }
            AlienSelectionUpdate::WaveStarted => {}
        }
        Ok(())
    }

    fn invoke_wave(
        &mut self,
        species: AlienSpecies,
        node_index: usize,
        pose: &mut AlienModelPose,
        animation: &mut AlienRingAnimationState,
    ) -> Result<(), AlienRingError> {
        match update_wave_callback(
            species,
            node_index,
            pose,
            animation,
            self.scene,
            self.camera_view,
        )? {
            AlienWaveCallbackUpdate::Waiting => {}
            AlienWaveCallbackUpdate::FinishRequested => {
                update_wave_finish(node_index, self.scene.wave_current_sample as u16, pose)?;
            }
            AlienWaveCallbackUpdate::CameraUpdateRequested => {
                update_wave_camera(node_index, self.camera_pan, pose, animation)?;
            }
        }
        Ok(())
    }
}

impl AlienRingCallbacks for AlienSceneRingCallbacks<'_> {
    fn invoke(
        &mut self,
        species: AlienSpecies,
        callback: AlienRingCallback,
        node_index: usize,
        pose: &mut AlienModelPose,
        animation: &mut AlienRingAnimationState,
        shared: &mut AlienRingSharedState,
    ) -> Result<(), AlienRingError> {
        match callback {
            AlienRingCallback::InitialCourse => {
                update_initial_course(species, node_index, pose, animation, shared)?;
            }
            AlienRingCallback::RestartInitialCourse => {
                restart_initial_course(node_index, pose, animation, shared, self.random_state)?
            }
            AlienRingCallback::BeginResumeClear => {
                begin_resume_clear(species, node_index, pose, animation, shared)?;
            }
            AlienRingCallback::FollowCourse => {
                match update_follow_course(
                    self.model_index,
                    node_index,
                    pose,
                    animation,
                    shared,
                    self.scene,
                )? {
                    super::AlienRingFollowerUpdate::FeedbackAdvanced => {}
                    super::AlienRingFollowerUpdate::CaptureResumeRequested => {
                        capture_resume_state(
                            species,
                            self.model_index,
                            node_index,
                            pose,
                            self.resume,
                        )?;
                    }
                    super::AlienRingFollowerUpdate::RestartInitialCourseRequested => {
                        restart_initial_course(
                            node_index,
                            pose,
                            animation,
                            shared,
                            self.random_state,
                        )?;
                    }
                    super::AlienRingFollowerUpdate::WaveSelectionRequested => {
                        self.invoke_wave_selection(species, node_index, pose, animation)?;
                    }
                }
            }
            AlienRingCallback::ClearHistory => {
                clear_next_ring_entry(node_index, animation, shared)?;
            }
            AlienRingCallback::Wave => {
                self.invoke_wave(species, node_index, pose, animation)?;
            }
            AlienRingCallback::WaveFinish => {
                update_wave_finish(node_index, self.scene.wave_current_sample as u16, pose)?;
            }
            AlienRingCallback::WaveMotion => {
                update_wave_motion(node_index, pose, animation)?;
            }
            AlienRingCallback::WaveReturn => {
                update_wave_return(node_index, pose, animation)?;
            }
            AlienRingCallback::WaveSelection => {
                self.invoke_wave_selection(species, node_index, pose, animation)?;
            }
        }
        Ok(())
    }
}

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
    /// Complete indexed atlas after a palette-remap frame, when it changed.
    pub texture_update: Option<Vec<u8>>,
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
    /// Per-model callback state for authored ring-animation methods.
    ring_states: Vec<Option<AlienRingAnimationState>>,
    /// Scene-wide motion history shared by every ring-animation model.
    ring_shared: AlienRingSharedState,
    /// Scene-wide captured-node state used by ring resume transitions.
    ring_resume: AlienRingResumeState,
    /// Deterministic random state shared by translated alien callbacks.
    behavior_random_state: u16,
    /// Shared continuation state for the palette-animation method.
    palette_state: AlienPaletteAnimationState,
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
    /// A ring-animation model has no decoded continuation state.
    MissingRingState {
        /// Model missing its state.
        model_index: usize,
    },
    /// One ring-animation method rejected its typed state.
    Ring {
        /// Model that failed.
        model_index: usize,
        /// Underlying ring or wave-continuation failure.
        error: AlienRingError,
    },
    /// The palette-animation method rejected its typed state.
    Palette(AlienPaletteError),
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
        let ring_states = asset
            .models
            .iter()
            .map(|model| {
                model.ring.as_ref().map(|ring| AlienRingAnimationState {
                    lifecycle: match ring.lifecycle {
                        AlienRingLifecycleData::Uninitialized => AlienRingLifecycle::Uninitialized,
                        AlienRingLifecycleData::TimerRunning => AlienRingLifecycle::TimerRunning,
                        AlienRingLifecycleData::TimerSuspended => {
                            AlienRingLifecycle::TimerSuspended
                        }
                    },
                    nodes: ring
                        .nodes
                        .iter()
                        .map(|node| AlienRingNodeState {
                            callback: match node.callback {
                                AlienRingInitialCallbackData::InitialCourse => {
                                    AlienRingCallback::InitialCourse
                                }
                                AlienRingInitialCallbackData::FollowCourse => {
                                    AlienRingCallback::FollowCourse
                                }
                            },
                            course_frames_remaining: node.course_frames_remaining,
                            feedback_phase: node.feedback_phase,
                            ring_slot: node.ring_slot,
                            behavior_seed: node.behavior_seed,
                            ..AlienRingNodeState::default()
                        })
                        .collect(),
                })
            })
            .collect();
        let ring_shared = AlienRingSharedState {
            timer: asset.ring_scene.timer,
            generation: asset.ring_scene.generation,
            next_ring_slot: asset.ring_scene.next_ring_slot,
            entries: asset.ring_scene.entries.map(|entry| AlienRingEntry {
                pitch_step: entry.pitch_step,
                pan_step: entry.pan_step,
                radial_offset: entry.radial_offset,
                command_flags: entry.command_flags,
            }),
        };
        let ring_resume = AlienRingResumeState {
            countdown: asset.ring_scene.resume_countdown,
            selected_node: asset.ring_scene.resume_node.map(|node| AlienSceneNode {
                model_index: node.model_index,
                node_index: node.node_index,
            }),
        };
        let behavior_random_state = asset.initial_behavior_random_state;
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
        let palette_state = AlienPaletteAnimationState {
            previous_level: asset.palette_animation.previous_level,
            step: asset.palette_animation.step,
            countdown: asset.palette_animation.countdown,
            pulse_countdown: asset.palette_animation.pulse_countdown,
            pulse_levels: asset.palette_animation.pulse_levels,
        };
        Self {
            asset,
            species,
            control,
            camera,
            primary,
            models,
            wave_states,
            ring_states,
            ring_shared,
            ring_resume,
            behavior_random_state,
            palette_state,
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
        let palette_input = AlienPaletteInput {
            x: camera_step.centered_cursor[0],
            y: camera_step.centered_cursor[1],
        };
        let mut texture_changed = false;
        for (model_index, (model, pose)) in
            self.asset.models.iter().zip(&mut self.models).enumerate()
        {
            if model.behavior == AlienBehaviorMethod::PaletteUpdate {
                let update = update_palette_animation(
                    self.species,
                    pose,
                    palette_input,
                    &mut self.callback_state.method_delta,
                    &mut self.palette_state,
                    &mut self.asset.texture.pixels,
                    &self.asset.palette_remap,
                )
                .map_err(AlienSceneError::Palette)?;
                texture_changed |= update.changed_texture_bytes != usize::MIN;
            }
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
            if model.behavior == AlienBehaviorMethod::RingAnimation {
                let state = self.ring_states[model_index]
                    .as_mut()
                    .ok_or(AlienSceneError::MissingRingState { model_index })?;
                let mut callbacks = AlienSceneRingCallbacks {
                    model_index,
                    scene: &mut self.callback_state,
                    resume: &mut self.ring_resume,
                    random_state: &mut self.behavior_random_state,
                    camera_view: self.camera.view,
                    camera_pan: self.control.pan as u16,
                    trigonometry: &self.asset.trigonometry,
                };
                update_or_initialize_ring(
                    self.species,
                    pose,
                    state,
                    &mut self.ring_shared,
                    &mut callbacks,
                )
                .map_err(|error| AlienSceneError::Ring { model_index, error })?;
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
            texture_update: texture_changed.then(|| self.asset.texture.pixels.clone()),
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
    const REMAP_TEST_LEVEL: i16 = 60;
    const REMAP_TEST_PREVIOUS_LEVEL: u16 = 56;

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
            let initial_ring_timer = scene.ring_shared.timer;
            let initial_ring_states = scene.ring_states.clone();
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
            assert_eq!(scene.ring_shared.timer, initial_ring_timer.wrapping_sub(1));
            let ring_advanced = scene.ring_shared.timer == u16::MIN;
            let ring_entry_count = scene.ring_shared.entries.len();
            for (before, after) in initial_ring_states.iter().zip(&scene.ring_states) {
                let (Some(before), Some(after)) = (before, after) else {
                    assert_eq!(before.is_none(), after.is_none());
                    continue;
                };
                assert_eq!(before.lifecycle, after.lifecycle);
                for (before, after) in before.nodes.iter().zip(&after.nodes) {
                    assert_eq!(
                        after.ring_slot,
                        if ring_advanced {
                            (before.ring_slot + 1) % ring_entry_count
                        } else {
                            before.ring_slot
                        }
                    );
                }
            }
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
            assert!(frame.texture_update.is_none());
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

            scene.callback_state.method_delta = REMAP_TEST_LEVEL;
            scene.palette_state.previous_level = REMAP_TEST_PREVIOUS_LEVEL;
            let remapped_texture = scene
                .step(CENTERED_MOUSE)
                .unwrap()
                .texture_update
                .expect("the verified palette range must remap texture indices");
            assert_eq!(
                remapped_texture.len(),
                scene.asset.texture.width * scene.asset.texture.height
            );
        }
    }
}
