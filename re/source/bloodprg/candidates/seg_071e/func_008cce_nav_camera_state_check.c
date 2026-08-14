#include <dos.h>

#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_manu3.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define NAV_CAMERA_ACTIVE_FLAG 0x01u
#define NAV_CAMERA_UI_FLAG 0x04u
#define NAV_CAMERA_FIRST_FRAME 8u
#define NAV_CAMERA_CENTER_Y 110
#define NAV_CAMERA_SCREEN_WIDTH 320u
#define NAV_CAMERA_SCREEN_HEIGHT 200
#define NAV_CAMERA_PRIMARY_ENTITY 0u
#define NAV_CAMERA_ARCH_ENTITY 1u
#define NAV_CAMERA_SUBOBJECT_FIRST_ENTITY 5u
#define NAV_CAMERA_FINAL_ENTITY 31u
#define NAV_CAMERA_RESOURCE_HANDLE 0x2cu
#define NAV_CAMERA_ARCH_FRAME 6u
#define NAV_CAMERA_KIND_SHIP 0x0010u
#define NAV_CAMERA_KIND_BLACK_HOLE 0x0100u
#define NAV_CAMERA_ENTITY_VISIBLE_FLAGS 0x03u
#define NAV_CAMERA_POINTER_WIDTH 4
#define NAV_CAMERA_POINTER_HEIGHT 4
#define NAV_CAMERA_LABEL_Y_BIAS 10
#define NAV_CAMERA_LABEL_COLOR 0xefu
#define NAV_CAMERA_HAND_LEFT 11u
#define NAV_CAMERA_HAND_RIGHT 12u
#define NAV_CAMERA_HALF_WIDTH 160u

typedef struct nav_camera_chart_record {
    cb_u16 kind;
    cb_u8 reserved_02[0x12];
    cb_u16 secondary_link;
    cb_u16 current_location;
    bloodprg_nav_chart_point marker;
} nav_camera_chart_record;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NAV_CAMERA_RECORD_AT(offset) \
    ((volatile nav_camera_chart_record CB_FAR *) \
        MK_FP(FP_SEG(vm_record_base), (offset)))
#else
#define NAV_CAMERA_RECORD_AT(offset) \
    ((volatile nav_camera_chart_record CB_FAR *) \
        (vm_record_base + (offset)))
#endif

void CB_NEAR nav_camera_state_check(
        const volatile bloodprg_sprite_source_extent CB_FAR *comparison_extent)
{
    const volatile bloodprg_nav_wipe_point CB_NEAR *endpoint;
    const volatile bloodprg_nav_wipe_span CB_FAR *span;
    const volatile cb_u16 CB_NEAR *object_list;
    const cb_u8 CB_FAR *label;
    volatile nav_camera_chart_record CB_FAR *arche;
    volatile nav_camera_chart_record CB_FAR *current_location;
    volatile nav_camera_chart_record CB_FAR *object;
    bloodprg_graphics_buffer_ptr saved_buffer;
    volatile cb_u8 *entity_flags;
    cb_u16 object_offset;
    cb_u16 picked_offset;
    cb_u16 object_count;
    cb_u16 entity_id;
    cb_u16 frame;
    cb_u16 left;
    cb_u16 width;
    cb_u16 right;
    cb_u16 draw_x;
    cb_u16 draw_y;
    cb_u16 label_width;
    cb_i16 row;
    cb_i16 label_x;
    cb_i16 label_y;
    cb_u8 state;

    state = nav_camera_view_state;
    if (state == 0u) {
        if ((nav_camera_view_active & NAV_CAMERA_ACTIVE_FLAG) == 0u) {
            return;
        }
        goto interactive;
    }

    nav_selected_location_record = 0u;
    if ((nav_camera_view_active & NAV_CAMERA_ACTIVE_FLAG) == 0u) {
        if (state == NAV_CAMERA_FIRST_FRAME) {
            vga_planar_to_chunky(
                    (const volatile cb_u8 CB_FAR *)MK_FP(0xa000u, 0xc000u),
                    graphics_work_surface);
            object_count = nav_chart_list_build();
            if (object_count != 0u) {
                nav_chart_object_count = object_count;
                saved_buffer = graphics_display_buffer_ds;
                graphics_display_buffer_ds = graphics_work_surface;
                object_list = vm_nav_chart_object_offsets;
                nav_chart_subobject_count = 0u;

                do {
                    object_offset = *object_list++;
                    object = NAV_CAMERA_RECORD_AT(object_offset);
                    nav_chart_secondary_marker =
                            object->secondary_link == 0u;
                    frame = 0u;
                    if ((object->kind & NAV_CAMERA_KIND_BLACK_HOLE) != 0u) {
                        frame = 1u;
                    } else if ((object->kind & NAV_CAMERA_KIND_SHIP) != 0u) {
                        frame = 2u;
                    }

                    draw_x = object->marker.x;
                    draw_y = object->marker.y;
                    entity_object_populate(
                            NAV_CAMERA_PRIMARY_ENTITY,
                            NAV_CAMERA_RESOURCE_HANDLE,
                            draw_x,
                            draw_y,
                            frame);
                    if ((nav_chart_secondary_marker & 1u) != 0u) {
                        draw_x = (cb_u16)(draw_x - 3u);
                        draw_y = (cb_u16)(draw_y - 3u);
                        frame = (cb_u16)(frame + 3u);
                        entity_id = (cb_u16)(
                                NAV_CAMERA_SUBOBJECT_FIRST_ENTITY
                                + nav_chart_subobject_count);
                        ++nav_chart_subobject_count;
                        entity_object_populate(
                                entity_id,
                                NAV_CAMERA_RESOURCE_HANDLE,
                                draw_x,
                                draw_y,
                                frame);
                    }
                    sprite_slot_dirty_range_render(
                            NAV_CAMERA_PRIMARY_ENTITY,
                            NAV_CAMERA_PRIMARY_ENTITY);
                } while (--object_count != 0u);

                entity_flag_state_transition(NAV_CAMERA_PRIMARY_ENTITY);
                graphics_display_buffer_ds = saved_buffer;
            }

            arche = NAV_CAMERA_RECORD_AT(vm_arche_record_offset);
            draw_x = (cb_u16)(arche->marker.x - 16u);
            if ((cb_i16)draw_x < 0) {
                draw_x = 0u;
            }
            draw_y = (cb_u16)(arche->marker.y - 13u);
            if ((cb_i16)draw_y < 0) {
                draw_y = 0u;
            }
            current_location =
                    NAV_CAMERA_RECORD_AT(arche->current_location);
            if ((current_location->kind
                    & NAV_CAMERA_KIND_BLACK_HOLE) != 0u) {
                draw_x = (cb_u16)(draw_x + 5u);
                draw_y = (cb_u16)(draw_y + 2u);
            }
            if ((current_location->kind & NAV_CAMERA_KIND_SHIP) != 0u) {
                draw_x = (cb_u16)(draw_x + 3u);
            }
            entity_object_populate(
                    NAV_CAMERA_ARCH_ENTITY,
                    NAV_CAMERA_RESOURCE_HANDLE,
                    draw_x,
                    draw_y,
                    NAV_CAMERA_ARCH_FRAME);
            entity_flag_state_transition(NAV_CAMERA_ARCH_ENTITY);
            entity_id = NAV_CAMERA_SUBOBJECT_FIRST_ENTITY;
            object_count = nav_chart_subobject_count;
            while (object_count-- != 0u) {
                entity_flag_state_transition(entity_id++);
            }
            entity_flag_state_transition(NAV_CAMERA_FINAL_ENTITY);
        }

        endpoint = &nav_center_wipe_endpoints[(cb_u8)(state - 1u)];
        nav_center_wipe_complete = state == 1u;
        nav_center_wipe_span_table_build(endpoint);
        span = (const volatile bloodprg_nav_wipe_span CB_FAR *)
                graphics_display_buffer_ds;
        row = endpoint->y;
        if (row < NAV_CAMERA_CENTER_Y) {
            draw_y = NAV_CAMERA_CENTER_Y;
            do {
                back_buffer_copy_from(0u, draw_y, NAV_CAMERA_SCREEN_WIDTH);
                ++draw_y;
            } while (draw_y != NAV_CAMERA_SCREEN_HEIGHT);

            while ((cb_i16)span->left >= 0) {
                left = span->left;
                width = span->width;
                ++span;
                back_buffer_copy_from(0u, (cb_u16)row, left);
                right = (cb_u16)(left + width);
                back_buffer_copy_from(
                        right,
                        (cb_u16)row,
                        (cb_u16)(NAV_CAMERA_SCREEN_WIDTH - right));
                ++row;
            }
        } else {
            row = NAV_CAMERA_CENTER_Y;
            for (;;) {
                left = span->left;
                width = span->width;
                ++span;
                if ((cb_i16)width < 0) {
                    break;
                }
                back_buffer_copy_from(left, (cb_u16)row, width);
                ++row;
            }
            while (row < NAV_CAMERA_SCREEN_HEIGHT) {
                back_buffer_copy_from(
                        0u, (cb_u16)row, NAV_CAMERA_SCREEN_WIDTH);
                ++row;
            }
        }
    } else {
        if (state == NAV_CAMERA_FIRST_FRAME) {
            nav_center_wipe_complete = 0u;
            entity_flag_state_transition(NAV_CAMERA_ARCH_ENTITY);
            entity_id = NAV_CAMERA_SUBOBJECT_FIRST_ENTITY;
            object_count = nav_chart_subobject_count;
            while (object_count-- != 0u) {
                entity_flag_state_transition(entity_id++);
            }

            saved_buffer = graphics_back_buffer_ds;
            graphics_back_buffer_ds = graphics_work_surface;
            pbm_palette_refresh_ds = 0u;
            bridge_panorama_frame_load(0u);
            ship_3d_hud_palette_snapshot_and_camera_reset();
            (void)page_flip();
            graphics_back_buffer_ds = saved_buffer;
        }

        endpoint = &nav_center_wipe_endpoints[(cb_u8)(9u - state)];
        nav_center_wipe_span_table_build(endpoint);
        span = (const volatile bloodprg_nav_wipe_span CB_FAR *)
                graphics_display_buffer_ds;
        row = endpoint->y;
        if (row < NAV_CAMERA_CENTER_Y) {
            draw_y = (cb_u16)(row - 1);
            while ((cb_i16)draw_y > 0) {
                back_buffer_copy_from(
                        0u, draw_y, NAV_CAMERA_SCREEN_WIDTH);
                --draw_y;
            }

            for (;;) {
                left = span->left;
                width = span->width;
                ++span;
                if ((cb_i16)width < 0) {
                    break;
                }
                back_buffer_copy_from(left, (cb_u16)row, width);
                ++row;
            }
        } else {
            row = NAV_CAMERA_CENTER_Y;
            draw_y = NAV_CAMERA_CENTER_Y - 1;
            while (draw_y > 0u) {
                back_buffer_copy_from(
                        0u, draw_y, NAV_CAMERA_SCREEN_WIDTH);
                --draw_y;
            }

            while ((cb_i16)span->left >= 0) {
                left = span->left;
                width = span->width;
                ++span;
                back_buffer_copy_from(0u, (cb_u16)row, left);
                right = (cb_u16)(left + width);
                back_buffer_copy_from(
                        right,
                        (cb_u16)row,
                        (cb_u16)(NAV_CAMERA_SCREEN_WIDTH - right));
                ++row;
            }
        }
    }

    --nav_camera_view_state;
    dirty_rects_copy_secondary_to_primary(
            (const volatile bloodprg_dirty_rect CB_FAR *)
            &bloodprg_dirty_rect_list[0]);
    return;

interactive:
    if ((nav_center_wipe_complete & 1u) == 0u) {
        return;
    }
    vm_ui_flags |= NAV_CAMERA_UI_FLAG;
    if (nav_selected_location_record != 0u) {
        location_info_panel_dispatch(comparison_extent);
        return;
    }

    entity_id = NAV_CAMERA_SUBOBJECT_FIRST_ENTITY;
    object_count = nav_chart_subobject_count;
    while (object_count-- != 0u) {
        entity_flags = (volatile cb_u8 *)
                &bloodprg_entity_table_ds[entity_id++].flags;
        *entity_flags |= NAV_CAMERA_ENTITY_VISIBLE_FLAGS;
        if ((nav_chart_entity_state_mask & 1u) == 0u) {
            *entity_flags &= (cb_u8)~1u;
        }
    }

    entity_flags = (volatile cb_u8 *)
            &bloodprg_entity_table_ds[NAV_CAMERA_ARCH_ENTITY].flags;
    *entity_flags |= NAV_CAMERA_ENTITY_VISIBLE_FLAGS;
    if ((nav_chart_entity_state_mask & 7u) != 0u) {
        *entity_flags &= (cb_u8)~1u;
    }

    picked_offset = nav_chart_object_pick(vm_record_base);
    if (picked_offset == 0u) {
        return;
    }

    object = NAV_CAMERA_RECORD_AT(picked_offset);
    if ((mouse_primary_pressed & 1u) == 0u) {
        label = (const cb_u8 CB_FAR *)object + 4u;
        label_width = text_width_dual_font_far(label, 1);
        label_x = (cb_i16)(
                (cb_u16)mouse_x - label_width);
        if (label_x < 0) {
            label_x = 0;
        }
        label_y = (cb_i16)(mouse_y - NAV_CAMERA_LABEL_Y_BIAS);
        if (label_y < 0) {
            label_y = 0;
        }
        main_font_text_draw_display(
                label,
                (cb_u16)label_x,
                (cb_u16)label_y,
                NAV_CAMERA_LABEL_COLOR);
        return;
    }

    manu3_animation_selector_current = 0u;
    manu3_animation_selector_request = NAV_CAMERA_HAND_LEFT;
    if ((cb_u16)mouse_x > NAV_CAMERA_HALF_WIDTH) {
        manu3_animation_selector_request = NAV_CAMERA_HAND_RIGHT;
    }
    mouse_primary_pressed = 0u;
    mouse_press_pending = 0u;

    arche = NAV_CAMERA_RECORD_AT(vm_arche_record_offset);
    if (picked_offset == arche->current_location) {
        return;
    }

    nav_selected_location_record = picked_offset;
    nav_location_panel_current_rect.x = mouse_x;
    nav_location_panel_current_rect.y = mouse_y;
    nav_location_panel_current_rect.width = NAV_CAMERA_POINTER_WIDTH;
    nav_location_panel_current_rect.height = NAV_CAMERA_POINTER_HEIGHT;
    framebuffer_transition_current_step = 0u;
    framebuffer_transition_total_steps = NAV_CAMERA_FIRST_FRAME;
    nav_location_panel_transition_state = 1u;
    nav_location_panel_scale_step = 0u;
    nav_location_panel_active = 1u;
    nav_deferred_record_link = picked_offset;
    entity_flag_state_transition(NAV_CAMERA_ARCH_ENTITY);
    entity_id = NAV_CAMERA_SUBOBJECT_FIRST_ENTITY;
    object_count = nav_chart_subobject_count;
    while (object_count-- != 0u) {
        entity_flag_state_transition(entity_id++);
    }
}
