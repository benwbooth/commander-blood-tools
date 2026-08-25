//! End-to-end typed state for the MANU3 skeletal hand model.

use std::error::Error;
use std::fmt::{Display, Formatter};

use commander_blood_formats::manu3::{
    ANIMATION_COUNT, AXIS_COUNT, IndexedTexture, Manu3Asset, NodeParent, TweenTarget,
};

use super::animation::{
    AnimationError, AnimationLibrary, CursorPosition, Manu3Animation, TweenScript,
    TweenSpecification,
};
use super::geometry::{
    GeometryError, ModelVertex, ProjectionCenter, ProjectionCopy, ProjectionNode, Transform3,
    TransformParent, TrigonometryPair, TrigonometryTable, build_projection_matrices,
    project_entities, transform_point,
};

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const LOCAL_POSITION_TARGET_BASE: usize = 0;
const ANGLE_TARGET_BASE: usize = AXIS_COUNT;
const TARGETS_PER_NODE: usize = AXIS_COUNT * 2;
const CURSOR_CENTER_X: i16 = 160;
const CURSOR_CENTER_Y: i16 = 100;
const CURSOR_ANGLE_SCALE: u16 = 2;
const REFERENCE_NODE_INDEX: usize = 3;
const REFERENCE_VERTEX_INDEX: usize = 34;
const DEPTH_FRACTIONAL_BITS: u32 = 8;
const VISIBLE_DEPTH_MINIMUM: i32 = 0;
const LOW_WORD_PRESERVE_MASK: u32 = 0xffff_0000;

/// One triangle retained from the original MANU3 face list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelFace {
    /// Three indices into [`Manu3Model::vertices`].
    pub vertices: [usize; AXIS_COUNT],
}

/// Per-frame input expected by the recovered MANU3 API coordinator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Manu3FrameRequest {
    /// Cursor position used for temporary hand steering and projection centering.
    pub cursor: CursorPosition,
    /// Nonzero selector starts a new animation before this frame advances.
    pub animation_selector: u16,
}

/// Invalid authored data or state encountered by the typed MANU3 runtime.
#[derive(Debug)]
pub enum Manu3ModelError {
    /// The decoded resource lacks the reference node or fingertip vertex used by
    /// the original coordinator to establish its projection center.
    IncompleteModel,
    /// Animation state could not be advanced.
    Animation(AnimationError),
    /// Hierarchy or geometry indices are invalid.
    Geometry(GeometryError),
}

impl Display for Manu3ModelError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IncompleteModel => {
                formatter.write_str("MANU3 model is missing its fingertip reference")
            }
            Self::Animation(error) => Display::fmt(error, formatter),
            Self::Geometry(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for Manu3ModelError {}

impl From<AnimationError> for Manu3ModelError {
    fn from(error: AnimationError) -> Self {
        Self::Animation(error)
    }
}

impl From<GeometryError> for Manu3ModelError {
    fn from(error: GeometryError) -> Self {
        Self::Geometry(error)
    }
}

/// Flat-memory MANU3 skeleton, animation state, geometry, and authored texture.
pub struct Manu3Model {
    animation: Manu3Animation,
    animation_library: AnimationLibrary,
    root: Transform3,
    nodes: Vec<ProjectionNode>,
    vertices: Vec<ModelVertex>,
    projection_copies: Vec<ProjectionCopy>,
    faces: Vec<ModelFace>,
    texture: IndexedTexture,
    trigonometry: TrigonometryTable,
    projection_center: ProjectionCenter,
}

impl Manu3Model {
    /// Convert a validated XDB asset into live, typed runtime state.
    pub fn from_asset(asset: Manu3Asset) -> Result<Self, Manu3ModelError> {
        if asset.nodes.get(REFERENCE_NODE_INDEX).is_none()
            || asset.vertices.get(REFERENCE_VERTEX_INDEX).is_none()
        {
            return Err(Manu3ModelError::IncompleteModel);
        }

        let root = Transform3 {
            matrix: asset.root.matrix,
            translation: asset.root.translation,
        };
        let nodes: Vec<ProjectionNode> = asset
            .nodes
            .iter()
            .map(|node| ProjectionNode {
                parent: match node.parent {
                    NodeParent::Root => TransformParent::Root(usize::MIN),
                    NodeParent::Node(parent) => TransformParent::Node(parent),
                },
                vertices: node.first_vertex..node.first_vertex + node.vertex_count,
                matrix: node.transform.matrix,
                translation: node.transform.translation,
                local_position: node.local_position,
                angles: node.angles,
                radial_offset: node.radial_offset,
            })
            .collect();
        let vertices = asset
            .vertices
            .iter()
            .map(|vertex| ModelVertex {
                texture: vertex.texture,
                position: vertex.position,
                ..ModelVertex::default()
            })
            .collect();
        let projection_copies = asset
            .projection_copies
            .iter()
            .map(|projection_copy| ProjectionCopy {
                source: projection_copy.source,
                destination: projection_copy.destination,
            })
            .collect();
        let faces = asset
            .faces
            .iter()
            .map(|face| ModelFace {
                vertices: face.vertices,
            })
            .collect();
        let trigonometry =
            TrigonometryTable::new(asset.trigonometry.map(|pair| TrigonometryPair {
                cosine: pair.cosine,
                sine: pair.sine,
            }));

        let initial_targets = nodes
            .iter()
            .flat_map(|node| {
                node.local_position
                    .map(|position| position as i16)
                    .into_iter()
                    .chain(node.angles.map(|angle| angle as i16))
            })
            .collect::<Vec<_>>();
        let animation_library = AnimationLibrary::new(asset.animations.map(|commands| {
            TweenScript::new(
                commands
                    .into_iter()
                    .map(|command| {
                        TweenSpecification::new(
                            command.frame_count,
                            command.phase,
                            target_index(command.target),
                            command.end_value,
                        )
                    })
                    .collect(),
            )
        }));
        let animation = Manu3Animation::new(initial_targets.len(), initial_targets);

        Ok(Self {
            animation,
            animation_library,
            root,
            nodes,
            vertices,
            projection_copies,
            faces,
            texture: asset.texture,
            trigonometry,
            projection_center: ProjectionCenter::default(),
        })
    }

    /// Current transformed and projected model vertices.
    pub fn vertices(&self) -> &[ModelVertex] {
        &self.vertices
    }

    /// Authored triangle topology.
    pub fn faces(&self) -> &[ModelFace] {
        &self.faces
    }

    /// Original indexed hand texture ready for GPU upload.
    pub const fn texture(&self) -> &IndexedTexture {
        &self.texture
    }

    /// Projection center calculated from the fingertip reference point.
    pub const fn projection_center(&self) -> ProjectionCenter {
        self.projection_center
    }

    /// Run the recovered animation, transform, centering, and projection order
    /// used by MANU3's main API entry at file offset `0x0000`.
    pub fn render_frame(&mut self, request: Manu3FrameRequest) -> Result<(), Manu3ModelError> {
        self.prepare_animation(request.cursor);
        if request.animation_selector != u16::MIN {
            self.animation
                .select_animation(request.animation_selector, &self.animation_library)?;
        }
        self.animation.step_tweens()?;
        self.apply_animation_targets();

        let saved_pitch = self.nodes[usize::MIN].angles[X_AXIS];
        let saved_yaw = self.nodes[usize::MIN].angles[Y_AXIS];
        let pitch_delta = (request.cursor.y as u16)
            .wrapping_sub(CURSOR_CENTER_Y as u16)
            .wrapping_mul(CURSOR_ANGLE_SCALE);
        let yaw_delta = (request.cursor.x as u16)
            .wrapping_sub(CURSOR_CENTER_X as u16)
            .wrapping_mul(CURSOR_ANGLE_SCALE);
        self.nodes[usize::MIN].angles[X_AXIS] = saved_pitch.wrapping_add(pitch_delta);
        self.nodes[usize::MIN].angles[Y_AXIS] = saved_yaw.wrapping_add(yaw_delta);
        build_projection_matrices(
            &mut self.nodes,
            std::slice::from_ref(&self.root),
            &self.trigonometry,
        )?;
        self.nodes[usize::MIN].angles[X_AXIS] = saved_pitch;
        self.nodes[usize::MIN].angles[Y_AXIS] = saved_yaw;

        self.update_projection_center(request.cursor);
        project_entities(
            &self.nodes,
            &mut self.vertices,
            &self.projection_copies,
            self.projection_center,
        )?;
        Ok(())
    }

    /// Advance the no-request frame path recovered at MANU3 file offset `0x0150`.
    pub fn step_frame(&mut self) -> Result<(), Manu3ModelError> {
        self.animation.step_tweens()?;
        self.apply_animation_targets();
        build_projection_matrices(
            &mut self.nodes,
            std::slice::from_ref(&self.root),
            &self.trigonometry,
        )?;
        project_entities(
            &self.nodes,
            &mut self.vertices,
            &self.projection_copies,
            self.projection_center,
        )?;
        Ok(())
    }

    fn prepare_animation(&mut self, cursor: CursorPosition) {
        self.animation.set_camera_input(
            cursor,
            self.nodes[usize::MIN].angles[X_AXIS],
            self.nodes[usize::MIN].angles[Y_AXIS],
        );
    }

    fn apply_animation_targets(&mut self) {
        for (node_index, node) in self.nodes.iter_mut().enumerate() {
            let target_base = node_index * TARGETS_PER_NODE;
            for axis in usize::MIN..AXIS_COUNT {
                let value =
                    self.animation.targets()[target_base + LOCAL_POSITION_TARGET_BASE + axis];
                node.local_position[axis] = ((node.local_position[axis] as u32
                    & LOW_WORD_PRESERVE_MASK)
                    | u32::from(value as u16)) as i32;
                node.angles[axis] =
                    self.animation.targets()[target_base + ANGLE_TARGET_BASE + axis] as u16;
            }
        }
    }

    fn update_projection_center(&mut self, cursor: CursorPosition) {
        let reference_node = &self.nodes[REFERENCE_NODE_INDEX];
        let reference_vertex = self.vertices[REFERENCE_VERTEX_INDEX];
        let transformed = transform_point(&reference_node.transform(), reference_vertex.position);
        let depth = transformed[Z_AXIS] >> DEPTH_FRACTIONAL_BITS;
        if depth <= VISIBLE_DEPTH_MINIMUM {
            return;
        }
        self.projection_center.y = i32::from(cursor.y).wrapping_add(transformed[Y_AXIS] / depth);
        self.projection_center.x = i32::from(cursor.x).wrapping_sub(transformed[X_AXIS] / depth);
    }
}

fn target_index(target: TweenTarget) -> usize {
    match target {
        TweenTarget::NodeLocalPosition { node, axis } => {
            node * TARGETS_PER_NODE + LOCAL_POSITION_TARGET_BASE + axis.index()
        }
        TweenTarget::NodeAngle { node, axis } => {
            node * TARGETS_PER_NODE + ANGLE_TARGET_BASE + axis.index()
        }
    }
}

const _: () = assert!(ANIMATION_COUNT == super::animation::ANIMATION_SEQUENCE_COUNT);

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::manu3::decode_manu3;

    use super::*;

    const EXPECTED_NODE_COUNT: usize = 16;
    const EXPECTED_VERTEX_COUNT: usize = 142;
    const EXPECTED_FACE_COUNT: usize = 216;
    const CENTERED_CURSOR: CursorPosition = CursorPosition { x: 160, y: 100 };

    fn original_xdb() -> Option<PathBuf> {
        [
            Path::new("output/_tmp_dat/manu3.xdb"),
            Path::new("../../output/_tmp_dat/manu3.xdb"),
        ]
        .into_iter()
        .find(|path| path.is_file())
        .map(Path::to_owned)
    }

    #[test]
    fn decoded_asset_runs_through_animation_transform_and_projection() {
        let Some(path) = original_xdb() else {
            return;
        };
        let asset = decode_manu3(&std::fs::read(path).unwrap()).unwrap();
        let mut model = Manu3Model::from_asset(asset).unwrap();
        assert_eq!(model.nodes.len(), EXPECTED_NODE_COUNT);
        assert_eq!(model.vertices().len(), EXPECTED_VERTEX_COUNT);
        assert_eq!(model.faces().len(), EXPECTED_FACE_COUNT);

        model
            .render_frame(Manu3FrameRequest {
                cursor: CENTERED_CURSOR,
                animation_selector: u16::MIN,
            })
            .unwrap();

        assert!(
            model
                .vertices()
                .iter()
                .any(|vertex| vertex.projected.clip_flags.bits()
                    != super::super::geometry::ClipFlags::BEHIND_CAMERA.bits())
        );
        for projection_copy in &model.projection_copies {
            assert_eq!(
                model.vertices[projection_copy.destination].projected,
                model.vertices[projection_copy.source].projected
            );
        }
    }

    #[test]
    fn authored_animation_changes_typed_skeleton_state() {
        const ACTIVE_ANIMATION_SELECTOR: u16 = 1;
        const MAXIMUM_TEST_FRAMES: usize = 24;

        let Some(path) = original_xdb() else {
            return;
        };
        let asset = decode_manu3(&std::fs::read(path).unwrap()).unwrap();
        let mut model = Manu3Model::from_asset(asset).unwrap();
        let before = model.nodes.clone();
        for frame in usize::MIN..MAXIMUM_TEST_FRAMES {
            model
                .render_frame(Manu3FrameRequest {
                    cursor: CENTERED_CURSOR,
                    animation_selector: if frame == usize::MIN {
                        ACTIVE_ANIMATION_SELECTOR
                    } else {
                        u16::MIN
                    },
                })
                .unwrap();
        }
        assert_ne!(model.nodes, before);
    }
}
