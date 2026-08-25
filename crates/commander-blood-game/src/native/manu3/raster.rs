//! Triangle preparation recovered from MANU3's software rasterizer.
//!
//! The DOS implementation linked faces and scanline records through offsets in
//! several segments. The modern port retains the game-visible decisions made
//! by that code, then emits owned triangles for wgpu. No native address,
//! segment, selector, or raster-record state crosses this module's boundary.

use std::error::Error;
use std::fmt::{Display, Formatter};

use commander_blood_formats::manu3::MAXIMUM_FACE_SPAN;

use super::geometry::ModelVertex;

const TRIANGLE_VERTEX_COUNT: usize = 3;
const ORIGINAL_SCREEN_WIDTH: usize = 320;
const X_COORDINATE: usize = 0;
const Y_COORDINATE: usize = 1;
const ZERO_DOUBLED_X: i16 = 0;

/// One triangle retained from the original MANU3 face list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelFace {
    /// Three indices into the model's vertex collection.
    pub vertices: [usize; TRIANGLE_VERTEX_COUNT],
}

/// One projected vertex ready for conversion to a GPU vertex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderVertex {
    /// Position in the original 320-by-200 projection coordinate system.
    pub screen: [i16; 2],
    /// Affine texture coordinates in original texture texels.
    pub texture: [i16; 2],
    /// Recovered transformed depth used to establish GPU depth ordering.
    pub depth: i32,
}

/// A visible, front-facing textured triangle in native activation order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderTriangle {
    /// Original face-list index, retained for diagnostics and oracle checks.
    pub source_face: usize,
    /// Cyclically ordered vertices consumed by the renderer.
    pub vertices: [RenderVertex; TRIANGLE_VERTEX_COUNT],
}

/// Invalid flat-memory model topology encountered during face preparation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceError {
    /// Face containing the invalid vertex index.
    pub face: usize,
    /// Vertex index outside the supplied vertex slice.
    pub vertex: usize,
    /// Number of available vertices.
    pub vertex_count: usize,
}

impl Display for FaceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "MANU3 face {} refers to vertex {}, outside {} vertices",
            self.face, self.vertex, self.vertex_count
        )
    }
}

impl Error for FaceError {}

/// Apply MANU3's recovered face sort and activation decisions.
///
/// This is the flat-memory counterpart to `xdb_manu3_face_bucket_sort` at XDB
/// offset `0x0700` and `xdb_manu3_face_activate` at `0x0d7d`. The cyclic face
/// rotations are persistent, matching the native face records. DOS scanline
/// allocation and VGA writes are intentionally replaced by the returned GPU
/// triangles.
pub fn prepare_render_triangles(
    vertices: &[ModelVertex],
    faces: &mut [ModelFace],
    reciprocals: &[i32; MAXIMUM_FACE_SPAN],
) -> Result<Vec<RenderTriangle>, FaceError> {
    let buckets = sort_faces_into_buckets(vertices, faces)?;
    let mut triangles = Vec::with_capacity(faces.len());
    for bucket in buckets {
        // Native faces are prepended to each bucket's linked list.
        for face_index in bucket.into_iter().rev() {
            let face = faces[face_index];
            if face_is_active(face, vertices, reciprocals) {
                triangles.push(RenderTriangle {
                    source_face: face_index,
                    vertices: face.vertices.map(|index| {
                        let vertex = vertices[index];
                        RenderVertex {
                            screen: vertex.projected.screen,
                            texture: vertex.texture,
                            depth: vertex.projected.depth,
                        }
                    }),
                });
            }
        }
    }
    Ok(triangles)
}

fn sort_faces_into_buckets(
    vertices: &[ModelVertex],
    faces: &mut [ModelFace],
) -> Result<Vec<Vec<usize>>, FaceError> {
    let mut buckets = vec![Vec::new(); ORIGINAL_SCREEN_WIDTH];

    for (face_index, face) in faces.iter_mut().enumerate() {
        validate_face(face_index, *face, vertices.len())?;
        if common_clip_flags(*face, vertices) != u16::MIN {
            continue;
        }

        rotate_lowest_x_first(face, vertices);
        let [first, second, third] = face.vertices.map(|index| vertices[index]);
        let x_0 = first.projected.screen[X_COORDINATE];
        let x_1 = second.projected.screen[X_COORDINATE];
        let x_2 = third.projected.screen[X_COORDINATE];
        let width_1 = word_difference(x_1, x_0);
        let width_2 = word_difference(x_2, x_0);
        if usize::from(width_1) >= MAXIMUM_FACE_SPAN || usize::from(width_2) >= MAXIMUM_FACE_SPAN {
            continue;
        }

        let doubled_x = (x_0 as u16).wrapping_mul(2);
        let bucket = if (doubled_x as i16) < ZERO_DOUBLED_X {
            usize::MIN
        } else {
            usize::from(doubled_x) / size_of::<u16>()
        };
        if let Some(bucket) = buckets.get_mut(bucket) {
            bucket.push(face_index);
        }
    }

    Ok(buckets)
}

fn validate_face(face_index: usize, face: ModelFace, vertex_count: usize) -> Result<(), FaceError> {
    for vertex in face.vertices {
        if vertex >= vertex_count {
            return Err(FaceError {
                face: face_index,
                vertex,
                vertex_count,
            });
        }
    }
    Ok(())
}

fn common_clip_flags(face: ModelFace, vertices: &[ModelVertex]) -> u16 {
    face.vertices
        .map(|index| vertices[index].projected.clip_flags.bits())
        .into_iter()
        .reduce(|common, flags| common & flags)
        .unwrap_or(u16::MIN)
}

fn rotate_lowest_x_first(face: &mut ModelFace, vertices: &[ModelVertex]) {
    let [vertex_0, vertex_1, vertex_2] = face.vertices;
    let x_0 = vertices[vertex_0].projected.screen[X_COORDINATE];
    let x_1 = vertices[vertex_1].projected.screen[X_COORDINATE];
    let x_2 = vertices[vertex_2].projected.screen[X_COORDINATE];

    if x_1 > x_2 {
        if x_0 >= x_2 {
            face.vertices = [vertex_2, vertex_0, vertex_1];
        }
    } else if x_0 > x_1 {
        face.vertices = [vertex_1, vertex_2, vertex_0];
    }
}

fn face_is_active(
    face: ModelFace,
    vertices: &[ModelVertex],
    reciprocals: &[i32; MAXIMUM_FACE_SPAN],
) -> bool {
    let [vertex_0, vertex_1, vertex_2] = face.vertices.map(|index| vertices[index]);
    let x_0 = vertex_0.projected.screen[X_COORDINATE];
    let x_1 = vertex_1.projected.screen[X_COORDINATE];
    let x_2 = vertex_2.projected.screen[X_COORDINATE];
    let width_1 = word_difference(x_1, x_0);
    let width_2 = word_difference(x_2, x_0);

    if width_1 == u16::MIN {
        if width_2 == u16::MIN || usize::from(width_2) >= MAXIMUM_FACE_SPAN {
            return false;
        }
        let y_0 = vertex_0.projected.screen[Y_COORDINATE];
        let y_1 = vertex_1.projected.screen[Y_COORDINATE];
        let vertical_span = word_difference(y_1, y_0);
        return (vertical_span as i16).is_positive()
            && usize::from(vertical_span) < MAXIMUM_FACE_SPAN;
    }
    if width_2 == u16::MIN {
        return false;
    }

    let y_0 = vertex_0.projected.screen[Y_COORDINATE];
    let y_1 = vertex_1.projected.screen[Y_COORDINATE];
    let y_2 = vertex_2.projected.screen[Y_COORDINATE];
    let edge_1_step =
        i32::from(y_1.wrapping_sub(y_0)).wrapping_mul(reciprocals[usize::from(width_1)]);
    let edge_0_step =
        i32::from(y_2.wrapping_sub(y_0)).wrapping_mul(reciprocals[usize::from(width_2)]);
    edge_0_step.wrapping_sub(edge_1_step).is_negative()
}

const fn word_difference(left: i16, right: i16) -> u16 {
    (left as u16).wrapping_sub(right as u16)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::Deserialize;

    use super::*;
    use crate::native::manu3::geometry::{ClipFlags, ProjectedVertex};

    const FACE_SPAN_RECIPROCAL_SCALE: i32 = 65_536;
    const FIRST_VERTEX: usize = 0;
    const SECOND_VERTEX: usize = 1;
    const THIRD_VERTEX: usize = 2;
    const NATIVE_FACE_LIST_OFFSET: u16 = 0x1000;
    const NATIVE_FACE_RECORD_SIZE: u16 = 8;
    const NATIVE_VERTEX_LIST_OFFSET: u16 = 0x3000;
    const NATIVE_FACE_VERTEX_AREA_SIZE: u16 = 0x0080;
    const NATIVE_VERTEX_RECORD_SPACING: u16 = 0x0020;

    #[derive(Deserialize)]
    struct GradientVector {
        name: String,
        screen: [[i16; 2]; TRIANGLE_VERTEX_COUNT],
        accepted: bool,
    }

    #[derive(Deserialize)]
    struct BucketVector {
        name: String,
        faces_before: Vec<BucketFaceBefore>,
        #[serde(default)]
        faces_after: Vec<BucketFaceAfter>,
        #[serde(default)]
        bucket_heads_after: BTreeMap<String, u16>,
    }

    #[derive(Deserialize)]
    struct BucketFaceBefore {
        vertices: [BucketVertex; TRIANGLE_VERTEX_COUNT],
    }

    #[derive(Clone, Copy, Deserialize)]
    struct BucketVertex {
        screen_x: i16,
        clip_flags: u16,
    }

    #[derive(Deserialize)]
    struct BucketFaceAfter {
        link: u16,
        vertices: [u16; TRIANGLE_VERTEX_COUNT],
    }

    fn reciprocal_table() -> [i32; MAXIMUM_FACE_SPAN] {
        std::array::from_fn(|span| {
            if span == usize::MIN {
                0
            } else {
                let span = i32::try_from(span).unwrap();
                (FACE_SPAN_RECIPROCAL_SCALE + span / 2) / span
            }
        })
    }

    fn vertices(screen: [[i16; 2]; TRIANGLE_VERTEX_COUNT]) -> Vec<ModelVertex> {
        screen
            .into_iter()
            .map(|screen| ModelVertex {
                projected: ProjectedVertex {
                    screen,
                    clip_flags: ClipFlags::NONE,
                    ..ProjectedVertex::default()
                },
                ..ModelVertex::default()
            })
            .collect()
    }

    #[test]
    fn fixed_point_activation_matches_original_gradient_oracle() {
        let vectors: Vec<GradientVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_manu3_func_0d7d_gradient_natural.json"
        ))
        .unwrap();
        let reciprocals = reciprocal_table();

        for vector in vectors {
            if vector.name == "inactive" {
                // The DOS case has an exhausted raster-record free list. wgpu
                // needs no equivalent allocation state.
                continue;
            }
            let vertices = vertices(vector.screen);
            let mut faces = [ModelFace {
                vertices: [FIRST_VERTEX, SECOND_VERTEX, THIRD_VERTEX],
            }];
            let triangles = prepare_render_triangles(&vertices, &mut faces, &reciprocals).unwrap();
            assert_eq!(!triangles.is_empty(), vector.accepted, "{}", vector.name);
        }
    }

    #[test]
    fn bucket_sort_matches_every_original_binary_case() {
        let vectors: Vec<BucketVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_manu3_func_0700_natural.json"
        ))
        .unwrap();

        for vector in vectors {
            let vertices = vector
                .faces_before
                .iter()
                .flat_map(|face| face.vertices)
                .map(|vertex| ModelVertex {
                    projected: ProjectedVertex {
                        screen: [vertex.screen_x, i16::MIN],
                        clip_flags: ClipFlags::from_bits(vertex.clip_flags),
                        ..ProjectedVertex::default()
                    },
                    ..ModelVertex::default()
                })
                .collect::<Vec<_>>();
            let mut faces = (usize::MIN..vector.faces_before.len())
                .map(|face| ModelFace {
                    vertices: std::array::from_fn(|corner| face * TRIANGLE_VERTEX_COUNT + corner),
                })
                .collect::<Vec<_>>();

            let buckets = sort_faces_into_buckets(&vertices, &mut faces).unwrap();

            for (face_index, (face, expected)) in faces.iter().zip(&vector.faces_after).enumerate()
            {
                let native_vertex_base = NATIVE_VERTEX_LIST_OFFSET
                    + u16::try_from(face_index).unwrap() * NATIVE_FACE_VERTEX_AREA_SIZE;
                let expected_vertices = expected.vertices.map(|offset| {
                    face_index * TRIANGLE_VERTEX_COUNT
                        + usize::from((offset - native_vertex_base) / NATIVE_VERTEX_RECORD_SPACING)
                });
                assert_eq!(face.vertices, expected_vertices, "{}", vector.name);
            }

            let actual_order = buckets
                .into_iter()
                .flat_map(|bucket| bucket.into_iter().rev())
                .collect::<Vec<_>>();
            let mut expected_order = Vec::new();
            for head in vector.bucket_heads_after.values() {
                let mut offset = *head;
                while let Some(face_index) = native_face_index(offset, vector.faces_after.len()) {
                    expected_order.push(face_index);
                    offset = vector.faces_after[face_index].link;
                }
            }
            assert_eq!(actual_order, expected_order, "{}", vector.name);
        }
    }

    #[test]
    fn cyclic_rotation_and_flat_empty_input_match_native_intent() {
        let vertices = vertices([[25, 20], [5, 100], [20, 30]]);
        let mut faces = [ModelFace {
            vertices: [FIRST_VERTEX, SECOND_VERTEX, THIRD_VERTEX],
        }];
        prepare_render_triangles(&vertices, &mut faces, &reciprocal_table()).unwrap();
        assert_eq!(
            faces[usize::MIN].vertices,
            [SECOND_VERTEX, THIRD_VERTEX, FIRST_VERTEX]
        );

        let mut empty_faces = [];
        let triangles =
            prepare_render_triangles(&[], &mut empty_faces, &reciprocal_table()).unwrap();
        assert!(triangles.is_empty());
    }

    fn native_face_index(offset: u16, face_count: usize) -> Option<usize> {
        let relative = offset.checked_sub(NATIVE_FACE_LIST_OFFSET)?;
        if relative % NATIVE_FACE_RECORD_SIZE != u16::MIN {
            return None;
        }
        let index = usize::from(relative / NATIVE_FACE_RECORD_SIZE);
        (index < face_count).then_some(index)
    }
}
