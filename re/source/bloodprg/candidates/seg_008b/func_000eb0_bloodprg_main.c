#include <dos.h>
#if defined(__WATCOMC__)
#include <i86.h>
#endif

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_ems.h"
#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_hardware.h"
#include "../include/bloodprg_input.h"
#include "../include/bloodprg_list.h"
#include "../include/bloodprg_manu3.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_platform.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_save.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_startup.h"
#include "../include/bloodprg_vm.h"

#define BLOODPRG_ARENA_BYTES 0x00010000UL
#define BLOODPRG_ARENA_WITH_DESCRIPTOR_BYTES 0x00010010UL
#define BLOODPRG_FRAME_TICKS 8u
#define BLOODPRG_INITIAL_SCENE_LINK 0x0010u
#define BLOODPRG_DEFAULT_ACTIVE_LINE 8u

#if defined(__WATCOMC__)
#define BLOODPRG_INTERRUPTS_DISABLE() _disable()
#define BLOODPRG_INTERRUPTS_ENABLE() _enable()
#else
#define BLOODPRG_INTERRUPTS_DISABLE() disable()
#define BLOODPRG_INTERRUPTS_ENABLE() enable()
#endif

void CB_FAR bloodprg_main(void)
{
    union REGS registers;
    bloodprg_resource_allocation_result allocation;
    bloodprg_resource_resolve_result driver_resource;
    volatile bloodprg_viewport_descriptor CB_FAR *viewport;
    cb_u16 scene_link_target;
    cb_u16 file_handle;
    cb_u8 profile_blocked;

    scene_link_target = BLOODPRG_INITIAL_SCENE_LINK;

    allocation = resource_allocate(8u, BLOODPRG_ARENA_BYTES);
    alien_overlay_slot.load_buffer = allocation.destination;

    allocation = resource_allocate(10u, BLOODPRG_ARENA_BYTES);
    graphics_display_buffer = allocation.destination;

    allocation = resource_allocate(11u, BLOODPRG_ARENA_WITH_DESCRIPTOR_BYTES);
    graphics_viewport_descriptor =
            (volatile bloodprg_viewport_descriptor CB_FAR *)
            allocation.destination;
    graphics_back_buffer = (volatile cb_u8 CB_FAR *)MK_FP(
            (cb_u16)(FP_SEG(allocation.destination) + 1u), 0u);

    allocation = resource_allocate(12u, BLOODPRG_ARENA_BYTES);
    resource_copy_buffer = allocation.destination;
    nav_presentation_resource_buffer = (volatile cb_u8 CB_FAR *)MK_FP(
            (cb_u16)(FP_SEG(allocation.destination) + 0x0640u), 0u);

    allocation = resource_allocate(9u, BLOODPRG_ARENA_BYTES);
    graphics_work_surface = allocation.destination;

    allocation = resource_allocate(100u, BLOODPRG_ARENA_WITH_DESCRIPTOR_BYTES);
    snd_bank_memory = allocation.destination;
    snd_stream_storage = (volatile cb_u8 CB_FAR *)MK_FP(
            (cb_u16)(FP_SEG(allocation.destination) + 0x0800u), 0u);

    startup_loading_screen_and_write_directory_prepare();
    resource_archive_index_backing_initialize();
    cdrom_audio_prepare();
    (void)resource_file_load(
            manu3_overlay_path, alien_overlay_slot.load_buffer);

    viewport = graphics_viewport_descriptor;
    viewport->field_00 = 0u;
    viewport->field_02 = 1u;
    viewport->field_04 = 4UL;
    viewport->width = 320u;
    viewport->height = 200u;
    viewport->field_0c = 0UL;

    (void)resource_source_select(bridge_panorama_path);
    if (!cb_dos_open_read_only(bridge_panorama_path, &file_handle)) {
        goto shutdown;
    }
    bridge_panorama_file_handle = file_handle;

    (void)resource_file_load(
            (volatile char CB_FAR *)save_slot_directory_path,
            (volatile cb_u8 CB_FAR *)save_slot_records);

    (void)resource_load_by_id(startup_audio_resource_id);
    if (startup_audio_resource_id != 1u) {
        voc_playback_enabled = 1u;
    }
    driver_resource = resource_handle_resolve(startup_audio_resource_id);
    audio_param_init_cd5((cb_u16)(driver_resource.segment - 0x0010u));
    (void)resource_named_file_load(
            0x002cu, (volatile cb_u8 CB_FAR *)0);

    ship_3d_nav_choice_sound_gate = 0u;
    presentation_mode_flag_27e0 = 1u;
    vm_ui_state.word = 1u;
    nav_screen_rebuild_pending = 1u;
    ship_3d_point_cloud_randomize();
    presentation_line_zero_run(scene_link_target);
    main_presentation_color_table = main_default_presentation_colors;
    snd_bank_loader(0u, default_snd_bank_path);
    (void)back_buffer_init();

    registers.x.ax = 4u;
    registers.x.cx = 720u;
    registers.x.dx = 150u;
    (void)int86(0x33, &registers, &registers);

    for (;;) {
        main_frame_delay_ticks = BLOODPRG_FRAME_TICKS;
        bloodprg_dirty_rect_list[0].left = 0xffffu;
        bloodprg_clip_snapshot_flags = 1u;
        input_action_dispatch();

        if ((main_loop_hud_refresh_enabled & 1u) == 0u
                && (vm_ui_state.word & 8u) == 0u) {
            poll_mouse();
            if ((mouse_press_pending & 3u) == 0u) {
                mouse_primary_pressed = 0u;
                mouse_secondary_pressed = 0u;
                nav_target_selection = 0u;
            } else if ((mouse_press_pending & 2u) != 0u) {
                mouse_press_pending = 0u;
            } else {
                --mouse_press_pending;
            }
        } else {
            mouse_x = mouse_last_x;
            mouse_y = mouse_last_y;
            registers.x.ax = 4u;
            registers.x.cx = (cb_u16)mouse_x;
            registers.x.dx = (cb_u16)mouse_y;
            (void)int86(0x33, &registers, &registers);
        }

        if ((ship_3d_nav_choice_sound_gate & 1u) != 0u) {
            goto shutdown;
        }
        main_loop_hud_refresh();
        if ((main_loop_hud_refresh_enabled & 1u) != 0u) {
            continue;
        }

        (void)mouse_button_edges_update();
        if ((presentation_mode_flag_27e0 & 1u) == 0u
                && vm_run_wrapper() < 0) {
            goto shutdown;
        }

        if (vm_script_profile_request != -1
                && (vm_ui_state.bytes.flags & 0x0eu) == 0u) {
            profile_blocked = vm_presentation_active;
            profile_blocked |= vm_ship_active_flags_low;
            profile_blocked |= render_update_flag_2751;
            profile_blocked |= vm_presentation_defer_a;
            profile_blocked |= vm_text_display_active;
            profile_blocked |= nav_choice_phase;
            profile_blocked |= save_request_active;
            profile_blocked |= load_request_active;
            profile_blocked |= nav_transition_pending;
            profile_blocked |= nav_actor_transition_phase;
            if (profile_blocked == 0u) {
                if (vm_resource_profile_select(
                        (cb_u16)vm_script_profile_request) < 0) {
                    goto shutdown;
                }
                vm_script_profile_request = -1;
                vm_execution_enabled = 1u;
                (void)vm_run_wrapper();
                vm_record_state_proc();
                object_heap_access();
                ship_3d_hud_palette_snapshot_and_camera_reset();
                nav_screen_rebuild_pending = 1u;
                nav_transition_pending = 0u;
            }
        }

        if ((vm_c2_presentation_gate & 1u) == 0u) {
            resource_frame_presented = 1u;
        }

        if ((vm_presentation_active & 1u) != 0u) {
            if ((vm_presentation_defer_a | vm_text_display_active) == 0u) {
                vm_presentation_start_lock = 0u;
                vm_presentation_hold_ready = 1u;
            }
            if ((vm_presentation_hold_ready & 1u) != 0u) {
                if (vm_presentation_word_buffer[0] != 0u) {
                    vm_presentation_word_choice_active = 1u;
                } else {
                    vm_presentation_defer_a = 0u;
                    vm_text_display_active = 0u;
                }
                goto presentation_ownership;
            }
            if ((vm_presentation_defer_a | vm_text_display_active) != 0u) {
                vm_presentation_owner_offset = 0x5e64u;
                if ((vm_text_display_active & 1u) == 0u) {
                    vm_presentation_owner_offset = 0x67b0u;
                }
            }
        }

        if ((vm_dialogue_hold_complete & 1u) != 0u) {
            if (vm_presentation_word_buffer[0] != 0u) {
                vm_presentation_word_choice_active =
                        (vm_presentation_active & 1u) != 0u;
            }
            if (vm_dialogue_hold_countdown != 0u
                    && (mouse_secondary_pressed & 1u) == 0u) {
                goto presentation_ownership;
            }
            vm_dialogue_hold_complete = 0u;
            vm_presentation_hold_ready =
                    (vm_presentation_active & 1u) != 0u;
            if ((vm_presentation_hold_ready & 1u) == 0u) {
                vm_text_display_active = 0u;
                vm_presentation_defer_a = 0u;
            } else {
                vm_presentation_request_flags &= (cb_u8)~1u;
            }
        }

presentation_ownership:
        if ((vm_presentation_active | vm_presentation_hold_ready
                | vm_ship_active_flags_low) == 0u) {
            vm_text_display_active = 0u;
            vm_presentation_defer_a = 0u;
        }

        if ((vm_presentation_active & 1u) != 0u
                && (vm_scene_gate | vm_sequence_active) != 0u
                && (vm_presentation_request_flags & 2u) == 0u) {
            if ((vm_presentation_request_flags
                    | (cb_u8)vm_dialogue_hold_countdown) != 0u) {
                if ((vm_text_menu_pending & 1u) != 0u) {
                    vm_text_menu_pending = 0u;
                    vm_c2_presentation_gate = 0u;
                    vm_active_line = (cb_u16)(vm_text_selector + 9);
                } else if ((vm_c2_presentation_gate & 1u) == 0u) {
                    vm_text_mode_0cfa = 0u;
                    vm_active_line = BLOODPRG_DEFAULT_ACTIVE_LINE;
                }
            } else {
                if (vm_active_line != BLOODPRG_DEFAULT_ACTIVE_LINE) {
                    if ((cb_u16)(list_d8c_entry_metric
                            - list_d8c_read_wrap_index) == 0u) {
                        goto presentation_audio;
                    }
                    scene_link_target = vm_presentation_owner_offset;
                    if (scene_link_target != 0u) {
                        *(volatile cb_u8 CB_NEAR *)scene_link_target = 1u;
                    } else {
                        vm_presentation_defer_a = 1u;
                        vm_text_menu_words =
                                vm_presentation_menu_words_buffer;
                    }
                }
                if ((vm_c2_presentation_gate & 1u) == 0u) {
                    vm_active_line = BLOODPRG_DEFAULT_ACTIVE_LINE;
                }
            }
        }

presentation_audio:
        if ((presentation_completion_audio_pending & 1u) != 0u) {
            presentation_completion_audio_pending = 0u;
            snd_driver_call();
            voc_tablo2_reset_gate = 0x78u;
            snd_stream_source_load(voc_tablo2_path);
            snd_stream_start();
        }

        bridge_render_frame(scene_link_target);
        confirm_dialog_step();
        snd_stream_refill();
        audio_process_ade();
        ship_presentation_fsm();
        scene_transition_step(scene_link_target);
        save_load_menu_step();
        presentation_choice_transition_step();
        if ((vm_presentation_request_flags & 2u) != 0u
                || (vm_presentation_request_flags & 1u) == 0u) {
            vm_text_mode_0cfa = 0u;
            vm_text_voice_trigger = 0u;
        }
        if ((resource_frame_presented & 1u) != 0u) {
            presentation_ready_gate();
        }
        chunky_to_planar_framebuffer(graphics_display_buffer);
        dlg_menu_words_inline_reveal_step();
        subtitle_reveal_pump();
        manu3_hand_frame_dispatch();
        palette_transition_step();
        while (main_frame_delay_ticks != 0u) {
        }
        BLOODPRG_INTERRUPTS_DISABLE();
        page_offset_helper();
        palette_upload_if_dirty();
        BLOODPRG_INTERRUPTS_ENABLE();
    }

shutdown:
    presentation_update_1fb2();
    snd_driver_call();
    presentation_line_one_stream_run(scene_link_target);
    snd_driver_call();
    startup_write_directory_enter();

    if (snd_voice_file_handle != 0u) {
        cb_dos_close(snd_voice_file_handle);
        (void)cb_dos_delete(snd_voice_temp_filename);
    }
    if (snd_bank_file_handle != 0u) {
        cb_dos_close(snd_bank_file_handle);
        (void)cb_dos_delete(snd_music_temp_filename);
    }
    if (resource_archive_cache_handle != 0u) {
        cb_dos_close(resource_archive_cache_handle);
        (void)cb_dos_delete(resource_archive_cache_filename);
    }
    startup_transient_files_delete();
    startup_original_directory_restore();
    if (resource_archive_handle != 0u) {
        cb_dos_close(resource_archive_handle);
    }
}
