//! Frame-timed centered subtitles for DESCRIPT sequence videos.

use commander_blood_formats::descript::DescriptSequenceSubtitle;

const SUBTITLE_WRAP_COLUMN: i8 = 28;
const SUBTITLE_CENTER_X: u16 = 160;
const SUBTITLE_GLYPH_HALF_WIDTH: u16 = 4;
const SUBTITLE_FIRST_Y: u16 = 110;
const SUBTITLE_LINE_HEIGHT: u16 = 8;
const SUBTITLE_COLOR: u8 = 239;

/// Current authored cue in a sequence-video subtitle list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SequenceSubtitlePlayback {
    cue_index: usize,
}

impl SequenceSubtitlePlayback {
    /// Return the cue that will be considered by the next presentation call.
    pub const fn cue_index(&self) -> usize {
        self.cue_index
    }

    /// Restart subtitle playback at the first authored cue.
    pub fn restart(&mut self) {
        self.cue_index = usize::MIN;
    }
}

/// One centered line ready for the game-font renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CenteredSequenceSubtitleLine<'a> {
    /// Authored bytes to draw, including a trailing wrap space when present.
    pub text: &'a [u8],
    /// Logical indexed-framebuffer position.
    pub position: [u16; 2],
    /// Original palette index.
    pub color: u8,
}

/// Video clock and game-font operations required by subtitle presentation.
pub trait SequenceSubtitleRenderer {
    /// Renderer-specific failure.
    type Error;

    /// Return the video frame currently visible to the player.
    fn visible_frame(&self) -> u16;

    /// Draw one complete centered game-font line.
    fn draw_centered_line(
        &mut self,
        line: CenteredSequenceSubtitleLine<'_>,
    ) -> Result<(), Self::Error>;
}

/// Observable result of one sequence-subtitle presentation call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequenceSubtitleOutcome {
    /// The decoded sequence has no cue at the current playback position.
    Finished,
    /// The current cue has not reached its first visible video frame.
    Waiting {
        /// Authored cue that remains pending.
        cue_index: usize,
    },
    /// The current cue was redrawn.
    Drawn {
        /// Authored cue that was presented.
        cue_index: usize,
        /// Number of centered lines submitted to the renderer.
        line_count: usize,
        /// Whether the following cue became current after this draw.
        advanced: bool,
    },
}

/// Draw the current frame-timed subtitle and advance at most one cue.
///
/// This translates `list_walk_f18` at BLOODPRG routine offset `0x007CE8`.
/// Decoded cue values and slice bounds replace its packed byte stream and
/// terminal marker. The renderer's frame is read again after drawing because
/// sequence playback can advance while game-font lines are submitted.
pub fn present_sequence_subtitle<Renderer: SequenceSubtitleRenderer>(
    subtitles: &[DescriptSequenceSubtitle],
    playback: &mut SequenceSubtitlePlayback,
    renderer: &mut Renderer,
) -> Result<SequenceSubtitleOutcome, Renderer::Error> {
    let Some(subtitle) = subtitles.get(playback.cue_index) else {
        return Ok(SequenceSubtitleOutcome::Finished);
    };
    if !frame_threshold_is_visible(subtitle.first_visible_frame(), renderer.visible_frame()) {
        return Ok(SequenceSubtitleOutcome::Waiting {
            cue_index: playback.cue_index,
        });
    }

    let cue_index = playback.cue_index;
    let lines = centered_lines(subtitle.text());
    for line in &lines {
        renderer.draw_centered_line(*line)?;
    }

    let advanced = subtitles.get(cue_index + 1).is_some_and(|next| {
        frame_threshold_is_visible(next.first_visible_frame(), renderer.visible_frame())
    });
    if advanced {
        playback.cue_index += 1;
    }

    Ok(SequenceSubtitleOutcome::Drawn {
        cue_index,
        line_count: lines.len(),
        advanced,
    })
}

fn centered_lines(text: &[u8]) -> Vec<CenteredSequenceSubtitleLine<'_>> {
    let mut lines = Vec::new();
    let mut line_start = usize::MIN;
    let mut line_length = u16::MIN;

    for (index, character) in text.iter().copied().enumerate() {
        line_length = line_length.wrapping_add(1);
        if character == b' ' && (line_length as u8 as i8) >= SUBTITLE_WRAP_COLUMN {
            lines.push(centered_line(
                &text[line_start..=index],
                line_length,
                lines.len(),
            ));
            line_start = index + 1;
            line_length = u16::MIN;
        }
    }
    lines.push(centered_line(&text[line_start..], line_length, lines.len()));
    lines
}

fn centered_line(
    text: &[u8],
    character_count: u16,
    line_index: usize,
) -> CenteredSequenceSubtitleLine<'_> {
    let x = SUBTITLE_CENTER_X.wrapping_sub(character_count.wrapping_mul(SUBTITLE_GLYPH_HALF_WIDTH));
    let y = SUBTITLE_FIRST_Y.wrapping_add((line_index as u16).wrapping_mul(SUBTITLE_LINE_HEIGHT));
    CenteredSequenceSubtitleLine {
        text,
        position: [x, y],
        color: SUBTITLE_COLOR,
    }
}

const fn frame_threshold_is_visible(first_visible_frame: u16, visible_frame: u16) -> bool {
    let signed_threshold = first_visible_frame as i16;
    signed_threshold >= 0 && signed_threshold <= visible_frame as i16
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 14;

    #[derive(Deserialize)]
    struct SubtitleOracle {
        name: String,
        threshold: u16,
        visible_index_before: u16,
        visible_index_after: u16,
        text_length: usize,
        line_records: Vec<LineRecord>,
        next_threshold_after: u16,
        next_valid: bool,
        helper_calls: usize,
    }

    #[derive(Deserialize)]
    struct LineRecord {
        character_count: usize,
        centered_x: u16,
    }

    struct OracleRenderer {
        visible_frame: u16,
        visible_frame_after_first_draw: u16,
        lines: Vec<OwnedLine>,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct OwnedLine {
        text: Box<[u8]>,
        position: [u16; 2],
        color: u8,
    }

    impl SequenceSubtitleRenderer for OracleRenderer {
        type Error = std::convert::Infallible;

        fn visible_frame(&self) -> u16 {
            self.visible_frame
        }

        fn draw_centered_line(
            &mut self,
            line: CenteredSequenceSubtitleLine<'_>,
        ) -> Result<(), Self::Error> {
            self.lines.push(OwnedLine {
                text: Box::from(line.text),
                position: line.position,
                color: line.color,
            });
            self.visible_frame = self.visible_frame_after_first_draw;
            Ok(())
        }
    }

    #[test]
    fn presenter_matches_every_original_semantic_vector() {
        let vectors: Vec<SubtitleOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_7ce8_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let text = oracle_text(&vector.name);
            assert_eq!(text.len(), vector.text_length, "{}", vector.name);
            let subtitles = [
                DescriptSequenceSubtitle::new(vector.threshold, text.clone()),
                DescriptSequenceSubtitle::new(
                    vector.next_threshold_after,
                    Box::from(b"following".as_slice()),
                ),
            ];
            let mut playback = SequenceSubtitlePlayback::default();
            let mut renderer = OracleRenderer {
                visible_frame: vector.visible_index_before,
                visible_frame_after_first_draw: vector.visible_index_after,
                lines: Vec::new(),
            };

            let outcome =
                present_sequence_subtitle(&subtitles, &mut playback, &mut renderer).unwrap();

            assert_eq!(renderer.lines.len(), vector.helper_calls, "{}", vector.name);
            for (index, (actual, expected)) in
                renderer.lines.iter().zip(&vector.line_records).enumerate()
            {
                assert_eq!(
                    actual.text.len(),
                    expected.character_count,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    actual.position,
                    [
                        expected.centered_x,
                        SUBTITLE_FIRST_Y + index as u16 * SUBTITLE_LINE_HEIGHT,
                    ],
                    "{}",
                    vector.name,
                );
                assert_eq!(actual.color, SUBTITLE_COLOR, "{}", vector.name);
            }

            if vector.helper_calls == usize::MIN {
                assert_eq!(
                    outcome,
                    SequenceSubtitleOutcome::Waiting { cue_index: 0 },
                    "{}",
                    vector.name,
                );
            } else {
                assert_eq!(
                    outcome,
                    SequenceSubtitleOutcome::Drawn {
                        cue_index: 0,
                        line_count: vector.line_records.len(),
                        advanced: vector.next_valid,
                    },
                    "{}",
                    vector.name,
                );
            }
            assert_eq!(
                playback.cue_index(),
                usize::from(vector.next_valid),
                "{}",
                vector.name,
            );
        }
    }

    #[test]
    fn sequence_end_and_restart_use_typed_list_bounds() {
        let mut playback = SequenceSubtitlePlayback { cue_index: 1 };
        let mut renderer = OracleRenderer {
            visible_frame: u16::MIN,
            visible_frame_after_first_draw: u16::MIN,
            lines: Vec::new(),
        };
        let subtitles = [DescriptSequenceSubtitle::new(
            u16::MIN,
            Box::from(b"cue".as_slice()),
        )];

        assert_eq!(
            present_sequence_subtitle(&subtitles, &mut playback, &mut renderer).unwrap(),
            SequenceSubtitleOutcome::Finished,
        );
        playback.restart();
        assert_eq!(playback.cue_index(), usize::MIN);
    }

    fn oracle_text(name: &str) -> Box<[u8]> {
        match name {
            "negative_initial_threshold" | "initial_threshold_above_visible" => {
                Box::from(b"ignored".as_slice())
            }
            "equal_threshold_single_line" => Box::from(b"ABC".as_slice()),
            "empty_text_zero_limit_draws_one_empty_line" => Box::new([]),
            "negative_next_threshold_keeps_cursor" => Box::from(b"NEXT".as_slice()),
            "next_threshold_above_visible_keeps_cursor" => Box::from(b"WAIT".as_slice()),
            "space_at_28_wraps" => repeated_words(27, b'B'),
            "space_at_27_does_not_wrap" => repeated_words(26, b'B'),
            "three_centered_lines" => {
                let mut text = vec![b'A'; 27];
                text.push(b' ');
                text.extend([b'B'; 27]);
                text.extend([b' ', b'C']);
                text.into_boxed_slice()
            }
            "signed_low_byte_suppresses_128_space_break" => repeated_words(127, b'B'),
            "wrapped_cursor_and_text" => Box::from(b"WRAP".as_slice()),
            "helper_invalidates_following_threshold" => Box::from(b"MUTATE".as_slice()),
            "helper_reduces_visible_index" => Box::from(b"LIMIT".as_slice()),
            "helper_increases_visible_index" => Box::from(b"OPEN".as_slice()),
            _ => panic!("unknown subtitle oracle vector {name}"),
        }
    }

    fn repeated_words(first_word_length: usize, final_character: u8) -> Box<[u8]> {
        let mut text = vec![b'A'; first_word_length];
        text.extend([b' ', final_character]);
        text.into_boxed_slice()
    }
}
