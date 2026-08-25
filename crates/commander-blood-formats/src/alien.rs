//! Typed decoder for the AMER, CROOLIS, and SCRUT interactive 3D overlays.
//!
//! Each XDB stores several paragraph-aligned sections and uses 16-bit offsets
//! within those sections. This loader validates that disk representation once
//! and returns owned models with local parent and vertex indices. Relocation
//! values and original pointer fields are not retained in [`AlienAsset`].

use std::collections::HashMap;

/// Number of spatial components and transform rows.
pub const AXIS_COUNT: usize = 3;
/// Number of entries in the overlays' fixed-point trigonometry table.
pub const TRIGONOMETRY_ENTRY_COUNT: usize = 1_024;
/// Number of entries in the face-raster reciprocal table.
pub const RASTER_RECIPROCAL_COUNT: usize = 500;
/// Width of the indexed texture atlas.
pub const TEXTURE_WIDTH: usize = 256;
/// Height of the indexed texture atlas across its two original banks.
pub const TEXTURE_HEIGHT: usize = 512;
/// Number of entries in the texture-index remap table.
pub const PALETTE_REMAP_ENTRY_COUNT: usize = 256;

const AMER_DATA_DELTA_FIELD: usize = 0x3275;
const CROOLIS_DATA_DELTA_FIELD: usize = 0x32e5;
const SCRUT_DATA_DELTA_FIELD: usize = 0x33a5;
const AMER_PALETTE_REMAP_POSITION: usize = 0x049b;
const OTHER_PALETTE_REMAP_POSITION: usize = 0x04dc;
const PARAGRAPH_BYTE_COUNT: usize = 16;
const DIRECTORY_OBJECT_DELTA_FIELD: usize = 0x000c;
const DIRECTORY_TEXTURE_DELTA_FIELD: usize = 0x000e;
const DIRECTORY_RASTER_DELTA_FIELD: usize = 0x0010;
const TRIGONOMETRY_POSITION: usize = 0x0036;
const TRIGONOMETRY_RECORD_SIZE: usize = 4;
const DISPLAY_PALETTE_POSITION: usize = 0x1f6a;
const PALETTE_ENTRY_COUNT: usize = 256;
const RGB_COMPONENT_COUNT: usize = 3;
const PALETTE_BYTE_COUNT: usize = PALETTE_ENTRY_COUNT * RGB_COMPONENT_COUNT;
const VGA_DAC_CHANNEL_MAXIMUM: u16 = 63;
const EIGHT_BIT_CHANNEL_MAXIMUM: u16 = 255;
const METHOD_TABLE_POSITION: usize = 0x103a;
const PRIMARY_CONTEXT_POSITION: usize = 0x2306;
const CONTEXT_LIST_POSITION: usize = 0x2308;
const CONTEXT_LIST_LIMIT: usize = 256;
const SCENE_CAMERA_TRANSFORM_POSITION: usize = 0x22a8;
const CAMERA_MATRIX_POSITION: usize = 0x22ba;
const CAMERA_RESULT_POSITION: usize = 0x22de;
const CAMERA_POSITION_POSITION: usize = 0x22ea;
const CAMERA_ANGLE_POSITION: usize = 0x22f6;
const CAMERA_DEPTH_VELOCITY_POSITION: usize = 0x22fc;
const CAMERA_HORIZONTAL_FILTER_POSITION: usize = 0x1058;
const AMER_STAR_SHADE_TABLE_POSITION: usize = 0x07d4;
const OTHER_STAR_SHADE_TABLE_POSITION: usize = 0x07d6;
const AMER_STAR_SEED_POSITION: usize = 0x08d4;
const OTHER_STAR_SEED_POSITION: usize = 0x08d6;
const STAR_SHADE_TABLE_ENTRY_COUNT: usize = 256;
const MODEL_MAGIC_POSITION: usize = 0x0000;
const MODEL_MAGIC: &[u8; 4] = b"3DB0";
const MODEL_HEADER_SIZE_FIELD: usize = 0x0004;
const MODEL_HEADER_SIZE: u16 = 0x0048;
const MODEL_VERSION_FIELD: usize = 0x0006;
const MODEL_VERSION: [u8; 2] = [1, 2];
const MODEL_NAME_FIELD: usize = 0x0008;
const MODEL_NAME_LENGTH: usize = 8;
const MODEL_ROOT_FIELD: usize = 0x0016;
const MODEL_NODE_COUNT_FIELD: usize = 0x001a;
const PRIMARY_VERTEX_START_FIELD: usize = 0x001c;
const PRIMARY_VERTEX_COUNT_FIELD: usize = 0x0020;
const MODEL_COPY_START_FIELD: usize = 0x0022;
const MODEL_COPY_COUNT_FIELD: usize = 0x0026;
const MODEL_FACE_START_FIELD: usize = 0x0028;
const MODEL_FACE_COUNT_FIELD: usize = 0x002c;
const MODEL_METHOD_TABLE_OFFSET_FIELD: usize = 0x0034;
const TRANSFORM_RECORD_SIZE: usize = 0x005e;
const NODE_PARENT_FIELD: usize = 0x0000;
const NODE_VERTEX_COUNT_FIELD: usize = 0x0002;
const NODE_VERTEX_START_FIELD: usize = 0x0006;
const NODE_MATRIX_FIELD: usize = 0x0012;
const NODE_TRANSLATION_FIELD: usize = 0x0036;
const NODE_LOCAL_POSITION_FIELD: usize = 0x0042;
const NODE_ANGLE_FIELD: usize = 0x004e;
const NODE_RADIAL_OFFSET_FIELD: usize = 0x0054;
const VERTEX_RECORD_SIZE: usize = 20;
const VERTEX_TEXTURE_FIELD: usize = 0x0000;
const VERTEX_POSITION_FIELD: usize = 0x0004;
const VERTEX_SCREEN_FIELD: usize = 0x000a;
const VERTEX_RASTER_DEPTH_FIELD: usize = 0x000e;
const FACE_RECORD_SIZE: usize = 8;
const FACE_FIRST_VERTEX_FIELD: usize = 0x0002;
const METHOD_SLOT_SIZE: usize = 2;
const METHOD_SLOT_NOOP_PRIMARY: usize = 0;
const METHOD_SLOT_WAVE: usize = 1;
const METHOD_SLOT_DISPATCH_PRIMARY: usize = 2;
const METHOD_SLOT_RING: usize = 3;
const METHOD_SLOT_DISPATCH_SECONDARY: usize = 4;
const METHOD_SLOT_NOOP_SECONDARY: usize = 5;
const METHOD_SLOT_WRAP_POSITIONS: usize = 6;
const METHOD_SLOT_PALETTE: usize = 7;
const METHOD_SLOT_SAMPLE_DELTA: usize = 8;
const METHOD_SLOT_SCALED_SAMPLE_DELTA: usize = 9;
const METHOD_SLOT_BOUNDS_WRAP: usize = 10;
const METHOD_SLOT_ANCHOR: usize = 11;
const METHOD_SLOT_ADJUST_STATE: usize = 12;
const METHOD_SLOT_RESUME: usize = 13;
const METHOD_SLOT_NOOP_TERTIARY: usize = 14;
const INVALID_METHOD_ENTRY: u16 = 0xffff;
const ZERO_COORDINATE: i16 = 0;
const ZERO_POSITION: [i16; AXIS_COUNT] = [ZERO_COORDINATE; AXIS_COUNT];

/// Alien overlay format variant being decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienXdbKind {
    /// `AMER.XDB`.
    Amer,
    /// `CROOLIS.XDB`.
    Croolis,
    /// `SCRUT.XDB`.
    Scrut,
}

impl AlienXdbKind {
    fn data_delta_field(self) -> usize {
        match self {
            Self::Amer => AMER_DATA_DELTA_FIELD,
            Self::Croolis => CROOLIS_DATA_DELTA_FIELD,
            Self::Scrut => SCRUT_DATA_DELTA_FIELD,
        }
    }

    fn star_shade_table_position(self) -> usize {
        match self {
            Self::Amer => AMER_STAR_SHADE_TABLE_POSITION,
            Self::Croolis | Self::Scrut => OTHER_STAR_SHADE_TABLE_POSITION,
        }
    }

    fn star_seed_position(self) -> usize {
        match self {
            Self::Amer => AMER_STAR_SEED_POSITION,
            Self::Croolis | Self::Scrut => OTHER_STAR_SEED_POSITION,
        }
    }

    fn palette_remap_position(self) -> usize {
        match self {
            Self::Amer => AMER_PALETTE_REMAP_POSITION,
            Self::Croolis | Self::Scrut => OTHER_PALETTE_REMAP_POSITION,
        }
    }
}

/// Fixed-point cosine and sine pair from an alien XDB.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienTrigonometryPair {
    /// Cosine component.
    pub cosine: i16,
    /// Sine component.
    pub sine: i16,
}

/// Initial row-major transform stored in a model node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienTransformData {
    /// Three-by-three orientation matrix.
    pub matrix: [[i32; AXIS_COUNT]; AXIS_COUNT],
    /// Three-component translation.
    pub translation: [i32; AXIS_COUNT],
}

/// Initial camera and control values authored in one alien overlay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienCameraData {
    /// Initial eased camera orientation matrix.
    pub matrix: [[i32; AXIS_COUNT]; AXIS_COUNT],
    /// Initial fixed-point camera position.
    pub position: [i32; AXIS_COUNT],
    /// Initial matrix-transformed view vector.
    pub transformed_view: [i32; AXIS_COUNT],
    /// Initial pitch, pan, and secondary-pan accumulators.
    pub angles: [i16; AXIS_COUNT],
    /// Initial forward/backward camera velocity.
    pub depth_velocity: i16,
    /// Initial horizontal mouse filter accumulator.
    pub horizontal_filter: i16,
}

/// Typed parent relation in one model hierarchy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienNodeParent {
    /// Shared camera transform synthesized by the scene controller.
    SceneCamera,
    /// Model root transform.
    Root,
    /// Earlier node in the same model.
    Node(usize),
}

/// Authored hierarchical transform and its consecutive vertex range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienNodeData {
    /// Root or earlier node supplying the parent transform.
    pub parent: AlienNodeParent,
    /// First vertex controlled by this node.
    pub first_vertex: usize,
    /// Number of consecutive vertices controlled by this node.
    pub vertex_count: usize,
    /// Initial composed transform.
    pub transform: AlienTransformData,
    /// Mutable local-position accumulators.
    pub local_position: [i32; AXIS_COUNT],
    /// Wrapping Euler angles.
    pub angles: [u16; AXIS_COUNT],
    /// Signed radial displacement.
    pub radial_offset: i16,
}

/// Texture and object-space coordinates for one mesh vertex.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienVertexData {
    /// Texture coordinates in the 256-by-512 atlas.
    pub texture: [i16; 2],
    /// Object-space position; an alias retains zero and uses a projection copy.
    pub position: [i16; AXIS_COUNT],
    /// Initial projected coordinate retained by primary vertices with invalid depth.
    pub initial_screen: [i16; 2],
    /// Authored depth/interpolation value consumed by the face raster stage.
    pub raster_depth: i32,
}

/// Projection sharing for a UV-seam alias vertex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienProjectionCopyData {
    /// Authored vertex supplying projected position and depth.
    pub source: usize,
    /// Alias vertex receiving projection while retaining independent UVs.
    pub destination: usize,
}

/// Three local vertex indices forming one authored triangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienFaceData {
    /// Indices into [`AlienMeshData::vertices`].
    pub vertices: [usize; AXIS_COUNT],
}

/// Owned geometry used by either the primary mesh or a behavior model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienMeshData {
    /// Authored vertices followed by projection aliases.
    pub vertices: Vec<AlienVertexData>,
    /// Projection copies for UV aliases.
    pub projection_copies: Vec<AlienProjectionCopyData>,
    /// Triangle list using local vertex indices.
    pub faces: Vec<AlienFaceData>,
}

/// Named camera-relative mesh rendered before the behavior models.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienPrimaryModelData {
    /// Eight-character authored model name.
    pub name: String,
    /// Primary model geometry.
    pub mesh: AlienMeshData,
}

/// Semantic method selected by a model's recovered dispatch-table slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienBehaviorMethod {
    /// No per-frame behavior.
    NoOperation,
    /// Proximity-selected wave motion.
    Wave,
    /// Species animation dispatch shared by method slots two and four.
    AnimationDispatch,
    /// Ring-driven animation update.
    RingAnimation,
    /// Camera-relative position wrapping.
    WrapPositions,
    /// Palette pulse and cycle update.
    PaletteUpdate,
    /// Apply an unscaled cyclic sample delta.
    ApplySampleDelta,
    /// Apply a distance-scaled cyclic sample delta.
    ApplyScaledSampleDelta,
    /// Bounds test followed by position wrapping.
    BoundsThenWrap,
    /// Publish the selected anchor state.
    AnchorState,
    /// Apply the species-specific state adjustment.
    AdjustState,
    /// Resume a queued behavior state.
    Resume,
}

/// One named hierarchical model and its initial behavior method.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienModelData {
    /// Eight-character authored model name.
    pub name: String,
    /// Root transform used by the first node.
    pub root: AlienTransformData,
    /// Topologically ordered model nodes.
    pub nodes: Vec<AlienNodeData>,
    /// Model mesh and projection aliases.
    pub mesh: AlienMeshData,
    /// Behavior selected by the model's method table slot.
    pub behavior: AlienBehaviorMethod,
}

/// Indexed atlas shared by all models in one alien overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienTextureAtlas {
    /// Atlas width in texels.
    pub width: usize,
    /// Atlas height in texels.
    pub height: usize,
    /// Row-major palette indices.
    pub pixels: Vec<u8>,
}

/// Complete authored resources currently required by the alien 3D renderer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienAsset {
    /// Overlay variant that supplied the resources.
    pub kind: AlienXdbKind,
    /// Camera-relative primary mesh rendered before behavior models.
    pub primary_model: AlienPrimaryModelData,
    /// Null-terminated model/context list in authored dispatch order.
    pub models: Vec<AlienModelData>,
    /// Shared indexed texture atlas.
    pub texture: AlienTextureAtlas,
    /// Expanded 8-bit RGB display palette.
    pub palette: [[u8; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
    /// Authored palette-index substitutions used to animate texture regions.
    pub palette_remap: [u8; PALETTE_REMAP_ENTRY_COUNT],
    /// Fixed-point trigonometry lookup table.
    pub trigonometry: [AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
    /// Fixed-point face-raster reciprocal table.
    pub raster_reciprocals: [i32; RASTER_RECIPROCAL_COUNT],
    /// Initial camera and control values.
    pub camera: AlienCameraData,
    /// Distance-to-palette lookup used by the starfield.
    pub star_shade_table: [u8; STAR_SHADE_TABLE_ENTRY_COUNT],
    /// Deterministic seed used to generate the static star distribution.
    pub star_seed: u32,
}

fn read_u16(data: &[u8], position: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(position..position.checked_add(size_of::<u16>())?)?
            .try_into()
            .ok()?,
    ))
}

fn read_i16(data: &[u8], position: usize) -> Option<i16> {
    Some(i16::from_le_bytes(
        data.get(position..position.checked_add(size_of::<i16>())?)?
            .try_into()
            .ok()?,
    ))
}

fn read_i32(data: &[u8], position: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        data.get(position..position.checked_add(size_of::<i32>())?)?
            .try_into()
            .ok()?,
    ))
}

fn read_u32(data: &[u8], position: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(position..position.checked_add(size_of::<u32>())?)?
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

fn section_start(base: usize, paragraph_delta: u16) -> Option<usize> {
    base.checked_add(usize::from(paragraph_delta).checked_mul(PARAGRAPH_BYTE_COUNT)?)
}

fn transform(data: &[u8], position: usize) -> Option<AlienTransformData> {
    Some(AlienTransformData {
        matrix: checked_array(|row| {
            checked_array(|column| {
                read_i32(
                    data,
                    position + NODE_MATRIX_FIELD + (row * AXIS_COUNT + column) * size_of::<i32>(),
                )
            })
        })?,
        translation: checked_array(|axis| {
            read_i32(
                data,
                position + NODE_TRANSLATION_FIELD + axis * size_of::<i32>(),
            )
        })?,
    })
}

fn model_header(data: &[u8], position: usize) -> Option<String> {
    if data.get(position + MODEL_MAGIC_POSITION..position + MODEL_MAGIC.len())? != MODEL_MAGIC
        || read_u16(data, position + MODEL_HEADER_SIZE_FIELD)? != MODEL_HEADER_SIZE
        || data.get(
            position + MODEL_VERSION_FIELD..position + MODEL_VERSION_FIELD + MODEL_VERSION.len(),
        )? != MODEL_VERSION
    {
        return None;
    }
    let name =
        data.get(position + MODEL_NAME_FIELD..position + MODEL_NAME_FIELD + MODEL_NAME_LENGTH)?;
    let name = std::str::from_utf8(name).ok()?.trim_end_matches('\0');
    (!name.is_empty()).then(|| name.to_owned())
}

fn behavior_method(method_table_offset: u16) -> Option<AlienBehaviorMethod> {
    let offset = usize::from(method_table_offset);
    if offset % METHOD_SLOT_SIZE != usize::MIN {
        return None;
    }
    match offset / METHOD_SLOT_SIZE {
        METHOD_SLOT_NOOP_PRIMARY | METHOD_SLOT_NOOP_SECONDARY | METHOD_SLOT_NOOP_TERTIARY => {
            Some(AlienBehaviorMethod::NoOperation)
        }
        METHOD_SLOT_WAVE => Some(AlienBehaviorMethod::Wave),
        METHOD_SLOT_DISPATCH_PRIMARY | METHOD_SLOT_DISPATCH_SECONDARY => {
            Some(AlienBehaviorMethod::AnimationDispatch)
        }
        METHOD_SLOT_RING => Some(AlienBehaviorMethod::RingAnimation),
        METHOD_SLOT_WRAP_POSITIONS => Some(AlienBehaviorMethod::WrapPositions),
        METHOD_SLOT_PALETTE => Some(AlienBehaviorMethod::PaletteUpdate),
        METHOD_SLOT_SAMPLE_DELTA => Some(AlienBehaviorMethod::ApplySampleDelta),
        METHOD_SLOT_SCALED_SAMPLE_DELTA => Some(AlienBehaviorMethod::ApplyScaledSampleDelta),
        METHOD_SLOT_BOUNDS_WRAP => Some(AlienBehaviorMethod::BoundsThenWrap),
        METHOD_SLOT_ANCHOR => Some(AlienBehaviorMethod::AnchorState),
        METHOD_SLOT_ADJUST_STATE => Some(AlienBehaviorMethod::AdjustState),
        METHOD_SLOT_RESUME => Some(AlienBehaviorMethod::Resume),
        _ => None,
    }
}

fn vertex(data: &[u8], object_start: usize, offset: usize) -> Option<AlienVertexData> {
    let position = object_start.checked_add(offset)?;
    Some(AlienVertexData {
        texture: checked_array(|axis| {
            read_i16(
                data,
                position + VERTEX_TEXTURE_FIELD + axis * size_of::<i16>(),
            )
        })?,
        position: checked_array(|axis| {
            read_i16(
                data,
                position + VERTEX_POSITION_FIELD + axis * size_of::<i16>(),
            )
        })?,
        initial_screen: checked_array(|axis| {
            read_i16(
                data,
                position + VERTEX_SCREEN_FIELD + axis * size_of::<i16>(),
            )
        })?,
        raster_depth: read_i32(data, position + VERTEX_RASTER_DEPTH_FIELD)?,
    })
}

fn faces(
    data: &[u8],
    object_start: usize,
    face_start: usize,
    face_count: usize,
    vertex_indices: &HashMap<usize, usize>,
) -> Option<Vec<AlienFaceData>> {
    let mut faces = Vec::with_capacity(face_count);
    for face_index in 0..face_count {
        let position = object_start
            .checked_add(face_start)?
            .checked_add(face_index.checked_mul(FACE_RECORD_SIZE)?)?;
        faces.push(AlienFaceData {
            vertices: checked_array(|corner| {
                let offset = usize::from(read_u16(
                    data,
                    position + FACE_FIRST_VERTEX_FIELD + corner * size_of::<u16>(),
                )?);
                vertex_indices.get(&offset).copied()
            })?,
        });
    }
    Some(faces)
}

fn primary_model(
    data: &[u8],
    data_start: usize,
    object_start: usize,
) -> Option<AlienPrimaryModelData> {
    let context_offset = usize::from(read_u16(data, data_start + PRIMARY_CONTEXT_POSITION)?);
    let context = data_start.checked_add(context_offset)?;
    let name = model_header(data, context)?;
    let vertex_start = usize::from(read_u16(data, context + PRIMARY_VERTEX_START_FIELD)?);
    let vertex_count = usize::from(read_u16(data, context + PRIMARY_VERTEX_COUNT_FIELD)?);
    let face_start = usize::from(read_u16(data, context + MODEL_FACE_START_FIELD)?);
    let face_count = usize::from(read_u16(data, context + MODEL_FACE_COUNT_FIELD)?);
    if vertex_count == usize::MIN || face_count == usize::MIN {
        return None;
    }

    let mut vertices = Vec::with_capacity(vertex_count);
    let mut vertex_indices = HashMap::with_capacity(vertex_count);
    for index in 0..vertex_count {
        let offset = vertex_start.checked_add(index.checked_mul(VERTEX_RECORD_SIZE)?)?;
        vertex_indices.insert(offset, index);
        vertices.push(vertex(data, object_start, offset)?);
    }
    Some(AlienPrimaryModelData {
        name,
        mesh: AlienMeshData {
            vertices,
            projection_copies: Vec::new(),
            faces: faces(data, object_start, face_start, face_count, &vertex_indices)?,
        },
    })
}

fn model(
    data: &[u8],
    data_start: usize,
    object_start: usize,
    offset: usize,
) -> Option<AlienModelData> {
    let context = data_start.checked_add(offset)?;
    let name = model_header(data, context)?;
    let root_offset = usize::from(read_u16(data, context + MODEL_ROOT_FIELD)?);
    let root_position = data_start.checked_add(root_offset)?;
    let node_count = usize::from(read_u16(data, context + MODEL_NODE_COUNT_FIELD)?);
    if node_count == usize::MIN {
        return None;
    }
    let root = transform(data, root_position)?;

    let mut vertices = Vec::new();
    let mut vertex_indices = HashMap::new();
    let mut nodes = Vec::with_capacity(node_count);
    let mut node_indices = HashMap::with_capacity(node_count);
    for node_index in 0..node_count {
        let node_offset = root_offset
            .checked_add(TRANSFORM_RECORD_SIZE)?
            .checked_add(node_index.checked_mul(TRANSFORM_RECORD_SIZE)?)?;
        let position = data_start.checked_add(node_offset)?;
        let parent_offset = usize::from(read_u16(data, position + NODE_PARENT_FIELD)?);
        let parent = match parent_offset {
            SCENE_CAMERA_TRANSFORM_POSITION => AlienNodeParent::SceneCamera,
            _ if parent_offset == root_offset => AlienNodeParent::Root,
            _ => AlienNodeParent::Node(*node_indices.get(&parent_offset)?),
        };
        let first_vertex = vertices.len();
        let vertex_count = usize::from(read_u16(data, position + NODE_VERTEX_COUNT_FIELD)?);
        if vertex_count == usize::MIN {
            return None;
        }
        let vertex_start = usize::from(read_u16(data, position + NODE_VERTEX_START_FIELD)?);
        for vertex_index in 0..vertex_count {
            let vertex_offset =
                vertex_start.checked_add(vertex_index.checked_mul(VERTEX_RECORD_SIZE)?)?;
            if vertex_indices
                .insert(vertex_offset, vertices.len())
                .is_some()
            {
                return None;
            }
            vertices.push(vertex(data, object_start, vertex_offset)?);
        }
        nodes.push(AlienNodeData {
            parent,
            first_vertex,
            vertex_count,
            transform: transform(data, position)?,
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
        node_indices.insert(node_offset, node_index);
    }

    let copy_start = usize::from(read_u16(data, context + MODEL_COPY_START_FIELD)?);
    let copy_count = usize::from(read_u16(data, context + MODEL_COPY_COUNT_FIELD)?);
    let mut projection_copies = Vec::with_capacity(copy_count);
    for copy_index in 0..copy_count {
        let copy_offset = copy_start.checked_add(copy_index.checked_mul(VERTEX_RECORD_SIZE)?)?;
        let position = object_start.checked_add(copy_offset)?;
        let source_offset = usize::from(read_u16(data, position + VERTEX_POSITION_FIELD)?);
        let source = *vertex_indices.get(&source_offset)?;
        let destination = vertices.len();
        if vertex_indices.insert(copy_offset, destination).is_some() {
            return None;
        }
        let mut alias = vertex(data, object_start, copy_offset)?;
        alias.position = ZERO_POSITION;
        vertices.push(alias);
        projection_copies.push(AlienProjectionCopyData {
            source,
            destination,
        });
    }

    let face_start = usize::from(read_u16(data, context + MODEL_FACE_START_FIELD)?);
    let face_count = usize::from(read_u16(data, context + MODEL_FACE_COUNT_FIELD)?);
    let behavior = behavior_method(read_u16(data, context + MODEL_METHOD_TABLE_OFFSET_FIELD)?)?;
    let method_slot = usize::from(read_u16(data, context + MODEL_METHOD_TABLE_OFFSET_FIELD)?);
    if read_u16(data, data_start + METHOD_TABLE_POSITION + method_slot)? == INVALID_METHOD_ENTRY {
        return None;
    }

    Some(AlienModelData {
        name,
        root,
        nodes,
        mesh: AlienMeshData {
            vertices,
            projection_copies,
            faces: faces(data, object_start, face_start, face_count, &vertex_indices)?,
        },
        behavior,
    })
}

/// Decode one original alien XDB into flat, typed authored resources.
///
/// Returns `None` if section bounds, model headers, hierarchy topology, method
/// slots, vertex references, palette values, or texture/raster extents are invalid.
pub fn decode_alien_xdb(data: &[u8], kind: AlienXdbKind) -> Option<AlienAsset> {
    let data_delta = read_u16(data, kind.data_delta_field())?;
    let data_start = usize::from(data_delta).checked_mul(PARAGRAPH_BYTE_COUNT)?;
    let object_start = section_start(
        data_start,
        read_u16(data, data_start + DIRECTORY_OBJECT_DELTA_FIELD)?,
    )?;
    let texture_start = section_start(
        object_start,
        read_u16(data, data_start + DIRECTORY_TEXTURE_DELTA_FIELD)?,
    )?;
    let raster_start = section_start(
        texture_start,
        read_u16(data, data_start + DIRECTORY_RASTER_DELTA_FIELD)?,
    )?;
    let texture_pixel_count = TEXTURE_WIDTH.checked_mul(TEXTURE_HEIGHT)?;
    if texture_start.checked_add(texture_pixel_count)? != raster_start {
        return None;
    }

    let primary_model = primary_model(data, data_start, object_start)?;
    let mut models = Vec::new();
    let mut seen_contexts = HashMap::new();
    for index in 0..CONTEXT_LIST_LIMIT {
        let list_position = data_start
            .checked_add(CONTEXT_LIST_POSITION)?
            .checked_add(index.checked_mul(size_of::<u16>())?)?;
        let context_offset = usize::from(read_u16(data, list_position)?);
        if context_offset == usize::MIN {
            break;
        }
        if seen_contexts.insert(context_offset, index).is_some() {
            return None;
        }
        models.push(model(data, data_start, object_start, context_offset)?);
    }
    if models.is_empty() || models.len() == CONTEXT_LIST_LIMIT {
        return None;
    }

    let palette_bytes = data.get(
        data_start + DISPLAY_PALETTE_POSITION
            ..data_start + DISPLAY_PALETTE_POSITION + PALETTE_BYTE_COUNT,
    )?;
    let mut palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
    for (entry, source) in palette
        .iter_mut()
        .zip(palette_bytes.chunks_exact(RGB_COMPONENT_COUNT))
    {
        for (component, value) in entry.iter_mut().zip(source) {
            if u16::from(*value) > VGA_DAC_CHANNEL_MAXIMUM {
                return None;
            }
            *component =
                (u16::from(*value) * EIGHT_BIT_CHANNEL_MAXIMUM / VGA_DAC_CHANNEL_MAXIMUM) as u8;
        }
    }

    let trigonometry = checked_array(|index| {
        let position = data_start + TRIGONOMETRY_POSITION + index * TRIGONOMETRY_RECORD_SIZE;
        Some(AlienTrigonometryPair {
            cosine: read_i16(data, position)?,
            sine: read_i16(data, position + size_of::<i16>())?,
        })
    })?;
    let raster_reciprocals = checked_array(|index| {
        read_i32(
            data,
            raster_start.checked_add(index.checked_mul(size_of::<i32>())?)?,
        )
    })?;
    let camera = AlienCameraData {
        matrix: checked_array(|row| {
            checked_array(|column| {
                read_i32(
                    data,
                    data_start
                        + CAMERA_MATRIX_POSITION
                        + (row * AXIS_COUNT + column) * size_of::<i32>(),
                )
            })
        })?,
        position: checked_array(|axis| {
            read_i32(
                data,
                data_start + CAMERA_POSITION_POSITION + axis * size_of::<i32>(),
            )
        })?,
        transformed_view: checked_array(|axis| {
            read_i32(
                data,
                data_start + CAMERA_RESULT_POSITION + axis * size_of::<i32>(),
            )
        })?,
        angles: checked_array(|axis| {
            read_i16(
                data,
                data_start + CAMERA_ANGLE_POSITION + axis * size_of::<i16>(),
            )
        })?,
        depth_velocity: read_i16(data, data_start + CAMERA_DEPTH_VELOCITY_POSITION)?,
        horizontal_filter: read_i16(data, data_start + CAMERA_HORIZONTAL_FILTER_POSITION)?,
    };
    let star_shade_table = checked_array(|index| {
        data.get(raster_start + kind.star_shade_table_position() + index)
            .copied()
    })?;
    let star_seed = read_u32(data, raster_start + kind.star_seed_position())?;
    let palette_remap =
        checked_array(|index| data.get(kind.palette_remap_position() + index).copied())?;

    Some(AlienAsset {
        kind,
        primary_model,
        models,
        texture: AlienTextureAtlas {
            width: TEXTURE_WIDTH,
            height: TEXTURE_HEIGHT,
            pixels: data
                .get(texture_start..texture_start + texture_pixel_count)?
                .to_vec(),
        },
        palette,
        palette_remap,
        trigonometry,
        raster_reciprocals,
        camera,
        star_shade_table,
        star_seed,
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    const EXPECTED_PRIMARY_VERTEX_COUNT: usize = 83;
    const EXPECTED_PRIMARY_FACE_COUNT: usize = 43;
    const EXPECTED_AMER_MODEL_COUNT: usize = 19;
    const EXPECTED_CROOLIS_MODEL_COUNT: usize = 15;
    const EXPECTED_SCRUT_MODEL_COUNT: usize = 14;
    const ZERO_RASTER_DEPTH: i32 = 0;

    fn original_xdb(name: &str) -> Option<PathBuf> {
        [
            Path::new("output/_tmp_dat").join(name),
            Path::new("../../output/_tmp_dat").join(name),
        ]
        .into_iter()
        .find(|path| path.is_file())
    }

    #[test]
    fn decodes_every_original_alien_scene_into_owned_models() {
        let cases = [
            (AlienXdbKind::Amer, "amer.xdb", EXPECTED_AMER_MODEL_COUNT),
            (
                AlienXdbKind::Croolis,
                "croolis.xdb",
                EXPECTED_CROOLIS_MODEL_COUNT,
            ),
            (AlienXdbKind::Scrut, "scrut.xdb", EXPECTED_SCRUT_MODEL_COUNT),
        ];

        let mut shared_palette_remap = None;
        for (kind, filename, expected_models) in cases {
            let Some(path) = original_xdb(filename) else {
                continue;
            };
            let data = std::fs::read(path).unwrap();
            let asset = decode_alien_xdb(&data, kind).unwrap();
            assert_eq!(asset.kind, kind);
            assert_eq!(asset.models.len(), expected_models);
            assert_eq!(
                asset.primary_model.mesh.vertices.len(),
                EXPECTED_PRIMARY_VERTEX_COUNT
            );
            assert!(!asset.primary_model.name.is_empty());
            assert_eq!(
                asset.primary_model.mesh.faces.len(),
                EXPECTED_PRIMARY_FACE_COUNT
            );
            assert!(
                asset
                    .primary_model
                    .mesh
                    .vertices
                    .iter()
                    .any(|vertex| vertex.raster_depth != ZERO_RASTER_DEPTH)
            );
            assert_eq!(asset.texture.pixels.len(), TEXTURE_WIDTH * TEXTURE_HEIGHT);
            assert!(
                asset
                    .palette
                    .iter()
                    .flatten()
                    .any(|component| *component != u8::MIN)
            );
            assert!(
                asset
                    .palette_remap
                    .iter()
                    .enumerate()
                    .any(|(index, entry)| usize::from(*entry) != index)
            );
            if let Some(expected) = shared_palette_remap {
                assert_eq!(asset.palette_remap, expected);
            } else {
                shared_palette_remap = Some(asset.palette_remap);
            }
            assert!(
                asset
                    .raster_reciprocals
                    .iter()
                    .any(|value| *value != i32::MIN)
            );
            for model in &asset.models {
                assert!(!model.name.is_empty());
                assert!(!model.nodes.is_empty());
                assert!(!model.mesh.vertices.is_empty());
                assert!(!model.mesh.faces.is_empty());
            }
        }
    }

    #[test]
    fn rejects_truncated_and_mismatched_alien_images() {
        assert!(decode_alien_xdb(&[], AlienXdbKind::Amer).is_none());
        let Some(path) = original_xdb("amer.xdb") else {
            return;
        };
        let data = std::fs::read(path).unwrap();
        assert!(decode_alien_xdb(&data, AlienXdbKind::Croolis).is_none());
        assert!(decode_alien_xdb(&data[..data.len() / 2], AlienXdbKind::Amer).is_none());
    }
}
