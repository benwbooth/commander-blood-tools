//! Typed bridge choice-list layout, interaction, and render planning.

/// Horizontal padding surrounding the widest choice label.
pub const CHOICE_LIST_WIDTH_PADDING: u16 = 20;
/// Vertical distance between adjacent choice rows.
pub const CHOICE_LIST_ROW_PITCH: u16 = 11;

const DEFAULT_CONTENT_WIDTH: u16 = 100;
const CANCEL_CONTENT_WIDTH: u16 = 55;
const CANCEL_EXTRA_HEIGHT: u16 = 10;
const HEIGHT_PADDING: u16 = 8;
const LOGICAL_SCREEN_HEIGHT: u16 = 200;
const TEXT_X_INSET: u16 = 10;
const TEXT_Y_INSET: u16 = 4;
const DEFAULT_COLOR: u8 = 232;
const HOVER_COLOR: u8 = 239;
const ACTIVE_COLOR: u8 = 254;

/// One pointer sample in the original 320 by 200 logical coordinate space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChoiceListPointer {
    /// Current signed logical position.
    pub position: [i16; 2],
    /// Whether the primary button is currently pressed.
    pub primary_pressed: bool,
}

/// Flat logical rectangle occupied by one choice list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChoiceListRect {
    /// Signed upper-left logical coordinate.
    pub origin: [i16; 2],
    /// Unsigned wrapped dimensions recovered from the original layout.
    pub size: [u16; 2],
}

/// Configurable behavior shared by bridge, options, and dialogue lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChoiceListConfig<'a> {
    /// Horizontal center in logical screen coordinates.
    pub center_x: i16,
    /// Keep each measured width when centering labels within the list.
    pub preserve_individual_widths: bool,
    /// Optional final cancel row.
    pub cancel_label: Option<&'a [u8]>,
    /// Measure and position the list without touching interaction state.
    pub layout_only: bool,
}

/// Host services whose ordering is observable in the recovered routine.
pub trait ChoiceListBackend {
    /// Measure one label using the game's square-cap font rules.
    fn measure_label(&mut self, label: &[u8]) -> u16;

    /// Prepare the background under an interactive list.
    fn prepare_background(&mut self, rect: ChoiceListRect);

    /// Read the pointer after background preparation has completed.
    fn pointer(&mut self) -> ChoiceListPointer;

    /// Return the live selector aliased with `nav_target_presentation_state`.
    fn current_hand_animation(&self) -> u16 {
        u16::MIN
    }

    /// Publish the selector writes produced by this list interaction.
    fn request_hand_animation(&mut self, _request: ChoiceListHandRequest) {}
}

/// MANU3 selectors owned by the recovered shared list widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum ChoiceListHandAnimation {
    /// Pointer is outside the list.
    Idle = 1,
    /// Pointer is over one row.
    Hover = 6,
    /// Primary pointer button is down over one row.
    Active = 7,
}

impl ChoiceListHandAnimation {
    /// Return the exact selector stored by the executable.
    pub const fn value(self) -> u16 {
        self as u16
    }
}

/// Ordered shared-selector update emitted by one list interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChoiceListHandRequest {
    /// Selector written to `DS:0x0A32`.
    pub animation: ChoiceListHandAnimation,
    /// Whether `DS:0x0A34` is cleared before the request write.
    pub restart_current: bool,
}

/// Semantic presentation state for the currently active list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChoiceListPresentation {
    /// The pointer is outside the list.
    #[default]
    Idle,
    /// One row is under the pointer.
    Hover,
    /// The primary button is down over one row.
    Active,
}

/// Persistent interaction state for one list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChoiceListState {
    /// Current pointer-driven presentation mode.
    pub presentation: ChoiceListPresentation,
    /// Current row under the pointer, including the optional cancel row.
    pub hovered_row: Option<usize>,
}

/// Identity of one rendered row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceListRowKind {
    /// An authored item at the contained zero-based index.
    Item(usize),
    /// The synthetic final cancel row.
    Cancel,
}

/// One text draw requested by the list renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChoiceListRow {
    /// Semantic row identity.
    pub kind: ChoiceListRowKind,
    /// Upper-left text position in logical coordinates.
    pub position: [u16; 2],
    /// Palette index selected by interaction state.
    pub color: u8,
}

/// Complete render and interaction result for one list update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceListFrame {
    /// List rectangle prepared for this frame.
    pub rect: ChoiceListRect,
    /// Text rows to draw; empty for a layout-only pass.
    pub rows: Vec<ChoiceListRow>,
    /// Selected authored item. A cancel-row click intentionally yields `None`.
    pub selected_item: Option<usize>,
    /// Whether the optional cancel row was selected.
    pub cancelled: bool,
}

/// Lay out and update one bridge-style choice list.
///
/// This translates `list_widget_layout_unified` at BLOODPRG routine offset
/// `0x008428`. Borrowed labels and measured widths replace near arrays, far
/// string pointers, sentinels, and record-name offset substitution. The result
/// is a renderer-neutral plan suitable for wgpu while preserving native pixel
/// layout, inclusive hit edges, callback order, and wrapping coordinate math.
pub fn update_choice_list<Backend: ChoiceListBackend>(
    labels: &[&[u8]],
    config: ChoiceListConfig<'_>,
    state: &mut ChoiceListState,
    backend: &mut Backend,
) -> ChoiceListFrame {
    let mut row_extent = if config.cancel_label.is_some() {
        CANCEL_EXTRA_HEIGHT
    } else {
        u16::MIN
    };
    let mut max_width = if config.cancel_label.is_some() {
        CANCEL_CONTENT_WIDTH
    } else {
        DEFAULT_CONTENT_WIDTH
    };
    let mut measured_widths =
        Vec::with_capacity(labels.len() + usize::from(config.cancel_label.is_some()));

    for label in labels {
        let width = backend.measure_label(label);
        max_width = max_width.max(width);
        measured_widths.push(width);
        row_extent = row_extent.wrapping_add(CHOICE_LIST_ROW_PITCH);
    }
    if config.cancel_label.is_some() {
        measured_widths.push(CANCEL_CONTENT_WIDTH);
    }
    if !config.preserve_individual_widths {
        measured_widths.fill(max_width);
    }

    let width = max_width.wrapping_add(CHOICE_LIST_WIDTH_PADDING);
    let height = row_extent.wrapping_add(HEIGHT_PADDING);
    let rect = ChoiceListRect {
        origin: [
            config.center_x.wrapping_sub((width >> 1) as i16),
            (LOGICAL_SCREEN_HEIGHT.wrapping_sub(height) >> 1) as i16,
        ],
        size: [width, height],
    };
    if config.layout_only {
        return ChoiceListFrame {
            rect,
            rows: Vec::new(),
            selected_item: None,
            cancelled: false,
        };
    }

    backend.prepare_background(rect);
    let pointer = backend.pointer();
    let row_count = labels.len() + usize::from(config.cancel_label.is_some());
    state.hovered_row = hovered_row(rect, pointer.position).filter(|row| *row < row_count);
    state.presentation = match state.hovered_row {
        None => ChoiceListPresentation::Idle,
        Some(_) if pointer.primary_pressed => ChoiceListPresentation::Active,
        Some(_) => ChoiceListPresentation::Hover,
    };
    publish_hand_animation(state.presentation, backend);

    let selected_row = pointer
        .primary_pressed
        .then_some(state.hovered_row)
        .flatten();
    let selected_item = selected_row.filter(|row| *row < labels.len());
    let cancelled = selected_row == config.cancel_label.map(|_| labels.len());
    let content_width = width.wrapping_sub(CHOICE_LIST_WIDTH_PADDING);
    let row_x = (rect.origin[0] as u16).wrapping_add(TEXT_X_INSET);
    let mut row_y = (rect.origin[1] as u16).wrapping_add(TEXT_Y_INSET);
    let mut rows = Vec::with_capacity(row_count);

    for (index, measured_width) in measured_widths
        .iter()
        .copied()
        .enumerate()
        .take(labels.len())
    {
        rows.push(ChoiceListRow {
            kind: ChoiceListRowKind::Item(index),
            position: [
                row_x.wrapping_add(content_width.wrapping_sub(measured_width) >> 1),
                row_y,
            ],
            color: row_color(index, state.hovered_row, pointer.primary_pressed),
        });
        row_y = row_y.wrapping_add(CHOICE_LIST_ROW_PITCH);
    }
    if config.cancel_label.is_some() {
        let index = labels.len();
        rows.push(ChoiceListRow {
            kind: ChoiceListRowKind::Cancel,
            position: [
                row_x.wrapping_add(content_width.wrapping_sub(measured_widths[index]) >> 1),
                row_y,
            ],
            color: row_color(index, state.hovered_row, pointer.primary_pressed),
        });
    }

    ChoiceListFrame {
        rect,
        rows,
        selected_item,
        cancelled,
    }
}

fn publish_hand_animation<Backend: ChoiceListBackend>(
    presentation: ChoiceListPresentation,
    backend: &mut Backend,
) {
    let animation = match presentation {
        ChoiceListPresentation::Idle => ChoiceListHandAnimation::Idle,
        ChoiceListPresentation::Hover => ChoiceListHandAnimation::Hover,
        ChoiceListPresentation::Active => ChoiceListHandAnimation::Active,
    };
    let restart_current = match presentation {
        ChoiceListPresentation::Active => {
            if backend.current_hand_animation() != ChoiceListHandAnimation::Hover.value() {
                backend.request_hand_animation(ChoiceListHandRequest {
                    animation: ChoiceListHandAnimation::Hover,
                    restart_current: true,
                });
            }
            false
        }
        ChoiceListPresentation::Idle | ChoiceListPresentation::Hover => {
            backend.current_hand_animation() != animation.value()
        }
    };

    if presentation == ChoiceListPresentation::Active || restart_current {
        backend.request_hand_animation(ChoiceListHandRequest {
            animation,
            restart_current,
        });
    }
}

fn hovered_row(rect: ChoiceListRect, pointer: [i16; 2]) -> Option<usize> {
    let right = rect.origin[0].wrapping_add(rect.size[0] as i16);
    if pointer[0] < rect.origin[0] || pointer[0] > right {
        return None;
    }
    let row_y = (rect.origin[1] as u16).wrapping_add(TEXT_Y_INSET);
    let offset = (pointer[1] as u16).wrapping_sub(row_y);
    let clickable_height = rect.size[1].wrapping_sub(HEIGHT_PADDING);
    if (offset as i16) < 0 || (offset as i16) >= clickable_height as i16 {
        return None;
    }
    Some(usize::from(offset / CHOICE_LIST_ROW_PITCH))
}

fn row_color(index: usize, hovered: Option<usize>, pressed: bool) -> u8 {
    if hovered == Some(index) {
        if pressed { ACTIVE_COLOR } else { HOVER_COLOR }
    } else {
        DEFAULT_COLOR
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 15;
    const CANCEL_LABEL: &[u8] = b"CANCEL";
    const SEEDED_REQUEST_SELECTOR: u16 = 0x3344;

    #[derive(Deserialize)]
    struct ListOracle {
        name: String,
        items: Vec<u16>,
        rect: [i16; 4],
        calls: Vec<serde_json::Value>,
        return_value: i16,
        presentation_states: [u16; 2],
        split_game_stack_segments: bool,
    }

    struct OracleBackend {
        widths: Vec<u16>,
        measured: usize,
        pointer: ChoiceListPointer,
        pointer_after_prepare: Option<ChoiceListPointer>,
        prepared: Vec<ChoiceListRect>,
        current_hand_animation: u16,
        requested_hand_animation: u16,
        hand_requests: Vec<ChoiceListHandRequest>,
    }

    impl ChoiceListBackend for OracleBackend {
        fn measure_label(&mut self, _label: &[u8]) -> u16 {
            let width = self.widths[self.measured];
            self.measured += 1;
            width
        }

        fn prepare_background(&mut self, rect: ChoiceListRect) {
            self.prepared.push(rect);
            if let Some(pointer) = self.pointer_after_prepare {
                self.pointer = pointer;
            }
        }

        fn pointer(&mut self) -> ChoiceListPointer {
            self.pointer
        }

        fn current_hand_animation(&self) -> u16 {
            self.current_hand_animation
        }

        fn request_hand_animation(&mut self, request: ChoiceListHandRequest) {
            if request.restart_current {
                self.current_hand_animation = u16::MIN;
            }
            self.requested_hand_animation = request.animation.value();
            self.hand_requests.push(request);
        }
    }

    #[test]
    fn layout_and_interaction_match_every_flat_original_vector() {
        let vectors: Vec<ListOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_8428_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let label_count = vector
                .items
                .iter()
                .take_while(|item| **item != u16::MIN && **item != u16::MAX)
                .count();
            let labels = vec![b"LABEL".as_slice(); label_count];
            let mut backend = backend_for(&vector.name, label_count, vector.presentation_states[0]);
            let config = config_for(&vector.name);
            let mut state = ChoiceListState::default();
            let frame = update_choice_list(&labels, config, &mut state, &mut backend);

            assert_eq!(
                frame.rect,
                ChoiceListRect {
                    origin: [vector.rect[0], vector.rect[1]],
                    size: [vector.rect[2] as u16, vector.rect[3] as u16],
                },
                "{}",
                vector.name
            );
            assert_eq!(backend.measured, label_count, "{}", vector.name);
            let expected_draws = vector
                .calls
                .iter()
                .filter(|call| call["call"] == "square_caps_text_draw_display")
                .collect::<Vec<_>>();
            assert_eq!(frame.rows.len(), expected_draws.len(), "{}", vector.name);
            if !vector.split_game_stack_segments {
                for (row, expected) in frame.rows.iter().zip(expected_draws) {
                    assert_eq!(
                        row.position[0],
                        expected["x"].as_u64().unwrap() as u16,
                        "{}",
                        vector.name
                    );
                    assert_eq!(
                        row.position[1],
                        expected["y"].as_u64().unwrap() as u16,
                        "{}",
                        vector.name
                    );
                    assert_eq!(
                        row.color,
                        expected["color"].as_u64().unwrap() as u8,
                        "{}",
                        vector.name
                    );
                }
            }

            if vector.split_game_stack_segments {
                assert!(
                    frame.selected_item.is_none_or(|index| index < labels.len()),
                    "flat state must reject the oracle's split-segment alias"
                );
            } else {
                assert_eq!(
                    frame.selected_item.map(|index| index as i16),
                    (vector.return_value >= 0).then_some(vector.return_value),
                    "{}",
                    vector.name
                );
            }
            assert_eq!(
                backend.current_hand_animation, vector.presentation_states[0],
                "{}",
                vector.name
            );
            assert_eq!(
                backend.requested_hand_animation, vector.presentation_states[1],
                "{}",
                vector.name
            );
            assert_eq!(
                backend.hand_requests,
                expected_hand_requests(&vector.name),
                "{}",
                vector.name
            );
        }
    }

    fn config_for(name: &str) -> ChoiceListConfig<'static> {
        let layout_only = matches!(
            name,
            "empty_zero_prepass"
                | "extra_sentinel_prepass_double_fill"
                | "width_floor_double_fill"
                | "per_label_widths_preserved"
                | "width_helper_adds_extra_entry"
        );
        let cancel_label = matches!(
            name,
            "extra_sentinel_prepass_double_fill" | "extra_cancel_row_hover"
        )
        .then_some(CANCEL_LABEL);
        let preserve_individual_widths = !matches!(
            name,
            "empty_zero_prepass" | "extra_sentinel_prepass_double_fill" | "width_floor_double_fill"
        );
        ChoiceListConfig {
            center_x: 100,
            preserve_individual_widths,
            cancel_label,
            layout_only,
        }
    }

    fn backend_for(name: &str, label_count: usize, current_hand_animation: u16) -> OracleBackend {
        let widths = match name {
            "width_floor_double_fill" => vec![20, 30],
            "per_label_widths_preserved" => vec![20, 140, 80],
            "active_save_name_substitution" => vec![44],
            "width_helper_adds_extra_entry" => vec![25],
            "second_row_hover" | "second_row_click_active" => vec![20, 80],
            "extra_cancel_row_hover" => vec![20],
            _ => vec![30; label_count],
        };
        let (pointer, pointer_after_prepare) = match name {
            "outside_left_requests_idle" => (pointer([39, 94], false), None),
            "left_top_boundary_hover" => (pointer([40, 94], false), None),
            "right_bottom_boundary_hover" => (pointer([160, 104], false), None),
            "below_click_band_is_outside" => (pointer([100, 105], false), None),
            "second_row_hover" => (pointer([100, 100], false), None),
            "second_row_click_active" => (pointer([100, 100], true), None),
            "extra_cancel_row_hover" => (pointer([100, 100], false), None),
            "remap_mutates_mouse_before_hittest" => {
                (pointer([39, 94], false), Some(pointer([100, 94], false)))
            }
            "split_ds_es_gs_ss_ownership" => (pointer([100, 94], true), None),
            _ => (ChoiceListPointer::default(), None),
        };
        OracleBackend {
            widths,
            measured: 0,
            pointer,
            pointer_after_prepare,
            prepared: Vec::new(),
            current_hand_animation,
            requested_hand_animation: SEEDED_REQUEST_SELECTOR,
            hand_requests: Vec::new(),
        }
    }

    fn expected_hand_requests(name: &str) -> Vec<ChoiceListHandRequest> {
        let request = |animation, restart_current| ChoiceListHandRequest {
            animation,
            restart_current,
        };
        match name {
            "outside_left_requests_idle" | "below_click_band_is_outside" => {
                vec![request(ChoiceListHandAnimation::Idle, true)]
            }
            "left_top_boundary_hover"
            | "second_row_hover"
            | "remap_mutates_mouse_before_hittest" => {
                vec![request(ChoiceListHandAnimation::Hover, true)]
            }
            "second_row_click_active" => {
                vec![request(ChoiceListHandAnimation::Active, false)]
            }
            "split_ds_es_gs_ss_ownership" => vec![
                request(ChoiceListHandAnimation::Hover, true),
                request(ChoiceListHandAnimation::Active, false),
            ],
            _ => Vec::new(),
        }
    }

    const fn pointer(position: [i16; 2], primary_pressed: bool) -> ChoiceListPointer {
        ChoiceListPointer {
            position,
            primary_pressed,
        }
    }
}
