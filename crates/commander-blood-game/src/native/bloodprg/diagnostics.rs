//! Startup diagnostic panel geometry and text composition.

use std::io::{self, Write};

use super::{append_decimal_i16, append_decimal_i32};

const LOGICAL_SCREEN_WIDTH: u16 = 320;
const LOGICAL_SCREEN_HEIGHT: u16 = 200;
const PANEL_HORIZONTAL_BORDER: u16 = 4;
const PANEL_VERTICAL_BORDER: u16 = 4;
const PANEL_CONTENT_INSET: u16 = 2;
const SMALL_FONT_CHARACTER_PITCH: u16 = 4;
const SMALL_FONT_ROW_HEIGHT: u16 = 6;
const ERROR_TEXT_PALETTE_INDEX: u8 = 15;
const PANEL_FILL_PALETTE_INDEX: u8 = 0;
const CODING_ERROR_ROW_COUNT: u16 = 1;
const FILE_ERROR_ROW_COUNT: u16 = 2;
const ALLOCATION_ERROR_ROW_COUNT: u16 = 3;
const CODING_ERROR_MODE: u16 = 0;
const FILE_ERROR_MODE: u16 = 1;
const ALLOCATION_ERROR_MODE: u16 = 2;

const CODING_ERROR_TEXT: &[u8] = b"ERREUR DE CODAGE !";
const FILE_ERROR_TEXT: &[u8] = b"ERREUR DE FICHIER :";
const ALLOCATION_ERROR_TEXT: &[u8] = b"ERREUR D'ALLOCATION MEMOIRE !";
const HANDLE_LABEL_TEXT: &[u8] = b"HANDLE : ";
const FREE_BYTES_LABEL_TEXT: &[u8] = b"LIBRE  : ";

/// One wrapping logical-screen rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticRectangle {
    /// Left coordinate.
    pub x: u16,
    /// Top coordinate.
    pub y: u16,
    /// Width in logical pixels.
    pub width: u16,
    /// Height in logical pixels.
    pub height: u16,
}

/// Centered diagnostic panel and its text origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticPanelLayout {
    /// Outer fill and outline rectangle.
    pub outer: DiagnosticRectangle,
    /// X coordinate of the first text cell.
    pub text_x: u16,
    /// Y coordinate of the first text row.
    pub text_y: u16,
    /// Palette index used to clear the panel.
    pub fill_palette_index: u8,
    /// Palette index used for the panel outline.
    pub outline_palette_index: u8,
}

/// Authored diagnostic selected by the startup coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorOverlayMode {
    /// Internal coding error.
    Coding,
    /// Resource or filesystem error followed by a caller detail line.
    File,
    /// Resource-allocation error with handle and free-byte diagnostics.
    Allocation,
}

/// Numeric startup error mode not recognized by the original dispatcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownErrorOverlayMode(pub u16);

impl TryFrom<u16> for ErrorOverlayMode {
    type Error = UnknownErrorOverlayMode;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            CODING_ERROR_MODE => Ok(Self::Coding),
            FILE_ERROR_MODE => Ok(Self::File),
            ALLOCATION_ERROR_MODE => Ok(Self::Allocation),
            _ => Err(UnknownErrorOverlayMode(value)),
        }
    }
}

/// One owned small-font line in an error overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorOverlayLine {
    /// Game-font bytes to render.
    pub text: Box<[u8]>,
    /// Logical text X coordinate.
    pub x: u16,
    /// Logical text Y coordinate.
    pub y: u16,
    /// Palette index used by the original small-font renderer.
    pub palette_index: u8,
}

/// Complete renderer-independent diagnostic overlay plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorOverlay {
    /// Panel geometry and colors.
    pub panel: DiagnosticPanelLayout,
    /// Ordered text lines drawn over the panel.
    pub lines: Box<[ErrorOverlayLine]>,
}

/// Write diagnostic bytes to a modern host console without adding a newline.
///
/// This is the host-facing replacement for `print_string_dos` at BLOODPRG
/// routine offset `0x000D61`. The original issued DOS function 2 once per byte;
/// one binary write preserves the observable byte stream without retaining the
/// interrupt ABI.
pub fn write_diagnostic_console_text(output: &mut impl Write, text: &[u8]) -> io::Result<()> {
    output.write_all(text)
}

/// Build the centered panel used by startup diagnostics.
///
/// This translates `layout_offset_calc` at BLOODPRG routine offset `0x000E62`.
/// All word-sized multiplication, subtraction, shifts, and returned coordinates
/// retain their wrapping behavior. Typed geometry and palette indices replace
/// immediate framebuffer draw calls.
pub fn calculate_diagnostic_panel_layout(columns: u16, rows: u16) -> DiagnosticPanelLayout {
    let width = columns
        .wrapping_mul(SMALL_FONT_CHARACTER_PITCH)
        .wrapping_add(PANEL_HORIZONTAL_BORDER);
    let height = rows
        .wrapping_mul(SMALL_FONT_ROW_HEIGHT)
        .wrapping_add(PANEL_VERTICAL_BORDER);
    let x = LOGICAL_SCREEN_WIDTH.wrapping_sub(width) >> 1;
    let y = LOGICAL_SCREEN_HEIGHT.wrapping_sub(height) >> 1;
    DiagnosticPanelLayout {
        outer: DiagnosticRectangle {
            x,
            y,
            width,
            height,
        },
        text_x: x.wrapping_add(PANEL_CONTENT_INSET),
        text_y: y.wrapping_add(PANEL_CONTENT_INSET),
        fill_palette_index: PANEL_FILL_PALETTE_INDEX,
        outline_palette_index: ERROR_TEXT_PALETTE_INDEX,
    }
}

/// Compose the selected startup error overlay from owned text lines.
///
/// This translates `error_overlay_draw` at BLOODPRG routine offset `0x000D75`.
/// Static French text, row placement, allocation diagnostics, decimal formatting,
/// and palette index remain exact. A draw plan replaces temporary VGA display-
/// segment rebasing and direct font/framebuffer calls.
pub fn build_error_overlay(
    mode: ErrorOverlayMode,
    detail: &[u8],
    resource_handle: i16,
    resource_free_bytes: i32,
) -> ErrorOverlay {
    match mode {
        ErrorOverlayMode::Coding => {
            let panel = calculate_diagnostic_panel_layout(
                CODING_ERROR_TEXT.len() as u16,
                CODING_ERROR_ROW_COUNT,
            );
            ErrorOverlay {
                panel,
                lines: vec![line(CODING_ERROR_TEXT, panel.text_x, panel.text_y)].into_boxed_slice(),
            }
        }
        ErrorOverlayMode::File => {
            let panel = calculate_diagnostic_panel_layout(
                FILE_ERROR_TEXT.len() as u16,
                FILE_ERROR_ROW_COUNT,
            );
            ErrorOverlay {
                panel,
                lines: vec![
                    line(FILE_ERROR_TEXT, panel.text_x, panel.text_y),
                    line(
                        detail,
                        panel.text_x,
                        panel.text_y.wrapping_add(SMALL_FONT_ROW_HEIGHT),
                    ),
                ]
                .into_boxed_slice(),
            }
        }
        ErrorOverlayMode::Allocation => {
            let panel = calculate_diagnostic_panel_layout(
                ALLOCATION_ERROR_TEXT.len() as u16,
                ALLOCATION_ERROR_ROW_COUNT,
            );
            let label_y = panel.text_y.wrapping_add(SMALL_FONT_ROW_HEIGHT);
            let free_y = label_y.wrapping_add(SMALL_FONT_ROW_HEIGHT);
            let numeric_x = panel.text_x.wrapping_add(
                (HANDLE_LABEL_TEXT.len() as u16).wrapping_mul(SMALL_FONT_CHARACTER_PITCH),
            );
            let mut handle_text = String::new();
            append_decimal_i16(&mut handle_text, resource_handle);
            let mut free_bytes_text = String::new();
            append_decimal_i32(&mut free_bytes_text, resource_free_bytes);
            ErrorOverlay {
                panel,
                lines: vec![
                    line(ALLOCATION_ERROR_TEXT, panel.text_x, panel.text_y),
                    line(HANDLE_LABEL_TEXT, panel.text_x, label_y),
                    line(handle_text.as_bytes(), numeric_x, label_y),
                    line(FREE_BYTES_LABEL_TEXT, panel.text_x, free_y),
                    line(free_bytes_text.as_bytes(), numeric_x, free_y),
                ]
                .into_boxed_slice(),
            }
        }
    }
}

fn line(text: &[u8], x: u16, y: u16) -> ErrorOverlayLine {
    ErrorOverlayLine {
        text: text.into(),
        x,
        y,
        palette_index: ERROR_TEXT_PALETTE_INDEX,
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const LAYOUT_ORACLE_VECTOR_COUNT: usize = 12;
    const ERROR_ORACLE_VECTOR_COUNT: usize = 5;

    #[derive(Deserialize)]
    struct LayoutOracle {
        columns: u16,
        rows: u16,
        outer: RectangleOracle,
        fill_color: u8,
        outline_color: u8,
        result: PointOracle,
    }

    #[derive(Deserialize)]
    struct RectangleOracle {
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    }

    #[derive(Deserialize)]
    struct PointOracle {
        x: u16,
        y: u16,
    }

    #[derive(Deserialize)]
    struct ErrorOracle {
        name: String,
        mode: u16,
        detail: String,
        resource_handle_signed: i16,
        resource_free_bytes_signed: i32,
        calls: Vec<ErrorCallOracle>,
    }

    #[derive(Deserialize)]
    struct ErrorCallOracle {
        callee: String,
        text: Option<String>,
        columns: Option<u16>,
        rows: Option<u16>,
        x: Option<u16>,
        y: Option<u16>,
        color: Option<u8>,
    }

    #[derive(Deserialize)]
    struct ConsoleOracle {
        payload: Vec<u8>,
        dos_calls: usize,
    }

    #[test]
    fn host_console_output_matches_every_original_payload_vector() {
        let vectors: Vec<ConsoleOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_0d61_natural.json"
        ))
        .unwrap();
        for vector in vectors {
            let mut output = Vec::new();
            write_diagnostic_console_text(&mut output, &vector.payload).unwrap();
            assert_eq!(output, vector.payload);
            assert_eq!(output.len(), vector.dos_calls);
        }
    }

    #[test]
    fn panel_layout_matches_every_original_wrapping_vector() {
        let vectors: Vec<LayoutOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_0e62_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), LAYOUT_ORACLE_VECTOR_COUNT);
        for vector in vectors {
            assert_eq!(
                calculate_diagnostic_panel_layout(vector.columns, vector.rows),
                DiagnosticPanelLayout {
                    outer: DiagnosticRectangle {
                        x: vector.outer.x,
                        y: vector.outer.y,
                        width: vector.outer.width,
                        height: vector.outer.height,
                    },
                    text_x: vector.result.x,
                    text_y: vector.result.y,
                    fill_palette_index: vector.fill_color,
                    outline_palette_index: vector.outline_color,
                }
            );
        }
    }

    #[test]
    fn overlay_composition_matches_every_original_mode_vector() {
        let vectors: Vec<ErrorOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_0d75_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ERROR_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let Ok(mode) = ErrorOverlayMode::try_from(vector.mode) else {
                assert_eq!(vector.name, "unknown_mode");
                assert!(vector.calls.is_empty());
                continue;
            };
            let overlay = build_error_overlay(
                mode,
                vector.detail.as_bytes(),
                vector.resource_handle_signed,
                vector.resource_free_bytes_signed,
            );
            let layout_call = vector
                .calls
                .iter()
                .find(|call| call.callee == "layout_offset_calc")
                .unwrap();
            assert_eq!(
                overlay.panel,
                calculate_diagnostic_panel_layout(
                    layout_call.columns.unwrap(),
                    layout_call.rows.unwrap(),
                ),
                "{}",
                vector.name
            );
            let original_lines = vector
                .calls
                .iter()
                .filter(|call| call.callee == "small_text_render")
                .collect::<Vec<_>>();
            assert_eq!(overlay.lines.len(), original_lines.len(), "{}", vector.name);
            for (line, original) in overlay.lines.iter().zip(original_lines) {
                assert_eq!(
                    line.text.as_ref(),
                    original.text.as_ref().unwrap().as_bytes()
                );
                assert_eq!(line.palette_index, original.color.unwrap());
            }
            assert_relative_line_geometry(&overlay, mode);
            assert!(
                vector
                    .calls
                    .iter()
                    .filter(|call| call.callee == "small_text_render")
                    .all(|call| call.x.is_some() && call.y.is_some())
            );
        }
    }

    fn assert_relative_line_geometry(overlay: &ErrorOverlay, mode: ErrorOverlayMode) {
        assert_eq!(overlay.lines[0].x, overlay.panel.text_x);
        assert_eq!(overlay.lines[0].y, overlay.panel.text_y);
        match mode {
            ErrorOverlayMode::Coding => assert_eq!(overlay.lines.len(), 1),
            ErrorOverlayMode::File => {
                assert_eq!(overlay.lines[1].x, overlay.panel.text_x);
                assert_eq!(
                    overlay.lines[1].y,
                    overlay.panel.text_y + SMALL_FONT_ROW_HEIGHT
                );
            }
            ErrorOverlayMode::Allocation => {
                let numeric_x = overlay.panel.text_x
                    + HANDLE_LABEL_TEXT.len() as u16 * SMALL_FONT_CHARACTER_PITCH;
                assert_eq!(overlay.lines[1].x, overlay.panel.text_x);
                assert_eq!(overlay.lines[2].x, numeric_x);
                assert_eq!(overlay.lines[1].y, overlay.lines[2].y);
                assert_eq!(overlay.lines[3].x, overlay.panel.text_x);
                assert_eq!(overlay.lines[4].x, numeric_x);
                assert_eq!(overlay.lines[3].y, overlay.lines[4].y);
                assert_eq!(
                    overlay.lines[3].y,
                    overlay.lines[1].y + SMALL_FONT_ROW_HEIGHT
                );
            }
        }
    }
}
