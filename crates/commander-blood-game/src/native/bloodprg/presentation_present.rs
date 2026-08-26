//! Presentation of one active streamed frame into flat indexed buffers.

use std::error::Error;
use std::fmt;

use super::{
    AbDecodeOutcome, ActivatedPresentationEntry, FramebufferCopyError, LOGICAL_FRAMEBUFFER_HEIGHT,
    LOGICAL_FRAMEBUFFER_WIDTH, PresentationAdError, PresentationAdOutcome, PresentationEntryFrame,
    PresentationPayload, PresentationRasterError, PresentationRectBlitOutcome,
    PresentationRectDecodeOutcome, blit_presentation_rect, copy_full_frame_to_display,
    decode_presentation_rect,
};

const LAYOUT_WIDTH_MASK: u16 = 0xF9FF;
const NO_COORDINATES_LAYOUT_FLAG: u16 = 0x0400;
const MAXIMUM_PRESENTATION_ROWS: u8 = 130;
const COORDINATE_BYTE_COUNT: usize = size_of::<u16>() * 2;

/// Active and most recently retired presentation-frame ownership.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresentationActiveEntryState {
    /// Frame waiting to be drawn once.
    pub active: Option<ActivatedPresentationEntry>,
    /// Frame retired by the latest presentation attempt.
    pub retired: Option<ActivatedPresentationEntry>,
    /// Whether any active frame reached the presenter.
    pub frame_presented: bool,
    /// Complete queue extent owning the active frame.
    pub active_queue_extent: Option<usize>,
    /// Sound side record published with the active queue entry.
    pub active_sound_record: Option<Box<[u8]>>,
    /// Palette blocks retained until this frame is due for presentation.
    pub pending_palette_payload: Option<Box<[u8]>>,
}

/// Runtime gates affecting the destination and row-count policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationPresentPolicy {
    /// Draw the rectangle into the back buffer before presenting it.
    pub draw_via_back_buffer: bool,
    /// Do not copy the existing back buffer before a display-buffer update.
    pub skip_back_buffer_present: bool,
    /// Permit authored row counts above the usual 130-row presentation band.
    pub unclamped_rows: bool,
    /// Logical rows added to authored rectangle coordinates.
    pub vertical_offset: usize,
}

/// Flat framebuffer selected for a rectangle operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationEntryRenderTarget {
    /// Current display framebuffer.
    Display,
    /// Buffered background framebuffer.
    BackBuffer,
}

/// Observable work completed while retiring one active entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationActiveEntryOutcome {
    /// Whether an active frame was retired.
    pub frame_presented: bool,
    /// Whether the back buffer was copied to the display.
    pub back_buffer_presented: bool,
    /// Destination and result of an ordinary rectangle blit.
    pub rectangle_blit: Option<(PresentationEntryRenderTarget, PresentationRectBlitOutcome)>,
    /// Result of a deferred transparent AD rectangle.
    pub rectangle_decode: Option<PresentationRectDecodeOutcome>,
}

/// Invalid active-frame data, geometry, or flat rendering operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationActiveEntryError {
    /// A coordinate-bearing frame body ended before both coordinates.
    CoordinatesTruncated {
        /// Available bytes after the typed frame header.
        available: usize,
    },
    /// A nonempty typed frame has no pixel body.
    MissingFrameBody,
    /// A deferred compressed rectangle reached the back-buffer-only path.
    DeferredRectangleInBackBufferMode,
    /// The native dispatcher left an unrecognized compressed destination stale.
    UnrecognizedDecodedPayload {
        /// Wrapping signature checksum reported by the dispatcher.
        checksum: u8,
    },
    /// Adding the vertical presentation offset overflowed a host coordinate.
    VerticalCoordinateOverflow {
        /// Authored top coordinate.
        y: usize,
        /// Runtime vertical adjustment.
        vertical_offset: usize,
    },
    /// Rectangle geometry lies outside the logical 320 by 200 framebuffer.
    RectangleOutOfBounds {
        /// Left pixel coordinate.
        x: usize,
        /// Adjusted top pixel coordinate.
        y: usize,
        /// Masked rectangle width.
        width: usize,
        /// Effective row count.
        rows: usize,
    },
    /// Complete back-buffer presentation failed.
    FramebufferCopy(FramebufferCopyError),
    /// Ordinary rectangle rasterization failed.
    Raster(PresentationRasterError),
    /// Deferred AD rectangle expansion failed.
    Decode(PresentationAdError),
}

impl fmt::Display for PresentationActiveEntryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid active presentation entry: {self:?}")
    }
}

impl Error for PresentationActiveEntryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::FramebufferCopy(source) => Some(source),
            Self::Raster(source) => Some(source),
            Self::Decode(source) => Some(source),
            _ => None,
        }
    }
}

impl From<FramebufferCopyError> for PresentationActiveEntryError {
    fn from(error: FramebufferCopyError) -> Self {
        Self::FramebufferCopy(error)
    }
}

impl From<PresentationRasterError> for PresentationActiveEntryError {
    fn from(error: PresentationRasterError) -> Self {
        Self::Raster(error)
    }
}

impl From<PresentationAdError> for PresentationActiveEntryError {
    fn from(error: PresentationAdError) -> Self {
        Self::Decode(error)
    }
}

/// Rendering operations required by active-entry presentation.
pub trait PresentationEntryPresenter {
    /// Copy the complete back buffer into the display buffer.
    fn present_back_buffer(&mut self) -> Result<(), PresentationActiveEntryError>;

    /// Draw an ordinary indexed rectangle into the selected buffer.
    fn blit_rectangle(
        &mut self,
        source: &[u8],
        target: PresentationEntryRenderTarget,
        x: usize,
        y: usize,
        width: usize,
        row_mode: u16,
    ) -> Result<PresentationRectBlitOutcome, PresentationActiveEntryError>;

    /// Expand a deferred transparent AD rectangle directly into the display.
    fn decode_rectangle(
        &mut self,
        source: &[u8],
        vertical_offset: usize,
        layout: u16,
        row_mode: u16,
    ) -> Result<PresentationRectDecodeOutcome, PresentationActiveEntryError>;
}

/// Concrete presenter over the modern game's owned indexed framebuffers.
pub struct FlatPresentationEntryPresenter<'a> {
    /// Current 320 by 200 indexed display.
    pub display_buffer: &'a mut [u8],
    /// Current 320 by 200 indexed back buffer.
    pub back_buffer: &'a mut [u8],
    /// Reusable AD decoder staging bytes.
    pub decode_staging: &'a mut [u8],
}

impl PresentationEntryPresenter for FlatPresentationEntryPresenter<'_> {
    fn present_back_buffer(&mut self) -> Result<(), PresentationActiveEntryError> {
        copy_full_frame_to_display(self.back_buffer, self.display_buffer)?;
        Ok(())
    }

    fn blit_rectangle(
        &mut self,
        source: &[u8],
        target: PresentationEntryRenderTarget,
        x: usize,
        y: usize,
        width: usize,
        row_mode: u16,
    ) -> Result<PresentationRectBlitOutcome, PresentationActiveEntryError> {
        let framebuffer = match target {
            PresentationEntryRenderTarget::Display => &mut *self.display_buffer,
            PresentationEntryRenderTarget::BackBuffer => &mut *self.back_buffer,
        };
        Ok(blit_presentation_rect(
            source,
            framebuffer,
            x,
            y,
            width,
            row_mode,
        )?)
    }

    fn decode_rectangle(
        &mut self,
        source: &[u8],
        vertical_offset: usize,
        layout: u16,
        row_mode: u16,
    ) -> Result<PresentationRectDecodeOutcome, PresentationActiveEntryError> {
        Ok(decode_presentation_rect(
            source,
            self.decode_staging,
            self.display_buffer,
            vertical_offset,
            layout,
            row_mode,
        )?)
    }
}

fn decoded_body(payload: &PresentationPayload) -> Result<&[u8], PresentationActiveEntryError> {
    match payload {
        PresentationPayload::Ab(AbDecodeOutcome { bytes, .. }) => Ok(bytes),
        PresentationPayload::Ad(PresentationAdOutcome { bytes, .. }) => Ok(bytes),
        PresentationPayload::Unrecognized { checksum } => {
            Err(PresentationActiveEntryError::UnrecognizedDecodedPayload {
                checksum: *checksum,
            })
        }
    }
}

fn frame_body(entry: &ActivatedPresentationEntry) -> Result<&[u8], PresentationActiveEntryError> {
    match &entry.frame {
        PresentationEntryFrame::Empty => {
            if entry.row_mode as u8 == u8::MIN {
                Ok(&[])
            } else {
                Err(PresentationActiveEntryError::MissingFrameBody)
            }
        }
        PresentationEntryFrame::Encoded(bytes) => Ok(bytes),
        PresentationEntryFrame::Decoded(payload) => decoded_body(payload),
        PresentationEntryFrame::DeferredTransparent(_) => {
            Err(PresentationActiveEntryError::DeferredRectangleInBackBufferMode)
        }
    }
}

struct PresentationRectangle<'a> {
    pixels: &'a [u8],
    x: usize,
    y: usize,
    width: usize,
    row_mode: u16,
}

fn presentation_rectangle<'a>(
    entry: &'a ActivatedPresentationEntry,
    policy: PresentationPresentPolicy,
    clamp_rows: bool,
) -> Result<PresentationRectangle<'a>, PresentationActiveEntryError> {
    let mut body = frame_body(entry)?;
    let (x, authored_y) = if entry.layout & NO_COORDINATES_LAYOUT_FLAG == 0 {
        let coordinates = body.get(..COORDINATE_BYTE_COUNT).ok_or(
            PresentationActiveEntryError::CoordinatesTruncated {
                available: body.len(),
            },
        )?;
        body = &body[COORDINATE_BYTE_COUNT..];
        (
            usize::from(u16::from_le_bytes([coordinates[0], coordinates[1]])),
            usize::from(u16::from_le_bytes([coordinates[2], coordinates[3]])),
        )
    } else {
        (usize::MIN, usize::MIN)
    };
    let y = authored_y.checked_add(policy.vertical_offset).ok_or(
        PresentationActiveEntryError::VerticalCoordinateOverflow {
            y: authored_y,
            vertical_offset: policy.vertical_offset,
        },
    )?;
    let width = usize::from(entry.layout & LAYOUT_WIDTH_MASK);
    let authored_rows = entry.row_mode as u8;
    let rows = if clamp_rows {
        authored_rows.min(MAXIMUM_PRESENTATION_ROWS)
    } else {
        authored_rows
    };
    let row_mode = u16::from_le_bytes([rows, entry.row_mode.to_le_bytes()[1]]);
    let rows = usize::from(rows);
    let right = x.checked_add(width);
    let bottom = y.checked_add(rows);
    if right.is_none_or(|right| right > LOGICAL_FRAMEBUFFER_WIDTH)
        || bottom.is_none_or(|bottom| bottom > LOGICAL_FRAMEBUFFER_HEIGHT)
    {
        return Err(PresentationActiveEntryError::RectangleOutOfBounds { x, y, width, rows });
    }
    Ok(PresentationRectangle {
        pixels: body,
        x,
        y,
        width,
        row_mode,
    })
}

/// Retire and present one active queue entry in original operation order.
///
/// This translates `list_d8c_active_present` at BLOODPRG offset `0x00A41A`.
/// Typed ownership replaces the active/retired far-pointer exchange. Ordinary
/// frames retain coordinate parsing, destination selection, presentation
/// ordering, transparency mode, and the 130-row clamp; deferred compressed
/// rectangles use the flat AD decoder directly.
pub fn present_active_entry(
    state: &mut PresentationActiveEntryState,
    policy: PresentationPresentPolicy,
    presenter: &mut impl PresentationEntryPresenter,
) -> Result<PresentationActiveEntryOutcome, PresentationActiveEntryError> {
    let Some(entry) = state.active.take() else {
        state.retired = None;
        return Ok(PresentationActiveEntryOutcome::default());
    };
    state.frame_presented = true;
    state.retired = Some(entry);
    let entry = state
        .retired
        .as_ref()
        .expect("active entry was moved into retired ownership");
    let mut outcome = PresentationActiveEntryOutcome {
        frame_presented: true,
        ..PresentationActiveEntryOutcome::default()
    };

    if policy.draw_via_back_buffer {
        if entry.row_mode as u8 != u8::MIN {
            let rectangle = presentation_rectangle(entry, policy, true)?;
            let result = presenter.blit_rectangle(
                rectangle.pixels,
                PresentationEntryRenderTarget::BackBuffer,
                rectangle.x,
                rectangle.y,
                rectangle.width,
                rectangle.row_mode,
            )?;
            outcome.rectangle_blit = Some((PresentationEntryRenderTarget::BackBuffer, result));
        }
        presenter.present_back_buffer()?;
        outcome.back_buffer_presented = true;
        return Ok(outcome);
    }

    if !policy.skip_back_buffer_present {
        presenter.present_back_buffer()?;
        outcome.back_buffer_presented = true;
    }
    if let PresentationEntryFrame::DeferredTransparent(source) = &entry.frame {
        outcome.rectangle_decode = Some(presenter.decode_rectangle(
            source,
            policy.vertical_offset,
            entry.layout,
            entry.row_mode,
        )?);
        return Ok(outcome);
    }
    if entry.row_mode as u8 == u8::MIN {
        return Ok(outcome);
    }

    let rectangle = presentation_rectangle(entry, policy, !policy.unclamped_rows)?;
    let result = presenter.blit_rectangle(
        rectangle.pixels,
        PresentationEntryRenderTarget::Display,
        rectangle.x,
        rectangle.y,
        rectangle.width,
        rectangle.row_mode,
    )?;
    outcome.rectangle_blit = Some((PresentationEntryRenderTarget::Display, result));
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::PresentationEntryStorage;

    const PRESENT_VECTOR_COUNT: usize = 10;
    const FLAT_PRESENT_VECTOR_COUNT: usize = 9;
    const PIXEL_PATTERN: u8 = 0x5A;

    #[derive(Deserialize)]
    struct PresentOracle {
        name: String,
        active: bool,
        header: u16,
        masked_width: usize,
        row_mode: u16,
        coordinates: Option<[u16; 2]>,
        vertical_offset: usize,
        adjusted_y: usize,
        direction: String,
        back_buffer_mode: bool,
        skip_present: bool,
        compressed: bool,
        unclamped_rows: bool,
        calls: Vec<serde_json::Value>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum RecordedCall {
        Present,
        Blit {
            target: PresentationEntryRenderTarget,
            x: usize,
            y: usize,
            width: usize,
            row_mode: u16,
        },
        Decode {
            vertical_offset: usize,
            layout: u16,
            row_mode: u16,
        },
    }

    #[derive(Default)]
    struct RecordingPresenter {
        calls: Vec<RecordedCall>,
    }

    impl PresentationEntryPresenter for RecordingPresenter {
        fn present_back_buffer(&mut self) -> Result<(), PresentationActiveEntryError> {
            self.calls.push(RecordedCall::Present);
            Ok(())
        }

        fn blit_rectangle(
            &mut self,
            _source: &[u8],
            target: PresentationEntryRenderTarget,
            x: usize,
            y: usize,
            width: usize,
            row_mode: u16,
        ) -> Result<PresentationRectBlitOutcome, PresentationActiveEntryError> {
            self.calls.push(RecordedCall::Blit {
                target,
                x,
                y,
                width,
                row_mode,
            });
            Ok(PresentationRectBlitOutcome {
                consumed_bytes: usize::MIN,
                changed_pixels: usize::MIN,
            })
        }

        fn decode_rectangle(
            &mut self,
            _source: &[u8],
            vertical_offset: usize,
            layout: u16,
            row_mode: u16,
        ) -> Result<PresentationRectDecodeOutcome, PresentationActiveEntryError> {
            self.calls.push(RecordedCall::Decode {
                vertical_offset,
                layout,
                row_mode,
            });
            Ok(PresentationRectDecodeOutcome {
                consumed_bytes: usize::MIN,
                staged_values_consumed: usize::MIN,
                changed_pixels: usize::MIN,
                x: usize::MIN,
                y: usize::MIN,
                width: usize::MIN,
                rows: usize::MIN,
                final_row_offset: usize::MIN,
                final_destination_offset: usize::MIN,
            })
        }
    }

    fn active_entry(vector: &PresentOracle) -> ActivatedPresentationEntry {
        let rows = usize::from(vector.row_mode as u8);
        let mut body = Vec::new();
        if let Some([x, y]) = vector.coordinates {
            body.extend_from_slice(&x.to_le_bytes());
            body.extend_from_slice(&y.to_le_bytes());
        }
        let required_pixels = vector.masked_width.saturating_mul(rows);
        body.resize(body.len() + required_pixels, PIXEL_PATTERN);

        let frame = if vector.row_mode as u8 == u8::MIN {
            PresentationEntryFrame::Empty
        } else if vector.compressed && !vector.back_buffer_mode {
            PresentationEntryFrame::DeferredTransparent(vec![PIXEL_PATTERN].into_boxed_slice())
        } else if vector.compressed {
            PresentationEntryFrame::Decoded(PresentationPayload::Ab(AbDecodeOutcome {
                bytes: body.into_boxed_slice(),
                consumed_bytes: usize::MIN,
            }))
        } else {
            PresentationEntryFrame::Encoded(body.into_boxed_slice())
        };
        ActivatedPresentationEntry {
            layout: vector.header,
            row_mode: vector.row_mode,
            storage: PresentationEntryStorage::Default,
            frame,
        }
    }

    fn expected_calls(vector: &PresentOracle) -> Vec<RecordedCall> {
        if !vector.active {
            return Vec::new();
        }
        let mut calls = Vec::new();
        let [x, authored_y] = vector.coordinates.unwrap_or([u16::MIN; 2]);
        let rows = vector.row_mode as u8;
        if vector.back_buffer_mode {
            if rows != u8::MIN {
                calls.push(RecordedCall::Blit {
                    target: PresentationEntryRenderTarget::BackBuffer,
                    x: usize::from(x),
                    y: vector.adjusted_y,
                    width: vector.masked_width,
                    row_mode: u16::from_le_bytes([
                        rows.min(MAXIMUM_PRESENTATION_ROWS),
                        vector.row_mode.to_le_bytes()[1],
                    ]),
                });
            }
            calls.push(RecordedCall::Present);
        } else {
            if !vector.skip_present {
                calls.push(RecordedCall::Present);
            }
            if vector.compressed {
                calls.push(RecordedCall::Decode {
                    vertical_offset: vector.vertical_offset,
                    layout: vector.header,
                    row_mode: vector.row_mode,
                });
            } else if rows != u8::MIN {
                calls.push(RecordedCall::Blit {
                    target: PresentationEntryRenderTarget::Display,
                    x: usize::from(x),
                    y: usize::from(authored_y) + vector.vertical_offset,
                    width: vector.masked_width,
                    row_mode: u16::from_le_bytes([
                        if vector.unclamped_rows {
                            rows
                        } else {
                            rows.min(MAXIMUM_PRESENTATION_ROWS)
                        },
                        vector.row_mode.to_le_bytes()[1],
                    ]),
                });
            }
        }
        calls
    }

    #[test]
    fn presentation_order_matches_every_flat_original_vector() {
        let vectors: Vec<PresentOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a41a_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PRESENT_VECTOR_COUNT);
        let mut matched = 0;

        for vector in vectors {
            assert!(
                matches!(vector.direction.as_str(), "forward" | "backward"),
                "{}",
                vector.name
            );
            let mut state = PresentationActiveEntryState {
                active: vector.active.then(|| active_entry(&vector)),
                retired: None,
                frame_presented: false,
                ..PresentationActiveEntryState::default()
            };
            let mut presenter = RecordingPresenter::default();
            let result = present_active_entry(
                &mut state,
                PresentationPresentPolicy {
                    draw_via_back_buffer: vector.back_buffer_mode,
                    skip_back_buffer_present: vector.skip_present,
                    unclamped_rows: vector.unclamped_rows,
                    vertical_offset: vector.vertical_offset,
                },
                &mut presenter,
            );

            if vector.name == "coordinate_and_source_wrap" {
                assert!(
                    matches!(
                        result,
                        Err(PresentationActiveEntryError::RectangleOutOfBounds { .. })
                    ),
                    "{}",
                    vector.name
                );
                continue;
            }
            let outcome = result.unwrap();
            assert_eq!(outcome.frame_presented, vector.active, "{}", vector.name);
            assert_eq!(presenter.calls, expected_calls(&vector), "{}", vector.name);
            assert_eq!(presenter.calls.len(), vector.calls.len(), "{}", vector.name);
            assert_eq!(state.active, None, "{}", vector.name);
            assert_eq!(state.retired.is_some(), vector.active, "{}", vector.name);
            matched += 1;
        }
        assert_eq!(matched, FLAT_PRESENT_VECTOR_COUNT);
    }

    #[test]
    fn flat_presenter_copies_and_blits_owned_framebuffers() {
        let pixel_count = LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT;
        let mut display = vec![u8::MIN; pixel_count];
        let mut back_buffer = vec![0x11; pixel_count];
        let mut staging = vec![u8::MIN; pixel_count];
        let mut presenter = FlatPresentationEntryPresenter {
            display_buffer: &mut display,
            back_buffer: &mut back_buffer,
            decode_staging: &mut staging,
        };
        let mut body = Vec::new();
        body.extend_from_slice(&2_u16.to_le_bytes());
        body.extend_from_slice(&3_u16.to_le_bytes());
        body.extend_from_slice(&[0x22; 6]);
        let mut state = PresentationActiveEntryState {
            active: Some(ActivatedPresentationEntry {
                layout: 3,
                row_mode: 2,
                storage: PresentationEntryStorage::Default,
                frame: PresentationEntryFrame::Encoded(body.into_boxed_slice()),
            }),
            ..PresentationActiveEntryState::default()
        };

        let outcome = present_active_entry(
            &mut state,
            PresentationPresentPolicy::default(),
            &mut presenter,
        )
        .unwrap();
        assert!(outcome.back_buffer_presented);
        let first_pixel = 3 * LOGICAL_FRAMEBUFFER_WIDTH + 2;
        assert_eq!(&display[first_pixel..first_pixel + 3], &[0x22; 3]);
        assert_eq!(display[0], 0x11);
    }
}
