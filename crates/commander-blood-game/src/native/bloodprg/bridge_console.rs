//! Typed bridge-console selection, record choosers, and options handling.

use super::{
    ChoiceListBackend, ChoiceListConfig, ChoiceListFrame, ChoiceListRect, ChoiceListState,
    update_choice_list,
};

const CONSOLE_FRAME_MIN: i16 = 40;
const CONSOLE_FRAME_MAX: i16 = 60;
const CONSOLE_FRAME_CENTER: i16 = 45;
const CONSOLE_RIGHT_BASE: u16 = 287;
const CONSOLE_WIDTH: u16 = 110;
const CONSOLE_Y_BASE: u16 = 72;
const CONSOLE_ROW_HEIGHT: u16 = 18;
const CONSOLE_TARGET_Y_BASE: u16 = 80;
const CONSOLE_PALETTE_FIRST_INDEX: u8 = 123;
const CONSOLE_PALETTE_BASE: Rgb6 = Rgb6::new(16, 12, 0);
const CONSOLE_PALETTE_HOVER: Rgb6 = Rgb6::new(63, 0, 0);
const CONSOLE_HOLD_TICKS: u16 = 90;
const PANEL_CENTER_X: i16 = 100;

/// One six-bit-per-channel color from the original VGA palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb6 {
    /// Red channel in the range used by the original DAC.
    pub red: u8,
    /// Green channel in the range used by the original DAC.
    pub green: u8,
    /// Blue channel in the range used by the original DAC.
    pub blue: u8,
}

impl Rgb6 {
    /// Build one original-range palette color.
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

/// The five commands displayed on the animated bridge console.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeConsoleChoice {
    /// Sound the Ark's horn.
    Horn,
    /// Choose an available navigation target.
    Navigation,
    /// Choose a known contact.
    Contacts,
    /// Open the radio record.
    Radio,
    /// Open text, music, save, load, and quit options.
    Options,
}

impl BridgeConsoleChoice {
    /// Convert a zero-based rendered row into a semantic command.
    pub const fn from_row(row: usize) -> Option<Self> {
        match row {
            0 => Some(Self::Horn),
            1 => Some(Self::Navigation),
            2 => Some(Self::Contacts),
            3 => Some(Self::Radio),
            4 => Some(Self::Options),
            _ => None,
        }
    }

    /// Return the zero-based console row.
    pub const fn row(self) -> usize {
        match self {
            Self::Horn => 0,
            Self::Navigation => 1,
            Self::Contacts => 2,
            Self::Radio => 3,
            Self::Options => 4,
        }
    }
}

/// State of a console submenu after the top-level command is selected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BridgeChoicePanelPhase {
    /// No submenu owns the console.
    #[default]
    Closed,
    /// The submenu must collect labels and calculate its opening rectangle.
    NeedsLayout,
    /// The submenu is moving into its interactive rectangle.
    Transitioning,
    /// The submenu accepts pointer input.
    Interactive,
}

/// Bridge actor state associated with console selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BridgeConsoleActorState {
    /// No console-selection actor state is active.
    #[default]
    Idle,
    /// A console command has been pressed and is being held.
    SelectionHeld,
}

/// Mutable state shared by the dispatcher and its five command handlers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BridgeConsoleState {
    /// Currently selected top-level command.
    pub selected: Option<BridgeConsoleChoice>,
    /// Whether another interface transition blocks command dispatch.
    pub interface_busy: bool,
    /// Whether the selected submenu owns the interface.
    pub interface_active: bool,
    /// Semantic bridge actor state.
    pub actor_state: BridgeConsoleActorState,
    /// Hold duration requested when a top-level row is pressed.
    pub hold_ticks: u16,
    /// Current submenu lifecycle.
    pub panel_phase: BridgeChoicePanelPhase,
    /// Vertical target of the selected console row.
    pub panel_target_y: u16,
}

/// Read-only dispatcher gates and pointer state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BridgeConsoleContext {
    /// A pending aboard-transfer presentation blocks the console.
    pub aboard_transfer_pending: bool,
    /// Save-panel motion is active.
    pub save_motion_active: bool,
    /// Load-panel motion is active.
    pub load_motion_active: bool,
    /// The text/options panel currently owns presentation.
    pub option_panel_active: bool,
    /// A sound-driven bridge action currently owns the console.
    pub sound_action_active: bool,
    /// Dialogue presentation currently owns the bridge.
    pub presentation_active: bool,
    /// Current animated bridge-view frame.
    pub bridge_view_frame: i16,
    /// Pointer position in logical bridge coordinates.
    pub pointer: [i16; 2],
    /// Whether the primary button is down.
    pub primary_pressed: bool,
}

/// Five palette rows requested by one top-level console hover update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeConsolePalettePlan {
    /// First original palette index occupied by the console rows.
    pub first_index: u8,
    /// Color for each semantic console row.
    pub rows: [Rgb6; 5],
}

impl BridgeConsolePalettePlan {
    fn with_hover(hovered: Option<BridgeConsoleChoice>) -> Self {
        let mut rows = [CONSOLE_PALETTE_BASE; 5];
        if let Some(choice) = hovered {
            rows[choice.row()] = CONSOLE_PALETTE_HOVER;
        }
        Self {
            first_index: CONSOLE_PALETTE_FIRST_INDEX,
            rows,
        }
    }
}

/// Gate that prevented top-level console work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeConsoleGate {
    /// Aboard-transfer presentation is pending.
    AboardTransfer,
    /// Save, load, options, or sound activity owns the console.
    InterfaceOwner,
    /// Dialogue presentation owns the bridge.
    Presentation,
    /// The bridge animation is outside the selectable frame range.
    FrameOutside,
    /// A selected submenu is still busy.
    InterfaceBusy,
}

/// Result of one top-level bridge-console dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeConsoleDispatchOutcome {
    /// A gate rejected the update.
    Gated(BridgeConsoleGate),
    /// Palette rows were reset, but the pointer is outside the menu.
    PointerOutside(BridgeConsolePalettePlan),
    /// One row is hovered without activation.
    Hovered {
        /// Semantic row under the pointer.
        choice: BridgeConsoleChoice,
        /// Palette update for the renderer.
        palette: BridgeConsolePalettePlan,
    },
    /// A top-level command was selected.
    Activated {
        /// Selected command.
        choice: BridgeConsoleChoice,
        /// Palette update for the renderer.
        palette: BridgeConsolePalettePlan,
        /// Original selection clip should be played.
        play_selection_clip: bool,
    },
    /// The selected command handler should run this frame.
    HandlerRequested(BridgeConsoleChoice),
}

/// Select or dispatch one command on the animated bridge console.
///
/// This translates `nav_choice_dispatch` at BLOODPRG routine offset
/// `0x0085E2`. Semantic gates, commands, palette colors, and panel state replace
/// packed UI bytes, VGA port writes, and numeric handler dispatch.
pub fn update_bridge_console_dispatch(
    context: BridgeConsoleContext,
    state: &mut BridgeConsoleState,
) -> BridgeConsoleDispatchOutcome {
    if context.aboard_transfer_pending {
        return BridgeConsoleDispatchOutcome::Gated(BridgeConsoleGate::AboardTransfer);
    }
    if context.save_motion_active
        || context.load_motion_active
        || context.option_panel_active
        || context.sound_action_active
    {
        return BridgeConsoleDispatchOutcome::Gated(BridgeConsoleGate::InterfaceOwner);
    }
    if context.presentation_active {
        return BridgeConsoleDispatchOutcome::Gated(BridgeConsoleGate::Presentation);
    }

    if state.selected.is_none() {
        if !(CONSOLE_FRAME_MIN..=CONSOLE_FRAME_MAX).contains(&context.bridge_view_frame) {
            return BridgeConsoleDispatchOutcome::Gated(BridgeConsoleGate::FrameOutside);
        }
        let Some(choice) = console_choice_at(context.bridge_view_frame, context.pointer) else {
            return BridgeConsoleDispatchOutcome::PointerOutside(
                BridgeConsolePalettePlan::with_hover(None),
            );
        };
        let palette = BridgeConsolePalettePlan::with_hover(Some(choice));
        if !context.primary_pressed {
            return BridgeConsoleDispatchOutcome::Hovered { choice, palette };
        }

        state.selected = Some(choice);
        state.interface_active = true;
        state.interface_busy = true;
        state.actor_state = BridgeConsoleActorState::SelectionHeld;
        state.hold_ticks = CONSOLE_HOLD_TICKS;
        state.panel_phase = BridgeChoicePanelPhase::NeedsLayout;
        state.panel_target_y = CONSOLE_TARGET_Y_BASE
            .wrapping_add(u16::try_from(choice.row()).unwrap_or(u16::MAX) * CONSOLE_ROW_HEIGHT);
        return BridgeConsoleDispatchOutcome::Activated {
            choice,
            palette,
            play_selection_clip: true,
        };
    }

    if state.interface_busy {
        BridgeConsoleDispatchOutcome::Gated(BridgeConsoleGate::InterfaceBusy)
    } else {
        BridgeConsoleDispatchOutcome::HandlerRequested(state.selected.unwrap())
    }
}

fn console_choice_at(frame: i16, pointer: [i16; 2]) -> Option<BridgeConsoleChoice> {
    let relative_frame = frame.wrapping_sub(CONSOLE_FRAME_CENTER);
    let right = CONSOLE_RIGHT_BASE.wrapping_sub((relative_frame as u16) << 3) as i16;
    let left = (right as u16).wrapping_sub(CONSOLE_WIDTH) as i16;
    if pointer[0] < left || pointer[0] > right || left < 0 {
        return None;
    }

    let distance = relative_frame.unsigned_abs();
    let quarter_distance = distance >> 2;
    let y_origin = CONSOLE_Y_BASE
        .wrapping_add(distance)
        .wrapping_add(quarter_distance);
    let row_height = CONSOLE_ROW_HEIGHT.wrapping_sub(quarter_distance >> 1);
    let y_offset = (pointer[1] as u16).wrapping_sub(y_origin);
    if (y_offset as i16) < 0 {
        return None;
    }
    BridgeConsoleChoice::from_row(usize::from(y_offset / row_height))
}

/// Semantic kind of deferred bridge record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeDeferredActionKind {
    /// Execute the actionable C3 presentation-queue record selected by the bridge.
    PresentationQueue,
}

/// One typed deferred record selected by a bridge command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeDeferredRecord<RecordId> {
    /// Stable decoded record identity.
    pub record: RecordId,
    /// Semantic action replacing the native record tag.
    pub action: BridgeDeferredActionKind,
}

/// Deferred-action state shared by horn, navigation, contacts, and radio.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BridgeDeferredState<RecordId> {
    /// Most recently selected deferred record.
    pub record: Option<BridgeDeferredRecord<RecordId>>,
    /// Whether selecting the record requires a bridge redraw.
    pub redraw_requested: bool,
}

/// Result of an immediate horn or radio command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImmediateBridgeChoiceOutcome {
    /// The command was not in its activation phase.
    Inactive,
    /// The command published its deferred record.
    Activated,
}

/// Publish the horn presentation record.
///
/// This translates `nav_choice_handler_0` at BLOODPRG routine offset
/// `0x008713` using a typed record identity and semantic action.
pub fn activate_horn_choice<RecordId>(
    record: RecordId,
    console: &mut BridgeConsoleState,
    deferred: &mut BridgeDeferredState<RecordId>,
) -> ImmediateBridgeChoiceOutcome {
    activate_immediate_record(record, console, deferred)
}

/// Host operations shared by bridge submenus.
pub trait BridgeChoiceBackend: ChoiceListBackend {
    /// Advance the panel transition and report whether it is complete.
    fn advance_panel_transition(&mut self, source: ChoiceListRect, target: ChoiceListRect) -> bool;

    /// Reload the radio sound bank after a navigation or radio command.
    fn reload_radio_sound_bank(&mut self);

    /// Start the options-menu music stream.
    fn start_music_stream(&mut self);
}

/// Publish the radio presentation record and reload its sound bank.
///
/// This translates `nav_choice_handler_3` at BLOODPRG routine offset
/// `0x008848` without a numeric record tag or DOS sound path pointer.
pub fn activate_radio_choice<RecordId, Backend: BridgeChoiceBackend>(
    record: RecordId,
    console: &mut BridgeConsoleState,
    deferred: &mut BridgeDeferredState<RecordId>,
    backend: &mut Backend,
) -> ImmediateBridgeChoiceOutcome {
    let outcome = activate_immediate_record(record, console, deferred);
    if outcome == ImmediateBridgeChoiceOutcome::Activated {
        backend.reload_radio_sound_bank();
    }
    outcome
}

fn activate_immediate_record<RecordId>(
    record: RecordId,
    console: &mut BridgeConsoleState,
    deferred: &mut BridgeDeferredState<RecordId>,
) -> ImmediateBridgeChoiceOutcome {
    if console.panel_phase != BridgeChoicePanelPhase::NeedsLayout {
        return ImmediateBridgeChoiceOutcome::Inactive;
    }
    deferred.record = Some(BridgeDeferredRecord {
        record,
        action: BridgeDeferredActionKind::PresentationQueue,
    });
    close_console(console);
    ImmediateBridgeChoiceOutcome::Activated
}

/// One decoded record and its already-resolved display label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeRecordChoice<RecordId> {
    /// Stable decoded record identity.
    pub record: RecordId,
    /// Owned original game-font label.
    pub label: Box<[u8]>,
}

impl<RecordId> BridgeRecordChoice<RecordId> {
    /// Build a typed record choice from a decoded identity and label.
    pub fn new(record: RecordId, label: impl Into<Box<[u8]>>) -> Self {
        Self {
            record,
            label: label.into(),
        }
    }
}

/// Persistent list state for navigation-target and contact submenus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeRecordChoiceState<RecordId> {
    /// Snapshot of choices collected when the panel opens.
    pub choices: Vec<BridgeRecordChoice<RecordId>>,
    /// Pointer state for the shared list widget.
    pub list: ChoiceListState,
    /// Rectangle calculated by the opening layout pass.
    pub current_rect: ChoiceListRect,
}

impl<RecordId> Default for BridgeRecordChoiceState<RecordId> {
    fn default() -> Self {
        Self {
            choices: Vec::new(),
            list: ChoiceListState::default(),
            current_rect: ChoiceListRect::default(),
        }
    }
}

/// Geometry and authored cancel text shared by bridge record lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeRecordChoiceContext<'a> {
    /// Target rectangle for the opening transition.
    pub animation_target: ChoiceListRect,
    /// Decoded cancel label.
    pub cancel_label: &'a [u8],
}

/// Result of one record-choice handler update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeRecordChoiceOutcome {
    /// The selected command does not own this handler.
    Inactive,
    /// The panel is still moving into place.
    Transitioning,
    /// The panel remains interactive with no selection.
    Interactive(ChoiceListFrame),
    /// A typed record was selected and the panel closed.
    Selected,
    /// The cancel row closed the panel.
    Cancelled,
}

/// Update the decoded navigation-target chooser.
///
/// This translates `nav_choice_handler_1` at BLOODPRG routine offset
/// `0x00872C`. Decoded record choices replace a sentinel-terminated array of
/// record-name offsets; selection publishes the record ID directly.
pub fn update_navigation_target_choice<RecordId: Clone, Backend: BridgeChoiceBackend>(
    available: &[BridgeRecordChoice<RecordId>],
    context: BridgeRecordChoiceContext<'_>,
    console: &mut BridgeConsoleState,
    panel: &mut BridgeRecordChoiceState<RecordId>,
    deferred: &mut BridgeDeferredState<RecordId>,
    backend: &mut Backend,
) -> BridgeRecordChoiceOutcome {
    let outcome = update_record_choice_panel(
        BridgeConsoleChoice::Navigation,
        available,
        context,
        console,
        panel,
        deferred,
        backend,
    );
    if outcome == BridgeRecordChoiceOutcome::Selected {
        backend.reload_radio_sound_bank();
    }
    outcome
}

/// Update the decoded known-contact chooser.
///
/// This translates `nav_choice_handler_2` at BLOODPRG routine offset
/// `0x0087BD`. Optional typed contact slots replace zero and all-ones sentinels;
/// selected record IDs are published without adding or subtracting offsets.
pub fn update_contact_choice<RecordId: Clone, Backend: BridgeChoiceBackend>(
    contact_slots: &[Option<BridgeRecordChoice<RecordId>>],
    context: BridgeRecordChoiceContext<'_>,
    console: &mut BridgeConsoleState,
    panel: &mut BridgeRecordChoiceState<RecordId>,
    deferred: &mut BridgeDeferredState<RecordId>,
    backend: &mut Backend,
) -> BridgeRecordChoiceOutcome {
    let available = contact_slots.iter().flatten().cloned().collect::<Vec<_>>();
    let outcome = update_record_choice_panel(
        BridgeConsoleChoice::Contacts,
        &available,
        context,
        console,
        panel,
        deferred,
        backend,
    );
    if outcome == BridgeRecordChoiceOutcome::Selected {
        deferred.redraw_requested = true;
    }
    outcome
}

fn update_record_choice_panel<RecordId: Clone, Backend: BridgeChoiceBackend>(
    owner: BridgeConsoleChoice,
    available: &[BridgeRecordChoice<RecordId>],
    context: BridgeRecordChoiceContext<'_>,
    console: &mut BridgeConsoleState,
    panel: &mut BridgeRecordChoiceState<RecordId>,
    deferred: &mut BridgeDeferredState<RecordId>,
    backend: &mut Backend,
) -> BridgeRecordChoiceOutcome {
    if console.selected != Some(owner) {
        return BridgeRecordChoiceOutcome::Inactive;
    }
    if console.panel_phase == BridgeChoicePanelPhase::NeedsLayout {
        panel.choices = available.to_vec();
        let labels = panel
            .choices
            .iter()
            .map(|choice| choice.label.as_ref())
            .collect::<Vec<_>>();
        panel.current_rect = update_choice_list(
            &labels,
            list_config(context.cancel_label, true),
            &mut panel.list,
            backend,
        )
        .rect;
        console.panel_phase = BridgeChoicePanelPhase::Transitioning;
    }
    if console.panel_phase == BridgeChoicePanelPhase::Transitioning {
        if !backend.advance_panel_transition(panel.current_rect, context.animation_target) {
            return BridgeRecordChoiceOutcome::Transitioning;
        }
        console.panel_phase = BridgeChoicePanelPhase::Interactive;
    }

    let labels = panel
        .choices
        .iter()
        .map(|choice| choice.label.as_ref())
        .collect::<Vec<_>>();
    let frame = update_choice_list(
        &labels,
        list_config(context.cancel_label, false),
        &mut panel.list,
        backend,
    );
    if frame.cancelled {
        close_console(console);
        return BridgeRecordChoiceOutcome::Cancelled;
    }
    let Some(index) = frame.selected_item else {
        return BridgeRecordChoiceOutcome::Interactive(frame);
    };
    let Some(choice) = panel.choices.get(index) else {
        return BridgeRecordChoiceOutcome::Interactive(frame);
    };
    deferred.record = Some(BridgeDeferredRecord {
        record: choice.record.clone(),
        action: BridgeDeferredActionKind::PresentationQueue,
    });
    close_console(console);
    BridgeRecordChoiceOutcome::Selected
}

fn list_config(cancel_label: &[u8], layout_only: bool) -> ChoiceListConfig<'_> {
    ChoiceListConfig {
        center_x: PANEL_CENTER_X,
        preserve_individual_widths: true,
        cancel_label: Some(cancel_label),
        layout_only,
    }
}

/// Authored options-menu command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionMenuChoice {
    /// Open text and subtitle options.
    Text,
    /// Toggle streamed music.
    Music,
    /// Open save-game motion.
    Save,
    /// Open load-game motion.
    Load,
    /// Request game exit.
    Quit,
}

impl OptionMenuChoice {
    /// Resolve one authored row index.
    pub const fn from_row(row: usize) -> Option<Self> {
        match row {
            0 => Some(Self::Text),
            1 => Some(Self::Music),
            2 => Some(Self::Save),
            3 => Some(Self::Load),
            4 => Some(Self::Quit),
            _ => None,
        }
    }
}

/// Label state for the music toggle row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MusicOptionLabel {
    /// Music is off, so the row offers enabling it.
    #[default]
    MusicOn,
    /// Music is on, so the row offers disabling it.
    MusicOff,
}

/// Persistent state owned by the bridge options submenu.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OptionMenuState {
    /// Shared list-widget interaction state.
    pub list: ChoiceListState,
    /// Rectangle calculated by the opening layout pass.
    pub current_rect: ChoiceListRect,
    /// Whether text options were requested.
    pub text_options_active: bool,
    /// Whether streamed music playback is available.
    pub music_supported: bool,
    /// Whether streamed music is currently active.
    pub music_active: bool,
    /// Current semantic music toggle label.
    pub music_label: MusicOptionLabel,
    /// Save-panel movement request.
    pub save_motion_requested: bool,
    /// Load-panel movement request.
    pub load_motion_requested: bool,
    /// Quit request consumed by the host application.
    pub quit_requested: bool,
}

/// Result of one options-handler update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OptionMenuOutcome<Choice = OptionMenuChoice> {
    /// Options do not own the selected console command.
    Inactive,
    /// The options panel is still moving into place.
    Transitioning,
    /// The options panel remains interactive.
    Interactive(ChoiceListFrame),
    /// One authored option was applied and the panel closed.
    Selected(Choice),
    /// The executable-authored final cancel row closed the panel without an action.
    Cancelled,
}

/// Update text, music, save, load, and quit options.
///
/// This translates `nav_choice_handler_4` at BLOODPRG routine offset
/// `0x00886C`. A typed option enum and host audio operation replace low-byte
/// aliases, packed flags, mutable label pointers, and DOS input globals.
pub fn update_option_menu<Backend: BridgeChoiceBackend>(
    labels: &[&[u8]],
    cancel_label: &[u8],
    animation_target: ChoiceListRect,
    console: &mut BridgeConsoleState,
    options: &mut OptionMenuState,
    backend: &mut Backend,
) -> OptionMenuOutcome {
    let frame = match option_menu_frame(
        labels,
        cancel_label,
        animation_target,
        console,
        options,
        backend,
    ) {
        OptionMenuFrame::Inactive => return OptionMenuOutcome::Inactive,
        OptionMenuFrame::Transitioning => return OptionMenuOutcome::Transitioning,
        OptionMenuFrame::Ready(frame) => frame,
    };
    let Some(index) = frame.selected_item else {
        if frame.cancelled {
            close_console(console);
            return OptionMenuOutcome::Cancelled;
        }
        return OptionMenuOutcome::Interactive(frame);
    };
    let Some(choice) = OptionMenuChoice::from_row(index) else {
        return OptionMenuOutcome::Interactive(frame);
    };
    apply_option_choice(choice, options, backend);
    close_console(console);
    OptionMenuOutcome::Selected(choice)
}

/// The sequel's authored command order, with shared actions kept semantic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SequelOptionMenuChoice {
    /// Open the simulation-countdown speed list.
    SimulationSpeed,
    /// Toggle travel animations independently of music support.
    Travel,
    /// Execute an action also present in Commander Blood.
    Common(OptionMenuChoice),
}

impl SequelOptionMenuChoice {
    /// Resolve a sequel row without applying the original low-byte ABI aliases.
    pub const fn from_row(row: usize) -> Option<Self> {
        match row {
            0 => Some(Self::SimulationSpeed),
            1 => Some(Self::Common(OptionMenuChoice::Text)),
            2 => Some(Self::Travel),
            3 => Some(Self::Common(OptionMenuChoice::Music)),
            4 => Some(Self::Common(OptionMenuChoice::Save)),
            5 => Some(Self::Common(OptionMenuChoice::Load)),
            6 => Some(Self::Common(OptionMenuChoice::Quit)),
            _ => None,
        }
    }
}

/// State owned by the sequel options handler at file `0x9A11`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SequelOptionMenuState {
    /// Shared list, music, text, save, load, and quit state.
    pub common: OptionMenuState,
    /// Whether the simulation-speed submenu owns the next choice update.
    pub simulation_options_active: bool,
    /// Simulation-speed list layout/transition phase byte.
    pub simulation_options_phase: u8,
    /// Text-speed list layout/transition phase byte.
    pub text_options_phase: u8,
    /// Current travel-animation flag; the travel label reports this value.
    pub travel_enabled: bool,
    /// Save/load panel activation shared by both motion requests.
    pub save_panel_active: bool,
    /// Shared primary button latch, cleared when requesting quit.
    pub primary_pointer_pressed: bool,
    /// Shared secondary button latch, cleared when requesting quit.
    pub secondary_pointer_pressed: bool,
}

/// Update the sequel's seven-command menu using the shared choice widget.
///
/// The command tail at file `0x9A67..0x9B43` opens either speed list, toggles
/// travel, and performs the five shared actions. Stream setup remains a host
/// operation; no DOS driver entry points are retained in the runtime.
pub fn update_sequel_option_menu<Backend: BridgeChoiceBackend>(
    labels: &[&[u8]],
    cancel_label: &[u8],
    animation_target: ChoiceListRect,
    console: &mut BridgeConsoleState,
    options: &mut SequelOptionMenuState,
    backend: &mut Backend,
) -> OptionMenuOutcome<SequelOptionMenuChoice> {
    let frame = match option_menu_frame(
        labels,
        cancel_label,
        animation_target,
        console,
        &mut options.common,
        backend,
    ) {
        OptionMenuFrame::Inactive => return OptionMenuOutcome::Inactive,
        OptionMenuFrame::Transitioning => return OptionMenuOutcome::Transitioning,
        OptionMenuFrame::Ready(frame) => frame,
    };
    let Some(index) = frame.selected_item else {
        if frame.cancelled {
            close_console(console);
            return OptionMenuOutcome::Cancelled;
        }
        return OptionMenuOutcome::Interactive(frame);
    };
    let Some(choice) = SequelOptionMenuChoice::from_row(index) else {
        return OptionMenuOutcome::Interactive(frame);
    };
    match choice {
        SequelOptionMenuChoice::SimulationSpeed => {
            options.simulation_options_active = true;
            options.simulation_options_phase = 1;
        }
        SequelOptionMenuChoice::Travel => options.travel_enabled = !options.travel_enabled,
        SequelOptionMenuChoice::Common(common) => {
            apply_option_choice(common, &mut options.common, backend);
            match common {
                OptionMenuChoice::Text => options.text_options_phase = 1,
                OptionMenuChoice::Save | OptionMenuChoice::Load => options.save_panel_active = true,
                OptionMenuChoice::Quit => {
                    options.primary_pointer_pressed = false;
                    options.secondary_pointer_pressed = false;
                }
                OptionMenuChoice::Music => {}
            }
        }
    }
    close_console(console);
    OptionMenuOutcome::Selected(choice)
}

enum OptionMenuFrame {
    Inactive,
    Transitioning,
    Ready(ChoiceListFrame),
}

fn option_menu_frame<Backend: BridgeChoiceBackend>(
    labels: &[&[u8]],
    cancel_label: &[u8],
    animation_target: ChoiceListRect,
    console: &mut BridgeConsoleState,
    options: &mut OptionMenuState,
    backend: &mut Backend,
) -> OptionMenuFrame {
    if console.selected != Some(BridgeConsoleChoice::Options) {
        return OptionMenuFrame::Inactive;
    }
    let config = ChoiceListConfig {
        center_x: PANEL_CENTER_X,
        preserve_individual_widths: true,
        cancel_label: Some(cancel_label),
        layout_only: true,
    };
    if console.panel_phase == BridgeChoicePanelPhase::NeedsLayout {
        options.current_rect = update_choice_list(labels, config, &mut options.list, backend).rect;
        console.panel_phase = BridgeChoicePanelPhase::Transitioning;
    }
    if console.panel_phase == BridgeChoicePanelPhase::Transitioning {
        if !backend.advance_panel_transition(options.current_rect, animation_target) {
            return OptionMenuFrame::Transitioning;
        }
        console.panel_phase = BridgeChoicePanelPhase::Interactive;
    }
    let frame = update_choice_list(
        labels,
        ChoiceListConfig {
            layout_only: false,
            ..config
        },
        &mut options.list,
        backend,
    );
    OptionMenuFrame::Ready(frame)
}

fn apply_option_choice<Backend: BridgeChoiceBackend>(
    choice: OptionMenuChoice,
    options: &mut OptionMenuState,
    backend: &mut Backend,
) {
    match choice {
        OptionMenuChoice::Text => options.text_options_active = true,
        OptionMenuChoice::Music if options.music_supported && options.music_active => {
            options.music_active = false;
            options.music_label = MusicOptionLabel::MusicOn;
        }
        OptionMenuChoice::Music if options.music_supported => {
            options.music_active = true;
            options.music_label = MusicOptionLabel::MusicOff;
            backend.start_music_stream();
        }
        OptionMenuChoice::Music => {}
        OptionMenuChoice::Save => options.save_motion_requested = true,
        OptionMenuChoice::Load => options.load_motion_requested = true,
        OptionMenuChoice::Quit => options.quit_requested = true,
    }
}

fn close_console(console: &mut BridgeConsoleState) {
    console.selected = None;
    console.interface_active = false;
    console.interface_busy = false;
    console.panel_phase = BridgeChoicePanelPhase::Closed;
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::{ChoiceListPointer, ChoiceListPresentation};

    const DISPATCH_VECTOR_COUNT: usize = 34;
    const IMMEDIATE_VECTOR_COUNT: usize = 6;
    const RECORD_HANDLER_VECTOR_COUNT: usize = 7;
    const OPTION_VECTOR_COUNT: usize = 17;
    const CANCEL_LABEL: &[u8] = b"CANCEL";
    const ANIMATION_TARGET: ChoiceListRect = ChoiceListRect {
        origin: [60, 60],
        size: [80, 40],
    };

    #[derive(Deserialize)]
    struct DispatchOracle {
        name: String,
        selection_before: u16,
        selection_after: u16,
        frame: i16,
        mouse: [i16; 2],
        primary: u8,
        terminal_path: String,
        hover_row: Option<usize>,
        target_y_after: u16,
        port_writes: Vec<[u16; 3]>,
    }

    #[test]
    fn top_level_dispatch_matches_every_original_semantic_vector() {
        let vectors: Vec<DispatchOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_85e2_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), DISPATCH_VECTOR_COUNT);

        for vector in vectors {
            let mut state = BridgeConsoleState {
                selected: decode_native_selection(vector.selection_before),
                interface_busy: vector.name == "selected_ui_busy",
                ..BridgeConsoleState::default()
            };
            let outcome = update_bridge_console_dispatch(context_for(&vector), &mut state);
            assert!(
                dispatch_path_matches(outcome, &vector.terminal_path),
                "{}: {outcome:?}",
                vector.name
            );
            assert_eq!(
                state.selected.map_or(0, |choice| choice.row() as u16 + 1),
                vector.selection_after,
                "{}",
                vector.name
            );
            if vector.terminal_path == "activate" {
                assert_eq!(
                    state.panel_target_y, vector.target_y_after,
                    "{}",
                    vector.name
                );
            }
            if let Some(row) = vector.hover_row.filter(|row| *row < 5) {
                let palette = outcome_palette(outcome).unwrap();
                assert_eq!(palette.rows[row], CONSOLE_PALETTE_HOVER, "{}", vector.name);
            }
            assert_eq!(
                vector.port_writes.is_empty(),
                outcome_palette(outcome).is_none(),
                "{}",
                vector.name
            );
        }
    }

    fn context_for(vector: &DispatchOracle) -> BridgeConsoleContext {
        BridgeConsoleContext {
            aboard_transfer_pending: vector.name == "c2_bit_zero_blocks",
            save_motion_active: vector.name == "left_motion_whole_byte",
            load_motion_active: vector.name == "right_motion_whole_byte",
            option_panel_active: vector.name == "menu_whole_byte",
            sound_action_active: vector.name == "sound_whole_byte",
            presentation_active: vector.name == "presentation_bit_zero_blocks",
            bridge_view_frame: vector.frame,
            pointer: vector.mouse,
            primary_pressed: vector.primary & 1 != 0,
        }
    }

    fn decode_native_selection(selection: u16) -> Option<BridgeConsoleChoice> {
        selection
            .checked_sub(1)
            .and_then(|row| BridgeConsoleChoice::from_row(usize::from(row)))
    }

    fn dispatch_path_matches(outcome: BridgeConsoleDispatchOutcome, path: &str) -> bool {
        match outcome {
            BridgeConsoleDispatchOutcome::Gated(BridgeConsoleGate::AboardTransfer) => {
                path == "c2_gate"
            }
            BridgeConsoleDispatchOutcome::Gated(BridgeConsoleGate::InterfaceOwner) => {
                path == "whole_byte_gate"
            }
            BridgeConsoleDispatchOutcome::Gated(BridgeConsoleGate::Presentation) => {
                path == "presentation_gate"
            }
            BridgeConsoleDispatchOutcome::Gated(BridgeConsoleGate::FrameOutside) => {
                matches!(path, "frame_low" | "frame_high")
            }
            BridgeConsoleDispatchOutcome::Gated(BridgeConsoleGate::InterfaceBusy) => {
                path == "ui_busy"
            }
            BridgeConsoleDispatchOutcome::PointerOutside(_) => {
                matches!(path, "x_left" | "x_right" | "y_above" | "row_outside")
            }
            BridgeConsoleDispatchOutcome::Hovered { .. } => path == "hover",
            BridgeConsoleDispatchOutcome::Activated { .. } => path == "activate",
            BridgeConsoleDispatchOutcome::HandlerRequested(_) => path == "handler",
        }
    }

    const fn outcome_palette(
        outcome: BridgeConsoleDispatchOutcome,
    ) -> Option<BridgeConsolePalettePlan> {
        match outcome {
            BridgeConsoleDispatchOutcome::PointerOutside(palette)
            | BridgeConsoleDispatchOutcome::Hovered { palette, .. }
            | BridgeConsoleDispatchOutcome::Activated { palette, .. } => Some(palette),
            _ => None,
        }
    }

    #[derive(Deserialize)]
    struct ImmediateOracle {
        name: String,
        phase_before: u8,
        source_record: u16,
        deferred_link_before: u16,
        deferred_link_after: u16,
        loader_called: bool,
    }

    #[test]
    fn horn_and_radio_handlers_match_every_original_semantic_vector() {
        for (path, radio) in [
            (
                "../../../../../re/tools/oracle_vectors/func_8713_natural.json",
                false,
            ),
            (
                "../../../../../re/tools/oracle_vectors/func_8848_natural.json",
                true,
            ),
        ] {
            let vectors: Vec<ImmediateOracle> = serde_json::from_str(match path {
                "../../../../../re/tools/oracle_vectors/func_8713_natural.json" => {
                    include_str!("../../../../../re/tools/oracle_vectors/func_8713_natural.json")
                }
                _ => include_str!("../../../../../re/tools/oracle_vectors/func_8848_natural.json"),
            })
            .unwrap();
            assert_eq!(vectors.len(), IMMEDIATE_VECTOR_COUNT);
            for vector in vectors {
                let mut console = selected_console(
                    if radio {
                        BridgeConsoleChoice::Radio
                    } else {
                        BridgeConsoleChoice::Horn
                    },
                    phase_from_activation_bit(vector.phase_before),
                );
                let mut deferred = BridgeDeferredState {
                    record: Some(BridgeDeferredRecord {
                        record: vector.deferred_link_before,
                        action: BridgeDeferredActionKind::PresentationQueue,
                    }),
                    redraw_requested: false,
                };
                let mut backend = OracleBackend::default();
                let outcome = if radio {
                    activate_radio_choice(
                        vector.source_record,
                        &mut console,
                        &mut deferred,
                        &mut backend,
                    )
                } else {
                    activate_horn_choice(vector.source_record, &mut console, &mut deferred)
                };
                assert_eq!(
                    outcome == ImmediateBridgeChoiceOutcome::Activated,
                    vector.phase_before & 1 != 0,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    deferred.record.as_ref().unwrap().record,
                    vector.deferred_link_after,
                    "{}",
                    vector.name
                );
                if vector.phase_before & 1 != 0 {
                    assert_eq!(
                        deferred.record.as_ref().unwrap().action,
                        BridgeDeferredActionKind::PresentationQueue,
                        "{}",
                        vector.name
                    );
                }
                assert_eq!(backend.reloads > 0, vector.loader_called, "{}", vector.name);
            }
        }
    }

    #[derive(Deserialize)]
    struct RecordOracle {
        name: String,
        phase_before: u8,
        targets_after: Vec<u16>,
        calls: Vec<serde_json::Value>,
        deferred_link_after: u16,
        render_flag_after: u8,
    }

    #[test]
    fn navigation_and_contact_handlers_match_every_original_semantic_vector() {
        for (json, contacts) in [
            (
                include_str!("../../../../../re/tools/oracle_vectors/func_872c_natural.json"),
                false,
            ),
            (
                include_str!("../../../../../re/tools/oracle_vectors/func_87bd_natural.json"),
                true,
            ),
        ] {
            let vectors: Vec<RecordOracle> = serde_json::from_str(json).unwrap();
            assert_eq!(vectors.len(), RECORD_HANDLER_VECTOR_COUNT);
            for vector in vectors {
                let choices = vector
                    .targets_after
                    .iter()
                    .copied()
                    .take_while(|value| *value != u16::MAX)
                    .map(|label_offset| {
                        BridgeRecordChoice::new(label_offset.wrapping_sub(4), b"TARGET".as_slice())
                    })
                    .collect::<Vec<_>>();
                let selection = interactive_selection(&vector.calls);
                let mut backend =
                    OracleBackend::for_selection(selection, transition_complete(&vector.calls));
                let owner = if contacts {
                    BridgeConsoleChoice::Contacts
                } else {
                    BridgeConsoleChoice::Navigation
                };
                let mut console = selected_console(owner, phase_from_native(vector.phase_before));
                let mut panel = BridgeRecordChoiceState::default();
                if console.panel_phase != BridgeChoicePanelPhase::NeedsLayout {
                    panel.choices = choices.clone();
                    panel.current_rect = ChoiceListRect {
                        origin: [40, 80],
                        size: [120, 30],
                    };
                }
                let mut deferred = BridgeDeferredState::<u16>::default();
                let outcome = if contacts {
                    let slots = choices.iter().cloned().map(Some).collect::<Vec<_>>();
                    update_contact_choice(
                        &slots,
                        record_context(),
                        &mut console,
                        &mut panel,
                        &mut deferred,
                        &mut backend,
                    )
                } else {
                    update_navigation_target_choice(
                        &choices,
                        record_context(),
                        &mut console,
                        &mut panel,
                        &mut deferred,
                        &mut backend,
                    )
                };
                let selected = matches!(outcome, BridgeRecordChoiceOutcome::Selected);
                if selected {
                    assert_eq!(
                        deferred.record.as_ref().unwrap().record,
                        vector.deferred_link_after,
                        "{}",
                        vector.name
                    );
                }
                assert_eq!(
                    deferred.redraw_requested,
                    contacts && selected,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    backend.reloads > 0,
                    !contacts && selected,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    vector.render_flag_after == 1,
                    contacts && selected,
                    "{}",
                    vector.name
                );
            }
        }
    }

    fn record_context() -> BridgeRecordChoiceContext<'static> {
        BridgeRecordChoiceContext {
            animation_target: ANIMATION_TARGET,
            cancel_label: CANCEL_LABEL,
        }
    }

    fn interactive_selection(calls: &[serde_json::Value]) -> Option<usize> {
        calls
            .iter()
            .rev()
            .find(|call| call["call"] == "list_widget_layout_unified" && call["editing"] != 1)
            .and_then(|call| {
                let result = call["result"].as_u64()? as u16;
                (result != u16::MAX).then_some(usize::from(result))
            })
    }

    fn transition_complete(calls: &[serde_json::Value]) -> bool {
        calls
            .iter()
            .find(|call| call["call"] == "framebuffer_rect_interpolate_and_remap_step")
            .and_then(|call| call["complete"].as_bool())
            .unwrap_or(true)
    }

    #[derive(Deserialize)]
    struct OptionOracle {
        name: String,
        phase_before: u8,
        selection: Option<u16>,
        voc_active_after: u8,
        motion_after: [u8; 3],
        calls: Vec<serde_json::Value>,
    }

    #[test]
    fn options_handler_matches_authored_rows_and_rejects_abi_aliases() {
        let vectors: Vec<OptionOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_886c_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), OPTION_VECTOR_COUNT);
        for vector in vectors {
            let selection = vector.selection.map(usize::from);
            let mut backend = OracleBackend::for_selection(
                selection.filter(|_| vector.phase_before != 2),
                transition_complete(&vector.calls),
            );
            let mut console = selected_console(
                BridgeConsoleChoice::Options,
                phase_from_native(vector.phase_before),
            );
            let mut options = OptionMenuState {
                music_supported: vector.name != "music_disabled",
                music_active: vector.name == "music_active_stops",
                ..OptionMenuState::default()
            };
            let labels = [b"TEXT".as_slice(), b"MUSIC", b"SAVE", b"LOAD", b"QUIT"];
            let outcome = update_option_menu(
                &labels,
                CANCEL_LABEL,
                ANIMATION_TARGET,
                &mut console,
                &mut options,
                &mut backend,
            );
            if let Some(row) = selection.filter(|_| vector.phase_before != 2) {
                if row == labels.len() {
                    assert_eq!(outcome, OptionMenuOutcome::Cancelled, "{}", vector.name);
                } else if let Some(choice) = OptionMenuChoice::from_row(row) {
                    assert_eq!(
                        outcome,
                        OptionMenuOutcome::Selected(choice),
                        "{}",
                        vector.name
                    );
                } else {
                    assert!(
                        matches!(outcome, OptionMenuOutcome::Interactive(_)),
                        "{}: {outcome:?}",
                        vector.name
                    );
                }
            }
            if vector.name.starts_with("music_") {
                assert_eq!(
                    options.music_active,
                    vector.voc_active_after & 1 != 0,
                    "{}",
                    vector.name
                );
            }
            assert_eq!(
                options.save_motion_requested,
                vector.motion_after[0] == 1,
                "{}",
                vector.name
            );
            assert_eq!(
                options.load_motion_requested,
                vector.motion_after[1] == 1,
                "{}",
                vector.name
            );
        }
    }

    #[derive(Deserialize)]
    struct SequelOptionsOracle {
        executable_sha256: String,
        cases: Vec<SequelOptionsCase>,
    }

    #[derive(Debug, Deserialize)]
    struct SequelOptionsCase {
        selected: i16,
        supported: bool,
        music: bool,
        travel: bool,
        simulation_active: u8,
        simulation_phase: u8,
        text_active: u8,
        text_phase: u8,
        travel_after: bool,
        music_after: bool,
        music_label_off: bool,
        save: bool,
        load: bool,
        panel: bool,
        quit: bool,
        primary: bool,
        secondary: bool,
        menu_open: bool,
        modal: bool,
        stream_starts: usize,
    }

    const SEQUEL_LABELS: [&[u8]; 7] = [
        b"VITESSE",
        b"TEXTES",
        b"VOYAGE_OFF",
        b"MUSIQUE_OFF",
        b"SAUVER",
        b"CHARGER",
        b"QUITTER",
    ];

    #[test]
    fn sequel_options_handler_matches_original_command_effects() {
        let oracle: SequelOptionsOracle = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/big_bug_bang_options.json"
        ))
        .unwrap();
        assert_eq!(
            oracle.executable_sha256,
            "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834"
        );
        assert_eq!(oracle.cases.len(), 80);
        for case in oracle.cases {
            let selection = usize::try_from(case.selected).ok();
            let mut backend = OracleBackend::for_selection(selection, true);
            let mut console = selected_console(
                BridgeConsoleChoice::Options,
                BridgeChoicePanelPhase::Interactive,
            );
            let mut options = SequelOptionMenuState {
                common: OptionMenuState {
                    music_supported: case.supported,
                    music_active: case.music,
                    music_label: if case.music {
                        MusicOptionLabel::MusicOff
                    } else {
                        MusicOptionLabel::MusicOn
                    },
                    ..OptionMenuState::default()
                },
                simulation_options_phase: 0x82,
                text_options_phase: 0x41,
                travel_enabled: case.travel,
                primary_pointer_pressed: true,
                secondary_pointer_pressed: true,
                ..SequelOptionMenuState::default()
            };
            let outcome = update_sequel_option_menu(
                &SEQUEL_LABELS,
                b"ANNULER",
                ANIMATION_TARGET,
                &mut console,
                &mut options,
                &mut backend,
            );
            assert_eq!(
                options.simulation_options_active,
                case.simulation_active == 1,
                "{case:?}"
            );
            assert_eq!(
                options.simulation_options_phase, case.simulation_phase,
                "{case:?}"
            );
            assert_eq!(
                options.common.text_options_active,
                case.text_active == 1,
                "{case:?}"
            );
            assert_eq!(options.text_options_phase, case.text_phase, "{case:?}");
            assert_eq!(options.travel_enabled, case.travel_after, "{case:?}");
            assert_eq!(options.common.music_active, case.music_after, "{case:?}");
            assert_eq!(
                options.common.music_label == MusicOptionLabel::MusicOff,
                case.music_label_off,
                "{case:?}"
            );
            assert_eq!(options.common.save_motion_requested, case.save, "{case:?}");
            assert_eq!(options.common.load_motion_requested, case.load, "{case:?}");
            assert_eq!(options.save_panel_active, case.panel, "{case:?}");
            assert_eq!(options.common.quit_requested, case.quit, "{case:?}");
            assert_eq!(options.primary_pointer_pressed, case.primary, "{case:?}");
            assert_eq!(
                options.secondary_pointer_pressed, case.secondary,
                "{case:?}"
            );
            assert_eq!(console.selected.is_some(), case.menu_open, "{case:?}");
            assert_eq!(console.interface_active, case.modal, "{case:?}");
            assert_eq!(backend.stream_starts, case.stream_starts, "{case:?}");
            match selection {
                None => assert!(
                    matches!(outcome, OptionMenuOutcome::Interactive(_)),
                    "{case:?}"
                ),
                Some(7) => assert_eq!(outcome, OptionMenuOutcome::Cancelled, "{case:?}"),
                Some(row) => assert_eq!(
                    outcome,
                    OptionMenuOutcome::Selected(SequelOptionMenuChoice::from_row(row).unwrap()),
                    "{case:?}"
                ),
            }
        }
    }

    #[test]
    fn sequel_options_wait_for_ownership_and_transition_before_applying_actions() {
        let mut console = BridgeConsoleState::default();
        let mut options = SequelOptionMenuState::default();
        let mut backend = OracleBackend::for_selection(Some(0), false);
        assert_eq!(
            update_sequel_option_menu(
                &SEQUEL_LABELS,
                b"ANNULER",
                ANIMATION_TARGET,
                &mut console,
                &mut options,
                &mut backend
            ),
            OptionMenuOutcome::Inactive
        );
        assert_eq!(options, SequelOptionMenuState::default());
        console = selected_console(
            BridgeConsoleChoice::Options,
            BridgeChoicePanelPhase::NeedsLayout,
        );
        assert_eq!(
            update_sequel_option_menu(
                &SEQUEL_LABELS,
                b"ANNULER",
                ANIMATION_TARGET,
                &mut console,
                &mut options,
                &mut backend
            ),
            OptionMenuOutcome::Transitioning
        );
        assert!(!options.simulation_options_active);
        assert_eq!(options.simulation_options_phase, 0);
        backend.transition_complete = true;
        assert_eq!(
            update_sequel_option_menu(
                &SEQUEL_LABELS,
                b"ANNULER",
                ANIMATION_TARGET,
                &mut console,
                &mut options,
                &mut backend
            ),
            OptionMenuOutcome::Selected(SequelOptionMenuChoice::SimulationSpeed)
        );
        assert!(options.simulation_options_active);
        assert_eq!(options.simulation_options_phase, 1);
        for row in [7, 8, 128, 255, 256, usize::MAX] {
            assert_eq!(SequelOptionMenuChoice::from_row(row), None);
        }
    }

    #[derive(Default)]
    struct OracleBackend {
        pointer: ChoiceListPointer,
        requested_selection: Option<usize>,
        transition_complete: bool,
        reloads: usize,
        stream_starts: usize,
    }

    impl OracleBackend {
        fn for_selection(selection: Option<usize>, transition_complete: bool) -> Self {
            Self {
                requested_selection: selection,
                transition_complete,
                ..Self::default()
            }
        }
    }

    impl ChoiceListBackend for OracleBackend {
        fn measure_label(&mut self, _label: &[u8]) -> u16 {
            30
        }
        fn prepare_background(&mut self, rect: ChoiceListRect) {
            if let Some(row) = self.requested_selection {
                self.pointer = ChoiceListPointer {
                    position: [
                        rect.origin[0].wrapping_add((rect.size[0] >> 1) as i16),
                        rect.origin[1]
                            .wrapping_add(4)
                            .wrapping_add(i16::try_from(row).unwrap_or(i16::MAX).wrapping_mul(11)),
                    ],
                    primary_pressed: true,
                };
            }
        }
        fn pointer(&mut self) -> ChoiceListPointer {
            self.pointer
        }
    }

    impl BridgeChoiceBackend for OracleBackend {
        fn advance_panel_transition(
            &mut self,
            _source: ChoiceListRect,
            _target: ChoiceListRect,
        ) -> bool {
            self.transition_complete
        }
        fn reload_radio_sound_bank(&mut self) {
            self.reloads += 1;
        }
        fn start_music_stream(&mut self) {
            self.stream_starts += 1;
        }
    }

    fn selected_console(
        choice: BridgeConsoleChoice,
        phase: BridgeChoicePanelPhase,
    ) -> BridgeConsoleState {
        BridgeConsoleState {
            selected: Some(choice),
            interface_active: true,
            panel_phase: phase,
            ..BridgeConsoleState::default()
        }
    }

    const fn phase_from_activation_bit(phase: u8) -> BridgeChoicePanelPhase {
        if phase & 1 != 0 {
            BridgeChoicePanelPhase::NeedsLayout
        } else {
            BridgeChoicePanelPhase::Transitioning
        }
    }

    const fn phase_from_native(phase: u8) -> BridgeChoicePanelPhase {
        match phase & 3 {
            0 => BridgeChoicePanelPhase::Interactive,
            1 | 3 => BridgeChoicePanelPhase::NeedsLayout,
            _ => BridgeChoicePanelPhase::Transitioning,
        }
    }

    #[test]
    fn list_state_is_semantic_not_segment_owned() {
        let state = ChoiceListState {
            presentation: ChoiceListPresentation::Hover,
            hovered_row: Some(2),
        };
        assert_eq!(state.hovered_row, Some(2));
    }
}
