/* Codegen probe for BLOODPRG 0x001EC1. */

typedef unsigned char u8;
typedef unsigned int u16;

#define FAR far
#define NEAR near
#define GAME_DATA __based(__segname("GAME_DATA"))

typedef volatile u8 FAR *graphics_buffer_ptr;

extern volatile u8 nav_choice_sound_gate_probe;
extern volatile u8 presentation_gate_probe;
extern volatile u16 active_line_probe;
extern graphics_buffer_ptr GAME_DATA display_buffer_probe;

void FAR blit_fill_row_probe(u16 color);
void FAR back_buffer_fill_probe(u16 color);
void FAR input_action_dispatch_probe(void);
void FAR scene_dispatch_probe(u16 link_target_offset);
void FAR chunky_to_planar_probe(const volatile u8 FAR *source);
void NEAR page_offset_helper_probe(void);
void NEAR palette_upload_if_dirty_probe(void);

#pragma aux blit_fill_row_probe parm [ax] modify exact []
#pragma aux back_buffer_fill_probe parm [ax] modify exact []
#pragma aux chunky_to_planar_probe parm [ds si] modify exact [dx]
#pragma aux page_offset_helper_probe modify exact [ax dx]
#pragma aux palette_upload_if_dirty_probe modify exact [ax bx cx dx si di es]

void NEAR presentation_line_zero_run_probe(u16 link_target_offset)
{
    blit_fill_row_probe(0u);
    back_buffer_fill_probe(0u);
    active_line_probe = 0u;

    for (;;) {
        input_action_dispatch_probe();
        if ((nav_choice_sound_gate_probe & 1u) != 0u) {
            break;
        }

        scene_dispatch_probe(link_target_offset);
        if ((presentation_gate_probe & 1u) == 0u) {
            break;
        }

        chunky_to_planar_probe(display_buffer_probe);
        page_offset_helper_probe();
        palette_upload_if_dirty_probe();
    }

    nav_choice_sound_gate_probe = 0u;
    presentation_gate_probe = 0u;
    active_line_probe = 0xffffu;
}
