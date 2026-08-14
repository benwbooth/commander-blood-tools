#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define SCREEN_TRANSITION_ACTIVE_FLAG 0x01u
#define PRESENTATION_REVERSE_FLAG 0x01u
#define PALETTE_BYTES 768u
#define DARK_REMAP_PERCENT (-50)
#define CONSOLE_PALETTE_BANK 0x00E0u

void CB_NEAR screen_flags_init(void)
{
    nav_screen_rebuild_pending = 0u;
    pbm_palette_refresh = 1u;
    palette_dirty = 1u;
    nav_actor_completion_latch = 0u;
    bloodprg_clip_snapshot_flags = 1u;

    if ((nav_transition_pending & SCREEN_TRANSITION_ACTIVE_FLAG) != 0u) {
        page_flip_transparent_zero = 0u;
        bloodprg_dirty_copy_flags = 0u;
        bridge_panorama_frame_load((cb_u16)vm_bridge_view_frame);
        blit_fill_row_5221(0u);
        entity_object_populate(20u, 11u, 0u, 0u, 0u);
    } else {
        (void)page_flip();
        entity_flag_state_transition(4u);
    }

    pbm_palette_refresh = 0u;
    nav_actor_ship_depth_offset = 0u;
    _fmemcpy(
            (void CB_FAR *)pbm_live_palette,
            (const void CB_FAR *)bridge_panorama_palette,
            PALETTE_BYTES);
    (void)palette_blend_remap_table_build(
            DARK_REMAP_PERCENT,
            0u,
            0u,
            0u,
            graphics_span_remap_table);
    tint_table_build_banked(
            CONSOLE_PALETTE_BANK,
            bloodprg_sprite_remap_6011_gs);

    if ((presentation_mode_flag_27e0 & PRESENTATION_REVERSE_FLAG) == 0u) {
        matrix_table_clear_2a1b();
    }
}
