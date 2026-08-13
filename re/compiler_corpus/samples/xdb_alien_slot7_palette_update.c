/* Codegen probe for the alien slot-7 object and palette update. */
#include <dos.h>

typedef unsigned char xdb_u8;
typedef unsigned int xdb_u16;
typedef signed char xdb_i8;
typedef signed int xdb_i16;
typedef unsigned long xdb_u32;
typedef signed long xdb_i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define XDB_FAR far
#define XDB_NEAR near
#define XDB_FAR_AT(type, segment, offset) \
    ((type XDB_FAR *)MK_FP((segment), (offset)))
#else
#define XDB_FAR
#define XDB_NEAR
#endif

#if defined(__WATCOMC__)
#define XDB_CODE_DATA __based(__segname("_CODE"))
#else
#define XDB_CODE_DATA XDB_FAR
#endif

typedef struct xdb_alien_slot7_root_state {
    xdb_u8 field_000[0x12];
    xdb_i32 field_012;
    xdb_u8 field_016[0x0c];
    xdb_i32 field_022;
    xdb_u8 field_026[0x0c];
    xdb_i32 field_032;
    xdb_i32 field_036;
    xdb_i32 field_03a;
} xdb_alien_slot7_root_state;

typedef struct xdb_alien_slot7_state {
    xdb_alien_slot7_root_state XDB_NEAR *root;
    xdb_u8 field_002[0x34];
    xdb_i32 field_036;
    xdb_i32 field_03a;
    xdb_i32 field_03e;
    xdb_i32 position_x;
    xdb_i32 position_y;
    xdb_u8 field_04a[0x04];
    xdb_i16 mouse_y;
    xdb_u16 mouse_x_0;
    xdb_u16 mouse_x_1;
} xdb_alien_slot7_state;

typedef union xdb_alien_palette_cycle {
    struct {
        xdb_i8 step;
        xdb_i8 countdown;
    } fields;
    xdb_u16 word;
} xdb_alien_palette_cycle;

typedef struct xdb_alien_state {
    xdb_u8 unused;
} xdb_alien_state;

typedef struct xdb_alien_method_context {
    xdb_u8 field_00[0x16];
    volatile xdb_alien_state XDB_NEAR *state;
} xdb_alien_method_context;

extern volatile xdb_u16 xdb_alien_mouse_x;
extern volatile xdb_u16 xdb_alien_mouse_y;
extern volatile xdb_i16 XDB_CODE_DATA xdb_alien_method_delta;
extern volatile xdb_u16 XDB_CODE_DATA xdb_alien_code_flags;
extern volatile xdb_u16 XDB_CODE_DATA xdb_alien_palette_previous_level;
extern volatile xdb_alien_palette_cycle XDB_CODE_DATA
        xdb_alien_palette_cycle_state;
extern const volatile xdb_u8 XDB_CODE_DATA xdb_croolis_palette_remap[256];
extern volatile xdb_u16 xdb_alien_palette_segment;
extern volatile xdb_u16 xdb_alien_palette_pulse_0;
extern volatile xdb_u16 xdb_alien_palette_pulse_1;
extern volatile xdb_u16 xdb_alien_palette_pulse_2;

void XDB_NEAR xdb_alien_method_slot_7_palette_update_probe(
        xdb_alien_method_context XDB_NEAR *context);

#if defined(__WATCOMC__)
#pragma aux xdb_alien_method_slot_7_palette_update_probe \
        parm [di] modify exact [ax bx cx dx si es]
#endif


void XDB_NEAR xdb_alien_method_slot_7_palette_update_probe(
        xdb_alien_method_context XDB_NEAR *context)
{
    volatile xdb_alien_slot7_root_state XDB_NEAR *root;
    volatile xdb_alien_slot7_state XDB_NEAR *state;
    volatile xdb_u8 XDB_FAR *palette;
    xdb_alien_palette_cycle cycle;
    xdb_u32 horizontal;
    xdb_u32 vertical;
    xdb_u32 scaled;
    xdb_u16 current;
    xdb_u16 lower;
    xdb_u16 upper;
    xdb_u16 high_lower;
    xdb_u16 high_upper;
    xdb_u16 page;
    xdb_u16 offset;
    xdb_u16 index;
    xdb_u16 swap;
    xdb_u16 pulse_shift;
    xdb_u8 next;
    xdb_u8 countdown;

    root = (volatile xdb_alien_slot7_root_state XDB_NEAR *)context->state;
    root->field_036 = 0;
    root->field_03a = 0;
    root->field_03a = 0;
    root->field_012 = 0x00008000L;
    root->field_022 = 0x00008000L;
    root->field_032 = 0x00008000L;

    state = (volatile xdb_alien_slot7_state XDB_NEAR *)(
            (volatile xdb_u8 XDB_NEAR *)root + 0x005eu);
    state->root = (xdb_alien_slot7_root_state XDB_NEAR *)root;
    scaled = (xdb_u32)state->field_03e;
    scaled = (xdb_u32)((xdb_i32)scaled >> 8);
    vertical = (0UL - 60UL) * scaled;
    horizontal = scaled * (xdb_u32)(xdb_i32)(xdb_i16)xdb_alien_mouse_x;
    state->mouse_x_1 = (xdb_u16)(xdb_alien_mouse_x << 2);
    state->mouse_x_0 = (xdb_u16)(xdb_alien_mouse_x << 2);
    state->mouse_y = (xdb_i16)(0u - xdb_alien_mouse_y);
    horizontal = (xdb_u32)((xdb_i32)horizontal >> 2);
    horizontal -= (xdb_u32)state->field_036;
    vertical -= (xdb_u32)state->field_03a;
    horizontal = (xdb_u32)((xdb_i32)horizontal >> 16);
    vertical = (xdb_u32)((xdb_i32)vertical >> 16);
    state->position_x = (xdb_i32)((xdb_u32)state->position_x + horizontal);
    state->position_y = (xdb_i32)((xdb_u32)state->position_y + vertical);

    current = xdb_alien_code_flags;
    if (current != 0u) {
        current = (xdb_u16)(current - 1u);
        xdb_alien_code_flags = current;
        pulse_shift = current & 3u;
        xdb_alien_palette_pulse_0 = (xdb_u16)(10u << pulse_shift);
        xdb_alien_palette_pulse_1 = (xdb_u16)(13u << pulse_shift);
        xdb_alien_palette_pulse_2 = (xdb_u16)(11u << pulse_shift);
    }

    current = (xdb_u16)xdb_alien_method_delta;
    if (current > 0x0080u) {
        return;
    }
    lower = 0x0080u - current;
    upper = 0x0080u - xdb_alien_palette_previous_level;
    xdb_alien_palette_previous_level = current;
    cycle.word = xdb_alien_palette_cycle_state.word;
    next = (xdb_u8)((xdb_u8)current + (xdb_u8)cycle.fields.step);
    if ((xdb_i8)next < 0) {
        return;
    }
    countdown = (xdb_u8)cycle.fields.countdown - 1u;
    if ((xdb_i8)countdown < 0) {
        countdown = 3u;
        cycle.fields.step = (xdb_i8)(0u - (xdb_u8)cycle.fields.step);
    }
    cycle.fields.countdown = (xdb_i8)countdown;
    xdb_alien_palette_cycle_state.word = cycle.word;
    xdb_alien_method_delta = (xdb_i16)next;

    if (lower == upper) {
        return;
    }
    if ((xdb_i16)lower > (xdb_i16)upper) {
        swap = lower;
        lower = upper;
        upper = swap;
    }
    palette = XDB_FAR_AT(xdb_u8, xdb_alien_palette_segment, 0);

    high_lower = lower >= 0x003fu ? lower - 0x003fu : 0u;
    high_upper = upper >= 0x003fu ? upper - 0x003fu : 0u;
    for (page = high_lower; page != high_upper; ++page) {
        offset = (xdb_u16)((page << 8) + 0x001eu);
        for (index = 0; index < 0x00e2u; ++index) {
            palette[offset] = xdb_croolis_palette_remap[palette[offset]];
            ++offset;
        }
    }

    lower = lower <= 0x003fu ? lower : 0x003fu;
    upper = upper <= 0x003fu ? upper : 0x003fu;
    for (page = lower; page != upper; ++page) {
        offset = (xdb_u16)(page << 8);
        for (index = 0; index < 0x001eu; ++index) {
            palette[offset] = xdb_croolis_palette_remap[palette[offset]];
            ++offset;
        }
    }
}
