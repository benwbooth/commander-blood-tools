//! Camera-relative projection and face preparation for the shared primary mesh.

use std::fmt;

use commander_blood_formats::alien::{
    AXIS_COUNT, AlienFaceData, AlienPrimaryModelData, AlienVertexData,
};

use super::faces::{AlienFaceBucketMap, finish_buckets, select_model_faces};
use super::{
    AlienFaceBucket, AlienFaceDecision, AlienFaceSelectionError, AlienProjectedVertex,
    AlienScreenCenter,
};

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const PRIMARY_MODEL_INDEX: usize = 0;
const DEPTH_SHIFT: u32 = 8;
const SCREEN_WIDTH: i32 = 320;
const SCREEN_HEIGHT: i32 = 200;
const CLIP_LEFT: u16 = 0x0001;
const CLIP_RIGHT: u16 = 0x0002;
const CLIP_TOP: u16 = 0x0004;
const CLIP_BOTTOM: u16 = 0x0008;
const COMMON_CLIP_INITIAL: u16 = 0x800f;
const ZERO_COMPONENT: i32 = 0;

type Matrix = [[i32; AXIS_COUNT]; AXIS_COUNT];

/// Mutable primary-mesh state retained between alien-scene frames.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienPrimaryMeshPose {
    /// Authored texture, position, initial screen, and raster-depth values.
    pub vertices: Vec<AlienVertexData>,
    /// Current projection parallel to [`Self::vertices`].
    pub projected_vertices: Vec<AlienProjectedVertex>,
    /// Whether each current vertex passed the primary routine's depth tests.
    pub valid_depth: Vec<bool>,
    /// Mutable cyclic vertex ordering for primary faces.
    pub faces: Vec<AlienFaceData>,
    /// Common clipping result from the most recent projection.
    pub common_clip: u16,
}

/// Render-facing result of one primary-mesh projection pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienPrimaryMeshFrame {
    /// Per-face clipping and bucket decisions, empty when the whole mesh is clipped.
    pub face_decisions: Vec<AlienFaceDecision>,
    /// Nonempty face buckets in increasing screen-column order.
    pub buckets: Vec<AlienFaceBucket>,
    /// Whether the original routine continued into its renderer.
    pub render_requested: bool,
}

/// Structural or arithmetic failure while projecting the primary mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienPrimaryProjectionError {
    /// The native routine requires at least one vertex.
    EmptyVertexList,
    /// Signed screen division overflowed for a projected vertex.
    ProjectionDivisionOverflow {
        /// Vertex that could not be projected.
        vertex_index: usize,
    },
    /// A face referred outside the projected vertex list.
    FaceSelection(AlienFaceSelectionError),
}

impl fmt::Display for AlienPrimaryProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien primary projection: {self:?}")
    }
}

impl std::error::Error for AlienPrimaryProjectionError {}

impl From<AlienFaceSelectionError> for AlienPrimaryProjectionError {
    fn from(error: AlienFaceSelectionError) -> Self {
        Self::FaceSelection(error)
    }
}

impl AlienPrimaryMeshPose {
    /// Build primary runtime state from fully decoded flat model data.
    pub fn from_model(model: &AlienPrimaryModelData) -> Self {
        Self {
            vertices: model.mesh.vertices.clone(),
            projected_vertices: model
                .mesh
                .vertices
                .iter()
                .map(|vertex| AlienProjectedVertex {
                    screen: vertex.initial_screen,
                    depth: vertex.raster_depth,
                    clip_flags: COMMON_CLIP_INITIAL,
                })
                .collect(),
            valid_depth: vec![false; model.mesh.vertices.len()],
            faces: model.mesh.faces.clone(),
            common_clip: COMMON_CLIP_INITIAL,
        }
    }

    /// Project the primary mesh and prepare its accepted faces for rendering.
    pub fn project_and_select(
        &mut self,
        camera_matrix: Matrix,
        screen_center: AlienScreenCenter,
    ) -> Result<AlienPrimaryMeshFrame, AlienPrimaryProjectionError> {
        if self.vertices.is_empty() {
            return Err(AlienPrimaryProjectionError::EmptyVertexList);
        }
        self.common_clip = COMMON_CLIP_INITIAL;
        self.valid_depth.fill(false);

        for (vertex_index, (vertex, projected)) in self
            .vertices
            .iter()
            .zip(&mut self.projected_vertices)
            .enumerate()
        {
            projected.clip_flags = COMMON_CLIP_INITIAL;
            let object_position = vertex.position.map(i32::from);
            let depth_accumulator = wrapping_dot(camera_matrix[Z_AXIS], object_position);
            if depth_accumulator < ZERO_COMPONENT {
                continue;
            }
            let depth = depth_accumulator >> DEPTH_SHIFT;
            if depth == ZERO_COMPONENT {
                continue;
            }

            let screen_x = wrapping_dot(camera_matrix[X_AXIS], object_position)
                .checked_div(depth)
                .ok_or(AlienPrimaryProjectionError::ProjectionDivisionOverflow { vertex_index })?;
            let screen_y = wrapping_dot(camera_matrix[Y_AXIS], object_position)
                .checked_div(depth)
                .ok_or(AlienPrimaryProjectionError::ProjectionDivisionOverflow { vertex_index })?;
            let screen_x = screen_x.wrapping_add(screen_center.x);
            let screen_y = screen_y.wrapping_neg().wrapping_add(screen_center.y);
            let mut clip_flags = u16::MIN;
            if screen_x < ZERO_COMPONENT {
                clip_flags = CLIP_LEFT;
            }
            if screen_x >= SCREEN_WIDTH {
                clip_flags = CLIP_RIGHT;
            }
            if screen_y < ZERO_COMPONENT {
                clip_flags |= CLIP_TOP;
            }
            if screen_y >= SCREEN_HEIGHT {
                clip_flags |= CLIP_BOTTOM;
            }

            self.common_clip &= clip_flags;
            projected.screen = [screen_x as i16, screen_y as i16];
            projected.clip_flags = clip_flags;
            self.valid_depth[vertex_index] = true;
        }

        if self.common_clip != u16::MIN {
            return Ok(AlienPrimaryMeshFrame {
                face_decisions: Vec::new(),
                buckets: Vec::new(),
                render_requested: false,
            });
        }

        let mut buckets = AlienFaceBucketMap::new();
        let (face_decisions, _crosses_camera) = select_model_faces(
            PRIMARY_MODEL_INDEX,
            &mut self.faces,
            &self.projected_vertices,
            &mut buckets,
        )?;
        Ok(AlienPrimaryMeshFrame {
            face_decisions,
            buckets: finish_buckets(buckets),
            render_requested: true,
        })
    }
}

fn wrapping_dot(left: [i32; AXIS_COUNT], right: [i32; AXIS_COUNT]) -> i32 {
    left.into_iter()
        .zip(right)
        .fold(u32::MIN, |accumulator, (left, right)| {
            accumulator.wrapping_add((left as u32).wrapping_mul(right as u32))
        }) as i32
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::alien::{
        AlienMeshData, AlienPrimaryModelData, AlienXdbKind, decode_alien_xdb,
    };
    use serde::Deserialize;

    use super::*;

    const ORIGINAL_SCREEN_CENTER: AlienScreenCenter = AlienScreenCenter { x: 160, y: 100 };
    const IDENTITY_MATRIX_COMPONENT: i32 = 32_768;

    #[derive(Deserialize)]
    struct VertexBeforeVector {
        position: [i16; AXIS_COUNT],
        screen: [i16; 2],
        raster_depth: i32,
    }

    #[derive(Deserialize)]
    struct ProjectedVector {
        valid_depth: bool,
        screen: [i16; 2],
        clip_flags: u16,
    }

    #[derive(Deserialize)]
    struct FaceDecisionVector {
        vertices: [usize; AXIS_COUNT],
        left_x: i16,
        bucket_column: Option<usize>,
    }

    #[derive(Deserialize)]
    struct FaceBucketVector {
        column: usize,
        faces: Vec<usize>,
    }

    #[derive(Deserialize)]
    struct PrimaryVector {
        name: String,
        camera_matrix: [i32; AXIS_COUNT * AXIS_COUNT],
        screen_center: [i32; 2],
        vertices_before: Vec<VertexBeforeVector>,
        faces_before: Vec<[usize; AXIS_COUNT]>,
        projected_vertices: Vec<ProjectedVector>,
        face_decisions: Vec<FaceDecisionVector>,
        buckets: Vec<FaceBucketVector>,
        render_requested: bool,
        common_clip: u16,
    }

    fn matrix(flat: [i32; AXIS_COUNT * AXIS_COUNT]) -> Matrix {
        std::array::from_fn(|row| std::array::from_fn(|column| flat[row * AXIS_COUNT + column]))
    }

    fn run_vector(vector: PrimaryVector) {
        let model = AlienPrimaryModelData {
            name: vector.name.clone(),
            mesh: AlienMeshData {
                vertices: vector
                    .vertices_before
                    .iter()
                    .map(|vertex| AlienVertexData {
                        position: vertex.position,
                        initial_screen: vertex.screen,
                        raster_depth: vertex.raster_depth,
                        ..AlienVertexData::default()
                    })
                    .collect(),
                projection_copies: Vec::new(),
                faces: vector
                    .faces_before
                    .iter()
                    .map(|vertices| AlienFaceData {
                        vertices: *vertices,
                    })
                    .collect(),
            },
        };
        let mut pose = AlienPrimaryMeshPose::from_model(&model);
        let frame = pose
            .project_and_select(
                matrix(vector.camera_matrix),
                AlienScreenCenter {
                    x: vector.screen_center[X_AXIS],
                    y: vector.screen_center[Y_AXIS],
                },
            )
            .unwrap();

        for ((actual, valid), expected) in pose
            .projected_vertices
            .iter()
            .zip(&pose.valid_depth)
            .zip(&vector.projected_vertices)
        {
            assert_eq!(*valid, expected.valid_depth, "{}", vector.name);
            assert_eq!(actual.screen, expected.screen, "{}", vector.name);
            assert_eq!(actual.clip_flags, expected.clip_flags, "{}", vector.name);
        }
        assert_eq!(pose.common_clip, vector.common_clip, "{}", vector.name);
        assert_eq!(
            frame.render_requested, vector.render_requested,
            "{}",
            vector.name
        );
        for (actual, expected) in frame.face_decisions.iter().zip(&vector.face_decisions) {
            assert_eq!(actual.vertices, expected.vertices, "{}", vector.name);
            assert_eq!(actual.left_x, expected.left_x, "{}", vector.name);
            assert_eq!(
                actual.bucket_column, expected.bucket_column,
                "{}",
                vector.name
            );
        }
        let buckets: Vec<_> = frame
            .buckets
            .iter()
            .map(|bucket| FaceBucketVector {
                column: bucket.column,
                faces: bucket.faces.iter().map(|face| face.face_index).collect(),
            })
            .collect();
        for (actual, expected) in buckets.iter().zip(&vector.buckets) {
            assert_eq!(actual.column, expected.column, "{}", vector.name);
            assert_eq!(actual.faces, expected.faces, "{}", vector.name);
        }
        assert_eq!(buckets.len(), vector.buckets.len(), "{}", vector.name);
    }

    fn original_xdb(name: &str) -> Option<PathBuf> {
        [
            Path::new("output/_tmp_dat").join(name),
            Path::new("../../output/_tmp_dat").join(name),
        ]
        .into_iter()
        .find(|path| path.is_file())
    }

    #[test]
    fn primary_mesh_matches_every_original_alien_overlay_vector() {
        let suites = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_059b_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_05dc_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_05dc_natural.json"),
        ];
        for json in suites {
            let vectors: Vec<PrimaryVector> = serde_json::from_str(json).unwrap();
            for vector in vectors {
                run_vector(vector);
            }
        }
    }

    #[test]
    fn every_original_primary_mesh_projects_from_decoded_authored_depth() {
        let cases = [
            (AlienXdbKind::Amer, "amer.xdb"),
            (AlienXdbKind::Croolis, "croolis.xdb"),
            (AlienXdbKind::Scrut, "scrut.xdb"),
        ];
        let camera_matrix = std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                if row == column {
                    IDENTITY_MATRIX_COMPONENT
                } else {
                    ZERO_COMPONENT
                }
            })
        });

        for (kind, filename) in cases {
            let Some(path) = original_xdb(filename) else {
                continue;
            };
            let data = std::fs::read(path).unwrap();
            let asset = decode_alien_xdb(&data, kind).unwrap();
            let authored_depth: Vec<_> = asset
                .primary_model
                .mesh
                .vertices
                .iter()
                .map(|vertex| vertex.raster_depth)
                .collect();
            let mut pose = AlienPrimaryMeshPose::from_model(&asset.primary_model);
            pose.project_and_select(camera_matrix, ORIGINAL_SCREEN_CENTER)
                .unwrap();
            assert_eq!(
                pose.projected_vertices
                    .iter()
                    .map(|vertex| vertex.depth)
                    .collect::<Vec<_>>(),
                authored_depth
            );
        }
    }
}
