//! Typed frame coordinator for the shared alien-scene geometry pipeline.

use std::fmt;

use commander_blood_formats::alien::{
    AXIS_COUNT, AlienAsset, AlienBehaviorMethod, AlienResumeCallbackData,
    AlienRingInitialCallbackData, AlienRingLifecycleData, AlienSlot2InitialCallbackData,
    AlienTransformData, AlienTrigonometryPair, AlienWaveSelectionData, AlienXdbKind,
    TRIGONOMETRY_ENTRY_COUNT,
};

use super::{
    AlienAmerFinishUpdate, AlienAmerLateSelectionUpdate, AlienAmerSelectionUpdate,
    AlienAmerUpdateHead, AlienBehaviorError, AlienBehindCameraSignal, AlienCallbackSceneState,
    AlienCameraAngles, AlienCameraControl, AlienCameraStep, AlienCameraTransform,
    AlienControlLatch, AlienCroolisCommonDispatch, AlienCroolisFadeUpdate, AlienCroolisResetUpdate,
    AlienCroolisSelectionUpdate, AlienCroolisUpdateHead, AlienFaceSelection,
    AlienFaceSelectionError, AlienModelPose, AlienMouseSample, AlienNodePose,
    AlienPaletteAnimationState, AlienPaletteError, AlienPaletteInput, AlienPrimaryMeshFrame,
    AlienPrimaryMeshPose, AlienPrimaryProjectionError, AlienProjectionError, AlienRasterError,
    AlienRenderGeometry, AlienResumeCallback, AlienResumeCallbacks, AlienResumeFinalStageError,
    AlienResumeMethodState, AlienResumePairContext, AlienResumePairStageError,
    AlienResumeQueueContext, AlienResumeQueueError, AlienResumeTextureError,
    AlienRingAnimationState, AlienRingCallback, AlienRingCallbacks, AlienRingEntry, AlienRingError,
    AlienRingLifecycle, AlienRingNodeState, AlienRingResumeState, AlienRingSharedState,
    AlienSceneNode, AlienScreenCenter, AlienScrutApproachUpdate, AlienScrutCommonDispatch,
    AlienScrutDampingUpdate, AlienScrutFadeUpdate, AlienScrutFinishUpdate, AlienScrutResetUpdate,
    AlienScrutSelectionBeginUpdate, AlienScrutSteeringPrecision, AlienScrutUpdateHead,
    AlienSelectionUpdate, AlienSlot2AnimationState, AlienSlot2Callback, AlienSlot2Callbacks,
    AlienSlot2Error, AlienSlot2NodeState, AlienSlot2SceneState, AlienSpecies, AlienStarfieldError,
    AlienStarfieldFrame, AlienWaveCallbackUpdate, AlienWaveError, AlienWaveMethodState,
    AlienWaveSelection, CROOLIS_AUTONOMOUS_RESET_DISTANCE, adjust_state, anchor_state,
    begin_amer_selection, begin_croolis_fade, begin_croolis_selection, begin_resume_clear,
    begin_scrut_finish, begin_scrut_selection, bounds_then_wrap, capture_resume_state,
    clear_next_ring_entry, continue_wave_steering, dispatch_croolis_common, dispatch_scrut_common,
    generate_starfield, initialize_or_dispatch_resume, initialize_or_dispatch_slot2,
    prepare_render_geometry, reset_amer_motion, reset_scrut_selection, restart_amer_update,
    restart_croolis_update, restart_initial_course, restart_scrut_selection, restart_scrut_update,
    select_faces, update_amer_common, update_amer_finish, update_amer_head,
    update_amer_late_selection, update_amer_return, update_amer_selection, update_amer_steering,
    update_croolis_fade, update_croolis_head, update_croolis_motion,
    update_croolis_reset_or_camera, update_croolis_selection, update_follow_course,
    update_initial_course, update_or_initialize_ring, update_or_initialize_wave,
    update_palette_animation, update_resume_final_stage, update_resume_pair_stage,
    update_resume_queue, update_resume_timeout, update_scrut_fade, update_scrut_finish,
    update_scrut_head, update_scrut_motion, update_scrut_reset_or_camera,
    update_scrut_selection_approach, update_scrut_selection_begin, update_scrut_selection_damping,
    update_scrut_steering, update_wave_callback, update_wave_camera, update_wave_finish,
    update_wave_motion, update_wave_return, update_wave_selection, wrap_positions,
};

const INITIAL_VIEW: [i16; AXIS_COUNT] = [1_885, -239, -9_790];
const INITIAL_PITCH: i16 = 0;
const INITIAL_PAN: i16 = 1_656;
const INITIAL_SECONDARY_PAN: i16 = 0;
const INITIAL_DEPTH_VELOCITY: i16 = 0;
const ACTIVE_INTERACTION_SIGNAL: u16 = 1;
const CAMERA_VERTICAL_AXIS: usize = 1;
const PRIMARY_BEHAVIOR_NODE: usize = 0;
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

/// Flat runtime context used to follow the recovered slot-2 callback graph.
struct AlienSceneSlot2Callbacks<'a> {
    model_index: usize,
    callback_scene: &'a mut AlienCallbackSceneState,
    camera: &'a AlienCameraTransform,
    camera_angles: AlienCameraAngles,
    camera_pan: u16,
    camera_depth_step: &'a mut i16,
    trigonometry: &'a [AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
}

impl AlienSceneSlot2Callbacks<'_> {
    fn invoke_callback(
        &mut self,
        species: AlienSpecies,
        callback: AlienSlot2Callback,
        pose: &mut AlienModelPose,
        animation: &mut AlienSlot2AnimationState,
        slot2_scene: &mut AlienSlot2SceneState,
    ) -> Result<(), AlienSlot2Error> {
        match callback {
            AlienSlot2Callback::Update => {
                self.invoke_update(species, pose, animation, slot2_scene)?;
            }
            AlienSlot2Callback::AmerReturn => {
                update_amer_return(pose, animation, &mut self.callback_scene.slot2_active)?;
            }
            AlienSlot2Callback::AmerSteer => {
                update_amer_steering(pose, animation, *self.camera_depth_step as u16)?;
            }
            AlienSlot2Callback::AmerFinish => {
                match update_amer_finish(pose, animation, *self.camera_depth_step as u16)? {
                    AlienAmerFinishUpdate::ResetRequested => reset_amer_motion(pose, animation)?,
                    AlienAmerFinishUpdate::SelectionWaitStarted
                    | AlienAmerFinishUpdate::Steering => {}
                }
            }
            AlienSlot2Callback::AmerSelectionWait => {
                let next = begin_amer_selection(pose, animation)?;
                self.invoke_callback(species, next, pose, animation, slot2_scene)?;
            }
            AlienSlot2Callback::AmerSelection => {
                self.invoke_amer_selection(species, pose, animation, slot2_scene)?;
            }
            AlienSlot2Callback::AmerSelectionLate => {
                self.invoke_amer_late_selection(species, pose, animation, slot2_scene)?;
            }
            AlienSlot2Callback::CroolisFade => {
                match update_croolis_fade(pose, animation, self.callback_scene)? {
                    AlienCroolisFadeUpdate::MotionRequested => {
                        update_croolis_motion(pose, animation)?;
                    }
                    AlienCroolisFadeUpdate::RestartRequested => {
                        let next = restart_croolis_update(pose, animation)?;
                        self.invoke_callback(species, next, pose, animation, slot2_scene)?;
                    }
                }
            }
            AlienSlot2Callback::CroolisSelection => {
                match update_croolis_selection(
                    self.model_index,
                    pose,
                    animation,
                    self.callback_scene,
                    self.camera.view[CAMERA_VERTICAL_AXIS],
                )? {
                    AlienCroolisSelectionUpdate::Tracking => {}
                    AlienCroolisSelectionUpdate::ResetRequested { camera_distance } => {
                        self.invoke_croolis_reset(
                            species,
                            pose,
                            animation,
                            camera_distance,
                            slot2_scene,
                        )?;
                    }
                }
            }
            AlienSlot2Callback::ScrutFade => {
                match update_scrut_fade(pose, animation, self.callback_scene)? {
                    AlienScrutFadeUpdate::MotionRequested => {
                        update_scrut_motion(pose, animation)?;
                    }
                    AlienScrutFadeUpdate::RestartRequested => {
                        let next = restart_scrut_update(pose, animation)?;
                        self.invoke_callback(species, next, pose, animation, slot2_scene)?;
                    }
                }
            }
            AlienSlot2Callback::ScrutSelectionBegin => {
                match update_scrut_selection_begin(pose, animation, self.callback_scene)? {
                    AlienScrutSelectionBeginUpdate::SelectionResetRequested => {
                        self.invoke_scrut_selection_reset(species, pose, animation, slot2_scene)?;
                    }
                    AlienScrutSelectionBeginUpdate::CameraResetRequested => {
                        self.invoke_scrut_reset(pose, animation)?;
                    }
                    AlienScrutSelectionBeginUpdate::DampingRequested => self.invoke_callback(
                        species,
                        AlienSlot2Callback::ScrutSelectionDamp,
                        pose,
                        animation,
                        slot2_scene,
                    )?,
                }
            }
            AlienSlot2Callback::ScrutSelectionDamp => {
                match update_scrut_selection_damping(pose, animation)? {
                    AlienScrutDampingUpdate::SteeringRequested => {
                        update_scrut_steering(
                            pose,
                            animation,
                            AlienScrutSteeringPrecision::Damping,
                        )?;
                    }
                    AlienScrutDampingUpdate::ApproachRequested => self.invoke_callback(
                        species,
                        AlienSlot2Callback::ScrutSelectionApproach,
                        pose,
                        animation,
                        slot2_scene,
                    )?,
                }
            }
            AlienSlot2Callback::ScrutSelectionApproach => {
                match update_scrut_selection_approach(
                    pose,
                    animation,
                    self.callback_scene,
                    self.camera,
                )? {
                    AlienScrutApproachUpdate::SelectionResetRequested => {
                        self.invoke_scrut_selection_reset(species, pose, animation, slot2_scene)?;
                    }
                    AlienScrutApproachUpdate::SelectionRestarted
                    | AlienScrutApproachUpdate::Steering => {}
                    AlienScrutApproachUpdate::FinishRequested => {
                        let next = begin_scrut_finish(pose, animation)?;
                        self.invoke_callback(species, next, pose, animation, slot2_scene)?;
                    }
                }
            }
            AlienSlot2Callback::ScrutFinish => {
                match update_scrut_finish(self.model_index, pose, animation, self.callback_scene)? {
                    AlienScrutFinishUpdate::Tracking | AlienScrutFinishUpdate::Descending => {}
                    AlienScrutFinishUpdate::SelectionInitRequested => {
                        self.invoke_scrut_selection_init(species, pose, animation, slot2_scene)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn invoke_update(
        &mut self,
        species: AlienSpecies,
        pose: &mut AlienModelPose,
        animation: &mut AlienSlot2AnimationState,
        slot2_scene: &mut AlienSlot2SceneState,
    ) -> Result<(), AlienSlot2Error> {
        match species {
            AlienSpecies::Amer => match update_amer_head(pose, animation, self.callback_scene)? {
                AlienAmerUpdateHead::SelectionRequested => {
                    let next = begin_amer_selection(pose, animation)?;
                    self.invoke_callback(species, next, pose, animation, slot2_scene)?;
                }
                AlienAmerUpdateHead::ResetRequested => reset_amer_motion(pose, animation)?,
                AlienAmerUpdateHead::CommonRequested => {
                    update_amer_common(
                        pose,
                        animation,
                        self.callback_scene,
                        self.camera,
                        self.camera_pan,
                        self.camera_depth_step,
                    )?;
                }
            },
            AlienSpecies::Croolis => {
                match update_croolis_head(pose, animation, self.callback_scene)? {
                    AlienCroolisUpdateHead::SelectionRequested => {
                        let next = begin_croolis_selection(pose, animation, self.callback_scene)?;
                        self.invoke_callback(species, next, pose, animation, slot2_scene)?;
                    }
                    AlienCroolisUpdateHead::CommonRequested => {
                        self.invoke_croolis_common(pose, animation, species, slot2_scene)?;
                    }
                    AlienCroolisUpdateHead::ResetRequested => self.invoke_croolis_reset(
                        species,
                        pose,
                        animation,
                        CROOLIS_AUTONOMOUS_RESET_DISTANCE,
                        slot2_scene,
                    )?,
                }
            }
            AlienSpecies::Scrut => match update_scrut_head(pose, animation, self.callback_scene)? {
                AlienScrutUpdateHead::SelectionRequested => {
                    self.invoke_scrut_selection_init(species, pose, animation, slot2_scene)?;
                }
                AlienScrutUpdateHead::CommonRequested => {
                    self.invoke_scrut_common(pose, animation)?;
                }
                AlienScrutUpdateHead::ResetRequested => {
                    self.invoke_scrut_reset(pose, animation)?;
                }
            },
        }
        Ok(())
    }

    fn invoke_amer_selection(
        &mut self,
        species: AlienSpecies,
        pose: &mut AlienModelPose,
        animation: &mut AlienSlot2AnimationState,
        slot2_scene: &mut AlienSlot2SceneState,
    ) -> Result<(), AlienSlot2Error> {
        match update_amer_selection(
            pose,
            animation,
            self.callback_scene,
            self.camera.view[CAMERA_VERTICAL_AXIS],
        )? {
            AlienAmerSelectionUpdate::RestartRequested => {
                let next = restart_amer_update(pose, animation)?;
                self.invoke_callback(species, next, pose, animation, slot2_scene)?;
            }
            AlienAmerSelectionUpdate::ResetRequested => reset_amer_motion(pose, animation)?,
            AlienAmerSelectionUpdate::LateSelectionStarted => {}
            AlienAmerSelectionUpdate::CommonRequested => {
                update_amer_common(
                    pose,
                    animation,
                    self.callback_scene,
                    self.camera,
                    self.camera_pan,
                    self.camera_depth_step,
                )?;
            }
        }
        Ok(())
    }

    fn invoke_amer_late_selection(
        &mut self,
        species: AlienSpecies,
        pose: &mut AlienModelPose,
        animation: &mut AlienSlot2AnimationState,
        slot2_scene: &mut AlienSlot2SceneState,
    ) -> Result<(), AlienSlot2Error> {
        match update_amer_late_selection(pose, animation, self.camera.view[CAMERA_VERTICAL_AXIS])? {
            AlienAmerLateSelectionUpdate::SelectionWaitRequested => {
                let next = begin_amer_selection(pose, animation)?;
                self.invoke_callback(species, next, pose, animation, slot2_scene)?;
            }
            AlienAmerLateSelectionUpdate::ResetRequested => reset_amer_motion(pose, animation)?,
            AlienAmerLateSelectionUpdate::CommonRequested => {
                update_amer_common(
                    pose,
                    animation,
                    self.callback_scene,
                    self.camera,
                    self.camera_pan,
                    self.camera_depth_step,
                )?;
            }
        }
        Ok(())
    }

    fn invoke_croolis_common(
        &mut self,
        pose: &mut AlienModelPose,
        animation: &mut AlienSlot2AnimationState,
        species: AlienSpecies,
        slot2_scene: &mut AlienSlot2SceneState,
    ) -> Result<(), AlienSlot2Error> {
        match dispatch_croolis_common(self.model_index, self.callback_scene) {
            AlienCroolisCommonDispatch::MotionRequested => {
                update_croolis_motion(pose, animation)?;
            }
            AlienCroolisCommonDispatch::FadeRequested => {
                let next = begin_croolis_fade(pose, animation)?;
                self.invoke_callback(species, next, pose, animation, slot2_scene)?;
            }
        }
        Ok(())
    }

    fn invoke_croolis_reset(
        &mut self,
        species: AlienSpecies,
        pose: &mut AlienModelPose,
        animation: &mut AlienSlot2AnimationState,
        camera_distance: i32,
        slot2_scene: &mut AlienSlot2SceneState,
    ) -> Result<(), AlienSlot2Error> {
        match update_croolis_reset_or_camera(
            pose,
            animation,
            self.camera,
            self.camera_angles,
            *self.camera_depth_step,
            self.trigonometry,
            camera_distance,
        )? {
            AlienCroolisResetUpdate::CommonRequested => {
                self.invoke_croolis_common(pose, animation, species, slot2_scene)?;
            }
            AlienCroolisResetUpdate::CameraReset => {}
        }
        Ok(())
    }

    fn invoke_scrut_selection_init(
        &mut self,
        species: AlienSpecies,
        pose: &mut AlienModelPose,
        animation: &mut AlienSlot2AnimationState,
        slot2_scene: &mut AlienSlot2SceneState,
    ) -> Result<(), AlienSlot2Error> {
        begin_scrut_selection(pose, animation, self.callback_scene)?;
        let next = restart_scrut_selection(pose, animation)?;
        self.invoke_callback(species, next, pose, animation, slot2_scene)
    }

    fn invoke_scrut_selection_reset(
        &mut self,
        species: AlienSpecies,
        pose: &mut AlienModelPose,
        animation: &mut AlienSlot2AnimationState,
        slot2_scene: &mut AlienSlot2SceneState,
    ) -> Result<(), AlienSlot2Error> {
        reset_scrut_selection(pose, animation, self.callback_scene)?;
        let next = restart_scrut_update(pose, animation)?;
        self.invoke_callback(species, next, pose, animation, slot2_scene)
    }

    fn invoke_scrut_common(
        &mut self,
        pose: &mut AlienModelPose,
        animation: &AlienSlot2AnimationState,
    ) -> Result<(), AlienSlot2Error> {
        match dispatch_scrut_common(self.model_index, self.callback_scene) {
            AlienScrutCommonDispatch::MotionRequested => update_scrut_motion(pose, animation)?,
            AlienScrutCommonDispatch::Halted => {}
        }
        Ok(())
    }

    fn invoke_scrut_reset(
        &mut self,
        pose: &mut AlienModelPose,
        animation: &mut AlienSlot2AnimationState,
    ) -> Result<(), AlienSlot2Error> {
        match update_scrut_reset_or_camera(
            pose,
            animation,
            self.camera,
            self.camera_angles,
            *self.camera_depth_step,
            self.trigonometry,
        )? {
            AlienScrutResetUpdate::CommonRequested => {
                self.invoke_scrut_common(pose, animation)?;
            }
            AlienScrutResetUpdate::CameraReset => {}
        }
        Ok(())
    }
}

impl AlienSlot2Callbacks for AlienSceneSlot2Callbacks<'_> {
    fn invoke(
        &mut self,
        species: AlienSpecies,
        callback: AlienSlot2Callback,
        pose: &mut AlienModelPose,
        animation: &mut AlienSlot2AnimationState,
        scene: &mut AlienSlot2SceneState,
    ) -> Result<(), AlienSlot2Error> {
        self.invoke_callback(species, callback, pose, animation, scene)
    }
}

/// Invalid typed ownership encountered while dispatching a resume callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumeRuntimeError {
    /// The resume behavior model has no primary node.
    MissingCurrentNode {
        /// Resume model missing its primary node.
        model_index: usize,
    },
    /// A stored scene-node identity selects no model.
    MissingModel {
        /// Invalid typed identity.
        node: AlienSceneNode,
    },
    /// A stored scene-node identity selects no node in its model.
    MissingNode {
        /// Invalid typed identity.
        node: AlienSceneNode,
    },
    /// A paired node has no ring-animation state.
    MissingRingState {
        /// Paired node whose model is not a ring owner.
        node: AlienSceneNode,
    },
    /// A paired node has no parallel ring callback state.
    MissingRingNodeState {
        /// Paired node missing its callback state.
        node: AlienSceneNode,
    },
    /// The final callback ran before an anchor behavior published its node.
    MissingAnchor,
    /// A pair or final callback ran without a typed paired node.
    MissingPairedNode,
    /// Resume and paired nodes unexpectedly occupy the same model owner.
    AliasedModel {
        /// Resume model node.
        current: AlienSceneNode,
        /// Paired ring node.
        paired: AlienSceneNode,
    },
    /// Queue consumption rejected its flat typed state.
    Queue(AlienResumeQueueError),
    /// Pair steering rejected its flat typed state.
    Pair(AlienResumePairStageError),
    /// Timeout texture animation rejected the model's coordinates.
    Texture(AlienResumeTextureError),
    /// Final steering rejected its flat typed state.
    Final(AlienResumeFinalStageError),
}

impl fmt::Display for AlienResumeRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid live alien resume state: {self:?}")
    }
}

impl std::error::Error for AlienResumeRuntimeError {}

impl From<AlienResumeQueueError> for AlienResumeRuntimeError {
    fn from(error: AlienResumeQueueError) -> Self {
        Self::Queue(error)
    }
}

impl From<AlienResumePairStageError> for AlienResumeRuntimeError {
    fn from(error: AlienResumePairStageError) -> Self {
        Self::Pair(error)
    }
}

impl From<AlienResumeTextureError> for AlienResumeRuntimeError {
    fn from(error: AlienResumeTextureError) -> Self {
        Self::Texture(error)
    }
}

impl From<AlienResumeFinalStageError> for AlienResumeRuntimeError {
    fn from(error: AlienResumeFinalStageError) -> Self {
        Self::Final(error)
    }
}

struct AlienSceneResumeCallbacks<'a> {
    current: AlienSceneNode,
    models: &'a mut [AlienModelPose],
    ring_states: &'a mut [Option<AlienRingAnimationState>],
    callback_scene: &'a mut AlienCallbackSceneState,
    countdown: &'a mut u16,
    trigonometry: &'a [AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
}

impl AlienSceneResumeCallbacks<'_> {
    fn invoke_begin(
        &mut self,
        species: AlienSpecies,
        state: &mut AlienResumeMethodState,
    ) -> Result<(), AlienResumeRuntimeError> {
        let paired = self
            .callback_scene
            .transition_queue
            .get(self.callback_scene.transition_queue_read_slot)
            .copied()
            .flatten();
        let current = self.current;
        let Self {
            models,
            ring_states,
            callback_scene,
            countdown,
            trigonometry,
            ..
        } = self;

        if let Some(paired) = paired {
            let paired_callback = ring_callback_mut(ring_states, paired)?;
            let (current_model, paired_model) = two_models_mut(models, current, paired)
                .ok_or(AlienResumeRuntimeError::AliasedModel { current, paired })?;
            let current_pose = current_model.nodes.get_mut(current.node_index).ok_or(
                AlienResumeRuntimeError::MissingCurrentNode {
                    model_index: current.model_index,
                },
            )?;
            let texture_coordinates = &mut current_model.texture_coordinates;
            let paired_pose = paired_model
                .nodes
                .get_mut(paired.node_index)
                .ok_or(AlienResumeRuntimeError::MissingNode { node: paired })?;
            update_resume_queue(
                species,
                state,
                callback_scene,
                AlienResumeQueueContext {
                    current,
                    current_pose,
                    texture_coordinates,
                    paired: Some(AlienResumePairContext {
                        node: paired,
                        pose: paired_pose,
                        callback: paired_callback,
                    }),
                    trigonometry,
                    countdown,
                },
            )?;
        } else {
            let current_model = models
                .get_mut(current.model_index)
                .ok_or(AlienResumeRuntimeError::MissingModel { node: current })?;
            let current_pose = current_model.nodes.get_mut(current.node_index).ok_or(
                AlienResumeRuntimeError::MissingCurrentNode {
                    model_index: current.model_index,
                },
            )?;
            update_resume_queue(
                species,
                state,
                callback_scene,
                AlienResumeQueueContext {
                    current,
                    current_pose,
                    texture_coordinates: &mut current_model.texture_coordinates,
                    paired: None,
                    trigonometry,
                    countdown,
                },
            )?;
        }
        Ok(())
    }

    fn invoke_pair(
        &mut self,
        species: AlienSpecies,
        state: &mut AlienResumeMethodState,
    ) -> Result<(), AlienResumeRuntimeError> {
        let current = self.current;
        let paired = state
            .paired_node
            .ok_or(AlienResumeRuntimeError::MissingPairedNode)?;
        let Self {
            models,
            ring_states,
            countdown,
            trigonometry,
            ..
        } = self;
        let paired_callback = ring_callback_mut(ring_states, paired)?;
        let (current_model, paired_model) = two_models_mut(models, current, paired)
            .ok_or(AlienResumeRuntimeError::AliasedModel { current, paired })?;
        let current_pose = current_model.nodes.get_mut(current.node_index).ok_or(
            AlienResumeRuntimeError::MissingCurrentNode {
                model_index: current.model_index,
            },
        )?;
        let texture_coordinates = &mut current_model.texture_coordinates;
        let paired_pose = paired_model
            .nodes
            .get_mut(paired.node_index)
            .ok_or(AlienResumeRuntimeError::MissingNode { node: paired })?;
        update_resume_pair_stage(
            species,
            state,
            current_pose,
            paired_pose,
            paired_callback,
            texture_coordinates,
            trigonometry,
            countdown,
        )?;
        Ok(())
    }

    fn invoke_timeout(
        &mut self,
        species: AlienSpecies,
        state: &mut AlienResumeMethodState,
    ) -> Result<(), AlienResumeRuntimeError> {
        let current_model = self
            .models
            .get_mut(self.current.model_index)
            .ok_or(AlienResumeRuntimeError::MissingModel { node: self.current })?;
        update_resume_timeout(
            species,
            state,
            &mut current_model.texture_coordinates,
            self.countdown,
        )?;
        Ok(())
    }

    fn invoke_final(
        &mut self,
        species: AlienSpecies,
        state: &mut AlienResumeMethodState,
    ) -> Result<(), AlienResumeRuntimeError> {
        let current = self.current;
        let paired = state
            .paired_node
            .ok_or(AlienResumeRuntimeError::MissingPairedNode)?;
        let anchor = self
            .callback_scene
            .active_node
            .ok_or(AlienResumeRuntimeError::MissingAnchor)?;
        let anchor_pose = model_node(self.models, anchor)?.clone();
        let paired_callback = ring_callback_mut(self.ring_states, paired)?;
        let current_model = self
            .models
            .get_mut(current.model_index)
            .ok_or(AlienResumeRuntimeError::MissingModel { node: current })?;
        let current_pose = current_model.nodes.get_mut(current.node_index).ok_or(
            AlienResumeRuntimeError::MissingCurrentNode {
                model_index: current.model_index,
            },
        )?;
        update_resume_final_stage(
            species,
            state,
            current_pose,
            &anchor_pose,
            paired_callback,
            self.trigonometry,
        )?;
        Ok(())
    }
}

impl AlienResumeCallbacks for AlienSceneResumeCallbacks<'_> {
    type Error = AlienResumeRuntimeError;

    fn invoke(
        &mut self,
        species: AlienSpecies,
        callback: AlienResumeCallback,
        state: &mut AlienResumeMethodState,
    ) -> Result<(), Self::Error> {
        match callback {
            AlienResumeCallback::Begin => self.invoke_begin(species, state),
            AlienResumeCallback::Pair => self.invoke_pair(species, state),
            AlienResumeCallback::Timeout => self.invoke_timeout(species, state),
            AlienResumeCallback::Final => self.invoke_final(species, state),
        }
    }
}

fn model_node(
    models: &[AlienModelPose],
    node: AlienSceneNode,
) -> Result<&AlienNodePose, AlienResumeRuntimeError> {
    models
        .get(node.model_index)
        .ok_or(AlienResumeRuntimeError::MissingModel { node })?
        .nodes
        .get(node.node_index)
        .ok_or(AlienResumeRuntimeError::MissingNode { node })
}

fn ring_callback_mut(
    states: &mut [Option<AlienRingAnimationState>],
    node: AlienSceneNode,
) -> Result<&mut AlienRingCallback, AlienResumeRuntimeError> {
    states
        .get_mut(node.model_index)
        .ok_or(AlienResumeRuntimeError::MissingModel { node })?
        .as_mut()
        .ok_or(AlienResumeRuntimeError::MissingRingState { node })?
        .nodes
        .get_mut(node.node_index)
        .map(|state| &mut state.callback)
        .ok_or(AlienResumeRuntimeError::MissingRingNodeState { node })
}

fn two_models_mut(
    models: &mut [AlienModelPose],
    current: AlienSceneNode,
    paired: AlienSceneNode,
) -> Option<(&mut AlienModelPose, &mut AlienModelPose)> {
    if current.model_index == paired.model_index {
        return None;
    }
    if current.model_index < paired.model_index {
        let (before, from_paired) = models.split_at_mut(paired.model_index);
        Some((
            before.get_mut(current.model_index)?,
            from_paired.first_mut()?,
        ))
    } else {
        let (before, from_current) = models.split_at_mut(current.model_index);
        Some((
            from_current.first_mut()?,
            before.get_mut(paired.model_index)?,
        ))
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
    /// Per-model callback state for authored slot-2/4 animation methods.
    slot2_states: Vec<Option<AlienSlot2AnimationState>>,
    /// Per-model callback state for authored resume methods.
    resume_states: Vec<Option<AlienResumeMethodState>>,
    /// Scene-wide motion history shared by every ring-animation model.
    ring_shared: AlienRingSharedState,
    /// Scene-wide captured-node state used by ring resume transitions.
    ring_resume: AlienRingResumeState,
    /// Deterministic random state shared by translated alien callbacks.
    behavior_random_state: u16,
    /// Species seed shared by slot-2 model initialization in authored order.
    slot2_scene: AlienSlot2SceneState,
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
    /// A decoded model selected a method-table routine unused by shipped scenes.
    UnassignedSampleBehavior {
        /// Model selecting the unassigned method.
        model_index: usize,
        /// Sample method selected by that model.
        behavior: AlienBehaviorMethod,
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
    /// A slot-2 animation model has no decoded continuation state.
    MissingSlot2State {
        /// Model missing its state.
        model_index: usize,
    },
    /// One slot-2 callback rejected its typed state.
    Slot2 {
        /// Model that failed.
        model_index: usize,
        /// Underlying slot-2 callback failure.
        error: AlienSlot2Error,
    },
    /// A resume behavior model has no decoded continuation state.
    MissingResumeState {
        /// Model missing its state.
        model_index: usize,
    },
    /// One live resume callback rejected its typed ownership state.
    Resume {
        /// Model that failed.
        model_index: usize,
        /// Underlying resume dispatch failure.
        error: AlienResumeRuntimeError,
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
        let slot2_states = asset
            .models
            .iter()
            .map(|model| {
                model.slot2.as_ref().map(|slot2| AlienSlot2AnimationState {
                    initialized: slot2.initialized,
                    callback: slot2.callback.map(|callback| match callback {
                        AlienSlot2InitialCallbackData::Update => AlienSlot2Callback::Update,
                    }),
                    phase_timer: slot2.phase_timer,
                    croolis_motion_accumulator: slot2.croolis_motion_accumulator,
                    species_seed_at_initialization: slot2.species_seed_at_initialization,
                    random_value: slot2.random_value,
                    amer_animation_phase: slot2.amer_animation_phase,
                    amer_velocity: slot2.amer_velocity,
                    nodes: slot2
                        .nodes
                        .iter()
                        .map(|node| AlienSlot2NodeState {
                            motion_parameter: node.motion_parameter,
                            radial_target: node.radial_target,
                            secondary_motion_parameter: node.secondary_motion_parameter,
                            behavior_seed: node.behavior_seed,
                        })
                        .collect(),
                })
            })
            .collect();
        let resume_states = asset
            .models
            .iter()
            .map(|model| {
                model.resume.map(|resume| AlienResumeMethodState {
                    callback: resume.callback.map(|callback| match callback {
                        AlienResumeCallbackData::Begin => AlienResumeCallback::Begin,
                        AlienResumeCallbackData::Pair => AlienResumeCallback::Pair,
                        AlienResumeCallbackData::Timeout => AlienResumeCallback::Timeout,
                        AlienResumeCallbackData::Final => AlienResumeCallback::Final,
                    }),
                    phase: resume.phase,
                    paired_node: resume.paired_node.map(|node| AlienSceneNode {
                        model_index: node.model_index,
                        node_index: node.node_index,
                    }),
                    resumed_node: resume.resumed_node.map(|node| AlienSceneNode {
                        model_index: node.model_index,
                        node_index: node.node_index,
                    }),
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
        let slot2_scene = AlienSlot2SceneState {
            species_seed: asset.slot2_scene.species_seed,
        };
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
            slot2_active: asset.slot2_scene.active,
            transition_queue: asset.resume_scene.queue.map(|node| {
                node.map(|node| AlienSceneNode {
                    model_index: node.model_index,
                    node_index: node.node_index,
                })
            }),
            transition_queue_slot: asset.resume_scene.write_slot,
            transition_queue_read_slot: asset.resume_scene.read_slot,
            active_node: asset.resume_scene.anchor_node.map(|node| AlienSceneNode {
                model_index: node.model_index,
                node_index: node.node_index,
            }),
            current_node: asset.resume_scene.current_node.map(|node| AlienSceneNode {
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
            slot2_states,
            resume_states,
            ring_shared,
            ring_resume,
            behavior_random_state,
            slot2_scene,
            palette_state,
            selected_model: None,
            callback_state,
            exit_requested: u16::MIN,
        }
    }

    /// Advance all currently translated native frame stages in original order.
    pub fn step(&mut self, mouse: AlienMouseSample) -> Result<AlienSceneFrame, AlienSceneError> {
        self.control.interaction_signal = control_input_signal(self.callback_state.control_latch);
        let camera_step = self.control.step(self.species, mouse);
        if self.species == AlienSpecies::Amer {
            self.callback_state.control_latch = AlienControlLatch::Inactive;
        }
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
        for model_index in usize::MIN..self.asset.models.len() {
            let behavior = self.asset.models[model_index].behavior;
            if behavior == AlienBehaviorMethod::Resume {
                let state = self.resume_states[model_index]
                    .as_mut()
                    .ok_or(AlienSceneError::MissingResumeState { model_index })?;
                let mut callbacks = AlienSceneResumeCallbacks {
                    current: AlienSceneNode {
                        model_index,
                        node_index: PRIMARY_BEHAVIOR_NODE,
                    },
                    models: &mut self.models,
                    ring_states: &mut self.ring_states,
                    callback_scene: &mut self.callback_state,
                    countdown: &mut self.ring_resume.countdown,
                    trigonometry: &self.asset.trigonometry,
                };
                initialize_or_dispatch_resume(self.species, state, &mut callbacks)
                    .map_err(|error| AlienSceneError::Resume { model_index, error })?;
            } else {
                let pose = &mut self.models[model_index];
                if behavior == AlienBehaviorMethod::PaletteUpdate {
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
                if behavior == AlienBehaviorMethod::Wave {
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
                if behavior == AlienBehaviorMethod::RingAnimation {
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
                if behavior == AlienBehaviorMethod::AnimationDispatch {
                    let state = self.slot2_states[model_index]
                        .as_mut()
                        .ok_or(AlienSceneError::MissingSlot2State { model_index })?;
                    let mut callbacks = AlienSceneSlot2Callbacks {
                        model_index,
                        callback_scene: &mut self.callback_state,
                        camera: &self.camera,
                        camera_angles: AlienCameraAngles {
                            pitch: self.control.pitch,
                            pan: self.control.pan,
                            secondary_pan: self.control.secondary_pan,
                        },
                        camera_pan: self.control.pan as u16,
                        camera_depth_step: &mut self.control.depth_velocity,
                        trigonometry: &self.asset.trigonometry,
                    };
                    initialize_or_dispatch_slot2(
                        self.species,
                        pose,
                        state,
                        &mut self.slot2_scene,
                        &mut self.behavior_random_state,
                        &mut callbacks,
                    )
                    .map_err(|error| AlienSceneError::Slot2 { model_index, error })?;
                }
                let behavior_result = match behavior {
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
                    AlienBehaviorMethod::ApplySampleDelta
                    | AlienBehaviorMethod::ApplyScaledSampleDelta => {
                        return Err(AlienSceneError::UnassignedSampleBehavior {
                            model_index,
                            behavior,
                        });
                    }
                    AlienBehaviorMethod::NoOperation => {
                        super::run_no_operation();
                        None
                    }
                    AlienBehaviorMethod::Wave
                    | AlienBehaviorMethod::AnimationDispatch
                    | AlienBehaviorMethod::RingAnimation
                    | AlienBehaviorMethod::PaletteUpdate
                    | AlienBehaviorMethod::Resume => None,
                };
                if let Some(result) = behavior_result {
                    result.map_err(|error| AlienSceneError::Behavior { model_index, error })?;
                }
            }
            self.models[model_index]
                .transform_and_project(
                    &self.asset.models[model_index].mesh,
                    scene_camera,
                    ORIGINAL_SCREEN_CENTER,
                    &self.asset.trigonometry,
                )
                .map_err(|error| AlienSceneError::ModelProjection { model_index, error })?;
        }
        if self.species != AlienSpecies::Amer {
            self.callback_state.control_latch = AlienControlLatch::Inactive;
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
                self.callback_state.control_latch = AlienControlLatch::Signal;
            }
            AlienBehindCameraSignal::Model(model_index) => {
                self.selected_model = Some(model_index);
                self.callback_state.control_latch = AlienControlLatch::Model(model_index);
            }
        }
        self.control.interaction_signal = control_input_signal(self.callback_state.control_latch);

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

fn control_input_signal(control_latch: AlienControlLatch) -> u16 {
    match control_latch {
        AlienControlLatch::Inactive => u16::MIN,
        AlienControlLatch::Signal | AlienControlLatch::Model(_) => ACTIVE_INTERACTION_SIGNAL,
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::alien::{AlienXdbKind, decode_alien_xdb};

    use crate::native::alien::AlienResumeUpdate;

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
    const EXPECTED_PAIRED_RESUME_PHASE: u16 = 2;
    const EXPECTED_PAIRED_RESUME_COUNTDOWN: u16 = 24;
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
            let initial_slot2_states = scene.slot2_states.clone();
            let initial_resume_states = scene.resume_states.clone();
            let initial_resume_read_slot = scene.callback_state.transition_queue_read_slot;
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
            for (before, after) in initial_slot2_states.iter().zip(&scene.slot2_states) {
                let (Some(before), Some(after)) = (before, after) else {
                    assert_eq!(before.is_none(), after.is_none());
                    continue;
                };
                assert!(before.initialized);
                assert_ne!(before, after);
            }
            assert_eq!(scene.resume_states, initial_resume_states);
            assert_eq!(
                scene.callback_state.transition_queue_read_slot,
                (initial_resume_read_slot + 1) % scene.callback_state.transition_queue.len()
            );
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

    #[test]
    fn every_original_resume_model_consumes_a_typed_ring_node() {
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
            let resume_model = asset
                .models
                .iter()
                .position(|model| model.behavior == AlienBehaviorMethod::Resume)
                .unwrap();
            let paired_model = asset
                .models
                .iter()
                .position(|model| model.behavior == AlienBehaviorMethod::RingAnimation)
                .unwrap();
            let current = AlienSceneNode {
                model_index: resume_model,
                node_index: PRIMARY_BEHAVIOR_NODE,
            };
            let paired = AlienSceneNode {
                model_index: paired_model,
                node_index: PRIMARY_BEHAVIOR_NODE,
            };
            let mut scene = AlienScene::from_asset(asset);
            scene.models[current.model_index].nodes[current.node_index].local_position =
                [i32::MIN; AXIS_COUNT];
            scene.models[paired.model_index].nodes[paired.node_index].local_position =
                [i32::MIN; AXIS_COUNT];
            let read_slot = scene.callback_state.transition_queue_read_slot;
            let anchor = scene.callback_state.active_node;
            scene.callback_state.transition_queue[read_slot] = Some(paired);
            scene.callback_state.current_node = Some(paired);

            {
                let state = scene.resume_states[resume_model].as_mut().unwrap();
                let mut callbacks = AlienSceneResumeCallbacks {
                    current,
                    models: &mut scene.models,
                    ring_states: &mut scene.ring_states,
                    callback_scene: &mut scene.callback_state,
                    countdown: &mut scene.ring_resume.countdown,
                    trigonometry: &scene.asset.trigonometry,
                };
                assert_eq!(
                    initialize_or_dispatch_resume(scene.species, state, &mut callbacks).unwrap(),
                    AlienResumeUpdate::CallbackInvoked
                );
            }

            let state = scene.resume_states[resume_model].unwrap();
            assert_eq!(state.callback, Some(AlienResumeCallback::Timeout));
            assert_eq!(state.phase, EXPECTED_PAIRED_RESUME_PHASE);
            assert_eq!(state.paired_node, Some(paired));
            assert_eq!(state.resumed_node, Some(paired));
            assert_eq!(scene.callback_state.transition_queue[read_slot], None);
            assert_eq!(scene.callback_state.current_node, None);
            assert_eq!(scene.callback_state.active_node, anchor);
            assert_eq!(
                scene.ring_resume.countdown,
                EXPECTED_PAIRED_RESUME_COUNTDOWN
            );
            assert_eq!(
                scene.ring_states[paired.model_index]
                    .as_ref()
                    .unwrap()
                    .nodes[paired.node_index]
                    .callback,
                AlienRingCallback::BeginResumeClear
            );
        }
    }
}
