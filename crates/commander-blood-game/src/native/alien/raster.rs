//! Textured-triangle preparation recovered from the alien software rasterizer.
//!
//! The original renderer turned accepted faces into linked scanline records in
//! a dedicated segment. The modern renderer retains the game-visible cyclic
//! ordering, fixed-point orientation test, UV bank coordinates, and depth, then
//! emits owned fixed-point records for the flat true-color software rasterizer.

use std::fmt;

use commander_blood_formats::alien::{
    AXIS_COUNT, AlienMeshData, AlienModelData, RASTER_RECIPROCAL_COUNT, TEXTURE_HEIGHT,
    TEXTURE_WIDTH,
};

use super::scanline::{
    ALIEN_RASTER_HEIGHT, ALIEN_RASTER_WIDTH, AlienRasterActivation, AlienRasterRecord,
    build_raster_record, rasterize_activations,
};
use super::{
    AlienFaceBucket, AlienFaceReference, AlienFaceSelection, AlienModelPose, AlienPrimaryMeshFrame,
    AlienPrimaryMeshPose, AlienProjectedVertex, AlienStar,
};

const FIRST_VERTEX: usize = 0;
const SECOND_VERTEX: usize = 1;
const THIRD_VERTEX: usize = 2;
const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const TRIANGLE_VERTEX_COUNT: usize = AXIS_COUNT;
const MAXIMUM_FACE_WIDTH: usize = RASTER_RECIPROCAL_COUNT;
const RGB_COMPONENT_COUNT: usize = 3;
const RGBA_COMPONENT_COUNT: usize = 4;
const ALPHA_COMPONENT: u8 = u8::MAX;

/// One projected alien vertex ready for fixed-point record construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienRenderVertex {
    /// Position in the original 320-by-200 projection coordinate system.
    pub screen: [i16; 2],
    /// Unsigned texel coordinate in the decoded 256-by-512 atlas.
    pub texture: [u16; 2],
    /// Recovered fixed-point depth used for surface ordering.
    pub depth: i32,
}

/// One visible textured triangle in recovered bucket and activation order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienRenderTriangle {
    /// Original model and face identity retained for diagnostics.
    pub source: AlienFaceReference,
    /// Cyclically ordered vertices consumed by the renderer.
    pub vertices: [AlienRenderVertex; TRIANGLE_VERTEX_COUNT],
    /// First logical column at which the recovered renderer activates the face.
    pub(crate) first_column: usize,
    /// Exact fixed-point state built by the native face-activation routine.
    pub(crate) record: AlienRasterRecord,
}

/// Render-facing geometry for the two original alien triangle passes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AlienRenderGeometry {
    /// Camera-relative primary mesh rendered before the starfield.
    pub primary_triangles: Vec<AlienRenderTriangle>,
    /// Behavior-model meshes rendered after the starfield, one native call per model.
    pub model_layers: Vec<Vec<AlienRenderTriangle>>,
}

/// One flat true-color frame produced by the recovered software raster rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AlienTrueColorFrame {
    pub(crate) pixels: Vec<u8>,
}

/// Invalid flat scene topology encountered while preparing triangles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienRasterError {
    /// A bucket referred outside the available models.
    InvalidModel {
        /// Invalid model index.
        model_index: usize,
        /// Number of decoded models.
        available: usize,
    },
    /// A bucket referred outside its model's face array.
    InvalidFace {
        /// Model containing the invalid face reference.
        model_index: usize,
        /// Invalid face index.
        face_index: usize,
        /// Number of faces in the model.
        available: usize,
    },
    /// A face referred outside its model's vertex arrays.
    InvalidVertex {
        /// Model containing the invalid vertex reference.
        model_index: usize,
        /// Face containing the invalid vertex reference.
        face_index: usize,
        /// Invalid vertex index.
        vertex_index: usize,
        /// Number of available vertices.
        available: usize,
    },
    /// A recovered texture bank addressed outside the decoded atlas.
    InvalidTextureAddress {
        /// Number of decoded source texels.
        available: usize,
    },
}

impl fmt::Display for AlienRasterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien render geometry: {self:?}")
    }
}

impl std::error::Error for AlienRasterError {}

/// Prepare both alien triangle layers with their exact raster-record state.
pub fn prepare_render_geometry(
    primary_mesh: &AlienMeshData,
    primary_pose: &AlienPrimaryMeshPose,
    primary_frame: &AlienPrimaryMeshFrame,
    models: &[AlienModelData],
    model_poses: &[AlienModelPose],
    model_selection: &AlienFaceSelection,
    reciprocals: &[i32; RASTER_RECIPROCAL_COUNT],
) -> Result<AlienRenderGeometry, AlienRasterError> {
    let primary_triangles = if primary_frame.render_requested {
        prepare_primary_triangles(
            primary_mesh,
            primary_pose,
            &primary_frame.buckets,
            reciprocals,
        )?
    } else {
        Vec::new()
    };
    let model_layers =
        prepare_model_triangles(models, model_poses, &model_selection.buckets, reciprocals)?;
    Ok(AlienRenderGeometry {
        primary_triangles,
        model_layers,
    })
}

/// Rasterize the native primary, starfield, and per-model passes to RGBA.
pub(crate) fn rasterize_true_color_frame(
    geometry: &AlienRenderGeometry,
    stars: &[AlienStar],
    texture: &[u8],
    palette: &[[u8; RGB_COMPONENT_COUNT]; 256],
) -> Result<AlienTrueColorFrame, AlienRasterError> {
    let mut pixels = vec![u8::MIN; ALIEN_RASTER_WIDTH * ALIEN_RASTER_HEIGHT * RGBA_COMPONENT_COUNT];
    for pixel in pixels.chunks_exact_mut(RGBA_COMPONENT_COUNT) {
        pixel[RGB_COMPONENT_COUNT] = ALPHA_COMPONENT;
    }
    rasterize_true_color_layer(&geometry.primary_triangles, texture, palette, &mut pixels)?;
    for star in stars {
        let [x, y] = star.screen;
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            continue;
        };
        if x < ALIEN_RASTER_WIDTH && y < ALIEN_RASTER_HEIGHT {
            write_true_color_pixel(&mut pixels, x, y, palette[usize::from(star.palette_index)]);
        }
    }
    for layer in &geometry.model_layers {
        rasterize_true_color_layer(layer, texture, palette, &mut pixels)?;
    }
    Ok(AlienTrueColorFrame { pixels })
}

fn rasterize_true_color_layer(
    triangles: &[AlienRenderTriangle],
    texture: &[u8],
    palette: &[[u8; RGB_COMPONENT_COUNT]; 256],
    pixels: &mut [u8],
) -> Result<(), AlienRasterError> {
    let activations = triangles
        .iter()
        .map(|triangle| AlienRasterActivation {
            first_column: triangle.first_column,
            record: triangle.record,
        })
        .collect::<Vec<_>>();
    let valid = rasterize_activations(&activations, texture.len(), |x, y, texture_offset| {
        let palette_index = texture[texture_offset];
        write_true_color_pixel(pixels, x, y, palette[usize::from(palette_index)]);
    });
    if !valid {
        return Err(AlienRasterError::InvalidTextureAddress {
            available: texture.len(),
        });
    }
    Ok(())
}

fn write_true_color_pixel(pixels: &mut [u8], x: usize, y: usize, color: [u8; RGB_COMPONENT_COUNT]) {
    let offset = (y * ALIEN_RASTER_WIDTH + x) * RGBA_COMPONENT_COUNT;
    pixels[offset..offset + RGB_COMPONENT_COUNT].copy_from_slice(&color);
    pixels[offset + RGB_COMPONENT_COUNT] = ALPHA_COMPONENT;
}

fn prepare_primary_triangles(
    mesh: &AlienMeshData,
    pose: &AlienPrimaryMeshPose,
    buckets: &[AlienFaceBucket],
    reciprocals: &[i32; RASTER_RECIPROCAL_COUNT],
) -> Result<Vec<AlienRenderTriangle>, AlienRasterError> {
    let mut triangles = Vec::with_capacity(pose.faces.len());
    for (first_column, source) in bucket_faces(buckets) {
        let face = pose
            .faces
            .get(source.face_index)
            .ok_or(AlienRasterError::InvalidFace {
                model_index: source.model_index,
                face_index: source.face_index,
                available: pose.faces.len(),
            })?;
        if face_is_active(face.vertices, &pose.projected_vertices, reciprocals, true) {
            let vertices =
                render_mesh_vertices(source, face.vertices, mesh, &pose.projected_vertices)?;
            triangles.push(AlienRenderTriangle {
                source,
                vertices,
                first_column,
                record: build_raster_record(vertices, reciprocals, true)
                    .expect("the shared native activation predicate accepted the face"),
            });
        }
    }
    Ok(triangles)
}

fn prepare_model_triangles(
    models: &[AlienModelData],
    poses: &[AlienModelPose],
    buckets: &[AlienFaceBucket],
    reciprocals: &[i32; RASTER_RECIPROCAL_COUNT],
) -> Result<Vec<Vec<AlienRenderTriangle>>, AlienRasterError> {
    let mut layers = vec![Vec::new(); models.len()];
    for (first_column, source) in bucket_faces(buckets) {
        models
            .get(source.model_index)
            .ok_or(AlienRasterError::InvalidModel {
                model_index: source.model_index,
                available: models.len(),
            })?;
        let pose = poses
            .get(source.model_index)
            .ok_or(AlienRasterError::InvalidModel {
                model_index: source.model_index,
                available: poses.len(),
            })?;
        let face = pose
            .faces
            .get(source.face_index)
            .ok_or(AlienRasterError::InvalidFace {
                model_index: source.model_index,
                face_index: source.face_index,
                available: pose.faces.len(),
            })?;
        if face_is_active(face.vertices, &pose.projected_vertices, reciprocals, true) {
            let vertices = render_pose_vertices(source, face.vertices, pose)?;
            layers[source.model_index].push(AlienRenderTriangle {
                source,
                vertices,
                first_column,
                record: build_raster_record(vertices, reciprocals, true)
                    .expect("the shared native activation predicate accepted the face"),
            });
        }
    }
    Ok(layers)
}

fn bucket_faces(
    buckets: &[AlienFaceBucket],
) -> impl Iterator<Item = (usize, AlienFaceReference)> + '_ {
    buckets.iter().enumerate().flat_map(|(column, bucket)| {
        bucket
            .faces
            .iter()
            .copied()
            .map(move |source| (column, source))
    })
}

fn render_mesh_vertices(
    source: AlienFaceReference,
    indices: [usize; TRIANGLE_VERTEX_COUNT],
    mesh: &AlienMeshData,
    projected: &[AlienProjectedVertex],
) -> Result<[AlienRenderVertex; TRIANGLE_VERTEX_COUNT], AlienRasterError> {
    render_vertices(
        source,
        indices,
        mesh.vertices.len(),
        projected,
        |vertex_index| mesh.vertices.get(vertex_index).map(|vertex| vertex.texture),
    )
}

fn render_pose_vertices(
    source: AlienFaceReference,
    indices: [usize; TRIANGLE_VERTEX_COUNT],
    pose: &AlienModelPose,
) -> Result<[AlienRenderVertex; TRIANGLE_VERTEX_COUNT], AlienRasterError> {
    render_vertices(
        source,
        indices,
        pose.texture_coordinates.len(),
        &pose.projected_vertices,
        |vertex_index| pose.texture_coordinates.get(vertex_index).copied(),
    )
}

fn render_vertices(
    source: AlienFaceReference,
    indices: [usize; TRIANGLE_VERTEX_COUNT],
    texture_count: usize,
    projected: &[AlienProjectedVertex],
    texture_at: impl Fn(usize) -> Option<[i16; 2]>,
) -> Result<[AlienRenderVertex; TRIANGLE_VERTEX_COUNT], AlienRasterError> {
    let vertices = indices.map(|vertex_index| {
        let texture = texture_at(vertex_index).ok_or(AlienRasterError::InvalidVertex {
            model_index: source.model_index,
            face_index: source.face_index,
            vertex_index,
            available: texture_count,
        })?;
        let projected = projected
            .get(vertex_index)
            .ok_or(AlienRasterError::InvalidVertex {
                model_index: source.model_index,
                face_index: source.face_index,
                vertex_index,
                available: projected.len(),
            })?;
        Ok(render_vertex(texture, *projected))
    });
    let [first, second, third] = vertices;
    Ok([first?, second?, third?])
}

fn render_vertex(texture: [i16; 2], projected: AlienProjectedVertex) -> AlienRenderVertex {
    AlienRenderVertex {
        screen: projected.screen,
        texture: texture.map(|coordinate| coordinate as u16),
        depth: projected.depth,
    }
}

/// Apply the original raster-capacity, edge, and fixed-point orientation tests.
fn face_is_active(
    indices: [usize; TRIANGLE_VERTEX_COUNT],
    vertices: &[AlienProjectedVertex],
    reciprocals: &[i32; RASTER_RECIPROCAL_COUNT],
    raster_capacity_available: bool,
) -> bool {
    let Some([first, second, third]) = projected_face(indices, vertices) else {
        return false;
    };
    face_coordinates_are_active(
        [first.screen, second.screen, third.screen],
        reciprocals,
        raster_capacity_available,
    )
}

fn projected_face(
    indices: [usize; TRIANGLE_VERTEX_COUNT],
    vertices: &[AlienProjectedVertex],
) -> Option<[AlienProjectedVertex; TRIANGLE_VERTEX_COUNT]> {
    Some([
        *vertices.get(indices[FIRST_VERTEX])?,
        *vertices.get(indices[SECOND_VERTEX])?,
        *vertices.get(indices[THIRD_VERTEX])?,
    ])
}

fn face_coordinates_are_active(
    screen: [[i16; 2]; TRIANGLE_VERTEX_COUNT],
    reciprocals: &[i32; RASTER_RECIPROCAL_COUNT],
    raster_capacity_available: bool,
) -> bool {
    if !raster_capacity_available {
        return false;
    }
    let width_1 = word_difference(screen[SECOND_VERTEX][X_AXIS], screen[FIRST_VERTEX][X_AXIS]);
    let width_2 = word_difference(screen[THIRD_VERTEX][X_AXIS], screen[FIRST_VERTEX][X_AXIS]);
    if width_1 == u16::MIN {
        if width_2 == u16::MIN || usize::from(width_2) >= MAXIMUM_FACE_WIDTH {
            return false;
        }
        let vertical_span =
            word_difference(screen[SECOND_VERTEX][Y_AXIS], screen[FIRST_VERTEX][Y_AXIS]);
        return (vertical_span as i16).is_positive()
            && usize::from(vertical_span) < MAXIMUM_FACE_WIDTH;
    }
    if width_2 == u16::MIN
        || usize::from(width_1) >= MAXIMUM_FACE_WIDTH
        || usize::from(width_2) >= MAXIMUM_FACE_WIDTH
    {
        return false;
    }

    let edge_1_step =
        i32::from(screen[SECOND_VERTEX][Y_AXIS].wrapping_sub(screen[FIRST_VERTEX][Y_AXIS]))
            .wrapping_mul(reciprocals[usize::from(width_1)]);
    let edge_0_step =
        i32::from(screen[THIRD_VERTEX][Y_AXIS].wrapping_sub(screen[FIRST_VERTEX][Y_AXIS]))
            .wrapping_mul(reciprocals[usize::from(width_2)]);
    edge_0_step.wrapping_sub(edge_1_step).is_negative()
}

const fn word_difference(left: i16, right: i16) -> u16 {
    (left as u16).wrapping_sub(right as u16)
}

const _: () = assert!(TEXTURE_WIDTH * TEXTURE_HEIGHT == 2 * (1 << 16));

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::alien::{AlienTransformData, AlienXdbKind, decode_alien_xdb};
    use serde::Deserialize;

    use super::*;
    use crate::native::alien::{AlienMouseSample, AlienScene};

    const CENTERED_MOUSE: AlienMouseSample = AlienMouseSample {
        x: 320,
        y: 512,
        buttons: 0,
    };
    const RESOURCE_EXHAUSTION_VECTOR: &str = "inactive";
    const RECIPROCAL_SCALE: u32 = 65_536;
    const TEST_FACE_REFERENCE: AlienFaceReference = AlienFaceReference {
        model_index: 0,
        face_index: 0,
    };
    const TEST_FACE_VERTICES: [usize; TRIANGLE_VERTEX_COUNT] = [0, 1, 2];
    const TEST_TEXTURE_COORDINATES: [[i16; 2]; TRIANGLE_VERTEX_COUNT] =
        [[11, 12], [21, 22], [31, 32]];

    #[derive(Deserialize)]
    struct ActivationVector {
        name: String,
        screen: [[i16; 2]; TRIANGLE_VERTEX_COUNT],
        accepted: bool,
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
    fn geometric_activation_matches_every_direct_alien_overlay_vector() {
        let reciprocal_table = std::array::from_fn(|width| {
            if width == usize::MIN {
                i32::MAX
            } else {
                (RECIPROCAL_SCALE / width as u32) as i32
            }
        });
        let fixtures = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_2b6d_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_2bdd_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_2c9d_natural.json"),
        ];
        for fixture in fixtures {
            let vectors: Vec<ActivationVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                assert_eq!(
                    face_coordinates_are_active(
                        vector.screen,
                        &reciprocal_table,
                        vector.name != RESOURCE_EXHAUSTION_VECTOR,
                    ),
                    vector.accepted,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn model_triangle_submission_uses_mutable_runtime_texture_coordinates() {
        let pose = AlienModelPose {
            root: AlienTransformData::default(),
            nodes: Vec::new(),
            projected_vertices: TEST_FACE_VERTICES
                .map(|vertex_index| AlienProjectedVertex {
                    screen: [vertex_index as i16, vertex_index as i16],
                    depth: i32::MAX,
                    clip_flags: u16::MIN,
                })
                .to_vec(),
            texture_coordinates: TEST_TEXTURE_COORDINATES.to_vec(),
            object_positions: vec![[i16::MIN; AXIS_COUNT]; TEST_FACE_VERTICES.len()],
            authored_vertex_count: TEST_FACE_VERTICES.len(),
            faces: Vec::new(),
            last_rotation_matrix: Default::default(),
            last_common_clip: u16::MIN,
        };
        let rendered =
            render_pose_vertices(TEST_FACE_REFERENCE, TEST_FACE_VERTICES, &pose).unwrap();
        assert_eq!(
            rendered.map(|vertex| vertex.texture),
            TEST_TEXTURE_COORDINATES.map(|texture| texture.map(|coordinate| coordinate as u16))
        );
    }

    #[test]
    fn shipped_scenes_emit_owned_primary_and_model_triangles() {
        let cases = [
            (AlienXdbKind::Amer, "amer.xdb"),
            (AlienXdbKind::Croolis, "croolis.xdb"),
            (AlienXdbKind::Scrut, "scrut.xdb"),
        ];
        for (kind, filename) in cases {
            let Some(path) = original_xdb(filename) else {
                continue;
            };
            let asset = decode_alien_xdb(&std::fs::read(path).unwrap(), kind).unwrap();
            let mut scene = AlienScene::from_asset(asset);
            let frame = scene.step(CENTERED_MOUSE).unwrap();
            assert!(!frame.geometry.primary_triangles.is_empty());
            assert!(
                frame
                    .geometry
                    .model_layers
                    .iter()
                    .any(|layer| !layer.is_empty())
            );
        }
    }
}
