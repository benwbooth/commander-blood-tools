//! Resource-backed bridge presentation-line playback.

/// Typed identity of one presentation animation resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PresentationResourceId(u16);

impl PresentationResourceId {
    /// Decode an authored presentation resource identity.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Return the authored resource identity.
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Semantic state carried by one presentation-line record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationLineFlags {
    /// The line is present in its actor slot.
    pub present: bool,
    /// The line has entered its secondary transition state.
    pub transition_latched: bool,
    /// The resource has been loaded by the backend.
    pub resource_loaded: bool,
    /// The line is ready for presentation playback.
    pub ready: bool,
}

/// One typed presentation animation line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationLine {
    /// Semantic line state.
    pub flags: PresentationLineFlags,
    /// Resource selected for this line.
    pub resource: PresentationResourceId,
    /// Last frame reported by the resource header.
    pub terminal_frame: u16,
    /// Frame submitted on the next accepted update.
    pub frame: u16,
    /// Logical draw position.
    pub position: [u16; 2],
}

/// Shared playback state used by presentation lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationLinePlayback {
    /// Another resource operation currently blocks line playback.
    pub busy: bool,
    /// Play the current line backwards.
    pub reverse: bool,
    /// The presentation area needs a redraw.
    pub redraw_requested: bool,
}

/// Resource and renderer operations required by line playback.
pub trait PresentationLineBackend {
    /// Resource-loading failure.
    type Error;

    /// Load a presentation animation and return its terminal frame.
    fn load_resource(&mut self, resource: PresentationResourceId) -> Result<u16, Self::Error>;

    /// Draw one frame of a loaded presentation resource.
    fn draw_resource_frame(
        &mut self,
        resource: PresentationResourceId,
        frame: u16,
        position: [u16; 2],
    );
}

/// Result of one presentation-line playback update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationLineOutcome {
    /// A shared resource operation blocked the line without mutation.
    Busy,
    /// One frame was drawn and playback remains active.
    Advanced,
    /// The terminal frame was drawn and playback completed.
    Completed,
}

/// Load, draw, and advance one bridge presentation line.
///
/// This translates `presentation_line_helper` at BLOODPRG routine offset
/// `0x007E1C`. A typed resource identity and backend cache replace the filename
/// table and shared resource buffer; ordinary fields replace its packed record.
pub fn update_presentation_line<Backend: PresentationLineBackend>(
    line: &mut PresentationLine,
    playback: &mut PresentationLinePlayback,
    backend: &mut Backend,
) -> Result<PresentationLineOutcome, Backend::Error> {
    if playback.busy {
        return Ok(PresentationLineOutcome::Busy);
    }

    if !line.flags.resource_loaded {
        playback.redraw_requested = true;
        line.terminal_frame = backend.load_resource(line.resource)?;
        line.frame = line.terminal_frame.wrapping_sub(1);
        if !playback.reverse {
            line.frame = u16::MIN;
            playback.reverse = false;
        }
        line.flags.resource_loaded = true;
    }

    backend.draw_resource_frame(line.resource, line.frame, line.position);
    let completed = if playback.reverse {
        if line.frame == u16::MIN {
            true
        } else {
            line.frame = line.frame.wrapping_sub(1);
            false
        }
    } else if line.frame == line.terminal_frame {
        true
    } else {
        line.frame = line.frame.wrapping_add(1);
        false
    };

    if completed {
        playback.reverse = false;
        playback.redraw_requested = false;
        Ok(PresentationLineOutcome::Completed)
    } else {
        Ok(PresentationLineOutcome::Advanced)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 12;
    const TEST_POSITION: [u16; 2] = [37, 91];

    #[derive(Deserialize)]
    struct LineOracle {
        name: String,
        busy_gate: bool,
        loaded_before: bool,
        resource_id: u16,
        loaded_terminal_frame: Option<u16>,
        frame_drawn: Option<u16>,
        terminal_frame_after: u16,
        frame_after: u16,
        ui_before: u8,
        ui_after: u8,
        reverse_before: u8,
        reverse_after: u8,
        completed_cf: bool,
        helper_calls: Vec<String>,
    }

    struct OracleBackend {
        terminal_frame: Option<u16>,
        calls: Vec<String>,
        drawn_frame: Option<u16>,
    }

    impl PresentationLineBackend for OracleBackend {
        type Error = std::convert::Infallible;

        fn load_resource(&mut self, _resource: PresentationResourceId) -> Result<u16, Self::Error> {
            self.calls.push(String::from("resource_load"));
            Ok(self.terminal_frame.unwrap())
        }

        fn draw_resource_frame(
            &mut self,
            _resource: PresentationResourceId,
            frame: u16,
            _position: [u16; 2],
        ) {
            self.calls.push(String::from("entity_setter"));
            self.drawn_frame = Some(frame);
        }
    }

    #[test]
    fn playback_matches_every_original_semantic_vector() {
        let vectors: Vec<LineOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_7e1c_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut line = PresentationLine {
                flags: PresentationLineFlags {
                    resource_loaded: vector.loaded_before,
                    ..PresentationLineFlags::default()
                },
                resource: PresentationResourceId::new(vector.resource_id),
                terminal_frame: vector.terminal_frame_after,
                frame: vector.frame_drawn.unwrap_or(vector.frame_after),
                position: TEST_POSITION,
            };
            let mut playback = PresentationLinePlayback {
                busy: vector.busy_gate,
                reverse: vector.reverse_before & 1 != u8::MIN,
                redraw_requested: vector.ui_before & 4 != u8::MIN,
            };
            let mut backend = OracleBackend {
                terminal_frame: vector.loaded_terminal_frame,
                calls: Vec::new(),
                drawn_frame: None,
            };

            let outcome = update_presentation_line(&mut line, &mut playback, &mut backend).unwrap();

            assert_eq!(backend.calls, vector.helper_calls, "{}", vector.name);
            assert_eq!(backend.drawn_frame, vector.frame_drawn, "{}", vector.name);
            assert_eq!(
                line.terminal_frame, vector.terminal_frame_after,
                "{}",
                vector.name
            );
            assert_eq!(line.frame, vector.frame_after, "{}", vector.name);
            assert_eq!(
                line.flags.resource_loaded,
                vector.loaded_before || !vector.busy_gate,
                "{}",
                vector.name
            );
            assert_eq!(
                playback.reverse,
                vector.reverse_after & 1 != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                playback.redraw_requested,
                vector.ui_after & 4 != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome == PresentationLineOutcome::Completed,
                vector.completed_cf,
                "{}",
                vector.name
            );
        }
    }
}
