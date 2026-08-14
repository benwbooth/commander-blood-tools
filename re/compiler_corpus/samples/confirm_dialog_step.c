/* Codegen probe for BLOODPRG 0x0014CA. */

typedef unsigned char u8;
typedef signed int i16;
typedef unsigned int u16;

#define FAR far
#define NEAR near

typedef struct rect_i16_probe {
    i16 x;
    i16 y;
    i16 width;
    i16 height;
} rect_i16_probe;

typedef union ui_state_probe {
    u16 word;
    struct {
        u8 flags;
        u8 auxiliary;
    } bytes;
} ui_state_probe;

extern volatile u8 confirm_gate_probe;
extern volatile u16 confirm_state_probe;
extern volatile ui_state_probe ui_state_value_probe;
extern volatile u8 mouse_primary_probe;
extern volatile u8 mouse_pending_probe;
extern const u8 question_probe[];
extern const u8 yes_probe[];
extern const u8 no_probe[];
extern const rect_i16_probe yes_region_probe;
extern const rect_i16_probe no_region_probe;

void FAR framebuffer_rect_fill_probe(
        u8 color, u16 x, u16 y, u16 width, u16 height);
void FAR composite_draw_probe(
        u8 color, u16 x, u16 y, u16 width, u16 height);
void FAR text_draw_probe(const u8 FAR *text, u16 x, u16 y, u8 color);
int FAR region_hittest_probe(const rect_i16_probe NEAR *rect);

#pragma aux framebuffer_rect_fill_probe \
        parm caller [ax] [bx] [cx] [dx] modify exact []
#pragma aux composite_draw_probe parm [ax] [bx] [cx] [dx] modify exact []
#pragma aux text_draw_probe parm [ds si] [bx] [dx] [ax] modify exact []

void NEAR confirm_dialog_step_probe(void)
{
    if ((confirm_gate_probe & 2u) == 0u) {
        return;
    }

    confirm_state_probe = 1u;
    ui_state_value_probe.bytes.flags |= 4u;
    framebuffer_rect_fill_probe(0xe2u, 90u, 80u, 140u, 40u);
    composite_draw_probe(0xe8u, 90u, 80u, 140u, 40u);
    text_draw_probe(question_probe, 100u, 88u, 0xe8u);
    text_draw_probe(yes_probe, 120u, 105u, 0xe8u);
    text_draw_probe(no_probe, 180u, 105u, 0xe8u);

    if (region_hittest_probe(&yes_region_probe)) {
        --confirm_gate_probe;
    } else if (region_hittest_probe(&no_region_probe)) {
        confirm_gate_probe = 0u;
        ui_state_value_probe.word &= (u16)~4u;
        confirm_state_probe = 11u;
        mouse_primary_probe = 0u;
        mouse_pending_probe = 0u;
    }
}
