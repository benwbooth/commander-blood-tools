//! Concrete runtime services assembled for the recovered top-level lifecycle.

use anyhow::{Context, Result, bail};
use commander_blood_formats::bloodprg::decode_bloodprg_bridge_resources;
use sdl3::video::Window;

use crate::native::bloodprg::{
    BridgeScene, BridgeSceneFrame, BridgeSceneInput, PbmDecodeResult, ScriptClock,
    ScriptFrameOutcome, ScriptProfileId, ScriptProfileLoadOutcome, ShipProjectionResources,
    StartupPreparationOutcome,
};
use crate::native::random::BloodPrng;

use super::{
    OriginalGameData, OriginalGameRuntime, RuntimeAssetLoadStatus, RuntimeInputHost,
    RuntimePresentationHost, RuntimeScriptBackend, RuntimeScriptCommand, RuntimeScriptSystem,
    VGA_BIOS_FONT_8X8,
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
    bridge_scene: Option<BridgeScene>,
    bridge_frame: Option<BridgeSceneFrame>,
    scripts: RuntimeScriptSystem,
    main_viewport_configured: bool,
}

impl<'window> ModernGameServices<'window> {
    /// Allocate flat game state and an artwork-only loading renderer.
    pub fn new(
        window: &'window Window,
        data: OriginalGameData,
        script_clock: ScriptClock,
    ) -> Result<Self> {
        let scripts = RuntimeScriptSystem::new(&data, script_clock);
        let runtime = OriginalGameRuntime::new(data);
        let presentation = RuntimePresentationHost::new_startup(window, &runtime)?;
        Ok(Self {
            runtime,
            input: RuntimeInputHost::new(INITIAL_LOGICAL_POINTER),
            presentation,
            bridge_scene: None,
            bridge_frame: None,
            scripts,
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

    /// Construct the live bridge and consume its exact startup PRNG sequence.
    pub fn initialize_bridge_scene(&mut self, packed_clock_seed: u8) -> Result<()> {
        if self.bridge_scene.is_some() {
            bail!("bridge scene is already initialized");
        }
        let resources = decode_bloodprg_bridge_resources(self.runtime.data().executable())
            .context("decoding bridge projection resources")?;
        let panorama = self
            .runtime
            .take_bridge_panorama()
            .context("bridge panorama must be opened before scene initialization")?;
        let mut random = BloodPrng::default();
        random.seed_from_clock_register(packed_clock_seed);
        self.bridge_scene = Some(
            BridgeScene::new(
                panorama,
                ShipProjectionResources::from(resources),
                &mut random,
            )
            .context("constructing live bridge scene")?,
        );
        Ok(())
    }

    /// Decode `CHART.FD` and restore it into the current presentation frame.
    pub fn initialize_back_buffer(&mut self) -> Result<PbmDecodeResult> {
        let result = self.runtime.initialize_back_buffer()?;
        self.runtime.restore_back_buffer();
        Ok(result)
    }

    /// Load one complete BloodScript profile and bind its concrete runtime services.
    pub fn load_script_profile(
        &mut self,
        profile: ScriptProfileId,
    ) -> Result<ScriptProfileLoadOutcome> {
        self.scripts.load_profile(&mut self.runtime, profile)
    }

    /// Execute one complete translated COD/BAS/presentation frame.
    pub fn execute_script_frame(&mut self, enabled: bool) -> Result<ScriptFrameOutcome> {
        self.scripts.execute_frame(&mut self.runtime, enabled)
    }

    /// Borrow the concrete script backend for lifecycle-state updates.
    pub const fn script_backend(&self) -> &RuntimeScriptBackend {
        self.scripts.backend()
    }

    /// Mutably borrow the concrete script backend for lifecycle-state updates.
    pub fn script_backend_mut(&mut self) -> &mut RuntimeScriptBackend {
        self.scripts.backend_mut()
    }

    /// Drain ordered renderer, audio, camera, and HUD commands from BloodScript.
    pub fn take_script_commands(&mut self) -> Vec<RuntimeScriptCommand> {
        self.scripts.take_commands()
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

    /// Advance the translated bridge steering, panorama, and point-cloud frame.
    pub fn render_bridge_frame(&mut self, input: BridgeSceneInput) -> Result<&BridgeSceneFrame> {
        let scene = self
            .bridge_scene
            .as_mut()
            .context("bridge scene has not been initialized")?;
        self.bridge_frame = Some(
            scene
                .render_frame(input)
                .context("rendering bridge scene")?,
        );
        Ok(self
            .bridge_frame
            .as_ref()
            .expect("rendered bridge frame was retained"))
    }

    /// Present one translated bridge scene frame and optional MANU3 overlay.
    pub fn present_bridge_frame(&mut self, bridge_frame: &BridgeSceneFrame) -> Result<()> {
        self.ensure_main_viewport()?;
        self.presentation
            .present_frame(&self.runtime, Some(bridge_frame))
    }

    /// Present the most recently generated bridge frame.
    pub fn present_current_bridge_frame(&mut self) -> Result<()> {
        self.ensure_main_viewport()?;
        let frame = self
            .bridge_frame
            .as_ref()
            .context("no rendered bridge frame is ready")?;
        self.presentation.present_frame(&self.runtime, Some(frame))
    }

    /// Drop the live bridge and its owned panorama during shutdown.
    pub fn close_bridge_scene(&mut self) -> bool {
        self.bridge_frame = None;
        self.bridge_scene.take().is_some()
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

    const TEST_CLOCK_SEED: u8 = 17;
    const TEST_SCRIPT_CLOCK: ScriptClock = ScriptClock {
        hour: 12,
        day: 2,
        month: 1,
    };

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
    #[ignore = "requires an active desktop and serialized SDL/wgpu ownership"]
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
        let mut services = ModernGameServices::new(&window, data, TEST_SCRIPT_CLOCK).unwrap();

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
        services.initialize_bridge_scene(TEST_CLOCK_SEED).unwrap();
        services.initialize_back_buffer().unwrap();
        services
            .load_script_profile(ScriptProfileId::new(u8::MIN).unwrap())
            .unwrap();
        let script = services.execute_script_frame(true).unwrap();
        assert_ne!(
            script.end,
            crate::native::bloodprg::ScriptFrameEnd::ExecutionDisabled
        );
        let bridge_frame = services
            .render_bridge_frame(BridgeSceneInput::default())
            .unwrap();
        assert!(!bridge_frame.starfield.plotted.is_empty());
        services.submit_indexed_frame().unwrap();
        services.present_current_bridge_frame().unwrap();

        assert_eq!(services.presented_frame_count(), 2);
        assert!(services.close_bridge_scene());
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
