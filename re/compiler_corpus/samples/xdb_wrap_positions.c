/* Codegen probe for the shared XDB slot-6 position-wrap method. */
#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef signed long i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

typedef struct alien_biased_state {
    u8 field_000[0x42];
    i32 position_x;
    i32 position_y;
    i32 position_z;
    u8 field_04e[0x10];
} alien_biased_state;

typedef struct alien_state {
    u8 field_000[0x0b0];
    i16 field_0b0;
} alien_state;

typedef struct alien_method_context {
    u8 field_000[0x16];
    alien_state NEAR *state;
    u8 field_018[0x02];
    u16 state_count;
} alien_method_context;

extern volatile i16 view_x;
extern volatile i16 view_y;
extern volatile i16 view_z;

void NEAR xdb_wrap_positions_probe(alien_method_context NEAR *context);

#if defined(__WATCOMC__)
#pragma aux xdb_wrap_positions_probe \
        parm [di] modify exact [ax bx cx dx si di bp]
#endif

void NEAR xdb_wrap_positions_probe(alien_method_context NEAR *context)
{
    alien_biased_state NEAR *state =
            (alien_biased_state NEAR *)
            ((u8 NEAR *)context->state + 0x005e);
    u16 count = context->state_count;
    u16 value;

    do {
        value = (u16)state->position_x + (u16)view_x;
        value = ((value + 0x4000u) & 0x7fffu) - 0x4000u;
        state->position_x = (i16)(value - (u16)view_x);

        value = (u16)state->position_y + (u16)view_y;
        value = ((value + 0x4000u) & 0x7fffu) - 0x4000u;
        state->position_y = (i16)(value - (u16)view_y);

        value = (u16)state->position_z + (u16)view_z;
        value = ((value + 0x4000u) & 0x7fffu) - 0x4000u;
        state->position_z = (i16)(value - (u16)view_z);

        ++state;
    } while (--count != 0u);
}
