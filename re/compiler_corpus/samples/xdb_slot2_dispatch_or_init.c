/* Codegen probe for the CROOLIS XDB slot-2/4 method. */
#include <stdlib.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef signed long i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

#if defined(__WATCOMC__)
#define CODE_DATA __based(__segname("_CODE"))
#else
#define CODE_DATA
#endif

typedef struct alien_biased_state alien_biased_state;
typedef void NEAR alien_state_function(alien_biased_state NEAR *state);
typedef alien_state_function NEAR *alien_state_callback;

struct alien_biased_state {
    u8 field_000[0x0e];
    alien_state_callback callback;
    u8 field_010[0x32];
    i32 position_x;
    i32 position_y;
    i32 position_z;
    i16 field_04e;
    u16 field_050;
    i16 field_052;
    i16 field_054;
    i16 field_056;
    u16 field_058;
    u16 ring_offset;
    u16 field_05c;
};

typedef struct alien_method_context {
    u8 field_000[0x16];
    u8 NEAR *state;
    u8 field_018[0x02];
    u16 state_count;
    u8 field_01c[0x1a];
    i16 control_state;
    u16 duration;
    u16 field_03a;
    i32 signed_seed;
    u8 field_040[0x02];
    u16 random_value;
} alien_method_context;

extern volatile u16 random_state;
extern volatile i16 CODE_DATA slot2_seed;
extern alien_state_function slot2_update;

void NEAR xdb_slot2_dispatch_or_init_probe(
        alien_method_context NEAR *context);

#if defined(__WATCOMC__)
#pragma aux alien_state_function parm [si] modify exact [ax bx cx dx]
#pragma aux xdb_slot2_dispatch_or_init_probe \
        parm [di] modify exact [ax bx cx dx si di bp]
#endif

void NEAR xdb_slot2_dispatch_or_init_probe(
        alien_method_context NEAR *context)
{
    alien_biased_state NEAR *state = (alien_biased_state NEAR *)
            (context->state + 0x005e);
    u16 value;
    u16 remaining;

    if (context->control_state != 0) {
        state->callback(state);
        return;
    }

    value = random_state;
    value = _rotr(value, 7);
    value += (i16)value >> 15;
    random_state = value;
    context->control_state = 1;
    context->duration = 0x32;
    context->field_03a = 0;
    context->signed_seed = slot2_seed;
    slot2_seed += 0x00fa;
    value = _rotr(value, 7);
    value += (i16)value >> 15;
    context->random_value = value;
    state->field_050 = value & 0x0ffcu;
    state->field_052 = 0;
    state->field_054 = 0;
    state->callback = slot2_update;
    state->field_056 = 0;
    state->field_058 = 0;

    remaining = context->state_count - 1u;
    do {
        ++state;
        state->field_056 = (i16)(u16)state->position_z;
    } while (--remaining != 0);
}
