#include "../include/xdb_alien.h"
#include "../include/xdb_mouse.h"

void XDB_NEAR xdb_scrut_method_slot_7_palette_update(
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
        xdb_alien_palette_pulse_0.words.low = (xdb_u16)(10u << pulse_shift);
        xdb_alien_palette_pulse_1.words.low = (xdb_u16)(13u << pulse_shift);
        xdb_alien_palette_pulse_2.words.low = (xdb_u16)(11u << pulse_shift);
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
            palette[offset] = xdb_scrut_palette_remap[palette[offset]];
            ++offset;
        }
    }

    lower = lower <= 0x003fu ? lower : 0x003fu;
    upper = upper <= 0x003fu ? upper : 0x003fu;
    for (page = lower; page != upper; ++page) {
        offset = (xdb_u16)(page << 8);
        for (index = 0; index < 0x001eu; ++index) {
            palette[offset] = xdb_scrut_palette_remap[palette[offset]];
            ++offset;
        }
    }
}
