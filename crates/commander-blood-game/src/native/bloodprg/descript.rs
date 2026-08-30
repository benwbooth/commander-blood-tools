//! Runtime state produced while applying typed DESCRIPT records.

use commander_blood_formats::descript::{
    DescriptBackgroundCommand, DescriptBackgroundSlot, DescriptCaptionCommand, DescriptIdleClip,
    DescriptLocationLayout, DescriptMusicName, DescriptRecordKind, DescriptSequenceSubtitle,
    DescriptSoundBankName, DescriptSpriteName, DescriptTalkClip, DescriptVideoName,
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

    /// Release every background retained by the native four-slot cache.
    pub fn clear(&mut self) {
        self.slots.fill(None);
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
    presentation.subtitle_reveal_cursor = None;
}

/// Video resources selected by one decoded DESCRIPT record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DescriptPresentationAssets {
    location_scene_video: Option<Box<[u8]>>,
    object_scene_video: Option<Box<[u8]>>,
    character_right_scene_video: Option<Box<[u8]>>,
    character_left_scene_video: Option<Box<[u8]>>,
    sound_bank: Option<Box<[u8]>>,
    talk_clips: Vec<DescriptTalkClip>,
    location_scene_top_row: Option<u16>,
    idle_clip: Option<DescriptIdleClip>,
    encoded_idle_video: Option<Box<[u8]>>,
    sequence_videos: Vec<DescriptVideoName>,
    sequence_subtitles: Vec<DescriptSequenceSubtitle>,
    character_sprite: Option<DescriptSpriteName>,
    music: Option<DescriptMusicName>,
}

impl DescriptPresentationAssets {
    pub(crate) fn begin_record_application(&mut self) {
        self.location_scene_video = None;
        self.object_scene_video = None;
        self.character_right_scene_video = None;
        self.character_left_scene_video = None;
        self.sound_bank = None;
        self.talk_clips.clear();
        self.idle_clip = None;
        self.sequence_videos.clear();
        self.sequence_subtitles.clear();
        self.character_sprite = None;
    }

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

    /// Return the character chatter and reaction SND bank.
    pub fn sound_bank(&self) -> Option<&[u8]> {
        self.sound_bank.as_deref()
    }

    /// Return character talk animations in authored playback-table order.
    pub fn talk_clips(&self) -> &[DescriptTalkClip] {
        &self.talk_clips
    }

    /// Return the first display row occupied by the location scene video.
    pub const fn location_scene_top_row(&self) -> Option<u16> {
        self.location_scene_top_row
    }

    /// Return the character idle animation selected by the record.
    pub fn idle_clip(&self) -> Option<&DescriptIdleClip> {
        self.idle_clip.as_ref()
    }

    /// Return the encoded idle HNM loaded for the modern renderer.
    pub fn encoded_idle_video(&self) -> Option<&[u8]> {
        self.encoded_idle_video.as_deref()
    }

    /// Return standalone sequence videos in authored playback order.
    pub fn sequence_videos(&self) -> &[DescriptVideoName] {
        &self.sequence_videos
    }

    /// Return centered sequence subtitles in authored threshold order.
    pub fn sequence_subtitles(&self) -> &[DescriptSequenceSubtitle] {
        &self.sequence_subtitles
    }

    /// Return the character portrait sprite selected by this record.
    pub fn character_sprite(&self) -> Option<&DescriptSpriteName> {
        self.character_sprite.as_ref()
    }

    /// Return the current normalized background-music VOC.
    pub fn music(&self) -> Option<&DescriptMusicName> {
        self.music.as_ref()
    }
}

/// Audio backend used to load a selected DESCRIPT SND bank.
pub trait DescriptSoundBankLoader {
    /// Backend-specific load failure.
    type Error;

    /// Load one logical SND bank by its case-preserving resource name.
    fn load_sound_bank(&mut self, bank_name: &[u8]) -> Result<(), Self::Error>;
}

/// Resource backend used to load an idle HNM into owned memory.
pub trait DescriptIdleClipSource {
    /// Backend-specific load failure.
    type Error;

    /// Load the complete encoded idle animation.
    fn load_idle_clip(&mut self, video_name: &[u8]) -> Result<Box<[u8]>, Self::Error>;
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

/// Select and, when no presentation is active, load a character SND bank.
///
/// This translates `byte_parser_snd_bank_name_load` at BLOODPRG file offset
/// `0x00763E`. The original loader's mode one and `sn/` path prefix are backend
/// details rather than runtime game state.
pub fn load_descript_sound_bank<Loader: DescriptSoundBankLoader>(
    bank: &DescriptSoundBankName,
    presentation_active: bool,
    assets: &mut DescriptPresentationAssets,
    loader: &mut Loader,
) -> Result<bool, Loader::Error> {
    assets.sound_bank = Some(Box::from(bank.as_bytes()));
    if presentation_active {
        return Ok(false);
    }

    loader.load_sound_bank(bank.as_bytes())?;
    Ok(true)
}

/// Append one character talk animation and its typed background selection.
///
/// This translates `dlg_line_asset_table_fill` at BLOODPRG file offset
/// `0x007684`. Owned vector entries replace the native parallel cursor tables.
pub fn append_descript_talk_clip(clip: &DescriptTalkClip, assets: &mut DescriptPresentationAssets) {
    assets.talk_clips.push(clip.clone());
}

/// Select the vertical placement of a location scene video.
///
/// This translates `byte_parser_store_word_1fa5` at BLOODPRG file offset
/// `0x0076BA`. All 64 shipped location records select row 35.
pub fn set_location_scene_top_row(
    layout: DescriptLocationLayout,
    assets: &mut DescriptPresentationAssets,
) {
    assets.location_scene_top_row = Some(layout.top_row());
}

/// Select and, when no presentation is active, load a character idle animation.
///
/// This translates `index_lookup_1fd7` at BLOODPRG file offset `0x0076EA`.
/// Owned bytes replace both original EMS and XMS cache paths.
pub fn load_descript_idle_clip<Source: DescriptIdleClipSource>(
    clip: &DescriptIdleClip,
    presentation_active: bool,
    assets: &mut DescriptPresentationAssets,
    source: &mut Source,
) -> Result<bool, Source::Error> {
    assets.idle_clip = Some(clip.clone());
    if presentation_active {
        return Ok(false);
    }

    assets.encoded_idle_video = Some(source.load_idle_clip(clip.video().as_bytes())?);
    Ok(true)
}

/// Append one standalone sequence video in authored playback order.
///
/// This translates `byte_parser_copy_131a_entry` at BLOODPRG file offset
/// `0x007754`. The owned vector replaces the native banked table of fixed-size
/// name slots and its wrapping byte count.
pub fn append_descript_sequence_video(
    video: &DescriptVideoName,
    assets: &mut DescriptPresentationAssets,
) {
    assets.sequence_videos.push(video.clone());
}

/// Append one frame-thresholded centered subtitle in authored order.
///
/// This translates `byte_parser_stream_0f18_append` at BLOODPRG file offset
/// `0x007776`. Typed owned cues replace the packed native byte stream, its
/// mutable cursor, and its trailing negative-threshold sentinel.
pub fn append_descript_sequence_subtitle(
    subtitle: &DescriptSequenceSubtitle,
    assets: &mut DescriptPresentationAssets,
) {
    assets.sequence_subtitles.push(subtitle.clone());
}

/// Select the character portrait sprite used by the name-area presentation.
///
/// This translates `fs_name_area_read` at BLOODPRG file offset `0x007788`.
/// Command presence in this typed option replaces both the FS-backed filename
/// area and its separate dirty flag.
pub fn select_descript_character_sprite(
    sprite: &DescriptSpriteName,
    assets: &mut DescriptPresentationAssets,
) {
    assets.character_sprite = Some(sprite.clone());
}

/// Result of selecting one DESCRIPT background-music resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptMusicSelectionOutcome {
    /// The normalized name differs within the newly authored name's length.
    Changed,
    /// The current name begins with every byte of the newly authored name.
    Reused,
}

/// Select a normalized background-music VOC and report whether it changed.
///
/// This translates `music_voc_name_patcher` at BLOODPRG file offset `0x0077A9`.
/// The original prefix-only comparison is retained, while the path template and
/// its accumulated changed/unchanged flag bytes become a typed outcome.
pub fn select_descript_music(
    music: &DescriptMusicName,
    assets: &mut DescriptPresentationAssets,
) -> DescriptMusicSelectionOutcome {
    let changed = !assets
        .music
        .as_ref()
        .map_or(&[][..], DescriptMusicName::as_bytes)
        .starts_with(music.as_bytes());
    assets.music = Some(music.clone());

    if changed {
        DescriptMusicSelectionOutcome::Changed
    } else {
        DescriptMusicSelectionOutcome::Reused
    }
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
        DescriptBackgroundError, DescriptCharacterBackground, DescriptIdleClipError,
        DescriptTalkClipError, decode_background_command, decode_caption_command, decode_idle_clip,
        decode_location_layout, decode_music_name, decode_sequence_subtitle,
        decode_sound_bank_name, decode_sprite_name, decode_talk_clip, decode_video_name,
    };
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 2;
    const BACKGROUND_ORACLE_VECTOR_COUNT: usize = 8;
    const INVALID_TALK_BACKGROUND_HIGH: u8 = 128;
    const INVALID_TALK_BACKGROUND_ZERO: u8 = 0;
    const IDLE_CLIP_ORACLE_VECTOR_COUNT: usize = 10;
    const LOCATION_LAYOUT_ORACLE_VECTOR_COUNT: usize = 8;
    const PRESENTATION_ACTIVE_BIT: u16 = 1;
    const SOUND_BANK_ORACLE_VECTOR_COUNT: usize = 8;
    const TALK_CLIP_ORACLE_VECTOR_COUNT: usize = 12;
    const SEQUENCE_VIDEO_ORACLE_VECTOR_COUNT: usize = 8;
    const SEQUENCE_SUBTITLE_ORACLE_VECTOR_COUNT: usize = 7;
    const SPRITE_ORACLE_VECTOR_COUNT: usize = 8;
    const MUSIC_ORACLE_VECTOR_COUNT: usize = 11;
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

    #[derive(Deserialize)]
    struct SoundBankOracle {
        name: String,
        input_hex: String,
        copied_hex: String,
        stopping_byte: u8,
        ui_state: u16,
        loader_called: bool,
    }

    #[derive(Deserialize)]
    struct TalkClipOracle {
        name: String,
        asset_id: u8,
        copied_hex: String,
        stopping_byte: u8,
    }

    #[derive(Deserialize)]
    struct LocationLayoutOracle {
        name: String,
        operand: u16,
        destination_after: u16,
    }

    #[derive(Deserialize)]
    struct IdleClipOracle {
        name: String,
        asset_id: u8,
        copied_hex: String,
        stopping_byte: u8,
        ui_state: u16,
        helper_called: Option<String>,
    }

    #[derive(Deserialize)]
    struct SequenceSubtitleOracle {
        name: String,
        leading_word: u16,
        suffix_hex: String,
        copied_hex: String,
    }

    #[derive(Deserialize)]
    struct MusicOracle {
        name: String,
        input_hex: String,
        transformed_hex: String,
        stopping_byte: u8,
        changed_before: u8,
        changed_after: u8,
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

    #[derive(Default)]
    struct RecordingSoundBankLoader {
        loaded_banks: Vec<Box<[u8]>>,
    }

    #[derive(Default)]
    struct RecordingIdleClipSource {
        payload: Box<[u8]>,
        loaded_names: Vec<Box<[u8]>>,
    }

    impl DescriptIdleClipSource for RecordingIdleClipSource {
        type Error = Infallible;

        fn load_idle_clip(&mut self, video_name: &[u8]) -> Result<Box<[u8]>, Self::Error> {
            self.loaded_names.push(Box::from(video_name));
            Ok(self.payload.clone())
        }
    }

    impl DescriptSoundBankLoader for RecordingSoundBankLoader {
        type Error = Infallible;

        fn load_sound_bank(&mut self, bank_name: &[u8]) -> Result<(), Self::Error> {
            self.loaded_banks.push(Box::from(bank_name));
            Ok(())
        }
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
    fn background_cache_clear_releases_every_owned_slot() {
        let mut cache = DescriptBackgroundCache::default();
        let mut source = RecordingBackgroundSource {
            payload: Box::from(*b"encoded"),
            ..RecordingBackgroundSource::default()
        };

        for encoded_slot in 1..=DescriptBackgroundSlot::COUNT as u8 {
            let slot = DescriptBackgroundSlot::decode(encoded_slot).unwrap();
            let name = format!("slot{encoded_slot}.lbm");
            cache_background_image(
                &background_command(slot, name.as_bytes()),
                &mut cache,
                &mut source,
            )
            .unwrap();
            assert!(cache.get(slot).is_some());
        }

        cache.clear();

        for encoded_slot in 1..=DescriptBackgroundSlot::COUNT as u8 {
            let slot = DescriptBackgroundSlot::decode(encoded_slot).unwrap();
            assert!(cache.get(slot).is_none());
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
                subtitle_reveal_cursor: Some(usize::MAX),
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
                presentation.subtitle_reveal_cursor,
                (vector.reveal_timer_after != usize::MIN).then_some(vector.reveal_timer_after),
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

    #[test]
    fn sound_bank_loading_matches_every_original_gate_vector() {
        let vectors: Vec<SoundBankOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_763e_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), SOUND_BANK_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let input = bytes_from_hex(&vector.input_hex);
            let expected = bytes_from_hex(&vector.copied_hex);
            let (bank, tail) = decode_sound_bank_name(&input).unwrap();
            assert_eq!(tail, &[vector.stopping_byte], "{}", vector.name);
            assert_eq!(bank.as_bytes(), expected.as_ref(), "{}", vector.name);

            let mut assets = DescriptPresentationAssets::default();
            let mut loader = RecordingSoundBankLoader::default();
            let loaded = load_descript_sound_bank(
                &bank,
                vector.ui_state & PRESENTATION_ACTIVE_BIT != u16::MIN,
                &mut assets,
                &mut loader,
            )
            .unwrap();

            assert_eq!(loaded, vector.loader_called, "{}", vector.name);
            assert_eq!(
                assets.sound_bank(),
                Some(expected.as_ref()),
                "{}",
                vector.name
            );
            assert_eq!(
                loader.loaded_banks.len(),
                usize::from(vector.loader_called),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn talk_clip_table_matches_every_original_fill_vector() {
        let vectors: Vec<TalkClipOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_7684_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), TALK_CLIP_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let expected_video = bytes_from_hex(&vector.copied_hex);
            let mut payload = vec![vector.asset_id];
            payload.extend_from_slice(&expected_video);
            payload.push(vector.stopping_byte);
            let decoded = decode_talk_clip(&payload);

            if matches!(
                vector.asset_id,
                INVALID_TALK_BACKGROUND_ZERO | INVALID_TALK_BACKGROUND_HIGH
            ) {
                assert_eq!(
                    decoded,
                    Err(DescriptTalkClipError::InvalidBackground(vector.asset_id)),
                    "{}",
                    vector.name
                );
                continue;
            }

            let (clip, tail) = decoded.unwrap();
            assert_eq!(tail, &[vector.stopping_byte], "{}", vector.name);
            assert_eq!(
                clip.video().as_bytes(),
                expected_video.as_ref(),
                "{}",
                vector.name
            );
            if vector.asset_id == u8::MAX {
                assert_eq!(
                    clip.background(),
                    DescriptCharacterBackground::None,
                    "{}",
                    vector.name
                );
            } else {
                assert_eq!(
                    clip.background(),
                    DescriptCharacterBackground::Cached(
                        DescriptBackgroundSlot::decode(vector.asset_id).unwrap()
                    ),
                    "{}",
                    vector.name
                );
            }

            let mut assets = DescriptPresentationAssets::default();
            append_descript_talk_clip(&clip, &mut assets);
            assert_eq!(assets.talk_clips(), &[clip], "{}", vector.name);
        }
    }

    #[test]
    fn location_scene_top_row_matches_every_original_store_vector() {
        let vectors: Vec<LocationLayoutOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_76ba_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), LOCATION_LAYOUT_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let encoded = vector.operand.to_le_bytes();
            let (layout, tail) = decode_location_layout(&encoded).unwrap();
            assert!(tail.is_empty(), "{}", vector.name);
            assert_eq!(
                layout.top_row(),
                vector.destination_after,
                "{}",
                vector.name
            );

            let mut assets = DescriptPresentationAssets::default();
            set_location_scene_top_row(layout, &mut assets);
            assert_eq!(
                assets.location_scene_top_row(),
                Some(vector.destination_after),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn idle_clip_loading_matches_every_original_valid_vector() {
        let vectors: Vec<IdleClipOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_76ea_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), IDLE_CLIP_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let expected_video = bytes_from_hex(&vector.copied_hex);
            let mut payload = vec![vector.asset_id];
            payload.extend_from_slice(&expected_video);
            payload.push(vector.stopping_byte);
            let decoded = decode_idle_clip(&payload);

            if matches!(
                vector.asset_id,
                INVALID_TALK_BACKGROUND_ZERO | INVALID_TALK_BACKGROUND_HIGH
            ) {
                assert_eq!(
                    decoded,
                    Err(DescriptIdleClipError::InvalidBackground(vector.asset_id)),
                    "{}",
                    vector.name
                );
                continue;
            }

            let (clip, tail) = decoded.unwrap();
            assert_eq!(tail, &[vector.stopping_byte], "{}", vector.name);
            let presentation_active = vector.ui_state & PRESENTATION_ACTIVE_BIT != u16::MIN;
            assert_eq!(
                vector.helper_called.is_some(),
                !presentation_active,
                "{}",
                vector.name
            );

            let mut assets = DescriptPresentationAssets::default();
            let mut source = RecordingIdleClipSource {
                payload: vec![165; expected_video.len() + 1].into_boxed_slice(),
                ..RecordingIdleClipSource::default()
            };
            let loaded =
                load_descript_idle_clip(&clip, presentation_active, &mut assets, &mut source)
                    .unwrap();

            assert_eq!(loaded, !presentation_active, "{}", vector.name);
            assert_eq!(assets.idle_clip(), Some(&clip), "{}", vector.name);
            assert_eq!(
                source.loaded_names.len(),
                usize::from(loaded),
                "{}",
                vector.name
            );
            assert_eq!(
                assets.encoded_idle_video().is_some(),
                loaded,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn ui_active_idle_lookup_preserves_the_existing_banked_payload() {
        let (first, _) =
            decode_idle_clip(&[1, b'f', b'i', b'r', b's', b't', b'.', b'h', b'n', b'm', 0])
                .unwrap();
        let (second, _) = decode_idle_clip(&[
            2, b's', b'e', b'c', b'o', b'n', b'd', b'.', b'h', b'n', b'm', 0,
        ])
        .unwrap();
        let retained_payload: Box<[u8]> = Box::from(*b"retained idle video");
        let mut assets = DescriptPresentationAssets::default();
        let mut source = RecordingIdleClipSource {
            payload: retained_payload.clone(),
            ..RecordingIdleClipSource::default()
        };

        assert!(load_descript_idle_clip(&first, false, &mut assets, &mut source).unwrap());
        assets.begin_record_application();
        assert!(!load_descript_idle_clip(&second, true, &mut assets, &mut source).unwrap());

        assert_eq!(assets.idle_clip(), Some(&second));
        assert_eq!(assets.encoded_idle_video(), Some(retained_payload.as_ref()));
        assert_eq!(source.loaded_names.len(), 1);
    }

    #[test]
    fn sequence_video_playlist_matches_every_original_append_vector() {
        let vectors: Vec<VideoOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_7754_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), SEQUENCE_VIDEO_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let input = bytes_from_hex(&vector.input_hex);
            let expected = bytes_from_hex(&vector.copied_hex);
            let (video, tail) = decode_video_name(&input).unwrap();
            assert_eq!(tail, &[vector.stopping_byte], "{}", vector.name);
            assert_eq!(video.as_bytes(), expected.as_ref(), "{}", vector.name);

            let first = DescriptVideoName::new(Box::from(*b"first.hnm"));
            let mut assets = DescriptPresentationAssets::default();
            append_descript_sequence_video(&first, &mut assets);
            append_descript_sequence_video(&video, &mut assets);
            assert_eq!(assets.sequence_videos(), &[first, video], "{}", vector.name);
        }
    }

    #[test]
    fn sequence_subtitle_list_matches_every_original_append_vector() {
        let vectors: Vec<SequenceSubtitleOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_7776_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), SEQUENCE_SUBTITLE_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut input = vector.leading_word.to_le_bytes().to_vec();
            input.extend_from_slice(&bytes_from_hex(&vector.suffix_hex));
            let copied = bytes_from_hex(&vector.copied_hex);
            let (subtitle, tail) = decode_sequence_subtitle(&input).unwrap();

            assert_eq!(
                subtitle.first_visible_frame(),
                vector.leading_word,
                "{}",
                vector.name
            );
            assert_eq!(
                subtitle.text(),
                &copied[size_of::<u16>()..copied.len() - 1],
                "{}",
                vector.name
            );
            assert_eq!(tail, &input[copied.len()..], "{}", vector.name);

            let mut assets = DescriptPresentationAssets::default();
            append_descript_sequence_subtitle(&subtitle, &mut assets);
            assert_eq!(assets.sequence_subtitles(), &[subtitle], "{}", vector.name);
        }
    }

    #[test]
    fn character_sprite_selection_matches_every_original_name_area_vector() {
        let vectors: Vec<VideoOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_7788_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), SPRITE_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let input = bytes_from_hex(&vector.input_hex);
            let expected = bytes_from_hex(&vector.copied_hex);
            let (sprite, tail) = decode_sprite_name(&input).unwrap();
            assert_eq!(tail, &[vector.stopping_byte], "{}", vector.name);
            assert_eq!(sprite.as_bytes(), expected.as_ref(), "{}", vector.name);

            let mut assets = DescriptPresentationAssets::default();
            select_descript_character_sprite(&sprite, &mut assets);
            assert_eq!(assets.character_sprite(), Some(&sprite), "{}", vector.name);
        }
    }

    #[test]
    fn music_selection_matches_every_original_clean_comparison_vector() {
        let vectors: Vec<MusicOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_77a9_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), MUSIC_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let input = bytes_from_hex(&vector.input_hex);
            let expected = bytes_from_hex(&vector.transformed_hex);
            let (music, tail) = decode_music_name(&input).unwrap();
            assert_eq!(tail, &[vector.stopping_byte], "{}", vector.name);
            assert_eq!(music.as_bytes(), expected.as_ref(), "{}", vector.name);

            let prior: &[u8] = match vector.name.as_str() {
                "immediate_space" | "immediate_nul" | "immediate_high" => b"OLD",
                "equal_upper" | "lowercase_matches" => b"ABCZ",
                "mismatch_sets_changed" => b"AXCZ",
                "backtick_not_masked" => b"`Z",
                "brace_masks_to_bracket" => b"[Z",
                "prechanged_bit_blocks_unchanged"
                | "even_changed_value_allows_unchanged"
                | "source_wrap" => b"AZ",
                name => panic!("unknown music oracle {name}"),
            };
            let mut assets = DescriptPresentationAssets::default();
            let previous = DescriptMusicName::new(Box::from(prior));
            select_descript_music(&previous, &mut assets);
            let outcome = select_descript_music(&music, &mut assets);

            if vector.changed_before == u8::MIN {
                assert_eq!(
                    outcome == DescriptMusicSelectionOutcome::Changed,
                    vector.changed_after != u8::MIN,
                    "{}",
                    vector.name
                );
            }
            assert_eq!(assets.music(), Some(&music), "{}", vector.name);
        }
    }
}
