#include <dos.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_input.h"
#include "../include/bloodprg_manu3.h"
#include "../include/bloodprg_platform.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define ALIEN_OVERLAY_TRIGGER 0x01u
#define ALIEN_OVERLAY_COUNT 3u
#define SHIP_3D_SEQUENCE_ACTIVE 0x01u

void CB_FAR alien_overlay_cycle(void)
{
    bloodprg_snd_bank_header saved_sound_header;
    cb_u16 saved_loader_flags;
    cb_i16 saved_mouse_x;
    cb_i16 saved_mouse_y;
    cb_u8 current_overlay;
    cb_u8 next_overlay;

    if ((ship_3d_temp_snd_trigger & ALIEN_OVERLAY_TRIGGER) == 0u) {
        return;
    }

    ship_3d_temp_snd_trigger = 0u;
    ship_3d_alien_overlay_armed = 0u;
    saved_mouse_x = mouse_x;
    saved_mouse_y = mouse_y;

    current_overlay = alien_overlay_index;
    next_overlay = (cb_u8)(current_overlay + 1u);
    if (next_overlay == ALIEN_OVERLAY_COUNT) {
        next_overlay = 0u;
    }
    alien_overlay_index = next_overlay;

    (void)resource_file_load(
            alien_overlay_paths[current_overlay],
            alien_overlay_slot.load_buffer);

    saved_sound_header = snd_bank_header_ds;
    snd_bank_loader(0u, ship_3d_snd_bank_path);

    alien_overlay_request.timing_scale =
            (volatile cb_u16 CB_FAR *)MK_FP(
                    FP_SEG(vm_record_base), vm_named_vbio_object);
    saved_loader_flags = snd_loader_flags_word;
    snd_driver_pending_flag = 0u;
    cdrom_audio_play_track_2();
    alien_overlay_slot.alien_entry(&alien_overlay_request);
    cdrom_audio_stop();
    snd_loader_flags_word = saved_loader_flags;

    snd_bank_loader(0u, default_snd_bank_path);
    snd_bank_header_ds = saved_sound_header;
    (void)resource_file_load(
            manu3_overlay_path,
            alien_overlay_slot.load_buffer);
    blit_fill_row_5221(0u);

    graphics_viewport_descriptor->field_00 = 0u;
    graphics_viewport_descriptor->field_02 = 1u;
    graphics_viewport_descriptor->field_04 = 4ul;
    graphics_viewport_descriptor->width = 320u;
    graphics_viewport_descriptor->height = 200u;
    graphics_viewport_descriptor->field_0c = 0ul;
    mouse_motion_idle_counter_ds = 0u;
    palette_dirty = 1u;
    mouse_y = saved_mouse_y;
    mouse_x = saved_mouse_x;

    if ((vm_sequence_active_ds & SHIP_3D_SEQUENCE_ACTIVE) != 0u) {
        ship_3d_plane_blit_crop_enabled_ds = 0u;
        (void)backbuffer_clear_flags();
        ship_3d_plane_blit_crop_enabled_ds = 1u;
        vm_loaded_scene_image_path = (volatile char CB_NEAR *)0xffffu;
    } else {
        (void)back_buffer_init();
        pbm_palette_refresh_ds = 0u;
        pbm_transparent_zero_ds = 0u;
        (void)pbm_image_load_and_decode_c(
                scene_transition_image_path_ds,
                graphics_back_buffer_ds);
    }
}
