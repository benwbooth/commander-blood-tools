//! Runtime bridge panorama loading from a decoded flat archive.

use commander_blood_formats::panorama::{
    BridgeOrbBox, BridgePanoramaArchive, BridgePanoramaError, BridgePanoramaFrameMetadata,
    BridgeStation, PanoramaDecodeMode,
};

use super::IndexedGamePalette;

/// Current eye-orb hit boxes for the four bridge panorama stations.
pub type BridgeStationOrbBoxes = [Option<BridgeOrbBox>; BridgeStation::COUNT];

/// Flat mutable destinations updated by one panorama frame load.
pub struct BridgePanoramaLoadTarget<'a> {
    framebuffer: &'a mut [u8],
    station_orb_boxes: &'a mut BridgeStationOrbBoxes,
    refresh_live_palette: bool,
    panorama_palette: &'a IndexedGamePalette,
    live_palette: &'a mut IndexedGamePalette,
}

impl<'a> BridgePanoramaLoadTarget<'a> {
    /// Group the frame, hit-box, and palette destinations for one load.
    pub fn new(
        framebuffer: &'a mut [u8],
        station_orb_boxes: &'a mut BridgeStationOrbBoxes,
        refresh_live_palette: bool,
        panorama_palette: &'a IndexedGamePalette,
        live_palette: &'a mut IndexedGamePalette,
    ) -> Self {
        Self {
            framebuffer,
            station_orb_boxes,
            refresh_live_palette,
            panorama_palette,
            live_palette,
        }
    }
}

/// Decode and publish one bridge panorama frame.
///
/// This translates `bridge_panorama_frame_load` at BLOODPRG routine offset
/// `0x00981B`. The archive decoder owns all serialized file positions; runtime
/// receives a frame number, flat framebuffer, typed station boxes, and palettes.
/// DOS handles, seek/read calls, chunk pointers, and unchecked station indexing
/// do not exist in this path.
pub fn load_bridge_panorama_frame(
    archive: &BridgePanoramaArchive,
    frame: usize,
    decode_mode: PanoramaDecodeMode,
    target: BridgePanoramaLoadTarget<'_>,
) -> Result<BridgePanoramaFrameMetadata, BridgePanoramaError> {
    let metadata = archive.decode_frame_over(frame, target.framebuffer, decode_mode)?;
    target.station_orb_boxes.fill(None);
    target.station_orb_boxes[metadata.station.index()] = metadata.orb_box;
    if target.refresh_live_palette {
        target.live_palette.copy_from_slice(target.panorama_palette);
    }
    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use commander_blood_formats::panorama::PANORAMA_FRAME_PIXEL_COUNT;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 7;
    const DIRECTORY_ENTRY_SIZE: usize = 8;
    const FRAME_HEADER_SIZE: usize = 10;
    const NATIVE_PALETTE_REFRESH_FLAG: u8 = 1;

    #[derive(Deserialize)]
    struct LoaderOracle {
        name: String,
        frame: usize,
        selected_station_unchecked: u16,
        selected_box: [u8; 8],
        palette_refresh_after_unpack: u8,
        palette_copied: bool,
    }

    #[test]
    fn loader_matches_valid_original_vectors_and_rejects_unchecked_station_overflow() {
        let vectors: Vec<LoaderOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_981b_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let archive = synthetic_archive(&vector);
            let mut framebuffer = vec![case_index as u8; PANORAMA_FRAME_PIXEL_COUNT];
            let framebuffer_before = framebuffer.clone();
            let mut station_orb_boxes = [Some(BridgeOrbBox {
                origin: [u16::MAX; 2],
                size: [u16::MAX; 2],
            }); BridgeStation::COUNT];
            let station_boxes_before = station_orb_boxes;
            let panorama_palette = indexed_palette(case_index, 17);
            let mut live_palette = indexed_palette(case_index, 29);
            let live_palette_before = live_palette;
            let refresh =
                vector.palette_refresh_after_unpack & NATIVE_PALETTE_REFRESH_FLAG != u8::MIN;

            let result = load_bridge_panorama_frame(
                &archive,
                vector.frame,
                PanoramaDecodeMode::Opaque,
                BridgePanoramaLoadTarget::new(
                    &mut framebuffer,
                    &mut station_orb_boxes,
                    refresh,
                    &panorama_palette,
                    &mut live_palette,
                ),
            );

            if vector.selected_station_unchecked >= BridgeStation::COUNT as u16 {
                assert_eq!(
                    result,
                    Err(BridgePanoramaError::InvalidStation(
                        vector.selected_station_unchecked
                    )),
                    "{}",
                    vector.name
                );
                assert_eq!(framebuffer, framebuffer_before, "{}", vector.name);
                assert_eq!(station_orb_boxes, station_boxes_before, "{}", vector.name);
                assert_eq!(live_palette, live_palette_before, "{}", vector.name);
                continue;
            }

            let metadata = result.unwrap_or_else(|error| panic!("{}: {error}", vector.name));
            let expected_box = BridgeOrbBox {
                origin: [word(&vector.selected_box, 0), word(&vector.selected_box, 2)],
                size: [word(&vector.selected_box, 4), word(&vector.selected_box, 6)],
            };
            assert_eq!(metadata.orb_box, Some(expected_box), "{}", vector.name);
            assert_eq!(
                station_orb_boxes[metadata.station.index()],
                Some(expected_box),
                "{}",
                vector.name
            );
            assert_eq!(
                station_orb_boxes.iter().flatten().count(),
                1,
                "{}",
                vector.name
            );
            assert_eq!(refresh, vector.palette_copied, "{}", vector.name);
            assert_eq!(
                live_palette,
                if refresh {
                    panorama_palette
                } else {
                    live_palette_before
                },
                "{}",
                vector.name
            );
        }
    }

    fn synthetic_archive(vector: &LoaderOracle) -> BridgePanoramaArchive {
        let frame_count = vector.frame + 1;
        let directory_size = frame_count * DIRECTORY_ENTRY_SIZE;
        let stream = opaque_stream();
        let chunk_size = FRAME_HEADER_SIZE + stream.len();
        let mut data = vec![u8::MIN; directory_size + chunk_size];
        for entry in data[..directory_size].chunks_exact_mut(DIRECTORY_ENTRY_SIZE) {
            entry[..4].copy_from_slice(&(directory_size as u32).to_le_bytes());
            entry[4..].copy_from_slice(&(chunk_size as u32).to_le_bytes());
        }
        let chunk = &mut data[directory_size..];
        chunk[..8].copy_from_slice(&vector.selected_box);
        chunk[8..10].copy_from_slice(&vector.selected_station_unchecked.to_le_bytes());
        chunk[FRAME_HEADER_SIZE..].copy_from_slice(&stream);
        BridgePanoramaArchive::decode(data.into_boxed_slice()).unwrap()
    }

    fn opaque_stream() -> Vec<u8> {
        const MAX_REPEAT_COUNT: usize = 129;
        let mut stream = Vec::new();
        let mut remaining = PANORAMA_FRAME_PIXEL_COUNT;
        while remaining != usize::MIN {
            let count = remaining.min(MAX_REPEAT_COUNT);
            stream.push((1_i16 - count as i16) as u8);
            stream.push((remaining / MAX_REPEAT_COUNT) as u8);
            remaining -= count;
        }
        stream
    }

    fn indexed_palette(case_index: usize, multiplier: usize) -> IndexedGamePalette {
        std::array::from_fn(|color| {
            std::array::from_fn(|component| {
                (color * multiplier + component * 31 + case_index * 37) as u8
            })
        })
    }

    fn word(bytes: &[u8; 8], start: usize) -> u16 {
        u16::from_le_bytes([bytes[start], bytes[start + 1]])
    }
}
