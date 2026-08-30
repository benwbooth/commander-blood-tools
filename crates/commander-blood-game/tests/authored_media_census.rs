use std::collections::{BTreeMap, BTreeSet};
use std::mem::size_of;

use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::bas::ScriptBasInstruction;
use commander_blood_formats::descript::{DescriptCharacterBackground, DescriptRecordKind};
use commander_blood_formats::descript_database::{DescriptCommand, DescriptRecord};
use commander_blood_formats::instruction::{DecodedScriptInstruction, ScriptText};
use commander_blood_formats::script::{ScriptDirectoryEntry, ScriptObjectId, ScriptObjectKind};
use commander_blood_game::native::bloodprg::{
    LoadedScriptProfile, PresentationResourceId, ScriptClock, ScriptFieldSelector, ScriptProfileId,
    TextPresentationState, presentation_line_for_text_selector, script_field_offset,
};
use commander_blood_game::runtime::{
    OriginalGameData, OriginalGameDataPaths, OriginalGameRuntime, RuntimePresentationBackground,
    RuntimePresentationCatalog, RuntimeScriptBackend,
};
use sha2::{Digest, Sha256};

const EXPECTED_AUTHORED_SAY_COUNT: usize = 5_536;
const EXPECTED_COD_SAY_COUNT: usize = 3_687;
const EXPECTED_BAS_SAY_COUNT: usize = 1_849;
const EXPECTED_PROFILE_COUNT: usize = 5;
const EXPECTED_PROFILE_SAY_COUNTS: [usize; EXPECTED_PROFILE_COUNT] =
    [112, 1_754, 1_458, 1_057, 1_155];
const EXPECTED_PROFILE_COD_SAY_COUNTS: [usize; EXPECTED_PROFILE_COUNT] =
    [111, 1_157, 1_048, 719, 652];
const EXPECTED_PROFILE_BAS_SAY_COUNTS: [usize; EXPECTED_PROFILE_COUNT] = [1, 597, 410, 338, 503];
const EXPECTED_ACTOR_COUNT: usize = 37;
const EXPECTED_PROCEDURE_COUNT: usize = 393;
const EXPECTED_TEXT_ONLY_SAY_COUNT: usize = 2_456;
const EXPECTED_PRESENTATION_SAY_COUNT: usize = 3_080;
const EXPECTED_FORCED_SUBTITLE_FONT_SAY_COUNT: usize = 1_156;
const EXPECTED_MODE_DEPENDENT_FONT_SAY_COUNT: usize = 4_380;
const EXPECTED_DYNAMIC_VIDEO_PLACEHOLDER_COUNT: usize = 947;
const EXPECTED_INHERITED_BACKGROUND_SAY_COUNT: usize = 4_531;
const EXPECTED_INHERITED_SOUND_BANK_SAY_COUNT: usize = 947;
const EXPECTED_INHERITED_MUSIC_SAY_COUNT: usize = EXPECTED_AUTHORED_SAY_COUNT;
const EXPECTED_REFERENCED_RESOURCE_COUNT: usize = 806;
const EXPECTED_LINE_MEDIA_SHA256: &str =
    "556367104c9d213a4d92afb56e6aabd11e72437144c4a6050c4e39b3ed0926c7";
const FIRST_PROFILE_NUMBER: u8 = 1;
const TEXT_ONLY_SELECTOR: i8 = -1;
const DYNAMIC_EXECUTABLE_PLACEHOLDER: &[u8] = b"xxxxxxxxxxxx";
const ASSET_CACHE_ENVIRONMENT_VARIABLE: &str = "CBLOOD_ASSET_CACHE";
const REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE: &str = "CBLOOD_REQUIRE_ACCURACY_TESTS";

const BACKGROUND_DIRECTORY: &[u8] = b"FD\\";
const LOCATION_VIDEO_DIRECTORY: &[u8] = b"PL\\";
const CHARACTER_VIDEO_DIRECTORY: &[u8] = b"PE\\";
const SEQUENCE_VIDEO_DIRECTORY: &[u8] = b"SQ\\";
const OBJECT_VIDEO_DIRECTORY: &[u8] = b"OB\\";
const SOUND_BANK_DIRECTORY: &[u8] = b"SN\\";
const MUSIC_DIRECTORY: &[u8] = b"MU\\";
const CENSUS_CLOCK: ScriptClock = ScriptClock {
    hour: 12,
    day: 1,
    month: 1,
};

const SHIPPED_AUTHORED_DEFECTS: [&[u8]; 3] =
    [b"SQ\\puven1.hnm", b"PL\\maluss20.hnm", b"FD\\marais1d.lbm"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatementImage {
    Cod,
    Bas,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FontClass {
    ForcedProgressiveSubtitle,
    PresentationModeDependent,
}

#[derive(Clone, Debug)]
struct AuthoredSay {
    profile: u8,
    image: StatementImage,
    source_offset: usize,
    actor: Box<[u8]>,
    procedure: Option<Box<[u8]>>,
    selector_root: Option<usize>,
    presentation_line: u16,
    text_only: bool,
    selected_video: Box<[u8]>,
    selected_background: DescriptCharacterBackground,
    selected_background_lbm: Option<Box<[u8]>>,
    character_sprite: Option<Box<[u8]>>,
    sound_bank: Option<Box<[u8]>>,
    music: Option<Box<[u8]>>,
    video_slot_count: usize,
    font_class: FontClass,
}

#[derive(Default)]
struct CensusSummary {
    cod_says: usize,
    bas_says: usize,
    text_only_says: usize,
    presentation_says: usize,
    forced_subtitle_font_says: usize,
    mode_dependent_font_says: usize,
    dynamic_video_placeholders: usize,
    inherited_background_says: usize,
    inherited_sound_bank_says: usize,
    inherited_music_says: usize,
    profile_says: [usize; EXPECTED_PROFILE_COUNT],
    profile_cod_says: [usize; EXPECTED_PROFILE_COUNT],
    profile_bas_says: [usize; EXPECTED_PROFILE_COUNT],
    unique_actors: BTreeSet<Box<[u8]>>,
    unique_procedures: BTreeSet<(u8, Box<[u8]>)>,
    referenced_resources: BTreeSet<Box<[u8]>>,
}

#[test]
fn every_authored_say_resolves_its_static_media_contract() {
    let Some(data) = original_data() else {
        return;
    };

    let mut unexpected_missing_resources = BTreeSet::new();
    let mut observed_authored_defects = BTreeSet::new();
    let mut referenced_resources = BTreeSet::new();
    validate_descript_resource_catalog(
        &data,
        &mut referenced_resources,
        &mut observed_authored_defects,
        &mut unexpected_missing_resources,
    );

    let mut runtime = OriginalGameRuntime::new(data);
    let mut census = Vec::with_capacity(EXPECTED_AUTHORED_SAY_COUNT);
    for profile_id in ScriptProfileId::all() {
        runtime.load_profile(profile_id).unwrap_or_else(|error| {
            panic!(
                "loading authored profile {}: {error:#}",
                profile_id.value() + FIRST_PROFILE_NUMBER
            )
        });
        census.extend(census_profile(
            runtime
                .current_profile()
                .expect("profile loader retained no profile"),
            runtime.data(),
        ));
    }

    let summary = summarize(&census, referenced_resources);
    assert_eq!(summary.cod_says, EXPECTED_COD_SAY_COUNT);
    assert_eq!(summary.bas_says, EXPECTED_BAS_SAY_COUNT);
    assert_eq!(census.len(), EXPECTED_AUTHORED_SAY_COUNT);
    assert_eq!(summary.profile_says, EXPECTED_PROFILE_SAY_COUNTS);
    assert_eq!(summary.profile_cod_says, EXPECTED_PROFILE_COD_SAY_COUNTS);
    assert_eq!(summary.profile_bas_says, EXPECTED_PROFILE_BAS_SAY_COUNTS);
    assert_eq!(summary.unique_actors.len(), EXPECTED_ACTOR_COUNT);
    assert_eq!(summary.unique_procedures.len(), EXPECTED_PROCEDURE_COUNT);
    assert_eq!(summary.text_only_says, EXPECTED_TEXT_ONLY_SAY_COUNT);
    assert_eq!(summary.presentation_says, EXPECTED_PRESENTATION_SAY_COUNT);
    assert_eq!(
        summary.forced_subtitle_font_says,
        EXPECTED_FORCED_SUBTITLE_FONT_SAY_COUNT
    );
    assert_eq!(
        summary.mode_dependent_font_says,
        EXPECTED_MODE_DEPENDENT_FONT_SAY_COUNT
    );
    assert_eq!(
        summary.dynamic_video_placeholders,
        EXPECTED_DYNAMIC_VIDEO_PLACEHOLDER_COUNT
    );
    assert_eq!(
        summary.inherited_background_says,
        EXPECTED_INHERITED_BACKGROUND_SAY_COUNT
    );
    assert_eq!(
        summary.inherited_sound_bank_says,
        EXPECTED_INHERITED_SOUND_BANK_SAY_COUNT
    );
    assert_eq!(
        summary.inherited_music_says,
        EXPECTED_INHERITED_MUSIC_SAY_COUNT
    );
    assert_eq!(
        summary.referenced_resources.len(),
        EXPECTED_REFERENCED_RESOURCE_COUNT
    );
    assert_eq!(line_media_sha256(&census), EXPECTED_LINE_MEDIA_SHA256);
    assert_eq!(ScriptProfileId::all().count(), EXPECTED_PROFILE_COUNT);
    assert!(
        unexpected_missing_resources.is_empty(),
        "unresolved authored media outside the explicit shipped defects: {}",
        format_resource_set(&unexpected_missing_resources)
    );
    assert_eq!(
        observed_authored_defects,
        SHIPPED_AUTHORED_DEFECTS
            .into_iter()
            .map(Box::from)
            .collect::<BTreeSet<_>>(),
        "the explicit defect allowlist must stay exact"
    );

    eprintln!(
        "authored-media census: says={} cod={} bas={} profile_says={:?} profile_cod={:?} profile_bas={:?} actors={} procedures={} text_only={} presentation={} forced_subtitle_font={} mode_dependent_font={} dynamic_video_placeholders={} inherited_background={} inherited_sound_bank={} inherited_music={} resources={} defects={}",
        census.len(),
        summary.cod_says,
        summary.bas_says,
        summary.profile_says,
        summary.profile_cod_says,
        summary.profile_bas_says,
        summary.unique_actors.len(),
        summary.unique_procedures.len(),
        summary.text_only_says,
        summary.presentation_says,
        summary.forced_subtitle_font_says,
        summary.mode_dependent_font_says,
        summary.dynamic_video_placeholders,
        summary.inherited_background_says,
        summary.inherited_sound_bank_says,
        summary.inherited_music_says,
        summary.referenced_resources.len(),
        observed_authored_defects.len(),
    );
}

fn line_media_sha256(census: &[AuthoredSay]) -> String {
    let mut says = census.iter().collect::<Vec<_>>();
    says.sort_by_key(|say| (say.profile, say.image as u8, say.source_offset));

    let mut hasher = Sha256::new();
    for say in says {
        hash_u64(&mut hasher, u64::from(say.profile));
        hash_u64(&mut hasher, say.image as u64);
        hash_u64(&mut hasher, say.source_offset as u64);
        hash_bytes(&mut hasher, &say.actor);
        hash_optional_bytes(&mut hasher, say.procedure.as_deref());
        hash_optional_u64(&mut hasher, say.selector_root.map(|value| value as u64));
        hash_u64(&mut hasher, u64::from(say.presentation_line));
        hash_u64(&mut hasher, say.text_only as u64);
        hash_bytes(&mut hasher, &say.selected_video);
        match say.selected_background {
            DescriptCharacterBackground::None => hash_u64(&mut hasher, u64::MIN),
            DescriptCharacterBackground::Cached(slot) => {
                hash_u64(&mut hasher, u64::from(slot.encode()))
            }
        }
        hash_optional_bytes(&mut hasher, say.selected_background_lbm.as_deref());
        hash_optional_bytes(&mut hasher, say.character_sprite.as_deref());
        hash_optional_bytes(&mut hasher, say.sound_bank.as_deref());
        hash_optional_bytes(&mut hasher, say.music.as_deref());
        hash_u64(&mut hasher, say.video_slot_count as u64);
        hash_u64(&mut hasher, say.font_class as u64);
    }
    format!("{:x}", hasher.finalize())
}

fn hash_optional_bytes(hasher: &mut Sha256, value: Option<&[u8]>) {
    match value {
        Some(value) => {
            hash_u64(hasher, 1);
            hash_bytes(hasher, value);
        }
        None => hash_u64(hasher, u64::MIN),
    }
}

fn hash_optional_u64(hasher: &mut Sha256, value: Option<u64>) {
    match value {
        Some(value) => {
            hash_u64(hasher, 1);
            hash_u64(hasher, value);
        }
        None => hash_u64(hasher, u64::MIN),
    }
}

fn hash_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hash_u64(hasher, bytes.len() as u64);
    hasher.update(bytes);
}

fn hash_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_le_bytes());
}

fn census_profile(profile: &LoadedScriptProfile, data: &OriginalGameData) -> Vec<AuthoredSay> {
    let profile_number = profile.id().value() + FIRST_PROFILE_NUMBER;
    let object_by_state_offset = profile
        .directory()
        .active_objects()
        .map(|(object, entry)| (usize::from(entry.value), (object, entry)))
        .collect::<BTreeMap<_, _>>();
    let procedures = procedure_ranges(profile);
    let bas_owners = bas_text_owners(profile);
    let mut census = Vec::new();

    for (token, instruction) in profile.code().tokens().iter().zip(profile.instructions()) {
        let DecodedScriptInstruction::Text(text) = instruction else {
            continue;
        };
        let (actor, entry) = object_by_state_offset
            .get(&text.line_record.byte_offset())
            .copied()
            .unwrap_or_else(|| {
                panic!(
                    "SCRIPT{profile_number} COD text at {} has no exact VAR/DEB owner",
                    token.source_offset().index()
                )
            });
        assert_actor(
            profile,
            actor,
            entry,
            StatementImage::Cod,
            token.source_offset().index(),
        );
        let procedure =
            procedure_at(&procedures, token.source_offset().index()).unwrap_or_else(|| {
                panic!(
                    "SCRIPT{profile_number} COD text at {} is outside every DEB procedure",
                    token.source_offset().index()
                )
            });
        census.push(resolve_say(
            profile_number,
            StatementImage::Cod,
            token.source_offset().index(),
            entry.name(),
            Some(procedure.name()),
            None,
            text,
            data,
        ));
    }

    for token in profile.dialogue().tokens() {
        let ScriptBasInstruction::Text(text) = token.instruction() else {
            continue;
        };
        let owners = bas_owners
            .get(&token.source_offset().index())
            .unwrap_or_else(|| {
                panic!(
                    "SCRIPT{profile_number} BAS text at {} has no typed actor block owner",
                    token.source_offset().index()
                )
            });
        assert_eq!(
            owners.len(),
            1,
            "SCRIPT{profile_number} BAS text at {} aliases multiple actor blocks: {owners:?}",
            token.source_offset().index()
        );
        let (actor, selector_root) = owners[0];
        let entry = profile.directory().object(actor).unwrap();
        assert_actor(
            profile,
            actor,
            entry,
            StatementImage::Bas,
            token.source_offset().index(),
        );
        census.push(resolve_say(
            profile_number,
            StatementImage::Bas,
            token.source_offset().index(),
            entry.name(),
            None,
            Some(selector_root),
            text,
            data,
        ));
    }

    census
}

fn resolve_say(
    profile: u8,
    image: StatementImage,
    source_offset: usize,
    actor: &[u8],
    procedure: Option<&[u8]>,
    selector_root: Option<usize>,
    text: &ScriptText,
    data: &OriginalGameData,
) -> AuthoredSay {
    let presentation_line = presentation_line_for_text_selector(text.presentation_selector);
    let descriptor = data.descript_database().lookup(actor);
    if let Some(record) = descriptor {
        assert_eq!(
            record.kind(),
            DescriptRecordKind::Character,
            "SCRIPT{profile} {image:?} text at {source_offset} actor {} resolves to a non-character DESCRIPT record",
            String::from_utf8_lossy(actor)
        );
    }

    let mut backend = RuntimeScriptBackend::new(data, CENSUS_CLOCK);
    let mut catalog = RuntimePresentationCatalog::new(data.presentation_catalog());
    if descriptor.is_some() {
        let mut presentation = TextPresentationState::default();
        let application = backend
            .apply_description(actor, true, &mut presentation)
            .unwrap_or_else(|error| {
                panic!(
                    "SCRIPT{profile} {image:?} text at {source_offset} could not apply actor DESCRIPT record {}: {error:#}",
                    String::from_utf8_lossy(actor)
                )
            });
        assert!(application.is_some());
        catalog
            .apply_descript_assets(backend.assets())
            .unwrap_or_else(|error| {
                panic!(
                    "SCRIPT{profile} {image:?} text at {source_offset} could not apply actor media slots {}: {error:#}",
                    String::from_utf8_lossy(actor)
                )
            });
    }
    let resource_line = PresentationResourceId::new(presentation_line);
    let selected_video = catalog
        .resource_name(resource_line)
        .unwrap_or_else(|| {
            panic!(
                "SCRIPT{profile} {image:?} text at {source_offset} selects absent presentation line {presentation_line}"
            )
        })
        .as_bytes();
    let selected_background = match catalog
        .background(resource_line)
        .expect("the presentation catalog retains a background for every line")
    {
        RuntimePresentationBackground::None => DescriptCharacterBackground::None,
        RuntimePresentationBackground::Cached(slot) => DescriptCharacterBackground::Cached(slot),
    };
    if descriptor.is_none() || is_dynamic_placeholder(selected_video) {
        assert!(
            is_dynamic_placeholder(selected_video),
            "SCRIPT{profile} {image:?} text at {source_offset} actor {} has no complete DESCRIPT resolution, and executable fallback {} is not a dynamic placeholder",
            String::from_utf8_lossy(actor),
            String::from_utf8_lossy(selected_video)
        );
    }
    let selected_background_lbm = match selected_background {
        DescriptCharacterBackground::None => None,
        DescriptCharacterBackground::Cached(slot) => descriptor
            .and_then(|record| record_background(record, slot))
            .map(|name| prefixed_name(BACKGROUND_DIRECTORY, name)),
    };

    AuthoredSay {
        profile,
        image,
        source_offset,
        actor: Box::from(actor),
        procedure: procedure.map(Box::from),
        selector_root,
        presentation_line,
        text_only: text.presentation_selector == TEXT_ONLY_SELECTOR,
        selected_video: Box::from(selected_video),
        selected_background,
        selected_background_lbm,
        character_sprite: descriptor.and_then(record_character_sprite).map(Box::from),
        sound_bank: descriptor.and_then(record_sound_bank).map(Box::from),
        music: descriptor.and_then(record_music).map(Box::from),
        video_slot_count: descriptor.map(record_video_slot_count).unwrap_or_default(),
        font_class: if text.control.emits_spoken_text() {
            FontClass::ForcedProgressiveSubtitle
        } else {
            FontClass::PresentationModeDependent
        },
    }
}

fn procedure_ranges(profile: &LoadedScriptProfile) -> Vec<ProcedureRange<'_>> {
    let mut procedures = profile
        .directory()
        .procedures()
        .map(|(_procedure, entry)| ProcedureRange {
            start: usize::from(
                entry
                    .value
                    .checked_sub(1)
                    .expect("procedure offset is one-based"),
            ),
            end: profile.code().end_marker_offset().index(),
            entry,
        })
        .collect::<Vec<_>>();
    procedures.sort_by_key(|procedure| procedure.start);
    for index in 0..procedures.len().saturating_sub(1) {
        procedures[index].end = procedures[index + 1].start;
    }
    procedures
}

struct ProcedureRange<'a> {
    start: usize,
    end: usize,
    entry: &'a ScriptDirectoryEntry,
}

impl ProcedureRange<'_> {
    fn name(&self) -> &[u8] {
        self.entry.name()
    }
}

fn procedure_at<'a>(
    procedures: &'a [ProcedureRange<'a>],
    offset: usize,
) -> Option<&'a ProcedureRange<'a>> {
    procedures
        .iter()
        .find(|procedure| offset >= procedure.start && offset < procedure.end)
}

fn bas_text_owners(profile: &LoadedScriptProfile) -> BTreeMap<usize, Vec<(ScriptObjectId, usize)>> {
    let mut owners = BTreeMap::<usize, Vec<(ScriptObjectId, usize)>>::new();
    for actor in profile
        .state()
        .objects()
        .iter()
        .filter(|object| object.kind == ScriptObjectKind::Actor)
    {
        let handoff_offset = script_field_offset(
            ScriptObjectKind::Actor,
            ScriptFieldSelector::PRESENTATION_HANDOFF,
        )
        .expect("actor layout retains its presentation handoff field");
        let handoff = profile
            .state()
            .object_word(actor.id, handoff_offset / size_of::<u16>())
            .and_then(|word| profile.state().word(word))
            .expect("decoded actor retains its BAS block pointer");
        if handoff == u16::MIN {
            continue;
        }
        let selector_root = usize::from(handoff);
        let start = profile
            .dialogue()
            .tokens()
            .binary_search_by_key(&selector_root, |token| token.source_offset().index())
            .unwrap_or_else(|_| {
                panic!(
                    "profile {} actor {:?} BAS root {selector_root} is not a token boundary",
                    profile.id().value() + FIRST_PROFILE_NUMBER,
                    actor.id
                )
            });
        let mut terminated = false;
        for token in &profile.dialogue().tokens()[start..] {
            match token.instruction() {
                ScriptBasInstruction::Text(_) => owners
                    .entry(token.source_offset().index())
                    .or_default()
                    .push((actor.id, selector_root)),
                ScriptBasInstruction::Yield | ScriptBasInstruction::End => {
                    terminated = true;
                    break;
                }
                _ => {}
            }
        }
        assert!(
            terminated,
            "profile {} actor {:?} BAS block at {selector_root} is unterminated",
            profile.id().value() + FIRST_PROFILE_NUMBER,
            actor.id
        );
    }
    owners
}

fn assert_actor(
    profile: &LoadedScriptProfile,
    actor: ScriptObjectId,
    entry: &ScriptDirectoryEntry,
    image: StatementImage,
    source_offset: usize,
) {
    assert_eq!(
        profile.state().object(actor).map(|object| object.kind),
        Some(ScriptObjectKind::Actor),
        "profile {} {image:?} text at {source_offset} owner {} is not an actor",
        profile.id().value() + FIRST_PROFILE_NUMBER,
        String::from_utf8_lossy(entry.name())
    );
}

fn summarize(census: &[AuthoredSay], referenced_resources: BTreeSet<Box<[u8]>>) -> CensusSummary {
    let mut summary = CensusSummary {
        referenced_resources,
        ..CensusSummary::default()
    };
    for say in census {
        let profile_index = usize::from(say.profile - FIRST_PROFILE_NUMBER);
        summary.profile_says[profile_index] += 1;
        match say.image {
            StatementImage::Cod => {
                summary.cod_says += 1;
                summary.profile_cod_says[profile_index] += 1;
                let procedure = say.procedure.as_ref().unwrap_or_else(|| {
                    panic!(
                        "SCRIPT{} COD text at {} lost procedure ownership",
                        say.profile, say.source_offset
                    )
                });
                assert!(say.selector_root.is_none());
                summary
                    .unique_procedures
                    .insert((say.profile, procedure.clone()));
            }
            StatementImage::Bas => {
                summary.bas_says += 1;
                summary.profile_bas_says[profile_index] += 1;
                assert!(say.procedure.is_none());
                assert!(say.selector_root.is_some());
            }
        }
        summary.unique_actors.insert(say.actor.clone());
        if say.text_only {
            summary.text_only_says += 1;
        } else {
            summary.presentation_says += 1;
        }
        match say.font_class {
            FontClass::ForcedProgressiveSubtitle => summary.forced_subtitle_font_says += 1,
            FontClass::PresentationModeDependent => summary.mode_dependent_font_says += 1,
        }
        if is_dynamic_placeholder(&say.selected_video) {
            summary.dynamic_video_placeholders += 1;
        } else {
            summary
                .referenced_resources
                .insert(say.selected_video.clone());
        }
        if matches!(
            say.selected_background,
            DescriptCharacterBackground::Cached(_)
        ) {
            if let Some(background) = &say.selected_background_lbm {
                summary.referenced_resources.insert(background.clone());
            } else {
                summary.inherited_background_says += 1;
            }
        }
        if say.sound_bank.is_none() {
            summary.inherited_sound_bank_says += 1;
        }
        if say.music.is_none() {
            summary.inherited_music_says += 1;
        }
        if let Some(sprite) = &say.character_sprite {
            summary.referenced_resources.insert(sprite.clone());
        }
        if let Some(bank) = &say.sound_bank {
            summary
                .referenced_resources
                .insert(prefixed_name(SOUND_BANK_DIRECTORY, bank));
        }
        if let Some(music) = &say.music {
            summary
                .referenced_resources
                .insert(prefixed_name(MUSIC_DIRECTORY, music));
        }
        assert!(say.presentation_line >= 8);
        assert!(say.video_slot_count <= 35);
    }
    summary
}

fn validate_descript_resource_catalog(
    data: &OriginalGameData,
    referenced: &mut BTreeSet<Box<[u8]>>,
    observed_defects: &mut BTreeSet<Box<[u8]>>,
    unexpected_missing: &mut BTreeSet<Box<[u8]>>,
) {
    for record in data.descript_database().records() {
        for command in record.commands() {
            let resource = match command {
                DescriptCommand::Background(background) => Some(prefixed_name(
                    BACKGROUND_DIRECTORY,
                    background.source_name(),
                )),
                DescriptCommand::LocationVideo(video) => {
                    Some(prefixed_name(LOCATION_VIDEO_DIRECTORY, video.as_bytes()))
                }
                DescriptCommand::TalkClip(clip) => Some(prefixed_name(
                    CHARACTER_VIDEO_DIRECTORY,
                    clip.video().as_bytes(),
                )),
                DescriptCommand::CharacterRightVideo(video)
                | DescriptCommand::CharacterLeftVideo(video) => {
                    Some(prefixed_name(CHARACTER_VIDEO_DIRECTORY, video.as_bytes()))
                }
                DescriptCommand::IdleClip(clip) => Some(prefixed_name(
                    video_directory(record.kind()),
                    clip.video().as_bytes(),
                )),
                DescriptCommand::SequenceVideo(video) => {
                    Some(prefixed_name(SEQUENCE_VIDEO_DIRECTORY, video.as_bytes()))
                }
                DescriptCommand::CharacterSprite(sprite) => Some(Box::from(sprite.as_bytes())),
                DescriptCommand::ObjectVideo(video) => {
                    Some(prefixed_name(OBJECT_VIDEO_DIRECTORY, video.as_bytes()))
                }
                DescriptCommand::SoundBank(bank) => {
                    Some(prefixed_name(SOUND_BANK_DIRECTORY, bank.as_bytes()))
                }
                DescriptCommand::Music(music) => {
                    Some(prefixed_name(MUSIC_DIRECTORY, music.as_bytes()))
                }
                DescriptCommand::Caption(_)
                | DescriptCommand::LocationLayout(_)
                | DescriptCommand::SequenceSubtitle(_) => None,
            };
            let Some(resource) = resource else {
                continue;
            };
            if is_dynamic_placeholder(&resource) {
                continue;
            }
            referenced.insert(resource.clone());
            let name = BloodResourceName::new(&resource).unwrap_or_else(|error| {
                panic!(
                    "DESCRIPT record {} has invalid resource {}: {error:?}",
                    String::from_utf8_lossy(record.name()),
                    String::from_utf8_lossy(&resource)
                )
            });
            if !data
                .resource_store()
                .resource_exists(&name)
                .unwrap_or_else(|error| {
                    panic!(
                        "probing DESCRIPT record {} resource {}: {error:#}",
                        String::from_utf8_lossy(record.name()),
                        String::from_utf8_lossy(&resource)
                    )
                })
            {
                if SHIPPED_AUTHORED_DEFECTS.contains(&resource.as_ref()) {
                    observed_defects.insert(resource);
                } else {
                    unexpected_missing.insert(resource);
                }
            }
        }
    }
}

fn record_background(
    record: &DescriptRecord,
    slot: commander_blood_formats::descript::DescriptBackgroundSlot,
) -> Option<&[u8]> {
    record
        .commands()
        .iter()
        .rev()
        .find_map(|command| match command {
            DescriptCommand::Background(background) if background.slot() == slot => {
                Some(background.source_name())
            }
            _ => None,
        })
}

fn record_character_sprite(record: &DescriptRecord) -> Option<&[u8]> {
    record.commands().iter().find_map(|command| match command {
        DescriptCommand::CharacterSprite(sprite) => Some(sprite.as_bytes()),
        _ => None,
    })
}

fn record_sound_bank(record: &DescriptRecord) -> Option<&[u8]> {
    record
        .commands()
        .iter()
        .rev()
        .find_map(|command| match command {
            DescriptCommand::SoundBank(bank) => Some(bank.as_bytes()),
            _ => None,
        })
}

fn record_music(record: &DescriptRecord) -> Option<&[u8]> {
    record
        .commands()
        .iter()
        .rev()
        .find_map(|command| match command {
            DescriptCommand::Music(music) => Some(music.as_bytes()),
            _ => None,
        })
}

fn record_video_slot_count(record: &DescriptRecord) -> usize {
    record
        .commands()
        .iter()
        .filter(|command| {
            matches!(
                command,
                DescriptCommand::LocationVideo(_)
                    | DescriptCommand::TalkClip(_)
                    | DescriptCommand::CharacterRightVideo(_)
                    | DescriptCommand::CharacterLeftVideo(_)
                    | DescriptCommand::IdleClip(_)
                    | DescriptCommand::SequenceVideo(_)
                    | DescriptCommand::ObjectVideo(_)
            )
        })
        .count()
}

fn video_directory(kind: DescriptRecordKind) -> &'static [u8] {
    match kind {
        DescriptRecordKind::Location => LOCATION_VIDEO_DIRECTORY,
        DescriptRecordKind::Character => CHARACTER_VIDEO_DIRECTORY,
        DescriptRecordKind::Sequence => SEQUENCE_VIDEO_DIRECTORY,
        DescriptRecordKind::Object => OBJECT_VIDEO_DIRECTORY,
    }
}

fn prefixed_name(directory: &[u8], name: &[u8]) -> Box<[u8]> {
    if name.contains(&b'/') || name.contains(&b'\\') {
        return Box::from(name);
    }
    let mut path = Vec::with_capacity(directory.len() + name.len());
    path.extend_from_slice(directory);
    path.extend_from_slice(name);
    path.into_boxed_slice()
}

fn is_dynamic_placeholder(name: &[u8]) -> bool {
    name.ends_with(DYNAMIC_EXECUTABLE_PLACEHOLDER)
}

fn format_resource_set(resources: &BTreeSet<Box<[u8]>>) -> String {
    resources
        .iter()
        .map(|resource| String::from_utf8_lossy(resource).into_owned())
        .collect::<Vec<_>>()
        .join(", ")
}

fn original_data() -> Option<OriginalGameData> {
    let paths = match OriginalGameDataPaths::discover(None) {
        Ok(paths) => paths,
        Err(error) if std::env::var_os(ASSET_CACHE_ENVIRONMENT_VARIABLE).is_some() => {
            panic!("configured Commander Blood asset cache is invalid: {error:#}")
        }
        Err(error) if accuracy_tests_are_required() => {
            panic!(
                "{REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE}=1 requires original Commander Blood data: {error:#}"
            )
        }
        Err(_) => return None,
    };
    Some(
        OriginalGameData::load_with_writable_root(paths, std::env::temp_dir())
            .expect("loading original Commander Blood data for authored-media census"),
    )
}

fn accuracy_tests_are_required() -> bool {
    std::env::var_os(REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE).is_some()
}
