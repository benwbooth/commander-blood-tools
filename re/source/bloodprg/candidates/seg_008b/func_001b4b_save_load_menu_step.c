#include <string.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_save.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_startup.h"
#include "../include/bloodprg_vm.h"

#define SAVE_LOAD_UI_ACTIVE 0x04u
#define SAVE_SLOT_NONE 0xffffu
#define SAVE_SLOT_QUICK_INDEX 9
#define SAVE_SLOT_DIRECTORY_BYTES \
    (BLOODPRG_SAVE_SLOT_COUNT * (cb_u16)sizeof(bloodprg_save_slot))
#define SAVE_STATE_WORD_BYTES 0x0200u
#define SAVE_STATE_STRING_BYTES 0x0060u

void CB_NEAR save_load_menu_step(void)
{
    volatile bloodprg_save_slot CB_NEAR *slot;
    cb_i16 selection;
    cb_u16 slot_offset;
    cb_u16 file_handle;
    cb_u16 byte_count;
    cb_u16 state_size;
    int transition_complete;

    if ((quicksave_request_active & 1u) != 0u) {
        memcpy(
                (void CB_NEAR *)save_slot_records[SAVE_SLOT_QUICK_INDEX].name,
                (const void CB_NEAR *)save_slot_quick_name_source,
                8u);
        save_slot_active_name =
                save_slot_records[SAVE_SLOT_QUICK_INDEX].name;
        quicksave_request_active = 0u;
        goto save_game;
    }

    if ((save_request_active | load_request_active) == 0u) {
        return;
    }

    vm_ui_flags |= SAVE_LOAD_UI_ACTIVE;
    if ((save_slot_menu_phase & 1u) != 0u) {
        presentation_list_editing = 1u;
        (void)list_widget_layout_unified(
                save_slot_item_offsets, save_slot_item_offsets);
        presentation_list_editing = 0u;
        save_slot_transition_aux = 0u;
        framebuffer_transition_current_step = 0u;
        framebuffer_transition_total_steps = 6u;
        save_slot_active_name = save_slot_records[0].name;
        save_slot_selected_index = 0u;
        memcpy(
                (void CB_NEAR *)save_slot_edit_buffer,
                (const void CB_NEAR *)save_slot_records[0].name,
                BLOODPRG_SAVE_SLOT_NAME_BYTES);
        ++save_slot_menu_phase;
    }

    if ((save_slot_menu_phase & 2u) != 0u) {
        transition_complete = framebuffer_transition_total_steps
                == framebuffer_transition_current_step;
        framebuffer_rect_interpolate_and_remap_step(
                (const bloodprg_rect_i16 CB_NEAR *)
                    presentation_choice_current_rect,
                (const bloodprg_rect_i16 CB_NEAR *)
                    presentation_choice_target_rect);
        if (!transition_complete) {
            return;
        }
        save_slot_menu_phase = 0u;
    }

    selection = list_widget_layout_unified(
            save_slot_item_offsets, save_slot_item_offsets);
    if ((save_request_active & 1u) != 0u) {
        save_slot_name_length = 0u;
        while (save_slot_edit_buffer[save_slot_name_length] != 0u
                && save_slot_edit_buffer[save_slot_name_length]
                    != (cb_u8)' ') {
            ++save_slot_name_length;
        }

        if (save_slot_name_edit_step()) {
            goto save_game;
        }
        if (selection < 0 || selection == SAVE_SLOT_QUICK_INDEX) {
            return;
        }

        save_slot_selected_index = (cb_u16)selection;
        slot_offset = save_slot_item_offsets[(cb_u16)selection];
        if (slot_offset == SAVE_SLOT_NONE) {
            goto close_menu;
        }
        slot = (volatile bloodprg_save_slot CB_NEAR *)slot_offset;
        save_slot_active_name = slot->name;
        memcpy(
                (void CB_NEAR *)save_slot_edit_buffer,
                (const void CB_NEAR *)slot->name,
                BLOODPRG_SAVE_SLOT_NAME_BYTES);
        return;
    }

    if (selection < 0) {
        return;
    }

    startup_write_directory_enter();
    slot_offset = save_slot_item_offsets[(cb_u16)selection];
    if (slot_offset == SAVE_SLOT_NONE) {
        goto close_menu;
    }
    slot = (volatile bloodprg_save_slot CB_NEAR *)slot_offset;
    if (!cb_dos_open_read_only(
            (const volatile char CB_FAR *)slot->filename,
            &file_handle)) {
        goto close_menu;
    }

    (void)cb_dos_read(
            file_handle,
            (volatile cb_u8 CB_FAR *)&vm_script_profile_request,
            (cb_u16)sizeof(vm_script_profile_request));
    (void)vm_resource_profile_select((cb_u16)vm_script_profile_request);
    vm_script_profile_request = -1;
    vm_execution_enabled = 1u;
    (void)vm_run_wrapper();
    (void)cb_dos_read(
            file_handle,
            (volatile cb_u8 CB_FAR *)vm_state_words,
            SAVE_STATE_WORD_BYTES);
    (void)cb_dos_read(
            file_handle,
            (volatile cb_u8 CB_FAR *)vm_record_string_slots,
            SAVE_STATE_STRING_BYTES);
    state_size = (cb_u16)resource_get_field4(vm_record_resource_handle);
    (void)cb_dos_read(file_handle, vm_record_base, state_size);
    byte_count = cb_dos_read(
            file_handle, graphics_work_surface, 0xffffu);
    (void)vm_patch_stream_apply(byte_count);
    vm_record_state_proc();
    ship_3d_hud_palette_snapshot_and_camera_reset();
    save_load_redraw_pending = 1u;
    palette_dirty = 1u;
    cb_dos_close(file_handle);
    goto close_menu;

save_game:
    startup_write_directory_enter();
    slot = (volatile bloodprg_save_slot CB_NEAR *)save_slot_active_name;
    if (!cb_dos_create_truncate(
            (const volatile char CB_FAR *)slot->filename,
            &file_handle)) {
        goto close_menu;
    }

    (void)cb_dos_write(
            file_handle,
            (const volatile cb_u8 CB_FAR *)&vm_resource_profile_index,
            (cb_u16)sizeof(vm_resource_profile_index));
    (void)cb_dos_write(
            file_handle,
            (const volatile cb_u8 CB_FAR *)vm_state_words,
            SAVE_STATE_WORD_BYTES);
    (void)cb_dos_write(
            file_handle,
            (const volatile cb_u8 CB_FAR *)vm_record_string_slots,
            SAVE_STATE_STRING_BYTES);
    state_size = (cb_u16)resource_get_field4(vm_record_resource_handle);
    (void)cb_dos_write(file_handle, vm_record_base, state_size);
    byte_count = vm_patch_stream_build();
    (void)cb_dos_write(file_handle, graphics_work_surface, byte_count);
    cb_dos_close(file_handle);
    (void)file_create_and_write(
            (const volatile char CB_FAR *)save_slot_directory_path,
            (const volatile cb_u8 CB_FAR *)save_slot_records,
            SAVE_SLOT_DIRECTORY_BYTES);

close_menu:
    vm_ui_flags &= (cb_u8)~SAVE_LOAD_UI_ACTIVE;
    save_request_active = 0u;
    load_request_active = 0u;
}
