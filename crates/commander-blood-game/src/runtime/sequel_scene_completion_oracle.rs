//! Native scene/lifecycle comparison at the empty queue boundary, not a decoder test.

use super::*;
use crate::native::bloodprg::{
    GameLifecycleState, IndexedGamePalette, PresentationPresentPolicy,
    PresentationSceneDispatchContext, PresentationSceneDispatchHost, PresentationSceneQueueService,
    PresentationSceneSource, dispatch_presentation_scene,
};
use crate::runtime::services::publish_sequel_scene_completion;
use serde::Deserialize;
use std::ops::Range;

#[derive(Deserialize)]
struct Fields {
    vm: u8,
    line: u16,
    displayed: u16,
    gate: u8,
    request: u8,
    redraw: u8,
    ship: u16,
    blocked: u8,
    overlay: u8,
    sound: u8,
    finale: u8,
    navigation_sound: u8,
    entry: u16,
    read: u16,
    palette: u16,
    depth_opening: u8,
    depth_step: u8,
    queue_status: u8,
    buffered: u16,
}

impl Fields {
    fn scene(&self) -> PresentationSceneDispatchState<DescriptBackgroundSlot> {
        let mut scene = PresentationSceneDispatchState::default();
        scene.presentation.active_line = (self.line != u16::MAX).then_some(self.line);
        scene.presentation.gate_flags = self.gate;
        scene.presentation.request_flags = self.request;
        scene.presentation.bridge_redraw_pending = self.redraw;
        scene.displayed_line = (self.displayed != u16::MAX).then_some(self.displayed);
        scene.ship_active_flags = self.ship;
        scene.dispatch_blocked = self.blocked & 1 != 0;
        scene.alien_overlay_armed = self.overlay & 1 != 0;
        scene.temporary_sound_trigger = self.sound & 1 != 0;
        scene.finale_requested = self.finale & 1 != 0;
        scene.navigation_choice_sound_gate = self.navigation_sound & 1 != 0;
        scene.entry_metric = self.entry;
        scene.read_wrap_index = self.read;
        scene.palette_transition_percent = self.palette;
        scene.depth_opening = self.depth_opening & 1 != 0;
        scene.depth_step = self.depth_step;
        scene
    }
}

#[derive(Deserialize)]
struct Vector {
    name: String,
    mode: String,
    input: Fields,
    output: Fields,
    calls: Vec<u16>,
}

struct EmptyQueueBoundary<'a> {
    input: &'a Fields,
    calls: Vec<u16>,
}

impl PresentationSceneDispatchHost<DescriptBackgroundSlot> for EmptyQueueBoundary<'_> {
    type Error = std::convert::Infallible;

    fn load_scene_image(
        &mut self,
        _: &DescriptBackgroundSlot,
        _: &mut IndexedGamePalette,
    ) -> Result<(), Self::Error> {
        panic!("the active-scene continuation cannot load an image")
    }

    fn clear_back_buffer_band(&mut self, _: Range<usize>, _: u8) -> Result<(), Self::Error> {
        panic!("the active-scene continuation cannot clear the back buffer")
    }

    fn load_presentation_sequence(
        &mut self,
        _: PresentationResourceId,
        _: PresentationSceneSource,
        _: PresentationPresentPolicy,
    ) -> Result<bool, Self::Error> {
        panic!("the active-scene continuation cannot load a sequence")
    }

    fn build_black_remap(&mut self, _: u8, _: [u8; 3]) -> Result<(), Self::Error> {
        panic!("the active-scene continuation cannot build a remap")
    }

    fn service_presentation_queue(
        &mut self,
        _: PresentationPresentPolicy,
    ) -> Result<PresentationSceneQueueService, Self::Error> {
        self.calls.push(0xB997);
        Ok(PresentationSceneQueueService {
            frame_presented: false,
            entry_metric: self.input.entry,
            read_wrap_index: self.input.read,
        })
    }

    fn presentation_source_open_or_draining(&mut self) -> bool {
        self.calls.push(0xBBF5);
        self.input.queue_status <= 1
    }

    fn clear_display_band(&mut self, _: Range<usize>, _: u8) -> Result<(), Self::Error> {
        panic!("line-five framebuffer work is outside these captures")
    }
}

#[test]
fn sequel_scene_completion_matches_original_latches_and_vm_writes() {
    let vectors: Vec<Vector> =
        include_str!("../../../../re/tools/oracle_vectors/big_bug_bang_scene_completion.jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
    assert_eq!(vectors.len(), 38);
    for vector in vectors {
        let mut scene = vector.input.scene();
        let completed = if vector.mode == "dispatch" {
            let mut host = EmptyQueueBoundary {
                input: &vector.input,
                calls: Vec::new(),
            };
            let mut palette = [[0; 3]; 256];
            let mut presentation_palette = [[0; 3]; 64];
            let mut context = PresentationSceneDispatchContext::<u16, _> {
                scenes: &[],
                active_record_related: None,
                scruter_jo_record: None,
                unclamped_line_ids: &[0; 8],
                shared_cache_available: false,
                scene_palette: &mut palette,
                presentation_palette: &mut presentation_palette,
            };
            let outcome = dispatch_presentation_scene(&mut scene, &mut context, &mut host).unwrap();
            assert_eq!(host.calls, vector.calls, "{}", vector.name);
            assert_eq!(vector.output.queue_status, vector.input.queue_status);
            outcome == PresentationSceneDispatchOutcome::PresentationFinished
        } else {
            assert_eq!(vector.mode, "cancel");
            let completed = release_scene_presentation(&mut scene);
            let calls = if !completed {
                vec![]
            } else if vector.input.buffered == 0 {
                vec![0xBAC7, 0xB924]
            } else {
                vec![0xBAC7]
            };
            assert_eq!(calls, vector.calls, "{}", vector.name);
            completed
        };
        assert_eq!(scene, vector.output.scene(), "{}", vector.name);
        assert_eq!(vector.input.buffered, vector.output.buffered);
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.vm_execution_enabled = vector.input.vm != 0;
        publish_sequel_scene_completion(true, completed, &mut lifecycle);
        assert_eq!(
            lifecycle.vm_execution_enabled,
            vector.output.vm != 0,
            "{}",
            vector.name
        );

        lifecycle.vm_execution_enabled = vector.input.vm != 0;
        publish_sequel_scene_completion(false, completed, &mut lifecycle);
        assert_eq!(
            lifecycle.vm_execution_enabled,
            vector.input.vm != 0,
            "Commander: {}",
            vector.name
        );
    }
}

#[test]
fn sequel_panel_completion_is_consumed_before_a_new_ui_pause() {
    let mut screen = RuntimePresentationScreen::new([[0; 3]; 256]).unwrap();
    let mut lifecycle = GameLifecycleState::default();
    lifecycle.vm_execution_enabled = false;
    assert!(!screen.take_scene_completion_output());
    screen.scene_completion_output = true;
    publish_sequel_scene_completion(true, screen.take_scene_completion_output(), &mut lifecycle);
    assert!(lifecycle.vm_execution_enabled);
    lifecycle.vm_execution_enabled = false;
    publish_sequel_scene_completion(true, screen.take_scene_completion_output(), &mut lifecycle);
    assert!(!lifecycle.vm_execution_enabled);
}
