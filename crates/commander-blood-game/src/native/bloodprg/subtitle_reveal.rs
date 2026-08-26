//! Progressive subtitle frame and text reveal coordination.

use std::fmt;
use std::ops::Range;

use super::TextPresentationState;

const BRIGHT_FRAME_COLOR: u8 = 255;
const DIM_FRAME_COLOR: u8 = 254;
const INITIAL_REVEAL_DELAY: u16 = 2;
const REVEAL_DELAY_SHIFT: u32 = 2;
const COMPLETION_HOLD_SHIFT: u32 = 2;
const SUBTITLE_LINE_PITCH: u16 = 8;

/// One line or column in the animated subtitle frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubtitleFramePrimitiveKind {
    /// A left-to-right span.
    Horizontal,
    /// A top-to-bottom span.
    Vertical,
}

/// Geometry for one subtitle-frame primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubtitleFramePrimitive {
    /// Primitive orientation.
    pub kind: SubtitleFramePrimitiveKind,
    /// Original logical pixel origin.
    pub origin: [u16; 2],
    /// Span length in original logical pixels.
    pub extent: u16,
}

/// One frame primitive ready for the renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubtitleFrameDraw {
    /// Primitive geometry.
    pub primitive: SubtitleFramePrimitive,
    /// Original indexed-palette color.
    pub color: u8,
    /// Whether the native secondary-frame color remap applies.
    pub remap: bool,
}

/// One carriage-return-delimited subtitle line ready for glyph rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubtitleRevealLine<'a> {
    /// Line bytes without the carriage-return delimiter.
    pub text: &'a [u8],
    /// Line's zero-based byte position in the complete subtitle.
    pub byte_offset: usize,
    /// Number of subtitle bytes currently exposed by the reveal animation.
    pub reveal_cursor: usize,
    /// Original logical pixel origin.
    pub position: [u16; 2],
}

/// Drawing operations emitted by one subtitle reveal update.
pub trait SubtitleRevealRenderer {
    /// Draw one animated frame span.
    fn draw_frame_primitive(&mut self, draw: SubtitleFrameDraw);

    /// Draw one text line using the shared reveal cursor.
    fn draw_subtitle_line(&mut self, line: SubtitleRevealLine<'_>);
}

/// Current stage of the two-frame subtitle opening animation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubtitleRevealPhase {
    /// Bright primary frame.
    BrightOpening,
    /// Dim primary frame.
    DimOpening,
    /// Secondary frame and progressive text.
    #[default]
    Text,
}

impl SubtitleRevealPhase {
    const fn next(self) -> Self {
        match self {
            Self::BrightOpening => Self::DimOpening,
            Self::DimOpening | Self::Text => Self::Text,
        }
    }
}

/// Subtitle-specific state not already owned by the shared text presentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SubtitleRevealState {
    /// A surrounding presentation explicitly requests subtitle rendering.
    pub display_mode: bool,
    /// The ready presentation hold belongs to this subtitle surface.
    pub hold_owned_by_subtitle: bool,
    /// Current frame/text reveal stage.
    pub phase: SubtitleRevealPhase,
    /// Opening-frame pulse consumed by the surrounding timer.
    pub opening_frame_pulse: bool,
    /// Frames remaining before another subtitle byte is exposed.
    pub reveal_delay: u16,
    /// Authored base delay shared by character reveal and completion hold.
    pub text_speed_step: u16,
    /// Ship HUD state that suppresses the normal subtitle completion hold.
    pub ship_hud_active: bool,
    /// Original logical origin of the first subtitle line.
    pub text_origin: [u16; 2],
}

/// Gate that rejected a subtitle update without changing state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubtitleRevealGate {
    /// No subtitle display source is active.
    Inactive,
    /// A presentation hold is ready, but another surface owns it.
    WrongHoldOwner,
}

/// Observable result of one subtitle reveal update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubtitleRevealOutcome {
    /// The routine returned before drawing.
    Gated(SubtitleRevealGate),
    /// One opening-frame stage was drawn.
    OpeningFrame {
        /// Stage used for this frame.
        phase: SubtitleRevealPhase,
        /// Whether the pulse advanced the stage after drawing.
        advanced: bool,
    },
    /// The secondary frame and all subtitle lines were drawn.
    TextFrame {
        /// Number of submitted text lines.
        line_count: usize,
        /// Whether this call exposed one additional byte.
        reveal_advanced: bool,
        /// Whether this call armed the final dialogue hold.
        completion_armed: bool,
    },
}

/// Invalid flat subtitle state that the native pointer walk could not bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubtitleRevealError {
    /// Text presentation reached its drawing phase without authored text.
    EmptyText,
    /// One line has no carriage-return delimiter.
    MissingLineDelimiter {
        /// Start of the malformed line.
        line_start: usize,
    },
    /// The reveal cursor points beyond the owned subtitle bytes.
    CursorOutOfRange {
        /// Supplied cursor.
        cursor: usize,
        /// Number of owned subtitle bytes.
        text_len: usize,
    },
}

impl fmt::Display for SubtitleRevealError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SubtitleRevealError {}

/// Draw and advance one progressive subtitle frame.
///
/// This translates `subtitle_reveal_pump` at BLOODPRG routine offset
/// `0x0093F5`. Owned subtitle bytes, an optional zero-based cursor, typed frame
/// primitives, and renderer requests replace native null/near pointers,
/// sentinel tables, graphics globals, and segment selection. Character and
/// completion timing retain the original 16-bit shift behavior.
pub fn update_subtitle_reveal<Renderer: SubtitleRevealRenderer>(
    presentation: &mut TextPresentationState,
    state: &mut SubtitleRevealState,
    primary_frame: &[SubtitleFramePrimitive],
    secondary_frame: &[SubtitleFramePrimitive],
    renderer: &mut Renderer,
) -> Result<SubtitleRevealOutcome, SubtitleRevealError> {
    if !state.display_mode && !presentation.subtitle_display_active {
        if !presentation.hold_ready {
            return Ok(SubtitleRevealOutcome::Gated(SubtitleRevealGate::Inactive));
        }
        if !state.hold_owned_by_subtitle {
            return Ok(SubtitleRevealOutcome::Gated(
                SubtitleRevealGate::WrongHoldOwner,
            ));
        }
    }

    if presentation.subtitle_reveal_cursor.is_none() {
        state.reveal_delay = INITIAL_REVEAL_DELAY;
        state.opening_frame_pulse = true;
        presentation.subtitle_reveal_cursor = Some(usize::MIN);
        state.phase = SubtitleRevealPhase::BrightOpening;
    }

    let phase = state.phase;
    let line_ranges = if phase == SubtitleRevealPhase::Text {
        Some(subtitle_line_ranges(&presentation.subtitle_text)?)
    } else {
        None
    };
    let (frame, color, remap) = match phase {
        SubtitleRevealPhase::BrightOpening => (primary_frame, BRIGHT_FRAME_COLOR, false),
        SubtitleRevealPhase::DimOpening => (primary_frame, DIM_FRAME_COLOR, false),
        SubtitleRevealPhase::Text => (secondary_frame, DIM_FRAME_COLOR, true),
    };
    for primitive in frame {
        renderer.draw_frame_primitive(SubtitleFrameDraw {
            primitive: *primitive,
            color,
            remap,
        });
    }

    if phase != SubtitleRevealPhase::Text {
        let advanced = !state.opening_frame_pulse;
        if advanced {
            state.opening_frame_pulse = true;
            state.phase = phase.next();
        }
        return Ok(SubtitleRevealOutcome::OpeningFrame { phase, advanced });
    }

    let text_len = presentation.subtitle_text.len();
    let cursor = presentation
        .subtitle_reveal_cursor
        .expect("the entry initialization always publishes a reveal cursor");
    if cursor > text_len {
        return Err(SubtitleRevealError::CursorOutOfRange { cursor, text_len });
    }

    let mut reveal_advanced = false;
    let mut completion_armed = false;
    if cursor < text_len {
        if state.reveal_delay == u16::MIN {
            state.reveal_delay = state.text_speed_step.wrapping_shr(REVEAL_DELAY_SHIFT);
            presentation.subtitle_reveal_cursor = Some(cursor + 1);
            reveal_advanced = true;
        }
    } else if !state.ship_hud_active
        && !presentation.dialogue_hold_complete
        && !presentation.hold_ready
    {
        presentation.subtitle_voice_trigger = false;
        presentation.dialogue_hold_countdown =
            state.text_speed_step.wrapping_shl(COMPLETION_HOLD_SHIFT);
        presentation.dialogue_hold_complete = true;
        completion_armed = true;
    }

    let reveal_cursor = presentation
        .subtitle_reveal_cursor
        .expect("the reveal cursor remains initialized while drawing");
    let line_ranges = line_ranges.expect("text phase always validates line ranges");
    let line_count = line_ranges.len();
    for (line_index, range) in line_ranges.into_iter().enumerate() {
        renderer.draw_subtitle_line(SubtitleRevealLine {
            text: &presentation.subtitle_text[range.clone()],
            byte_offset: range.start,
            reveal_cursor,
            position: [
                state.text_origin[0],
                state.text_origin[1]
                    .wrapping_add((line_index as u16).wrapping_mul(SUBTITLE_LINE_PITCH)),
            ],
        });
    }

    Ok(SubtitleRevealOutcome::TextFrame {
        line_count,
        reveal_advanced,
        completion_armed,
    })
}

fn subtitle_line_ranges(text: &[u8]) -> Result<Vec<Range<usize>>, SubtitleRevealError> {
    if text.is_empty() {
        return Err(SubtitleRevealError::EmptyText);
    }

    let mut ranges = Vec::new();
    let mut line_start = usize::MIN;
    while line_start < text.len() {
        let delimiter = text[line_start..]
            .iter()
            .position(|byte| *byte == b'\r')
            .map(|relative| line_start + relative)
            .ok_or(SubtitleRevealError::MissingLineDelimiter { line_start })?;
        ranges.push(line_start..delimiter);
        line_start = delimiter + 1;
    }
    Ok(ranges)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const NATIVE_TEXT_OFFSET: usize = 3_608;
    const NATIVE_SUBTITLE_OWNER: u16 = 24_164;
    const TEXT_SPEED_STEP: u16 = 8;
    const FIRST_LINE_ORIGIN: [u16; 2] = [10, 8];
    const SUBTITLE_TEXT: &[u8] = b"AB\rCD\r";
    const PRIMARY_FRAME: &[SubtitleFramePrimitive] = &[
        SubtitleFramePrimitive {
            kind: SubtitleFramePrimitiveKind::Horizontal,
            origin: [11, 22],
            extent: 33,
        },
        SubtitleFramePrimitive {
            kind: SubtitleFramePrimitiveKind::Vertical,
            origin: [44, 55],
            extent: 66,
        },
    ];
    const SECONDARY_FRAME: &[SubtitleFramePrimitive] = &[SubtitleFramePrimitive {
        kind: SubtitleFramePrimitiveKind::Horizontal,
        origin: [77, 88],
        extent: 99,
    }];

    #[derive(Deserialize)]
    struct RevealVector {
        name: String,
        mode: u8,
        active: u8,
        hold_ready: u8,
        owner: u16,
        cursor: usize,
        phase: u16,
        pulse: u16,
        delay: u16,
        ship_flags: u16,
        hold_complete: u8,
        calls: Vec<OracleCall>,
    }

    #[derive(Deserialize)]
    struct OracleCall {
        name: String,
        ax: u16,
        bx: u16,
        cx: u16,
        dx: u16,
        si: u16,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum RenderCall {
        Frame(SubtitleFrameDraw),
        Line {
            text: Box<[u8]>,
            byte_offset: usize,
            reveal_cursor: usize,
            position: [u16; 2],
        },
    }

    #[derive(Default)]
    struct RecordingRenderer {
        calls: Vec<RenderCall>,
    }

    impl SubtitleRevealRenderer for RecordingRenderer {
        fn draw_frame_primitive(&mut self, draw: SubtitleFrameDraw) {
            self.calls.push(RenderCall::Frame(draw));
        }

        fn draw_subtitle_line(&mut self, line: SubtitleRevealLine<'_>) {
            self.calls.push(RenderCall::Line {
                text: Box::from(line.text),
                byte_offset: line.byte_offset,
                reveal_cursor: line.reveal_cursor,
                position: line.position,
            });
        }
    }

    #[test]
    fn subtitle_reveal_matches_every_original_vector() {
        let vectors: Vec<RevealVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_93f5_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 11);

        for vector in vectors {
            let native_cursor = vector.cursor;
            let cursor = (native_cursor != usize::MIN)
                .then(|| native_cursor.checked_sub(NATIVE_TEXT_OFFSET).unwrap());
            let mut presentation = TextPresentationState {
                subtitle_display_active: vector.active & 1 != u8::MIN,
                hold_ready: vector.hold_ready & 1 != u8::MIN,
                subtitle_voice_trigger: true,
                subtitle_reveal_cursor: cursor,
                dialogue_hold_countdown: 13_621,
                dialogue_hold_complete: vector.hold_complete & 1 != u8::MIN,
                subtitle_text: Box::from(SUBTITLE_TEXT),
                ..TextPresentationState::default()
            };
            let mut state = SubtitleRevealState {
                display_mode: vector.mode & 2 != u8::MIN,
                hold_owned_by_subtitle: vector.owner == NATIVE_SUBTITLE_OWNER,
                phase: decode_phase(vector.phase),
                opening_frame_pulse: vector.pulse != u16::MIN,
                reveal_delay: vector.delay,
                text_speed_step: TEXT_SPEED_STEP,
                ship_hud_active: vector.ship_flags & 4 != u16::MIN,
                text_origin: FIRST_LINE_ORIGIN,
            };
            let mut renderer = RecordingRenderer::default();

            let outcome = update_subtitle_reveal(
                &mut presentation,
                &mut state,
                PRIMARY_FRAME,
                SECONDARY_FRAME,
                &mut renderer,
            )
            .unwrap_or_else(|error| panic!("{}: {error}", vector.name));

            assert_calls_match(
                &vector,
                &renderer.calls,
                presentation.subtitle_reveal_cursor,
            );
            assert_state_matches(&vector, native_cursor, &presentation, &state, outcome);
        }
    }

    fn decode_phase(phase: u16) -> SubtitleRevealPhase {
        match phase {
            2 => SubtitleRevealPhase::BrightOpening,
            1 => SubtitleRevealPhase::DimOpening,
            _ => SubtitleRevealPhase::Text,
        }
    }

    fn assert_calls_match(
        vector: &RevealVector,
        actual: &[RenderCall],
        final_reveal_cursor: Option<usize>,
    ) {
        assert_eq!(actual.len(), vector.calls.len(), "{}", vector.name);
        for (actual, expected) in actual.iter().zip(&vector.calls) {
            match actual {
                RenderCall::Frame(draw) => {
                    let expected_kind = match expected.name.as_str() {
                        "span" => SubtitleFramePrimitiveKind::Horizontal,
                        "vertical" => SubtitleFramePrimitiveKind::Vertical,
                        name => panic!("{}: unexpected oracle call {name}", vector.name),
                    };
                    assert_eq!(draw.primitive.kind, expected_kind, "{}", vector.name);
                    assert_eq!(draw.color, expected.ax as u8, "{}", vector.name);
                    assert_eq!(
                        draw.primitive.origin,
                        [expected.bx, expected.cx],
                        "{}",
                        vector.name
                    );
                    assert_eq!(draw.primitive.extent, expected.dx, "{}", vector.name);
                    assert_eq!(draw.remap, vector.phase == u16::MIN, "{}", vector.name);
                }
                RenderCall::Line {
                    text,
                    byte_offset,
                    reveal_cursor,
                    position,
                } => {
                    assert_eq!(expected.name, "draw", "{}", vector.name);
                    assert_eq!(
                        *byte_offset,
                        usize::from(expected.si) - NATIVE_TEXT_OFFSET,
                        "{}",
                        vector.name
                    );
                    assert_eq!(*position, [expected.bx, expected.dx], "{}", vector.name);
                    assert_eq!(Some(*reveal_cursor), final_reveal_cursor, "{}", vector.name);
                    assert_eq!(
                        text.as_ref(),
                        if *byte_offset == usize::MIN {
                            b"AB"
                        } else {
                            b"CD"
                        },
                        "{}",
                        vector.name
                    );
                }
            }
        }
    }

    fn assert_state_matches(
        vector: &RevealVector,
        native_cursor: usize,
        presentation: &TextPresentationState,
        state: &SubtitleRevealState,
        outcome: SubtitleRevealOutcome,
    ) {
        let entered = vector.mode & 2 != u8::MIN
            || vector.active & 1 != u8::MIN
            || (vector.hold_ready & 1 != u8::MIN && vector.owner == NATIVE_SUBTITLE_OWNER);
        let completion_armed = native_cursor == NATIVE_TEXT_OFFSET + SUBTITLE_TEXT.len()
            && vector.ship_flags & 4 == u16::MIN
            && vector.hold_complete & 1 == u8::MIN
            && vector.hold_ready & 1 == u8::MIN;
        let expected_outcome = if !entered {
            SubtitleRevealOutcome::Gated(if vector.hold_ready & 1 == u8::MIN {
                SubtitleRevealGate::Inactive
            } else {
                SubtitleRevealGate::WrongHoldOwner
            })
        } else if native_cursor == usize::MIN {
            SubtitleRevealOutcome::OpeningFrame {
                phase: SubtitleRevealPhase::BrightOpening,
                advanced: false,
            }
        } else if vector.phase == 2 {
            SubtitleRevealOutcome::OpeningFrame {
                phase: SubtitleRevealPhase::BrightOpening,
                advanced: vector.pulse == u16::MIN,
            }
        } else if vector.phase == 1 {
            SubtitleRevealOutcome::OpeningFrame {
                phase: SubtitleRevealPhase::DimOpening,
                advanced: vector.pulse == u16::MIN,
            }
        } else {
            SubtitleRevealOutcome::TextFrame {
                line_count: 2,
                reveal_advanced: native_cursor == NATIVE_TEXT_OFFSET && vector.delay == u16::MIN,
                completion_armed,
            }
        };
        assert_eq!(outcome, expected_outcome, "{}", vector.name);

        if !entered {
            assert_eq!(
                presentation.subtitle_reveal_cursor,
                Some(native_cursor - NATIVE_TEXT_OFFSET),
                "{}",
                vector.name
            );
            assert_eq!(state.reveal_delay, vector.delay, "{}", vector.name);
            return;
        }

        if native_cursor == usize::MIN {
            assert_eq!(
                presentation.subtitle_reveal_cursor,
                Some(usize::MIN),
                "{}",
                vector.name
            );
            assert_eq!(
                state.phase,
                SubtitleRevealPhase::BrightOpening,
                "{}",
                vector.name
            );
            assert!(state.opening_frame_pulse, "{}", vector.name);
            assert_eq!(state.reveal_delay, INITIAL_REVEAL_DELAY, "{}", vector.name);
            return;
        }

        match vector.phase {
            2 if vector.pulse == u16::MIN => {
                assert_eq!(
                    state.phase,
                    SubtitleRevealPhase::DimOpening,
                    "{}",
                    vector.name
                );
                assert!(state.opening_frame_pulse, "{}", vector.name);
            }
            2 => assert_eq!(
                state.phase,
                SubtitleRevealPhase::BrightOpening,
                "{}",
                vector.name
            ),
            1 => assert_eq!(
                state.phase,
                SubtitleRevealPhase::DimOpening,
                "{}",
                vector.name
            ),
            _ if native_cursor == NATIVE_TEXT_OFFSET && vector.delay == u16::MIN => {
                assert_eq!(
                    presentation.subtitle_reveal_cursor,
                    Some(1),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.reveal_delay,
                    TEXT_SPEED_STEP >> REVEAL_DELAY_SHIFT,
                    "{}",
                    vector.name
                );
            }
            _ if native_cursor == NATIVE_TEXT_OFFSET + SUBTITLE_TEXT.len() => {
                assert_eq!(
                    presentation.dialogue_hold_complete,
                    completion_armed || vector.hold_complete & 1 != u8::MIN,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    presentation.dialogue_hold_countdown,
                    if completion_armed {
                        TEXT_SPEED_STEP << COMPLETION_HOLD_SHIFT
                    } else {
                        13_621
                    },
                    "{}",
                    vector.name
                );
                assert_eq!(
                    presentation.subtitle_voice_trigger, !completion_armed,
                    "{}",
                    vector.name
                );
            }
            _ => {
                assert_eq!(
                    presentation.subtitle_reveal_cursor,
                    Some(native_cursor - NATIVE_TEXT_OFFSET),
                    "{}",
                    vector.name
                );
                assert_eq!(state.reveal_delay, vector.delay, "{}", vector.name);
            }
        }
    }

    #[test]
    fn malformed_flat_text_is_rejected_before_rendering() {
        let mut presentation = TextPresentationState {
            subtitle_display_active: true,
            subtitle_reveal_cursor: Some(usize::MIN),
            subtitle_text: Box::from(b"missing delimiter".as_slice()),
            ..TextPresentationState::default()
        };
        let mut state = SubtitleRevealState::default();
        let mut renderer = RecordingRenderer::default();

        assert_eq!(
            update_subtitle_reveal(
                &mut presentation,
                &mut state,
                PRIMARY_FRAME,
                SECONDARY_FRAME,
                &mut renderer,
            ),
            Err(SubtitleRevealError::MissingLineDelimiter {
                line_start: usize::MIN,
            })
        );
        assert!(renderer.calls.is_empty());
    }
}
