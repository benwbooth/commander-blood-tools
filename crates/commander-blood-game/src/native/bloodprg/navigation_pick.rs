//! Typed navigation-chart marker hit testing.

const MARKER_ORIGIN_BIAS: u16 = 2;
const DEFAULT_MARKER_EXTENT: [u16; 2] = [12, 11];
const BLACK_HOLE_MARKER_EXTENT: [u16; 2] = [19, 12];
const SHIP_MARKER_EXTENT: [u16; 2] = [21, 10];

/// Marker endpoint selected for one navigation-chart object.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavigationChartMarkerEndpoint {
    /// Primary marker used by ordinary objects and near-side black holes.
    #[default]
    Near,
    /// Secondary marker used when black-hole endpoint context differs.
    Far,
}

/// One decoded navigation-chart object with stable identity and marker data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationChartPickObject<RecordId, EndpointContext> {
    /// Stable object identity replacing its native record offset.
    pub record: RecordId,
    /// Whether the object's kind includes the ship-navigation bit.
    pub is_ship: bool,
    /// Whether the object's kind includes the black-hole bit.
    pub is_black_hole: bool,
    /// Endpoint relation used to select a black hole's marker.
    pub endpoint_context: EndpointContext,
    /// Primary chart marker.
    pub near_marker: [u16; 2],
    /// Secondary chart marker.
    pub far_marker: [u16; 2],
}

/// Scratch state retained from the last object considered by hit testing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationChartPickState {
    /// Inclusive marker width and height selected for the last object.
    pub marker_extent: [u16; 2],
    /// Endpoint selected for the last object.
    pub marker_endpoint: NavigationChartMarkerEndpoint,
}

/// Result of one navigation-chart pointer query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationChartPickOutcome<RecordId> {
    /// No object marker contains the pointer.
    None,
    /// First object whose selected marker contains the pointer.
    Picked {
        /// Stable identity of the selected object.
        record: RecordId,
        /// Marker endpoint used for the successful hit test.
        endpoint: NavigationChartMarkerEndpoint,
    },
}

/// Pick the first navigation-chart object containing the logical pointer.
///
/// This translates native BLOODPRG routine `0x0092A3`. Owned object records
/// replace the record segment and stack-owned offset list. Wrapping origin and
/// edge arithmetic remains explicit because it affects hit testing near the
/// unsigned coordinate boundary.
pub fn pick_navigation_chart_object<RecordId: Clone, EndpointContext: Eq>(
    objects: &[NavigationChartPickObject<RecordId, EndpointContext>],
    arche_endpoint_context: &EndpointContext,
    pointer: [u16; 2],
    state: &mut NavigationChartPickState,
) -> NavigationChartPickOutcome<RecordId> {
    for object in objects {
        let (marker, endpoint, extent) = marker_for_object(object, arche_endpoint_context);
        state.marker_extent = extent;
        state.marker_endpoint = endpoint;

        let origin = [
            marker[0].wrapping_sub(MARKER_ORIGIN_BIAS),
            marker[1].wrapping_sub(MARKER_ORIGIN_BIAS),
        ];
        if coordinate_inside(pointer[0], origin[0], extent[0])
            && coordinate_inside(pointer[1], origin[1], extent[1])
        {
            return NavigationChartPickOutcome::Picked {
                record: object.record.clone(),
                endpoint,
            };
        }
    }

    NavigationChartPickOutcome::None
}

fn marker_for_object<'a, RecordId, EndpointContext: Eq>(
    object: &'a NavigationChartPickObject<RecordId, EndpointContext>,
    arche_endpoint_context: &EndpointContext,
) -> (&'a [u16; 2], NavigationChartMarkerEndpoint, [u16; 2]) {
    if object.is_black_hole {
        if &object.endpoint_context != arche_endpoint_context {
            return (
                &object.far_marker,
                NavigationChartMarkerEndpoint::Far,
                BLACK_HOLE_MARKER_EXTENT,
            );
        }
        if object.is_ship {
            return (
                &object.near_marker,
                NavigationChartMarkerEndpoint::Near,
                SHIP_MARKER_EXTENT,
            );
        }
        return (
            &object.near_marker,
            NavigationChartMarkerEndpoint::Near,
            BLACK_HOLE_MARKER_EXTENT,
        );
    }

    let extent = if object.is_ship {
        SHIP_MARKER_EXTENT
    } else {
        DEFAULT_MARKER_EXTENT
    };
    (
        &object.near_marker,
        NavigationChartMarkerEndpoint::Near,
        extent,
    )
}

const fn coordinate_inside(point: u16, origin: u16, extent: u16) -> bool {
    point >= origin && point <= origin.wrapping_add(extent)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const DEFAULT_ARCH_CONTEXT: u16 = 30_583;
    const DEFAULT_FAR_MARKER: [u16; 2] = [51_966, 47_806];

    #[derive(Deserialize)]
    struct PickVector {
        name: String,
        mouse: [u16; 2],
        object_offsets: Vec<u16>,
        arche_context: u16,
        result: u16,
        terminal_endpoint: u8,
        scratch_before: [u16; 2],
        scratch_after: [u16; 2],
    }

    #[test]
    fn chart_pick_matches_every_original_vector() {
        let vectors: Vec<PickVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_92a3_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 13);

        for vector in vectors {
            let objects = objects_for_vector(&vector);
            assert_eq!(
                objects
                    .iter()
                    .map(|object| object.record)
                    .collect::<Vec<_>>(),
                vector.object_offsets,
                "{}",
                vector.name
            );
            let mut state = NavigationChartPickState {
                marker_extent: vector.scratch_before,
                marker_endpoint: NavigationChartMarkerEndpoint::Near,
            };
            let outcome = pick_navigation_chart_object(
                &objects,
                &vector.arche_context,
                vector.mouse,
                &mut state,
            );

            assert_eq!(state.marker_extent, vector.scratch_after, "{}", vector.name);
            let expected_endpoint = if vector.terminal_endpoint == u8::MIN {
                NavigationChartMarkerEndpoint::Near
            } else {
                NavigationChartMarkerEndpoint::Far
            };
            assert_eq!(state.marker_endpoint, expected_endpoint, "{}", vector.name);
            let expected = if vector.result == u16::MIN {
                NavigationChartPickOutcome::None
            } else {
                NavigationChartPickOutcome::Picked {
                    record: vector.result,
                    endpoint: expected_endpoint,
                }
            };
            assert_eq!(outcome, expected, "{}", vector.name);
        }
    }

    fn objects_for_vector(vector: &PickVector) -> Vec<NavigationChartPickObject<u16, u16>> {
        match vector.name.as_str() {
            "empty_list_preserves_scratch" => Vec::new(),
            "default_lower_edges_inclusive" => vec![object(
                4_096,
                false,
                false,
                DEFAULT_ARCH_CONTEXT,
                [50, 60],
                DEFAULT_FAR_MARKER,
            )],
            "default_upper_edges_inclusive" => vec![object(
                4_352,
                false,
                false,
                DEFAULT_ARCH_CONTEXT,
                [50, 60],
                DEFAULT_FAR_MARKER,
            )],
            "first_miss_second_hit" => vec![
                object(
                    4_608,
                    false,
                    false,
                    DEFAULT_ARCH_CONTEXT,
                    [20, 20],
                    DEFAULT_FAR_MARKER,
                ),
                object(
                    4_864,
                    false,
                    false,
                    DEFAULT_ARCH_CONTEXT,
                    [100, 100],
                    DEFAULT_FAR_MARKER,
                ),
            ],
            "first_overlapping_hit_wins" => vec![
                object(
                    5_120,
                    false,
                    false,
                    DEFAULT_ARCH_CONTEXT,
                    [80, 80],
                    DEFAULT_FAR_MARKER,
                ),
                object(
                    5_376,
                    false,
                    false,
                    DEFAULT_ARCH_CONTEXT,
                    [80, 80],
                    DEFAULT_FAR_MARKER,
                ),
            ],
            "ship_wide_far_edge" | "ship_short_y_rejects" => vec![object(
                vector.object_offsets[0],
                true,
                false,
                DEFAULT_ARCH_CONTEXT,
                [100, 60],
                DEFAULT_FAR_MARKER,
            )],
            "black_hole_near_endpoint" => vec![object(6_144, false, true, 7, [150, 60], [250, 90])],
            "black_hole_far_endpoint" => vec![object(6_400, false, true, 7, [150, 60], [250, 90])],
            "both_bits_near_uses_ship_box" => {
                vec![object(6_656, true, true, 7, [50, 120], [200, 150])]
            }
            "both_bits_far_keeps_black_hole_box" => {
                vec![object(6_912, true, true, 8, [50, 120], [200, 150])]
            }
            "wrapped_origin_rejects_between_wrapped_bounds" => vec![object(
                7_168,
                false,
                false,
                DEFAULT_ARCH_CONTEXT,
                [1, 10],
                DEFAULT_FAR_MARKER,
            )],
            "wrapped_object_fields_and_reverse_df" => {
                vec![object(65_520, false, true, 4_660, [40, 50], [240, 150])]
            }
            _ => panic!("unknown navigation pick vector {}", vector.name),
        }
    }

    fn object(
        record: u16,
        is_ship: bool,
        is_black_hole: bool,
        endpoint_context: u16,
        near_marker: [u16; 2],
        far_marker: [u16; 2],
    ) -> NavigationChartPickObject<u16, u16> {
        NavigationChartPickObject {
            record,
            is_ship,
            is_black_hole,
            endpoint_context,
            near_marker,
            far_marker,
        }
    }
}
