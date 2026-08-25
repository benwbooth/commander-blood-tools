//! Typed DESCRIPT lookup and command application.

use commander_blood_formats::descript::DescriptRecordKind;
use commander_blood_formats::descript_database::{
    DescriptCommand, DescriptDatabase, DescriptRecord, DescriptRecordEnd,
};

use super::{
    DescriptBackgroundCache, DescriptBackgroundCacheOutcome, DescriptBackgroundSource,
    DescriptIdleClipSource, DescriptMusicSelectionOutcome, DescriptPresentationAssets,
    DescriptRecordBoundary, DescriptSoundBankLoader, TextPresentationState,
    append_descript_sequence_subtitle, append_descript_sequence_video, append_descript_talk_clip,
    cache_background_image, load_descript_idle_clip, load_descript_sound_bank,
    select_character_left_scene_video, select_character_right_scene_video,
    select_descript_character_sprite, select_descript_music, select_location_scene_video,
    select_object_scene_video, set_location_scene_top_row, stage_descript_caption,
    stop_before_character_record, stop_before_location_record, stop_before_object_record,
    stop_before_sequence_record,
};

/// Mutable runtime services and state needed to apply one DESCRIPT record.
pub struct DescriptApplicationContext<'a, BackgroundSource, SoundLoader, IdleSource> {
    presentation_active: bool,
    assets: &'a mut DescriptPresentationAssets,
    text: &'a mut TextPresentationState,
    backgrounds: &'a mut DescriptBackgroundCache,
    background_source: &'a mut BackgroundSource,
    sound_loader: &'a mut SoundLoader,
    idle_source: &'a mut IdleSource,
}

impl<'a, BackgroundSource, SoundLoader, IdleSource>
    DescriptApplicationContext<'a, BackgroundSource, SoundLoader, IdleSource>
{
    /// Collect the flat runtime state and resource boundaries for one lookup.
    pub fn new(
        presentation_active: bool,
        assets: &'a mut DescriptPresentationAssets,
        text: &'a mut TextPresentationState,
        backgrounds: &'a mut DescriptBackgroundCache,
        background_source: &'a mut BackgroundSource,
        sound_loader: &'a mut SoundLoader,
        idle_source: &'a mut IdleSource,
    ) -> Self {
        Self {
            presentation_active,
            assets,
            text,
            backgrounds,
            background_source,
            sound_loader,
            idle_source,
        }
    }
}

/// Resource-backend failure while applying a decoded DESCRIPT record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DescriptApplicationError<BackgroundError, SoundError, IdleError> {
    /// Loading one location background failed.
    Background(BackgroundError),
    /// Loading the selected SND bank failed.
    SoundBank(SoundError),
    /// Loading the selected idle HNM failed.
    IdleClip(IdleError),
}

/// Result of applying DESCRIPT commands through three explicit resource backends.
pub type DescriptApplicationResult<Value, BackgroundError, SoundError, IdleError> =
    Result<Value, DescriptApplicationError<BackgroundError, SoundError, IdleError>>;

/// Observable result of applying one matched DESCRIPT record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DescriptRecordApplication {
    record_kind: DescriptRecordKind,
    boundary: DescriptRecordBoundary,
    music_selection: Option<DescriptMusicSelectionOutcome>,
    character_sprite_selected: bool,
    loaded_background_count: usize,
    cached_background_count: usize,
    sound_bank_loaded: bool,
    idle_clip_loaded: bool,
}

impl DescriptRecordApplication {
    fn new(record_kind: DescriptRecordKind) -> Self {
        Self {
            record_kind,
            boundary: DescriptRecordBoundary::default(),
            music_selection: None,
            character_sprite_selected: false,
            loaded_background_count: usize::MIN,
            cached_background_count: usize::MIN,
            sound_bank_loaded: false,
            idle_clip_loaded: false,
        }
    }

    /// Return the matched record's semantic kind.
    pub const fn record_kind(self) -> DescriptRecordKind {
        self.record_kind
    }

    /// Return the typed boundary encountered after the command stream.
    pub const fn boundary(self) -> DescriptRecordBoundary {
        self.boundary
    }

    /// Return whether this record changed or reused the current music.
    pub const fn music_selection(self) -> Option<DescriptMusicSelectionOutcome> {
        self.music_selection
    }

    /// Return whether this record explicitly selected a character portrait.
    pub const fn character_sprite_selected(self) -> bool {
        self.character_sprite_selected
    }

    /// Return the number of background images loaded from the resource backend.
    pub const fn loaded_background_count(self) -> usize {
        self.loaded_background_count
    }

    /// Return the number of background commands satisfied by the retained cache.
    pub const fn cached_background_count(self) -> usize {
        self.cached_background_count
    }

    /// Return whether the selected SND bank was loaded during this application.
    pub const fn sound_bank_loaded(self) -> bool {
        self.sound_bank_loaded
    }

    /// Return whether the selected idle HNM was loaded during this application.
    pub const fn idle_clip_loaded(self) -> bool {
        self.idle_clip_loaded
    }
}

/// Find and apply the first exact case-sensitive DESCRIPT directory match.
///
/// This translates `vm_c2_descript_lookup` at BLOODPRG file offset `0x007409`.
/// The parsed database, typed command list, and resource backends replace all DOS
/// file operations, scratch-buffer parsing, fixed tables, and sentinel writes.
pub fn lookup_and_apply_descript_record<BackgroundSource, SoundLoader, IdleSource>(
    database: &DescriptDatabase,
    record_name: &[u8],
    context: &mut DescriptApplicationContext<'_, BackgroundSource, SoundLoader, IdleSource>,
) -> DescriptApplicationResult<
    Option<DescriptRecordApplication>,
    BackgroundSource::Error,
    SoundLoader::Error,
    IdleSource::Error,
>
where
    BackgroundSource: DescriptBackgroundSource,
    SoundLoader: DescriptSoundBankLoader,
    IdleSource: DescriptIdleClipSource,
{
    context.assets.begin_record_application();
    let Some(record) = database.lookup(record_name) else {
        return Ok(None);
    };

    apply_descript_record(record, context).map(Some)
}

fn apply_descript_record<BackgroundSource, SoundLoader, IdleSource>(
    record: &DescriptRecord,
    context: &mut DescriptApplicationContext<'_, BackgroundSource, SoundLoader, IdleSource>,
) -> DescriptApplicationResult<
    DescriptRecordApplication,
    BackgroundSource::Error,
    SoundLoader::Error,
    IdleSource::Error,
>
where
    BackgroundSource: DescriptBackgroundSource,
    SoundLoader: DescriptSoundBankLoader,
    IdleSource: DescriptIdleClipSource,
{
    let mut application = DescriptRecordApplication::new(record.kind());

    for command in record.commands() {
        match command {
            DescriptCommand::Background(command) => {
                match cache_background_image(
                    command,
                    context.backgrounds,
                    context.background_source,
                )
                .map_err(DescriptApplicationError::Background)?
                {
                    DescriptBackgroundCacheOutcome::Hit => {
                        application.cached_background_count += 1;
                    }
                    DescriptBackgroundCacheOutcome::Loaded { .. } => {
                        application.loaded_background_count += 1;
                    }
                }
            }
            DescriptCommand::Caption(command) => stage_descript_caption(command, context.text),
            DescriptCommand::LocationVideo(video) => {
                select_location_scene_video(video, context.assets);
            }
            DescriptCommand::TalkClip(clip) => append_descript_talk_clip(clip, context.assets),
            DescriptCommand::LocationLayout(layout) => {
                set_location_scene_top_row(*layout, context.assets);
            }
            DescriptCommand::CharacterRightVideo(video) => {
                select_character_right_scene_video(video, context.assets);
            }
            DescriptCommand::CharacterLeftVideo(video) => {
                select_character_left_scene_video(video, context.assets);
            }
            DescriptCommand::IdleClip(clip) => {
                application.idle_clip_loaded = load_descript_idle_clip(
                    clip,
                    context.presentation_active,
                    context.assets,
                    context.idle_source,
                )
                .map_err(DescriptApplicationError::IdleClip)?;
            }
            DescriptCommand::SequenceVideo(video) => {
                append_descript_sequence_video(video, context.assets);
            }
            DescriptCommand::SequenceSubtitle(subtitle) => {
                append_descript_sequence_subtitle(subtitle, context.assets);
            }
            DescriptCommand::CharacterSprite(sprite) => {
                select_descript_character_sprite(sprite, context.assets);
                application.character_sprite_selected = true;
            }
            DescriptCommand::ObjectVideo(video) => {
                select_object_scene_video(video, context.assets);
            }
            DescriptCommand::SoundBank(sound_bank) => {
                application.sound_bank_loaded = load_descript_sound_bank(
                    sound_bank,
                    context.presentation_active,
                    context.assets,
                    context.sound_loader,
                )
                .map_err(DescriptApplicationError::SoundBank)?;
            }
            DescriptCommand::Music(music) => {
                application.music_selection = Some(select_descript_music(music, context.assets));
            }
        }
    }

    apply_record_end(record.end(), &mut application.boundary);
    Ok(application)
}

fn apply_record_end(end: DescriptRecordEnd, boundary: &mut DescriptRecordBoundary) {
    let DescriptRecordEnd::NextRecord(kind) = end else {
        return;
    };
    match kind {
        DescriptRecordKind::Location => stop_before_location_record(boundary),
        DescriptRecordKind::Character => stop_before_character_record(boundary),
        DescriptRecordKind::Sequence => stop_before_sequence_record(boundary),
        DescriptRecordKind::Object => stop_before_object_record(boundary),
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::path::{Path, PathBuf};

    use commander_blood_formats::descript::{
        DescriptBackgroundSlot, DescriptCharacterBackground, DescriptSpriteName,
    };
    use serde::Deserialize;

    use super::*;

    const DISPATCHER_ORACLE_VECTOR_COUNT: usize = 25;
    const DISPATCH_SLOT_VECTOR_COUNT: usize = 18;
    const ONDOYA_BACKGROUND_COUNT: usize = 4;
    const SCRUTER_JO_TALK_CLIP_COUNT: usize = 22;

    #[derive(Default)]
    struct RecordingBackgroundSource {
        loaded_names: Vec<Box<[u8]>>,
    }

    impl DescriptBackgroundSource for RecordingBackgroundSource {
        type Error = Infallible;

        fn load_background(&mut self, source_name: &[u8]) -> Result<Box<[u8]>, Self::Error> {
            self.loaded_names.push(Box::from(source_name));
            Ok(Box::from(*b"encoded-lbm"))
        }
    }

    #[derive(Default)]
    struct RecordingSoundLoader {
        loaded_names: Vec<Box<[u8]>>,
    }

    impl DescriptSoundBankLoader for RecordingSoundLoader {
        type Error = Infallible;

        fn load_sound_bank(&mut self, bank_name: &[u8]) -> Result<(), Self::Error> {
            self.loaded_names.push(Box::from(bank_name));
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingIdleSource {
        loaded_names: Vec<Box<[u8]>>,
    }

    impl DescriptIdleClipSource for RecordingIdleSource {
        type Error = Infallible;

        fn load_idle_clip(&mut self, video_name: &[u8]) -> Result<Box<[u8]>, Self::Error> {
            self.loaded_names.push(Box::from(video_name));
            Ok(Box::from(*b"encoded-hnm"))
        }
    }

    #[derive(Deserialize)]
    struct DispatcherOracle {
        name: String,
    }

    fn original_asset() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood/DESCRIPT.DES")
    }

    fn database() -> DescriptDatabase {
        DescriptDatabase::parse(&std::fs::read(original_asset()).unwrap()).unwrap()
    }

    #[test]
    fn dispatcher_oracle_covers_lookup_edges_and_every_native_slot() {
        let vectors: Vec<DispatcherOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_7409_natural.json"
        ))
        .unwrap();

        assert_eq!(vectors.len(), DISPATCHER_ORACLE_VECTOR_COUNT);
        assert_eq!(
            vectors
                .iter()
                .filter(|vector| vector.name.starts_with("dispatch_slot_"))
                .count(),
            DISPATCH_SLOT_VECTOR_COUNT
        );
    }

    #[test]
    fn shipped_location_application_preserves_cache_and_music_lifetimes() {
        let database = database();
        let mut assets = DescriptPresentationAssets::default();
        let mut text = TextPresentationState::default();
        let mut backgrounds = DescriptBackgroundCache::default();
        let mut background_source = RecordingBackgroundSource::default();
        let mut sound_loader = RecordingSoundLoader::default();
        let mut idle_source = RecordingIdleSource::default();

        let first = {
            let mut context = DescriptApplicationContext::new(
                false,
                &mut assets,
                &mut text,
                &mut backgrounds,
                &mut background_source,
                &mut sound_loader,
                &mut idle_source,
            );
            lookup_and_apply_descript_record(&database, b"Ondoya", &mut context)
                .unwrap()
                .unwrap()
        };

        assert_eq!(first.record_kind(), DescriptRecordKind::Location);
        assert_eq!(first.loaded_background_count(), ONDOYA_BACKGROUND_COUNT);
        assert_eq!(first.cached_background_count(), usize::MIN);
        assert_eq!(
            first.music_selection(),
            Some(DescriptMusicSelectionOutcome::Changed)
        );
        assert_eq!(assets.location_scene_video(), Some(&b"ondoya.hnm"[..]));
        assert_eq!(assets.location_scene_top_row(), Some(35));
        assert_eq!(assets.music().unwrap().as_bytes(), b"ZEF1.VOC");
        assert_eq!(text.subtitle_text.as_ref(), b"planet Ondoya\r");
        assert_eq!(
            first.boundary().next_record_kind(),
            Some(DescriptRecordKind::Location)
        );
        for encoded_slot in 1..=ONDOYA_BACKGROUND_COUNT as u8 {
            assert!(
                backgrounds
                    .get(DescriptBackgroundSlot::decode(encoded_slot).unwrap())
                    .is_some()
            );
        }

        let second = {
            let mut context = DescriptApplicationContext::new(
                false,
                &mut assets,
                &mut text,
                &mut backgrounds,
                &mut background_source,
                &mut sound_loader,
                &mut idle_source,
            );
            lookup_and_apply_descript_record(&database, b"Ondoya", &mut context)
                .unwrap()
                .unwrap()
        };
        assert_eq!(second.loaded_background_count(), usize::MIN);
        assert_eq!(second.cached_background_count(), ONDOYA_BACKGROUND_COUNT);
        assert_eq!(
            second.music_selection(),
            Some(DescriptMusicSelectionOutcome::Reused)
        );
        assert_eq!(
            background_source.loaded_names.len(),
            ONDOYA_BACKGROUND_COUNT
        );
    }

    #[test]
    fn shipped_character_and_sequence_records_apply_all_typed_assets() {
        let database = database();
        let mut assets = DescriptPresentationAssets::default();
        let mut text = TextPresentationState::default();
        let mut backgrounds = DescriptBackgroundCache::default();
        let mut background_source = RecordingBackgroundSource::default();
        let mut sound_loader = RecordingSoundLoader::default();
        let mut idle_source = RecordingIdleSource::default();

        let character = {
            let mut context = DescriptApplicationContext::new(
                false,
                &mut assets,
                &mut text,
                &mut backgrounds,
                &mut background_source,
                &mut sound_loader,
                &mut idle_source,
            );
            lookup_and_apply_descript_record(&database, b"Scruter_Jo", &mut context)
                .unwrap()
                .unwrap()
        };
        assert_eq!(character.record_kind(), DescriptRecordKind::Character);
        assert!(character.character_sprite_selected());
        assert!(character.sound_bank_loaded());
        assert!(character.idle_clip_loaded());
        assert_eq!(assets.talk_clips().len(), SCRUTER_JO_TALK_CLIP_COUNT);
        assert_eq!(assets.sound_bank(), Some(&b"scrut.snd"[..]));
        assert_eq!(
            assets.character_sprite().map(DescriptSpriteName::as_bytes),
            Some(&b"scruter.spr"[..])
        );
        assert_eq!(assets.location_scene_video(), None);
        assert_eq!(sound_loader.loaded_names, [Box::from(*b"scrut.snd")]);
        assert_eq!(idle_source.loaded_names, [Box::from(*b"scr20.hnm")]);
        assert_eq!(
            assets.talk_clips()[0].background(),
            DescriptCharacterBackground::Cached(DescriptBackgroundSlot::decode(4).unwrap())
        );

        let sequence = {
            let mut context = DescriptApplicationContext::new(
                false,
                &mut assets,
                &mut text,
                &mut backgrounds,
                &mut background_source,
                &mut sound_loader,
                &mut idle_source,
            );
            lookup_and_apply_descript_record(&database, b"present", &mut context)
                .unwrap()
                .unwrap()
        };
        assert_eq!(sequence.record_kind(), DescriptRecordKind::Sequence);
        assert_eq!(assets.sequence_videos().len(), 1);
        assert_eq!(assets.sequence_videos()[0].as_bytes(), b"cliptoot.hnm");
        assert_eq!(assets.sequence_subtitles().len(), 3);
        assert_eq!(assets.sequence_subtitles()[0].first_visible_frame(), 1);
        assert!(assets.talk_clips().is_empty());
        assert!(assets.character_sprite().is_none());
        assert!(assets.idle_clip().is_none());
    }

    #[test]
    fn missing_lookup_clears_per_record_assets_but_preserves_current_music() {
        let database = database();
        let mut assets = DescriptPresentationAssets::default();
        let mut text = TextPresentationState::default();
        let mut backgrounds = DescriptBackgroundCache::default();
        let mut background_source = RecordingBackgroundSource::default();
        let mut sound_loader = RecordingSoundLoader::default();
        let mut idle_source = RecordingIdleSource::default();

        {
            let mut context = DescriptApplicationContext::new(
                false,
                &mut assets,
                &mut text,
                &mut backgrounds,
                &mut background_source,
                &mut sound_loader,
                &mut idle_source,
            );
            lookup_and_apply_descript_record(&database, b"present", &mut context)
                .unwrap()
                .unwrap();
        }
        let current_music = assets.music().unwrap().clone();
        let missing = {
            let mut context = DescriptApplicationContext::new(
                false,
                &mut assets,
                &mut text,
                &mut backgrounds,
                &mut background_source,
                &mut sound_loader,
                &mut idle_source,
            );
            lookup_and_apply_descript_record(&database, b"PRESENT", &mut context).unwrap()
        };

        assert!(missing.is_none());
        assert!(assets.sequence_videos().is_empty());
        assert!(assets.sequence_subtitles().is_empty());
        assert_eq!(assets.music(), Some(&current_music));
    }
}
