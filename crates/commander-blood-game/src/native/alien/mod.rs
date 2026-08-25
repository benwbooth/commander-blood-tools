//! Shared native engine used by the AMER, CROOLIS, and SCRUT 3D scenes.

mod camera;
mod control;
mod projection;

pub use camera::{AlienCameraAngles, AlienCameraTransform};
pub use control::{
    AlienCameraControl, AlienCameraStep, AlienInputAction, AlienMouseSample, AlienSpecies,
};
pub use projection::{
    AlienModelPose, AlienNodePose, AlienProjectedVertex, AlienProjectionError, AlienScreenCenter,
};
