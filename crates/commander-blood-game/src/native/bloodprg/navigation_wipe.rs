//! Typed span generation for the navigation chart's center wipe.

use std::fmt;

/// Horizontal center of the original navigation-chart viewport.
pub const NAVIGATION_WIPE_CENTER_X: u16 = 160;
/// Vertical center of the original navigation-chart viewport.
pub const NAVIGATION_WIPE_CENTER_Y: u16 = 110;

const LOGICAL_DISPLAY_HEIGHT: u16 = 200;
const SYMMETRIC_SPAN_WIDTH_MULTIPLIER: i32 = 2;

/// One symmetric horizontal span emitted for the navigation wipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationWipeSpan {
    /// Left edge of the span.
    pub left: u16,
    /// Number of pixels extending symmetrically toward the right edge.
    pub width: u16,
}

/// Endpoint that cannot describe a bounded center wipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationWipeEndpointError {
    /// Supplied endpoint.
    pub endpoint: [u16; 2],
}

impl fmt::Display for NavigationWipeEndpointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "navigation wipe endpoint {:?} is outside its domain",
            self.endpoint
        )
    }
}

impl std::error::Error for NavigationWipeEndpointError {}

/// Build the navigation chart's symmetric center-wipe spans.
///
/// This translates `nav_center_wipe_span_table_build` at BLOODPRG routine
/// offset `0x009364`. The output is an owned vector rather than a sentinel-
/// terminated table in a graphics segment. Endpoints that made the DOS code
/// underflow widths or walk 65,536 wrapped entries are rejected or normalized
/// to an empty geometric result.
pub fn build_navigation_wipe_spans(
    endpoint: [u16; 2],
) -> Result<Box<[NavigationWipeSpan]>, NavigationWipeEndpointError> {
    if endpoint[0] > NAVIGATION_WIPE_CENTER_X || endpoint[1] >= LOGICAL_DISPLAY_HEIGHT {
        return Err(NavigationWipeEndpointError { endpoint });
    }

    let center = [
        i32::from(NAVIGATION_WIPE_CENTER_X),
        i32::from(NAVIGATION_WIPE_CENTER_Y),
    ];
    let endpoint = [i32::from(endpoint[0]), i32::from(endpoint[1])];
    let (mut start, end) = if endpoint[1] >= center[1] {
        (center, endpoint)
    } else {
        (endpoint, center)
    };
    let horizontal_delta = (end[0] - start[0]).abs();
    let vertical_delta = end[1] - start[1];
    if vertical_delta == 0 {
        return Ok(Vec::new().into_boxed_slice());
    }

    let x_step = if end[0] >= start[0] { 1 } else { -1 };
    let doubled_horizontal = horizontal_delta * SYMMETRIC_SPAN_WIDTH_MULTIPLIER;
    let doubled_vertical = vertical_delta * SYMMETRIC_SPAN_WIDTH_MULTIPLIER;
    let mut spans = Vec::with_capacity(vertical_delta as usize);

    if vertical_delta >= horizontal_delta {
        let mut error = doubled_horizontal - vertical_delta;
        for _ in 0..vertical_delta {
            spans.push(span_at(start[0]));
            if error >= 0 {
                start[0] += x_step;
                error -= doubled_vertical;
            }
            error += doubled_horizontal;
        }
    } else {
        let mut error = doubled_vertical - horizontal_delta;
        for _ in 0..horizontal_delta {
            start[0] += x_step;
            if error >= 0 {
                spans.push(span_at(start[0]));
                error -= doubled_horizontal;
            }
            error += doubled_vertical;
        }
    }

    Ok(spans.into_boxed_slice())
}

fn span_at(left: i32) -> NavigationWipeSpan {
    NavigationWipeSpan {
        left: left as u16,
        width: ((i32::from(NAVIGATION_WIPE_CENTER_X) - left) * SYMMETRIC_SPAN_WIDTH_MULTIPLIER)
            as u16,
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 20;

    #[derive(Deserialize)]
    struct WipeOracle {
        name: String,
        loaded_point: [u16; 2],
        span_count: usize,
        span_bytes_sha256: String,
    }

    #[test]
    fn valid_geometry_matches_original_spans_and_wrapping_geometry_is_rejected() {
        let vectors: Vec<WipeOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_9364_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let result = build_navigation_wipe_spans(vector.loaded_point);
            if vector.loaded_point[0] > NAVIGATION_WIPE_CENTER_X {
                assert_eq!(
                    result,
                    Err(NavigationWipeEndpointError {
                        endpoint: vector.loaded_point,
                    }),
                    "{}",
                    vector.name
                );
                continue;
            }

            let spans = result.unwrap_or_else(|error| panic!("{}: {error}", vector.name));
            let expected_count =
                if vector.loaded_point == [NAVIGATION_WIPE_CENTER_X, NAVIGATION_WIPE_CENTER_Y] {
                    0
                } else {
                    vector.span_count
                };
            assert_eq!(spans.len(), expected_count, "{}", vector.name);
            if expected_count == vector.span_count {
                let bytes: Vec<u8> = spans
                    .iter()
                    .flat_map(|span| {
                        span.left
                            .to_le_bytes()
                            .into_iter()
                            .chain(span.width.to_le_bytes())
                    })
                    .collect();
                assert_eq!(
                    format!("{:x}", Sha256::digest(bytes)),
                    vector.span_bytes_sha256,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn rows_below_the_logical_display_are_rejected() {
        let endpoint = [0, LOGICAL_DISPLAY_HEIGHT];
        assert_eq!(
            build_navigation_wipe_spans(endpoint),
            Err(NavigationWipeEndpointError { endpoint })
        );
    }
}
