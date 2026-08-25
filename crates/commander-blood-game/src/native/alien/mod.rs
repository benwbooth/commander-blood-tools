//! Shared native engine used by the AMER, CROOLIS, and SCRUT 3D scenes.

mod camera;
mod control;

pub use camera::{AlienCameraAngles, AlienCameraTransform};
pub use control::{
    AlienCameraControl, AlienCameraStep, AlienInputAction, AlienMouseSample, AlienSpecies,
};
