//! Startup-only decoding of location-panel artwork into immutable RGBA pixels.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use commander_blood_formats::world_art::WorldArtworkLayout;

use crate::assets::OriginalResourceStore;
use crate::native::bloodprg::{
    BridgeSpriteEntity, BridgeSpriteFrameSource, BridgeSpritePosition, IndexedGamePalette,
    OriginalResourceCache, OriginalResourceCatalog, PaletteResourceTarget, ResourceId,
    populate_bridge_sprite_from_cache,
};
use crate::ui::RgbaUiOverlay;

const FRAME_HEADER_BYTES: usize = 8;
const RGBA_COMPONENTS: usize = 4;
const FIXED_POINT_FRACTION_BITS: u32 = 16;

pub(super) struct WorldArtworkAssets(BTreeMap<ResourceId, WorldArtworkImage>);

struct WorldArtworkImage {
    size: [usize; 2],
    pixels: Box<[u8]>,
}

impl WorldArtworkAssets {
    pub(super) fn import(
        store: &OriginalResourceStore,
        catalog: &OriginalResourceCatalog,
        layout: &[WorldArtworkLayout],
        defaults: &IndexedGamePalette,
    ) -> Result<Self> {
        let mut images = BTreeMap::new();
        let mut cache = OriginalResourceCache::new();
        for entry in layout {
            let resource = ResourceId::new(entry.resource_id);
            if images.contains_key(&resource) {
                continue;
            }
            // Each source owns its colors; imports cannot inherit another planet's bank.
            let mut colors = *defaults;
            cache.load_palette_resource(
                store,
                catalog,
                resource,
                PaletteResourceTarget::Cached,
                &mut colors,
            )?;
            let mut entities = [BridgeSpriteEntity::default()];
            if !populate_bridge_sprite_from_cache(
                &cache,
                &mut entities,
                0,
                resource,
                BridgeSpritePosition::default(),
                0,
            )? {
                bail!("world artwork {} has no first frame", resource.value());
            }
            let entity = entities[0];
            let Some(BridgeSpriteFrameSource::CachedResource { byte_offset, .. }) =
                entity.frame.map(|frame| frame.source)
            else {
                bail!("world artwork has no cached frame");
            };
            let size = [
                usize::from(entity.source_extent.width),
                usize::from(entity.source_extent.height),
            ];
            if size.contains(&0) {
                bail!("world artwork {} has an empty frame", resource.value());
            }
            // The C panel selects sprite_draw_scaled_transparent: a raw first frame,
            // with index zero transparent, irrespective of the unscaled blitter flags.
            let start = byte_offset + FRAME_HEADER_BYTES;
            let indexed = cache
                .resolve(resource)
                .context("missing imported world artwork")?
                .get(start..start + size[0] * size[1])
                .context("truncated world artwork pixels")?;
            let pixels = indexed
                .iter()
                .flat_map(|&index| {
                    if index == 0 {
                        [0; RGBA_COMPONENTS]
                    } else {
                        let rgb =
                            colors[usize::from(index)].map(|value| (value << 2) | (value >> 4));
                        [rgb[0], rgb[1], rgb[2], 255]
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();
            images.insert(resource, WorldArtworkImage { size, pixels });
        }
        Ok(Self(images))
    }

    pub(super) fn draw(
        &self,
        overlay: &mut RgbaUiOverlay,
        entity: &BridgeSpriteEntity,
    ) -> Result<()> {
        let Some(BridgeSpriteFrameSource::CachedResource { resource, .. }) =
            entity.frame.map(|frame| frame.source)
        else {
            bail!("location panel has no artwork resource");
        };
        let image = self
            .0
            .get(&resource)
            .context("location panel artwork was not imported")?;
        image.draw(overlay, entity);
        Ok(())
    }
}

impl WorldArtworkImage {
    fn draw(&self, overlay: &mut RgbaUiOverlay, entity: &BridgeSpriteEntity) {
        let [width, height] = [
            usize::from(entity.extent.width),
            usize::from(entity.extent.height),
        ];
        if width == 0 || height == 0 {
            return;
        }
        let origin = [
            i32::from(entity.draw_position.x as i16),
            i32::from(entity.draw_position.y as i16),
        ];
        let Some(clip) = entity.dirty_region else {
            return;
        };
        let steps = [
            (self.size[0] << FIXED_POINT_FRACTION_BITS) / width,
            (self.size[1] << FIXED_POINT_FRACTION_BITS) / height,
        ];
        // Keep C's truncated 16.16 sampling, not floating-point nearest rounding.
        for y in clip.top.max(origin[1])..clip.bottom.min(origin[1] + height as i32) {
            let source_y = ((y - origin[1]) as usize * steps[1]) >> FIXED_POINT_FRACTION_BITS;
            for x in clip.left.max(origin[0])..clip.right.min(origin[0] + width as i32) {
                let source_x = ((x - origin[0]) as usize * steps[0]) >> FIXED_POINT_FRACTION_BITS;
                let offset = (source_y * self.size[0] + source_x) * RGBA_COMPONENTS;
                let color: [u8; RGBA_COMPONENTS] = self.pixels[offset..offset + RGBA_COMPONENTS]
                    .try_into()
                    .unwrap();
                if color[3] != 0 {
                    overlay.fill_rect([x, y], [1, 1], color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::bloodprg::{
        BridgeSpriteExtent, BridgeSpriteRect, blit_scaled_transparent_sprite,
    };
    use crate::runtime::{OriginalGameData, OriginalGameDataPaths, OriginalGameRuntime};

    #[test]
    fn all_world_artwork_matches_native_scaling_without_publishing_source_colors() {
        let paths = match OriginalGameDataPaths::discover(None) {
            Ok(paths) => paths,
            Err(error) => {
                assert!(
                    std::env::var_os("CBLOOD_REQUIRE_ACCURACY_TESTS").is_none(),
                    "{error}"
                );
                return;
            }
        };
        let data = OriginalGameData::load(paths).unwrap();
        let resources = data
            .world_artwork_layout()
            .iter()
            .map(|entry| ResourceId::new(entry.resource_id))
            .collect::<Vec<_>>();
        assert_eq!(resources.len(), 42);
        let mut runtime = OriginalGameRuntime::new(data);
        let mut cache = OriginalResourceCache::new();
        for resource in resources {
            runtime.live_palette_mut().fill([63, 0, 0]);
            runtime.load_cached_palette_sprite(resource).unwrap();
            assert_eq!(*runtime.live_palette(), [[63, 0, 0]; 256]);
            let data = runtime.data();
            let mut colors = *data.default_vga_palette();
            cache
                .load_palette_resource(
                    data.resource_store(),
                    data.resource_catalog(),
                    resource,
                    PaletteResourceTarget::Cached,
                    &mut colors,
                )
                .unwrap();
            let mut entities = [BridgeSpriteEntity::default()];
            assert!(
                populate_bridge_sprite_from_cache(
                    &cache,
                    &mut entities,
                    0,
                    resource,
                    BridgeSpritePosition::default(),
                    0
                )
                .unwrap()
            );
            for (position, extent) in [
                ([0, 0], [1, 1]),
                ([25, 20], [73, 49]),
                ([80, 40], [113, 87]),
                ([-13, -9], [127, 97]),
                ([280, 175], [90, 70]),
            ] {
                let mut entity = entities[0];
                entity.draw_position = BridgeSpritePosition {
                    x: position[0] as u16,
                    y: position[1] as u16,
                };
                entity.extent = BridgeSpriteExtent {
                    width: extent[0],
                    height: extent[1],
                };
                entity.dirty_region = Some(BridgeSpriteRect {
                    left: 0,
                    top: 0,
                    right: 320,
                    bottom: 200,
                });
                let mut reference = vec![0; 320 * 200];
                blit_scaled_transparent_sprite(
                    &entity,
                    cache.resolve(resource).unwrap(),
                    &mut reference,
                )
                .unwrap();
                let mut overlay = RgbaUiOverlay::new(320, 200);
                data.world_artwork_assets
                    .draw(&mut overlay, &entity)
                    .unwrap();
                for (&index, actual) in reference
                    .iter()
                    .zip(overlay.pixels().chunks_exact(RGBA_COMPONENTS))
                {
                    let expected = if index == 0 {
                        [0; RGBA_COMPONENTS]
                    } else {
                        let rgb =
                            colors[usize::from(index)].map(|value| (value << 2) | (value >> 4));
                        [rgb[0], rgb[1], rgb[2], 255]
                    };
                    assert_eq!(
                        actual,
                        expected,
                        "resource {} at {position:?}, {extent:?}",
                        resource.value()
                    );
                }
            }
        }
    }
}
