//! Concrete flat runtime host for loading-screen and writable-data preparation.

use anyhow::{Context, Result};
use commander_blood_formats::archive::BloodResourceName;

use crate::native::bloodprg::{
    BiosFont8x8, FontPoint, IndexedGamePalette, StartupLoadingText, StartupPreparationHost,
    StartupPreparationOutcome, StartupWritableResourceId, draw_bios_font_text,
    prepare_startup_writable_resources,
};

use super::{IndexedFramebuffer, OriginalGameRuntime};

struct RuntimeStartupHost<'runtime, PresentLoadingFrame> {
    runtime: &'runtime mut OriginalGameRuntime,
    bios_font: &'runtime BiosFont8x8,
    present_loading_frame: PresentLoadingFrame,
}

impl<PresentLoadingFrame> StartupPreparationHost for RuntimeStartupHost<'_, PresentLoadingFrame>
where
    PresentLoadingFrame: FnMut(&IndexedFramebuffer, &IndexedGamePalette) -> Result<()>,
{
    type Error = anyhow::Error;

    fn publish_loading_palette(&mut self, palette: &IndexedGamePalette) -> Result<()> {
        *self.runtime.live_palette_mut() = *palette;
        Ok(())
    }

    fn clear_loading_frame(&mut self, color: u8) -> Result<()> {
        self.runtime.front_buffer_mut().clear(color);
        Ok(())
    }

    fn draw_loading_text(&mut self, text: StartupLoadingText) -> Result<()> {
        let character_limit = u8::try_from(text.byte_limit)
            .context("loading-screen character limit exceeds the original byte field")?;
        draw_bios_font_text(
            self.runtime.front_buffer_mut().pixels_mut(),
            self.bios_font,
            text.text,
            FontPoint {
                x: i32::from(text.position[0]),
                y: i32::from(text.position[1]),
            },
            text.color,
            character_limit,
        )
        .context("drawing the original LOADING label")?;
        Ok(())
    }

    fn present_loading_frame(&mut self) -> Result<()> {
        (self.present_loading_frame)(self.runtime.front_buffer(), self.runtime.live_palette())
    }

    fn ensure_write_directory(&mut self) -> Result<bool> {
        let root = self.runtime.data().resource_store().writable_root();
        std::fs::create_dir_all(root)
            .with_context(|| format!("creating writable game-data root {}", root.display()))?;
        Ok(root.is_dir())
    }

    fn writable_resource_exists(
        &mut self,
        _resource: StartupWritableResourceId,
        name: &BloodResourceName,
    ) -> Result<bool> {
        self.runtime
            .data()
            .resource_store()
            .writable_resource_exists(name)
    }

    fn copy_resource_to_writable(
        &mut self,
        resource: StartupWritableResourceId,
        name: &BloodResourceName,
    ) -> Result<()> {
        self.runtime
            .data()
            .resource_store()
            .copy_to_loose(name, name)
            .with_context(|| {
                format!(
                    "copying startup resource {} ({})",
                    resource.index(),
                    resource_name(name)
                )
            })?;
        Ok(())
    }
}

impl OriginalGameRuntime {
    /// Draw and publish the authentic loading frame, then populate the writable data root.
    pub fn prepare_startup_resources<PresentLoadingFrame>(
        &mut self,
        bios_font: &BiosFont8x8,
        present_loading_frame: PresentLoadingFrame,
    ) -> Result<StartupPreparationOutcome>
    where
        PresentLoadingFrame: FnMut(&IndexedFramebuffer, &IndexedGamePalette) -> Result<()>,
    {
        let catalog = self.data().writable_resource_catalog().clone();
        let loading_palette = *self.live_palette();
        let mut host = RuntimeStartupHost {
            runtime: self,
            bios_font,
            present_loading_frame,
        };
        prepare_startup_writable_resources(&catalog, &loading_palette, &mut host)
            .context("preparing original startup resources")
    }
}

fn resource_name(name: &BloodResourceName) -> String {
    String::from_utf8_lossy(name.as_bytes()).into_owned()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::super::{OriginalGameData, OriginalGameDataPaths, VGA_BIOS_FONT_8X8};
    use super::*;

    const LOADING_TEXT_PALETTE_INDEX: u8 = 239;
    static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(u64::MIN);

    struct TemporaryRoot(std::path::PathBuf);

    impl TemporaryRoot {
        fn create() -> Self {
            let sequence = TEMPORARY_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "commander-blood-startup-test-{}-{sequence}",
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
    fn real_startup_draws_loading_and_copies_every_unique_authored_resource() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        let writable_root = TemporaryRoot::create();
        let data = OriginalGameData::load_with_writable_root(paths, &writable_root.0).unwrap();
        let unique_names: BTreeSet<_> = data
            .writable_resource_catalog()
            .iter()
            .map(|(_resource, name)| name.clone())
            .collect();
        let mut runtime = OriginalGameRuntime::new(data);
        let mut presented_loading_frame = None;
        let writable_path = writable_root.0.clone();

        let outcome = runtime
            .prepare_startup_resources(&VGA_BIOS_FONT_8X8, |frame, palette| {
                assert!(!writable_path.exists());
                presented_loading_frame = Some((frame.clone(), *palette));
                Ok(())
            })
            .unwrap();

        assert!(outcome.write_directory_created);
        assert_eq!(
            outcome.probed_resources,
            crate::native::bloodprg::STARTUP_WRITABLE_RESOURCE_COUNT
        );
        assert_eq!(outcome.copied_resources.len(), unique_names.len());
        let (loading_frame, loading_palette) = presented_loading_frame.unwrap();
        assert_eq!(loading_palette, *runtime.data().default_vga_palette());
        assert!(loading_frame.pixels().contains(&LOADING_TEXT_PALETTE_INDEX));

        for name in unique_names {
            assert!(
                runtime
                    .data()
                    .resource_store()
                    .writable_resource_exists(&name)
                    .unwrap(),
                "{}",
                resource_name(&name)
            );
        }

        let second_outcome = runtime
            .prepare_startup_resources(&VGA_BIOS_FONT_8X8, |_frame, _palette| Ok(()))
            .unwrap();
        assert!(second_outcome.copied_resources.is_empty());
    }
}
