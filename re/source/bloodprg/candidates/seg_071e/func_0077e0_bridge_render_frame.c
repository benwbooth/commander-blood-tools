#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_input.h"
#include "../include/bloodprg_list.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define BRIDGE_RENDER_ACTIVE_FLAG 0x0001u
#define BRIDGE_SCENE_DISPATCH_FLAG 0x02u
#define BRIDGE_SCREEN_REBUILD_FLAG 0x01u
#define BRIDGE_TRANSITION_FLAG 0x01u
#define BRIDGE_PRESENTATION_QUEUE_FLAG 0x01u
#define BRIDGE_FRAME_READY_FLAG 0x01u
#define BRIDGE_COMPLETION_FLAG 0x01u
#define BRIDGE_LEFT_SCREEN_EDGE 160u

typedef struct bridge_frame_context {
    cb_u8 reserved_00[4];
    const volatile bloodprg_sprite_source_extent CB_FAR *comparison_extent;
} bridge_frame_context;

#if defined(__WATCOMC__)
#define BRIDGE_REMAP(table, x, y, width, height) \
    framebuffer_rect_palette_remap_ds_bp( \
        (const cb_u8 CB_NEAR *)(table), (x), (y), (width), (height))
#else
#define BRIDGE_REMAP(table, x, y, width, height) \
    framebuffer_rect_palette_remap( \
        (const cb_u8 CB_FAR *)(table), (x), (y), (width), (height))
#endif

void CB_FAR bridge_render_frame(cb_u16 scene_link_target)
{
    const volatile bridge_frame_context CB_NEAR *frame_context;

    if ((vm_ui_state.word & BRIDGE_RENDER_ACTIVE_FLAG) == 0u) {
        return;
    }

    if ((nav_actor_transition_phase & BRIDGE_SCENE_DISPATCH_FLAG) != 0u) {
        dlg_line_id_scene_dispatch(scene_link_target);
        return;
    }

    if ((nav_screen_rebuild_pending & BRIDGE_SCREEN_REBUILD_FLAG) != 0u) {
        nav_actor_presentation_state = 1u;
        presentation_mode_previous_state = 1u;
        screen_flags_init();
    }

    if (bridge_steer_update(&scene_link_target)) {
        nav_actor_presentation_state = 2u;
        if ((cb_u16)mouse_x <= BRIDGE_LEFT_SCREEN_EDGE) {
            nav_actor_presentation_state = 3u;
        }
        (void)page_flip();
    }

    if ((nav_transition_pending & BRIDGE_TRANSITION_FLAG) != 0u) {
        camera_fsm_state_gate(scene_link_target);
    }
    (void)presentation_mode_bits_update();
    sprite_slot_commit_dirty_range(0u, 31u);
    bloodprg_clip_snapshot_flags = 1u;
    presentation_mode_dispatch();
    nav_actor_slot_update_loop();

    if ((vm_c2_presentation_gate & BRIDGE_PRESENTATION_QUEUE_FLAG) == 0u) {
        if ((nav_transition_pending & BRIDGE_TRANSITION_FLAG) != 0u) {
            sprite_slot_dirty_range_render(20u, 31u);
        } else if (nav_camera_view_state == 0u) {
            dirty_rects_copy_secondary_to_primary(
                    (const volatile bloodprg_dirty_rect CB_FAR *)
                    &bloodprg_dirty_rect_list[0]);
        }
    }

    frame_context =
            (const volatile bridge_frame_context CB_NEAR *)scene_link_target;
    nav_camera_state_check(frame_context->comparison_extent);
    camera_nav_update();
    screen_mode_update(scene_link_target);
    if ((resource_frame_presented & BRIDGE_FRAME_READY_FLAG) == 0u) {
        return;
    }

    sprite_slot_dirty_range_render(1u, 19u);
    name_area_palette_effect_update();
    nav_state_gate();
    nav_choice_dispatch();
    if ((nav_actor_completion_latch & BRIDGE_COMPLETION_FLAG) != 0u) {
        BRIDGE_REMAP(bloodprg_sprite_remap_6011, 137u, 139u, 50u, 44u);
    }
}
