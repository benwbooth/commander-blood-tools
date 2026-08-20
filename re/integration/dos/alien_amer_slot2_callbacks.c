#include <stdio.h>
#include <string.h>

#include "xdb_alien.h"

#define RESULT_FILE "RESULT.TXT"

typedef union extended_state {
    xdb_u16 alignment;
    xdb_u8 bytes[0x0300];
} extended_state;

typedef union extended_context {
    xdb_u16 alignment;
    xdb_u8 bytes[0x0060];
} extended_context;

xdb_u16 XDB_NEAR xdb_test_slot1_selection_state(void);
void XDB_NEAR xdb_test_set_slot1_selection_state(xdb_u16 state);
void XDB_NEAR xdb_test_set_method_delta(xdb_i16 delta);

static int write_result(const char *status)
{
    FILE *result = fopen(RESULT_FILE, "w");

    if (result == NULL) {
        return 90;
    }
    fputs(status, result);
    fputc('\n', result);
    fclose(result);
    return status[0] == 'P' ? 0 : 1;
}

static xdb_u16 ror7_sbb_zero(xdb_u16 value)
{
    xdb_u16 rotated = (xdb_u16)((value >> 7) | (value << 9));

    return (xdb_u16)(rotated - ((value >> 6) & 1u));
}

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

static const char *check_reset(void)
{
    extended_state state_space;
    extended_context context_space;
    xdb_alien_biased_state *state;
    xdb_alien_method_context *context;
    xdb_u16 expected_seed;

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    context = (xdb_alien_method_context *)context_space.bytes;
    context->continuation.amer_slot2.random_value = 0x1234;
    context->continuation.amer_slot2_motion.velocity_x = 0x7777;
    state->field_052 = 0x1111;
    state->field_05c = 0x2222;
    expected_seed = ror7_sbb_zero(0x1234);
    xdb_amer_slot2_reset(state, context);
    if (context->continuation.amer_slot2.random_value != expected_seed
            || context->continuation.amer_slot2_motion.velocity_x != 0
            || state->field_04e != sar16((xdb_i16)expected_seed, 6u)
            || state->field_052 != 0
            || state->field_05c != 0
            || state->field_054 != 0x3c
            || state->field_056 != 0x20
            || state->callback != xdb_amer_slot2_steer_update) {
        return "FAIL amer slot2 reset";
    }
    return NULL;
}

static const char *check_countdown_common_tail(void)
{
    extended_state state_space;
    extended_context context_space;
    xdb_alien_biased_state *state;
    xdb_alien_method_context *context;

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    context = (xdb_alien_method_context *)context_space.bytes;
    state->field_038 = 0x100;
    state->field_03c = 0x100;
    state->field_040 = 0x100;
    state->field_058 = 0x20;
    context->continuation.amer_slot2.field_038 = 2;
    context->continuation.amer_slot2_motion.velocity_x = 8;
    xdb_test_set_method_delta(-1);
    xdb_amer_slot2_update(state, context);
    if (context->continuation.amer_slot2.field_038 != 1
            || state->field_054 != 4
            || state->field_052 != 8
            || state->field_050 != 1
            || *(xdb_u16 *)(context_space.bytes + 0x42) != 0x84
            || *(xdb_i16 *)(state_space.bytes + 0x0ac)
                    != (xdb_i16)0xfe00u) {
        return "FAIL amer slot2 common tail";
    }
    return NULL;
}

static const char *check_selection_transitions(void)
{
    extended_state state_space;
    extended_context context_space;
    xdb_alien_biased_state *state;
    xdb_alien_method_context *context;

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    context = (xdb_alien_method_context *)context_space.bytes;
    state->field_03c = 0x100;
    state->field_040 = 0x0400;
    xdb_test_set_slot1_selection_state(1);
    xdb_amer_slot2_selection_wait(state, context);
    if (state->callback != xdb_amer_slot2_selection_update
            || state->field_058 != 0x28
            || state->field_050 != 0xffc0u
            || state->field_054 != 5) {
        return "FAIL amer slot2 selection invoke";
    }

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    context = (xdb_alien_method_context *)context_space.bytes;
    state->field_038 = 0x100;
    state->field_03c = 0x100;
    state->field_040 = 0x100;
    context->continuation.amer_slot2.field_038 = 1;
    xdb_test_set_method_delta(-1);
    xdb_test_set_slot1_selection_state(0);
    xdb_amer_slot2_selection_update(state, context);
    if (state->callback != xdb_amer_slot2_update
            || state->field_058 != 0x14
            || state->field_054 != 2) {
        return "FAIL amer slot2 restart";
    }
    return NULL;
}

static const char *check_finish_transition(void)
{
    extended_state state_space;
    extended_context context_space;
    xdb_alien_biased_state *state;
    xdb_alien_method_context *context;

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    context = (xdb_alien_method_context *)context_space.bytes;
    state->field_056 = 1;
    state->field_040 = 0x05dc;
    state->field_054 = 0x20;
    xdb_alien_camera_depth_step = 0;
    xdb_amer_slot2_finish_update(state, context);
    if (state->field_056 != 0
            || state->callback != xdb_amer_slot2_selection_wait
            || state->field_054 != 0x10) {
        return "FAIL amer slot2 finish transition";
    }
    return NULL;
}

int main(void)
{
    const char *error;

    error = check_reset();
    if (error != NULL) {
        return write_result(error);
    }
    error = check_countdown_common_tail();
    if (error != NULL) {
        return write_result(error);
    }
    error = check_selection_transitions();
    if (error != NULL) {
        return write_result(error);
    }
    error = check_finish_transition();
    if (error != NULL) {
        return write_result(error);
    }
    return write_result("PASS amer slot2 callbacks");
}
