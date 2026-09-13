//! Runtime decoding and flat rendering for Big Bug Bang's simulation overview.

use anyhow::{Context, Result, bail};
use commander_blood_formats::code::ScriptDialect;
use commander_blood_formats::script::{
    ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateObjectReference,
};

use crate::native::bloodprg::{
    BridgeSpriteRect, FontPoint, FontVerticalBand, GameLifecycleState, NavigationChartInputState,
    RasterPoint, RasterSpanPaint, ScriptObjectFlag, SequelOverviewActor, SequelOverviewControl,
    SequelOverviewDraw, SequelOverviewOutcome, SequelOverviewState, draw_horizontal_span,
    draw_line_segment, draw_planar_square_caps_text_clipped, draw_rect_outline, draw_vertical_span,
    fill_framebuffer_rect, object_has_flag, resolve_navigation_position, update_sequel_overview,
};

use super::{
    LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_WIDTH, ModernGameServices, OriginalGameRuntime,
};

const ACTOR_FLAGS_OFFSET: usize = 2;
const ACTOR_GROUP_OFFSET: usize = 20;
const ACTOR_QUANTITY_OFFSET: usize = 22;
const ACTOR_HOLDER_OFFSET: usize = 24;
const ACTOR_BALANCE_OFFSET: usize = 52;
const ACTOR_OPPONENT_OFFSET: usize = 72;
const ACTOR_CONFLICT_FLAG: u16 = 8;
const FULL_DISPLAY_CLIP: BridgeSpriteRect = BridgeSpriteRect {
    left: 0,
    right: LOGICAL_FRAMEBUFFER_WIDTH as i32,
    top: 0,
    bottom: LOGICAL_FRAMEBUFFER_HEIGHT as i32,
};
const FULL_FONT_BAND: FontVerticalBand = FontVerticalBand {
    top: 0,
    bottom: LOGICAL_FRAMEBUFFER_HEIGHT as i32 - 1,
};
const TRASHLANDO_NAME: &[u8] = b"Trashlando";

#[derive(Default)]
pub(super) struct RuntimeSequelOverview {
    state: SequelOverviewState,
}

impl RuntimeSequelOverview {
    pub(super) const fn active(&self) -> bool {
        self.state.active
    }

    pub(super) fn set_active(&mut self, active: bool) {
        self.state.active = active;
    }

    pub(super) fn update(
        &mut self,
        services: &mut ModernGameServices<'_>,
        lifecycle: &mut GameLifecycleState,
        input: &mut NavigationChartInputState,
    ) -> Result<SequelOverviewOutcome> {
        if !self.state.active && !lifecycle.secondary_pointer_pressed {
            return Ok(SequelOverviewOutcome::Inactive);
        }
        let actors = decode_actors(services.runtime())?;
        let mut control = SequelOverviewControl {
            blocked_word: 0,
            ui_flags: 0,
            pointer: input.pointer,
            primary_pressed: input.primary_pressed,
            secondary_pressed: lifecycle.secondary_pointer_pressed,
            press_pending: lifecycle.pointer_press_pending,
            hand_selector: services.manu3_hand_state().requested_animation,
            camera_actor_flags: services.overview_camera_actor_flags(),
        };
        let frame = update_sequel_overview(&actors, &mut control, &mut self.state);

        input.primary_pressed = control.primary_pressed;
        input.press_pending = control.press_pending != 0;
        lifecycle.primary_pointer_pressed = control.primary_pressed;
        lifecycle.secondary_pointer_pressed = control.secondary_pressed;
        lifecycle.pointer_press_pending = control.press_pending;
        services.set_overview_camera_actor_flags(control.camera_actor_flags);
        services.manu3_hand_state_mut().requested_animation = control.hand_selector;
        render_draws(services.runtime_mut(), &frame.draws)?;
        Ok(frame.outcome)
    }
}

fn decode_actors(runtime: &OriginalGameRuntime) -> Result<Vec<SequelOverviewActor>> {
    let profile = runtime
        .current_profile()
        .context("simulation overview requires a loaded BloodScript profile")?;
    if profile.state().dialect() != ScriptDialect::BigBugBang {
        bail!("simulation overview is only defined by the Big Bug Bang runtime");
    }
    let arche = profile
        .builtins()
        .archetype
        .context("loaded sequel profile has no arche object")?;
    let excluded_holder = profile.directory().find_active_object(TRASHLANDO_NAME);
    profile
        .state()
        .objects()
        .iter()
        .filter(|object| object.kind == ScriptObjectKind::Actor)
        .map(|object| {
            let flags = record_word(object.bytes(), ACTOR_FLAGS_OFFSET, object.id)?;
            let holder = object_reference(profile.state(), object.id, ACTOR_HOLDER_OFFSET)?;
            let position = resolved_position(profile.state(), object.id, arche)?;
            let opponent_position = if flags & ACTOR_CONFLICT_FLAG == 0 {
                None
            } else {
                let opponent = object_reference(profile.state(), object.id, ACTOR_OPPONENT_OFFSET)
                    .with_context(|| {
                        format!("conflicting overview actor {:?} has no opponent", object.id)
                    })?;
                Some(resolved_position(profile.state(), opponent, arche)?)
            };
            Ok(SequelOverviewActor {
                flags,
                in_play: object.kind.mask() & ScriptObjectKind::Actor.mask() != 0,
                group_mask: record_word(object.bytes(), ACTOR_GROUP_OFFSET, object.id)?,
                quantity: record_word(object.bytes(), ACTOR_QUANTITY_OFFSET, object.id)?,
                balance: record_word(object.bytes(), ACTOR_BALANCE_OFFSET, object.id)?,
                holder_active: object_has_flag(profile.state(), holder, ScriptObjectFlag::Active)
                    .unwrap_or(false),
                holder_in_play: object_has_flag(profile.state(), holder, ScriptObjectFlag::InPlay)
                    .unwrap_or(false),
                holder_excluded: Some(holder) == excluded_holder,
                position,
                opponent_position,
            })
        })
        .collect()
}

fn object_reference(
    state: &ScriptState,
    actor: ScriptObjectId,
    byte_offset: usize,
) -> Result<ScriptObjectId> {
    let field = state
        .object_word(actor, byte_offset / size_of::<u16>())
        .with_context(|| format!("overview actor {actor:?} has no word at byte {byte_offset}"))?;
    match state.object_reference(field) {
        Some(ScriptStateObjectReference::Object(object)) => Ok(object),
        Some(ScriptStateObjectReference::Sentinel) => {
            bail!("overview actor {actor:?} has a sentinel relation at byte {byte_offset}")
        }
        None => bail!("overview actor {actor:?} has an invalid relation at byte {byte_offset}"),
    }
}

fn resolved_position(
    state: &ScriptState,
    object: ScriptObjectId,
    arche: ScriptObjectId,
) -> Result<[u16; 2]> {
    let field = resolve_navigation_position(state, object, arche, u16::MIN)
        .with_context(|| format!("resolving overview marker for {object:?}"))?;
    state
        .word_pair(field)
        .with_context(|| format!("reading resolved overview marker for {object:?}"))
}

fn record_word(bytes: &[u8], offset: usize, actor: ScriptObjectId) -> Result<u16> {
    let bytes = bytes
        .get(offset..offset + size_of::<u16>())
        .with_context(|| format!("overview actor {actor:?} has no word at byte {offset}"))?;
    Ok(u16::from_le_bytes(
        bytes
            .try_into()
            .expect("checked overview word has fixed size"),
    ))
}

fn render_draws(runtime: &mut OriginalGameRuntime, draws: &[SequelOverviewDraw]) -> Result<()> {
    let fonts = runtime.data().font_resources().clone();
    let framebuffer = runtime.front_buffer_mut().pixels_mut();
    for draw in draws.iter().copied() {
        match draw {
            SequelOverviewDraw::Fill {
                origin,
                extent,
                color,
            } => {
                fill_framebuffer_rect(
                    framebuffer,
                    FULL_DISPLAY_CLIP,
                    point(origin),
                    extent[0],
                    extent[1],
                    color,
                )?;
            }
            SequelOverviewDraw::Horizontal {
                origin,
                extent,
                color,
            } => {
                draw_horizontal_span(
                    framebuffer,
                    FULL_DISPLAY_CLIP,
                    point(origin),
                    extent,
                    RasterSpanPaint::Solid(color),
                )?;
            }
            SequelOverviewDraw::Vertical {
                origin,
                extent,
                color,
            } => {
                draw_vertical_span(
                    framebuffer,
                    FULL_DISPLAY_CLIP,
                    point(origin),
                    extent,
                    RasterSpanPaint::Solid(color),
                )?;
            }
            SequelOverviewDraw::Outline {
                origin,
                extent,
                color,
            } => {
                draw_rect_outline(
                    framebuffer,
                    FULL_DISPLAY_CLIP,
                    point(origin),
                    extent[0],
                    extent[1],
                    RasterSpanPaint::Solid(color),
                )?;
            }
            SequelOverviewDraw::Line { start, end, color } => {
                draw_line_segment(
                    framebuffer,
                    FULL_DISPLAY_CLIP,
                    point(start),
                    point(end),
                    color,
                )?;
            }
            SequelOverviewDraw::Text {
                label,
                origin,
                color,
            } => {
                draw_planar_square_caps_text_clipped(
                    framebuffer,
                    &fonts,
                    label.text(),
                    FontPoint {
                        x: signed_coordinate(origin[0]),
                        y: signed_coordinate(origin[1]),
                    },
                    FULL_FONT_BAND,
                    color,
                )?;
            }
        }
    }
    Ok(())
}

const fn point(position: [u16; 2]) -> RasterPoint {
    RasterPoint {
        x: signed_coordinate(position[0]),
        y: signed_coordinate(position[1]),
    }
}

const fn signed_coordinate(value: u16) -> i32 {
    value as i16 as i32
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::code::ScriptDialect;

    use crate::native::bloodprg::{
        NavActorSlotFlags, ScriptProfileId, SequelOverviewControl, SequelOverviewState,
        update_sequel_overview,
    };
    use crate::runtime::{OriginalGameData, OriginalGameDataPaths, OriginalGameRuntime};

    use super::{decode_actors, render_draws};

    #[test]
    #[ignore = "requires all original Big Bug Bang profiles and executable fonts"]
    fn every_authentic_profile_builds_and_renders_an_overview() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/imported-assets");
        let paths = OriginalGameDataPaths::from_root(root).unwrap();
        let data = OriginalGameData::load(paths).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let mut actor_count = 0_usize;
        let mut represented_groups = 0_u16;

        for index in 0..17 {
            runtime
                .load_profile(
                    ScriptProfileId::new_for_dialect(index, ScriptDialect::BigBugBang).unwrap(),
                )
                .unwrap_or_else(|error| panic!("profile {index}: {error:?}"));
            let actors = decode_actors(&runtime)
                .unwrap_or_else(|error| panic!("profile {index}: {error:?}"));
            actor_count += actors.len();
            represented_groups |= actors.iter().fold(0, |mask, actor| mask | actor.group_mask);
            let mut control = SequelOverviewControl {
                secondary_pressed: true,
                camera_actor_flags: NavActorSlotFlags::from_executable(0x0f),
                ..SequelOverviewControl::default()
            };
            let mut state = SequelOverviewState::default();
            let frame = update_sequel_overview(&actors, &mut control, &mut state);
            render_draws(&mut runtime, &frame.draws)
                .unwrap_or_else(|error| panic!("profile {index}: {error:?}"));

            let eligible = actors
                .iter()
                .copied()
                .map(|mut actor| {
                    actor.flags |= 5;
                    actor.holder_active = true;
                    actor.holder_in_play = true;
                    actor.holder_excluded = false;
                    actor
                })
                .collect::<Vec<_>>();
            control.secondary_pressed = true;
            state = SequelOverviewState::default();
            let frame = update_sequel_overview(&eligible, &mut control, &mut state);
            render_draws(&mut runtime, &frame.draws)
                .unwrap_or_else(|error| panic!("eligible profile {index}: {error:?}"));
        }

        assert!(actor_count > 0);
        assert_eq!(represented_groups, u16::MAX);
    }
}
