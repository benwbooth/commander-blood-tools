//! Shared native engine used by the AMER, CROOLIS, and SCRUT 3D scenes.

mod behavior;
mod callback;
mod camera;
mod control;
mod faces;
mod palette;
mod primary;
mod projection;
mod raster;
mod resume;
mod ring;
mod scene;
mod selection;
mod slot2;
mod starfield;
mod wave;

pub use behavior::{
    AlienBehaviorError, AlienSampleState, adjust_state, anchor_state, apply_sample_delta,
    apply_scaled_sample_delta, bounds_then_wrap, wrap_positions,
};
pub use callback::{AlienCallbackSceneState, AlienControlLatch};
pub use camera::{AlienCameraAngles, AlienCameraTransform};
pub use control::{
    AlienCameraControl, AlienCameraStep, AlienInputAction, AlienMouseSample, AlienSpecies,
};
pub use faces::{
    AlienBehindCameraSignal, AlienFaceBucket, AlienFaceDecision, AlienFaceReference,
    AlienFaceSelection, AlienFaceSelectionError, select_faces,
};
pub use palette::{
    AlienPaletteAnimationState, AlienPaletteError, AlienPaletteInput, AlienPaletteUpdate,
    update_palette_animation,
};
pub use primary::{AlienPrimaryMeshFrame, AlienPrimaryMeshPose, AlienPrimaryProjectionError};
pub use projection::{
    AlienModelPose, AlienNodePose, AlienProjectedVertex, AlienProjectionError, AlienSceneNode,
    AlienScreenCenter,
};
pub use raster::{
    AlienRasterError, AlienRenderGeometry, AlienRenderTriangle, AlienRenderVertex,
    prepare_render_geometry,
};
pub use resume::{
    AlienResumeCallback, AlienResumeCallbacks, AlienResumeMethodState, AlienResumePairUpdate,
    AlienResumeUpdate, initialize_or_dispatch_resume, update_resume_pair_steering,
};
pub use ring::{
    AlienRingAnimationState, AlienRingCallback, AlienRingCallbacks, AlienRingClearUpdate,
    AlienRingCourseUpdate, AlienRingEntry, AlienRingError, AlienRingFollowerUpdate,
    AlienRingLifecycle, AlienRingNodeState, AlienRingResumeState, AlienRingUpdate,
    AlienWaveSteeringState, begin_resume_clear, capture_resume_state, clear_next_ring_entry,
    restart_initial_course, update_follow_course, update_initial_course, update_or_initialize_ring,
};
pub use scene::{AlienScene, AlienSceneError, AlienSceneFrame};
pub use selection::{
    AlienSelectionError, AlienSelectionUpdate, AlienWaveCallbackUpdate, AlienWaveMotionUpdate,
    AlienWaveReturnUpdate, continue_wave_steering, update_wave_callback, update_wave_camera,
    update_wave_finish, update_wave_motion, update_wave_return, update_wave_selection,
};
pub use slot2::{
    AlienAmerCommonUpdate, AlienAmerFinishUpdate, AlienAmerLateSelectionUpdate,
    AlienAmerReturnUpdate, AlienAmerSelectionUpdate, AlienAmerSteeringUpdate, AlienAmerUpdateHead,
    AlienCroolisCommonDispatch, AlienCroolisFadeUpdate, AlienCroolisResetUpdate,
    AlienCroolisSelectionUpdate, AlienCroolisUpdateHead, AlienScrutActiveResetSetup,
    AlienScrutApproachUpdate, AlienScrutCommonDispatch, AlienScrutDampingUpdate,
    AlienScrutFadeUpdate, AlienScrutFinishUpdate, AlienScrutResetUpdate,
    AlienScrutSelectionBeginUpdate, AlienScrutSelectionInit, AlienScrutSelectionResetUpdate,
    AlienScrutSteeringPrecision, AlienScrutSteeringUpdate, AlienScrutUpdateHead,
    AlienSlot2AnimationState, AlienSlot2Callback, AlienSlot2Callbacks, AlienSlot2Error,
    AlienSlot2NodeState, AlienSlot2SceneState, AlienSlot2Update, AlienUnreferencedSteeringState,
    CROOLIS_SELECTION_RESET_DISTANCE, begin_amer_selection, begin_croolis_fade,
    begin_croolis_selection, begin_scrut_fade, begin_scrut_finish, begin_scrut_selection,
    begin_unreferenced_scrut_active_reset, dispatch_croolis_common, dispatch_scrut_common,
    initialize_or_dispatch_slot2, reset_amer_motion, reset_scrut_selection, restart_amer_update,
    restart_croolis_update, restart_scrut_selection, restart_scrut_update, update_amer_common,
    update_amer_finish, update_amer_head, update_amer_late_selection, update_amer_return,
    update_amer_selection, update_amer_steering, update_croolis_fade, update_croolis_head,
    update_croolis_motion, update_croolis_reset_or_camera, update_croolis_selection,
    update_scrut_fade, update_scrut_finish, update_scrut_head, update_scrut_motion,
    update_scrut_reset_or_camera, update_scrut_selection_approach, update_scrut_selection_begin,
    update_scrut_selection_damping, update_scrut_steering, update_unreferenced_steering,
};
pub use starfield::{
    AlienStar, AlienStarRejections, AlienStarfieldError, AlienStarfieldFrame, STAR_COUNT,
    generate_starfield,
};
pub use wave::{
    AlienWaveError, AlienWaveMethodState, AlienWaveSceneState, AlienWaveSelection, AlienWaveUpdate,
    update_or_initialize_wave,
};
