//! Concrete runtime services assembled for the recovered top-level lifecycle.

use anyhow::{Context, Result, bail};
use sdl3::video::Window;

use crate::native::bloodprg::{BridgeSceneFrame, PbmDecodeResult, StartupPreparationOutcome};

use super::{
    OriginalGameData, OriginalGameRuntime, RuntimeAssetLoadStatus, RuntimeInputHost,
    RuntimePresentationHost, VGA_BIOS_FONT_8X8,
};

const INITIAL_LOGICAL_POINTER: [i16; 2] = [160, 100];

/// Owned flat services that concrete `GameLifecycleHost` methods delegate to.
///
/// This type deliberately exposes only operations backed by translated logic
/// and a real host implementation. Audio, VM coordination, and save handling
/// are added here only when their complete services can be wired without a
/// placeholder path.
pub struct ModernGameServices<'window> {
    runtime: OriginalGameRuntime,
    input: RuntimeInputHost,
    presentation: RuntimePresentationHost<'window>,
    main_viewport_configured: bool,
}

impl<'window> ModernGameServices<'window> {
    /// Allocate flat game state and an artwork-only loading renderer.
    pub fn new(window: &'window Window, data: OriginalGameData) -> Result<Self> {
        let runtime = OriginalGameRuntime::new(data);
        let presentation = RuntimePresentationHost::new_startup(window, &runtime)?;
        Ok(Self {
            runtime,
            input: RuntimeInputHost::new(INITIAL_LOGICAL_POINTER),
            presentation,
            main_viewport_configured: false,
        })
    }

    /// Draw and present `LOADING`, then populate missing writable resources.
    pub fn prepare_startup_resources(&mut self) -> Result<StartupPreparationOutcome> {
        let runtime = &mut self.runtime;
        let presentation = &mut self.presentation;
        runtime.prepare_startup_resources(&VGA_BIOS_FONT_8X8, |frame, palette| {
            presentation.submit_frame(frame, palette)?;
            presentation.present_artwork(&[])
        })
    }

    /// Decode the authored MANU3 overlay exactly once.
    pub fn load_manu3_overlay(&mut self) -> Result<RuntimeAssetLoadStatus> {
        self.runtime.load_manu3()
    }

    /// Recreate GPU resources for bridge rendering and MANU3 composition.
    pub fn initialize_logical_viewport(&mut self) -> Result<()> {
        if self.runtime.manu3().is_none() {
            bail!("MANU3 must be loaded before the main logical viewport");
        }
        self.presentation
            .configure_main_game(&self.runtime)
            .context("configuring main logical viewport")?;
        self.main_viewport_configured = true;
        Ok(())
    }

    /// Decode the complete bridge panorama archive into owned flat storage.
    pub fn open_bridge_panorama(&mut self) -> Result<RuntimeAssetLoadStatus> {
        self.runtime.open_bridge_panorama()
    }

    /// Decode `CHART.FD` and restore it into the current presentation frame.
    pub fn initialize_back_buffer(&mut self) -> Result<PbmDecodeResult> {
        let result = self.runtime.initialize_back_buffer()?;
        self.runtime.restore_back_buffer();
        Ok(result)
    }

    /// Reconfigure the wgpu surface after a nonzero SDL pixel-size event.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.presentation.resize(width, height);
    }

    /// Upload the complete current indexed frame and live VGA palette.
    pub fn submit_indexed_frame(&mut self) -> Result<()> {
        self.presentation.submit_indexed_frame(&self.runtime)
    }

    /// Present current indexed artwork and the optional MANU3 overlay.
    pub fn present_artwork(&mut self) -> Result<()> {
        self.ensure_main_viewport()?;
        let triangles = self
            .runtime
            .manu3()
            .map(|model| model.render_triangles())
            .unwrap_or(&[]);
        self.presentation.present_artwork(triangles)
    }

    /// Present one translated bridge scene frame and optional MANU3 overlay.
    pub fn present_bridge_frame(&mut self, bridge_frame: &BridgeSceneFrame) -> Result<()> {
        self.ensure_main_viewport()?;
        self.presentation
            .present_frame(&self.runtime, Some(bridge_frame))
    }

    /// Core owned game state used by translated script and scene systems.
    pub const fn runtime(&self) -> &OriginalGameRuntime {
        &self.runtime
    }

    /// Mutable core state for translated script and scene updates.
    pub fn runtime_mut(&mut self) -> &mut OriginalGameRuntime {
        &mut self.runtime
    }

    /// SDL input queue, latches, and logical pointer sampler.
    pub const fn input(&self) -> &RuntimeInputHost {
        &self.input
    }

    /// Mutable SDL input service used by the event pump.
    pub fn input_mut(&mut self) -> &mut RuntimeInputHost {
        &mut self.input
    }

    /// Number of frames published by the wgpu presentation service.
    pub const fn presented_frame_count(&self) -> u64 {
        self.presentation.presented_frame_count()
    }

    fn ensure_main_viewport(&self) -> Result<()> {
        if !self.main_viewport_configured {
            bail!("main logical viewport has not been configured");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::runtime::OriginalGameDataPaths;

    static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(u64::MIN);

    struct TemporaryRoot(std::path::PathBuf);

    impl TemporaryRoot {
        fn create() -> Self {
            let sequence = TEMPORARY_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "commander-blood-services-test-{}-{sequence}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for TemporaryRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn real_services_run_the_complete_available_startup_slice() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        if std::env::var_os("WAYLAND_DISPLAY").is_none() && std::env::var_os("DISPLAY").is_none() {
            return;
        }
        let sdl = sdl3::init().unwrap();
        let video = sdl.video().unwrap();
        let window = video
            .window("Commander Blood service test", 640, 480)
            .position_centered()
            .metal_view()
            .build()
            .unwrap();
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let mut services = ModernGameServices::new(&window, data).unwrap();

        let startup = services.prepare_startup_resources().unwrap();
        assert!(startup.write_directory_created);
        assert_eq!(
            services.load_manu3_overlay().unwrap(),
            RuntimeAssetLoadStatus::LoadedNow
        );
        services.initialize_logical_viewport().unwrap();
        assert_eq!(
            services.open_bridge_panorama().unwrap(),
            RuntimeAssetLoadStatus::LoadedNow
        );
        services.initialize_back_buffer().unwrap();
        services.submit_indexed_frame().unwrap();
        services.present_artwork().unwrap();

        assert_eq!(services.presented_frame_count(), 2);
        assert!(
            services
                .runtime()
                .front_buffer()
                .pixels()
                .iter()
                .any(|pixel| *pixel != u8::MIN)
        );
    }
}
