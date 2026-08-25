//! Shared typed state for recovered alien behavior callbacks.

use commander_blood_formats::alien::AXIS_COUNT;

use super::projection::AlienSceneNode;
use super::wave::AlienWaveSelection;

const TRANSITION_QUEUE_LENGTH: usize = 8;

/// Typed replacement for the scene-wide callback control word.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlienControlLatch {
    /// No callback has published control for the current scene pass.
    #[default]
    Inactive,
    /// A callback published the original literal control signal.
    Signal,
    /// A callback published the identity of its owning model context.
    Model(usize),
}

/// Scene state shared by the recovered alien callback families.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienCallbackSceneState {
    /// Current typed callback-control publication.
    pub control_latch: AlienControlLatch,
    /// Frames requested before the next scene callback dispatch.
    pub callback_countdown: u16,
    /// Current camera-relative wave-selection lifecycle.
    pub wave_selection: AlienWaveSelection,
    /// Persistent wrapping palette pulse values.
    pub palette_pulses: [i32; AXIS_COUNT],
    /// Shared signed method delta adjusted by wave completion.
    pub method_delta: i16,
    /// Whether the slot-2 callback family currently owns the camera handoff.
    pub slot2_active: bool,
    /// Model currently owning CROOLIS slot-2 selection tracking.
    pub slot2_selected_model: Option<usize>,
    /// SCRUT selection signal retained after model initialization has consumed the shared seed.
    pub scrut_selection_signal: i16,
    /// Scene node selected by the latest successful wave bounds check.
    pub wave_selected_node: Option<AlienSceneNode>,
    /// Fixed transition queue storing typed scene-node identities.
    pub transition_queue: [Option<AlienSceneNode>; TRANSITION_QUEUE_LENGTH],
    /// Queue slot selected by the surrounding slot-11 behavior.
    pub transition_queue_slot: usize,
    /// Most recently published scene-node identity.
    pub current_node: Option<AlienSceneNode>,
}
