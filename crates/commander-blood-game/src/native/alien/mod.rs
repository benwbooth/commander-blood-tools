//! Shared native engine used by the AMER, CROOLIS, and SCRUT 3D scenes.

mod camera;
mod control;
mod faces;
mod projection;

pub use camera::{AlienCameraAngles, AlienCameraTransform};
pub use control::{
    AlienCameraControl, AlienCameraStep, AlienInputAction, AlienMouseSample, AlienSpecies,
};
pub use faces::{
    AlienBehindCameraSignal, AlienFaceBucket, AlienFaceDecision, AlienFaceReference,
    AlienFaceSelection, AlienFaceSelectionError, select_faces,
};
pub use projection::{
    AlienModelPose, AlienNodePose, AlienProjectedVertex, AlienProjectionError, AlienScreenCenter,
};
