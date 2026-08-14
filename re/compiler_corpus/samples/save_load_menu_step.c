/* Codegen probe for BLOODPRG 0x001B4B. */

#include <string.h>

typedef unsigned char u8;
typedef signed int i16;
typedef unsigned int u16;
typedef unsigned long u32;

#define FAR far
#define NEAR near

typedef struct rect_i16_probe {
    i16 x;
    i16 y;
    i16 width;
    i16 height;
} rect_i16_probe;

typedef struct save_slot_probe {
    u8 name[16];
    u8 filename[16];
} save_slot_probe;

extern volatile u8 quicksave_request_probe;
extern volatile u8 save_request_probe;
extern volatile u8 load_request_probe;
extern volatile u8 save_menu_phase_probe;
extern volatile u8 list_editing_probe;
extern volatile u8 transition_aux_probe;
extern volatile u8 transition_current_probe;
extern volatile u8 transition_total_probe;
extern volatile u8 ui_flags_probe;
extern volatile u8 redraw_probe;
extern volatile u8 palette_dirty_probe;
extern volatile u8 vm_enabled_probe;
extern volatile u16 name_length_probe;
extern volatile u16 selected_index_probe;
extern volatile u8 NEAR * volatile active_name_probe;
extern volatile u8 edit_buffer_probe[16];
extern const u8 quick_name_probe[8];
extern const char directory_path_probe[];
extern const u16 slot_offsets_probe[];
extern volatile save_slot_probe slots_probe[10];
extern const i16 target_rect_probe[4];
extern const i16 current_rect_probe[4];
extern volatile u16 current_profile_probe;
extern volatile i16 profile_request_probe;
extern volatile u16 state_resource_handle_probe;
extern volatile u16 state_words_probe[];
extern volatile u8 state_strings_probe[][16];
extern volatile u8 FAR *record_base_probe;
extern volatile u8 FAR *work_surface_probe;

i16 FAR list_widget_probe(const u16 NEAR *items);
void FAR transition_step_probe(
        const rect_i16_probe NEAR *source,
        const rect_i16_probe NEAR *target);
int NEAR name_edit_probe(void);
void FAR write_directory_enter_probe(void);
int NEAR dos_create_probe(const volatile char FAR *path, u16 *handle);
int NEAR dos_open_probe(const volatile char FAR *path, u16 *handle);
u16 NEAR dos_read_probe(
        u16 handle, volatile u8 FAR *destination, u16 byte_count);
u16 NEAR dos_write_probe(
        u16 handle, const volatile u8 FAR *source, u16 byte_count);
void NEAR dos_close_probe(u16 handle);
u32 FAR resource_size_probe(u16 handle);
u16 NEAR patch_build_probe(void);
u16 NEAR patch_apply_probe(u16 byte_count);
i16 FAR profile_select_probe(u16 profile);
i16 FAR vm_run_probe(void);
void FAR record_rebuild_probe(void);
void FAR hud_reset_probe(void);
u32 FAR file_create_and_write_probe(
        const volatile char FAR *path,
        const volatile u8 FAR *source,
        u32 byte_count);

#pragma aux list_widget_probe parm [si] value [ax]
#pragma aux transition_step_probe parm [si] [di] modify exact []
#pragma aux resource_size_probe parm [ax] value [dx ax] modify exact [ax dx]
#pragma aux patch_build_probe value [ax] modify exact [ax]
#pragma aux patch_apply_probe parm [ax] value [ax] modify exact [ax]

void NEAR save_load_menu_step_probe(void)
{
    volatile save_slot_probe NEAR *slot;
    i16 selection;
    u16 slot_offset;
    u16 file_handle;
    u16 byte_count;
    u16 state_size;
    int transition_complete;

    if ((quicksave_request_probe & 1u) != 0u) {
        memcpy((void NEAR *)slots_probe[9].name,
                (const void NEAR *)quick_name_probe, 8u);
        active_name_probe = slots_probe[9].name;
        quicksave_request_probe = 0u;
        goto save_game;
    }

    if ((save_request_probe | load_request_probe) == 0u) {
        return;
    }

    ui_flags_probe |= 4u;
    if ((save_menu_phase_probe & 1u) != 0u) {
        list_editing_probe = 1u;
        (void)list_widget_probe(slot_offsets_probe);
        list_editing_probe = 0u;
        transition_aux_probe = 0u;
        transition_current_probe = 0u;
        transition_total_probe = 6u;
        active_name_probe = slots_probe[0].name;
        selected_index_probe = 0u;
        memcpy((void NEAR *)edit_buffer_probe,
                (const void NEAR *)slots_probe[0].name, 16u);
        ++save_menu_phase_probe;
    }

    if ((save_menu_phase_probe & 2u) != 0u) {
        transition_complete =
                transition_total_probe == transition_current_probe;
        transition_step_probe(
                (const rect_i16_probe NEAR *)current_rect_probe,
                (const rect_i16_probe NEAR *)target_rect_probe);
        if (!transition_complete) {
            return;
        }
        save_menu_phase_probe = 0u;
    }

    selection = list_widget_probe(slot_offsets_probe);
    if ((save_request_probe & 1u) != 0u) {
        name_length_probe = 0u;
        while (edit_buffer_probe[name_length_probe] != 0u
                && edit_buffer_probe[name_length_probe] != (u8)' ') {
            ++name_length_probe;
        }
        if (name_edit_probe()) {
            goto save_game;
        }
        if (selection < 0 || selection == 9) {
            return;
        }
        selected_index_probe = (u16)selection;
        slot_offset = slot_offsets_probe[(u16)selection];
        if (slot_offset == 0xffffu) {
            goto close_menu;
        }
        slot = (volatile save_slot_probe NEAR *)slot_offset;
        active_name_probe = slot->name;
        memcpy((void NEAR *)edit_buffer_probe,
                (const void NEAR *)slot->name, 16u);
        return;
    }

    if (selection < 0) {
        return;
    }
    write_directory_enter_probe();
    slot_offset = slot_offsets_probe[(u16)selection];
    if (slot_offset == 0xffffu) {
        goto close_menu;
    }
    slot = (volatile save_slot_probe NEAR *)slot_offset;
    if (!dos_open_probe((const volatile char FAR *)slot->filename,
            &file_handle)) {
        goto close_menu;
    }
    (void)dos_read_probe(file_handle,
            (volatile u8 FAR *)&profile_request_probe, 2u);
    (void)profile_select_probe((u16)profile_request_probe);
    profile_request_probe = -1;
    vm_enabled_probe = 1u;
    (void)vm_run_probe();
    (void)dos_read_probe(file_handle,
            (volatile u8 FAR *)state_words_probe, 0x200u);
    (void)dos_read_probe(file_handle,
            (volatile u8 FAR *)state_strings_probe, 0x60u);
    state_size = (u16)resource_size_probe(state_resource_handle_probe);
    (void)dos_read_probe(file_handle, record_base_probe, state_size);
    byte_count = dos_read_probe(file_handle, work_surface_probe, 0xffffu);
    (void)patch_apply_probe(byte_count);
    record_rebuild_probe();
    hud_reset_probe();
    redraw_probe = 1u;
    palette_dirty_probe = 1u;
    dos_close_probe(file_handle);
    goto close_menu;

save_game:
    write_directory_enter_probe();
    slot = (volatile save_slot_probe NEAR *)active_name_probe;
    if (!dos_create_probe((const volatile char FAR *)slot->filename,
            &file_handle)) {
        goto close_menu;
    }
    (void)dos_write_probe(file_handle,
            (const volatile u8 FAR *)&current_profile_probe, 2u);
    (void)dos_write_probe(file_handle,
            (const volatile u8 FAR *)state_words_probe, 0x200u);
    (void)dos_write_probe(file_handle,
            (const volatile u8 FAR *)state_strings_probe, 0x60u);
    state_size = (u16)resource_size_probe(state_resource_handle_probe);
    (void)dos_write_probe(file_handle, record_base_probe, state_size);
    byte_count = patch_build_probe();
    (void)dos_write_probe(file_handle, work_surface_probe, byte_count);
    dos_close_probe(file_handle);
    (void)file_create_and_write_probe(
            (const volatile char FAR *)directory_path_probe,
            (const volatile u8 FAR *)slots_probe,
            10u * (u16)sizeof(save_slot_probe));

close_menu:
    ui_flags_probe &= (u8)~4u;
    save_request_probe = 0u;
    load_request_probe = 0u;
}
