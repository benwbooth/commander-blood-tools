#include "../include/bloodprg_nav.h"

int CB_NEAR presentation_line_helper(
        volatile bloodprg_presentation_line_record CB_NEAR *line)
{
    const volatile bloodprg_presentation_resource_header CB_FAR *resource;
    cb_u16 frame;

    if ((vm_ui_flags & BLOODPRG_PRESENTATION_UI_BUSY_GATE) != 0) {
        return 0;
    }

    if ((line->flags & BLOODPRG_PRESENTATION_LINE_LOADED_FLAG) == 0) {
        vm_ui_flags = (cb_u8)(vm_ui_flags | BLOODPRG_PRESENTATION_UI_REDRAW_FLAG);
        resource_file_load(fs_presentation_resource_names[line->resource_id],
            nav_presentation_resource_buffer);

        resource = (const volatile bloodprg_presentation_resource_header CB_FAR *)
            nav_presentation_resource_buffer;
        line->terminal_frame = resource->terminal_frame;

        frame = (cb_u16)(line->terminal_frame - 1u);
        if ((nav_presentation_reverse & 1u) == 0) {
            frame = 0;
            nav_presentation_reverse = 0;
        }
        line->frame_index = frame;
        line->flags = (cb_u8)(line->flags | BLOODPRG_PRESENTATION_LINE_LOADED_FLAG);
    }

    entity_record_setter(4u, nav_presentation_resource_buffer,
        line->draw_x, line->draw_y, line->frame_index);

    if ((nav_presentation_reverse & 1u) != 0) {
        frame = line->frame_index;
        if (frame == 0) {
            nav_presentation_reverse = 0;
            vm_ui_flags = (cb_u8)(vm_ui_flags & 0xfbu);
            return 1;
        }
        line->frame_index = (cb_u16)(frame - 1u);
    } else {
        frame = line->frame_index;
        if (frame == line->terminal_frame) {
            nav_presentation_reverse = 0;
            vm_ui_flags = (cb_u8)(vm_ui_flags & 0xfbu);
            return 1;
        }
        line->frame_index = (cb_u16)(frame + 1u);
    }

    return 0;
}
