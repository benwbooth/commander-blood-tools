//! Face rejection, cyclic ordering, and screen-column bucketing for alien scenes.

use std::collections::BTreeMap;
use std::fmt;

use commander_blood_formats::alien::{AXIS_COUNT, AlienFaceData};

use super::{AlienModelPose, AlienProjectedVertex, AlienSpecies};

const FIRST_VERTEX: usize = 0;
const SECOND_VERTEX: usize = 1;
const THIRD_VERTEX: usize = 2;
const FACE_VERTEX_COUNT: usize = AXIS_COUNT;
const MAXIMUM_FACE_WIDTH: u16 = 500;
const BUCKET_SCALE_SHIFT: u32 = 1;
const BEHIND_CAMERA_CLIP: u16 = 0x8000;
const FIRST_BUCKET_COLUMN: usize = 0;
const ZERO_SCREEN_COORDINATE: i16 = 0;

pub(super) type AlienFaceBucketMap = BTreeMap<usize, Vec<AlienFaceReference>>;

/// Typed replacement for the original pointer-valued behind-camera latch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienBehindCameraSignal {
    /// No accepted face requested a latch update, so existing game state is retained.
    Unchanged,
    /// AMER observed an accepted face crossing the camera plane.
    General,
    /// CROOLIS or SCRUT selected the final model crossing the camera plane.
    Model(usize),
}

/// Stable identity of one face in the flat scene model array.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienFaceReference {
    /// Model/context index.
    pub model_index: usize,
    /// Face index within that model.
    pub face_index: usize,
}

/// Recovered selection decision for one face, including rejected faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienFaceDecision {
    /// Cyclic vertex order after choosing the leftmost first vertex.
    pub vertices: [usize; FACE_VERTEX_COUNT],
    /// Signed horizontal coordinate of the chosen first vertex.
    pub left_x: i16,
    /// Screen-column bucket, or `None` when clipping or width rejected the face.
    pub bucket_column: Option<usize>,
}

/// Faces linked into one screen-column bucket in original LIFO order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienFaceBucket {
    /// Horizontal screen column owning this bucket.
    pub column: usize,
    /// Most recently encountered face first, matching the native linked list.
    pub faces: Vec<AlienFaceReference>,
}

/// Complete output of one shared alien face-selection pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienFaceSelection {
    /// Per-model decisions parallel to each model's face array.
    pub decisions: Vec<Vec<AlienFaceDecision>>,
    /// Nonempty buckets ordered by increasing screen column.
    pub buckets: Vec<AlienFaceBucket>,
    /// Semantic camera-plane signal published by this pass.
    pub behind_camera: AlienBehindCameraSignal,
}

/// Invalid typed model state encountered while selecting faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienFaceSelectionError {
    /// The native routine requires at least one model context.
    EmptyModelList,
    /// A face referred outside its model's projected vertex array.
    InvalidVertex {
        /// Model containing the face.
        model_index: usize,
        /// Face containing the invalid index.
        face_index: usize,
        /// Invalid projected vertex index.
        vertex_index: usize,
        /// Number of available projected vertices.
        available: usize,
    },
}

impl fmt::Display for AlienFaceSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien face-selection state: {self:?}")
    }
}

impl std::error::Error for AlienFaceSelectionError {}

/// Select and bucket projected model faces using the shared recovered routine.
///
/// This mutates only each face's cyclic vertex order. Raster linked-list
/// addresses are replaced by typed face references grouped by screen column.
pub fn select_faces(
    species: AlienSpecies,
    models: &mut [AlienModelPose],
) -> Result<AlienFaceSelection, AlienFaceSelectionError> {
    if models.is_empty() {
        return Err(AlienFaceSelectionError::EmptyModelList);
    }

    let mut decisions = Vec::with_capacity(models.len());
    let mut buckets = AlienFaceBucketMap::new();
    let mut behind_camera = AlienBehindCameraSignal::Unchanged;
    for (model_index, model) in models.iter_mut().enumerate() {
        let (model_decisions, model_crosses_camera) = select_model_faces(
            model_index,
            &mut model.faces,
            &model.projected_vertices,
            &mut buckets,
        )?;
        if species == AlienSpecies::Amer && model_crosses_camera {
            behind_camera = AlienBehindCameraSignal::General;
        }
        if species != AlienSpecies::Amer && model_crosses_camera {
            behind_camera = AlienBehindCameraSignal::Model(model_index);
        }
        decisions.push(model_decisions);
    }

    Ok(AlienFaceSelection {
        decisions,
        buckets: finish_buckets(buckets),
        behind_camera,
    })
}

pub(super) fn select_model_faces(
    model_index: usize,
    faces: &mut [AlienFaceData],
    projected_vertices: &[AlienProjectedVertex],
    buckets: &mut AlienFaceBucketMap,
) -> Result<(Vec<AlienFaceDecision>, bool), AlienFaceSelectionError> {
    let mut decisions = Vec::with_capacity(faces.len());
    let mut crosses_camera = false;
    for (face_index, face) in faces.iter_mut().enumerate() {
        let mut vertices = face.vertices;
        let projected = vertices.map(|vertex_index| {
            projected_vertices.get(vertex_index).copied().ok_or(
                AlienFaceSelectionError::InvalidVertex {
                    model_index,
                    face_index,
                    vertex_index,
                    available: projected_vertices.len(),
                },
            )
        });
        let [vertex_0, vertex_1, vertex_2] = projected;
        let projected = [vertex_0?, vertex_1?, vertex_2?];
        let mut screen_x = projected.map(|vertex| vertex.screen[FIRST_VERTEX]);
        let common_clip = projected
            .iter()
            .fold(u16::MAX, |clip, vertex| clip & vertex.clip_flags);
        let mut bucket_column = None;

        if common_clip == u16::MIN {
            let combined_clip = projected
                .iter()
                .fold(u16::MIN, |clip, vertex| clip | vertex.clip_flags);
            crosses_camera |= combined_clip & BEHIND_CAMERA_CLIP != u16::MIN;

            rotate_leftmost(&mut vertices, &mut screen_x);
            face.vertices = vertices;
            let first_x = screen_x[FIRST_VERTEX] as u16;
            let first_span = (screen_x[SECOND_VERTEX] as u16).wrapping_sub(first_x);
            let second_span = (screen_x[THIRD_VERTEX] as u16).wrapping_sub(first_x);
            if first_span < MAXIMUM_FACE_WIDTH && second_span < MAXIMUM_FACE_WIDTH {
                let doubled_x = first_x.wrapping_shl(BUCKET_SCALE_SHIFT);
                let column = if doubled_x as i16 >= ZERO_SCREEN_COORDINATE {
                    usize::from(doubled_x >> BUCKET_SCALE_SHIFT)
                } else {
                    FIRST_BUCKET_COLUMN
                };
                bucket_column = Some(column);
                buckets.entry(column).or_default().insert(
                    FIRST_VERTEX,
                    AlienFaceReference {
                        model_index,
                        face_index,
                    },
                );
            }
        }

        decisions.push(AlienFaceDecision {
            vertices,
            left_x: screen_x[FIRST_VERTEX],
            bucket_column,
        });
    }
    Ok((decisions, crosses_camera))
}

pub(super) fn finish_buckets(buckets: AlienFaceBucketMap) -> Vec<AlienFaceBucket> {
    buckets
        .into_iter()
        .map(|(column, faces)| AlienFaceBucket { column, faces })
        .collect()
}

fn rotate_leftmost(
    vertices: &mut [usize; FACE_VERTEX_COUNT],
    screen_x: &mut [i16; FACE_VERTEX_COUNT],
) {
    if screen_x[SECOND_VERTEX] > screen_x[THIRD_VERTEX] {
        if screen_x[FIRST_VERTEX] >= screen_x[THIRD_VERTEX] {
            vertices.rotate_right(SECOND_VERTEX);
            screen_x.rotate_right(SECOND_VERTEX);
        }
    } else if screen_x[FIRST_VERTEX] > screen_x[SECOND_VERTEX] {
        vertices.rotate_left(SECOND_VERTEX);
        screen_x.rotate_left(SECOND_VERTEX);
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::alien::{
        AXIS_COUNT, AlienFaceData, AlienTransformData, AlienXdbKind, decode_alien_xdb,
    };
    use serde::Deserialize;

    use super::*;
    use crate::native::alien::AlienScreenCenter;

    const DEFAULT_SCREEN_Y: i16 = 0;
    const DEFAULT_DEPTH: i32 = 0;
    const ORIGINAL_SCREEN_CENTER: AlienScreenCenter = AlienScreenCenter { x: 160, y: 100 };
    const IDENTITY_MATRIX_COMPONENT: i32 = 32_768;
    const ZERO_MATRIX_COMPONENT: i32 = 0;

    #[derive(Deserialize)]
    struct FaceBeforeVector {
        screen_x: [i16; FACE_VERTEX_COUNT],
        clip_flags: [u16; FACE_VERTEX_COUNT],
    }

    #[derive(Deserialize)]
    struct FaceDecisionVector {
        vertices: [usize; FACE_VERTEX_COUNT],
        left_x: i16,
        bucket_column: Option<usize>,
    }

    #[derive(Deserialize)]
    struct FaceBucketVector {
        column: usize,
        faces: Vec<[usize; 2]>,
    }

    #[derive(Deserialize)]
    struct BehindSignalVector {
        kind: String,
        context: Option<usize>,
    }

    #[derive(Deserialize)]
    struct FaceSelectionVector {
        name: String,
        module: String,
        contexts_before: Vec<Vec<FaceBeforeVector>>,
        decisions: Vec<Vec<FaceDecisionVector>>,
        buckets: Vec<FaceBucketVector>,
        behind_signal: BehindSignalVector,
    }

    fn species(module: &str) -> AlienSpecies {
        match module {
            "amer" => AlienSpecies::Amer,
            "croolis" => AlienSpecies::Croolis,
            "scrut" => AlienSpecies::Scrut,
            _ => panic!("unknown alien module {module}"),
        }
    }

    fn model_pose(faces: &[FaceBeforeVector]) -> AlienModelPose {
        let mut projected_vertices = Vec::with_capacity(faces.len() * FACE_VERTEX_COUNT);
        let mut model_faces = Vec::with_capacity(faces.len());
        for face in faces {
            let first_vertex = projected_vertices.len();
            projected_vertices.extend(face.screen_x.into_iter().zip(face.clip_flags).map(
                |(screen_x, clip_flags)| AlienProjectedVertex {
                    screen: [screen_x, DEFAULT_SCREEN_Y],
                    depth: DEFAULT_DEPTH,
                    clip_flags,
                },
            ));
            model_faces.push(AlienFaceData {
                vertices: std::array::from_fn(|index| first_vertex + index),
            });
        }
        AlienModelPose {
            root: AlienTransformData::default(),
            nodes: Vec::new(),
            texture_coordinates: vec![[i16::MIN; 2]; projected_vertices.len()],
            object_positions: vec![[i16::MIN; AXIS_COUNT]; projected_vertices.len()],
            authored_vertex_count: projected_vertices.len(),
            projected_vertices,
            faces: model_faces,
            last_rotation_matrix: Default::default(),
            last_common_clip: u16::MIN,
        }
    }

    fn expected_signal(vector: &BehindSignalVector) -> AlienBehindCameraSignal {
        match vector.kind.as_str() {
            "unchanged" => AlienBehindCameraSignal::Unchanged,
            "general" => AlienBehindCameraSignal::General,
            "context" => AlienBehindCameraSignal::Model(vector.context.unwrap()),
            kind => panic!("unknown behind-camera signal {kind}"),
        }
    }

    fn run_vector(vector: FaceSelectionVector) {
        let mut models: Vec<_> = vector
            .contexts_before
            .iter()
            .map(|context| model_pose(context))
            .collect();
        let selection = select_faces(species(&vector.module), &mut models).unwrap();

        for (actual_context, expected_context) in selection.decisions.iter().zip(&vector.decisions)
        {
            for (actual, expected) in actual_context.iter().zip(expected_context) {
                assert_eq!(actual.vertices, expected.vertices, "{}", vector.name);
                assert_eq!(actual.left_x, expected.left_x, "{}", vector.name);
                assert_eq!(
                    actual.bucket_column, expected.bucket_column,
                    "{}",
                    vector.name
                );
            }
        }
        let expected_buckets: Vec<_> = vector
            .buckets
            .iter()
            .map(|bucket| AlienFaceBucket {
                column: bucket.column,
                faces: bucket
                    .faces
                    .iter()
                    .map(|face| AlienFaceReference {
                        model_index: face[FIRST_VERTEX],
                        face_index: face[SECOND_VERTEX],
                    })
                    .collect(),
            })
            .collect();
        assert_eq!(selection.buckets, expected_buckets, "{}", vector.name);
        assert_eq!(
            selection.behind_camera,
            expected_signal(&vector.behind_signal),
            "{}",
            vector.name
        );
        for ((model, decisions), expected) in models
            .iter()
            .zip(&selection.decisions)
            .zip(&vector.decisions)
        {
            assert_eq!(model.faces.len(), decisions.len());
            for ((face, decision), expected) in model.faces.iter().zip(decisions).zip(expected) {
                assert_eq!(face.vertices, decision.vertices, "{}", vector.name);
                assert_eq!(face.vertices, expected.vertices, "{}", vector.name);
            }
        }
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
    fn face_selection_matches_every_original_alien_overlay_vector() {
        let suites = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_24cf_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_2514_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_25d4_natural.json"),
        ];
        for json in suites {
            let vectors: Vec<FaceSelectionVector> = serde_json::from_str(json).unwrap();
            for vector in vectors {
                run_vector(vector);
            }
        }
    }

    #[test]
    fn every_decoded_original_face_selects_from_flat_projected_vertices() {
        let cases = [
            (AlienXdbKind::Amer, AlienSpecies::Amer, "amer.xdb"),
            (AlienXdbKind::Croolis, AlienSpecies::Croolis, "croolis.xdb"),
            (AlienXdbKind::Scrut, AlienSpecies::Scrut, "scrut.xdb"),
        ];
        let scene_camera = AlienTransformData {
            matrix: std::array::from_fn(|row| {
                std::array::from_fn(|column| {
                    if row == column {
                        IDENTITY_MATRIX_COMPONENT
                    } else {
                        ZERO_MATRIX_COMPONENT
                    }
                })
            }),
            translation: [ZERO_MATRIX_COMPONENT; AXIS_COUNT],
        };

        for (kind, species, filename) in cases {
            let Some(path) = original_xdb(filename) else {
                continue;
            };
            let data = std::fs::read(path).unwrap();
            let asset = decode_alien_xdb(&data, kind).unwrap();
            let mut poses: Vec<_> = asset
                .models
                .iter()
                .map(AlienModelPose::from_model)
                .collect();
            for (model, pose) in asset.models.iter().zip(&mut poses) {
                pose.transform_and_project(
                    &model.mesh,
                    scene_camera,
                    ORIGINAL_SCREEN_CENTER,
                    &asset.trigonometry,
                )
                .unwrap_or_else(|error| panic!("{} failed projection: {error}", model.name));
            }
            let selection = select_faces(species, &mut poses).unwrap();
            assert_eq!(selection.decisions.len(), asset.models.len());
            for (decisions, model) in selection.decisions.iter().zip(&asset.models) {
                assert_eq!(decisions.len(), model.mesh.faces.len());
            }
        }
    }
}
