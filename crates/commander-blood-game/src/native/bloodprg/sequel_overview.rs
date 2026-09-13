//! Big Bug Bang's bridge simulation-overview controller and draw plan.

use super::NavActorSlotFlags;

const ACTOR_ACTIVE_FLAG: u16 = 1;
const ACTOR_ELIGIBLE_FLAG: u16 = 4;
const ACTOR_CONFLICT_FLAG: u16 = 8;
const PANEL_X: u16 = 3;
const PANEL_BOTTOM_Y: u16 = 197;
const PANEL_BASE_WIDTH: u16 = 132;
const PANEL_COLUMN_WIDTH: u16 = 90;
const PANEL_MAX_HEIGHT: u16 = 40;
const PANEL_ROW_HEIGHT: u16 = 7;
const PANEL_BASE_HEIGHT: u16 = 4;
const PANEL_ROWS_PER_COLUMN: u8 = 5;
const ALL_LABEL_X: u16 = 6;
const ALL_LABEL_WIDTH: u16 = 30;
const GROUP_LABEL_X: u16 = 46;
const GROUP_HIT_WIDTH: u16 = 90;
const MARKER_WIDTH: u16 = 10;
const MARKER_HEIGHT: u16 = 9;
const MARKER_CENTER_X: u16 = 4;
const MARKER_CENTER_Y: u16 = 3;
const PANEL_FILL_COLOR: u8 = 23;
const PANEL_LIGHT_EDGE_COLOR: u8 = 26;
const PANEL_DARK_EDGE_COLOR: u8 = 21;
const NORMAL_COLOR: u8 = 0xFE;
const INACTIVE_COLOR: u8 = 0xEE;
const CONFLICT_COLOR: u8 = 0xFC;
const UNSTABLE_COLOR: u8 = 0x79;
const OVERVIEW_HAND_SELECTOR: u16 = 1;
const CAMERA_REOPEN_FLAGS: u8 = 9;
const UI_BLOCK_MASK: u8 = 3;

/// Native high-bit-first category labels at BLOOD2PG file offset `0x16EAE`.
pub const SEQUEL_OVERVIEW_GROUP_LABELS: [&[u8]; 16] = [
    b"croolis_red",
    b"croolis_green",
    b"migrax",
    b"slimers",
    b"izwals",
    b"sinox",
    b"waves",
    b"tromps",
    b"kam",
    b"tubular_brain",
    b"quizzers",
    b"zen",
    b"scruters",
    b"robots",
    b"bob",
    b"gluxx",
];

/// Controller globals retained between bridge frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SequelOverviewState {
    /// Whether the overview currently owns its bridge overlay.
    pub active: bool,
    /// Whether every qualifying actor is shown instead of one selected group.
    pub show_all: bool,
    /// Groups represented by qualifying actors when the overlay was opened.
    pub available_group_mask: u16,
    /// Population count of `available_group_mask`.
    pub available_group_count: u8,
    /// Group hovered during the previous completed frame.
    pub hovered_group_mask: u16,
    /// Group selected by the latest category click.
    pub selected_group_mask: u16,
    /// Groups containing a conflicting actor in this frame's selected roster.
    pub conflict_group_mask: u16,
    /// Groups containing an unstable actor in this frame's selected roster.
    pub unstable_group_mask: u16,
}

/// Shared input and actor-slot values read or consumed by the controller.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SequelOverviewControl {
    /// Native `0x2A5F` gate; any nonzero word blocks the whole controller.
    pub blocked_word: u16,
    /// Low two UI bits that block the whole controller.
    pub ui_flags: u8,
    /// Current logical pointer position.
    pub pointer: [u16; 2],
    /// Primary edge latch consumed only by a hit label.
    pub primary_pressed: bool,
    /// Secondary edge latch that opens or closes the overlay.
    pub secondary_pressed: bool,
    /// Shared pending-press byte cleared with a consumed secondary edge.
    pub press_pending: u8,
    /// Current MANU3 hand selector.
    pub hand_selector: u16,
    /// Camera actor's low flag byte represented as typed flags.
    pub camera_actor_flags: NavActorSlotFlags,
}

/// One decoded actor and its resolved chart markers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SequelOverviewActor {
    /// Low actor state flags at byte two.
    pub flags: u16,
    /// Whether the actor's state header carries the in-play bit.
    pub in_play: bool,
    /// One-hot or combined category mask at actor byte 20.
    pub group_mask: u16,
    /// Current quantity at actor byte 22.
    pub quantity: u16,
    /// Balance threshold at actor byte 52.
    pub balance: u16,
    /// Whether the direct holder carries the active bit.
    pub holder_active: bool,
    /// Whether the direct holder carries the in-play bit.
    pub holder_in_play: bool,
    /// Whether the direct holder is the special `Trashlando` binding.
    pub holder_excluded: bool,
    /// Position returned by the native navigation-position resolver.
    pub position: [u16; 2],
    /// Resolved opponent position used only while the conflict flag is set.
    pub opponent_position: Option<[u16; 2]>,
}

impl SequelOverviewActor {
    const fn active_and_eligible(self) -> bool {
        self.flags & (ACTOR_ACTIVE_FLAG | ACTOR_ELIGIBLE_FLAG)
            == ACTOR_ACTIVE_FLAG | ACTOR_ELIGIBLE_FLAG
    }

    const fn conflict(self) -> bool {
        self.flags & ACTOR_CONFLICT_FLAG != 0
    }

    const fn unstable(self) -> bool {
        (self.quantity as i16) > (self.balance as i16)
    }

    const fn qualifies_for_all(self) -> bool {
        self.in_play && self.active_and_eligible() && self.holder_in_play
    }

    const fn qualifies_for_group(self, group_mask: u16) -> bool {
        self.in_play
            && self.active_and_eligible()
            && self.group_mask & group_mask != 0
            && !self.holder_excluded
            && self.holder_active
            && self.holder_in_play
    }
}

/// Text source selected by one draw operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequelOverviewLabel {
    /// Static `ALL` label at native data offset `0x01FA`.
    All,
    /// One of the 16 group labels in its native bit index.
    Group(u8),
}

impl SequelOverviewLabel {
    /// Return the exact executable-authored ASCII bytes.
    pub fn text(self) -> &'static [u8] {
        match self {
            Self::All => b"ALL",
            Self::Group(index) => SEQUEL_OVERVIEW_GROUP_LABELS[usize::from(index)],
        }
    }
}

/// Ordered flat-renderer operation emitted by one controller pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequelOverviewDraw {
    /// Solid panel background.
    Fill {
        /// Top-left logical pixel.
        origin: [u16; 2],
        /// Width and height in logical pixels.
        extent: [u16; 2],
        /// Palette index.
        color: u8,
    },
    /// Top, bottom, or other horizontal panel edge.
    Horizontal {
        /// Leftmost logical pixel.
        origin: [u16; 2],
        /// Horizontal pixel count.
        extent: u16,
        /// Palette index.
        color: u8,
    },
    /// Left, right, or other vertical panel edge.
    Vertical {
        /// Topmost logical pixel.
        origin: [u16; 2],
        /// Vertical pixel count.
        extent: u16,
        /// Palette index.
        color: u8,
    },
    /// Actor marker rectangle.
    Outline {
        /// Top-left logical pixel.
        origin: [u16; 2],
        /// Width and height in logical pixels.
        extent: [u16; 2],
        /// Palette index.
        color: u8,
    },
    /// Conflict relation between actor and opponent marker centers.
    Line {
        /// First logical endpoint.
        start: [u16; 2],
        /// Second logical endpoint.
        end: [u16; 2],
        /// Palette index.
        color: u8,
    },
    /// Planar square-cap label.
    Text {
        /// Executable-authored label selector.
        label: SequelOverviewLabel,
        /// Top-left logical pen position.
        origin: [u16; 2],
        /// Palette index.
        color: u8,
    },
}

/// Observable control path taken by one pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequelOverviewOutcome {
    /// A native gate returned before consuming input or touching state.
    Blocked,
    /// The inactive controller had no secondary edge to consume.
    Inactive,
    /// A secondary edge opened the overlay and this frame rendered it.
    Opened,
    /// A secondary edge closed the overlay before any rendering.
    Closed,
    /// An active overlay rendered normally.
    Rendered,
    /// No available categories remained, so the controller closed itself.
    ClosedEmpty,
}

/// Draw plan and path result for one translated frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequelOverviewFrame {
    /// Control path taken by the update.
    pub outcome: SequelOverviewOutcome,
    /// Native-order draw operations.
    pub draws: Vec<SequelOverviewDraw>,
}

/// Translate BLOOD2PG's complete controller at file offset `0xA286`.
pub fn update_sequel_overview(
    actors: &[SequelOverviewActor],
    control: &mut SequelOverviewControl,
    state: &mut SequelOverviewState,
) -> SequelOverviewFrame {
    if control.blocked_word != 0 || control.ui_flags & UI_BLOCK_MASK != 0 {
        return frame(SequelOverviewOutcome::Blocked, Vec::new());
    }

    let mut opened = false;
    if control.secondary_pressed {
        control.secondary_pressed = false;
        control.press_pending = 0;
        if state.active {
            close(state, control);
            return frame(SequelOverviewOutcome::Closed, Vec::new());
        }
        opened = true;
        control.hand_selector = OVERVIEW_HAND_SELECTOR;
        control.camera_actor_flags = NavActorSlotFlags::default();
        state.active = true;
        state.show_all = true;
        state.available_group_mask = actors
            .iter()
            .copied()
            .filter(|actor| actor.qualifies_for_all())
            .fold(0, |mask, actor| mask | actor.group_mask);
        state.available_group_count = state.available_group_mask.count_ones() as u8;
    }
    if !state.active {
        return frame(SequelOverviewOutcome::Inactive, Vec::new());
    }

    state.conflict_group_mask = 0;
    state.unstable_group_mask = 0;
    let mut draws = Vec::new();
    if state.show_all {
        for actor in actors
            .iter()
            .copied()
            .filter(|actor| actor.qualifies_for_all())
        {
            render_actor(actor, true, state, &mut draws);
        }
    } else {
        let prior_hover = state.hovered_group_mask;
        if prior_hover != 0 {
            for actor in actors
                .iter()
                .copied()
                .filter(|actor| actor.qualifies_for_group(prior_hover))
            {
                render_actor(actor, false, state, &mut draws);
            }
        }
        let selected_group_mask = state.selected_group_mask;
        if selected_group_mask != 0 {
            for actor in actors
                .iter()
                .copied()
                .filter(|actor| actor.qualifies_for_group(selected_group_mask))
            {
                render_actor(actor, true, state, &mut draws);
            }
        }
    }

    if state.available_group_count == 0 {
        close(state, control);
        return frame(SequelOverviewOutcome::ClosedEmpty, draws);
    }

    render_panel(control, state, &mut draws);
    frame(
        if opened {
            SequelOverviewOutcome::Opened
        } else {
            SequelOverviewOutcome::Rendered
        },
        draws,
    )
}

fn render_actor(
    actor: SequelOverviewActor,
    collect_status: bool,
    state: &mut SequelOverviewState,
    draws: &mut Vec<SequelOverviewDraw>,
) {
    let color = if actor.conflict() {
        if collect_status {
            state.conflict_group_mask |= actor.group_mask;
        }
        CONFLICT_COLOR
    } else if actor.unstable() {
        if collect_status {
            state.unstable_group_mask |= actor.group_mask;
        }
        UNSTABLE_COLOR
    } else {
        NORMAL_COLOR
    };
    draws.push(SequelOverviewDraw::Outline {
        origin: [
            actor.position[0].wrapping_sub(1),
            actor.position[1].wrapping_sub(1),
        ],
        extent: [MARKER_WIDTH, MARKER_HEIGHT],
        color,
    });
    if actor.conflict()
        && let Some(opponent) = actor.opponent_position
    {
        draws.push(SequelOverviewDraw::Line {
            start: [
                actor.position[0].wrapping_add(MARKER_CENTER_X),
                actor.position[1].wrapping_add(MARKER_CENTER_Y),
            ],
            end: [
                opponent[0].wrapping_add(MARKER_CENTER_X),
                opponent[1].wrapping_add(MARKER_CENTER_Y),
            ],
            color: CONFLICT_COLOR,
        });
    }
}

fn render_panel(
    control: &mut SequelOverviewControl,
    state: &mut SequelOverviewState,
    draws: &mut Vec<SequelOverviewDraw>,
) {
    let count = u16::from(state.available_group_count);
    let height = if count < u16::from(PANEL_ROWS_PER_COLUMN) {
        PANEL_ROW_HEIGHT * count + PANEL_BASE_HEIGHT
    } else {
        PANEL_MAX_HEIGHT
    };
    let width = PANEL_BASE_WIDTH
        + PANEL_COLUMN_WIDTH * (count.saturating_sub(1) / u16::from(PANEL_ROWS_PER_COLUMN));
    let panel_y = PANEL_BOTTOM_Y.wrapping_sub(height);
    draws.extend([
        SequelOverviewDraw::Fill {
            origin: [PANEL_X, panel_y],
            extent: [width, height],
            color: PANEL_FILL_COLOR,
        },
        SequelOverviewDraw::Horizontal {
            origin: [PANEL_X, panel_y],
            extent: width,
            color: PANEL_LIGHT_EDGE_COLOR,
        },
        SequelOverviewDraw::Vertical {
            origin: [PANEL_X, panel_y],
            extent: height,
            color: PANEL_LIGHT_EDGE_COLOR,
        },
        SequelOverviewDraw::Horizontal {
            origin: [PANEL_X, panel_y.wrapping_add(height)],
            extent: width,
            color: PANEL_DARK_EDGE_COLOR,
        },
        SequelOverviewDraw::Vertical {
            origin: [PANEL_X.wrapping_add(width), panel_y],
            extent: height,
            color: PANEL_DARK_EDGE_COLOR,
        },
    ]);

    let label_y = panel_y.wrapping_add(2);
    let all_hovered = inclusive_hit(
        control.pointer,
        [ALL_LABEL_X, label_y],
        [ALL_LABEL_WIDTH, PANEL_ROW_HEIGHT],
    );
    if all_hovered && control.primary_pressed {
        control.primary_pressed = false;
        state.show_all = !state.show_all;
    }
    draws.push(SequelOverviewDraw::Text {
        label: SequelOverviewLabel::All,
        origin: [ALL_LABEL_X, label_y],
        color: if all_hovered {
            NORMAL_COLOR
        } else {
            INACTIVE_COLOR
        },
    });

    let mut x = GROUP_LABEL_X;
    let mut y = label_y;
    let mut rows_remaining = PANEL_ROWS_PER_COLUMN;
    state.hovered_group_mask = 0;
    for index in (0..16_u8).rev() {
        let group = 1_u16 << index;
        if state.available_group_mask & group == 0 {
            continue;
        }
        let mut color = NORMAL_COLOR;
        let mut highlighted = state.show_all;
        if !state.show_all {
            if exclusive_lower_hit(control.pointer, [x, y], [GROUP_HIT_WIDTH, PANEL_ROW_HEIGHT]) {
                highlighted = true;
                state.hovered_group_mask = group;
                if control.primary_pressed {
                    control.primary_pressed = false;
                    state.selected_group_mask = group;
                }
            } else if state.selected_group_mask & group != 0 {
                highlighted = true;
            } else {
                color = INACTIVE_COLOR;
            }
        }
        if highlighted {
            if state.unstable_group_mask & group != 0 {
                color = UNSTABLE_COLOR;
            }
            if state.conflict_group_mask & group != 0 {
                color = CONFLICT_COLOR;
            }
        }
        draws.push(SequelOverviewDraw::Text {
            label: SequelOverviewLabel::Group(index),
            origin: [x, y],
            color,
        });

        y = y.wrapping_add(PANEL_ROW_HEIGHT);
        rows_remaining -= 1;
        if rows_remaining == 0 {
            rows_remaining = PANEL_ROWS_PER_COLUMN;
            x = x.wrapping_add(PANEL_COLUMN_WIDTH);
            y = label_y;
        }
    }
}

const fn inclusive_hit(pointer: [u16; 2], origin: [u16; 2], extent: [u16; 2]) -> bool {
    pointer[0] >= origin[0]
        && pointer[0] <= origin[0].wrapping_add(extent[0])
        && pointer[1] >= origin[1]
        && pointer[1] <= origin[1].wrapping_add(extent[1])
}

const fn exclusive_lower_hit(pointer: [u16; 2], origin: [u16; 2], extent: [u16; 2]) -> bool {
    pointer[0] > origin[0]
        && pointer[0] <= origin[0].wrapping_add(extent[0])
        && pointer[1] > origin[1]
        && pointer[1] <= origin[1].wrapping_add(extent[1])
}

fn close(state: &mut SequelOverviewState, control: &mut SequelOverviewControl) {
    state.show_all = false;
    state.active = false;
    control.camera_actor_flags = NavActorSlotFlags::from_executable(CAMERA_REOPEN_FLAGS);
}

fn frame(outcome: SequelOverviewOutcome, draws: Vec<SequelOverviewDraw>) -> SequelOverviewFrame {
    SequelOverviewFrame { outcome, draws }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde::Deserialize;
    use serde_json::{Value, json};

    use super::*;

    #[derive(Deserialize)]
    struct OracleCase {
        name: String,
        input: OracleInput,
        actors: Vec<OracleActor>,
        output: OracleOutput,
    }

    #[derive(Default, Deserialize)]
    #[serde(default)]
    struct OracleInput {
        blocked_word: u16,
        ui_flags: u8,
        pointer: [u16; 2],
        primary: u8,
        secondary: u8,
        press_pending: u8,
        hand: u16,
        camera_slot: u8,
        active: u8,
        show_all: u8,
        available_mask: u16,
        available_count: u8,
        hovered_mask: u16,
        selected_mask: u16,
        conflict_mask: u16,
        unstable_mask: u16,
    }

    #[derive(Deserialize)]
    struct OracleActor {
        group: u16,
        flags: u16,
        #[serde(default = "in_play_state")]
        state_flags: u16,
        holder_flags: u16,
        quantity: u16,
        balance: u16,
        position: [u16; 2],
        opponent: Option<usize>,
        #[serde(default)]
        excluded_holder: bool,
    }

    const fn in_play_state() -> u16 {
        2
    }

    #[derive(Deserialize)]
    struct OracleOutput {
        secondary: u8,
        press_pending: u8,
        primary: u8,
        hand: u16,
        active: u8,
        show_all: u8,
        available_mask: u16,
        available_count: u8,
        hovered_mask: u16,
        selected_mask: u16,
        conflict_mask: u16,
        unstable_mask: u16,
        camera_slot: u8,
        draws: Vec<Value>,
        helpers: Vec<String>,
    }

    #[test]
    fn controller_matches_every_original_executable_case() {
        let cases = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../re/tools/oracle_vectors/big_bug_bang_overview.jsonl"
        ));
        for line in cases.lines() {
            let case: OracleCase = serde_json::from_str(line).unwrap();
            let actors = case
                .actors
                .iter()
                .map(|actor| SequelOverviewActor {
                    flags: actor.flags,
                    in_play: actor.state_flags & 2 != 0,
                    group_mask: actor.group,
                    quantity: actor.quantity,
                    balance: actor.balance,
                    holder_active: actor.holder_flags & 1 != 0,
                    holder_in_play: actor.holder_flags & 2 != 0,
                    holder_excluded: actor.excluded_holder,
                    position: actor.position,
                    opponent_position: actor.opponent.map(|index| case.actors[index].position),
                })
                .collect::<Vec<_>>();
            let mut control = SequelOverviewControl {
                blocked_word: case.input.blocked_word,
                ui_flags: case.input.ui_flags,
                pointer: case.input.pointer,
                primary_pressed: case.input.primary != 0,
                secondary_pressed: case.input.secondary != 0,
                press_pending: case.input.press_pending,
                hand_selector: if case.input.hand == 0 {
                    23
                } else {
                    case.input.hand
                },
                camera_actor_flags: NavActorSlotFlags::from_executable(
                    if case.input.camera_slot == 0 {
                        15
                    } else {
                        case.input.camera_slot
                    },
                ),
            };
            let mut state = SequelOverviewState {
                active: case.input.active != 0,
                show_all: case.input.show_all != 0,
                available_group_mask: if case.input.available_mask == 0 {
                    0xA55A
                } else {
                    case.input.available_mask
                },
                available_group_count: if case.input.available_count == 0 {
                    0xA5
                } else {
                    case.input.available_count
                },
                hovered_group_mask: case.input.hovered_mask,
                selected_group_mask: case.input.selected_mask,
                conflict_group_mask: if case.input.conflict_mask == 0 {
                    0x5AA5
                } else {
                    case.input.conflict_mask
                },
                unstable_group_mask: if case.input.unstable_mask == 0 {
                    0x55AA
                } else {
                    case.input.unstable_mask
                },
            };

            let frame = update_sequel_overview(&actors, &mut control, &mut state);
            assert_eq!(
                u8::from(control.secondary_pressed),
                case.output.secondary,
                "{}",
                case.name
            );
            assert_eq!(
                control.press_pending, case.output.press_pending,
                "{}",
                case.name
            );
            assert_eq!(
                u8::from(control.primary_pressed),
                case.output.primary,
                "{}",
                case.name
            );
            assert_eq!(control.hand_selector, case.output.hand, "{}", case.name);
            assert_eq!(u8::from(state.active), case.output.active, "{}", case.name);
            assert_eq!(
                u8::from(state.show_all),
                case.output.show_all,
                "{}",
                case.name
            );
            assert_eq!(
                state.available_group_mask, case.output.available_mask,
                "{}",
                case.name
            );
            assert_eq!(
                state.available_group_count, case.output.available_count,
                "{}",
                case.name
            );
            assert_eq!(
                state.hovered_group_mask, case.output.hovered_mask,
                "{}",
                case.name
            );
            assert_eq!(
                state.selected_group_mask, case.output.selected_mask,
                "{}",
                case.name
            );
            assert_eq!(
                state.conflict_group_mask, case.output.conflict_mask,
                "{}",
                case.name
            );
            assert_eq!(
                state.unstable_group_mask, case.output.unstable_mask,
                "{}",
                case.name
            );
            assert_eq!(
                control.camera_actor_flags.executable_flags(),
                case.output.camera_slot,
                "{}",
                case.name
            );
            assert_eq!(
                frame
                    .draws
                    .iter()
                    .copied()
                    .map(draw_json)
                    .collect::<Vec<_>>(),
                case.output.draws,
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn oracle_executes_real_roster_and_position_helpers() {
        let cases = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../re/tools/oracle_vectors/big_bug_bang_overview.jsonl"
        ));
        let helpers = cases
            .lines()
            .flat_map(|line| {
                serde_json::from_str::<OracleCase>(line)
                    .unwrap()
                    .output
                    .helpers
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            helpers,
            BTreeSet::from([
                "0x67b8".to_owned(),
                "0x6ff2".to_owned(),
                "0x706e".to_owned(),
            ])
        );
    }

    fn draw_json(draw: SequelOverviewDraw) -> Value {
        match draw {
            SequelOverviewDraw::Fill {
                origin,
                extent,
                color,
            } => {
                json!({"kind":"fill", "origin":origin, "extent":extent, "color":color})
            }
            SequelOverviewDraw::Horizontal {
                origin,
                extent,
                color,
            } => {
                json!({"kind":"horizontal", "origin":origin, "extent":extent, "color":color})
            }
            SequelOverviewDraw::Vertical {
                origin,
                extent,
                color,
            } => {
                json!({"kind":"vertical", "origin":origin, "extent":extent, "color":color})
            }
            SequelOverviewDraw::Outline {
                origin,
                extent,
                color,
            } => {
                json!({"kind":"outline", "origin":origin, "extent":extent, "color":color})
            }
            SequelOverviewDraw::Line { start, end, color } => {
                json!({"kind":"line", "start":start, "end":end, "color":color})
            }
            SequelOverviewDraw::Text {
                label,
                origin,
                color,
            } => {
                json!({"kind":"text", "text":String::from_utf8_lossy(label.text()),
                       "origin":origin, "color":color})
            }
        }
    }
}
