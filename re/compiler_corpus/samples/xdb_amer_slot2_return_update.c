/* Codegen probe for the AMER slot-2 return-transition callback. */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;
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

typedef struct alien_state alien_state;
typedef struct alien_context alien_context;
typedef void NEAR alien_callback(alien_state NEAR *, alien_context NEAR *);
typedef alien_callback NEAR *alien_callback_pointer;

struct alien_state {
    u8 field_000[0x0e];
    alien_callback_pointer callback;
    u8 field_010[0x32];
    i32 position_x;
    i32 position_y;
    i32 position_z;
    i16 field_04e;
    u16 field_050;
    i16 field_052;
    i16 field_054;
};

struct alien_context {
    u8 field_000[0x36];
    i16 control_state;
    i16 countdown;
    i16 velocity_x;
    i16 velocity_y;
    i16 velocity_z;
};

extern alien_callback base_update;
extern volatile u16 CODE_DATA slot2_active;

void NEAR xdb_amer_slot2_return_update_probe(
        alien_state NEAR *state,
        alien_context NEAR *context);

#if defined(__WATCOMC__)
#pragma aux alien_callback parm [si] [di] modify exact [ax bx cx dx]
#pragma aux xdb_amer_slot2_return_update_probe \
        parm [si] [di] modify exact [ax bx cx dx]
#endif

void NEAR xdb_amer_slot2_return_update_probe(
        alien_state NEAR *state,
        alien_context NEAR *context)
{
    u16 countdown = (u16)context->countdown - 1u;
    i16 horizontal;
    i16 vertical;

    context->countdown = (i16)countdown;
    state->field_054 = 0;
    if ((countdown & 0x8000u) == 0) {
        state->field_050 += 0x80u;
        state->field_052 = (i16)((u16)state->field_052 - 0x75u);
        state->position_x = (i32)((u32)state->position_x + context->velocity_x);
        state->position_y = (i32)((u32)state->position_y + context->velocity_y);
        state->position_z = (i32)((u32)state->position_z + context->velocity_z);
        return;
    }

    context->control_state = 1;
    context->countdown = 0x20;
    horizontal = (i16)(u16)(state->field_050 << 4);
    horizontal >>= 4;
    vertical = (i16)(u16)((u16)state->field_052 << 4);
    vertical >>= 4;
    state->field_050 = (u16)horizontal;
    state->field_052 = vertical;
    context->velocity_x = (i16)(0u - (u16)vertical);
    context->velocity_x >>= 5;
    state->callback = base_update;
    slot2_active = 0;
}
