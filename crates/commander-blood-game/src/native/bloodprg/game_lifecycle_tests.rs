use std::convert::Infallible;

use serde::Deserialize;

use super::*;

const ORACLE_VECTOR_COUNT: usize = 11;
const LOGICAL_VIEWPORT: [u64; 6] = [0, 1, 4, 320, 200, 0];
const INITIAL_SCENE_LINK_OFFSET: u16 = 16;
const SUBTITLE_OWNER_OFFSET: u16 = 24_164;
const MENU_WORDS_OFFSET: u16 = 0;

#[derive(Deserialize)]
struct MainOracle {
    name: String,
    allocations: Vec<[u64; 2]>,
    viewport: Vec<u64>,
    events: Vec<serde_json::Value>,
    final_scene_link_target: u16,
}

#[derive(Clone, Copy)]
struct Scenario {
    panorama_opens: bool,
    frames: usize,
    pending_profile: Option<ScriptProfileId>,
    profile_status: GameProfileLoadStatus,
    vm_status: GameVmRunStatus,
    presentation_mode: bool,
    presentation_active: bool,
    scene_gate_active: bool,
    active_line: Option<u16>,
    list_entry_metric: u16,
    list_read_wrap_index: u16,
    owner: Option<GamePresentationOwner>,
    request_flags: u8,
    completion_audio_pending: bool,
    subtitle_word_list_mode: bool,
    subtitle_voice_trigger: bool,
    dialogue_hold_complete: bool,
    word_buffer_nonempty: bool,
    dialogue_hold_countdown: u16,
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            panorama_opens: true,
            frames: usize::MIN,
            pending_profile: None,
            profile_status: GameProfileLoadStatus::Loaded,
            vm_status: GameVmRunStatus::Continue,
            presentation_mode: true,
            presentation_active: false,
            scene_gate_active: false,
            active_line: None,
            list_entry_metric: u16::MIN,
            list_read_wrap_index: u16::MIN,
            owner: None,
            request_flags: u8::MIN,
            completion_audio_pending: false,
            subtitle_word_list_mode: false,
            subtitle_voice_trigger: false,
            dialogue_hold_complete: false,
            word_buffer_nonempty: false,
            dialogue_hold_countdown: u16::MIN,
        }
    }
}

struct OracleHost {
    scenario: Scenario,
    input_dispatches: usize,
    pending_profiles_at_input: Vec<Option<ScriptProfileId>>,
    calls: Vec<&'static str>,
}

impl OracleHost {
    fn call(&mut self, name: &'static str) {
        self.calls.push(name);
    }
}

macro_rules! plain_host_call {
    ($method:ident, $name:literal) => {
        fn $method(&mut self) -> Result<(), Self::Error> {
            self.call($name);
            Ok(())
        }
    };
}

macro_rules! state_host_call {
    ($method:ident, $name:literal) => {
        fn $method(&mut self, _state: &mut GameLifecycleState) -> Result<(), Self::Error> {
            self.call($name);
            Ok(())
        }
    };
}

impl GameLifecycleHost for OracleHost {
    type Error = Infallible;

    plain_host_call!(initialize_runtime_storage, "initialize_runtime_storage");
    plain_host_call!(
        prepare_startup_resources,
        "startup_loading_screen_and_write_directory_prepare"
    );
    plain_host_call!(
        initialize_archive_index,
        "resource_archive_index_backing_initialize"
    );
    plain_host_call!(prepare_cd_audio, "cdrom_audio_prepare");
    plain_host_call!(load_manu3_overlay, "load_manu3_overlay");
    plain_host_call!(initialize_logical_viewport, "initialize_logical_viewport");

    fn open_bridge_panorama(&mut self) -> Result<bool, Self::Error> {
        self.call("open_bridge_panorama");
        Ok(self.scenario.panorama_opens)
    }

    plain_host_call!(load_save_slots, "load_save_slots");
    plain_host_call!(load_startup_audio, "load_startup_audio");
    plain_host_call!(configure_startup_audio, "configure_startup_audio");
    plain_host_call!(load_initial_audio_resource, "load_initial_audio_resource");
    plain_host_call!(randomize_ship_point_cloud, "ship_3d_point_cloud_randomize");

    fn run_initial_presentation(&mut self, link: GameSceneLink) -> Result<(), Self::Error> {
        assert_eq!(link, GameSceneLink::Initial);
        self.call("presentation_line_zero_run");
        Ok(())
    }

    plain_host_call!(load_default_sound_bank, "snd_bank_loader");
    plain_host_call!(initialize_back_buffer, "back_buffer_init");

    fn dispatch_input(&mut self, state: &mut GameLifecycleState) -> Result<(), Self::Error> {
        self.call("input_action_dispatch");
        self.pending_profiles_at_input.push(state.pending_profile);
        self.input_dispatches += 1;
        if self.input_dispatches == 1 {
            apply_scenario(state, self.scenario);
            let failure_driven = self.scenario.profile_status == GameProfileLoadStatus::Failed
                || self.scenario.vm_status == GameVmRunStatus::ExitRequested;
            if self.scenario.frames == usize::MIN && !failure_driven {
                state.exit_requested = true;
            }
        } else {
            state.exit_requested = true;
        }
        Ok(())
    }

    state_host_call!(poll_pointer, "poll_mouse");
    state_host_call!(refresh_pause_hud, "main_loop_hud_refresh");
    state_host_call!(update_pointer_buttons, "mouse_button_edges_update");

    fn run_vm(&mut self, _state: &mut GameLifecycleState) -> Result<GameVmRunStatus, Self::Error> {
        self.call("vm_run_wrapper");
        Ok(self.scenario.vm_status)
    }

    fn load_profile(
        &mut self,
        _profile: ScriptProfileId,
        _state: &mut GameLifecycleState,
    ) -> Result<GameProfileLoadStatus, Self::Error> {
        self.call("vm_resource_profile_select");
        Ok(self.scenario.profile_status)
    }

    state_host_call!(rebuild_record_state, "vm_record_state_proc");
    state_host_call!(refresh_object_access, "object_heap_access");
    state_host_call!(
        reset_ship_hud,
        "ship_3d_hud_palette_snapshot_and_camera_reset"
    );
    plain_host_call!(stop_completion_audio, "snd_driver_call");
    plain_host_call!(load_completion_audio, "snd_stream_source_load");
    plain_host_call!(start_completion_audio, "snd_stream_start");

    fn render_bridge_frame(
        &mut self,
        _link: GameSceneLink,
        _state: &mut GameLifecycleState,
    ) -> Result<(), Self::Error> {
        self.call("bridge_render_frame");
        Ok(())
    }

    state_host_call!(update_confirm_dialog, "confirm_dialog_step");
    plain_host_call!(refill_audio_stream, "snd_stream_refill");
    state_host_call!(process_audio, "audio_process_ade");

    fn update_ship_presentation(
        &mut self,
        _link: GameSceneLink,
        _state: &mut GameLifecycleState,
    ) -> Result<(), Self::Error> {
        self.call("ship_presentation_fsm");
        Ok(())
    }

    fn update_scene_transition(
        &mut self,
        _link: GameSceneLink,
        _state: &mut GameLifecycleState,
    ) -> Result<(), Self::Error> {
        self.call("scene_transition_step");
        Ok(())
    }

    state_host_call!(update_save_load, "save_load_menu_step");
    state_host_call!(
        update_presentation_choices,
        "presentation_choice_transition_step"
    );
    state_host_call!(mark_presentation_ready, "presentation_ready_gate");
    plain_host_call!(submit_indexed_frame, "chunky_to_planar_framebuffer");
    state_host_call!(reveal_inline_menu, "dlg_menu_words_inline_reveal_step");
    state_host_call!(update_subtitles, "subtitle_reveal_pump");
    state_host_call!(update_manu3, "manu3_hand_frame_dispatch");
    state_host_call!(update_palette_transition, "palette_transition_step");

    fn pace_frame(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    plain_host_call!(present_frame, "present_frame");
    plain_host_call!(finish_presentations, "presentation_update_1fb2");
    plain_host_call!(stop_audio, "snd_driver_call");

    fn run_final_presentation(&mut self, _link: GameSceneLink) -> Result<(), Self::Error> {
        self.call("presentation_line_one_stream_run");
        Ok(())
    }

    plain_host_call!(remove_transient_voice, "remove_transient_voice");
    plain_host_call!(remove_transient_music, "remove_transient_music");
    plain_host_call!(
        remove_transient_archive_index,
        "remove_transient_archive_index"
    );
    plain_host_call!(delete_startup_transients, "startup_transient_files_delete");
    plain_host_call!(close_bridge_panorama, "close_bridge_panorama");
}

#[test]
fn recovered_ui_bits_keep_presentation_and_profile_gates_independent() {
    let mut state = GameLifecycleState::default();
    state.set_presentation_interface_active(true);
    assert!(state.presentation_interface_active());
    assert!(!state.profile_ui_blocked());

    state.set_modal_ui_busy(true);
    assert!(state.profile_ui_blocked());
    state.set_navigation_ui_busy(true);
    state.set_modal_ui_busy(false);
    assert!(state.profile_ui_blocked());

    state.set_navigation_ui_busy(false);
    assert!(!state.profile_ui_blocked());
    assert!(state.presentation_interface_active());
}

#[test]
fn lifecycle_matches_all_original_control_flow_vectors() {
    let vectors: Vec<MainOracle> = serde_json::from_str(include_str!(
        "../../../../../re/tools/oracle_vectors/func_0eb0_natural.json"
    ))
    .unwrap();
    assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

    for vector in vectors {
        assert_native_storage_evidence(&vector);
        let scenario = scenario_for(&vector.name);
        let mut host = OracleHost {
            scenario,
            input_dispatches: usize::MIN,
            pending_profiles_at_input: Vec::new(),
            calls: Vec::new(),
        };
        let mut state = GameLifecycleState::default();
        let outcome = run_game_lifecycle(&mut state, &mut host).unwrap();

        assert_eq!(
            host.calls,
            normalized_oracle_calls(&vector),
            "{}",
            vector.name
        );
        assert_eq!(
            outcome.final_scene_link,
            scene_link_for_native_offset(vector.final_scene_link_target),
            "{}",
            vector.name
        );
        assert_eq!(
            outcome.rendered_frames, scenario.frames as u64,
            "{}",
            vector.name
        );
        assert_case_outcome(&vector.name, outcome.exit);
        assert_case_state(&vector.name, &state);
        if scenario.panorama_opens {
            assert_eq!(
                host.pending_profiles_at_input.first(),
                Some(&Some(ScriptProfileId::INITIAL)),
                "{}",
                vector.name
            );
        } else {
            assert!(host.pending_profiles_at_input.is_empty(), "{}", vector.name);
        }
    }
}

fn apply_scenario(state: &mut GameLifecycleState, scenario: Scenario) {
    state.exit_requested = false;
    state.pause_hud_active = false;
    state.pointer_position_locked = false;
    state.pointer_press_pending = u8::MIN;
    state.set_modal_ui_busy(false);
    state.set_navigation_ui_busy(false);
    state.pending_profile = scenario.pending_profile;
    state.profile_change_blockers = GameProfileChangeBlockers::default();
    state.presentation_mode = scenario.presentation_mode;
    state.presentation = GamePresentationScheduler {
        active: scenario.presentation_active,
        scene_gate_active: scenario.scene_gate_active,
        active_line: scenario.active_line,
        list_entry_metric: scenario.list_entry_metric,
        list_read_wrap_index: scenario.list_read_wrap_index,
        owner: scenario.owner,
        request_flags: PresentationRequestFlags::decode(scenario.request_flags),
        completion_audio_pending: scenario.completion_audio_pending,
        subtitle_word_list_mode: scenario.subtitle_word_list_mode,
        subtitle_voice_trigger: scenario.subtitle_voice_trigger,
        dialogue_hold_complete: scenario.dialogue_hold_complete,
        word_buffer_nonempty: scenario.word_buffer_nonempty,
        dialogue_hold_countdown: scenario.dialogue_hold_countdown,
        ..GamePresentationScheduler::default()
    };
    state.frame_presented = false;
}

fn scenario_for(name: &str) -> Scenario {
    match name {
        "tb_big_open_failure" => Scenario {
            panorama_opens: false,
            ..Scenario::default()
        },
        "input_exit_and_complete_cleanup" => Scenario::default(),
        "one_normal_frame_then_exit" => Scenario {
            frames: 1,
            ..Scenario::default()
        },
        "profile_switch_full_sequence" => Scenario {
            frames: 1,
            pending_profile: ScriptProfileId::new(3),
            ..Scenario::default()
        },
        "profile_switch_failure_shuts_down" => Scenario {
            pending_profile: ScriptProfileId::new(4),
            profile_status: GameProfileLoadStatus::Failed,
            ..Scenario::default()
        },
        "vm_failure_shuts_down" => Scenario {
            vm_status: GameVmRunStatus::ExitRequested,
            presentation_mode: false,
            ..Scenario::default()
        },
        "presentation_owner_is_forwarded" => Scenario {
            frames: 1,
            presentation_active: true,
            scene_gate_active: true,
            active_line: Some(7),
            list_entry_metric: 5,
            list_read_wrap_index: 1,
            owner: Some(GamePresentationOwner::Subtitle),
            ..Scenario::default()
        },
        "zero_owner_selects_menu_word_buffer" => Scenario {
            frames: 1,
            presentation_active: true,
            scene_gate_active: true,
            active_line: Some(7),
            list_entry_metric: 5,
            list_read_wrap_index: 1,
            ..Scenario::default()
        },
        "request_bit_zero_preserves_text_modes_and_plays_audio" => Scenario {
            frames: 1,
            request_flags: PRIMARY_TEXT_REQUEST_PENDING,
            completion_audio_pending: true,
            subtitle_word_list_mode: true,
            subtitle_voice_trigger: true,
            ..Scenario::default()
        },
        "request_bit_one_clears_text_modes" => Scenario {
            frames: 1,
            request_flags: 2,
            subtitle_word_list_mode: true,
            subtitle_voice_trigger: true,
            ..Scenario::default()
        },
        "dialogue_countdown_holds_completion" => Scenario {
            frames: 1,
            dialogue_hold_complete: true,
            word_buffer_nonempty: true,
            dialogue_hold_countdown: 1,
            ..Scenario::default()
        },
        other => panic!("unknown main-loop oracle {other}"),
    }
}

fn normalized_oracle_calls(vector: &MainOracle) -> Vec<&'static str> {
    let mut calls = Vec::new();
    let mut storage_initialized = false;
    for event in &vector.events {
        let name = event["event"].as_str().unwrap();
        match name {
            "resource_allocate" if !storage_initialized => {
                storage_initialized = true;
                calls.push("initialize_runtime_storage");
            }
            "resource_allocate" => {}
            "startup_loading_screen_and_write_directory_prepare" => {
                calls.push("startup_loading_screen_and_write_directory_prepare")
            }
            "resource_archive_index_backing_initialize" => {
                calls.push("resource_archive_index_backing_initialize")
            }
            "cdrom_audio_prepare" => calls.push("cdrom_audio_prepare"),
            "resource_file_load" => match event["path"].as_str().unwrap() {
                "manu3.xdb" => calls.push("load_manu3_overlay"),
                "blood.sav" => calls.push("load_save_slots"),
                path => panic!("unexpected lifecycle resource {path}"),
            },
            "resource_source_select" => calls.push("initialize_logical_viewport"),
            "dos_open" => calls.push("open_bridge_panorama"),
            "resource_load_by_id" => calls.push("load_startup_audio"),
            "resource_handle_resolve" => calls.push("configure_startup_audio"),
            "audio_param_init_cd5" => {}
            "resource_named_file_load" => calls.push("load_initial_audio_resource"),
            "ship_3d_point_cloud_randomize" => calls.push("ship_3d_point_cloud_randomize"),
            "presentation_line_zero_run" => calls.push("presentation_line_zero_run"),
            "snd_bank_loader" => calls.push("snd_bank_loader"),
            "back_buffer_init" => calls.push("back_buffer_init"),
            "mouse_position_set" => {}
            "input_action_dispatch" => calls.push("input_action_dispatch"),
            "poll_mouse" => calls.push("poll_mouse"),
            "main_loop_hud_refresh" => calls.push("main_loop_hud_refresh"),
            "mouse_button_edges_update" => calls.push("mouse_button_edges_update"),
            "vm_run_wrapper" => calls.push("vm_run_wrapper"),
            "vm_resource_profile_select" => calls.push("vm_resource_profile_select"),
            "vm_record_state_proc" => calls.push("vm_record_state_proc"),
            "object_heap_access" => calls.push("object_heap_access"),
            "ship_3d_hud_palette_snapshot_and_camera_reset" => {
                calls.push("ship_3d_hud_palette_snapshot_and_camera_reset")
            }
            "snd_driver_call" => calls.push("snd_driver_call"),
            "snd_stream_source_load" => calls.push("snd_stream_source_load"),
            "snd_stream_start" => calls.push("snd_stream_start"),
            "bridge_render_frame" => calls.push("bridge_render_frame"),
            "confirm_dialog_step" => calls.push("confirm_dialog_step"),
            "snd_stream_refill" => calls.push("snd_stream_refill"),
            "audio_process_ade" => calls.push("audio_process_ade"),
            "ship_presentation_fsm" => calls.push("ship_presentation_fsm"),
            "scene_transition_step" => calls.push("scene_transition_step"),
            "save_load_menu_step" => calls.push("save_load_menu_step"),
            "presentation_choice_transition_step" => {
                calls.push("presentation_choice_transition_step")
            }
            "presentation_ready_gate" => calls.push("presentation_ready_gate"),
            "chunky_to_planar_framebuffer" => calls.push("chunky_to_planar_framebuffer"),
            "dlg_menu_words_inline_reveal_step" => calls.push("dlg_menu_words_inline_reveal_step"),
            "subtitle_reveal_pump" => calls.push("subtitle_reveal_pump"),
            "manu3_hand_frame_dispatch" => calls.push("manu3_hand_frame_dispatch"),
            "palette_transition_step" => calls.push("palette_transition_step"),
            "page_offset_helper" => calls.push("present_frame"),
            "palette_upload_if_dirty" => {}
            "presentation_update_1fb2" => calls.push("presentation_update_1fb2"),
            "presentation_line_one_stream_run" => calls.push("presentation_line_one_stream_run"),
            "startup_write_directory_enter"
            | "dos_delete"
            | "startup_original_directory_restore" => {}
            "startup_transient_files_delete" => calls.push("startup_transient_files_delete"),
            "dos_close" => match event["handle"].as_u64().unwrap() {
                4_369 => calls.push("remove_transient_voice"),
                8_738 => calls.push("remove_transient_music"),
                13_107 => calls.push("remove_transient_archive_index"),
                17_476 => calls.push("close_bridge_panorama"),
                handle => panic!("unexpected lifecycle handle {handle}"),
            },
            other => panic!("unexpected lifecycle event {other}"),
        }
    }
    calls
}

fn assert_native_storage_evidence(vector: &MainOracle) {
    assert_eq!(
        vector.allocations,
        vec![
            [8, 65_536],
            [10, 65_536],
            [11, 65_552],
            [12, 65_536],
            [9, 65_536],
            [100, 65_552],
        ],
        "{}",
        vector.name
    );
    assert_eq!(vector.viewport, LOGICAL_VIEWPORT, "{}", vector.name);
}

fn scene_link_for_native_offset(offset: u16) -> GameSceneLink {
    match offset {
        INITIAL_SCENE_LINK_OFFSET => GameSceneLink::Initial,
        SUBTITLE_OWNER_OFFSET => GameSceneLink::SubtitlePresentation,
        MENU_WORDS_OFFSET => GameSceneLink::MenuWords,
        other => panic!("unmapped native scene-link offset {other}"),
    }
}

fn assert_case_outcome(name: &str, actual: GameLifecycleExit) {
    let expected = match name {
        "tb_big_open_failure" => GameLifecycleExit::BridgePanoramaUnavailable,
        "profile_switch_failure_shuts_down" => GameLifecycleExit::ProfileLoadFailed,
        "vm_failure_shuts_down" => GameLifecycleExit::VmRequestedExit,
        _ => GameLifecycleExit::InputRequested,
    };
    assert_eq!(actual, expected, "{name}");
}

fn assert_case_state(name: &str, state: &GameLifecycleState) {
    match name {
        "request_bit_zero_preserves_text_modes_and_plays_audio" => {
            assert!(state.presentation.subtitle_word_list_mode);
            assert!(state.presentation.subtitle_voice_trigger);
            assert_eq!(
                state.completion_audio_reset_frames,
                Some(COMPLETION_AUDIO_RESET_FRAMES)
            );
        }
        "request_bit_one_clears_text_modes" => {
            assert!(!state.presentation.subtitle_word_list_mode);
            assert!(!state.presentation.subtitle_voice_trigger);
        }
        "dialogue_countdown_holds_completion" => {
            assert!(state.presentation.dialogue_hold_complete);
            assert!(!state.presentation.word_choice_active);
        }
        "zero_owner_selects_menu_word_buffer" => {
            assert_eq!(
                state.presentation.menu_word_source,
                GameMenuWordSource::PresentationBuffer
            );
            assert!(state.presentation.menu_deferred);
        }
        "presentation_owner_is_forwarded" => {
            assert!(state.presentation.subtitle_display_active);
        }
        _ => {}
    }
}
