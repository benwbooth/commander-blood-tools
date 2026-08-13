/* Codegen probe for the AMER slot-2 steering callback. */
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

typedef struct alien_state alien_state;
typedef struct alien_context alien_context;
typedef void NEAR alien_callback(alien_state NEAR *, alien_context NEAR *);
typedef alien_callback NEAR *alien_callback_pointer;

struct alien_state {
    u8 field_000[0x0e];
    alien_callback_pointer callback;
    u8 field_010[0x0a];
    i32 field_01a;
    u8 field_01e[0x14];
    i32 field_032;
    u8 field_036[0x02];
    i16 field_038;
    u8 field_03a[0x06];
    u16 field_040;
    u8 field_042[0x0e];
    u16 field_050;
    u8 field_052[0x04];
    i16 countdown;
};

struct alien_context {
    u8 unused;
};

extern volatile i16 camera_depth_step;
extern alien_callback finish_update;

void NEAR xdb_amer_slot2_steer_update_probe(
        alien_state NEAR *state,
        alien_context NEAR *context);

#if defined(__WATCOMC__)
#pragma aux alien_callback parm [si] [di] modify exact [ax bx cx dx]
#pragma aux xdb_amer_slot2_steer_update_probe \
        parm [si] [di] modify exact [ax bx cx dx]
#endif

void NEAR xdb_amer_slot2_steer_update_probe(
        alien_state NEAR *state,
        alien_context NEAR *context)
{
    u32 first_factor =
            (u32)(i32)(i16)state->field_040 -
            (u16)camera_depth_step - 0x03e8UL;
    u32 score;
    u16 countdown;

    (void)context;
    first_factor = 0UL - first_factor;
    score = first_factor * (u32)state->field_01a;
    score += (u32)(i32)state->field_038 * (u32)state->field_032;
    state->field_050 += (i32)score < 0 ? 0x20u : 0xffe0u;

    countdown = (u16)state->countdown - 1u;
    state->countdown = (i16)countdown;
    if ((countdown & 0x8000u) != 0) {
        state->callback = finish_update;
        state->countdown = 0x40;
    }
}
