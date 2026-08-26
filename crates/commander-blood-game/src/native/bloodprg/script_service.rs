//! Stateful composition of translated COD, BAS, presentation, and action logic.

use std::fmt;

use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::script::{ScriptObjectId, ScriptState};

use super::{
    ScriptAboardRecordContext, ScriptActionContext, ScriptActionError, ScriptActionHost,
    ScriptActionState, ScriptBasDispatchError, ScriptBasDispatchHost, ScriptBasDispatchState,
    ScriptClock, ScriptControlFlowError, ScriptDialogueControlDispatchContext,
    ScriptDialogueExecutionContext, ScriptDispatchHost, ScriptDispatchState,
    ScriptEnvironmentActivity, ScriptPostScanContext, ScriptPresentationEntity,
    ScriptPresentationScanContext, ScriptPresentationScanError, ScriptPresentationScanHost,
    ScriptPresentationScanOutcome, ScriptPresentationScanState, ScriptRecordActionDispatchContext,
    ScriptRecordStateNavigationContext, ScriptRuntime, ScriptTransferContext,
    SequenceRequestContext, dispatch_script_action, execute_script_dialogue_control,
    scan_script_presentations,
};

/// Runtime facts and external effects required by translated BloodScript logic.
///
/// The script service owns all recovered game-state transitions. This boundary
/// contains only work that must be supplied by the modern renderer, audio
/// system, descriptor catalog, clock, and enclosing scene scheduler.
pub trait ScriptExecutionBackend {
    /// Backend failure propagated without erasing its concrete type.
    type Error;

    /// Apply the recovered pre-frame object-state processor.
    fn prepare_script_state(
        &mut self,
        state: &mut ScriptState,
        runtime: &mut ScriptRuntime,
        dispatch: &mut ScriptDispatchState,
    ) -> Result<(), Self::Error>;

    /// Return current bridge, travel, and contact activity.
    fn environment_activity(&self) -> ScriptEnvironmentActivity;

    /// Return the current game clock.
    fn clock(&self) -> ScriptClock;

    /// Return the UI gates used by authored sequence requests.
    fn sequence_context(&self) -> SequenceRequestContext;

    /// Resolve the dynamic navigation operands and active archetype.
    fn navigation_context(&self) -> Option<ScriptRecordStateNavigationContext>;

    /// Resolve descriptor and interface gates for an aboard operation.
    fn aboard_context(
        &mut self,
        related: ScriptObjectId,
    ) -> Result<ScriptAboardRecordContext, Self::Error>;

    /// Resolve descriptor and interface gates for an inventory transfer.
    fn transfer_context(
        &mut self,
        item: ScriptObjectId,
    ) -> Result<ScriptTransferContext, Self::Error>;

    /// Return the active BAS selector root for a committed concept.
    fn selector_root(&self) -> Option<ScriptCodeOffset>;

    /// Resolve and stage presentation assets for an object.
    fn lookup_presentation_description(
        &mut self,
        related: ScriptObjectId,
    ) -> Result<(), Self::Error>;

    /// Restart the name-area visual after new assets are staged.
    fn restart_name_area_effect(&mut self) -> Result<(), Self::Error>;

    /// Advance one fixed presentation renderer entity.
    fn transition_presentation_entity(
        &mut self,
        entity: ScriptPresentationEntity,
    ) -> Result<(), Self::Error>;

    /// Return whether an object has a descriptor record.
    fn description_available(&mut self, object: ScriptObjectId) -> Result<bool, Self::Error>;

    /// Restart navigation music after a descriptor-backed target change.
    fn restart_navigation_music(&mut self) -> Result<(), Self::Error>;

    /// Execute an object's authored COD state after an encounter update.
    fn execute_object_code(&mut self, object: ScriptObjectId) -> Result<(), Self::Error>;

    /// Start the fixed radio clip selected by a presentation action.
    fn play_radio_clip(&mut self) -> Result<(), Self::Error>;

    /// Start the camera transition used by black-hole travel.
    fn start_camera_transition(&mut self) -> Result<(), Self::Error>;

    /// Rebuild the ship HUD and reset the modern 3D camera.
    fn reset_ship_hud(&mut self) -> Result<(), Self::Error>;
}

/// Failure from an inline BAS or action callback during presentation scanning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptPresentationCallbackError<BackendError> {
    /// BAS selector or body execution failed.
    Dialogue(ScriptControlFlowError<ScriptBasDispatchError<BackendError>>),
    /// C1 through C8 post-frame action execution failed.
    Action(ScriptActionError<BackendError>),
    /// Descriptor, renderer, audio, camera, or nested-script work failed.
    Backend(BackendError),
}

impl<BackendError: fmt::Debug> fmt::Display for ScriptPresentationCallbackError<BackendError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl<BackendError: fmt::Debug> std::error::Error for ScriptPresentationCallbackError<BackendError> {}

/// Failure from the complete stateful BloodScript service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptExecutionServiceError<BackendError> {
    /// A shipped profile did not bind its required `blood` object.
    MissingPlayerBinding,
    /// A shipped profile did not bind its required `arche` object.
    MissingArchetypeBinding,
    /// The post-frame presentation and action scan failed.
    Presentation(ScriptPresentationScanError<ScriptPresentationCallbackError<BackendError>>),
    /// A pre-frame or COD-time backend operation failed.
    Backend(BackendError),
}

impl<BackendError: fmt::Debug> fmt::Display for ScriptExecutionServiceError<BackendError> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl<BackendError: fmt::Debug> std::error::Error for ScriptExecutionServiceError<BackendError> {}

/// Persistent translated state surrounding repeated execution of one profile.
pub struct ScriptExecutionService<Backend> {
    backend: Backend,
    presentation: ScriptPresentationScanState,
    action: ScriptActionState,
    bas: ScriptBasDispatchState,
    last_presentation_outcome: Option<ScriptPresentationScanOutcome>,
}

impl<Backend> ScriptExecutionService<Backend> {
    /// Construct a service around a concrete modern runtime backend.
    pub fn new(backend: Backend) -> Self {
        Self {
            backend,
            presentation: ScriptPresentationScanState::default(),
            action: ScriptActionState::default(),
            bas: ScriptBasDispatchState::default(),
            last_presentation_outcome: None,
        }
    }

    /// Borrow the modern runtime backend.
    pub const fn backend(&self) -> &Backend {
        &self.backend
    }

    /// Mutably borrow the modern runtime backend.
    pub fn backend_mut(&mut self) -> &mut Backend {
        &mut self.backend
    }

    /// Consume the service and return its modern runtime backend.
    pub fn into_backend(self) -> Backend {
        self.backend
    }

    /// Borrow persistent post-frame presentation state.
    pub const fn presentation_state(&self) -> &ScriptPresentationScanState {
        &self.presentation
    }

    /// Mutably borrow persistent post-frame presentation state.
    pub fn presentation_state_mut(&mut self) -> &mut ScriptPresentationScanState {
        &mut self.presentation
    }

    /// Borrow persistent C1 through C8 action state.
    pub const fn action_state(&self) -> &ScriptActionState {
        &self.action
    }

    /// Mutably borrow persistent C1 through C8 action state.
    pub fn action_state_mut(&mut self) -> &mut ScriptActionState {
        &mut self.action
    }

    /// Return the observable result of the most recent completed post-frame scan.
    pub const fn last_presentation_outcome(&self) -> Option<&ScriptPresentationScanOutcome> {
        self.last_presentation_outcome.as_ref()
    }

    /// Reset profile-local state when the main loop installs a different profile.
    pub fn reset_for_profile_change(&mut self) {
        self.presentation = ScriptPresentationScanState::default();
        self.action = ScriptActionState::default();
        self.bas.reset();
        self.last_presentation_outcome = None;
    }
}

impl<Backend: Default> Default for ScriptExecutionService<Backend> {
    fn default() -> Self {
        Self::new(Backend::default())
    }
}

impl<Backend: ScriptExecutionBackend> ScriptDispatchHost for ScriptExecutionService<Backend> {
    type Error = ScriptExecutionServiceError<Backend::Error>;

    fn prepare_script_state(
        &mut self,
        state: &mut ScriptState,
        runtime: &mut ScriptRuntime,
        dispatch: &mut ScriptDispatchState,
    ) -> Result<(), Self::Error> {
        self.backend
            .prepare_script_state(state, runtime, dispatch)
            .map_err(ScriptExecutionServiceError::Backend)
    }

    fn environment_activity(&self) -> ScriptEnvironmentActivity {
        self.backend.environment_activity()
    }

    fn clock(&self) -> ScriptClock {
        self.backend.clock()
    }

    fn sequence_context(&self) -> SequenceRequestContext {
        self.backend.sequence_context()
    }

    fn navigation_context(&self) -> Option<ScriptRecordStateNavigationContext> {
        self.backend.navigation_context()
    }

    fn aboard_context(
        &mut self,
        related: ScriptObjectId,
    ) -> Result<ScriptAboardRecordContext, Self::Error> {
        self.backend
            .aboard_context(related)
            .map_err(ScriptExecutionServiceError::Backend)
    }

    fn transfer_context(
        &mut self,
        item: ScriptObjectId,
    ) -> Result<ScriptTransferContext, Self::Error> {
        self.backend
            .transfer_context(item)
            .map_err(ScriptExecutionServiceError::Backend)
    }

    fn selector_root(&self) -> Option<ScriptCodeOffset> {
        self.backend.selector_root()
    }

    fn scan_presentation(&mut self, context: ScriptPostScanContext<'_>) -> Result<(), Self::Error> {
        let player = context
            .builtins
            .player
            .ok_or(ScriptExecutionServiceError::MissingPlayerBinding)?;
        let arche = context
            .builtins
            .archetype
            .ok_or(ScriptExecutionServiceError::MissingArchetypeBinding)?;
        let ScriptPostScanContext {
            instructions,
            dialogue,
            state,
            dictionary,
            directory,
            records,
            selector,
            runtime,
            dispatch,
            builtins,
        } = context;
        let outcome = {
            let mut adapter = PresentationAdapter {
                instructions,
                dialogue,
                dictionary,
                directory,
                builtins,
                dispatch,
                bas: &mut self.bas,
                action: &mut self.action,
                backend: &mut self.backend,
            };
            scan_script_presentations(
                ScriptPresentationScanContext {
                    state,
                    records,
                    runtime,
                    selector,
                    presentation: &mut self.presentation,
                    player,
                    arche,
                },
                &mut adapter,
            )
            .map_err(ScriptExecutionServiceError::Presentation)?
        };
        self.last_presentation_outcome = Some(outcome);
        Ok(())
    }
}

struct PresentationAdapter<'a, Backend> {
    instructions: &'a [commander_blood_formats::instruction::DecodedScriptInstruction],
    dialogue: &'a commander_blood_formats::bas::ScriptBas,
    dictionary: &'a commander_blood_formats::script::ScriptDictionary,
    directory: &'a commander_blood_formats::script::ScriptDirectory,
    builtins: super::ScriptProfileBuiltins,
    dispatch: &'a mut ScriptDispatchState,
    bas: &'a mut ScriptBasDispatchState,
    action: &'a mut ScriptActionState,
    backend: &'a mut Backend,
}

impl<Backend: ScriptExecutionBackend> ScriptPresentationScanHost<super::ScriptProfileRecordState>
    for PresentationAdapter<'_, Backend>
{
    type Error = ScriptPresentationCallbackError<Backend::Error>;

    fn dispatch_dialogue_control(
        &mut self,
        context: ScriptDialogueControlDispatchContext<'_, super::ScriptProfileRecordState>,
    ) -> Result<(), Self::Error> {
        let mut host = ExternalHost {
            backend: self.backend,
        };
        execute_script_dialogue_control(
            ScriptDialogueExecutionContext {
                actor: context.actor,
                selector_root: context.selector_root,
                instructions: self.instructions,
                dialogue: self.dialogue,
                state: context.state,
                dictionary: self.dictionary,
                directory: self.directory,
                builtins: self.builtins,
                runtime: context.runtime,
                selector: context.selector,
                records: context.records,
                dispatch: self.dispatch,
                bas: self.bas,
            },
            &mut host,
        )
        .map_err(ScriptPresentationCallbackError::Dialogue)?;
        Ok(())
    }

    fn lookup_presentation_description(
        &mut self,
        related: ScriptObjectId,
    ) -> Result<(), Self::Error> {
        self.backend
            .lookup_presentation_description(related)
            .map_err(ScriptPresentationCallbackError::Backend)
    }

    fn restart_name_area_effect(&mut self) -> Result<(), Self::Error> {
        self.backend
            .restart_name_area_effect()
            .map_err(ScriptPresentationCallbackError::Backend)
    }

    fn transition_presentation_entity(
        &mut self,
        entity: ScriptPresentationEntity,
    ) -> Result<(), Self::Error> {
        self.backend
            .transition_presentation_entity(entity)
            .map_err(ScriptPresentationCallbackError::Backend)
    }

    fn dispatch_record_action(
        &mut self,
        context: ScriptRecordActionDispatchContext<'_, super::ScriptProfileRecordState>,
    ) -> Result<super::ScriptActionDispatch, Self::Error> {
        let player = self
            .builtins
            .player
            .expect("service validates the player binding before scanning");
        let navigation = self.backend.navigation_context();
        let action_records = &mut context.records.action_records;
        let aboard_objects = context.records.record_runtime.aboard_objects_mut();
        let mut host = ExternalHost {
            backend: self.backend,
        };
        dispatch_script_action(
            ScriptActionContext {
                state: context.state,
                records: action_records,
                aboard_objects,
                request_flags: &mut self.dispatch.text_presentation.request_flags,
                presentation: context.presentation,
                action: self.action,
                owner: context.owner,
                slot: context.slot,
                player,
                navigation,
            },
            context.record,
            &mut host,
        )
        .map_err(ScriptPresentationCallbackError::Action)
    }
}

struct ExternalHost<'a, Backend> {
    backend: &'a mut Backend,
}

impl<Backend: ScriptExecutionBackend> ScriptBasDispatchHost for ExternalHost<'_, Backend> {
    type Error = Backend::Error;

    fn sequence_context(&self) -> SequenceRequestContext {
        self.backend.sequence_context()
    }

    fn transfer_context(
        &mut self,
        item: ScriptObjectId,
    ) -> Result<ScriptTransferContext, Self::Error> {
        self.backend.transfer_context(item)
    }
}

impl<Backend: ScriptExecutionBackend> ScriptActionHost for ExternalHost<'_, Backend> {
    type Error = Backend::Error;

    fn description_available(&mut self, object: ScriptObjectId) -> Result<bool, Self::Error> {
        self.backend.description_available(object)
    }

    fn restart_navigation_music(&mut self) -> Result<(), Self::Error> {
        self.backend.restart_navigation_music()
    }

    fn execute_object_code(&mut self, object: ScriptObjectId) -> Result<(), Self::Error> {
        self.backend.execute_object_code(object)
    }

    fn play_radio_clip(&mut self) -> Result<(), Self::Error> {
        self.backend.play_radio_clip()
    }

    fn start_camera_transition(&mut self) -> Result<(), Self::Error> {
        self.backend.start_camera_transition()
    }

    fn reset_ship_hud(&mut self) -> Result<(), Self::Error> {
        self.backend.reset_ship_hud()
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::path::{Path, PathBuf};

    use crate::assets::OriginalResourceStore;

    use super::super::{
        OriginalResourceCache, OriginalResourceCatalog, OriginalScriptProfileCatalog,
        ScriptFrameEnd, ScriptProfileBuiltins, ScriptProfileId, ScriptProfileManager,
        execute_loaded_script_frame,
    };
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum BackendEvent {
        Prepared,
        DescriptionLookup(ScriptObjectId),
        NameAreaRestart,
        EntityTransition(ScriptPresentationEntity),
        DescriptionProbe(ScriptObjectId),
        NavigationMusicRestart,
        ObjectCode(ScriptObjectId),
        RadioClip,
        CameraTransition,
        ShipHudReset,
    }

    struct RecordingBackend {
        builtins: ScriptProfileBuiltins,
        events: Vec<BackendEvent>,
    }

    impl ScriptExecutionBackend for RecordingBackend {
        type Error = Infallible;

        fn prepare_script_state(
            &mut self,
            _state: &mut ScriptState,
            _runtime: &mut ScriptRuntime,
            _dispatch: &mut ScriptDispatchState,
        ) -> Result<(), Self::Error> {
            self.events.push(BackendEvent::Prepared);
            Ok(())
        }

        fn environment_activity(&self) -> ScriptEnvironmentActivity {
            ScriptEnvironmentActivity {
                bridge_active: true,
                travel_active: true,
                contact_active: true,
            }
        }

        fn clock(&self) -> ScriptClock {
            ScriptClock {
                hour: 12,
                day: 2,
                month: 1,
            }
        }

        fn sequence_context(&self) -> SequenceRequestContext {
            SequenceRequestContext {
                ship_active: true,
                scene_gate_active: true,
            }
        }

        fn navigation_context(&self) -> Option<ScriptRecordStateNavigationContext> {
            Some(ScriptRecordStateNavigationContext {
                primary_object: self.builtins.player?,
                secondary_object: self.builtins.player?,
                arche: self.builtins.archetype?,
            })
        }

        fn aboard_context(
            &mut self,
            related: ScriptObjectId,
        ) -> Result<ScriptAboardRecordContext, Self::Error> {
            self.events.push(BackendEvent::DescriptionProbe(related));
            Ok(ScriptAboardRecordContext {
                ship_interface_active: false,
                descriptor_available: true,
            })
        }

        fn transfer_context(
            &mut self,
            item: ScriptObjectId,
        ) -> Result<ScriptTransferContext, Self::Error> {
            self.events.push(BackendEvent::DescriptionProbe(item));
            Ok(ScriptTransferContext {
                ship_interface_active: false,
                descriptor_available: true,
            })
        }

        fn selector_root(&self) -> Option<ScriptCodeOffset> {
            None
        }

        fn lookup_presentation_description(
            &mut self,
            related: ScriptObjectId,
        ) -> Result<(), Self::Error> {
            self.events.push(BackendEvent::DescriptionLookup(related));
            Ok(())
        }

        fn restart_name_area_effect(&mut self) -> Result<(), Self::Error> {
            self.events.push(BackendEvent::NameAreaRestart);
            Ok(())
        }

        fn transition_presentation_entity(
            &mut self,
            entity: ScriptPresentationEntity,
        ) -> Result<(), Self::Error> {
            self.events.push(BackendEvent::EntityTransition(entity));
            Ok(())
        }

        fn description_available(&mut self, object: ScriptObjectId) -> Result<bool, Self::Error> {
            self.events.push(BackendEvent::DescriptionProbe(object));
            Ok(true)
        }

        fn restart_navigation_music(&mut self) -> Result<(), Self::Error> {
            self.events.push(BackendEvent::NavigationMusicRestart);
            Ok(())
        }

        fn execute_object_code(&mut self, object: ScriptObjectId) -> Result<(), Self::Error> {
            self.events.push(BackendEvent::ObjectCode(object));
            Ok(())
        }

        fn play_radio_clip(&mut self) -> Result<(), Self::Error> {
            self.events.push(BackendEvent::RadioClip);
            Ok(())
        }

        fn start_camera_transition(&mut self) -> Result<(), Self::Error> {
            self.events.push(BackendEvent::CameraTransition);
            Ok(())
        }

        fn reset_ship_hud(&mut self) -> Result<(), Self::Error> {
            self.events.push(BackendEvent::ShipHudReset);
            Ok(())
        }
    }

    fn original_data_root() -> Option<PathBuf> {
        [
            Path::new("output/_tmp_iso"),
            Path::new("commander-blood-audio/_tmp_iso"),
            Path::new("accuracy/cblood_install/cblood"),
        ]
        .into_iter()
        .find(|root| root.join("SCRIPT1.COD").is_file())
        .map(Path::to_owned)
    }

    #[test]
    fn every_shipped_profile_runs_through_the_complete_stateful_service() {
        let Some(root) = original_data_root() else {
            return;
        };
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let store = OriginalResourceStore::new(root, None, [], true);
        let resources = OriginalResourceCatalog::decode_bloodprg(executable).unwrap();
        let catalog = OriginalScriptProfileCatalog::decode_bloodprg(executable).unwrap();
        let mut cache = OriginalResourceCache::new();
        let mut manager = ScriptProfileManager::new(catalog);

        for profile_id in ScriptProfileId::all() {
            manager
                .select(profile_id, &mut cache, &store, &resources)
                .unwrap();
            let profile = manager.current_mut().unwrap();
            let object_count = profile.state().objects().len();
            let mut dispatch = ScriptDispatchState::default();
            let mut service = ScriptExecutionService::new(RecordingBackend {
                builtins: profile.builtins(),
                events: Vec::new(),
            });

            let outcome = execute_loaded_script_frame(profile, true, &mut dispatch, &mut service)
                .unwrap_or_else(|error| {
                    panic!(
                        "profile {} service failed: {error:?}",
                        profile_id.value() + 1
                    )
                });

            assert_ne!(outcome.end, ScriptFrameEnd::ExecutionDisabled);
            assert_eq!(
                service
                    .last_presentation_outcome()
                    .expect("completed post-frame scan")
                    .processed_objects,
                object_count
            );
            assert_eq!(
                service.backend().events.first(),
                Some(&BackendEvent::Prepared)
            );
            profile.synchronized_state().unwrap();
        }
    }
}
