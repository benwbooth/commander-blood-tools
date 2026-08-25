#include <stdio.h>
#include <string.h>

#include "xdb_alien.h"

#define RESULT_FILE "RESULT.TXT"

typedef union extended_state {
    xdb_u16 alignment;
    xdb_u8 bytes[0x0500];
} extended_state;

typedef union extended_context {
    xdb_u16 alignment;
    xdb_u8 bytes[0x0060];
} extended_context;

xdb_u16 XDB_NEAR xdb_test_slot2_active(void);
void XDB_NEAR xdb_test_set_slot2_active(xdb_u16 active);
xdb_i16 XDB_NEAR xdb_test_scrut_slot2_shared_word(void);
void XDB_NEAR xdb_test_set_scrut_slot2_shared_word(xdb_i16 value);
void XDB_NEAR xdb_test_set_slot1_selection_state(xdb_u16 state);

static int write_result(const char *status)
{
    FILE *result = fopen(RESULT_FILE, "w");

    if (result == NULL) return 90;
    fputs(status, result);
    fputc('\n', result);
    fclose(result);
    return status[0] == 'P' ? 0 : 1;
}

static const char *check_latch_and_motion(void)
{
    extended_state state_space;
    extended_context context_space;
    xdb_alien_biased_state *state;
    xdb_alien_method_context *context;
    xdb_alien_biased_state *follower;

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    context = (xdb_alien_method_context *)context_space.bytes;
    context->continuation.scrut_slot2.duration = 1;
    state->ring_offset = 0x10;
    xdb_test_set_slot1_selection_state(0);
    xdb_alien_control_latch = (xdb_u16)(size_t)context;
    xdb_scrut_slot2_update(state, context);
    if (context->continuation.scrut_slot2.duration != 0
            || state->field_052 != 0
            || state->callback == xdb_scrut_slot2_fade_update) {
        return "FAIL scrut latch return";
    }

    context->continuation.scrut_slot2.duration = 1;
    xdb_alien_control_latch = 0;
    xdb_scrut_slot2_update(state, context);
    follower = state + 1;
    if (state->field_052 != 0x10
            || state->field_050 != 1
            || follower->field_050 != (xdb_u16)-8
            || follower->field_052 != -4
            || (state + 5)->field_052 != -4) {
        return "FAIL scrut countdown motion";
    }
    return NULL;
}

static const char *check_fade(void)
{
    extended_state state_space;
    extended_context context_space;
    xdb_alien_biased_state *state;
    xdb_alien_method_context *context;

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    context = (xdb_alien_method_context *)context_space.bytes;
    context->continuation.scrut_slot2.duration = 4;
    state->ring_offset = 8;
    xdb_scrut_slot2_begin_fade(state, context);
    if (state->callback != xdb_scrut_slot2_fade_update
            || context->continuation.scrut_slot2.duration != 0
            || state->field_052 != 8) {
        return "FAIL scrut fade";
    }
    return NULL;
}

static const char *check_selection_damp(void)
{
    extended_state state_space;
    extended_context context_space;
    xdb_alien_biased_state *state;
    xdb_alien_method_context *context;

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    context = (xdb_alien_method_context *)context_space.bytes;
    state->field_040 = 0x02bc;
    state->field_038 = -0x01f4;
    state->field_052 = 7;
    state->field_054 = 2;
    *(xdb_i16 *)(state_space.bytes + 0x36) = 0x22;
    xdb_test_set_slot1_selection_state(1);
    xdb_scrut_slot2_selection_init(state, context);
    if (xdb_test_scrut_slot2_shared_word() != -1
            || xdb_test_slot2_active() != 0
            || state->callback != xdb_scrut_slot2_selection_damp
            || state->field_056 != 0x00c8
            || state->ring_offset != 7
            || state->field_058 != 0
            || state->field_054 != 1) {
        return "FAIL scrut selection damp";
    }
    return NULL;
}

static const char *check_helper_and_finish(void)
{
    extended_state state_space;
    extended_context context_space;
    xdb_alien_biased_state *state;
    xdb_alien_method_context *context;

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    context = (xdb_alien_method_context *)context_space.bytes;
    if (xdb_scrut_slot2_steering_helper(state, 0x14u) != 0) {
        return "FAIL scrut helper clear";
    }

    state->field_038 = 0x20;
    state->field_032 = 0x00008000L;
    state->ring_offset = 1;
    if (xdb_scrut_slot2_steering_helper(state, 0x14u) == 0
            || state->field_052 != -1
            || state->ring_offset != (xdb_u16)-1) {
        return "FAIL scrut helper carry";
    }

    memset(&state_space, 0, sizeof(state_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    state->field_040 = 0x012c;
    state->field_01a = -0x0da8L;
    state->field_04e = 0x40;
    xdb_test_set_slot1_selection_state(1);
    xdb_scrut_slot2_selection_approach(state, context);
    if (state->callback == xdb_scrut_slot2_finish_update
            || state->field_04e != 0x20) {
        return "FAIL scrut helper turn transition";
    }

    memset(&state_space, 0, sizeof(state_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    state->field_040 = 0x012c;
    state->field_04e = 0x40;
    xdb_scrut_slot2_selection_approach(state, context);
    if (state->callback != xdb_scrut_slot2_finish_update
            || xdb_test_scrut_slot2_shared_word() != 0x03e8
            || state->field_04e != 0x00) {
        return "FAIL scrut helper finish transition";
    }
    return NULL;
}

static const char *check_selection_reset(void)
{
    extended_state state_space;
    extended_context context_space;
    xdb_alien_biased_state *state;
    xdb_alien_method_context *context;

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    context = (xdb_alien_method_context *)context_space.bytes;
    *(xdb_i16 *)(state_space.bytes + 0x0346) = 11;
    *(xdb_i16 *)(state_space.bytes + 0x034a) = 22;
    *(xdb_i16 *)(state_space.bytes + 0x03a4) = 33;
    *(xdb_i16 *)(state_space.bytes + 0x03a8) = 44;
    state->field_058 = 9;
    state->ring_offset = 10;
    xdb_test_set_slot1_selection_state(0);
    xdb_test_set_slot2_active(1);
    xdb_scrut_slot2_selection_reset_restart(state, context);
    if (state->callback != xdb_scrut_slot2_update
            || xdb_test_slot2_active() != 0
            || *(xdb_i16 *)(state_space.bytes + 0x0332) != 11
            || *(xdb_i16 *)(state_space.bytes + 0x033a) != 22
            || *(xdb_i16 *)(state_space.bytes + 0x0390) != 33
            || *(xdb_i16 *)(state_space.bytes + 0x0398) != 44) {
        return "FAIL scrut selection reset";
    }
    return NULL;
}

static const char *check_camera_reset(void)
{
    extended_state state_space;
    extended_context context_space;
    xdb_alien_biased_state *state;
    xdb_alien_method_context *context;

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    context = (xdb_alien_method_context *)context_space.bytes;
    state->field_040 = (xdb_u16)-501;
    state->position_x = 0x11110000L;
    state->position_y = 0x22220000L;
    state->position_z = 0x33330000L;
    xdb_alien_angle_table[0].cosine = 1;
    xdb_alien_camera_matrix[0] = 0x00010000L;
    xdb_alien_camera_matrix[1] = 0x00020000L;
    xdb_alien_camera_matrix[3] = 0x00020000L;
    xdb_alien_camera_matrix[4] = 0x00030000L;
    xdb_alien_camera_matrix[6] = 0x00040000L;
    xdb_alien_camera_matrix[7] = 0x00050000L;
    xdb_alien_view_x = 1;
    xdb_alien_view_y = 2;
    xdb_alien_view_z = 3;
    xdb_alien_camera_pitch = 0x12;
    xdb_alien_camera_pan = 0x34;
    xdb_alien_camera_depth_step = 0x40;
    xdb_scrut_slot2_reset_or_camera(state, context);
    if (state->position_x != 0x11110002L
            || state->position_y != 0x22220003L
            || state->position_z != 0x33330006L
            || state->field_04e != 0x12
            || state->field_050 != 0x34
            || state->field_054 != 0x016c
            || state->field_058 != 0x016c
            || context->continuation.scrut_slot2.duration != 8) {
        return "FAIL scrut camera reset";
    }
    return NULL;
}

static const char *check_unreferenced_steering(void)
{
    extended_state state_space;
    extended_context context_space;
    xdb_alien_biased_state *state;
    xdb_alien_method_context *context;

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    context = (xdb_alien_method_context *)context_space.bytes;
    context->state = (xdb_alien_state *)state_space.bytes;
    state = (xdb_alien_biased_state *)(state_space.bytes
            + XDB_ALIEN_CURSOR_BIAS);
    state->field_038 = 1;
    state->field_032 = 1;
    state->field_058 = 0x10;
    state->field_050 = 100;
    xdb_scrut_unreferenced_steering_update(context);
    if (state->field_054 != 0x0a
            || state->field_058 != (xdb_u16)-8
            || state->field_050 != 92) {
        return "FAIL scrut unreferenced steering";
    }
    return NULL;
}

int main(void)
{
    const char *error;

    error = check_latch_and_motion();
    if (error != NULL) return write_result(error);
    error = check_fade();
    if (error != NULL) return write_result(error);
    error = check_selection_damp();
    if (error != NULL) return write_result(error);
    error = check_helper_and_finish();
    if (error != NULL) return write_result(error);
    error = check_selection_reset();
    if (error != NULL) return write_result(error);
    error = check_camera_reset();
    if (error != NULL) return write_result(error);
    error = check_unreferenced_steering();
    if (error != NULL) return write_result(error);
    return write_result("PASS scrut slot2 callbacks");
}
