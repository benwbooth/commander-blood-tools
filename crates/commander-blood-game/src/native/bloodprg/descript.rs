//! Runtime state produced while applying typed DESCRIPT records.

use commander_blood_formats::descript::{
    DescriptBackgroundCommand, DescriptBackgroundSlot, DescriptCaptionCommand, DescriptRecordKind,
    DescriptVideoName,
};

use super::text_handler::TextPresentationState;

/// One background resource retained for modern rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedDescriptBackground {
    source_name: Box<[u8]>,
    encoded_image: Box<[u8]>,
}

impl CachedDescriptBackground {
    /// Return the case-preserving LBM resource name.
    pub fn source_name(&self) -> &[u8] {
        &self.source_name
    }

    /// Return the encoded image bytes loaded from the game data.
    pub fn encoded_image(&self) -> &[u8] {
        &self.encoded_image
    }
}

/// Four owned background images selected by a DESCRIPT location record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescriptBackgroundCache {
    slots: [Option<CachedDescriptBackground>; DescriptBackgroundSlot::COUNT],
}

impl Default for DescriptBackgroundCache {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
        }
    }
}

impl DescriptBackgroundCache {
    /// Return the image currently retained in one typed slot.
    pub fn get(&self, slot: DescriptBackgroundSlot) -> Option<&CachedDescriptBackground> {
        self.slots[slot.index()].as_ref()
    }
}

/// Resource boundary used to load one LBM directly into owned memory.
pub trait DescriptBackgroundSource {
    /// Backend-specific load failure.
    type Error;

    /// Load the complete encoded image named by a DESCRIPT command.
    fn load_background(&mut self, source_name: &[u8]) -> Result<Box<[u8]>, Self::Error>;
}

/// Result of applying one background command to the four-slot cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptBackgroundCacheOutcome {
    /// The requested name matched the beginning of the slot's retained name.
    Hit,
    /// A different image was loaded and replaced the selected slot.
    Loaded {
        /// Number of encoded bytes retained for the renderer.
        encoded_byte_count: usize,
    },
}

/// Cache one DESCRIPT background image using owned bytes instead of DOS temporary files.
///
/// This translates `index_lookup_dca` at BLOODPRG file offset `0x00755E`.
/// The original compared only the requested name's bytes, so a request such as
/// `short` deliberately remains a hit for a retained `shorter.lbm` resource.
pub fn cache_background_image<Source: DescriptBackgroundSource>(
    command: &DescriptBackgroundCommand,
    cache: &mut DescriptBackgroundCache,
    source: &mut Source,
) -> Result<DescriptBackgroundCacheOutcome, Source::Error> {
    let slot = command.slot();
    if cache
        .get(slot)
        .is_some_and(|cached| cached.source_name.starts_with(command.source_name()))
    {
        return Ok(DescriptBackgroundCacheOutcome::Hit);
    }

    let encoded_image = source.load_background(command.source_name())?;
    let encoded_byte_count = encoded_image.len();
    cache.slots[slot.index()] = Some(CachedDescriptBackground {
        source_name: Box::from(command.source_name()),
        encoded_image,
    });
    Ok(DescriptBackgroundCacheOutcome::Loaded { encoded_byte_count })
}

/// Stage one DESCRIPT location or ship caption for progressive subtitle reveal.
///
/// This translates `credit_presenter_b_cryo` at BLOODPRG file offset
/// `0x007612`. The old fixed text buffer becomes owned caption bytes, and the
/// pointer-valued reveal cursor becomes a zero-based byte index.
pub fn stage_descript_caption(
    command: &DescriptCaptionCommand,
    presentation: &mut TextPresentationState,
) {
    presentation.subtitle_text = Box::from(command.text());
    presentation.subtitle_display_active = true;
    presentation.subtitle_reveal_cursor = usize::MIN;
}

/// Video resources selected by one decoded DESCRIPT record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DescriptPresentationAssets {
    location_scene_video: Option<Box<[u8]>>,
    object_scene_video: Option<Box<[u8]>>,
    character_right_scene_video: Option<Box<[u8]>>,
    character_left_scene_video: Option<Box<[u8]>>,
}

impl DescriptPresentationAssets {
    /// Return the location scene's primary HNM resource.
    pub fn location_scene_video(&self) -> Option<&[u8]> {
        self.location_scene_video.as_deref()
    }

    /// Return the inventory or world-object HNM resource.
    pub fn object_scene_video(&self) -> Option<&[u8]> {
        self.object_scene_video.as_deref()
    }

    /// Return the character HNM authored for the right-side view.
    pub fn character_right_scene_video(&self) -> Option<&[u8]> {
        self.character_right_scene_video.as_deref()
    }

    /// Return the character HNM authored for the left-side view.
    pub fn character_left_scene_video(&self) -> Option<&[u8]> {
        self.character_left_scene_video.as_deref()
    }
}

/// Select the primary location scene HNM.
///
/// This translates `byte_parser_copy_20b8_printable` at BLOODPRG file offset
/// `0x007629`.
pub fn select_location_scene_video(
    video: &DescriptVideoName,
    assets: &mut DescriptPresentationAssets,
) {
    assets.location_scene_video = Some(Box::from(video.as_bytes()));
}

/// Select the inventory or world-object scene HNM.
///
/// This translates `byte_parser_copy_24c6_printable` at BLOODPRG file offset
/// `0x00766F`.
pub fn select_object_scene_video(
    video: &DescriptVideoName,
    assets: &mut DescriptPresentationAssets,
) {
    assets.object_scene_video = Some(Box::from(video.as_bytes()));
}

/// Select the character HNM authored for the right-side view.
///
/// This translates `byte_parser_copy_2460_printable` at BLOODPRG file offset
/// `0x0076C0`.
pub fn select_character_right_scene_video(
    video: &DescriptVideoName,
    assets: &mut DescriptPresentationAssets,
) {
    assets.character_right_scene_video = Some(Box::from(video.as_bytes()));
}

/// Select the character HNM authored for the left-side view.
///
/// This translates `byte_parser_copy_247a_printable` at BLOODPRG file offset
/// `0x0076D5`.
pub fn select_character_left_scene_video(
    video: &DescriptVideoName,
    assets: &mut DescriptPresentationAssets,
) {
    assets.character_left_scene_video = Some(Box::from(video.as_bytes()));
}

/// Boundary detected after the current DESCRIPT command stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DescriptRecordBoundary {
    next_record_kind: Option<DescriptRecordKind>,
}

impl DescriptRecordBoundary {
    /// Return whether execution must stop before the next record.
    pub const fn should_stop(self) -> bool {
        self.next_record_kind.is_some()
    }

    /// Return the kind byte that begins the following directory record.
    pub const fn next_record_kind(self) -> Option<DescriptRecordKind> {
        self.next_record_kind
    }

    fn stop_before(&mut self, kind: DescriptRecordKind) {
        self.next_record_kind = Some(kind);
    }
}

/// Stop the current stream before a following Location record.
///
/// This translates `byte_parser_op_01_mark_b16` at BLOODPRG file offset
/// `0x007542`. The native Boolean marker becomes an explicit record kind.
pub fn stop_before_location_record(boundary: &mut DescriptRecordBoundary) {
    boundary.stop_before(DescriptRecordKind::Location);
}

/// Stop the current stream before a following Character record.
///
/// This translates `byte_parser_op_02_mark_b16` at BLOODPRG file offset
/// `0x007549`.
pub fn stop_before_character_record(boundary: &mut DescriptRecordBoundary) {
    boundary.stop_before(DescriptRecordKind::Character);
}

/// Stop the current stream before a following Object record.
///
/// This translates `byte_parser_op_0f_mark_b16` at BLOODPRG file offset
/// `0x007550`.
pub fn stop_before_object_record(boundary: &mut DescriptRecordBoundary) {
    boundary.stop_before(DescriptRecordKind::Object);
}

/// Stop the current stream before a following Sequence record.
///
/// This translates `byte_parser_op_04_mark_b16` at BLOODPRG file offset
/// `0x007557`.
pub fn stop_before_sequence_record(boundary: &mut DescriptRecordBoundary) {
    boundary.stop_before(DescriptRecordKind::Sequence);
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use commander_blood_formats::descript::{
        DescriptBackgroundError, decode_background_command, decode_caption_command,
        decode_video_name,
    };
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 2;
    const BACKGROUND_ORACLE_VECTOR_COUNT: usize = 8;
    const VIDEO_ORACLE_VECTOR_COUNT: usize = 8;

    #[derive(Deserialize)]
    struct StopOracle {
        name: String,
        flag_after: u8,
    }

    #[derive(Deserialize)]
    struct BackgroundOracle {
        name: String,
        slot: u8,
        copied_name: String,
        stopping_byte: u8,
        cache_hit: bool,
        requested_bytes: usize,
        written_bytes: usize,
    }

    #[derive(Deserialize)]
    struct CaptionOracle {
        name: String,
        input_hex: String,
        reveal_active_after: u8,
        reveal_timer_after: usize,
    }

    #[derive(Deserialize)]
    struct VideoOracle {
        name: String,
        input_hex: String,
        copied_hex: String,
        stopping_byte: u8,
    }

    #[derive(Clone, Copy)]
    enum VideoAssetField {
        Location,
        Object,
        CharacterRight,
        CharacterLeft,
    }

    impl VideoAssetField {
        fn selected(self, assets: &DescriptPresentationAssets) -> Option<&[u8]> {
            match self {
                Self::Location => assets.location_scene_video(),
                Self::Object => assets.object_scene_video(),
                Self::CharacterRight => assets.character_right_scene_video(),
                Self::CharacterLeft => assets.character_left_scene_video(),
            }
        }
    }

    #[derive(Default)]
    struct RecordingBackgroundSource {
        payload: Box<[u8]>,
        loaded_names: Vec<Box<[u8]>>,
    }

    impl DescriptBackgroundSource for RecordingBackgroundSource {
        type Error = Infallible;

        fn load_background(&mut self, source_name: &[u8]) -> Result<Box<[u8]>, Self::Error> {
            self.loaded_names.push(Box::from(source_name));
            Ok(self.payload.clone())
        }
    }

    fn background_command(slot: DescriptBackgroundSlot, name: &[u8]) -> DescriptBackgroundCommand {
        DescriptBackgroundCommand::new(slot, Box::from(name))
    }

    fn bytes_from_hex(encoded: &str) -> Box<[u8]> {
        encoded
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    fn assert_stop_handler(
        input: &str,
        expected_kind: DescriptRecordKind,
        handler: fn(&mut DescriptRecordBoundary),
    ) {
        let vectors: Vec<StopOracle> = serde_json::from_str(input).unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut boundary = DescriptRecordBoundary::default();
            match vector.name.as_str() {
                "already_set" => handler(&mut boundary),
                "overwrite_marker" if expected_kind == DescriptRecordKind::Location => {
                    stop_before_sequence_record(&mut boundary);
                }
                "overwrite_marker" => stop_before_location_record(&mut boundary),
                name => panic!("unknown DESCRIPT stop oracle {name}"),
            }

            handler(&mut boundary);
            assert_eq!(vector.flag_after, 1, "{}", vector.name);
            assert!(boundary.should_stop(), "{}", vector.name);
            assert_eq!(
                boundary.next_record_kind(),
                Some(expected_kind),
                "{}",
                vector.name
            );
        }
    }

    fn assert_video_selector(
        input: &str,
        field: VideoAssetField,
        selector: fn(&DescriptVideoName, &mut DescriptPresentationAssets),
    ) {
        let vectors: Vec<VideoOracle> = serde_json::from_str(input).unwrap();
        assert_eq!(vectors.len(), VIDEO_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let input = bytes_from_hex(&vector.input_hex);
            let expected = bytes_from_hex(&vector.copied_hex);
            let (video, tail) = decode_video_name(&input).unwrap();
            assert_eq!(tail, &[vector.stopping_byte], "{}", vector.name);
            assert_eq!(video.as_bytes(), expected.as_ref(), "{}", vector.name);

            let mut assets = DescriptPresentationAssets::default();
            selector(&DescriptVideoName::new(Box::from(*b"old.hnm")), &mut assets);
            selector(&video, &mut assets);
            assert_eq!(
                field.selected(&assets),
                Some(expected.as_ref()),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn location_boundary_matches_original_marker_writes() {
        assert_stop_handler(
            include_str!("../../../../../re/tools/oracle_vectors/func_7542_natural.json"),
            DescriptRecordKind::Location,
            stop_before_location_record,
        );
    }

    #[test]
    fn character_boundary_matches_original_marker_writes() {
        assert_stop_handler(
            include_str!("../../../../../re/tools/oracle_vectors/func_7549_natural.json"),
            DescriptRecordKind::Character,
            stop_before_character_record,
        );
    }

    #[test]
    fn object_boundary_matches_original_marker_writes() {
        assert_stop_handler(
            include_str!("../../../../../re/tools/oracle_vectors/func_7550_natural.json"),
            DescriptRecordKind::Object,
            stop_before_object_record,
        );
    }

    #[test]
    fn sequence_boundary_matches_original_marker_writes() {
        assert_stop_handler(
            include_str!("../../../../../re/tools/oracle_vectors/func_7557_natural.json"),
            DescriptRecordKind::Sequence,
            stop_before_sequence_record,
        );
    }

    #[test]
    fn background_cache_matches_every_original_lookup_vector() {
        let vectors: Vec<BackgroundOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_755e_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), BACKGROUND_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut payload = vec![vector.slot];
            payload.extend_from_slice(vector.copied_name.as_bytes());
            payload.push(vector.stopping_byte);
            let decoded = decode_background_command(&payload);

            if vector.name == "high_stop_decrement_before_sign_extend" {
                assert_eq!(
                    decoded,
                    Err(DescriptBackgroundError::InvalidSlot(vector.slot))
                );
                continue;
            }

            let (command, tail) = decoded.unwrap();
            assert_eq!(tail, &[vector.stopping_byte], "{}", vector.name);
            assert!(
                vector.requested_bytes >= vector.written_bytes,
                "{}",
                vector.name
            );

            let mut cache = DescriptBackgroundCache::default();
            let mut source = RecordingBackgroundSource::default();
            if vector.cache_hit {
                let retained_name: &[u8] = match vector.name.as_str() {
                    "exact_cache_hit" => b"same.lbm",
                    "prefix_cache_hit" => b"shorter.lbm",
                    name => panic!("unknown background cache-hit oracle {name}"),
                };
                source.payload = Box::from(*b"seed");
                cache_background_image(
                    &background_command(command.slot(), retained_name),
                    &mut cache,
                    &mut source,
                )
                .unwrap();
                source.loaded_names.clear();
            } else {
                source.payload = vec![165; vector.written_bytes].into_boxed_slice();
            }

            let outcome = cache_background_image(&command, &mut cache, &mut source).unwrap();
            assert_eq!(
                outcome == DescriptBackgroundCacheOutcome::Hit,
                vector.cache_hit,
                "{}",
                vector.name
            );
            assert_eq!(
                source.loaded_names.len(),
                usize::from(!vector.cache_hit),
                "{}",
                vector.name
            );

            let cached = cache.get(command.slot()).unwrap();
            if !vector.cache_hit {
                assert_eq!(
                    cached.source_name(),
                    vector.copied_name.as_bytes(),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    cached.encoded_image().len(),
                    vector.written_bytes,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn caption_staging_matches_every_original_presenter_vector() {
        let vectors: Vec<CaptionOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_7612_natural.json"
        ))
        .unwrap();

        for vector in vectors {
            let input = bytes_from_hex(&vector.input_hex);
            let (command, tail) = decode_caption_command(&input).unwrap();
            assert!(tail.is_empty(), "{}", vector.name);

            let mut presentation = TextPresentationState {
                subtitle_reveal_cursor: usize::MAX,
                ..TextPresentationState::default()
            };
            stage_descript_caption(&command, &mut presentation);

            assert_eq!(
                presentation.subtitle_text.as_ref(),
                &input[..input.len() - 1],
                "{}",
                vector.name
            );
            assert_eq!(
                presentation.subtitle_display_active,
                vector.reveal_active_after != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                presentation.subtitle_reveal_cursor, vector.reveal_timer_after,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn location_video_selection_matches_every_original_copy_vector() {
        assert_video_selector(
            include_str!("../../../../../re/tools/oracle_vectors/func_7629_natural.json"),
            VideoAssetField::Location,
            select_location_scene_video,
        );
    }

    #[test]
    fn object_video_selection_matches_every_original_copy_vector() {
        assert_video_selector(
            include_str!("../../../../../re/tools/oracle_vectors/func_766f_natural.json"),
            VideoAssetField::Object,
            select_object_scene_video,
        );
    }

    #[test]
    fn character_right_video_selection_matches_every_original_copy_vector() {
        assert_video_selector(
            include_str!("../../../../../re/tools/oracle_vectors/func_76c0_natural.json"),
            VideoAssetField::CharacterRight,
            select_character_right_scene_video,
        );
    }

    #[test]
    fn character_left_video_selection_matches_every_original_copy_vector() {
        assert_video_selector(
            include_str!("../../../../../re/tools/oracle_vectors/func_76d5_natural.json"),
            VideoAssetField::CharacterLeft,
            select_character_left_scene_video,
        );
    }
}
