//! Typed resources embedded in the original `BLOODPRG.EXE` image.

use std::error::Error;
use std::fmt;

use crate::archive::{BloodArchiveError, BloodResourceName};

/// Number of authored navigation anchors stored before the angle table.
pub const BLOODPRG_BRIDGE_AUTHORED_ANCHOR_COUNT: usize = 10;
/// Number of anchors consumed by the recovered bridge object projector.
pub const BLOODPRG_BRIDGE_PROJECTION_ANCHOR_COUNT: usize = 11;
/// Number of two-degree samples in the bridge trigonometry table.
pub const BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT: usize = 180;
/// Number of byte values represented by the proportional-font character maps.
pub const BLOODPRG_PROPORTIONAL_FONT_CHARACTER_COUNT: usize = 176;
/// Number of byte values represented by the compact small-font character map.
pub const BLOODPRG_SMALL_FONT_CHARACTER_COUNT: usize = 128;
/// Byte count consumed by the dual-font measurement routine from each advance base.
pub const BLOODPRG_FONT_MEASUREMENT_ADVANCE_COUNT: usize = 256;
/// Number of square-cap glyphs embedded in the executable.
pub const BLOODPRG_SQUARE_CAPS_GLYPH_COUNT: usize = 48;
/// Number of main dialogue glyphs embedded in the executable.
pub const BLOODPRG_MAIN_FONT_GLYPH_COUNT: usize = 86;
/// Number of subtitle-console glyphs embedded in the executable.
pub const BLOODPRG_SUBTITLE_FONT_GLYPH_COUNT: usize = 55;
/// Number of compact small-font glyphs embedded in the executable.
pub const BLOODPRG_SMALL_FONT_GLYPH_COUNT: usize = 42;
/// Byte-to-glyph entries in each sequel proportional/subtitle map.
pub const BLOOD2PG_PROPORTIONAL_FONT_CHARACTER_COUNT: usize = 232;
/// Reachable square-cap glyphs in the sequel, including its extra character.
pub const BLOOD2PG_SQUARE_CAPS_GLYPH_COUNT: usize = 49;
/// Reachable main dialogue glyphs in the sequel.
pub const BLOOD2PG_MAIN_FONT_GLYPH_COUNT: usize = 87;
/// Eight-row subtitle glyphs in the sequel.
pub const BLOOD2PG_SUBTITLE_FONT_GLYPH_COUNT: usize = 66;
/// Number of presentation-line templates indexed by the native scene dispatcher.
pub const BLOODPRG_PRESENTATION_LINE_COUNT: usize = 45;
/// Number of authored line IDs allowed to exceed the ordinary 130-row presentation band.
pub const BLOODPRG_UNCLAMPED_PRESENTATION_LINE_COUNT: usize = 8;
/// Number of commands in the executable-authored bridge options menu.
pub const BLOODPRG_OPTION_MENU_LABEL_COUNT: usize = 5;
/// Number of choices in the executable-authored text-speed menu.
pub const BLOODPRG_TEXT_SPEED_LABEL_COUNT: usize = 5;
/// Number of executable-authored hyperspace clips selected by camera travel.
pub const BLOODPRG_HYPERSPACE_SEQUENCE_COUNT: usize = 8;
/// Number of executable-authored bridge actor records.
pub const BLOODPRG_NAV_ACTOR_RECORD_COUNT: usize = 6;
/// Number of radial-wipe endpoints shared by closing and opening navigation frames.
pub const BLOODPRG_NAVIGATION_WIPE_ENDPOINT_COUNT: usize = 9;

const MZ_SIGNATURE: [u8; 2] = [b'M', b'Z'];
const MZ_SIGNATURE_FILE_OFFSET: usize = 0;
const BLOODPRG_DATA_FILE_OFFSET: usize = 0xD420;
const PRESENTATION_INDEX_DATA_OFFSET: usize = 0x1FB5;
const PRESENTATION_DESCRIPTOR_DATA_END_OFFSET: usize = 0x24F5;
const PRESENTATION_INDEX_ENTRY_BYTE_COUNT: usize = 4;
const PRESENTATION_DESCRIPTOR_OFFSET_FIELD: usize = 0;
const PRESENTATION_SCENE_IMAGE_OFFSET_FIELD: usize = 2;
const PRESENTATION_DESCRIPTOR_HEADER_BYTE_COUNT: usize = 2;
const PRESENTATION_RESOURCE_NAME_MAXIMUM_FIELD_BYTE_COUNT: usize = 16;
const NO_PRESENTATION_SCENE_IMAGE_OFFSET: u16 = u16::MAX;
const UNCLAMPED_PRESENTATION_LINE_IDS_DATA_OFFSET: usize = 0x0DBE;
const CONFIRM_DIALOG_YES_REGION_DATA_OFFSET: usize = 0x2555;
const CONFIRM_DIALOG_NO_REGION_DATA_OFFSET: usize = 0x255D;
const LIST_WIDGET_CANCEL_LABEL_DATA_OFFSET: usize = 0x0174;
const INITIAL_TEXT_SPEED_STEP_DATA_OFFSET: usize = 0x0ACA;
const OPTION_MENU_POINTER_LIST_DATA_OFFSET: usize = 0x2567;
const MUSIC_ON_LABEL_DATA_OFFSET: usize = 0x2578;
const TEXT_SPEED_POINTER_LIST_DATA_OFFSET: usize = 0x259D;
const HYPERSPACE_SEQUENCE_NAMES_DATA_OFFSET: usize = 0x1F22;
const HYPERSPACE_SEQUENCE_NAME_FIELD_BYTE_COUNT: usize = 16;
const NAV_ACTOR_RECORDS_DATA_OFFSET: usize = 0x2A1B;
const NAV_ACTOR_RECORD_BYTE_COUNT: usize = 24;
const NAV_ACTOR_FLAGS_FIELD_OFFSET: usize = 0;
const NAV_ACTOR_RESOURCE_FIELD_OFFSET: usize = 2;
const NAV_ACTOR_TRANSITION_RESOURCE_FIELD_OFFSET: usize = 4;
const NAV_ACTOR_TERMINAL_FRAME_FIELD_OFFSET: usize = 6;
const NAV_ACTOR_FRAME_FIELD_OFFSET: usize = 8;
const NAV_ACTOR_TARGET_ARC_FIELD_OFFSET: usize = 10;
const NAV_ACTOR_HIT_RECTANGLE_FIELD_OFFSET: usize = 12;
const NAV_ACTOR_DRAW_POSITION_FIELD_OFFSET: usize = 20;
const NO_NAV_ACTOR_TRANSITION_RESOURCE: u16 = u16::MAX;
const NAVIGATION_PLANET_LABEL_DATA_OFFSET: usize = 0x012E;
const NAVIGATION_SHIP_LABEL_DATA_OFFSET: usize = 0x0137;
const NAVIGATION_BLACK_HOLE_LABEL_DATA_OFFSET: usize = 0x013E;
const NAVIGATION_LIFE_SUPPORT_LABEL_DATA_OFFSET: usize = 0x014B;
const NAVIGATION_LABEL_DATA_END_OFFSET: usize = 0x015A;
const NAVIGATION_WIPE_ENDPOINTS_DATA_OFFSET: usize = 0x2752;
const NAVIGATION_WIPE_ENDPOINT_COMPONENT_COUNT: usize = 2;
const NAVIGATION_WIPE_ENDPOINT_BYTE_COUNT: usize =
    NAVIGATION_WIPE_ENDPOINT_COMPONENT_COUNT * WORD_BYTE_COUNT;
const POINTER_LIST_SENTINEL: u16 = u16::MAX;
const MENU_LABEL_MAXIMUM_BYTE_COUNT: usize = 32;
const BRIDGE_PROJECTION_ANCHOR_DATA_OFFSET: usize = 0x4F09;
const BRIDGE_TRIGONOMETRY_DATA_OFFSET: usize = 0x4F45;
const POSITION_COMPONENT_COUNT: usize = 3;
const TRIGONOMETRY_COMPONENT_COUNT: usize = 2;
const WORD_BYTE_COUNT: usize = 2;
const RECTANGLE_COMPONENT_COUNT: usize = 4;
const RECTANGLE_BYTE_COUNT: usize = RECTANGLE_COMPONENT_COUNT * WORD_BYTE_COUNT;
const PROJECTION_ANCHOR_BYTE_COUNT: usize = POSITION_COMPONENT_COUNT * WORD_BYTE_COUNT;
const TRIGONOMETRY_SAMPLE_BYTE_COUNT: usize = TRIGONOMETRY_COMPONENT_COUNT * WORD_BYTE_COUNT;
const SQUARE_CAPS_GLYPH_HEIGHT: usize = 10;
const SQUARE_CAPS_ROW_BYTE_COUNT: usize = 2;
const MAIN_FONT_GLYPH_HEIGHT: usize = 8;
const SUBTITLE_FONT_GLYPH_HEIGHT: usize = 8;
const SMALL_FONT_GLYPH_HEIGHT: usize = 5;
const SMALL_FONT_CHARACTER_MAP_DATA_OFFSET: usize = 0x6FA8;
const SMALL_FONT_GLYPH_DATA_OFFSET: usize = 0x7028;
const SUBTITLE_FONT_CHARACTER_MAP_DATA_OFFSET: usize = 0x70FA;
const SUBTITLE_FONT_GLYPH_DATA_OFFSET: usize = 0x71AA;
const SQUARE_CAPS_CHARACTER_MAP_DATA_OFFSET: usize = 0x7362;
const SQUARE_CAPS_ADVANCE_DATA_OFFSET: usize = 0x7412;
const SQUARE_CAPS_GLYPH_DATA_OFFSET: usize = 0x7442;
const MAIN_FONT_CHARACTER_MAP_DATA_OFFSET: usize = 0x7802;
const MAIN_FONT_ADVANCE_DATA_OFFSET: usize = 0x78B2;
const MAIN_FONT_GLYPH_DATA_OFFSET: usize = 0x7908;
#[cfg(test)]
const SQUARE_CAPS_GLYPH_BYTE_COUNT: usize =
    BLOODPRG_SQUARE_CAPS_GLYPH_COUNT * SQUARE_CAPS_GLYPH_HEIGHT * SQUARE_CAPS_ROW_BYTE_COUNT;
const MAIN_FONT_GLYPH_BYTE_COUNT: usize = BLOODPRG_MAIN_FONT_GLYPH_COUNT * MAIN_FONT_GLYPH_HEIGHT;
#[cfg(test)]
const SUBTITLE_FONT_GLYPH_BYTE_COUNT: usize =
    BLOODPRG_SUBTITLE_FONT_GLYPH_COUNT * SUBTITLE_FONT_GLYPH_HEIGHT;
const SMALL_FONT_GLYPH_BYTE_COUNT: usize =
    BLOODPRG_SMALL_FONT_GLYPH_COUNT * SMALL_FONT_GLYPH_HEIGHT;
const PROJECTION_ANCHOR_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + BRIDGE_PROJECTION_ANCHOR_DATA_OFFSET;
const TRIGONOMETRY_FILE_OFFSET: usize = BLOODPRG_DATA_FILE_OFFSET + BRIDGE_TRIGONOMETRY_DATA_OFFSET;
// Native sequel projector 0xB337, matrix builder 0xB058 and actor dispatcher
// use these independently recovered positions, not a universal relocation delta.
const BLOOD2PG_PROJECTION_ANCHOR_FILE_OFFSET: usize = 0x14AC9;
const BLOOD2PG_TRIGONOMETRY_FILE_OFFSET: usize = 0x14B05;
const BLOOD2PG_NAV_ACTOR_RECORDS_FILE_OFFSET: usize = 0x124AB;
const NAV_ACTOR_RECORDS_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + NAV_ACTOR_RECORDS_DATA_OFFSET;
const CONFIRM_DIALOG_YES_REGION_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + CONFIRM_DIALOG_YES_REGION_DATA_OFFSET;
const CONFIRM_DIALOG_NO_REGION_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + CONFIRM_DIALOG_NO_REGION_DATA_OFFSET;
const SMALL_FONT_CHARACTER_MAP_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SMALL_FONT_CHARACTER_MAP_DATA_OFFSET;
const SMALL_FONT_GLYPH_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SMALL_FONT_GLYPH_DATA_OFFSET;
const SUBTITLE_FONT_CHARACTER_MAP_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SUBTITLE_FONT_CHARACTER_MAP_DATA_OFFSET;
const SUBTITLE_FONT_GLYPH_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SUBTITLE_FONT_GLYPH_DATA_OFFSET;
const SQUARE_CAPS_CHARACTER_MAP_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SQUARE_CAPS_CHARACTER_MAP_DATA_OFFSET;
const SQUARE_CAPS_ADVANCE_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SQUARE_CAPS_ADVANCE_DATA_OFFSET;
const SQUARE_CAPS_GLYPH_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + SQUARE_CAPS_GLYPH_DATA_OFFSET;
const MAIN_FONT_CHARACTER_MAP_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + MAIN_FONT_CHARACTER_MAP_DATA_OFFSET;
const MAIN_FONT_ADVANCE_FILE_OFFSET: usize =
    BLOODPRG_DATA_FILE_OFFSET + MAIN_FONT_ADVANCE_DATA_OFFSET;
const MAIN_FONT_GLYPH_FILE_OFFSET: usize = BLOODPRG_DATA_FILE_OFFSET + MAIN_FONT_GLYPH_DATA_OFFSET;
#[cfg(test)]
const REQUIRED_EXECUTABLE_LENGTH: usize = TRIGONOMETRY_FILE_OFFSET
    + BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT * TRIGONOMETRY_SAMPLE_BYTE_COUNT;
const FONT_REQUIRED_EXECUTABLE_LENGTH: usize =
    MAIN_FONT_GLYPH_FILE_OFFSET + MAIN_FONT_GLYPH_BYTE_COUNT;

struct ExecutableFontLayout {
    small_map: usize,
    small_glyphs: usize,
    proportional_map_count: usize,
    subtitle_map: usize,
    subtitle_glyphs: usize,
    subtitle_glyph_count: usize,
    square_map: usize,
    square_advances: usize,
    square_glyphs: usize,
    square_glyph_count: usize,
    main_map: usize,
    main_advances: usize,
    main_glyphs: usize,
    main_glyph_count: usize,
    required_length: usize,
}

const COMMANDER_FONT_LAYOUT: ExecutableFontLayout = ExecutableFontLayout {
    small_map: SMALL_FONT_CHARACTER_MAP_FILE_OFFSET,
    small_glyphs: SMALL_FONT_GLYPH_FILE_OFFSET,
    proportional_map_count: BLOODPRG_PROPORTIONAL_FONT_CHARACTER_COUNT,
    subtitle_map: SUBTITLE_FONT_CHARACTER_MAP_FILE_OFFSET,
    subtitle_glyphs: SUBTITLE_FONT_GLYPH_FILE_OFFSET,
    subtitle_glyph_count: BLOODPRG_SUBTITLE_FONT_GLYPH_COUNT,
    square_map: SQUARE_CAPS_CHARACTER_MAP_FILE_OFFSET,
    square_advances: SQUARE_CAPS_ADVANCE_FILE_OFFSET,
    square_glyphs: SQUARE_CAPS_GLYPH_FILE_OFFSET,
    square_glyph_count: BLOODPRG_SQUARE_CAPS_GLYPH_COUNT,
    main_map: MAIN_FONT_CHARACTER_MAP_FILE_OFFSET,
    main_advances: MAIN_FONT_ADVANCE_FILE_OFFSET,
    main_glyphs: MAIN_FONT_GLYPH_FILE_OFFSET,
    main_glyph_count: BLOODPRG_MAIN_FONT_GLYPH_COUNT,
    required_length: FONT_REQUIRED_EXECUTABLE_LENGTH,
};

// Font data referenced by the sequel renderers at 0x3486, 0x3512, 0x39BE
// and 0x3A78; the dual-width routine at 0x344D uses the same maps/advances.
const SEQUEL_FONT_LAYOUT: ExecutableFontLayout = ExecutableFontLayout {
    small_map: 0x1709E,
    small_glyphs: 0x1711E,
    proportional_map_count: BLOOD2PG_PROPORTIONAL_FONT_CHARACTER_COUNT,
    subtitle_map: 0x171F0,
    subtitle_glyphs: 0x172D8,
    subtitle_glyph_count: BLOOD2PG_SUBTITLE_FONT_GLYPH_COUNT,
    square_map: 0x174E8,
    square_advances: 0x175D0,
    square_glyphs: 0x17602,
    square_glyph_count: BLOOD2PG_SQUARE_CAPS_GLYPH_COUNT,
    main_map: 0x179D6,
    main_advances: 0x17ABE,
    main_glyphs: 0x17B16,
    main_glyph_count: BLOOD2PG_MAIN_FONT_GLYPH_COUNT,
    required_length: 0x17B16 + BLOOD2PG_MAIN_FONT_GLYPH_COUNT * MAIN_FONT_GLYPH_HEIGHT,
};
const PRESENTATION_REQUIRED_EXECUTABLE_LENGTH: usize =
    BLOODPRG_DATA_FILE_OFFSET + PRESENTATION_DESCRIPTOR_DATA_END_OFFSET;
const CONFIRM_DIALOG_REQUIRED_EXECUTABLE_LENGTH: usize =
    CONFIRM_DIALOG_NO_REGION_FILE_OFFSET + RECTANGLE_BYTE_COUNT;
const MENU_TEXT_REQUIRED_EXECUTABLE_LENGTH: usize = BLOODPRG_DATA_FILE_OFFSET
    + TEXT_SPEED_POINTER_LIST_DATA_OFFSET
    + (BLOODPRG_TEXT_SPEED_LABEL_COUNT + 1) * WORD_BYTE_COUNT;
const HYPERSPACE_RESOURCES_REQUIRED_EXECUTABLE_LENGTH: usize = BLOODPRG_DATA_FILE_OFFSET
    + HYPERSPACE_SEQUENCE_NAMES_DATA_OFFSET
    + BLOODPRG_HYPERSPACE_SEQUENCE_COUNT * HYPERSPACE_SEQUENCE_NAME_FIELD_BYTE_COUNT;
const NAVIGATION_RESOURCES_REQUIRED_EXECUTABLE_LENGTH: usize = BLOODPRG_DATA_FILE_OFFSET
    + NAVIGATION_WIPE_ENDPOINTS_DATA_OFFSET
    + BLOODPRG_NAVIGATION_WIPE_ENDPOINT_COUNT * NAVIGATION_WIPE_ENDPOINT_BYTE_COUNT;

/// Authored title and roster labels used by navigation panels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BloodprgNavigationLabels {
    planet: Box<[u8]>,
    ship: Box<[u8]>,
    black_hole: Box<[u8]>,
    life_support: Box<[u8]>,
}

impl BloodprgNavigationLabels {
    /// Prefix for an ordinary planet or celestial location.
    pub const fn planet(&self) -> &[u8] {
        &self.planet
    }

    /// Prefix for a ship navigation entity.
    pub const fn ship(&self) -> &[u8] {
        &self.ship
    }

    /// Prefix for a black-hole destination.
    pub const fn black_hole(&self) -> &[u8] {
        &self.black_hole
    }

    /// Heading above the location's active life-support roster.
    pub const fn life_support(&self) -> &[u8] {
        &self.life_support
    }
}

/// Flat navigation constants decoded from initialized executable data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BloodprgNavigationResources {
    wipe_endpoints:
        [[u16; NAVIGATION_WIPE_ENDPOINT_COMPONENT_COUNT]; BLOODPRG_NAVIGATION_WIPE_ENDPOINT_COUNT],
    labels: BloodprgNavigationLabels,
}

impl BloodprgNavigationResources {
    /// Nine radial-wipe endpoints in executable lookup order.
    pub const fn wipe_endpoints(
        &self,
    ) -> &[[u16; NAVIGATION_WIPE_ENDPOINT_COMPONENT_COUNT]; BLOODPRG_NAVIGATION_WIPE_ENDPOINT_COUNT]
    {
        &self.wipe_endpoints
    }

    /// Exact game-font labels used by navigation status and location panels.
    pub const fn labels(&self) -> &BloodprgNavigationLabels {
        &self.labels
    }
}

/// Malformed navigation constants in `BLOODPRG.EXE`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BloodprgNavigationResourcesError {
    /// The input does not begin with an MZ executable signature.
    InvalidExecutableSignature,
    /// The executable ends before the complete endpoint table.
    TruncatedExecutable {
        /// Supplied executable byte count.
        actual: usize,
        /// Minimum byte count required by all navigation constants.
        required: usize,
    },
    /// One bounded label region has no C-string terminator.
    MissingLabelTerminator {
        /// Data-image-relative start of the malformed label.
        label_offset: usize,
    },
    /// One label contains a byte outside the printable game-font range.
    InvalidLabelByte {
        /// Data-image-relative start of the malformed label.
        label_offset: usize,
        /// Rejected source byte.
        byte: u8,
    },
}

impl fmt::Display for BloodprgNavigationResourcesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid BLOODPRG navigation resources: {self:?}")
    }
}

impl Error for BloodprgNavigationResourcesError {}

/// Decode navigation labels and radial-wipe geometry into owned flat values.
pub fn decode_bloodprg_navigation_resources(
    executable: &[u8],
) -> Result<BloodprgNavigationResources, BloodprgNavigationResourcesError> {
    if executable.len() < NAVIGATION_RESOURCES_REQUIRED_EXECUTABLE_LENGTH {
        return Err(BloodprgNavigationResourcesError::TruncatedExecutable {
            actual: executable.len(),
            required: NAVIGATION_RESOURCES_REQUIRED_EXECUTABLE_LENGTH,
        });
    }
    if executable.get(MZ_SIGNATURE_FILE_OFFSET..MZ_SIGNATURE.len()) != Some(&MZ_SIGNATURE) {
        return Err(BloodprgNavigationResourcesError::InvalidExecutableSignature);
    }

    let wipe_table = BLOODPRG_DATA_FILE_OFFSET + NAVIGATION_WIPE_ENDPOINTS_DATA_OFFSET;
    let wipe_endpoints = std::array::from_fn(|endpoint| {
        let position = wipe_table + endpoint * NAVIGATION_WIPE_ENDPOINT_BYTE_COUNT;
        std::array::from_fn(|component| {
            read_unsigned_word(executable, position + component * WORD_BYTE_COUNT)
        })
    });
    let labels = BloodprgNavigationLabels {
        planet: read_navigation_label(
            executable,
            NAVIGATION_PLANET_LABEL_DATA_OFFSET,
            NAVIGATION_SHIP_LABEL_DATA_OFFSET,
        )?,
        ship: read_navigation_label(
            executable,
            NAVIGATION_SHIP_LABEL_DATA_OFFSET,
            NAVIGATION_BLACK_HOLE_LABEL_DATA_OFFSET,
        )?,
        black_hole: read_navigation_label(
            executable,
            NAVIGATION_BLACK_HOLE_LABEL_DATA_OFFSET,
            NAVIGATION_LIFE_SUPPORT_LABEL_DATA_OFFSET,
        )?,
        life_support: read_navigation_label(
            executable,
            NAVIGATION_LIFE_SUPPORT_LABEL_DATA_OFFSET,
            NAVIGATION_LABEL_DATA_END_OFFSET,
        )?,
    };
    Ok(BloodprgNavigationResources {
        wipe_endpoints,
        labels,
    })
}

fn read_navigation_label(
    executable: &[u8],
    label_offset: usize,
    label_end_offset: usize,
) -> Result<Box<[u8]>, BloodprgNavigationResourcesError> {
    let field = &executable
        [BLOODPRG_DATA_FILE_OFFSET + label_offset..BLOODPRG_DATA_FILE_OFFSET + label_end_offset];
    let length = field
        .iter()
        .position(|byte| *byte == u8::MIN)
        .ok_or(BloodprgNavigationResourcesError::MissingLabelTerminator { label_offset })?;
    if let Some(byte) = field[..length]
        .iter()
        .copied()
        .find(|byte| !matches!(byte, b' '..=b'~'))
    {
        return Err(BloodprgNavigationResourcesError::InvalidLabelByte { label_offset, byte });
    }
    Ok(field[..length].into())
}

/// Flat, validated hyperspace clip names decoded from the executable data image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BloodprgHyperspaceResources {
    sequence_names: [Box<[u8]>; BLOODPRG_HYPERSPACE_SEQUENCE_COUNT],
}

impl BloodprgHyperspaceResources {
    /// Return `hyper_00.hnm` through `hyper_07.hnm` in native selection order.
    pub const fn sequence_names(&self) -> &[Box<[u8]>; BLOODPRG_HYPERSPACE_SEQUENCE_COUNT] {
        &self.sequence_names
    }
}

/// Malformed camera-travel resources in `BLOODPRG.EXE`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BloodprgHyperspaceResourcesError {
    /// The input does not begin with an MZ executable signature.
    InvalidExecutableSignature,
    /// The executable ends before the complete fixed-width name table.
    TruncatedExecutable {
        /// Supplied executable byte count.
        actual: usize,
        /// Minimum byte count required by the table.
        required: usize,
    },
    /// One fixed-width source field has no C-string terminator.
    MissingNameTerminator {
        /// Zero-based sequence index.
        sequence: usize,
    },
    /// Bytes after a sequence name's terminator are not zero-filled.
    NonZeroNamePadding {
        /// Zero-based sequence index.
        sequence: usize,
        /// Unexpected padding byte.
        byte: u8,
    },
    /// A decoded name is not accepted by the shared DOS resource-name type.
    InvalidResourceName {
        /// Zero-based sequence index.
        sequence: usize,
        /// Resource-name validation failure.
        source: BloodArchiveError,
    },
}

impl fmt::Display for BloodprgHyperspaceResourcesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid BLOODPRG hyperspace resources: {self:?}")
    }
}

impl Error for BloodprgHyperspaceResourcesError {}

/// Decode the fixed-width hyperspace clip table into ordinary owned strings.
pub fn decode_bloodprg_hyperspace_resources(
    executable: &[u8],
) -> Result<BloodprgHyperspaceResources, BloodprgHyperspaceResourcesError> {
    if executable.len() < HYPERSPACE_RESOURCES_REQUIRED_EXECUTABLE_LENGTH {
        return Err(BloodprgHyperspaceResourcesError::TruncatedExecutable {
            actual: executable.len(),
            required: HYPERSPACE_RESOURCES_REQUIRED_EXECUTABLE_LENGTH,
        });
    }
    if executable.get(MZ_SIGNATURE_FILE_OFFSET..MZ_SIGNATURE.len()) != Some(&MZ_SIGNATURE) {
        return Err(BloodprgHyperspaceResourcesError::InvalidExecutableSignature);
    }

    let table_file_offset = BLOODPRG_DATA_FILE_OFFSET + HYPERSPACE_SEQUENCE_NAMES_DATA_OFFSET;
    let names = (0..BLOODPRG_HYPERSPACE_SEQUENCE_COUNT)
        .map(|sequence| {
            let start = table_file_offset + sequence * HYPERSPACE_SEQUENCE_NAME_FIELD_BYTE_COUNT;
            let field = &executable[start..start + HYPERSPACE_SEQUENCE_NAME_FIELD_BYTE_COUNT];
            let length = field
                .iter()
                .position(|byte| *byte == u8::MIN)
                .ok_or(BloodprgHyperspaceResourcesError::MissingNameTerminator { sequence })?;
            if let Some(byte) = field[length + 1..]
                .iter()
                .copied()
                .find(|byte| *byte != u8::MIN)
            {
                return Err(BloodprgHyperspaceResourcesError::NonZeroNamePadding {
                    sequence,
                    byte,
                });
            }
            BloodResourceName::new(&field[..length])
                .map(|name| Box::from(name.as_bytes()))
                .map_err(
                    |source| BloodprgHyperspaceResourcesError::InvalidResourceName {
                        sequence,
                        source,
                    },
                )
        })
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| unreachable!("decoded exactly BLOODPRG_HYPERSPACE_SEQUENCE_COUNT names"))?;
    Ok(BloodprgHyperspaceResources {
        sequence_names: names,
    })
}

/// Flat, owned bridge-menu text decoded from the executable's data image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BloodprgBridgeMenuText {
    option_labels: [Box<[u8]>; BLOODPRG_OPTION_MENU_LABEL_COUNT],
    music_on_label: Box<[u8]>,
    text_speed_labels: [Box<[u8]>; BLOODPRG_TEXT_SPEED_LABEL_COUNT],
    cancel_label: Box<[u8]>,
    initial_text_speed_step: u16,
}

impl BloodprgBridgeMenuText {
    /// Return `TEXT`, the current music-toggle face, `SAVE`, `LOAD`, and `QUIT` labels.
    pub const fn option_labels(&self) -> &[Box<[u8]>; BLOODPRG_OPTION_MENU_LABEL_COUNT] {
        &self.option_labels
    }

    /// Return the alternate music-toggle face used when playback is disabled.
    pub const fn music_on_label(&self) -> &[u8] {
        &self.music_on_label
    }

    /// Return the five labels ordered from very fast through very slow.
    pub const fn text_speed_labels(&self) -> &[Box<[u8]>; BLOODPRG_TEXT_SPEED_LABEL_COUNT] {
        &self.text_speed_labels
    }

    /// Return the shared trailing label used by cancellable bridge lists.
    pub const fn cancel_label(&self) -> &[u8] {
        &self.cancel_label
    }

    /// Return the subtitle-speed step shipped in the executable's initialized data.
    pub const fn initial_text_speed_step(&self) -> u16 {
        self.initial_text_speed_step
    }
}

/// Malformed bridge-menu text tables in `BLOODPRG.EXE`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BloodprgBridgeMenuTextError {
    /// The input does not begin with an MZ executable signature.
    InvalidExecutableSignature,
    /// The executable ends before the complete pointer tables.
    TruncatedExecutable {
        /// Supplied executable byte count.
        actual: usize,
        /// Minimum byte count required by the menu tables.
        required: usize,
    },
    /// A fixed-size pointer list does not end with the native all-ones marker.
    MissingPointerListSentinel {
        /// Data-image-relative pointer-list position.
        list_offset: usize,
        /// Unexpected word stored after the final expected pointer.
        actual: u16,
    },
    /// A label pointer does not address a byte in the executable image.
    LabelPointerOutsideExecutable {
        /// Data-image-relative label pointer.
        label_offset: usize,
    },
    /// A label has no terminator within its bounded source field.
    MissingLabelTerminator {
        /// Data-image-relative label pointer.
        label_offset: usize,
    },
    /// A label contains a control or non-ASCII byte not accepted by the game font.
    InvalidLabelByte {
        /// Data-image-relative label pointer.
        label_offset: usize,
        /// Invalid byte value.
        byte: u8,
    },
}

impl fmt::Display for BloodprgBridgeMenuTextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid BLOODPRG bridge-menu text: {self:?}")
    }
}

impl Error for BloodprgBridgeMenuTextError {}

/// Decode all bridge options and text-speed labels into flat owned byte strings.
///
/// The two source tables contain data-segment offsets terminated by `0xFFFF`.
/// This loader resolves those serialized offsets once, validates each bounded
/// NUL-terminated label, and exposes no segmented address to runtime game code.
pub fn decode_bloodprg_bridge_menu_text(
    executable: &[u8],
) -> Result<BloodprgBridgeMenuText, BloodprgBridgeMenuTextError> {
    if executable.len() < MENU_TEXT_REQUIRED_EXECUTABLE_LENGTH {
        return Err(BloodprgBridgeMenuTextError::TruncatedExecutable {
            actual: executable.len(),
            required: MENU_TEXT_REQUIRED_EXECUTABLE_LENGTH,
        });
    }
    if executable.get(MZ_SIGNATURE_FILE_OFFSET..MZ_SIGNATURE.len()) != Some(&MZ_SIGNATURE) {
        return Err(BloodprgBridgeMenuTextError::InvalidExecutableSignature);
    }

    let option_labels = decode_menu_pointer_list::<BLOODPRG_OPTION_MENU_LABEL_COUNT>(
        executable,
        OPTION_MENU_POINTER_LIST_DATA_OFFSET,
    )?;
    let text_speed_labels = decode_menu_pointer_list::<BLOODPRG_TEXT_SPEED_LABEL_COUNT>(
        executable,
        TEXT_SPEED_POINTER_LIST_DATA_OFFSET,
    )?;

    Ok(BloodprgBridgeMenuText {
        option_labels,
        music_on_label: decode_menu_label(executable, MUSIC_ON_LABEL_DATA_OFFSET)?,
        text_speed_labels,
        cancel_label: decode_menu_label(executable, LIST_WIDGET_CANCEL_LABEL_DATA_OFFSET)?,
        initial_text_speed_step: read_unsigned_word(
            executable,
            BLOODPRG_DATA_FILE_OFFSET + INITIAL_TEXT_SPEED_STEP_DATA_OFFSET,
        ),
    })
}

fn decode_menu_pointer_list<const LABEL_COUNT: usize>(
    executable: &[u8],
    list_offset: usize,
) -> Result<[Box<[u8]>; LABEL_COUNT], BloodprgBridgeMenuTextError> {
    let list_file_offset = BLOODPRG_DATA_FILE_OFFSET + list_offset;
    let sentinel = read_unsigned_word(executable, list_file_offset + LABEL_COUNT * WORD_BYTE_COUNT);
    if sentinel != POINTER_LIST_SENTINEL {
        return Err(BloodprgBridgeMenuTextError::MissingPointerListSentinel {
            list_offset,
            actual: sentinel,
        });
    }

    let labels = (0..LABEL_COUNT)
        .map(|index| {
            let pointer =
                read_unsigned_word(executable, list_file_offset + index * WORD_BYTE_COUNT);
            decode_menu_label(executable, usize::from(pointer))
        })
        .collect::<Result<Vec<_>, _>>()?;
    labels
        .try_into()
        .map_err(|_| unreachable!("decoded exactly LABEL_COUNT labels"))
}

fn decode_menu_label(
    executable: &[u8],
    label_offset: usize,
) -> Result<Box<[u8]>, BloodprgBridgeMenuTextError> {
    let file_offset = BLOODPRG_DATA_FILE_OFFSET + label_offset;
    let available = executable
        .get(file_offset..)
        .ok_or(BloodprgBridgeMenuTextError::LabelPointerOutsideExecutable { label_offset })?;
    let bounded = &available[..available.len().min(MENU_LABEL_MAXIMUM_BYTE_COUNT + 1)];
    let length = bounded
        .iter()
        .position(|byte| *byte == u8::MIN)
        .ok_or(BloodprgBridgeMenuTextError::MissingLabelTerminator { label_offset })?;
    let label = &bounded[..length];
    if let Some(byte) = label
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_graphic() && *byte != b' ')
    {
        return Err(BloodprgBridgeMenuTextError::InvalidLabelByte { label_offset, byte });
    }
    Ok(label.into())
}

/// One executable-authored presentation-line template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BloodprgPresentationLineDescriptor {
    flags: u8,
    variant: u8,
    resource_name: BloodResourceName,
    initial_scene_image_name: Option<BloodResourceName>,
}

impl BloodprgPresentationLineDescriptor {
    /// Low-byte stream behavior flags used by the presentation queue.
    pub const fn flags(&self) -> u8 {
        self.flags
    }

    /// Initial high-byte stream variant installed before dynamic scene selection.
    pub const fn variant(&self) -> u8 {
        self.variant
    }

    /// Initial HNM resource name, including its authored DOS directory.
    pub const fn resource_name(&self) -> &BloodResourceName {
        &self.resource_name
    }

    /// Initial scene artwork, absent for every entry in the shipped executable.
    pub const fn initial_scene_image_name(&self) -> Option<&BloodResourceName> {
        self.initial_scene_image_name.as_ref()
    }
}

/// Complete executable-authored presentation-line template catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BloodprgPresentationCatalog {
    lines: [BloodprgPresentationLineDescriptor; BLOODPRG_PRESENTATION_LINE_COUNT],
    unclamped_line_ids: [u8; BLOODPRG_UNCLAMPED_PRESENTATION_LINE_COUNT],
}

impl BloodprgPresentationCatalog {
    /// Return every presentation line in native line-number order.
    pub const fn lines(
        &self,
    ) -> &[BloodprgPresentationLineDescriptor; BLOODPRG_PRESENTATION_LINE_COUNT] {
        &self.lines
    }

    /// Resolve one line without native 16-bit table aliasing.
    pub fn line(&self, line: u16) -> Option<&BloodprgPresentationLineDescriptor> {
        self.lines.get(usize::from(line))
    }

    /// Return the first eight executable bytes scanned by the scene dispatcher.
    pub const fn unclamped_line_ids(&self) -> &[u8; BLOODPRG_UNCLAMPED_PRESENTATION_LINE_COUNT] {
        &self.unclamped_line_ids
    }
}

/// Malformed executable presentation-line tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BloodprgPresentationCatalogError {
    /// The input does not begin with an MZ executable signature.
    InvalidExecutableSignature,
    /// The executable ends before the complete index and descriptor area.
    TruncatedExecutable {
        /// Supplied executable byte count.
        actual: usize,
        /// Minimum byte count required by the presentation tables.
        required: usize,
    },
    /// Descriptor slots do not progress through distinct bounded regions.
    InvalidDescriptorRange {
        /// Zero-based presentation line.
        line: usize,
        /// Start relative to the executable data image.
        start: usize,
        /// Exclusive end relative to the executable data image.
        end: usize,
    },
    /// A descriptor's resource name has no terminator inside its own slot.
    MissingResourceNameTerminator {
        /// Zero-based presentation line.
        line: usize,
    },
    /// A descriptor resource name is not a valid original archive name.
    InvalidResourceName {
        /// Zero-based presentation line.
        line: usize,
        /// Exact resource-name validation failure.
        source: BloodArchiveError,
    },
    /// An initial scene-image pointer falls outside the executable data image.
    InvalidSceneImageOffset {
        /// Zero-based presentation line.
        line: usize,
        /// Invalid data-image-relative position.
        offset: usize,
    },
    /// An initial scene-image name has no terminator in its bounded field.
    MissingSceneImageNameTerminator {
        /// Zero-based presentation line.
        line: usize,
    },
    /// An initial scene-image name is not a valid original archive name.
    InvalidSceneImageName {
        /// Zero-based presentation line.
        line: usize,
        /// Exact resource-name validation failure.
        source: BloodArchiveError,
    },
}

impl fmt::Display for BloodprgPresentationCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid BLOODPRG presentation catalog: {self:?}")
    }
}

impl Error for BloodprgPresentationCatalogError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidResourceName { source, .. }
            | Self::InvalidSceneImageName { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Decode every executable-authored presentation-line template into flat owned values.
///
/// The source index contains DOS data-segment offsets, including mutable scene-image
/// pointers. This loader resolves those serialized positions once and does not expose
/// them to runtime game code.
pub fn decode_bloodprg_presentation_catalog(
    executable: &[u8],
) -> Result<BloodprgPresentationCatalog, BloodprgPresentationCatalogError> {
    if executable.len() < PRESENTATION_REQUIRED_EXECUTABLE_LENGTH {
        return Err(BloodprgPresentationCatalogError::TruncatedExecutable {
            actual: executable.len(),
            required: PRESENTATION_REQUIRED_EXECUTABLE_LENGTH,
        });
    }
    if executable.get(MZ_SIGNATURE_FILE_OFFSET..MZ_SIGNATURE.len()) != Some(&MZ_SIGNATURE) {
        return Err(BloodprgPresentationCatalogError::InvalidExecutableSignature);
    }

    let index_file_offset = BLOODPRG_DATA_FILE_OFFSET + PRESENTATION_INDEX_DATA_OFFSET;
    let descriptor_offsets: [usize; BLOODPRG_PRESENTATION_LINE_COUNT] =
        std::array::from_fn(|line| {
            let entry = index_file_offset + line * PRESENTATION_INDEX_ENTRY_BYTE_COUNT;
            usize::from(read_unsigned_word(
                executable,
                entry + PRESENTATION_DESCRIPTOR_OFFSET_FIELD,
            ))
        });
    let scene_image_offsets: [u16; BLOODPRG_PRESENTATION_LINE_COUNT] =
        std::array::from_fn(|line| {
            let entry = index_file_offset + line * PRESENTATION_INDEX_ENTRY_BYTE_COUNT;
            read_unsigned_word(executable, entry + PRESENTATION_SCENE_IMAGE_OFFSET_FIELD)
        });

    let descriptor_data_start = PRESENTATION_INDEX_DATA_OFFSET
        + BLOODPRG_PRESENTATION_LINE_COUNT * PRESENTATION_INDEX_ENTRY_BYTE_COUNT;
    let mut lines = Vec::with_capacity(BLOODPRG_PRESENTATION_LINE_COUNT);
    for line in 0..BLOODPRG_PRESENTATION_LINE_COUNT {
        let start = descriptor_offsets[line];
        let end = descriptor_offsets
            .get(line + 1)
            .copied()
            .unwrap_or(PRESENTATION_DESCRIPTOR_DATA_END_OFFSET);
        if start < descriptor_data_start
            || start
                .checked_add(PRESENTATION_DESCRIPTOR_HEADER_BYTE_COUNT)
                .is_none_or(|header_end| header_end >= end)
            || end > PRESENTATION_DESCRIPTOR_DATA_END_OFFSET
        {
            return Err(BloodprgPresentationCatalogError::InvalidDescriptorRange {
                line,
                start,
                end,
            });
        }

        let descriptor =
            &executable[BLOODPRG_DATA_FILE_OFFSET + start..BLOODPRG_DATA_FILE_OFFSET + end];
        let name_field = &descriptor[PRESENTATION_DESCRIPTOR_HEADER_BYTE_COUNT..];
        let name_length = name_field
            .iter()
            .position(|byte| *byte == u8::MIN)
            .ok_or(BloodprgPresentationCatalogError::MissingResourceNameTerminator { line })?;
        let resource_name =
            BloodResourceName::new(&name_field[..name_length]).map_err(|source| {
                BloodprgPresentationCatalogError::InvalidResourceName { line, source }
            })?;
        let initial_scene_image_name =
            decode_presentation_scene_image_name(executable, line, scene_image_offsets[line])?;
        lines.push(BloodprgPresentationLineDescriptor {
            flags: descriptor[0],
            variant: descriptor[1],
            resource_name,
            initial_scene_image_name,
        });
    }

    Ok(BloodprgPresentationCatalog {
        lines: lines
            .try_into()
            .expect("one descriptor is emitted for every presentation line"),
        unclamped_line_ids: executable[BLOODPRG_DATA_FILE_OFFSET
            + UNCLAMPED_PRESENTATION_LINE_IDS_DATA_OFFSET
            ..BLOODPRG_DATA_FILE_OFFSET
                + UNCLAMPED_PRESENTATION_LINE_IDS_DATA_OFFSET
                + BLOODPRG_UNCLAMPED_PRESENTATION_LINE_COUNT]
            .try_into()
            .expect("the presentation catalog length check covers the line-mode table"),
    })
}

fn decode_presentation_scene_image_name(
    executable: &[u8],
    line: usize,
    data_offset: u16,
) -> Result<Option<BloodResourceName>, BloodprgPresentationCatalogError> {
    if data_offset == NO_PRESENTATION_SCENE_IMAGE_OFFSET {
        return Ok(None);
    }

    let offset = usize::from(data_offset);
    let file_offset = BLOODPRG_DATA_FILE_OFFSET
        .checked_add(offset)
        .ok_or(BloodprgPresentationCatalogError::InvalidSceneImageOffset { line, offset })?;
    let field_end = file_offset
        .checked_add(PRESENTATION_RESOURCE_NAME_MAXIMUM_FIELD_BYTE_COUNT)
        .filter(|end| *end <= executable.len())
        .ok_or(BloodprgPresentationCatalogError::InvalidSceneImageOffset { line, offset })?;
    let field = &executable[file_offset..field_end];
    let name_length = field
        .iter()
        .position(|byte| *byte == u8::MIN)
        .ok_or(BloodprgPresentationCatalogError::MissingSceneImageNameTerminator { line })?;
    BloodResourceName::new(&field[..name_length])
        .map(Some)
        .map_err(|source| BloodprgPresentationCatalogError::InvalidSceneImageName { line, source })
}

/// One world-space navigation anchor decoded from the executable image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BloodprgBridgeAnchor {
    /// Three wrapping source-coordinate components.
    pub position: [u16; POSITION_COMPONENT_COUNT],
}

/// One signed Q14 cosine and sine pair from the executable image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BloodprgBridgeTrigonometrySample {
    /// Cosine at this two-degree step.
    pub cosine: i16,
    /// Sine at this two-degree step.
    pub sine: i16,
}

/// One flat bridge actor record decoded from the executable data image.
///
/// The original program aliases these same 24 bytes as navigation-slot and
/// presentation-line state. Keeping the complete authored record together
/// preserves that relationship without exposing segmented addresses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BloodprgNavActorRecord {
    /// Shared low-byte navigation and presentation flags.
    pub flags: u8,
    /// Primary sprite resource catalog identity.
    pub resource_id: u16,
    /// Optional transition sprite resource catalog identity.
    pub transition_resource_id: Option<u16>,
    /// Initial terminal animation frame.
    pub terminal_frame: u16,
    /// Initial frame selected for presentation.
    pub frame: u16,
    /// Navigation target in the doubled bridge-frame coordinate domain.
    pub target_arc: u16,
    /// Persistent authored mouse-hit rectangle.
    pub hit_rectangle: BloodprgHitRectangle,
    /// Logical sprite draw position.
    pub draw_position: [u16; 2],
}

/// Complete bridge projection resources decoded from `BLOODPRG.EXE`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BloodprgBridgeResources {
    /// Eleven projector inputs, including the recovered final overlapping read.
    pub projection_anchors: [BloodprgBridgeAnchor; BLOODPRG_BRIDGE_PROJECTION_ANCHOR_COUNT],
    /// Complete authored two-degree angle table.
    pub trigonometry: [BloodprgBridgeTrigonometrySample; BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT],
    /// Six records shared by bridge navigation and actor presentation logic.
    pub nav_actor_records: [BloodprgNavActorRecord; BLOODPRG_NAV_ACTOR_RECORD_COUNT],
}

/// One executable-authored logical hit rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BloodprgHitRectangle {
    /// Upper-left logical coordinate.
    pub origin: [i16; 2],
    /// Signed logical width and height consumed by the native hit tester.
    pub size: [i16; 2],
}

/// YES and NO hit regions used by the navigation confirmation modal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BloodprgConfirmDialogRegions {
    /// Affirmative response region.
    pub yes: BloodprgHitRectangle,
    /// Negative response region.
    pub no: BloodprgHitRectangle,
}

/// Complete executable font maps, advances, and glyph bitmaps for either game.
/// Variable-sized tables retain their exact authored size, without padding a
/// Commander table or truncating the sequel's additional characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BloodprgFontResources {
    /// Byte-to-glyph map used by the compact five-row font.
    pub small_character_map: [u8; BLOODPRG_SMALL_FONT_CHARACTER_COUNT],
    /// Five one-byte rows for every compact glyph.
    pub small_glyphs: [u8; SMALL_FONT_GLYPH_BYTE_COUNT],
    /// Byte-to-glyph map used by the fixed-width subtitle console font.
    pub subtitle_character_map: Box<[u8]>,
    /// Eight one-byte rows for every subtitle glyph.
    pub subtitle_glyphs: Box<[u8]>,
    /// Byte-to-glyph map used by the square-cap UI font.
    pub square_caps_character_map: Box<[u8]>,
    /// Signed-byte pen advances indexed by square-cap glyph number.
    pub square_caps_advances: Box<[u8]>,
    /// Complete unsigned lookup region consumed by dual-font measurement.
    pub square_caps_measurement_advances: [u8; BLOODPRG_FONT_MEASUREMENT_ADVANCE_COUNT],
    /// Ten big-endian two-byte rows for every square-cap glyph.
    pub square_caps_glyphs: Box<[u8]>,
    /// Byte-to-glyph map used by the main dialogue font.
    pub main_character_map: Box<[u8]>,
    /// Signed-byte pen advances indexed by main-font glyph number.
    pub main_advances: Box<[u8]>,
    /// Complete unsigned lookup region consumed by dual-font measurement.
    pub main_measurement_advances: [u8; BLOODPRG_FONT_MEASUREMENT_ADVANCE_COUNT],
    /// Eight one-byte rows for every main-font glyph.
    pub main_glyphs: Box<[u8]>,
}

/// Malformed or truncated executable font resources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BloodprgFontResourceError {
    /// The input does not begin with an MZ executable signature.
    InvalidExecutableSignature,
    /// A recovered font range extends beyond the supplied executable image.
    TruncatedExecutable {
        /// Supplied executable byte count.
        actual: usize,
        /// Minimum byte count required by every font table.
        required: usize,
    },
}

impl fmt::Display for BloodprgFontResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid BLOODPRG font resources: {self:?}")
    }
}

impl Error for BloodprgFontResourceError {}

/// Malformed or truncated `BLOODPRG.EXE` bridge resources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BloodprgBridgeResourceError {
    /// The input does not begin with an MZ executable signature.
    InvalidExecutableSignature,
    /// A fixed bridge resource range extends beyond the supplied image.
    TruncatedExecutable {
        /// Supplied executable byte count.
        actual: usize,
        /// Minimum byte count required by all decoded ranges.
        required: usize,
    },
}

impl fmt::Display for BloodprgBridgeResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid BLOODPRG bridge resources: {self:?}")
    }
}

impl Error for BloodprgBridgeResourceError {}

/// Malformed or truncated confirmation-dialog resources in `BLOODPRG.EXE`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BloodprgConfirmDialogResourceError {
    /// The input does not begin with an MZ executable signature.
    InvalidExecutableSignature,
    /// One of the two fixed rectangle records extends beyond the image.
    TruncatedExecutable {
        /// Supplied executable byte count.
        actual: usize,
        /// Minimum byte count required by both records.
        required: usize,
    },
}

impl fmt::Display for BloodprgConfirmDialogResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid BLOODPRG confirmation-dialog resources: {self:?}"
        )
    }
}

impl Error for BloodprgConfirmDialogResourceError {}

/// Decode the authored YES and NO hit rectangles into flat logical geometry.
pub fn decode_bloodprg_confirm_dialog_regions(
    executable: &[u8],
) -> Result<BloodprgConfirmDialogRegions, BloodprgConfirmDialogResourceError> {
    if executable.len() < CONFIRM_DIALOG_REQUIRED_EXECUTABLE_LENGTH {
        return Err(BloodprgConfirmDialogResourceError::TruncatedExecutable {
            actual: executable.len(),
            required: CONFIRM_DIALOG_REQUIRED_EXECUTABLE_LENGTH,
        });
    }
    if executable.get(MZ_SIGNATURE_FILE_OFFSET..MZ_SIGNATURE.len()) != Some(&MZ_SIGNATURE) {
        return Err(BloodprgConfirmDialogResourceError::InvalidExecutableSignature);
    }

    Ok(BloodprgConfirmDialogRegions {
        yes: read_hit_rectangle(executable, CONFIRM_DIALOG_YES_REGION_FILE_OFFSET),
        no: read_hit_rectangle(executable, CONFIRM_DIALOG_NO_REGION_FILE_OFFSET),
    })
}

/// Decode bridge projection anchors and trigonometry into owned arrays.
///
/// The ten authored anchors end at the angle table. The original projector
/// consumes eleven records, so its final six-byte input is decoded from the
/// beginning of that adjacent table. This overlap is resolved here once; game
/// code receives independent typed arrays and never handles executable offsets.
pub fn decode_bloodprg_bridge_resources(
    executable: &[u8],
) -> Result<BloodprgBridgeResources, BloodprgBridgeResourceError> {
    decode_bridge_resources_at(
        executable,
        PROJECTION_ANCHOR_FILE_OFFSET,
        TRIGONOMETRY_FILE_OFFSET,
        NAV_ACTOR_RECORDS_FILE_OFFSET,
    )
}

/// Decode bridge tables for the analyzed `BLOOD2PG.EXE` layout.
/// The caller must identify the executable build before choosing this decoder.
pub fn decode_blood2pg_bridge_resources(
    executable: &[u8],
) -> Result<BloodprgBridgeResources, BloodprgBridgeResourceError> {
    decode_bridge_resources_at(
        executable,
        BLOOD2PG_PROJECTION_ANCHOR_FILE_OFFSET,
        BLOOD2PG_TRIGONOMETRY_FILE_OFFSET,
        BLOOD2PG_NAV_ACTOR_RECORDS_FILE_OFFSET,
    )
}

fn decode_bridge_resources_at(
    executable: &[u8],
    anchor_base: usize,
    trig_base: usize,
    actors_base: usize,
) -> Result<BloodprgBridgeResources, BloodprgBridgeResourceError> {
    let required = (anchor_base
        + BLOODPRG_BRIDGE_PROJECTION_ANCHOR_COUNT * PROJECTION_ANCHOR_BYTE_COUNT)
        .max(trig_base + BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT * TRIGONOMETRY_SAMPLE_BYTE_COUNT)
        .max(actors_base + BLOODPRG_NAV_ACTOR_RECORD_COUNT * NAV_ACTOR_RECORD_BYTE_COUNT);
    if executable.len() < required {
        return Err(BloodprgBridgeResourceError::TruncatedExecutable {
            actual: executable.len(),
            required,
        });
    }
    if executable.get(MZ_SIGNATURE_FILE_OFFSET..MZ_SIGNATURE.len()) != Some(&MZ_SIGNATURE) {
        return Err(BloodprgBridgeResourceError::InvalidExecutableSignature);
    }

    let mut projection_anchors =
        [BloodprgBridgeAnchor::default(); BLOODPRG_BRIDGE_PROJECTION_ANCHOR_COUNT];
    for (index, anchor) in projection_anchors.iter_mut().enumerate() {
        let position = anchor_base + index * PROJECTION_ANCHOR_BYTE_COUNT;
        anchor.position = std::array::from_fn(|component| {
            read_unsigned_word(executable, position + component * WORD_BYTE_COUNT)
        });
    }

    let mut trigonometry =
        [BloodprgBridgeTrigonometrySample::default(); BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT];
    for (index, sample) in trigonometry.iter_mut().enumerate() {
        let position = trig_base + index * TRIGONOMETRY_SAMPLE_BYTE_COUNT;
        sample.cosine = read_signed_word(executable, position);
        sample.sine = read_signed_word(executable, position + WORD_BYTE_COUNT);
    }

    let nav_actor_records = std::array::from_fn(|index| {
        let position = actors_base + index * NAV_ACTOR_RECORD_BYTE_COUNT;
        let transition_resource_id = read_unsigned_word(
            executable,
            position + NAV_ACTOR_TRANSITION_RESOURCE_FIELD_OFFSET,
        );
        BloodprgNavActorRecord {
            flags: executable[position + NAV_ACTOR_FLAGS_FIELD_OFFSET],
            resource_id: read_unsigned_word(executable, position + NAV_ACTOR_RESOURCE_FIELD_OFFSET),
            transition_resource_id: (transition_resource_id != NO_NAV_ACTOR_TRANSITION_RESOURCE)
                .then_some(transition_resource_id),
            terminal_frame: read_unsigned_word(
                executable,
                position + NAV_ACTOR_TERMINAL_FRAME_FIELD_OFFSET,
            ),
            frame: read_unsigned_word(executable, position + NAV_ACTOR_FRAME_FIELD_OFFSET),
            target_arc: read_unsigned_word(
                executable,
                position + NAV_ACTOR_TARGET_ARC_FIELD_OFFSET,
            ),
            hit_rectangle: read_hit_rectangle(
                executable,
                position + NAV_ACTOR_HIT_RECTANGLE_FIELD_OFFSET,
            ),
            draw_position: std::array::from_fn(|component| {
                read_unsigned_word(
                    executable,
                    position + NAV_ACTOR_DRAW_POSITION_FIELD_OFFSET + component * WORD_BYTE_COUNT,
                )
            }),
        }
    });

    Ok(BloodprgBridgeResources {
        projection_anchors,
        trigonometry,
        nav_actor_records,
    })
}

/// Decode every recovered font table into owned flat arrays.
///
/// Executable positions are resolved once at load time. Render and measurement
/// code receives ordinary arrays and does not retain DOS data-segment offsets
/// or rely on adjacency between unrelated native tables.
pub fn decode_bloodprg_font_resources(
    executable: &[u8],
) -> Result<BloodprgFontResources, BloodprgFontResourceError> {
    decode_font_resources_at(executable, &COMMANDER_FONT_LAYOUT)
}

/// Decode the sequel's expanded font maps, glyphs, and native width lookups.
/// The caller must identify the executable build before choosing this decoder.
pub fn decode_blood2pg_font_resources(
    executable: &[u8],
) -> Result<BloodprgFontResources, BloodprgFontResourceError> {
    decode_font_resources_at(executable, &SEQUEL_FONT_LAYOUT)
}

fn decode_font_resources_at(
    executable: &[u8],
    layout: &ExecutableFontLayout,
) -> Result<BloodprgFontResources, BloodprgFontResourceError> {
    if executable.len() < layout.required_length {
        return Err(BloodprgFontResourceError::TruncatedExecutable {
            actual: executable.len(),
            required: layout.required_length,
        });
    }
    if executable.get(MZ_SIGNATURE_FILE_OFFSET..MZ_SIGNATURE.len()) != Some(&MZ_SIGNATURE) {
        return Err(BloodprgFontResourceError::InvalidExecutableSignature);
    }

    let owned = |start, count| Box::from(&executable[start..start + count]);
    Ok(BloodprgFontResources {
        small_character_map: read_byte_array(executable, layout.small_map),
        small_glyphs: read_byte_array(executable, layout.small_glyphs),
        subtitle_character_map: owned(layout.subtitle_map, layout.proportional_map_count),
        subtitle_glyphs: owned(
            layout.subtitle_glyphs,
            layout.subtitle_glyph_count * SUBTITLE_FONT_GLYPH_HEIGHT,
        ),
        square_caps_character_map: owned(layout.square_map, layout.proportional_map_count),
        square_caps_advances: owned(layout.square_advances, layout.square_glyph_count),
        square_caps_measurement_advances: read_byte_array(executable, layout.square_advances),
        square_caps_glyphs: owned(
            layout.square_glyphs,
            layout.square_glyph_count * SQUARE_CAPS_GLYPH_HEIGHT * SQUARE_CAPS_ROW_BYTE_COUNT,
        ),
        main_character_map: owned(layout.main_map, layout.proportional_map_count),
        main_advances: owned(layout.main_advances, layout.main_glyph_count),
        main_measurement_advances: read_byte_array(executable, layout.main_advances),
        main_glyphs: owned(
            layout.main_glyphs,
            layout.main_glyph_count * MAIN_FONT_GLYPH_HEIGHT,
        ),
    })
}

fn read_byte_array<const BYTE_COUNT: usize>(data: &[u8], position: usize) -> [u8; BYTE_COUNT] {
    data[position..position + BYTE_COUNT]
        .try_into()
        .expect("validated BLOODPRG font range")
}

fn read_unsigned_word(data: &[u8], position: usize) -> u16 {
    u16::from_le_bytes([data[position], data[position + 1]])
}

fn read_signed_word(data: &[u8], position: usize) -> i16 {
    i16::from_le_bytes([data[position], data[position + 1]])
}

fn read_hit_rectangle(data: &[u8], position: usize) -> BloodprgHitRectangle {
    let components: [i16; RECTANGLE_COMPONENT_COUNT] = std::array::from_fn(|component| {
        read_signed_word(data, position + component * WORD_BYTE_COUNT)
    });
    BloodprgHitRectangle {
        origin: [components[0], components[1]],
        size: [components[2], components[3]],
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const RESOURCE_ORACLE_COUNT: usize = 1;
    const SHIPPED_UNCLAMPED_PRESENTATION_LINES: [u8; BLOODPRG_UNCLAMPED_PRESENTATION_LINE_COUNT] =
        [41, 42, 0, 1, 4, 5, 6, 44];

    #[derive(Deserialize)]
    struct BridgeResourceOracle {
        data_file_offset: usize,
        projection_anchor_offset: usize,
        authored_anchor_count: usize,
        projection_anchor_count: usize,
        anchors: Vec<[u16; POSITION_COMPONENT_COUNT]>,
        trigonometry_offset: usize,
        trigonometry_count: usize,
        trigonometry: Vec<[i16; TRIGONOMETRY_COMPONENT_COUNT]>,
    }

    #[test]
    fn bridge_resources_match_every_original_executable_value() {
        let vectors: Vec<BridgeResourceOracle> = serde_json::from_str(include_str!(
            "../../../re/tools/oracle_vectors/bloodprg_bridge_resources.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), RESOURCE_ORACLE_COUNT);

        for vector in vectors {
            assert_eq!(vector.data_file_offset, BLOODPRG_DATA_FILE_OFFSET);
            assert_eq!(
                vector.projection_anchor_offset,
                BRIDGE_PROJECTION_ANCHOR_DATA_OFFSET
            );
            assert_eq!(
                vector.authored_anchor_count,
                BLOODPRG_BRIDGE_AUTHORED_ANCHOR_COUNT
            );
            assert_eq!(
                vector.projection_anchor_count,
                BLOODPRG_BRIDGE_PROJECTION_ANCHOR_COUNT
            );
            assert_eq!(vector.trigonometry_offset, BRIDGE_TRIGONOMETRY_DATA_OFFSET);
            assert_eq!(
                vector.trigonometry_count,
                BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT
            );

            let executable = executable_fixture(&vector);
            let resources = decode_bloodprg_bridge_resources(&executable).unwrap();
            assert_eq!(
                resources
                    .projection_anchors
                    .map(|anchor| anchor.position)
                    .as_slice(),
                vector.anchors
            );
            assert_eq!(
                resources
                    .trigonometry
                    .map(|sample| [sample.cosine, sample.sine])
                    .as_slice(),
                vector.trigonometry
            );
        }
    }

    #[test]
    fn bridge_actor_records_match_the_original_executable_table() {
        let executable = include_bytes!("../../../re/bin/BLOODPRG.EXE");
        let records = decode_bloodprg_bridge_resources(executable)
            .unwrap()
            .nav_actor_records;

        assert_eq!(
            records,
            [
                actor_record(1, 17, None, 0, [0, 0, 0, 0], [132, 108]),
                actor_record(1, 13, None, 90, [0, 0, 0, 0], [110, 42]),
                actor_record(9, 15, None, 180, [0, 0, 0, 0], [137, 139]),
                actor_record(1, 16, None, 270, [0, 0, 0, 0], [156, 63]),
                actor_record(0, 19, Some(21), 0, [18, 111, 96, 47], [17, 104]),
                actor_record(0, 18, Some(20), 0, [215, 112, 75, 27], [195, 110]),
            ]
        );
    }

    #[test]
    fn navigation_resources_match_initialized_executable_data() {
        let executable = include_bytes!("../../../re/bin/BLOODPRG.EXE");
        let resources = decode_bloodprg_navigation_resources(executable).unwrap();

        assert_eq!(
            resources.wipe_endpoints(),
            &[
                [160, 0],
                [140, 0],
                [120, 0],
                [60, 0],
                [0, 0],
                [0, 50],
                [0, 90],
                [0, 130],
                [0, 190],
            ]
        );
        assert_eq!(resources.labels().planet(), b"PLANET: ");
        assert_eq!(resources.labels().ship(), b"SHIP: ");
        assert_eq!(resources.labels().black_hole(), b"BLACK HOLE: ");
        assert_eq!(resources.labels().life_support(), b"LIFE SUPPORT:");
    }

    #[test]
    fn malformed_navigation_resources_are_rejected_before_use() {
        let mut executable = include_bytes!("../../../re/bin/BLOODPRG.EXE").to_vec();
        let planet_start = BLOODPRG_DATA_FILE_OFFSET + NAVIGATION_PLANET_LABEL_DATA_OFFSET;
        let planet_end = BLOODPRG_DATA_FILE_OFFSET + NAVIGATION_SHIP_LABEL_DATA_OFFSET;
        executable[planet_start..planet_end].fill(b'P');
        assert_eq!(
            decode_bloodprg_navigation_resources(&executable),
            Err(BloodprgNavigationResourcesError::MissingLabelTerminator {
                label_offset: NAVIGATION_PLANET_LABEL_DATA_OFFSET,
            })
        );

        let mut executable = include_bytes!("../../../re/bin/BLOODPRG.EXE").to_vec();
        executable[planet_start] = 1;
        assert_eq!(
            decode_bloodprg_navigation_resources(&executable),
            Err(BloodprgNavigationResourcesError::InvalidLabelByte {
                label_offset: NAVIGATION_PLANET_LABEL_DATA_OFFSET,
                byte: 1,
            })
        );
    }

    #[test]
    fn malformed_executables_are_rejected_before_decoding() {
        assert_eq!(
            decode_bloodprg_bridge_resources(&[]),
            Err(BloodprgBridgeResourceError::TruncatedExecutable {
                actual: usize::MIN,
                required: REQUIRED_EXECUTABLE_LENGTH,
            })
        );

        let truncated = vec![u8::MIN; REQUIRED_EXECUTABLE_LENGTH - 1];
        assert_eq!(
            decode_bloodprg_bridge_resources(&truncated),
            Err(BloodprgBridgeResourceError::TruncatedExecutable {
                actual: REQUIRED_EXECUTABLE_LENGTH - 1,
                required: REQUIRED_EXECUTABLE_LENGTH,
            })
        );

        let invalid_signature = vec![u8::MIN; REQUIRED_EXECUTABLE_LENGTH];
        assert_eq!(
            decode_bloodprg_bridge_resources(&invalid_signature),
            Err(BloodprgBridgeResourceError::InvalidExecutableSignature)
        );
    }

    #[test]
    fn executable_font_tables_decode_into_independent_owned_arrays() {
        let executable = include_bytes!("../../../re/bin/BLOODPRG.EXE");
        let resources = decode_bloodprg_font_resources(executable).unwrap();

        assert_eq!(
            resources.square_caps_advances.as_ref(),
            &resources.square_caps_measurement_advances[..BLOODPRG_SQUARE_CAPS_GLYPH_COUNT]
        );
        assert_eq!(
            resources.main_advances.as_ref(),
            &resources.main_measurement_advances[..BLOODPRG_MAIN_FONT_GLYPH_COUNT]
        );
        assert_ne!(
            resources.small_glyphs,
            [u8::MIN; SMALL_FONT_GLYPH_BYTE_COUNT]
        );
        assert_ne!(
            resources.subtitle_glyphs.as_ref(),
            &[u8::MIN; SUBTITLE_FONT_GLYPH_BYTE_COUNT]
        );
        assert_ne!(
            resources.square_caps_glyphs.as_ref(),
            &[u8::MIN; SQUARE_CAPS_GLYPH_BYTE_COUNT]
        );
        assert_ne!(
            resources.main_glyphs.as_ref(),
            &[u8::MIN; MAIN_FONT_GLYPH_BYTE_COUNT]
        );
    }

    #[test]
    fn malformed_font_executables_are_rejected_before_decoding() {
        assert_eq!(
            decode_bloodprg_font_resources(&[]),
            Err(BloodprgFontResourceError::TruncatedExecutable {
                actual: usize::MIN,
                required: FONT_REQUIRED_EXECUTABLE_LENGTH,
            })
        );

        let truncated = vec![u8::MIN; FONT_REQUIRED_EXECUTABLE_LENGTH - 1];
        assert_eq!(
            decode_bloodprg_font_resources(&truncated),
            Err(BloodprgFontResourceError::TruncatedExecutable {
                actual: FONT_REQUIRED_EXECUTABLE_LENGTH - 1,
                required: FONT_REQUIRED_EXECUTABLE_LENGTH,
            })
        );

        let invalid_signature = vec![u8::MIN; FONT_REQUIRED_EXECUTABLE_LENGTH];
        assert_eq!(
            decode_bloodprg_font_resources(&invalid_signature),
            Err(BloodprgFontResourceError::InvalidExecutableSignature)
        );
    }

    #[test]
    fn sequel_font_decoder_rejects_short_or_non_executable_inputs() {
        let required = SEQUEL_FONT_LAYOUT.required_length;
        for count in [0, required - 1] {
            assert_eq!(
                decode_blood2pg_font_resources(&vec![0; count]),
                Err(BloodprgFontResourceError::TruncatedExecutable {
                    actual: count,
                    required
                })
            );
        }
        assert_eq!(
            decode_blood2pg_font_resources(&vec![0; required]),
            Err(BloodprgFontResourceError::InvalidExecutableSignature)
        );
    }

    #[test]
    fn sequel_bridge_decoder_checks_the_complete_relocated_range() {
        let required = BLOOD2PG_TRIGONOMETRY_FILE_OFFSET
            + BLOODPRG_BRIDGE_TRIGONOMETRY_SAMPLE_COUNT * TRIGONOMETRY_SAMPLE_BYTE_COUNT;
        for count in [0, required - 1] {
            assert_eq!(
                decode_blood2pg_bridge_resources(&vec![0; count]),
                Err(BloodprgBridgeResourceError::TruncatedExecutable {
                    actual: count,
                    required
                })
            );
        }
        assert_eq!(
            decode_blood2pg_bridge_resources(&vec![0; required]),
            Err(BloodprgBridgeResourceError::InvalidExecutableSignature)
        );
    }

    #[test]
    #[ignore = "requires the original Big Bug Bang executable under output/big-bug-bang/disc"]
    fn sequel_font_tables_include_every_authored_map_entry_and_glyph() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/disc/BLOOD2PG.EXE");
        let executable = std::fs::read(path).unwrap();
        let fonts = decode_blood2pg_font_resources(&executable).unwrap();
        let commander =
            decode_bloodprg_font_resources(include_bytes!("../../../re/bin/BLOODPRG.EXE")).unwrap();
        assert_eq!(fonts.small_character_map, commander.small_character_map);
        assert_eq!(fonts.small_glyphs, commander.small_glyphs);
        for (map, glyphs, glyph_count, glyph_bytes, map_file, glyph_file) in [
            (
                &fonts.subtitle_character_map,
                &fonts.subtitle_glyphs,
                66,
                8,
                0x171F0,
                0x172D8,
            ),
            (
                &fonts.square_caps_character_map,
                &fonts.square_caps_glyphs,
                49,
                20,
                0x174E8,
                0x17602,
            ),
            (
                &fonts.main_character_map,
                &fonts.main_glyphs,
                87,
                8,
                0x179D6,
                0x17B16,
            ),
        ] {
            assert_eq!(map.len(), 232);
            assert_eq!(glyphs.len(), glyph_count * glyph_bytes);
            assert_eq!(map.as_ref(), &executable[map_file..map_file + 232]);
            assert_eq!(
                glyphs.as_ref(),
                &executable[glyph_file..glyph_file + glyph_count * glyph_bytes]
            );
            assert_eq!(
                map.iter().copied().filter(|&index| index != 255).max(),
                Some((glyph_count - 1) as u8)
            );
            assert!(
                map.iter()
                    .all(|&index| index == 255 || usize::from(index) < glyph_count)
            );
        }
        assert_eq!(
            fonts.square_caps_advances.as_ref(),
            &executable[0x175D0..0x175D0 + 49]
        );
        assert_eq!(
            fonts.main_advances.as_ref(),
            &executable[0x17ABE..0x17ABE + 87]
        );
        assert_eq!(
            fonts.square_caps_measurement_advances,
            executable[0x175D0..0x175D0 + 256]
        );
        assert_eq!(
            fonts.main_measurement_advances,
            executable[0x17ABE..0x17ABE + 256]
        );
        assert_eq!(fonts.main_advances[69], 5);
        assert_eq!(fonts.main_advances[71], 8);
        assert_eq!(commander.main_advances[69], 8);
        assert_eq!(commander.main_advances[71], 5);
        assert_eq!(
            &fonts.subtitle_glyphs[..commander.subtitle_glyphs.len()],
            commander.subtitle_glyphs.as_ref()
        );
        assert_eq!(
            &fonts.square_caps_glyphs[..commander.square_caps_glyphs.len()],
            commander.square_caps_glyphs.as_ref()
        );
        assert_eq!(
            &fonts.main_glyphs[..commander.main_glyphs.len()],
            commander.main_glyphs.as_ref()
        );
    }

    #[test]
    #[ignore = "requires the original Big Bug Bang executable under output/big-bug-bang/disc"]
    fn sequel_bridge_tables_match_native_layout_without_commander_resource_ids() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/disc/BLOOD2PG.EXE");
        let executable = std::fs::read(path).unwrap();
        let sequel = decode_blood2pg_bridge_resources(&executable).unwrap();
        let commander =
            decode_bloodprg_bridge_resources(include_bytes!("../../../re/bin/BLOODPRG.EXE"))
                .unwrap();
        assert_eq!(sequel.projection_anchors, commander.projection_anchors);
        assert_eq!(sequel.trigonometry, commander.trigonometry);
        assert_eq!(
            sequel.nav_actor_records.map(|actor| actor.resource_id),
            [17, 13, 15, 16, 19, 18]
        );
        assert_eq!(
            sequel.nav_actor_records.map(|actor| actor.flags),
            [1, 1, 9, 1, 0, 0]
        );
        assert_eq!(
            sequel.nav_actor_records.map(|actor| actor.target_arc),
            [0, 90, 180, 270, 0, 0]
        );
        for (index, actor) in sequel.nav_actor_records.iter().enumerate() {
            let raw = &executable[0x124AB + index * 24..0x124AB + (index + 1) * 24];
            let word = |offset| u16::from_le_bytes([raw[offset], raw[offset + 1]]);
            assert_eq!(
                actor.transition_resource_id,
                (word(4) != u16::MAX).then_some(word(4))
            );
            assert_eq!(actor.terminal_frame, word(6));
            assert_eq!(actor.frame, word(8));
            assert_eq!(
                actor.hit_rectangle.origin,
                [word(12) as i16, word(14) as i16]
            );
            assert_eq!(actor.hit_rectangle.size, [word(16) as i16, word(18) as i16]);
            assert_eq!(actor.draw_position, [word(20), word(22)]);
        }
    }

    #[test]
    fn confirmation_dialog_regions_match_the_shipped_executable() {
        let executable = include_bytes!("../../../re/bin/BLOODPRG.EXE");
        let regions = decode_bloodprg_confirm_dialog_regions(executable).unwrap();
        assert_eq!(
            regions.yes,
            BloodprgHitRectangle {
                origin: [120, 105],
                size: [30, 10],
            }
        );
        assert_eq!(
            regions.no,
            BloodprgHitRectangle {
                origin: [180, 105],
                size: [20, 10],
            }
        );
    }

    #[test]
    fn malformed_confirmation_dialog_images_are_rejected() {
        assert_eq!(
            decode_bloodprg_confirm_dialog_regions(&[]),
            Err(BloodprgConfirmDialogResourceError::TruncatedExecutable {
                actual: usize::MIN,
                required: CONFIRM_DIALOG_REQUIRED_EXECUTABLE_LENGTH,
            })
        );

        let truncated = vec![u8::MIN; CONFIRM_DIALOG_REQUIRED_EXECUTABLE_LENGTH - 1];
        assert_eq!(
            decode_bloodprg_confirm_dialog_regions(&truncated),
            Err(BloodprgConfirmDialogResourceError::TruncatedExecutable {
                actual: CONFIRM_DIALOG_REQUIRED_EXECUTABLE_LENGTH - 1,
                required: CONFIRM_DIALOG_REQUIRED_EXECUTABLE_LENGTH,
            })
        );

        let invalid_signature = vec![u8::MIN; CONFIRM_DIALOG_REQUIRED_EXECUTABLE_LENGTH];
        assert_eq!(
            decode_bloodprg_confirm_dialog_regions(&invalid_signature),
            Err(BloodprgConfirmDialogResourceError::InvalidExecutableSignature)
        );
    }

    #[test]
    fn presentation_catalog_matches_every_shipped_line_template() {
        let executable = include_bytes!("../../../re/bin/BLOODPRG.EXE");
        let catalog = decode_bloodprg_presentation_catalog(executable).unwrap();
        assert_eq!(catalog.lines().len(), BLOODPRG_PRESENTATION_LINE_COUNT);
        assert!(catalog.lines().iter().all(|line| line.flags() == 0));
        assert!(catalog.lines().iter().all(|line| line.variant() == 16));
        assert!(
            catalog
                .lines()
                .iter()
                .all(|line| line.initial_scene_image_name().is_none())
        );
        assert_eq!(
            catalog.unclamped_line_ids(),
            &SHIPPED_UNCLAMPED_PRESENTATION_LINES
        );

        let first_names: [&[u8]; 8] = [
            b"sq\\mind.HNM",
            b"sq\\the_star.HNM",
            b"sq\\xxxxxxxxxxxx",
            b"pl\\xxxxxxxxxxxx",
            b"sq\\ejectorx.HNM",
            b"sq\\ejection.HNM",
            b"sq\\xxxxxxxxxxxx",
            b"sq\\xxxxxxxxxxxx",
        ];
        for (line, expected) in first_names.into_iter().enumerate() {
            assert_eq!(catalog.lines()[line].resource_name().as_bytes(), expected);
        }
        for line in 8..=40 {
            assert_eq!(
                catalog.lines()[line].resource_name().as_bytes(),
                b"pe\\xxxxxxxxxxxx"
            );
        }
        let final_names: [&[u8]; 4] = [
            b"sq\\cryogel.hnm",
            b"sq\\cryorad.hnm",
            b"ob\\xxxxxxxxxxxx",
            b"sq\\pollup.hnm",
        ];
        for (line, expected) in (41..BLOODPRG_PRESENTATION_LINE_COUNT).zip(final_names) {
            assert_eq!(catalog.lines()[line].resource_name().as_bytes(), expected);
        }
    }

    #[test]
    fn presentation_catalog_resolves_scene_names_without_retaining_offsets() {
        let mut executable = include_bytes!("../../../re/bin/BLOODPRG.EXE").to_vec();
        let first_entry = BLOODPRG_DATA_FILE_OFFSET + PRESENTATION_INDEX_DATA_OFFSET;
        let first_name_data_offset = 0x206B_u16;
        executable[first_entry + PRESENTATION_SCENE_IMAGE_OFFSET_FIELD
            ..first_entry + PRESENTATION_SCENE_IMAGE_OFFSET_FIELD + WORD_BYTE_COUNT]
            .copy_from_slice(&first_name_data_offset.to_le_bytes());

        let catalog = decode_bloodprg_presentation_catalog(&executable).unwrap();
        assert_eq!(
            catalog.lines()[0]
                .initial_scene_image_name()
                .unwrap()
                .as_bytes(),
            b"sq\\mind.HNM"
        );
    }

    #[test]
    fn malformed_presentation_catalogs_fail_inside_bounded_fields() {
        assert_eq!(
            decode_bloodprg_presentation_catalog(&[]),
            Err(BloodprgPresentationCatalogError::TruncatedExecutable {
                actual: 0,
                required: PRESENTATION_REQUIRED_EXECUTABLE_LENGTH,
            })
        );

        let mut invalid_signature = vec![u8::MIN; PRESENTATION_REQUIRED_EXECUTABLE_LENGTH];
        assert_eq!(
            decode_bloodprg_presentation_catalog(&invalid_signature),
            Err(BloodprgPresentationCatalogError::InvalidExecutableSignature)
        );

        invalid_signature[..MZ_SIGNATURE.len()].copy_from_slice(&MZ_SIGNATURE);
        let first_entry = BLOODPRG_DATA_FILE_OFFSET + PRESENTATION_INDEX_DATA_OFFSET;
        let first_descriptor = usize::from(read_unsigned_word(
            &invalid_signature,
            first_entry + PRESENTATION_DESCRIPTOR_OFFSET_FIELD,
        ));
        assert_eq!(
            decode_bloodprg_presentation_catalog(&invalid_signature),
            Err(BloodprgPresentationCatalogError::InvalidDescriptorRange {
                line: 0,
                start: first_descriptor,
                end: first_descriptor,
            })
        );

        let mut missing_terminator = include_bytes!("../../../re/bin/BLOODPRG.EXE").to_vec();
        let first_descriptor = PRESENTATION_INDEX_DATA_OFFSET
            + BLOODPRG_PRESENTATION_LINE_COUNT * PRESENTATION_INDEX_ENTRY_BYTE_COUNT;
        let second_descriptor = usize::from(read_unsigned_word(
            &missing_terminator,
            first_entry + PRESENTATION_INDEX_ENTRY_BYTE_COUNT,
        ));
        missing_terminator[BLOODPRG_DATA_FILE_OFFSET
            + first_descriptor
            + PRESENTATION_DESCRIPTOR_HEADER_BYTE_COUNT
            ..BLOODPRG_DATA_FILE_OFFSET + second_descriptor]
            .fill(b'x');
        assert_eq!(
            decode_bloodprg_presentation_catalog(&missing_terminator),
            Err(BloodprgPresentationCatalogError::MissingResourceNameTerminator { line: 0 })
        );
    }

    #[test]
    fn bridge_menu_text_matches_the_shipped_executable() {
        let executable = include_bytes!("../../../re/bin/BLOODPRG.EXE");
        let text = decode_bloodprg_bridge_menu_text(executable).unwrap();
        let option_labels = text
            .option_labels()
            .iter()
            .map(Box::as_ref)
            .collect::<Vec<_>>();
        let speed_labels = text
            .text_speed_labels()
            .iter()
            .map(Box::as_ref)
            .collect::<Vec<_>>();

        assert_eq!(
            option_labels,
            [
                b"TEXT".as_slice(),
                b"MUSIC_OFF".as_slice(),
                b"SAVE".as_slice(),
                b"LOAD".as_slice(),
                b"QUIT".as_slice(),
            ]
        );
        assert_eq!(text.music_on_label(), b"MUSIC_ON");
        assert_eq!(
            speed_labels,
            [
                b"VERY FAST".as_slice(),
                b"FAST".as_slice(),
                b"MEDIUM".as_slice(),
                b"SLOW".as_slice(),
                b"VERY SLOW".as_slice(),
            ]
        );
        assert_eq!(text.cancel_label(), b"CANCEL");
        assert_eq!(text.initial_text_speed_step(), 2);
    }

    #[test]
    fn hyperspace_resources_match_the_shipped_executable() {
        let executable = include_bytes!("../../../re/bin/BLOODPRG.EXE");
        let resources = decode_bloodprg_hyperspace_resources(executable).unwrap();
        let names = resources
            .sequence_names()
            .iter()
            .map(Box::as_ref)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            [
                b"hyper_00.hnm".as_slice(),
                b"hyper_01.hnm".as_slice(),
                b"hyper_02.hnm".as_slice(),
                b"hyper_03.hnm".as_slice(),
                b"hyper_04.hnm".as_slice(),
                b"hyper_05.hnm".as_slice(),
                b"hyper_06.hnm".as_slice(),
                b"hyper_07.hnm".as_slice(),
            ]
        );
    }

    #[test]
    fn malformed_hyperspace_resources_are_rejected_before_runtime_use() {
        assert_eq!(
            decode_bloodprg_hyperspace_resources(&[]),
            Err(BloodprgHyperspaceResourcesError::TruncatedExecutable {
                actual: 0,
                required: HYPERSPACE_RESOURCES_REQUIRED_EXECUTABLE_LENGTH,
            })
        );

        let mut executable = include_bytes!("../../../re/bin/BLOODPRG.EXE").to_vec();
        let first_name = BLOODPRG_DATA_FILE_OFFSET + HYPERSPACE_SEQUENCE_NAMES_DATA_OFFSET;
        executable[first_name..first_name + HYPERSPACE_SEQUENCE_NAME_FIELD_BYTE_COUNT].fill(b'x');
        assert_eq!(
            decode_bloodprg_hyperspace_resources(&executable),
            Err(BloodprgHyperspaceResourcesError::MissingNameTerminator { sequence: 0 })
        );

        let mut executable = include_bytes!("../../../re/bin/BLOODPRG.EXE").to_vec();
        executable[first_name + HYPERSPACE_SEQUENCE_NAME_FIELD_BYTE_COUNT - 1] = b'x';
        assert_eq!(
            decode_bloodprg_hyperspace_resources(&executable),
            Err(BloodprgHyperspaceResourcesError::NonZeroNamePadding {
                sequence: 0,
                byte: b'x',
            })
        );
    }

    #[test]
    fn malformed_bridge_menu_tables_are_rejected_before_pointer_use() {
        assert_eq!(
            decode_bloodprg_bridge_menu_text(&[]),
            Err(BloodprgBridgeMenuTextError::TruncatedExecutable {
                actual: 0,
                required: MENU_TEXT_REQUIRED_EXECUTABLE_LENGTH,
            })
        );

        let mut executable = include_bytes!("../../../re/bin/BLOODPRG.EXE").to_vec();
        executable[BLOODPRG_DATA_FILE_OFFSET
            + OPTION_MENU_POINTER_LIST_DATA_OFFSET
            + BLOODPRG_OPTION_MENU_LABEL_COUNT * WORD_BYTE_COUNT
            ..BLOODPRG_DATA_FILE_OFFSET
                + OPTION_MENU_POINTER_LIST_DATA_OFFSET
                + (BLOODPRG_OPTION_MENU_LABEL_COUNT + 1) * WORD_BYTE_COUNT]
            .copy_from_slice(&u16::MIN.to_le_bytes());
        assert_eq!(
            decode_bloodprg_bridge_menu_text(&executable),
            Err(BloodprgBridgeMenuTextError::MissingPointerListSentinel {
                list_offset: OPTION_MENU_POINTER_LIST_DATA_OFFSET,
                actual: u16::MIN,
            })
        );
    }

    fn executable_fixture(vector: &BridgeResourceOracle) -> Vec<u8> {
        let mut executable = vec![u8::MIN; REQUIRED_EXECUTABLE_LENGTH];
        executable[MZ_SIGNATURE_FILE_OFFSET..MZ_SIGNATURE.len()].copy_from_slice(&MZ_SIGNATURE);
        for (index, anchor) in vector.anchors.iter().copied().enumerate() {
            let position = PROJECTION_ANCHOR_FILE_OFFSET + index * PROJECTION_ANCHOR_BYTE_COUNT;
            for (component, value) in anchor.into_iter().enumerate() {
                let component_position = position + component * WORD_BYTE_COUNT;
                executable[component_position..component_position + WORD_BYTE_COUNT]
                    .copy_from_slice(&value.to_le_bytes());
            }
        }
        for (index, sample) in vector.trigonometry.iter().copied().enumerate() {
            let position = TRIGONOMETRY_FILE_OFFSET + index * TRIGONOMETRY_SAMPLE_BYTE_COUNT;
            for (component, value) in sample.into_iter().enumerate() {
                let component_position = position + component * WORD_BYTE_COUNT;
                executable[component_position..component_position + WORD_BYTE_COUNT]
                    .copy_from_slice(&value.to_le_bytes());
            }
        }
        executable
    }

    const fn actor_record(
        flags: u8,
        resource_id: u16,
        transition_resource_id: Option<u16>,
        target_arc: u16,
        hit_rectangle: [i16; RECTANGLE_COMPONENT_COUNT],
        draw_position: [u16; 2],
    ) -> BloodprgNavActorRecord {
        BloodprgNavActorRecord {
            flags,
            resource_id,
            transition_resource_id,
            terminal_frame: u16::MIN,
            frame: u16::MIN,
            target_arc,
            hit_rectangle: BloodprgHitRectangle {
                origin: [hit_rectangle[0], hit_rectangle[1]],
                size: [hit_rectangle[2], hit_rectangle[3]],
            },
            draw_position,
        }
    }
}
