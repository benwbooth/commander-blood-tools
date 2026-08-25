//! Initialization and indirect dispatch for alien resume behavior.

use super::AlienSpecies;

/// Resume callback selected by the recovered slot-13 coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumeCallback {
    /// Begin the species-specific resume state machine.
    Begin,
}

/// Typed continuation state owned by one resumable behavior method.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienResumeMethodState {
    /// Callback selected for the next coordinator invocation.
    pub callback: Option<AlienResumeCallback>,
    /// Current phase within the resume state machine.
    pub phase: u16,
    /// Optional node paired with the currently resumed node.
    pub paired_node: Option<usize>,
    /// Optional node whose state is being resumed.
    pub resumed_node: Option<usize>,
}

/// Callback boundary retained by the recovered resume coordinator.
pub trait AlienResumeCallbacks {
    /// Error returned by the concrete callback implementation.
    type Error;

    /// Invoke the selected resume callback.
    fn invoke(
        &mut self,
        species: AlienSpecies,
        callback: AlienResumeCallback,
        state: &mut AlienResumeMethodState,
    ) -> Result<(), Self::Error>;
}

/// Stage completed by one invocation of the resume coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumeUpdate {
    /// The initial callback and empty pairing state were installed.
    Initialized,
    /// The previously selected callback was invoked.
    CallbackInvoked,
}

/// Initialize or dispatch the recovered slot-13 resume method.
pub fn initialize_or_dispatch_resume<C: AlienResumeCallbacks>(
    species: AlienSpecies,
    state: &mut AlienResumeMethodState,
    callbacks: &mut C,
) -> Result<AlienResumeUpdate, C::Error> {
    if let Some(callback) = state.callback {
        callbacks.invoke(species, callback, state)?;
        return Ok(AlienResumeUpdate::CallbackInvoked);
    }

    state.callback = Some(AlienResumeCallback::Begin);
    state.phase = u16::MIN;
    state.paired_node = None;
    Ok(AlienResumeUpdate::Initialized)
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use serde::Deserialize;

    use super::*;

    const PRESERVED_RESUMED_NODE: usize = 37;

    #[derive(Deserialize)]
    struct ResumeVector {
        name: String,
        module: String,
        resume_before: u16,
        resume_after: u16,
        resume_step_before: u16,
        resume_step_after: u16,
        resume_value_before: u16,
        resume_value_after: u16,
        tail_dispatched: bool,
    }

    #[derive(Default)]
    struct CallbackRecorder {
        calls: Vec<(AlienSpecies, AlienResumeCallback)>,
    }

    impl AlienResumeCallbacks for CallbackRecorder {
        type Error = Infallible;

        fn invoke(
            &mut self,
            species: AlienSpecies,
            callback: AlienResumeCallback,
            _state: &mut AlienResumeMethodState,
        ) -> Result<(), Self::Error> {
            self.calls.push((species, callback));
            Ok(())
        }
    }

    fn species(module: &str) -> AlienSpecies {
        match module {
            "amer" => AlienSpecies::Amer,
            "croolis" => AlienSpecies::Croolis,
            "scrut" => AlienSpecies::Scrut,
            _ => panic!("unknown alien module {module}"),
        }
    }

    fn fixtures() -> [&'static str; 3] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1bea_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1b46_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1bfb_natural.json"),
        ]
    }

    #[test]
    fn initialization_matches_every_original_overlay_vector() {
        for fixture in fixtures() {
            let vectors: Vec<ResumeVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors.into_iter().take(3) {
                assert_eq!(vector.resume_before, u16::MIN);
                assert!(!vector.tail_dispatched);
                let mut state = AlienResumeMethodState {
                    callback: None,
                    phase: vector.resume_step_before,
                    paired_node: Some(usize::from(vector.resume_value_before)),
                    resumed_node: Some(PRESERVED_RESUMED_NODE),
                };
                let mut callbacks = CallbackRecorder::default();

                assert_eq!(
                    initialize_or_dispatch_resume(
                        species(&vector.module),
                        &mut state,
                        &mut callbacks,
                    )
                    .unwrap(),
                    AlienResumeUpdate::Initialized,
                    "{}",
                    vector.name
                );
                assert_eq!(state.callback, Some(AlienResumeCallback::Begin));
                assert_ne!(vector.resume_after, u16::MIN);
                assert_eq!(state.phase, vector.resume_step_after);
                assert_eq!(state.paired_node, None);
                assert_eq!(vector.resume_value_after, u16::MIN);
                assert_eq!(state.resumed_node, Some(PRESERVED_RESUMED_NODE));
                assert!(callbacks.calls.is_empty());
            }
        }
    }

    #[test]
    fn dispatch_matches_every_original_overlay_vector() {
        for fixture in fixtures() {
            let vectors: Vec<ResumeVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors.into_iter().skip(3) {
                assert_ne!(vector.resume_before, u16::MIN);
                assert!(vector.tail_dispatched);
                let paired_node = Some(usize::from(vector.resume_value_before));
                let mut state = AlienResumeMethodState {
                    callback: Some(AlienResumeCallback::Begin),
                    phase: vector.resume_step_before,
                    paired_node,
                    resumed_node: Some(PRESERVED_RESUMED_NODE),
                };
                let mut callbacks = CallbackRecorder::default();

                assert_eq!(
                    initialize_or_dispatch_resume(
                        species(&vector.module),
                        &mut state,
                        &mut callbacks,
                    )
                    .unwrap(),
                    AlienResumeUpdate::CallbackInvoked,
                    "{}",
                    vector.name
                );
                assert_eq!(state.callback, Some(AlienResumeCallback::Begin));
                assert_eq!(vector.resume_after, vector.resume_before);
                assert_eq!(state.phase, vector.resume_step_after);
                assert_eq!(vector.resume_step_after, vector.resume_step_before);
                assert_eq!(state.paired_node, paired_node);
                assert_eq!(vector.resume_value_after, vector.resume_value_before);
                assert_eq!(state.resumed_node, Some(PRESERVED_RESUMED_NODE));
                assert_eq!(
                    callbacks.calls,
                    vec![(species(&vector.module), AlienResumeCallback::Begin)]
                );
            }
        }
    }
}
