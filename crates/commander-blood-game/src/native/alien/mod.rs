//! Shared native engine used by the AMER, CROOLIS, and SCRUT 3D scenes.

mod behavior;
mod camera;
mod control;
mod faces;
mod palette;
mod primary;
mod projection;
mod raster;
mod ring;
mod scene;
mod starfield;
mod wave;

pub use behavior::{
    AlienBehaviorError, AlienSampleState, adjust_state, anchor_state, apply_sample_delta,
    apply_scaled_sample_delta, bounds_then_wrap, wrap_positions,
};
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
    AlienModelPose, AlienNodePose, AlienProjectedVertex, AlienProjectionError, AlienScreenCenter,
};
pub use raster::{
    AlienRasterError, AlienRenderGeometry, AlienRenderTriangle, AlienRenderVertex,
    prepare_render_geometry,
};
pub use ring::{
    AlienRingAnimationState, AlienRingCallback, AlienRingCallbacks, AlienRingEntry, AlienRingError,
    AlienRingLifecycle, AlienRingNodeState, AlienRingUpdate, update_or_initialize_ring,
};
pub use scene::{AlienScene, AlienSceneError, AlienSceneFrame};
pub use starfield::{
    AlienStar, AlienStarRejections, AlienStarfieldError, AlienStarfieldFrame, STAR_COUNT,
    generate_starfield,
};
pub use wave::{
    AlienWaveError, AlienWaveMethodState, AlienWaveSceneState, AlienWaveSelection, AlienWaveUpdate,
    update_or_initialize_wave,
};
