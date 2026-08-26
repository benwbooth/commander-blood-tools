//! Location-information panel sprite scaling and placement.

const SCALE_NUMERATOR: u8 = 3;
const SCALE_SHIFT: u32 = 1;
const EXTENT_SHIFT: u32 = 4;
const POSITION_DIVISOR: i16 = 13;
const TARGET_Y_BIAS: u16 = 10;

/// Live panel positions read after the extent update callback.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocationPanelLayout {
    /// Current panel origin in original logical coordinates.
    pub current: [u16; 2],
    /// Final panel origin in original logical coordinates.
    pub target: [u16; 2],
}

/// Mutable geometry state for the location-information panel sprite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocationPanelGeometryState {
    /// Native opening/closing scale step.
    pub scale_step: u8,
    /// Horizontal source-width adjustment calculated from loaded artwork.
    pub source_width: u16,
    /// Current and target panel positions.
    pub layout: LocationPanelLayout,
}

/// Ordered entity operations used by the geometry step.
pub trait LocationPanelGeometryHost<ComparisonExtent> {
    /// Commit the scaled entity extent.
    ///
    /// The callback may update `layout`; the recovered routine rereads both
    /// positions after this call before calculating the entity origin.
    fn update_panel_extent(
        &mut self,
        extent: [u16; 2],
        comparison_extent: &ComparisonExtent,
        source_width: &mut u16,
        layout: &mut LocationPanelLayout,
    );

    /// Commit the resulting panel entity origin.
    fn update_panel_position(&mut self, position: [u16; 2]);
}

/// Geometry emitted by one location-panel scale step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocationPanelGeometry {
    /// Low-byte scale derived from the native step.
    pub scale: u8,
    /// Scaled width and height committed to entity zero.
    pub extent: [u16; 2],
    /// Final wrapped entity origin.
    pub position: [u16; 2],
}

/// Update native location-panel entity geometry routine `0x009240`.
///
/// The modern port retains the original low-byte products, signed 8-bit scale,
/// truncating signed division, and wrapping coordinate arithmetic. Typed
/// layout and extent values replace the native entity table and far pointer.
pub fn update_location_panel_geometry<ComparisonExtent, Host>(
    state: &mut LocationPanelGeometryState,
    source_extent: [u16; 2],
    comparison_extent: &ComparisonExtent,
    host: &mut Host,
) -> LocationPanelGeometry
where
    Host: LocationPanelGeometryHost<ComparisonExtent>,
{
    let scale = state
        .scale_step
        .wrapping_mul(SCALE_NUMERATOR)
        .wrapping_shr(SCALE_SHIFT)
        .wrapping_add(1);
    let extent = [
        u16::from(source_extent[0] as u8)
            .wrapping_mul(u16::from(scale))
            .wrapping_shr(EXTENT_SHIFT),
        u16::from(source_extent[1] as u8)
            .wrapping_mul(u16::from(scale))
            .wrapping_shr(EXTENT_SHIFT),
    ];
    host.update_panel_extent(
        extent,
        comparison_extent,
        &mut state.source_width,
        &mut state.layout,
    );

    let signed_scale = i16::from(scale as i8);
    let delta_x = state.layout.target[0]
        .wrapping_sub(state.source_width)
        .wrapping_sub(state.layout.current[0]) as i16;
    let delta_y = state.layout.target[1]
        .wrapping_add(TARGET_Y_BIAS)
        .wrapping_sub(state.layout.current[1]) as i16;
    let step_x = (delta_x / POSITION_DIVISOR) as i8;
    let step_y = (delta_y / POSITION_DIVISOR) as i8;
    let position = [
        state.layout.current[0].wrapping_add(i16::from(step_x).wrapping_mul(signed_scale) as u16),
        state.layout.current[1].wrapping_add(i16::from(step_y).wrapping_mul(signed_scale) as u16),
    ];
    host.update_panel_position(position);

    LocationPanelGeometry {
        scale,
        extent,
        position,
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct GeometryVector {
        name: String,
        zoom: u8,
        scale: u8,
        source_extent: [u16; 2],
        scaled_extent: [u16; 2],
        source_width: u16,
        target_before: [u16; 2],
        current_before: [u16; 2],
        target_for_position: [u16; 2],
        current_for_position: [u16; 2],
        draw_position: [u16; 2],
    }

    struct OracleHost {
        source_width_after_extent: u16,
        layout_after_extent: LocationPanelLayout,
        extent_call: Option<[u16; 2]>,
        position_call: Option<[u16; 2]>,
    }

    impl LocationPanelGeometryHost<()> for OracleHost {
        fn update_panel_extent(
            &mut self,
            extent: [u16; 2],
            _comparison_extent: &(),
            source_width: &mut u16,
            layout: &mut LocationPanelLayout,
        ) {
            assert!(self.extent_call.replace(extent).is_none());
            *source_width = self.source_width_after_extent;
            *layout = self.layout_after_extent;
        }

        fn update_panel_position(&mut self, position: [u16; 2]) {
            assert!(self.position_call.replace(position).is_none());
        }
    }

    #[test]
    fn geometry_matches_every_original_vector() {
        let vectors: Vec<GeometryVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_9240_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 10);

        for vector in vectors {
            let mut state = LocationPanelGeometryState {
                scale_step: vector.zoom,
                source_width: vector.source_width,
                layout: LocationPanelLayout {
                    current: vector.current_before,
                    target: vector.target_before,
                },
            };
            let mut host = OracleHost {
                source_width_after_extent: if vector.name == "helper_mutation_visible_to_position" {
                    20
                } else {
                    vector.source_width
                },
                layout_after_extent: LocationPanelLayout {
                    current: vector.current_for_position,
                    target: vector.target_for_position,
                },
                extent_call: None,
                position_call: None,
            };

            let geometry =
                update_location_panel_geometry(&mut state, vector.source_extent, &(), &mut host);

            assert_eq!(geometry.scale, vector.scale, "{}", vector.name);
            assert_eq!(geometry.extent, vector.scaled_extent, "{}", vector.name);
            assert_eq!(geometry.position, vector.draw_position, "{}", vector.name);
            assert_eq!(
                host.extent_call,
                Some(vector.scaled_extent),
                "{}",
                vector.name
            );
            assert_eq!(
                host.position_call,
                Some(vector.draw_position),
                "{}",
                vector.name
            );
            assert_eq!(state.layout, host.layout_after_extent, "{}", vector.name);
        }
    }
}
