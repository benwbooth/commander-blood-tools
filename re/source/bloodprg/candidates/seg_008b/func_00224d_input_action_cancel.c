#include <string.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_input.h"
#include "../include/bloodprg_list.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define INPUT_CANCEL_PRESENTATION_GATE 0x01u
#define INPUT_CANCEL_SHIP_BLOCK 0x04u
#define INPUT_CANCEL_LINE_FIRST_BLOCKED 8u
#define INPUT_CANCEL_LINE_LAST_BLOCKED 40u
#define INPUT_CANCEL_DIALOGUE_READY_LINE 4u
#define INPUT_CANCEL_PALETTE_BYTES 384u

void CB_NEAR input_action_cancel(cb_u8 raw_low_byte)
{
    main_loop_hud_refresh_enabled = 0u;
    if ((vm_c2_presentation_gate & INPUT_CANCEL_PRESENTATION_GATE) != 0u
            && (ship_3d_dialogue_phase_ready
                & INPUT_CANCEL_PRESENTATION_GATE) == 0u
            && (vm_ship_active_flags & INPUT_CANCEL_SHIP_BLOCK) == 0u
            && (vm_active_line < INPUT_CANCEL_LINE_FIRST_BLOCKED
                || vm_active_line > INPUT_CANCEL_LINE_LAST_BLOCKED)) {
        ship_3d_dialogue_phase_ready =
                vm_active_line == INPUT_CANCEL_DIALOGUE_READY_LINE;
        resource_source_offset = resource_index_start;
        resource_source_remaining = resource_index_remaining;
        list_d8c_init();
        memset((void CB_GAME_DATA *)scene_palette_dwords,
                0, INPUT_CANCEL_PALETTE_BYTES);
        palette_dirty = 1u;
        return;
    }

    input_action_latch_text_key(raw_low_byte);
}
