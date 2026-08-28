//! Concrete service adapter for the recovered bridge-frame coordinator.

use anyhow::{Context, Result};

use crate::native::bloodprg::{
    BridgeActorPresentationState, BridgeFrameBackend, BridgeFrameOutcome, BridgeSceneContext,
    BridgeSceneInput, BridgeSpriteExtent, BridgeSpriteRange, GameLifecycleState, GameSceneLink,
    Manu3AnimationSelector, render_bridge_frame as coordinate_bridge_frame,
};

use super::ModernGameServices;
use super::camera_approach::update_runtime_camera_approach;

const FIRST_BRIDGE_ENTITY: u16 = 0;
const FIRST_TRANSITION_ENTITY: u16 = 20;
const FIRST_ACTOR_ENTITY: u16 = 1;
const AFTER_LAST_BRIDGE_ENTITY: u16 = 32;
const AFTER_LAST_ACTOR_ENTITY: u16 = 20;
const LOCATION_PANEL_ENTITY_INDEX: usize = 0;

/// Run one bridge frame through the translated coordinator and concrete owners.
pub(super) fn run_runtime_bridge_frame(
    services: &mut ModernGameServices<'_>,
    lifecycle: &mut GameLifecycleState,
    scene_link: GameSceneLink,
    input: BridgeSceneInput,
    navigation_animation_phase: u8,
) -> Result<BridgeFrameOutcome> {
    let presentation_primary_pressed = lifecycle.primary_pointer_pressed;
    let scene_dispatch_pending = services.bridge_scene_dispatch_pending();
    let transition_pending = services.runtime().camera_approach().transition_pending;
    let primary_camera_view = services.bridge_actor_camera_transition_step()? == u8::MIN;
    let actor_completion = services.bridge_actor_completion_latched()?;
    let mouse_x = services.input().pointer_sample().position[0] as u16;
    let mut state = services.runtime_mut().take_bridge_frame_state();
    state.set_active(lifecycle.presentation_interface_active());
    state.set_scene_dispatch_pending(scene_dispatch_pending);
    state.set_screen_rebuild_pending(lifecycle.navigation_rebuild_pending);
    state.set_transition_pending(transition_pending);
    state.set_presentation_queued(lifecycle.presentation.c2_presentation_gate);
    state.set_primary_camera_view(primary_camera_view);
    state.set_frame_ready(lifecycle.frame_presented);
    state.set_actor_completion(actor_completion);
    state.set_mouse_x(mouse_x);

    let mut context = BridgeSceneContext::new(scene_link, BridgeSpriteExtent::default());
    let result = {
        let mut backend = RuntimeBridgeFrameBackend {
            services,
            lifecycle,
            input,
            navigation_animation_phase,
            presentation_primary_pressed,
        };
        coordinate_bridge_frame(&mut state, &mut context, &mut backend)
            .context("coordinating recovered bridge frame")
    };
    services.runtime_mut().restore_bridge_frame_state(state);
    result
}

struct RuntimeBridgeFrameBackend<'state, 'window> {
    services: &'state mut ModernGameServices<'window>,
    lifecycle: &'state mut GameLifecycleState,
    input: BridgeSceneInput,
    navigation_animation_phase: u8,
    presentation_primary_pressed: bool,
}

impl BridgeFrameBackend for RuntimeBridgeFrameBackend<'_, '_> {
    type SceneLink = GameSceneLink;
    type ComparisonExtent = BridgeSpriteExtent;
    type Error = anyhow::Error;

    fn dispatch_scene(
        &mut self,
        _scene_link: &Self::SceneLink,
        _state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        self.services
            .dispatch_ship_scene()
            .context("dispatching the bridge travel scene")?;
        Ok(())
    }

    fn initialize_screen_flags(
        &mut self,
        state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        self.services
            .request_manu3_animation(Manu3AnimationSelector::BridgeActive);
        self.services
            .set_previous_manu3_animation(Manu3AnimationSelector::BridgeActive);
        self.services.initialize_bridge_screen_with_transition(
            self.lifecycle.presentation_mode,
            self.lifecycle.presentation.ship_active,
            state.transition_pending(),
        )?;
        self.lifecycle.navigation_rebuild_pending = false;
        state.set_screen_rebuild_pending(false);
        state.set_actor_completion(false);
        Ok(())
    }

    fn update_steering(
        &mut self,
        context: &mut BridgeSceneContext<Self::SceneLink, Self::ComparisonExtent>,
        _state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<bool> {
        let comparison_extent = self
            .services
            .runtime()
            .bridge_sprite_source_extent(LOCATION_PANEL_ENTITY_INDEX)
            .context("reading the bridge camera comparison extent")?;
        *context = BridgeSceneContext::new(*context.scene_link(), comparison_extent);
        Ok(self
            .services
            .update_bridge_steering(self.input)?
            .view_changed)
    }

    fn flip_page(&mut self, state: &mut crate::native::bloodprg::BridgeFrameState) -> Result<()> {
        let selector = match state.actor_presentation() {
            BridgeActorPresentationState::SteeringRight => Manu3AnimationSelector::SteeringRight,
            BridgeActorPresentationState::SteeringLeft => Manu3AnimationSelector::SteeringLeft,
            other => unreachable!("page flip requested for non-steering state {other:?}"),
        };
        self.services.request_manu3_animation(selector);
        self.services
            .flip_bridge_camera_page(self.lifecycle.presentation.ship_active)?;
        Ok(())
    }

    fn advance_camera_transition(
        &mut self,
        scene_link: &Self::SceneLink,
        state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        update_runtime_camera_approach(self.services, *scene_link, self.lifecycle)?;
        state.set_transition_pending(self.services.runtime().camera_approach().transition_pending);
        Ok(())
    }

    fn update_presentation_mode_bits(
        &mut self,
        _state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        self.services.update_bridge_presentation_mode_bits()
    }

    fn commit_sprite_geometry(
        &mut self,
        range: BridgeSpriteRange,
        _state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        self.services.commit_ship_entities(entity_range(range))?;
        Ok(())
    }

    fn dispatch_presentation_mode(
        &mut self,
        _state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        self.services.render_current_bridge_frame()?;
        self.services.update_bridge_presentation_hover();
        Ok(())
    }

    fn update_actor_slots(
        &mut self,
        state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        self.services.update_runtime_bridge_actors(self.lifecycle)?;
        state.set_transition_pending(self.services.runtime().camera_approach().transition_pending);
        state.set_presentation_queued(self.lifecycle.presentation.c2_presentation_gate);
        state.set_actor_completion(self.services.bridge_actor_completion_latched()?);
        Ok(())
    }

    fn render_dirty_sprites(
        &mut self,
        range: BridgeSpriteRange,
        _state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        self.services
            .rasterize_bridge_frame_sprite_range(entity_range(range))?;
        Ok(())
    }

    fn copy_dirty_regions(
        &mut self,
        _state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        self.services.copy_ship_dirty_regions()?;
        Ok(())
    }

    fn check_camera_state(
        &mut self,
        comparison_extent: &Self::ComparisonExtent,
        _state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        if self.services.runtime().current_profile().is_some() {
            self.services
                .update_runtime_navigation_chart_with_comparison(
                    self.lifecycle,
                    self.navigation_animation_phase,
                    *comparison_extent,
                )?;
        }
        Ok(())
    }

    fn update_camera_navigation(
        &mut self,
        _state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        if self.services.runtime().current_profile().is_some() {
            self.services
                .update_runtime_camera_navigation(self.lifecycle)?;
        }
        Ok(())
    }

    fn update_screen_mode(
        &mut self,
        scene_link: &Self::SceneLink,
        state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        self.services
            .update_presentation_screen(scene_link, self.presentation_primary_pressed)?;
        self.services
            .consume_presentation_screen_outputs(self.lifecycle)?;
        state.set_frame_ready(self.lifecycle.frame_presented);
        state.set_presentation_queued(self.lifecycle.presentation.c2_presentation_gate);
        state.set_screen_rebuild_pending(self.lifecycle.navigation_rebuild_pending);
        Ok(())
    }

    fn update_name_area_palette(
        &mut self,
        _state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        self.services.advance_bridge_name_area_effect()?;
        Ok(())
    }

    fn update_navigation_state(
        &mut self,
        _state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        if self.services.runtime().current_profile().is_some() {
            self.services
                .update_runtime_navigation_status(self.lifecycle)?;
        }
        Ok(())
    }

    fn dispatch_navigation_choice(
        &mut self,
        _state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        self.services.update_runtime_bridge_console(self.lifecycle)
    }

    fn remap_completion_region(
        &mut self,
        _state: &mut crate::native::bloodprg::BridgeFrameState,
    ) -> Result<()> {
        self.services.remap_bridge_completion_region()?;
        Ok(())
    }
}

const fn entity_range(range: BridgeSpriteRange) -> std::ops::Range<u16> {
    match range {
        BridgeSpriteRange::All => FIRST_BRIDGE_ENTITY..AFTER_LAST_BRIDGE_ENTITY,
        BridgeSpriteRange::Transition => FIRST_TRANSITION_ENTITY..AFTER_LAST_BRIDGE_ENTITY,
        BridgeSpriteRange::Actors => FIRST_ACTOR_ENTITY..AFTER_LAST_ACTOR_ENTITY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_ranges_map_to_exact_half_open_entity_sets() {
        assert_eq!(entity_range(BridgeSpriteRange::All), 0..32);
        assert_eq!(entity_range(BridgeSpriteRange::Transition), 20..32);
        assert_eq!(entity_range(BridgeSpriteRange::Actors), 1..20);
    }
}
