//! Modern Commander Blood game port.
//!
//! Native game behavior is translated from the recovered C sources and checked
//! against original-binary oracle vectors. Host services and presentation use
//! modern platform APIs; they do not emulate DOS registers or segmented memory.
//! Source-file offsets are consumed by format decoders and never become runtime
//! addresses. Runtime relationships use typed ownership, indices, and slices.

#![deny(missing_docs)]

mod alien_render;
pub mod app;
mod asset_import;
pub mod assets;
mod bridge_render;
pub mod game;
mod media_import;
pub mod native;
pub mod render;
pub mod runtime;
mod script_rebuild;
mod ui;
mod video_import;
mod window_icon;

#[cfg(test)]
mod gpu_test {
    use std::sync::{Mutex, MutexGuard};

    static GPU: Mutex<()> = Mutex::new(());

    // The host Vulkan loader can unload drivers during another test's GPU use.
    // Hold this guard until all device/queue/resource destructors have finished.
    pub(crate) fn lock() -> MutexGuard<'static, ()> {
        GPU.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
