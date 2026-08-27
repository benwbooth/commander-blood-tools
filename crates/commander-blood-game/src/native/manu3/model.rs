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
use super::raster::{FaceError, ModelFace, RenderTriangle, prepare_render_triangles};

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const LOCAL_POSITION_TARGET_BASE: usize = 0;
const ANGLE_TARGET_BASE: usize = AXIS_COUNT;
const TARGETS_PER_NODE: usize = AXIS_COUNT * 2;
const CURSOR_CENTER_X: i16 = 160;
const CURSOR_CENTER_Y: i16 = 100;
const CURSOR_ANGLE_SCALE: u16 = 2;
const ANIMATION_SELECTOR_MASK: u16 = 0x001f;
const REFERENCE_NODE_INDEX: usize = 3;
const REFERENCE_VERTEX_INDEX: usize = 34;
const DEPTH_FRACTIONAL_BITS: u32 = 8;
const VISIBLE_DEPTH_MINIMUM: i32 = 0;
const LOW_WORD_PRESERVE_MASK: u32 = 0xffff_0000;

/// Per-frame input expected by the recovered MANU3 API coordinator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Manu3FrameRequest {
    /// Cursor position used for temporary hand steering and projection centering.
    pub cursor: CursorPosition,
    /// Nonzero selector starts a new animation before this frame advances.
    pub animation_selector: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameStage {
    Tween,
    Matrix,
    Projection,
    Faces,
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
    /// Face topology is invalid.
    Face(FaceError),
}

impl Display for Manu3ModelError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IncompleteModel => {
                formatter.write_str("MANU3 model is missing its fingertip reference")
            }
            Self::Animation(error) => Display::fmt(error, formatter),
            Self::Geometry(error) => Display::fmt(error, formatter),
            Self::Face(error) => Display::fmt(error, formatter),
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

impl From<FaceError> for Manu3ModelError {
    fn from(error: FaceError) -> Self {
        Self::Face(error)
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
    raster_reciprocals: [i32; commander_blood_formats::manu3::MAXIMUM_FACE_SPAN],
    projection_center: ProjectionCenter,
    render_triangles: Vec<RenderTriangle>,
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
            raster_reciprocals: asset.raster_reciprocals,
            projection_center: ProjectionCenter::default(),
            render_triangles: Vec::new(),
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

    /// Visible textured triangles prepared in native activation order.
    pub fn render_triangles(&self) -> &[RenderTriangle] {
        &self.render_triangles
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
        if let Some(animation_selector) = animation_selector_for_request(request.animation_selector)
        {
            self.animation
                .select_animation(animation_selector, &self.animation_library)?;
        }
        self.animation.step_tweens()?;
        self.apply_animation_targets();

        self.project_current_pose(request.cursor)
    }

    /// Reproject the current authored pose without advancing animation state.
    ///
    /// Modern presentation can call this between recovered C simulation ticks
    /// so the hand follows relative pointer samples at display cadence.
    pub fn reproject_frame(&mut self, cursor: CursorPosition) -> Result<(), Manu3ModelError> {
        self.project_current_pose(cursor)
    }

    fn project_current_pose(&mut self, cursor: CursorPosition) -> Result<(), Manu3ModelError> {
        let saved_view = [
            self.nodes[usize::MIN].angles[X_AXIS],
            self.nodes[usize::MIN].angles[Y_AXIS],
        ];
        let adjusted_view = adjusted_view_angles(saved_view, cursor);
        self.nodes[usize::MIN].angles[X_AXIS] = adjusted_view[X_AXIS];
        self.nodes[usize::MIN].angles[Y_AXIS] = adjusted_view[Y_AXIS];
        build_projection_matrices(
            &mut self.nodes,
            std::slice::from_ref(&self.root),
            &self.trigonometry,
        )?;
        self.nodes[usize::MIN].angles[X_AXIS] = saved_view[X_AXIS];
        self.nodes[usize::MIN].angles[Y_AXIS] = saved_view[Y_AXIS];

        self.update_projection_center(cursor);
        project_entities(
            &self.nodes,
            &mut self.vertices,
            &self.projection_copies,
            self.projection_center,
        )?;
        self.prepare_render_triangles()?;
        Ok(())
    }

    /// Advance the no-request frame path recovered at MANU3 file offset `0x0150`.
    pub fn step_frame(&mut self) -> Result<(), Manu3ModelError> {
        self.step_frame_with_trace(|_| {})
    }

    fn step_frame_with_trace(
        &mut self,
        mut trace: impl FnMut(FrameStage),
    ) -> Result<(), Manu3ModelError> {
        trace(FrameStage::Tween);
        self.animation.step_tweens()?;
        self.apply_animation_targets();
        trace(FrameStage::Matrix);
        build_projection_matrices(
            &mut self.nodes,
            std::slice::from_ref(&self.root),
            &self.trigonometry,
        )?;
        trace(FrameStage::Projection);
        project_entities(
            &self.nodes,
            &mut self.vertices,
            &self.projection_copies,
            self.projection_center,
        )?;
        trace(FrameStage::Faces);
        self.prepare_render_triangles()?;
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
        self.projection_center = projection_center_from_reference(
            reference_node.transform(),
            reference_vertex.position,
            cursor,
            self.projection_center,
        );
    }

    fn prepare_render_triangles(&mut self) -> Result<(), Manu3ModelError> {
        self.render_triangles =
            prepare_render_triangles(&self.vertices, &mut self.faces, &self.raster_reciprocals)?;
        Ok(())
    }
}

fn adjusted_view_angles(saved_view: [u16; 2], cursor: CursorPosition) -> [u16; 2] {
    let pitch_delta = (cursor.y as u16)
        .wrapping_sub(CURSOR_CENTER_Y as u16)
        .wrapping_mul(CURSOR_ANGLE_SCALE);
    let yaw_delta = (cursor.x as u16)
        .wrapping_sub(CURSOR_CENTER_X as u16)
        .wrapping_mul(CURSOR_ANGLE_SCALE);
    [
        saved_view[X_AXIS].wrapping_add(pitch_delta),
        saved_view[Y_AXIS].wrapping_add(yaw_delta),
    ]
}

fn animation_selector_for_request(selector: u16) -> Option<u16> {
    let selector = selector & ANIMATION_SELECTOR_MASK;
    (selector != u16::MIN).then_some(selector)
}

fn projection_center_from_reference(
    transform: Transform3,
    reference: [i16; AXIS_COUNT],
    cursor: CursorPosition,
    current: ProjectionCenter,
) -> ProjectionCenter {
    let transformed = transform_point(&transform, reference);
    let depth = transformed[Z_AXIS] >> DEPTH_FRACTIONAL_BITS;
    if depth <= VISIBLE_DEPTH_MINIMUM {
        return current;
    }
    ProjectionCenter {
        x: i32::from(cursor.x).wrapping_sub(transformed[X_AXIS] / depth),
        y: i32::from(cursor.y).wrapping_add(transformed[Y_AXIS] / depth),
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
    use serde::Deserialize;

    use super::*;

    const EXPECTED_NODE_COUNT: usize = 16;
    const EXPECTED_VERTEX_COUNT: usize = 142;
    const EXPECTED_FACE_COUNT: usize = 216;
    const CENTERED_CURSOR: CursorPosition = CursorPosition { x: 160, y: 100 };

    #[derive(Deserialize)]
    struct ApiCoordinatorVector {
        name: String,
        cursor: Option<[i16; 2]>,
        selector: Option<u16>,
        masked_selector: Option<u16>,
        initial_view: Option<[u16; 2]>,
        adjusted_view: Option<[u16; 2]>,
        restored_view: Option<[u16; 2]>,
        matrix: Option<[[i32; AXIS_COUNT]; AXIS_COUNT]>,
        translation: Option<[i32; AXIS_COUNT]>,
        reference: Option<[i16; AXIS_COUNT]>,
        depth: Option<i32>,
        screen_center_before: Option<[i32; 2]>,
        screen_center: Option<[i32; 2]>,
    }

    #[derive(Deserialize)]
    struct FrameStepVector {
        name: String,
        active_data_segment: u16,
        ordered_callees: Vec<u16>,
    }

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

    #[test]
    fn visual_reprojection_preserves_authored_animation_state() {
        let Some(path) = original_xdb() else {
            return;
        };
        let asset = decode_manu3(&std::fs::read(path).unwrap()).unwrap();
        let mut model = Manu3Model::from_asset(asset).unwrap();
        model
            .render_frame(Manu3FrameRequest {
                cursor: CENTERED_CURSOR,
                animation_selector: 1,
            })
            .unwrap();
        let animation_before = model.animation.clone();
        let pose_before = model
            .nodes
            .iter()
            .map(|node| (node.local_position, node.angles))
            .collect::<Vec<_>>();
        let triangles_before = model.render_triangles.clone();

        model
            .reproject_frame(CursorPosition { x: 240, y: 150 })
            .unwrap();

        assert_eq!(model.animation, animation_before);
        assert_eq!(
            model
                .nodes
                .iter()
                .map(|node| (node.local_position, node.angles))
                .collect::<Vec<_>>(),
            pose_before
        );
        assert_ne!(model.render_triangles, triangles_before);
    }

    #[test]
    fn api_coordinator_matches_all_original_semantic_vectors() {
        let vectors: Vec<ApiCoordinatorVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_manu3_func_0000_natural.json"
        ))
        .unwrap();

        for vector in vectors {
            let Some(cursor) = vector.cursor else {
                assert_eq!(vector.name, "inactive_initializes_overlay");
                continue;
            };
            let cursor = CursorPosition {
                x: cursor[X_AXIS],
                y: cursor[Y_AXIS],
            };
            let selector = vector.selector.unwrap();
            let expected_selector = vector.masked_selector.unwrap();
            assert_eq!(
                animation_selector_for_request(selector),
                (expected_selector != u16::MIN).then_some(expected_selector),
                "{}",
                vector.name
            );
            assert_eq!(
                adjusted_view_angles(vector.initial_view.unwrap(), cursor),
                vector.adjusted_view.unwrap(),
                "{}",
                vector.name
            );
            assert_eq!(
                vector.initial_view.unwrap(),
                vector.restored_view.unwrap(),
                "{}",
                vector.name
            );

            let before = vector.screen_center_before.unwrap();
            let center = projection_center_from_reference(
                Transform3 {
                    matrix: vector.matrix.unwrap(),
                    translation: vector.translation.unwrap(),
                },
                vector.reference.unwrap(),
                cursor,
                ProjectionCenter {
                    x: before[X_AXIS],
                    y: before[Y_AXIS],
                },
            );
            let expected = vector.screen_center.unwrap();
            assert_eq!([center.x, center.y], expected, "{}", vector.name);
            if vector.depth.unwrap() <= VISIBLE_DEPTH_MINIMUM {
                assert_eq!(expected, before, "{}", vector.name);
            }
        }
    }

    #[test]
    fn frame_step_matches_original_active_gate_and_stage_order() {
        const TWEEN_STEP_OFFSET: u16 = 0x019b;
        const MATRIX_BUILD_OFFSET: u16 = 0x0270;
        const ENTITY_PROJECT_OFFSET: u16 = 0x0549;
        const FACE_BUILDER_OFFSET: u16 = 0x06f6;

        let Some(path) = original_xdb() else {
            return;
        };
        let asset = decode_manu3(&std::fs::read(path).unwrap()).unwrap();
        let vectors: Vec<FrameStepVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_manu3_func_0150_natural.json"
        ))
        .unwrap();

        for vector in vectors {
            let mut model = (vector.active_data_segment != u16::MIN)
                .then(|| Manu3Model::from_asset(asset.clone()).unwrap());
            let mut stages = Vec::new();
            if let Some(model) = &mut model {
                model
                    .step_frame_with_trace(|stage| stages.push(stage))
                    .unwrap();
            }
            let callee_offsets = stages
                .into_iter()
                .map(|stage| match stage {
                    FrameStage::Tween => TWEEN_STEP_OFFSET,
                    FrameStage::Matrix => MATRIX_BUILD_OFFSET,
                    FrameStage::Projection => ENTITY_PROJECT_OFFSET,
                    FrameStage::Faces => FACE_BUILDER_OFFSET,
                })
                .collect::<Vec<_>>();
            assert_eq!(callee_offsets, vector.ordered_callees, "{}", vector.name);
        }
    }
}
