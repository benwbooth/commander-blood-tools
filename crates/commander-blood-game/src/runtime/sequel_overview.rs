//! Runtime decoding and flat rendering for Big Bug Bang's simulation overview.

use anyhow::{Context, Result, bail};
use commander_blood_formats::code::ScriptDialect;
use commander_blood_formats::script::{
    ScriptObjectId, ScriptObjectKind, ScriptState, ScriptStateObjectReference,
};

use crate::native::bloodprg::{
    BridgeSpriteRect, FontPoint, FontVerticalBand, GameLifecycleState, NavActorSlotFlags,
    NavigationChartInputState, RasterPoint, RasterSpanPaint, ScriptObjectFlag, SequelOverviewActor,
    SequelOverviewControl, SequelOverviewDraw, SequelOverviewOutcome, SequelOverviewState,
    draw_horizontal_span, draw_line_segment, draw_planar_square_caps_text_clipped,
    draw_rect_outline, draw_vertical_span, fill_framebuffer_rect, object_has_flag,
    resolve_navigation_position, update_sequel_overview,
};

use super::{
    LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_WIDTH, ModernGameServices, OriginalGameRuntime,
};

const ACTOR_FLAGS_OFFSET: usize = 2;
const ACTOR_REQUIRED_FLAGS: u16 = 1 | 4;
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
        if frame.outcome == SequelOverviewOutcome::ClosedEmpty {
            // Keep the base chart usable before the simulation has participants.
            // The native empty-overlay close arms another camera animation.
            control.camera_actor_flags = NavActorSlotFlags {
                active: true,
                ..Default::default()
            };
        }

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
    let mut actors = Vec::new();
    for object in profile
        .state()
        .objects()
        .iter()
        .filter(|object| object.kind == ScriptObjectKind::Actor)
    {
        let flags = record_word(object.bytes(), ACTOR_FLAGS_OFFSET, object.id)?;
        // Native rosters 0x6FF2/0x706E reject nonparticipants before
        // dereferencing their holder. Aboard actors may use a sentinel.
        if flags & ACTOR_REQUIRED_FLAGS != ACTOR_REQUIRED_FLAGS
            || record_word(object.bytes(), 0, object.id)? & ScriptObjectKind::Actor.mask() == 0
        {
            continue;
        }
        let holder = object_reference(profile.state(), object.id, ACTOR_HOLDER_OFFSET)?;
        let holder_in_play = object_has_flag(profile.state(), holder, ScriptObjectFlag::InPlay)
            .with_context(|| {
                format!(
                    "overview actor {:?} holder {holder:?} has no flags",
                    object.id
                )
            })?;
        if !holder_in_play {
            continue;
        }
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
        actors.push(SequelOverviewActor {
            flags,
            in_play: object.kind.mask() & ScriptObjectKind::Actor.mask() != 0,
            group_mask: record_word(object.bytes(), ACTOR_GROUP_OFFSET, object.id)?,
            quantity: record_word(object.bytes(), ACTOR_QUANTITY_OFFSET, object.id)?,
            balance: record_word(object.bytes(), ACTOR_BALANCE_OFFSET, object.id)?,
            holder_active: object_has_flag(profile.state(), holder, ScriptObjectFlag::Active)
                .unwrap_or(false),
            holder_in_play,
            holder_excluded: Some(holder) == excluded_holder,
            position,
            opponent_position,
        });
    }
    Ok(actors)
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
    #[ignore = "requires original Big Bug Bang assets"]
    fn initialized_profiles_keep_the_overview_readable() {
        use crate::native::bloodprg::{GameLifecycleState, ScriptClock};
        use crate::runtime::RuntimeScriptSystem;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../output/big-bug-bang/imported-assets");
        let data = OriginalGameData::load(OriginalGameDataPaths::from_root(root).unwrap()).unwrap();
        let mut scripts = RuntimeScriptSystem::new(
            &data,
            ScriptClock {
                hour: 12,
                day: 1,
                month: 1,
            },
        );
        let mut runtime = OriginalGameRuntime::new(data);
        for index in 0..17 {
            scripts
                .load_profile(
                    &mut runtime,
                    ScriptProfileId::new_for_dialect(index, ScriptDialect::BigBugBang).unwrap(),
                )
                .unwrap();
            let mut lifecycle = GameLifecycleState::default();
            for frame in 0..20 {
                scripts
                    .execute_lifecycle_frame(&mut runtime, &mut lifecycle, true)
                    .unwrap_or_else(|e| {
                        panic!("profile {index} frame {frame} initialization: {e:#}")
                    });
                let actors = decode_actors(&runtime)
                    .unwrap_or_else(|e| panic!("profile {index} frame {frame} overview: {e:#}"));
                let mut control = SequelOverviewControl {
                    secondary_pressed: true,
                    ..Default::default()
                };
                let mut overview = SequelOverviewState::default();
                let draws = update_sequel_overview(&actors, &mut control, &mut overview);
                render_draws(&mut runtime, &draws.draws).unwrap();
                if index == 1 && frame == 0 {
                    let profile = runtime.current_profile_mut().unwrap();
                    let daddy = profile
                        .directory()
                        .find_active_object(b"Daddy_Gluxx")
                        .unwrap();
                    let flags = profile.state().object_word(daddy, 1).unwrap();
                    let before = profile.state().word(flags).unwrap();
                    assert_eq!(
                        before & 4,
                        0,
                        "Daddy aboard must not participate in the map"
                    );
                    assert!(profile.state_mut().set_word(flags, before | 4));
                    let error = decode_actors(&runtime).unwrap_err();
                    assert!(
                        error.to_string().contains("sentinel relation at byte 24"),
                        "{error:#}"
                    );
                    assert!(
                        runtime
                            .current_profile_mut()
                            .unwrap()
                            .state_mut()
                            .set_word(flags, before)
                    );
                }
            }
        }
    }

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
            let mut control = SequelOverviewControl {
                secondary_pressed: true,
                camera_actor_flags: NavActorSlotFlags::from_executable(0x0f),
                ..SequelOverviewControl::default()
            };
            let mut state = SequelOverviewState::default();
            let frame = update_sequel_overview(&actors, &mut control, &mut state);
            render_draws(&mut runtime, &frame.draws)
                .unwrap_or_else(|error| panic!("profile {index}: {error:?}"));

            // Exercise every authored marker through the runtime adapter,
            // enabling participation in source records before roster selection.
            let profile = runtime.current_profile_mut().unwrap();
            let participating = profile
                .state()
                .objects()
                .iter()
                .filter(|o| o.kind == commander_blood_formats::script::ScriptObjectKind::Actor)
                .map(|o| {
                    (
                        o.id,
                        super::object_reference(profile.state(), o.id, 24).unwrap(),
                    )
                })
                .collect::<Vec<_>>();
            for (actor, holder) in participating {
                for (object, mask) in [(actor, 5), (holder, 3)] {
                    let field = profile.state().object_word(object, 1).unwrap();
                    let flags = profile.state().word(field).unwrap();
                    assert!(profile.state_mut().set_word(field, flags | mask));
                }
            }
            let eligible = decode_actors(&runtime).unwrap();
            actor_count += eligible.len();
            represented_groups |= eligible
                .iter()
                .fold(0, |mask, actor| mask | actor.group_mask);
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
