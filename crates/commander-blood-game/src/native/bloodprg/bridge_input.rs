//! Typed bridge pointer hit-testing and fixed-region polling.

use super::PresentationHitRectangle;

/// Number of attempts made while polling the bridge status region.
pub const STATUS_REGION_POLL_ATTEMPTS: usize = 32;

/// One primary-pointer sample in logical bridge coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimaryPointerSample {
    /// Whether the primary button is currently pressed.
    pub primary_pressed: bool,
    /// Signed logical pointer position.
    pub position: [i16; 2],
}

/// Latch a primary-pointer hit while preserving an existing hit.
///
/// This translates `mouse_hit_test` at BLOODPRG routine offset `0x008269`.
/// A boolean latch replaces the native hit bit while the shared typed rectangle
/// retains the original signed, inclusive, wrapping coordinate comparisons.
pub fn latch_primary_pointer_hit(
    pointer: PrimaryPointerSample,
    region: PresentationHitRectangle,
    hit_latched: &mut bool,
) {
    if primary_pointer_hits_region(pointer, region) {
        *hit_latched = true;
    }
}

/// Test whether a pressed primary pointer lies inside a bridge region.
///
/// This translates `region_record_hittest` at BLOODPRG routine offset
/// `0x008295`. The modern return value is an ordinary boolean; native carry
/// state, calling convention, and pointer representation are eliminated.
pub fn primary_pointer_hits_region(
    pointer: PrimaryPointerSample,
    region: PresentationHitRectangle,
) -> bool {
    pointer.primary_pressed && region.contains(pointer.position)
}

/// Dynamic pointer and entity state read by the fixed status-region poll.
pub trait StatusRegionPollBackend {
    /// Return whether the status-region entity is enabled for this attempt.
    fn status_region_enabled(&mut self, attempts_remaining: u8) -> bool;

    /// Sample the primary pointer for an enabled-region attempt.
    fn primary_pointer_sample(&mut self, attempts_remaining: u8) -> PrimaryPointerSample;
}

/// Successful status-region poll result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusRegionPollHit {
    /// Attempts that remained when the pointer hit was observed.
    pub attempts_remaining: u8,
}

/// Poll one fixed bridge status region up to 32 times.
///
/// This translates `ui_region_31_poll` at BLOODPRG routine offset `0x0082C3`.
/// The original repeatedly rereads entity and mouse state, so the backend is
/// queried on every attempt. A typed rectangle and bounded loop replace entity
/// table slot arithmetic and a negative integer sentinel.
pub fn poll_status_region<Backend: StatusRegionPollBackend>(
    region: PresentationHitRectangle,
    backend: &mut Backend,
) -> Option<StatusRegionPollHit> {
    let initial_attempts_remaining = STATUS_REGION_POLL_ATTEMPTS - 1;
    for completed_attempts in 0..STATUS_REGION_POLL_ATTEMPTS {
        let attempts_remaining = (initial_attempts_remaining - completed_attempts) as u8;
        if backend.status_region_enabled(attempts_remaining) {
            let pointer = backend.primary_pointer_sample(attempts_remaining);
            if primary_pointer_hits_region(pointer, region) {
                return Some(StatusRegionPollHit { attempts_remaining });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const HIT_TEST_VECTOR_COUNT: usize = 12;
    const POLL_VECTOR_COUNT: usize = 6;
    const NO_POLL_HIT: i16 = -1;

    #[derive(Deserialize)]
    struct HitTestOracle {
        name: String,
        primary: u8,
        mouse: [i16; 2],
        rect: [i16; 4],
        initial_hit_flags: Option<u8>,
        result_hit_flags: Option<u8>,
        hit: bool,
    }

    #[test]
    fn latch_matches_every_original_mouse_hit_vector() {
        let vectors: Vec<HitTestOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_8269_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), HIT_TEST_VECTOR_COUNT);

        for vector in vectors {
            let mut hit_latched = vector.initial_hit_flags.unwrap() & 8 != u8::MIN;
            latch_primary_pointer_hit(pointer(&vector), rectangle(vector.rect), &mut hit_latched);
            assert_eq!(
                hit_latched,
                vector.result_hit_flags.unwrap() & 8 != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(hit_latched, vector.hit, "{}", vector.name);
        }
    }

    #[test]
    fn boolean_hit_test_matches_every_original_region_vector() {
        let vectors: Vec<HitTestOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_8295_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), HIT_TEST_VECTOR_COUNT);

        for vector in vectors {
            assert_eq!(
                primary_pointer_hits_region(pointer(&vector), rectangle(vector.rect)),
                vector.hit,
                "{}",
                vector.name
            );
        }
    }

    fn pointer(vector: &HitTestOracle) -> PrimaryPointerSample {
        PrimaryPointerSample {
            primary_pressed: vector.primary & 1 != u8::MIN,
            position: vector.mouse,
        }
    }

    fn rectangle(values: [i16; 4]) -> PresentationHitRectangle {
        PresentationHitRectangle::new([values[0], values[1]], [values[2], values[3]])
    }

    #[derive(Deserialize)]
    struct PollOracle {
        name: String,
        mouse: [i16; 2],
        primary: u8,
        initial_flags: u16,
        rect: [i16; 4],
        flags_on_iteration: Option<usize>,
        primary_on_call: Option<usize>,
        result: i16,
        iterations: usize,
        calls: Vec<PollCall>,
    }

    #[derive(Deserialize)]
    struct PollCall {
        attempts_remaining: u8,
        rect: [i16; 4],
    }

    struct OraclePollBackend {
        pointer_position: [i16; 2],
        primary_initially_pressed: bool,
        region_initially_enabled: bool,
        flags_on_iteration: Option<usize>,
        primary_on_call: Option<usize>,
        iterations: usize,
        pointer_calls: Vec<u8>,
    }

    impl StatusRegionPollBackend for OraclePollBackend {
        fn status_region_enabled(&mut self, _attempts_remaining: u8) -> bool {
            self.iterations += 1;
            self.region_initially_enabled
                || self
                    .flags_on_iteration
                    .is_some_and(|iteration| self.iterations >= iteration)
        }

        fn primary_pointer_sample(&mut self, attempts_remaining: u8) -> PrimaryPointerSample {
            self.pointer_calls.push(attempts_remaining);
            PrimaryPointerSample {
                primary_pressed: self.primary_initially_pressed
                    || self
                        .primary_on_call
                        .is_some_and(|call| self.pointer_calls.len() >= call),
                position: self.pointer_position,
            }
        }
    }

    #[test]
    fn fixed_region_poll_matches_every_original_vector() {
        let vectors: Vec<PollOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_82c3_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), POLL_VECTOR_COUNT);

        for vector in vectors {
            let mut backend = OraclePollBackend {
                pointer_position: vector.mouse,
                primary_initially_pressed: vector.primary & 1 != u8::MIN,
                region_initially_enabled: vector.initial_flags & 1 != u16::MIN,
                flags_on_iteration: vector.flags_on_iteration,
                primary_on_call: vector.primary_on_call,
                iterations: usize::MIN,
                pointer_calls: Vec::new(),
            };
            let result = poll_status_region(rectangle(vector.rect), &mut backend);

            assert_eq!(
                result.map(|hit| i16::from(hit.attempts_remaining)),
                (vector.result != NO_POLL_HIT).then_some(vector.result),
                "{}",
                vector.name
            );
            assert_eq!(backend.iterations, vector.iterations, "{}", vector.name);
            assert_eq!(
                backend.pointer_calls,
                vector
                    .calls
                    .iter()
                    .map(|call| call.attempts_remaining)
                    .collect::<Vec<_>>(),
                "{}",
                vector.name
            );
            assert!(
                vector.calls.iter().all(|call| call.rect == vector.rect),
                "{}",
                vector.name
            );
        }
    }
}
