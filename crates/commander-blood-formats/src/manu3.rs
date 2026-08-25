//! Typed decoder for the `MANU3.XDB` skeletal hand model and animation data.
//!
//! The XDB is a raw overlay image containing code followed by initialized data
//! and work sections. This decoder follows the overlay's own relocation deltas
//! to recover authored resources. It does not construct DOS addresses or expose
//! the mutable projection and raster scratch fields embedded in those records.

/// Number of transform rows, position components, and Euler angles.
pub const AXIS_COUNT: usize = 3;
/// Number of animated skeleton nodes in the MANU3 model.
pub const NODE_COUNT: usize = 16;
/// Number of selectable animation scripts.
pub const ANIMATION_COUNT: usize = 32;
/// Number of Q14 cosine/sine table entries.
pub const TRIGONOMETRY_ENTRY_COUNT: usize = 1_024;

const DATA_DELTA_FIELD: usize = 0x1368;
const DATA_DIRECTORY_WORK_DELTAS: usize = 0x000c;
const GEOMETRY_WORK_DELTA_INDEX: usize = 0;
const TEXTURE_WORK_DELTA_INDEX: usize = 1;
const PARAGRAPH_BYTE_COUNT: usize = 16;
const ROOT_RECORD_POSITION: usize = 0x2274;
const NODE_RECORD_POSITION: usize = 0x2394;
const NODE_RECORD_SIZE: usize = 0x005e;
const NODE_VERTEX_COUNT_FIELD: usize = 0x0002;
const NODE_VERTEX_START_FIELD: usize = 0x0006;
const NODE_MATRIX_FIELD: usize = 0x0012;
const NODE_TRANSLATION_FIELD: usize = 0x0036;
const NODE_LOCAL_POSITION_FIELD: usize = 0x0042;
const NODE_ANGLE_FIELD: usize = 0x004e;
const NODE_RADIAL_OFFSET_FIELD: usize = 0x0054;
const ROOT_MATRIX_FIELD: usize = NODE_MATRIX_FIELD;
const ROOT_TRANSLATION_FIELD: usize = NODE_TRANSLATION_FIELD;
const MODEL_HEADER_POSITION: usize = 0x22d8;
const MODEL_MAGIC_FIELD: usize = 0x0000;
const MODEL_MAGIC: &[u8; 4] = b"3DB0";
const PROJECTION_COPY_START_FIELD: usize = 0x22fa;
const PROJECTION_COPY_COUNT_FIELD: usize = 0x22fe;
const FACE_START_FIELD: usize = 0x2300;
const FACE_COUNT_FIELD: usize = 0x2304;
const ANIMATION_TABLE_FIELD: usize = 0x2306;
const TRIGONOMETRY_POSITION: usize = 0x0026;
const TRIGONOMETRY_PAIR_SIZE: usize = 4;
const VERTEX_RECORD_SIZE: usize = 20;
const VERTEX_TEXTURE_FIELD: usize = 0;
const VERTEX_POSITION_FIELD: usize = 4;
const FACE_RECORD_SIZE: usize = 8;
const FACE_FIRST_VERTEX_FIELD: usize = 2;
const TWEEN_RECORD_SIZE: usize = 8;
const TWEEN_UNUSED_WORD_FIELD: usize = 2;
const TWEEN_TARGET_FIELD: usize = 4;
const TWEEN_END_VALUE_FIELD: usize = 6;
const TEXTURE_WIDTH: usize = 256;
const TEXTURE_HEIGHT: usize = 64;
const TEXTURE_PIXEL_COUNT: usize = TEXTURE_WIDTH * TEXTURE_HEIGHT;
const LOCAL_POSITION_FIELD_STRIDE: usize = 4;
const ANGLE_FIELD_STRIDE: usize = 2;
const ZERO_POSITION: [i16; AXIS_COUNT] = [0; AXIS_COUNT];

/// One of the three spatial axes used by MANU3 data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Horizontal X component.
    X,
    /// Vertical Y component.
    Y,
    /// Depth Z component.
    Z,
}

impl Axis {
    /// Return the zero-based component index used by arrays in decoded data.
    pub const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }

    fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::X),
            1 => Some(Self::Y),
            2 => Some(Self::Z),
            _ => None,
        }
    }
}

/// One Q14 trigonometry pair authored in MANU3.XDB.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrigonometryPair {
    /// Cosine scaled by 16,384.
    pub cosine: i16,
    /// Sine scaled by 16,384.
    pub sine: i16,
}

/// Initial fixed-point transform stored in an XDB node or root record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransformData {
    /// Row-major 3-by-3 orientation matrix.
    pub matrix: [[i32; AXIS_COUNT]; AXIS_COUNT],
    /// Three-component translation.
    pub translation: [i32; AXIS_COUNT],
}

/// Typed parent relation for one skeleton node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeParent {
    /// Parent is the model's root transform.
    Root,
    /// Parent is an earlier node in the same skeleton.
    Node(usize),
}

/// Authored state for one skeletal hand node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeData {
    /// Root or earlier skeletal node supplying the parent transform.
    pub parent: NodeParent,
    /// First model vertex controlled by this node.
    pub first_vertex: usize,
    /// Number of consecutive model vertices controlled by this node.
    pub vertex_count: usize,
    /// Initial composed transform fields from the overlay image.
    pub transform: TransformData,
    /// Three mutable local-position accumulators.
    pub local_position: [i32; AXIS_COUNT],
    /// Three wrapping native Euler angles.
    pub angles: [u16; AXIS_COUNT],
    /// Signed radial displacement applied during matrix construction.
    pub radial_offset: i16,
}

/// Authored texture and model coordinates for one vertex or UV alias.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VertexData {
    /// Texture coordinates in texels.
    pub texture: [i16; 2],
    /// Model-space position. Alias entries retain zero here and use a
    /// [`ProjectionCopyData`] for their projected position.
    pub position: [i16; AXIS_COUNT],
}

/// Projection sharing used by a UV-seam alias vertex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectionCopyData {
    /// Authored vertex that supplies the projected position.
    pub source: usize,
    /// Alias vertex receiving that position while retaining its own UV.
    pub destination: usize,
}

/// Three vertex indices forming one authored triangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceData {
    /// Indices into [`Manu3Asset::vertices`].
    pub vertices: [usize; AXIS_COUNT],
}

/// Exact node field controlled by an animation command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TweenTarget {
    /// Low signed word of a node's local-position accumulator.
    NodeLocalPosition {
        /// Skeleton node index.
        node: usize,
        /// Position component.
        axis: Axis,
    },
    /// One of a node's wrapping Euler angles.
    NodeAngle {
        /// Skeleton node index.
        node: usize,
        /// Angle axis.
        axis: Axis,
    },
}

/// One phased animation command decoded from the XDB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TweenCommand {
    /// Number of frames used by the interpolation.
    pub frame_count: u8,
    /// Sequence phase in which the interpolation starts.
    pub phase: u8,
    /// Word retained in the authored record but not read by the native tween
    /// constructor. It is preserved for lossless inspection.
    pub unused_word: u16,
    /// Typed node field receiving interpolated values.
    pub target: TweenTarget,
    /// Signed final value.
    pub end_value: i16,
}

/// Indexed texture stored in the XDB's texture section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexedTexture {
    /// Texture width in texels.
    pub width: usize,
    /// Texture height in texels.
    pub height: usize,
    /// Row-major palette indices.
    pub pixels: Vec<u8>,
}

/// Complete authored MANU3 model recovered from one original XDB image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manu3Asset {
    /// Root transform used by the first skeletal node.
    pub root: TransformData,
    /// Sixteen topologically ordered skeleton nodes.
    pub nodes: Vec<NodeData>,
    /// Authored vertices followed by UV-seam alias vertices.
    pub vertices: Vec<VertexData>,
    /// Projection sharing for every alias vertex.
    pub projection_copies: Vec<ProjectionCopyData>,
    /// Authored triangle list.
    pub faces: Vec<FaceData>,
    /// Indexed 256-by-64 hand texture.
    pub texture: IndexedTexture,
    /// Complete Q14 cosine/sine lookup table.
    pub trigonometry: [TrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
    /// Thirty-two selector entries; repeated null selectors remain repeated.
    pub animations: [Vec<TweenCommand>; ANIMATION_COUNT],
}

fn read_u16(data: &[u8], position: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(position..position + size_of::<u16>())?
            .try_into()
            .ok()?,
    ))
}

fn read_i16(data: &[u8], position: usize) -> Option<i16> {
    Some(i16::from_le_bytes(
        data.get(position..position + size_of::<i16>())?
            .try_into()
            .ok()?,
    ))
}

fn read_i32(data: &[u8], position: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        data.get(position..position + size_of::<i32>())?
            .try_into()
            .ok()?,
    ))
}

fn checked_array<T, const LENGTH: usize>(
    mut element: impl FnMut(usize) -> Option<T>,
) -> Option<[T; LENGTH]> {
    (0..LENGTH)
        .map(&mut element)
        .collect::<Option<Vec<_>>>()?
        .try_into()
        .ok()
}

fn transform(
    data: &[u8],
    record: usize,
    matrix_field: usize,
    translation_field: usize,
) -> Option<TransformData> {
    let matrix = checked_array(|row| {
        checked_array(|column| {
            read_i32(
                data,
                record + matrix_field + (row * AXIS_COUNT + column) * size_of::<i32>(),
            )
        })
    })?;
    let translation =
        checked_array(|axis| read_i32(data, record + translation_field + axis * size_of::<i32>()))?;
    Some(TransformData {
        matrix,
        translation,
    })
}

fn tween_target(position: usize) -> Option<TweenTarget> {
    let relative = position.checked_sub(NODE_RECORD_POSITION)?;
    let node = relative / NODE_RECORD_SIZE;
    if node >= NODE_COUNT {
        return None;
    }
    let field = relative % NODE_RECORD_SIZE;
    if let Some(axis) = (0..AXIS_COUNT)
        .find(|axis| field == NODE_LOCAL_POSITION_FIELD + axis * LOCAL_POSITION_FIELD_STRIDE)
    {
        return Some(TweenTarget::NodeLocalPosition {
            node,
            axis: Axis::from_index(axis)?,
        });
    }
    let axis =
        (0..AXIS_COUNT).find(|axis| field == NODE_ANGLE_FIELD + axis * ANGLE_FIELD_STRIDE)?;
    Some(TweenTarget::NodeAngle {
        node,
        axis: Axis::from_index(axis)?,
    })
}

/// Decode the original `MANU3.XDB` resource into flat, typed authored data.
/// Returns `None` when section bounds, model magic, node topology, indices, or
/// animation targets are malformed.
pub fn decode_manu3(data: &[u8]) -> Option<Manu3Asset> {
    let data_delta = usize::from(read_u16(data, DATA_DELTA_FIELD)?);
    let data_start = data_delta.checked_mul(PARAGRAPH_BYTE_COUNT)?;
    let geometry_delta = usize::from(read_u16(
        data,
        data_start + DATA_DIRECTORY_WORK_DELTAS + GEOMETRY_WORK_DELTA_INDEX * size_of::<u16>(),
    )?);
    let texture_delta = usize::from(read_u16(
        data,
        data_start + DATA_DIRECTORY_WORK_DELTAS + TEXTURE_WORK_DELTA_INDEX * size_of::<u16>(),
    )?);
    let geometry_start = data_start.checked_add(geometry_delta * PARAGRAPH_BYTE_COUNT)?;
    let texture_start = geometry_start.checked_add(texture_delta * PARAGRAPH_BYTE_COUNT)?;
    let data_at = |position: usize| data_start.checked_add(position);

    let magic = data.get(
        data_at(MODEL_HEADER_POSITION + MODEL_MAGIC_FIELD)?
            ..data_at(MODEL_HEADER_POSITION + MODEL_MAGIC_FIELD + MODEL_MAGIC.len())?,
    )?;
    if magic != MODEL_MAGIC {
        return None;
    }

    let root_position = data_at(ROOT_RECORD_POSITION)?;
    let root = transform(
        data,
        root_position,
        ROOT_MATRIX_FIELD,
        ROOT_TRANSLATION_FIELD,
    )?;
    let mut nodes = Vec::with_capacity(NODE_COUNT);
    for node_index in 0..NODE_COUNT {
        let relative_position = NODE_RECORD_POSITION + node_index * NODE_RECORD_SIZE;
        let position = data_at(relative_position)?;
        let parent_position = usize::from(read_u16(data, position)?);
        let parent = if parent_position == ROOT_RECORD_POSITION {
            NodeParent::Root
        } else {
            let relative_parent = parent_position.checked_sub(NODE_RECORD_POSITION)?;
            if relative_parent % NODE_RECORD_SIZE != usize::MIN {
                return None;
            }
            let parent_node = relative_parent / NODE_RECORD_SIZE;
            if parent_node >= node_index {
                return None;
            }
            NodeParent::Node(parent_node)
        };
        let first_vertex_bytes = usize::from(read_u16(data, position + NODE_VERTEX_START_FIELD)?);
        if first_vertex_bytes % VERTEX_RECORD_SIZE != usize::MIN {
            return None;
        }
        nodes.push(NodeData {
            parent,
            first_vertex: first_vertex_bytes / VERTEX_RECORD_SIZE,
            vertex_count: usize::from(read_u16(data, position + NODE_VERTEX_COUNT_FIELD)?),
            transform: transform(data, position, NODE_MATRIX_FIELD, NODE_TRANSLATION_FIELD)?,
            local_position: checked_array(|axis| {
                read_i32(
                    data,
                    position + NODE_LOCAL_POSITION_FIELD + axis * size_of::<i32>(),
                )
            })?,
            angles: checked_array(|axis| {
                read_u16(data, position + NODE_ANGLE_FIELD + axis * size_of::<u16>())
            })?,
            radial_offset: read_i16(data, position + NODE_RADIAL_OFFSET_FIELD)?,
        });
    }

    let authored_vertex_count = nodes
        .iter()
        .map(|node| node.first_vertex.checked_add(node.vertex_count))
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .max()?;
    let copy_start_bytes = usize::from(read_u16(data, data_at(PROJECTION_COPY_START_FIELD)?)?);
    let copy_count = usize::from(read_u16(data, data_at(PROJECTION_COPY_COUNT_FIELD)?)?);
    if copy_start_bytes % VERTEX_RECORD_SIZE != usize::MIN
        || copy_start_bytes / VERTEX_RECORD_SIZE != authored_vertex_count
    {
        return None;
    }
    let total_vertex_count = authored_vertex_count.checked_add(copy_count)?;
    let mut vertices = Vec::with_capacity(total_vertex_count);
    for vertex_index in 0..total_vertex_count {
        let position = geometry_start.checked_add(vertex_index * VERTEX_RECORD_SIZE)?;
        vertices.push(VertexData {
            texture: checked_array(|axis| {
                read_i16(
                    data,
                    position + VERTEX_TEXTURE_FIELD + axis * size_of::<i16>(),
                )
            })?,
            position: if vertex_index < authored_vertex_count {
                checked_array(|axis| {
                    read_i16(
                        data,
                        position + VERTEX_POSITION_FIELD + axis * size_of::<i16>(),
                    )
                })?
            } else {
                ZERO_POSITION
            },
        });
    }
    let mut projection_copies = Vec::with_capacity(copy_count);
    for copy_index in 0..copy_count {
        let destination = authored_vertex_count + copy_index;
        let position = geometry_start.checked_add(destination * VERTEX_RECORD_SIZE)?;
        let source_bytes = usize::from(read_u16(data, position + VERTEX_POSITION_FIELD)?);
        if source_bytes % VERTEX_RECORD_SIZE != usize::MIN {
            return None;
        }
        let source = source_bytes / VERTEX_RECORD_SIZE;
        if source >= authored_vertex_count {
            return None;
        }
        projection_copies.push(ProjectionCopyData {
            source,
            destination,
        });
    }

    let face_start = usize::from(read_u16(data, data_at(FACE_START_FIELD)?)?);
    let face_count = usize::from(read_u16(data, data_at(FACE_COUNT_FIELD)?)?);
    let mut faces = Vec::with_capacity(face_count);
    for face_index in 0..face_count {
        let position = geometry_start
            .checked_add(face_start)?
            .checked_add(face_index * FACE_RECORD_SIZE)?;
        let face_vertices = checked_array(|corner| {
            let vertex_bytes = usize::from(read_u16(
                data,
                position + FACE_FIRST_VERTEX_FIELD + corner * size_of::<u16>(),
            )?);
            if vertex_bytes % VERTEX_RECORD_SIZE != usize::MIN {
                return None;
            }
            let vertex = vertex_bytes / VERTEX_RECORD_SIZE;
            (vertex < total_vertex_count).then_some(vertex)
        })?;
        faces.push(FaceData {
            vertices: face_vertices,
        });
    }

    let texture_end = texture_start.checked_add(TEXTURE_PIXEL_COUNT)?;
    let texture = IndexedTexture {
        width: TEXTURE_WIDTH,
        height: TEXTURE_HEIGHT,
        pixels: data.get(texture_start..texture_end)?.to_vec(),
    };
    let trigonometry_start = data_at(TRIGONOMETRY_POSITION)?;
    let trigonometry = checked_array(|index| {
        let position = trigonometry_start + index * TRIGONOMETRY_PAIR_SIZE;
        Some(TrigonometryPair {
            cosine: read_i16(data, position)?,
            sine: read_i16(data, position + size_of::<i16>())?,
        })
    })?;

    let animation_table = usize::from(read_u16(data, data_at(ANIMATION_TABLE_FIELD)?)?);
    let animations = checked_array(|animation| {
        let relative = usize::from(read_u16(
            data,
            data_at(animation_table + animation * size_of::<u16>())?,
        )?);
        let mut position = animation_table.checked_add(relative)?;
        let mut commands = Vec::new();
        loop {
            let record = data_at(position)?;
            let frame_count = *data.get(record)?;
            let phase = *data.get(record + size_of::<u8>())?;
            if frame_count == u8::MIN {
                break;
            }
            commands.push(TweenCommand {
                frame_count,
                phase,
                unused_word: read_u16(data, record + TWEEN_UNUSED_WORD_FIELD)?,
                target: tween_target(usize::from(read_u16(data, record + TWEEN_TARGET_FIELD)?))?,
                end_value: read_i16(data, record + TWEEN_END_VALUE_FIELD)?,
            });
            position = position.checked_add(TWEEN_RECORD_SIZE)?;
        }
        Some(commands)
    })?;

    Some(Manu3Asset {
        root,
        nodes,
        vertices,
        projection_copies,
        faces,
        texture,
        trigonometry,
        animations,
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    const AUTHORED_VERTEX_COUNT: usize = 110;
    const ALIAS_VERTEX_COUNT: usize = 32;
    const FACE_COUNT: usize = 216;
    const DISTINCT_NONEMPTY_ANIMATION_COUNT: usize = 16;
    const NODE_ANIMATED_FIELD_COUNT: usize = AXIS_COUNT * 2;
    const FIRST_TRIGONOMETRY_COSINE: i16 = 16_384;
    const FIRST_TRIGONOMETRY_SINE: i16 = 0;

    fn original_xdb() -> Option<PathBuf> {
        [
            Path::new("output/_tmp_dat/manu3.xdb"),
            Path::new("../../output/_tmp_dat/manu3.xdb"),
            Path::new("commander-blood-audio/_tmp_iso/MANU3.XDB"),
            Path::new("../../commander-blood-audio/_tmp_iso/MANU3.XDB"),
        ]
        .into_iter()
        .find(|path| path.is_file())
        .map(Path::to_owned)
    }

    #[test]
    fn decodes_the_complete_authored_manu3_model() {
        let Some(path) = original_xdb() else {
            return;
        };
        let data = std::fs::read(path).unwrap();
        let asset = decode_manu3(&data).unwrap();

        assert_eq!(asset.nodes.len(), NODE_COUNT);
        assert_eq!(
            asset
                .nodes
                .iter()
                .map(|node| node.vertex_count)
                .sum::<usize>(),
            AUTHORED_VERTEX_COUNT
        );
        assert_eq!(
            asset.vertices.len(),
            AUTHORED_VERTEX_COUNT + ALIAS_VERTEX_COUNT
        );
        assert_eq!(asset.projection_copies.len(), ALIAS_VERTEX_COUNT);
        assert_eq!(asset.faces.len(), FACE_COUNT);
        assert!(asset.faces.iter().all(|face| {
            face.vertices
                .iter()
                .all(|vertex| *vertex < asset.vertices.len())
        }));
        assert_eq!(
            (
                asset.texture.width,
                asset.texture.height,
                asset.texture.pixels.len()
            ),
            (TEXTURE_WIDTH, TEXTURE_HEIGHT, TEXTURE_PIXEL_COUNT)
        );
        assert_eq!(
            asset.trigonometry[usize::MIN],
            TrigonometryPair {
                cosine: FIRST_TRIGONOMETRY_COSINE,
                sine: FIRST_TRIGONOMETRY_SINE,
            }
        );
        assert_eq!(
            asset
                .animations
                .iter()
                .filter(|script| !script.is_empty())
                .count(),
            DISTINCT_NONEMPTY_ANIMATION_COUNT
        );
    }

    #[test]
    fn every_animation_target_is_a_typed_skeleton_field() {
        let Some(path) = original_xdb() else {
            return;
        };
        let data = std::fs::read(path).unwrap();
        let asset = decode_manu3(&data).unwrap();
        let command_count = asset.animations.iter().map(Vec::len).sum::<usize>();
        assert!(command_count > NODE_ANIMATED_FIELD_COUNT);
        for command in asset.animations.iter().flatten() {
            let node = match command.target {
                TweenTarget::NodeLocalPosition { node, .. }
                | TweenTarget::NodeAngle { node, .. } => node,
            };
            assert!(node < asset.nodes.len());
        }
    }

    #[test]
    fn rejects_truncated_or_unrecognized_images() {
        assert!(decode_manu3(&[]).is_none());
        let Some(path) = original_xdb() else {
            return;
        };
        let mut data = std::fs::read(path).unwrap();
        let data_start =
            usize::from(read_u16(&data, DATA_DELTA_FIELD).unwrap()) * PARAGRAPH_BYTE_COUNT;
        data[data_start + MODEL_HEADER_POSITION] = u8::MIN;
        assert!(decode_manu3(&data).is_none());
    }
}
