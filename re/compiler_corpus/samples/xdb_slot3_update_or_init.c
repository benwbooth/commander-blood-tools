/* Codegen probe for the complete XDB slot-3 update/initializer method. */
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
typedef struct alien_method_context alien_method_context;
typedef void NEAR alien_state_function(
        alien_biased_state NEAR *state,
        alien_method_context NEAR *context);
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

typedef struct alien_state {
    u8 field_000[0x0b0];
} alien_state;

typedef struct alien_ring_entry {
    i16 field_000;
    i16 field_002;
    i16 field_004;
    i16 field_006;
} alien_ring_entry;

struct alien_method_context {
    u8 field_000[0x16];
    alien_state NEAR *state;
    u8 field_018[2];
    u16 state_count;
    u8 field_01c[0x1a];
    i16 control_state;
};

extern volatile u16 CODE_DATA slot3_timer;
extern volatile u16 CODE_DATA slot3_generation;
extern volatile u16 CODE_DATA slot3_ring_cursor;
extern volatile alien_ring_entry CODE_DATA slot3_ring[];
extern alien_state_function slot3_initial_update;
extern alien_state_function slot3_update;

void NEAR xdb_slot3_update_or_init_probe(
        alien_method_context NEAR *context);

#if defined(__WATCOMC__)
#pragma aux alien_state_function parm [si] [di] modify exact [ax bx cx dx]
#pragma aux xdb_slot3_update_or_init_probe \
        parm [di] modify exact [ax bx cx dx si di bp]
#endif

void NEAR xdb_slot3_update_or_init_probe(
        alien_method_context NEAR *context)
{
    alien_biased_state NEAR *state = (alien_biased_state NEAR *)
            ((u8 NEAR *)context->state + 0x005e);
    u16 count = context->state_count;
    u16 ring_cursor;
    u16 phase;

    if (context->control_state == 0) {
        context->control_state = 1;
        ring_cursor = slot3_ring_cursor;
        slot3_timer = 7;
        state->position_x = 0;
        state->position_y = 0x06a4L;
        state->position_z = 0;
        state->callback = slot3_initial_update;
        state->field_056 = 0x19;
        state->field_058 = 0;
        state->ring_offset = ring_cursor;
        state->field_05c = 0xa957u;
        state->field_04e = 0;
        state->field_050 = 0;
        state->field_052 = 0;
        state->field_054 = 0;
        slot3_ring[ring_cursor >> 3].field_000 = 0;
        slot3_ring[ring_cursor >> 3].field_002 = 0;
        slot3_ring[ring_cursor >> 3].field_004 = 0x46;
        slot3_ring[ring_cursor >> 3].field_006 = 0;
        if (--count == 0) {
            slot3_ring_cursor = (ring_cursor - 8u) & 0x03fcu;
            return;
        }
        ring_cursor -= 8u;
        if (++slot3_generation != 0) {
            context->control_state = -1;
            state->callback = slot3_update;
            slot3_ring[ring_cursor >> 3].field_004 = 0;
            state->field_04e = 0;
            state->field_050 = 0;
            state->field_052 = 0;
            state->position_x = 0;
            state->position_y = 0x06a4L;
            state->position_z = 0;
        }
        phase = 0;
        do {
            ++state;
            ring_cursor = (ring_cursor - 8u) & 0x03ffu;
            phase += 0x0100u;
            state->callback = slot3_update;
            state->field_058 = phase;
            state->ring_offset = ring_cursor;
            state->field_05c = 0;
            slot3_ring[ring_cursor >> 3].field_000 = 0;
            slot3_ring[ring_cursor >> 3].field_002 = 0;
            slot3_ring[ring_cursor >> 3].field_004 = 0;
            slot3_ring[ring_cursor >> 3].field_006 = 0;
            state->field_04e = 0;
            state->field_050 = 0;
            state->field_052 = 0;
            state->field_054 = 0;
            state->position_x = 0;
            state->position_y = 0x06a4L;
            state->position_z = 0;
        } while (--count != 0);
        slot3_ring_cursor = (ring_cursor - 8u) & 0x03fcu;
        return;
    }

    if (context->control_state >= 0) {
        --slot3_timer;
        if (slot3_timer & 0x8000u) {
            slot3_timer = 7;
        }
    }
    do {
        state->callback(state, context);
        ++state;
    } while (--count != 0);
}
