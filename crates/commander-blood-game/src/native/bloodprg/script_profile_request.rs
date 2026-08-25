//! Flat pending-profile state written by BloodScript D2 instructions.

use std::fmt;

use commander_blood_formats::instruction::ScriptProfileRequest;

use super::ScriptProfileId;

const NO_PENDING_PROFILE_INDEX: i16 = -1;

/// Exact semantic state of the native pending-profile word.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PendingScriptProfileRequest {
    /// No profile switch is pending.
    #[default]
    Empty,
    /// A request names one of the five playable profiles.
    Profile(ScriptProfileId),
    /// A malformed script produced a signed index outside the playable domain.
    Invalid(i16),
}

impl PendingScriptProfileRequest {
    /// Return the signed value the original D2 handler stored.
    pub const fn raw_zero_based_index(self) -> i16 {
        match self {
            Self::Empty => NO_PENDING_PROFILE_INDEX,
            Self::Profile(profile) => profile.value() as i16,
            Self::Invalid(index) => index,
        }
    }
}

/// Invalid pending request detected before attempting a profile load.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptProfileRequestError {
    /// Signed zero-based index produced by the authored operand.
    pub requested_index: i16,
}

impl fmt::Display for ScriptProfileRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid BloodScript profile index {}",
            self.requested_index
        )
    }
}

impl std::error::Error for ScriptProfileRequestError {}

/// One-shot profile request owned by the modern main-loop coordinator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptProfileRequestSlot {
    pending: PendingScriptProfileRequest,
}

impl ScriptProfileRequestSlot {
    /// Apply `vm_op_d2_script_profile_request` to typed flat state.
    pub fn schedule(&mut self, request: ScriptProfileRequest) -> PendingScriptProfileRequest {
        let requested_index = request.zero_based_profile_index();
        self.pending = if requested_index == NO_PENDING_PROFILE_INDEX {
            PendingScriptProfileRequest::Empty
        } else if let Ok(index) = u8::try_from(requested_index) {
            ScriptProfileId::new(index).map_or(
                PendingScriptProfileRequest::Invalid(requested_index),
                PendingScriptProfileRequest::Profile,
            )
        } else {
            PendingScriptProfileRequest::Invalid(requested_index)
        };
        self.pending
    }

    /// Return the exact pending state without consuming it.
    pub const fn pending(self) -> PendingScriptProfileRequest {
        self.pending
    }

    /// Resolve a playable request for the profile loader.
    ///
    /// The slot remains armed until [`Self::clear_after_load`], because the
    /// original main loop only clears it after profile selection succeeds.
    pub const fn pending_profile(
        self,
    ) -> Result<Option<ScriptProfileId>, ScriptProfileRequestError> {
        match self.pending {
            PendingScriptProfileRequest::Empty => Ok(None),
            PendingScriptProfileRequest::Profile(profile) => Ok(Some(profile)),
            PendingScriptProfileRequest::Invalid(requested_index) => {
                Err(ScriptProfileRequestError { requested_index })
            }
        }
    }

    /// Clear a request after its profile has loaded successfully.
    pub fn clear_after_load(&mut self) {
        self.pending = PendingScriptProfileRequest::Empty;
    }
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::code::decode_script_code;
    use commander_blood_formats::instruction::decode_script_profile_request;
    use serde::Deserialize;

    use super::*;

    const PROFILE_REQUEST_OPCODE: u8 = 0xD2;
    const CODE_END_MARKER: u8 = 0xFF;

    #[derive(Deserialize)]
    struct ProfileRequestOracle {
        operand_byte: u8,
        stored_request: u16,
    }

    fn decode_request(operand: u8) -> ScriptProfileRequest {
        let code = decode_script_code(&[PROFILE_REQUEST_OPCODE, operand, CODE_END_MARKER]).unwrap();
        decode_script_profile_request(&code.tokens()[0]).unwrap()
    }

    #[test]
    fn requests_match_every_original_signed_operand_vector() {
        let vectors: Vec<ProfileRequestOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_64b8_natural.json"
        ))
        .unwrap();

        for vector in vectors {
            let request = decode_request(vector.operand_byte);
            let mut slot = ScriptProfileRequestSlot::default();
            let pending = slot.schedule(request);

            assert_eq!(
                request.zero_based_profile_index() as u16,
                vector.stored_request
            );
            assert_eq!(pending.raw_zero_based_index() as u16, vector.stored_request);
        }
    }

    #[test]
    fn valid_requests_remain_armed_until_the_profile_load_succeeds() {
        let mut slot = ScriptProfileRequestSlot::default();
        slot.schedule(decode_request(5));
        let expected = ScriptProfileId::new(4).unwrap();

        assert_eq!(slot.pending_profile().unwrap(), Some(expected));
        assert_eq!(slot.pending_profile().unwrap(), Some(expected));

        slot.clear_after_load();
        assert_eq!(slot.pending_profile().unwrap(), None);
    }

    #[test]
    fn invalid_requests_are_reported_without_clamping_or_clearing() {
        let mut slot = ScriptProfileRequestSlot::default();
        slot.schedule(decode_request(127));

        assert_eq!(
            slot.pending_profile().unwrap_err(),
            ScriptProfileRequestError {
                requested_index: 126,
            }
        );
        assert_eq!(slot.pending(), PendingScriptProfileRequest::Invalid(126));
    }
}
