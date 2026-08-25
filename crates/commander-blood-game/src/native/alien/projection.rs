//! Hierarchical model transformation and projection shared by all alien scenes.

use std::fmt;

use commander_blood_formats::alien::{
    AXIS_COUNT, AlienMeshData, AlienModelData, AlienNodeData, AlienNodeParent, AlienTransformData,
    AlienTrigonometryPair, TRIGONOMETRY_ENTRY_COUNT,
};

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const SCREEN_AXIS_COUNT: usize = 2;
const ANGLE_MASK: u16 = 0x0ffc;
const ANGLE_TABLE_SHIFT: u32 = 2;
const DOUBLE_COMPONENT: i32 = 2;
const HALF_COMPONENT_SHIFT: u32 = 1;
const MATRIX_PRODUCT_SHIFT: u32 = 15;
const RADIAL_PRODUCT_SHIFT: u32 = 16;
const RADIAL_ROUNDING_SHIFT: u32 = 15;
const RADIAL_ROUNDING_MASK: u32 = 0x0000_0001;
const DEPTH_SHIFT: u32 = 8;
const BEHIND_PROJECTION_SHIFT: u32 = 12;
const SCREEN_WIDTH: i32 = 320;
const SCREEN_HEIGHT: i32 = 200;
const LEFT_CLAMP_THRESHOLD: i32 = -90;
const LEFT_CLAMP_VALUE: i32 = -89;
const RIGHT_CLAMP_THRESHOLD: i32 = 410;
const RIGHT_CLAMP_VALUE: i32 = 409;
const TOP_CLAMP_THRESHOLD: i32 = -150;
const TOP_CLAMP_VALUE: i32 = -149;
const BOTTOM_CLAMP_THRESHOLD: i32 = 350;
const BOTTOM_CLAMP_VALUE: i32 = 349;
const CLIP_LEFT: u16 = 0x0001;
const CLIP_RIGHT: u16 = 0x0002;
const CLIP_TOP: u16 = 0x0004;
const CLIP_BOTTOM: u16 = 0x0008;
const CLIP_BEHIND: u16 = 0x8000;
const CLIP_HIGH_BYTE_MASK: u16 = 0xff00;
const COMMON_CLIP_INITIAL: u16 = CLIP_BEHIND | CLIP_LEFT | CLIP_RIGHT | CLIP_TOP | CLIP_BOTTOM;
const FULLY_REJECTED: u16 = 0x00ff;
const ZERO_COMPONENT: i32 = 0;
const ZERO_RADIAL_OFFSET: i16 = 0;

type Matrix = [[i32; AXIS_COUNT]; AXIS_COUNT];

/// Logical center used to convert projected coordinates into screen coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienScreenCenter {
    /// Horizontal screen-space center.
    pub x: i32,
    /// Vertical screen-space center.
    pub y: i32,
}

/// Screen-space result retained for one authored or UV-alias vertex.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienProjectedVertex {
    /// Clamped signed screen coordinate.
    pub screen: [i16; SCREEN_AXIS_COUNT],
    /// Recovered fixed-point depth value.
    pub depth: i32,
    /// Original visibility and clipping bit field.
    pub clip_flags: u16,
}

/// Mutable per-node state consumed by alien behavior and projection routines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienNodePose {
    /// Root, scene camera, or earlier node supplying the parent transform.
    pub parent: AlienNodeParent,
    /// First vertex controlled by this node.
    pub first_vertex: usize,
    /// Number of consecutive vertices controlled by this node.
    pub vertex_count: usize,
    /// Composed transform produced for the current frame.
    pub transform: AlienTransformData,
    /// Persistent local-position accumulators updated by radial movement.
    pub local_position: [i32; AXIS_COUNT],
    /// Wrapping pitch, pan, and secondary-pan angles.
    pub angles: [u16; AXIS_COUNT],
    /// Signed radial displacement applied along the local forward axis.
    pub radial_offset: i16,
}

impl From<&AlienNodeData> for AlienNodePose {
    fn from(node: &AlienNodeData) -> Self {
        Self {
            parent: node.parent,
            first_vertex: node.first_vertex,
            vertex_count: node.vertex_count,
            transform: node.transform,
            local_position: node.local_position,
            angles: node.angles,
            radial_offset: node.radial_offset,
        }
    }
}

/// Flat runtime pose and projection output for one decoded alien model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienModelPose {
    /// Mutable model-root transform used by root-relative nodes.
    pub root: AlienTransformData,
    /// Topologically ordered mutable node states.
    pub nodes: Vec<AlienNodePose>,
    /// Projection output parallel to the decoded mesh vertex array.
    pub projected_vertices: Vec<AlienProjectedVertex>,
    /// Rotation matrix generated for the final node in the last projection pass.
    pub last_rotation_matrix: Matrix,
    /// Common clip mask generated for the final node in the last projection pass.
    pub last_common_clip: u16,
}

impl AlienModelPose {
    /// Build mutable runtime state from flat decoded model data.
    pub fn from_model(model: &AlienModelData) -> Self {
        Self {
            root: model.root,
            nodes: model.nodes.iter().map(AlienNodePose::from).collect(),
            projected_vertices: vec![AlienProjectedVertex::default(); model.mesh.vertices.len()],
            last_rotation_matrix: [[ZERO_COMPONENT; AXIS_COUNT]; AXIS_COUNT],
            last_common_clip: COMMON_CLIP_INITIAL,
        }
    }

    /// Compose every node and project every vertex using recovered fixed-point rules.
    ///
    /// The model, camera, node graph, and vertex arrays are ordinary Rust values.
    /// Original overlay offsets and segmented addresses are not represented here.
    pub fn transform_and_project(
        &mut self,
        mesh: &AlienMeshData,
        scene_camera: AlienTransformData,
        screen_center: AlienScreenCenter,
        trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
    ) -> Result<(), AlienProjectionError> {
        if self.nodes.is_empty() {
            return Err(AlienProjectionError::EmptyHierarchy);
        }
        if self.projected_vertices.len() != mesh.vertices.len() {
            self.projected_vertices
                .resize(mesh.vertices.len(), AlienProjectedVertex::default());
        }

        for node_index in usize::MIN..self.nodes.len() {
            let parent = self.parent_transform(node_index, scene_camera)?;
            let rotation = node_rotation_matrix(self.nodes[node_index].angles, trigonometry);
            self.last_rotation_matrix = rotation;
            apply_radial_offset(&mut self.nodes[node_index], rotation);

            let local_position = self.nodes[node_index]
                .local_position
                .map(|component| i32::from(component as i16));
            self.nodes[node_index].transform.translation = std::array::from_fn(|row| {
                wrapping_dot(parent.matrix[row], local_position, parent.translation[row])
            });
            self.nodes[node_index].transform.matrix = compose_matrices(parent.matrix, rotation);

            let first_vertex = self.nodes[node_index].first_vertex;
            let vertex_count = self.nodes[node_index].vertex_count;
            if vertex_count == usize::MIN {
                return Err(AlienProjectionError::EmptyVertexRange { node_index });
            }
            let end_vertex = first_vertex.checked_add(vertex_count).ok_or(
                AlienProjectionError::InvalidVertexRange {
                    node_index,
                    first_vertex,
                    vertex_count,
                    available: mesh.vertices.len(),
                },
            )?;
            if end_vertex > mesh.vertices.len() {
                return Err(AlienProjectionError::InvalidVertexRange {
                    node_index,
                    first_vertex,
                    vertex_count,
                    available: mesh.vertices.len(),
                });
            }

            let transform = self.nodes[node_index].transform;
            let mut common_clip = COMMON_CLIP_INITIAL;
            for vertex_index in first_vertex..end_vertex {
                let object_position = mesh.vertices[vertex_index].position.map(i32::from);
                let camera_position = std::array::from_fn(|row| {
                    wrapping_dot(
                        transform.matrix[row],
                        object_position,
                        transform.translation[row],
                    )
                });
                let projected =
                    project_vertex(camera_position, screen_center, node_index, vertex_index)?;
                common_clip &= projected.clip_flags;
                self.projected_vertices[vertex_index] = projected;
            }
            if common_clip != u16::MIN {
                for projected in &mut self.projected_vertices[first_vertex..end_vertex] {
                    projected.clip_flags = FULLY_REJECTED;
                }
            }
            self.last_common_clip = common_clip;
        }

        for copy in &mesh.projection_copies {
            let source = self.projected_vertices.get(copy.source).copied().ok_or(
                AlienProjectionError::InvalidProjectionCopy {
                    source: copy.source,
                    destination: copy.destination,
                    available: self.projected_vertices.len(),
                },
            )?;
            let available = self.projected_vertices.len();
            let destination = self.projected_vertices.get_mut(copy.destination).ok_or(
                AlienProjectionError::InvalidProjectionCopy {
                    source: copy.source,
                    destination: copy.destination,
                    available,
                },
            )?;
            *destination = source;
        }
        Ok(())
    }

    fn parent_transform(
        &self,
        node_index: usize,
        scene_camera: AlienTransformData,
    ) -> Result<AlienTransformData, AlienProjectionError> {
        match self.nodes[node_index].parent {
            AlienNodeParent::SceneCamera => Ok(scene_camera),
            AlienNodeParent::Root => Ok(self.root),
            AlienNodeParent::Node(parent_index) if parent_index < node_index => {
                Ok(self.nodes[parent_index].transform)
            }
            AlienNodeParent::Node(parent_index) => Err(AlienProjectionError::InvalidParent {
                node_index,
                parent_index,
            }),
        }
    }
}

/// Structural or arithmetic failure in a decoded model projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienProjectionError {
    /// A projection context had no nodes, while the recovered routine requires one.
    EmptyHierarchy,
    /// A node had no authored vertices.
    EmptyVertexRange {
        /// Index of the invalid node.
        node_index: usize,
    },
    /// A node's consecutive vertex range exceeded the mesh.
    InvalidVertexRange {
        /// Index of the invalid node.
        node_index: usize,
        /// First requested vertex.
        first_vertex: usize,
        /// Number of requested vertices.
        vertex_count: usize,
        /// Number of available mesh vertices.
        available: usize,
    },
    /// A node referred to itself or a later node as its parent.
    InvalidParent {
        /// Index of the invalid node.
        node_index: usize,
        /// Invalid parent index.
        parent_index: usize,
    },
    /// A projection-copy source or destination was outside the mesh.
    InvalidProjectionCopy {
        /// Requested source index.
        source: usize,
        /// Requested destination index.
        destination: usize,
        /// Number of available projected vertices.
        available: usize,
    },
    /// Signed division overflowed while projecting a vertex.
    ProjectionDivisionOverflow {
        /// Node controlling the vertex.
        node_index: usize,
        /// Vertex that could not be projected.
        vertex_index: usize,
    },
}

impl fmt::Display for AlienProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien projection state: {self:?}")
    }
}

impl std::error::Error for AlienProjectionError {}

fn angle_sample(
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
    angle: u16,
) -> AlienTrigonometryPair {
    trigonometry[usize::from((angle & ANGLE_MASK) >> ANGLE_TABLE_SHIFT)]
}

fn node_rotation_matrix(
    angles: [u16; AXIS_COUNT],
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
) -> Matrix {
    let pitch = angles[X_AXIS] & ANGLE_MASK;
    let pan = angles[Y_AXIS] & ANGLE_MASK;
    let secondary = angles[Z_AXIS] & ANGLE_MASK;
    let mut matrix = [[ZERO_COMPONENT; AXIS_COUNT]; AXIS_COUNT];
    matrix[Y_AXIS][Z_AXIS] = i32::from(angle_sample(trigonometry, pitch).sine)
        .wrapping_mul(DOUBLE_COMPONENT)
        .wrapping_neg();

    let combined = pan.wrapping_add(secondary) & ANGLE_MASK;
    let first = angle_sample(trigonometry, pitch.wrapping_sub(combined));
    let second = angle_sample(trigonometry, pitch.wrapping_add(combined));
    let base = angle_sample(trigonometry, combined);
    let cosine_half_difference =
        i32::from(first.cosine).wrapping_sub(i32::from(second.cosine)) >> HALF_COMPONENT_SHIFT;
    let sine_half_sum =
        i32::from(first.sine).wrapping_add(i32::from(second.sine)) >> HALF_COMPONENT_SHIFT;
    matrix[X_AXIS][Y_AXIS] = cosine_half_difference.wrapping_add(i32::from(base.sine));
    matrix[Z_AXIS][X_AXIS] = matrix[X_AXIS][Y_AXIS].wrapping_neg();
    matrix[X_AXIS][X_AXIS] = sine_half_sum.wrapping_add(i32::from(base.cosine));
    matrix[Z_AXIS][Y_AXIS] = matrix[X_AXIS][X_AXIS];

    let combined = pan.wrapping_sub(secondary) & ANGLE_MASK;
    let first = angle_sample(trigonometry, pitch.wrapping_sub(combined));
    let second = angle_sample(trigonometry, pitch.wrapping_add(combined));
    let base = angle_sample(trigonometry, combined);
    let cosine_half_difference =
        i32::from(first.cosine).wrapping_sub(i32::from(second.cosine)) >> HALF_COMPONENT_SHIFT;
    let sine_half_sum =
        i32::from(first.sine).wrapping_add(i32::from(second.sine)) >> HALF_COMPONENT_SHIFT;
    let sine_correction = i32::from(base.sine).wrapping_sub(cosine_half_difference);
    let cosine_correction = i32::from(base.cosine).wrapping_sub(sine_half_sum);
    matrix[X_AXIS][Y_AXIS] = matrix[X_AXIS][Y_AXIS].wrapping_sub(sine_correction);
    matrix[Z_AXIS][X_AXIS] = matrix[Z_AXIS][X_AXIS].wrapping_sub(sine_correction);
    matrix[X_AXIS][X_AXIS] = matrix[X_AXIS][X_AXIS].wrapping_add(cosine_correction);
    matrix[Z_AXIS][Y_AXIS] = matrix[Z_AXIS][Y_AXIS].wrapping_sub(cosine_correction);

    let first = angle_sample(trigonometry, secondary.wrapping_add(pitch));
    let second = angle_sample(trigonometry, secondary.wrapping_sub(pitch));
    matrix[Y_AXIS][Y_AXIS] = i32::from(first.cosine).wrapping_add(i32::from(second.cosine));
    matrix[Y_AXIS][X_AXIS] = i32::from(first.sine)
        .wrapping_add(i32::from(second.sine))
        .wrapping_neg();

    let first = angle_sample(trigonometry, pan.wrapping_add(pitch));
    let second = angle_sample(trigonometry, pan.wrapping_sub(pitch));
    matrix[Z_AXIS][Z_AXIS] = i32::from(first.cosine).wrapping_add(i32::from(second.cosine));
    matrix[X_AXIS][Z_AXIS] = i32::from(first.sine).wrapping_add(i32::from(second.sine));
    matrix
}

fn apply_radial_offset(node: &mut AlienNodePose, rotation: Matrix) {
    if node.radial_offset == ZERO_RADIAL_OFFSET {
        return;
    }
    let radial = i32::from(node.radial_offset);
    for (axis, rotation_row) in rotation.into_iter().enumerate() {
        let product = wrapping_multiply(rotation_row[Z_AXIS], radial);
        let mut delta = product >> RADIAL_PRODUCT_SHIFT;
        if axis == Y_AXIS {
            delta = delta.wrapping_add(
                ((product as u32 >> RADIAL_ROUNDING_SHIFT) & RADIAL_ROUNDING_MASK) as i32,
            );
        }
        node.local_position[axis] = node.local_position[axis].wrapping_add(delta);
    }
}

fn compose_matrices(parent: Matrix, local: Matrix) -> Matrix {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            let local_column = std::array::from_fn(|term| local[term][column]);
            wrapping_dot(parent[row], local_column, ZERO_COMPONENT) >> MATRIX_PRODUCT_SHIFT
        })
    })
}

fn wrapping_multiply(left: i32, right: i32) -> i32 {
    (left as u32).wrapping_mul(right as u32) as i32
}

fn wrapping_dot(left: [i32; AXIS_COUNT], right: [i32; AXIS_COUNT], bias: i32) -> i32 {
    left.into_iter()
        .zip(right)
        .fold(bias as u32, |accumulator, (left, right)| {
            accumulator.wrapping_add((left as u32).wrapping_mul(right as u32))
        }) as i32
}

fn project_vertex(
    camera_position: [i32; AXIS_COUNT],
    center: AlienScreenCenter,
    node_index: usize,
    vertex_index: usize,
) -> Result<AlienProjectedVertex, AlienProjectionError> {
    let depth = camera_position[Z_AXIS] >> DEPTH_SHIFT;
    let (mut screen_x, mut screen_y, mut clip_flags) = if depth > ZERO_COMPONENT {
        let screen_x = camera_position[X_AXIS].checked_div(depth).ok_or(
            AlienProjectionError::ProjectionDivisionOverflow {
                node_index,
                vertex_index,
            },
        )?;
        let screen_y = camera_position[Y_AXIS].checked_div(depth).ok_or(
            AlienProjectionError::ProjectionDivisionOverflow {
                node_index,
                vertex_index,
            },
        )?;
        (screen_x, screen_y, u16::MIN)
    } else {
        (
            camera_position[X_AXIS] >> BEHIND_PROJECTION_SHIFT,
            camera_position[Y_AXIS] >> BEHIND_PROJECTION_SHIFT,
            CLIP_BEHIND,
        )
    };

    screen_y = screen_y.wrapping_neg();
    screen_x = screen_x.wrapping_add(center.x);
    if screen_x < ZERO_COMPONENT {
        clip_flags |= CLIP_LEFT;
        if screen_x <= LEFT_CLAMP_THRESHOLD {
            screen_x = LEFT_CLAMP_VALUE;
        }
    }
    if screen_x >= SCREEN_WIDTH {
        clip_flags = (clip_flags & CLIP_HIGH_BYTE_MASK) | CLIP_RIGHT;
        if screen_x >= RIGHT_CLAMP_THRESHOLD {
            screen_x = RIGHT_CLAMP_VALUE;
        }
    }

    screen_y = screen_y.wrapping_add(center.y);
    if screen_y < ZERO_COMPONENT {
        clip_flags |= CLIP_TOP;
        if screen_y <= TOP_CLAMP_THRESHOLD {
            screen_y = TOP_CLAMP_VALUE;
        }
    }
    if screen_y >= SCREEN_HEIGHT {
        clip_flags |= CLIP_BOTTOM;
        if screen_y >= BOTTOM_CLAMP_THRESHOLD {
            screen_y = BOTTOM_CLAMP_VALUE;
        }
    }

    Ok(AlienProjectedVertex {
        screen: [screen_x as i16, screen_y as i16],
        depth,
        clip_flags,
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::alien::{
        AlienFaceData, AlienProjectionCopyData, AlienVertexData, AlienXdbKind, decode_alien_xdb,
    };
    use serde::Deserialize;

    use super::*;

    const ORIGINAL_SCREEN_CENTER: AlienScreenCenter = AlienScreenCenter { x: 160, y: 100 };
    const IDENTITY_MATRIX_COMPONENT: i32 = 32_768;

    #[derive(Deserialize)]
    struct TrigonometryPattern {
        cosine_multiplier: u16,
        cosine_offset: u16,
        sine_multiplier: u16,
        sine_offset: u16,
    }

    #[derive(Deserialize)]
    struct TransformVector {
        matrix: [i32; AXIS_COUNT * AXIS_COUNT],
        translation: [i32; AXIS_COUNT],
    }

    #[derive(Deserialize)]
    struct NodeBeforeVector {
        parent: Option<usize>,
        angles: [u16; AXIS_COUNT],
        radial_offset: i16,
        local_position: [i32; AXIS_COUNT],
        vertices: Vec<[i16; AXIS_COUNT]>,
    }

    #[derive(Deserialize)]
    struct NodeAfterVector {
        local_position: [i32; AXIS_COUNT],
        matrix: [i32; AXIS_COUNT * AXIS_COUNT],
        translation: [i32; AXIS_COUNT],
    }

    #[derive(Deserialize)]
    struct ProjectedVector {
        screen_x: i16,
        screen_y: i16,
        depth: i32,
        flags: u16,
    }

    #[derive(Deserialize)]
    struct ProjectionVector {
        name: String,
        screen_center: [i32; SCREEN_AXIS_COUNT],
        trigonometry_pattern: TrigonometryPattern,
        root: TransformVector,
        nodes_before: Vec<NodeBeforeVector>,
        nodes_after: Vec<NodeAfterVector>,
        projection_copies: Vec<AlienProjectionCopyVector>,
        projected_vertices: Vec<ProjectedVector>,
        last_rotation_matrix: [i32; AXIS_COUNT * AXIS_COUNT],
        last_common_clip: u16,
    }

    #[derive(Deserialize)]
    struct AlienProjectionCopyVector {
        source: usize,
        destination: usize,
    }

    fn matrix(flat: [i32; AXIS_COUNT * AXIS_COUNT]) -> Matrix {
        std::array::from_fn(|row| std::array::from_fn(|column| flat[row * AXIS_COUNT + column]))
    }

    fn trigonometry(
        pattern: TrigonometryPattern,
    ) -> [AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT] {
        std::array::from_fn(|index| AlienTrigonometryPair {
            cosine: (index as u16)
                .wrapping_mul(pattern.cosine_multiplier)
                .wrapping_add(pattern.cosine_offset) as i16,
            sine: (index as u16)
                .wrapping_mul(pattern.sine_multiplier)
                .wrapping_add(pattern.sine_offset) as i16,
        })
    }

    fn run_vector(vector: ProjectionVector) {
        let mut first_vertex = usize::MIN;
        let mut vertices = Vec::new();
        let nodes = vector
            .nodes_before
            .iter()
            .map(|node| {
                let node_first_vertex = first_vertex;
                vertices.extend(node.vertices.iter().map(|position| AlienVertexData {
                    position: *position,
                    ..AlienVertexData::default()
                }));
                first_vertex += node.vertices.len();
                AlienNodePose {
                    parent: node
                        .parent
                        .map_or(AlienNodeParent::Root, AlienNodeParent::Node),
                    first_vertex: node_first_vertex,
                    vertex_count: node.vertices.len(),
                    transform: AlienTransformData::default(),
                    local_position: node.local_position,
                    angles: node.angles,
                    radial_offset: node.radial_offset,
                }
            })
            .collect();
        vertices.resize(vector.projected_vertices.len(), AlienVertexData::default());
        let mesh = AlienMeshData {
            vertices,
            projection_copies: vector
                .projection_copies
                .iter()
                .map(|copy| AlienProjectionCopyData {
                    source: copy.source,
                    destination: copy.destination,
                })
                .collect(),
            faces: Vec::<AlienFaceData>::new(),
        };
        let mut pose = AlienModelPose {
            root: AlienTransformData {
                matrix: matrix(vector.root.matrix),
                translation: vector.root.translation,
            },
            nodes,
            projected_vertices: vec![AlienProjectedVertex::default(); mesh.vertices.len()],
            last_rotation_matrix: [[ZERO_COMPONENT; AXIS_COUNT]; AXIS_COUNT],
            last_common_clip: COMMON_CLIP_INITIAL,
        };
        let table = trigonometry(vector.trigonometry_pattern);
        pose.transform_and_project(
            &mesh,
            pose.root,
            AlienScreenCenter {
                x: vector.screen_center[X_AXIS],
                y: vector.screen_center[Y_AXIS],
            },
            &table,
        )
        .unwrap();

        for (node, expected) in pose.nodes.iter().zip(&vector.nodes_after) {
            assert_eq!(
                node.local_position, expected.local_position,
                "{}",
                vector.name
            );
            assert_eq!(
                node.transform.matrix,
                matrix(expected.matrix),
                "{}",
                vector.name
            );
            assert_eq!(
                node.transform.translation, expected.translation,
                "{}",
                vector.name
            );
        }
        for (projected, expected) in pose
            .projected_vertices
            .iter()
            .zip(&vector.projected_vertices)
        {
            assert_eq!(
                projected.screen,
                [expected.screen_x, expected.screen_y],
                "{}",
                vector.name
            );
            assert_eq!(projected.depth, expected.depth, "{}", vector.name);
            assert_eq!(projected.clip_flags, expected.flags, "{}", vector.name);
        }
        assert_eq!(
            pose.last_rotation_matrix,
            matrix(vector.last_rotation_matrix),
            "{}",
            vector.name
        );
        assert_eq!(
            pose.last_common_clip, vector.last_common_clip,
            "{}",
            vector.name
        );
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
    fn model_projection_matches_every_original_alien_overlay_vector() {
        let suites = [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_2027_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_206c_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_212c_natural.json"),
        ];
        for json in suites {
            let vectors: Vec<ProjectionVector> = serde_json::from_str(json).unwrap();
            for vector in vectors {
                run_vector(vector);
            }
        }
    }

    #[test]
    fn every_decoded_original_model_projects_through_flat_runtime_state() {
        let cases = [
            (AlienXdbKind::Amer, "amer.xdb"),
            (AlienXdbKind::Croolis, "croolis.xdb"),
            (AlienXdbKind::Scrut, "scrut.xdb"),
        ];
        let scene_camera = AlienTransformData {
            matrix: std::array::from_fn(|row| {
                std::array::from_fn(|column| {
                    if row == column {
                        IDENTITY_MATRIX_COMPONENT
                    } else {
                        ZERO_COMPONENT
                    }
                })
            }),
            translation: [ZERO_COMPONENT; AXIS_COUNT],
        };

        for (kind, filename) in cases {
            let Some(path) = original_xdb(filename) else {
                continue;
            };
            let data = std::fs::read(path).unwrap();
            let asset = decode_alien_xdb(&data, kind).unwrap();
            for model in &asset.models {
                let mut pose = AlienModelPose::from_model(model);
                pose.transform_and_project(
                    &model.mesh,
                    scene_camera,
                    ORIGINAL_SCREEN_CENTER,
                    &asset.trigonometry,
                )
                .unwrap_or_else(|error| panic!("{} failed projection: {error}", model.name));
                assert_eq!(pose.projected_vertices.len(), model.mesh.vertices.len());
            }
        }
    }
}
