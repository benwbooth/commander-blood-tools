//! Bridge page preparation over explicit render targets and typed outcomes.

/// Logical destination selected for bridge page rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgePageTarget {
    /// Retained page prepared before it is presented.
    Secondary,
    /// Page currently visible to the player.
    Primary,
}

/// Mutable page state formerly stored in shared graphics flag bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BridgePageState {
    /// The indexed palette must be uploaded before presentation.
    pub palette_dirty: bool,
    /// Palette index zero is transparent during retained-page composition.
    pub transparent_zero: bool,
    /// Retained dirty regions must be copied during the next presentation.
    pub dirty_copy_requested: bool,
}

/// Semantic result returned after preparing a bridge page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgePageOutcome {
    /// The active 3D ship layer remains the prepared page.
    ShipLayerPrepared,
    /// The inactive ship path restored a panorama frame on the primary page.
    PanoramaLoaded {
        /// Panorama frame loaded from the decoded bridge asset.
        frame: u16,
    },
}

/// Renderer and decoded-asset operations sequenced by page preparation.
pub trait BridgePageBackend {
    /// Backend error returned by rendering or resource work.
    type Error;

    /// Clear the retained page to palette index zero.
    fn clear_page(
        &mut self,
        target: BridgePageTarget,
        state: &BridgePageState,
    ) -> Result<(), Self::Error>;
    /// Build the current 3D ship projection matrix.
    fn build_ship_projection(&mut self, state: &BridgePageState) -> Result<(), Self::Error>;
    /// Project the ship-view point cloud.
    fn project_ship_point_cloud(&mut self, state: &BridgePageState) -> Result<(), Self::Error>;
    /// Project ship objects into typed sprite geometry.
    fn project_ship_objects(&mut self, state: &BridgePageState) -> Result<(), Self::Error>;
    /// Commit the authored ship-object sprite group.
    fn commit_ship_sprites(
        &mut self,
        target: BridgePageTarget,
        state: &BridgePageState,
    ) -> Result<(), Self::Error>;
    /// Render the authored ship-object sprite group.
    fn render_ship_sprites(
        &mut self,
        target: BridgePageTarget,
        state: &BridgePageState,
    ) -> Result<(), Self::Error>;
    /// Decode and draw one panorama frame on the primary page.
    fn load_panorama_frame(
        &mut self,
        target: BridgePageTarget,
        frame: u16,
        state: &BridgePageState,
    ) -> Result<(), Self::Error>;
}

/// Prepare the retained ship page and restore the panorama when appropriate.
///
/// This translates `page_flip` at BLOODPRG routine offset `0x00954A`.
/// Explicit primary and secondary targets replace display-buffer pointer swaps;
/// semantic booleans replace shared graphics flag bytes and no native address
/// survives into the renderer interface.
pub fn render_bridge_page<Backend: BridgePageBackend>(
    ship_active: bool,
    panorama_frame: u16,
    state: &mut BridgePageState,
    backend: &mut Backend,
) -> Result<BridgePageOutcome, Backend::Error> {
    state.palette_dirty = true;
    backend.clear_page(BridgePageTarget::Secondary, state)?;
    backend.build_ship_projection(state)?;
    backend.project_ship_point_cloud(state)?;
    backend.project_ship_objects(state)?;
    backend.commit_ship_sprites(BridgePageTarget::Secondary, state)?;
    backend.render_ship_sprites(BridgePageTarget::Secondary, state)?;

    if ship_active {
        return Ok(BridgePageOutcome::ShipLayerPrepared);
    }

    state.transparent_zero = true;
    state.dirty_copy_requested = true;
    backend.load_panorama_frame(BridgePageTarget::Primary, panorama_frame, state)?;
    Ok(BridgePageOutcome::PanoramaLoaded {
        frame: panorama_frame,
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 7;
    const NATIVE_SHIP_ACTIVE_FLAG: u16 = 1;

    #[derive(Deserialize)]
    struct PageOracle {
        name: String,
        ship_flags: u16,
        bridge_frame: Option<u16>,
        transparent_before: u8,
        transparent_after: u8,
        dirty_copy_before: u8,
        dirty_copy_after: u8,
        calls: Vec<CallOracle>,
    }

    #[derive(Deserialize)]
    struct CallOracle {
        name: String,
        palette_dirty: u8,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Event {
        Clear(BridgePageTarget),
        BuildProjection,
        ProjectPointCloud,
        ProjectObjects,
        CommitShipSprites(BridgePageTarget),
        RenderShipSprites(BridgePageTarget),
        LoadPanorama(BridgePageTarget, u16),
    }

    #[derive(Default)]
    struct OracleBackend {
        events: Vec<Event>,
        palette_dirty_at_call: Vec<bool>,
    }

    impl OracleBackend {
        fn record(&mut self, event: Event, state: &BridgePageState) {
            self.events.push(event);
            self.palette_dirty_at_call.push(state.palette_dirty);
        }
    }

    impl BridgePageBackend for OracleBackend {
        type Error = std::convert::Infallible;

        fn clear_page(
            &mut self,
            target: BridgePageTarget,
            state: &BridgePageState,
        ) -> Result<(), Self::Error> {
            self.record(Event::Clear(target), state);
            Ok(())
        }

        fn build_ship_projection(&mut self, state: &BridgePageState) -> Result<(), Self::Error> {
            self.record(Event::BuildProjection, state);
            Ok(())
        }

        fn project_ship_point_cloud(&mut self, state: &BridgePageState) -> Result<(), Self::Error> {
            self.record(Event::ProjectPointCloud, state);
            Ok(())
        }

        fn project_ship_objects(&mut self, state: &BridgePageState) -> Result<(), Self::Error> {
            self.record(Event::ProjectObjects, state);
            Ok(())
        }

        fn commit_ship_sprites(
            &mut self,
            target: BridgePageTarget,
            state: &BridgePageState,
        ) -> Result<(), Self::Error> {
            self.record(Event::CommitShipSprites(target), state);
            Ok(())
        }

        fn render_ship_sprites(
            &mut self,
            target: BridgePageTarget,
            state: &BridgePageState,
        ) -> Result<(), Self::Error> {
            self.record(Event::RenderShipSprites(target), state);
            Ok(())
        }

        fn load_panorama_frame(
            &mut self,
            target: BridgePageTarget,
            frame: u16,
            state: &BridgePageState,
        ) -> Result<(), Self::Error> {
            self.record(Event::LoadPanorama(target, frame), state);
            Ok(())
        }
    }

    #[test]
    fn page_preparation_matches_every_original_call_and_flag_vector() {
        let vectors: Vec<PageOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_954a_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let ship_active = vector.ship_flags & NATIVE_SHIP_ACTIVE_FLAG != u16::MIN;
            let panorama_frame = vector.bridge_frame.unwrap_or(u16::MIN);
            let mut state = BridgePageState {
                palette_dirty: false,
                transparent_zero: vector.transparent_before != u8::MIN,
                dirty_copy_requested: vector.dirty_copy_before != u8::MIN,
            };
            let mut backend = OracleBackend::default();

            let outcome =
                render_bridge_page(ship_active, panorama_frame, &mut state, &mut backend).unwrap();

            assert_eq!(
                outcome,
                if ship_active {
                    BridgePageOutcome::ShipLayerPrepared
                } else {
                    BridgePageOutcome::PanoramaLoaded {
                        frame: panorama_frame,
                    }
                },
                "{}",
                vector.name
            );
            assert!(state.palette_dirty, "{}", vector.name);
            assert_eq!(
                state.transparent_zero,
                vector.transparent_after != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.dirty_copy_requested,
                vector.dirty_copy_after != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                backend.events,
                expected_events(panorama_frame, ship_active),
                "{}",
                vector.name
            );
            assert_eq!(backend.events.len(), vector.calls.len(), "{}", vector.name);
            assert_eq!(
                vector
                    .calls
                    .iter()
                    .map(|call| call.name.as_str())
                    .collect::<Vec<_>>(),
                expected_native_calls(ship_active),
                "{}",
                vector.name
            );
            assert!(
                backend
                    .palette_dirty_at_call
                    .iter()
                    .zip(&vector.calls)
                    .all(|(actual, call)| *actual == (call.palette_dirty != u8::MIN)),
                "{}",
                vector.name
            );
        }
    }

    fn expected_events(frame: u16, ship_active: bool) -> Vec<Event> {
        let mut events = vec![
            Event::Clear(BridgePageTarget::Secondary),
            Event::BuildProjection,
            Event::ProjectPointCloud,
            Event::ProjectObjects,
            Event::CommitShipSprites(BridgePageTarget::Secondary),
            Event::RenderShipSprites(BridgePageTarget::Secondary),
        ];
        if !ship_active {
            events.push(Event::LoadPanorama(BridgePageTarget::Primary, frame));
        }
        events
    }

    fn expected_native_calls(ship_active: bool) -> Vec<&'static str> {
        let mut calls = vec![
            "blit_fill_row_5221",
            "ship_3d_projection_matrix_build",
            "ship_3d_point_cloud_project",
            "ship_3d_object_sprite_project",
            "sprite_slot_commit_dirty_range",
            "sprite_slot_dirty_range_render",
        ];
        if !ship_active {
            calls.push("bridge_panorama_frame_load");
        }
        calls
    }
}
