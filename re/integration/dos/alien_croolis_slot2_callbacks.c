#include <stdio.h>
#include <string.h>

#include "xdb_alien.h"

#define RESULT_FILE "RESULT.TXT"

typedef union extended_state {
    xdb_u16 alignment;
    xdb_u8 bytes[0x0400];
} extended_state;

typedef union extended_context {
    xdb_u16 alignment;
    xdb_u8 bytes[0x0060];
} extended_context;

xdb_u16 XDB_NEAR xdb_test_slot2_active(void);
void XDB_NEAR xdb_test_set_slot2_active(xdb_u16 active);
xdb_alien_state_cursor XDB_NEAR
        xdb_test_croolis_slot2_selected_state(void);
void XDB_NEAR xdb_test_set_croolis_slot2_selected_state(
        xdb_alien_state_cursor state);
void XDB_NEAR xdb_test_set_slot1_selection_state(xdb_u16 state);

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

static const char *check_countdown_motion(void)
{
    extended_state state_space;
    extended_context context_space;
    xdb_alien_biased_state *state;
    xdb_alien_method_context *context;

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    context = (xdb_alien_method_context *)context_space.bytes;
    context->continuation.croolis_slot2.duration = 1;
    context->continuation.croolis_slot2.field_03a = 8;
    state->field_058 = 0;
    state->field_054 = 0;
    xdb_alien_control_latch = 0;
    xdb_test_set_slot1_selection_state(0);
    xdb_croolis_slot2_update(state, context);
    if (context->continuation.croolis_slot2.duration != 0
            || state->field_052 != 8
            || state->field_050 != 1
            || *(xdb_i16 *)(state_space.bytes + 0x010a) != 4
            || *(xdb_i16 *)(state_space.bytes + 0x00ae) != -4
            || *(xdb_i16 *)(state_space.bytes + 0x0168) != -4) {
        return "FAIL croolis slot2 countdown motion";
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
    state->position_y = 0x12340064L;
    xdb_croolis_slot2_begin_fade(state, context);
    if ((xdb_u16)state->position_y != 0x0046u
            || (xdb_u16)(state->position_y >> 16) != 0x1234u
            || context->continuation.croolis_slot2.duration != 0x00ae
            || state->callback != xdb_croolis_slot2_fade_update
            || *(xdb_i16 *)(state_space.bytes + 0x01be) != 0x00b2) {
        return "FAIL croolis slot2 fade begin";
    }

    context->continuation.croolis_slot2.duration = 0x0096;
    xdb_croolis_slot2_fade_update(state, context);
    if (context->continuation.croolis_slot2.duration != 0x0092) {
        return "FAIL croolis slot2 fade decrement";
    }
    return NULL;
}

static const char *check_selection(void)
{
    extended_state state_space;
    extended_context context_space;
    xdb_alien_biased_state *state;
    xdb_alien_method_context *context;

    memset(&state_space, 0, sizeof(state_space));
    memset(&context_space, 0, sizeof(context_space));
    state = (xdb_alien_biased_state *)state_space.bytes;
    context = (xdb_alien_method_context *)context_space.bytes;
    state->field_040 = 0x03e8;
    state->field_032 = (xdb_i16)0x8000u;
    state->field_01a = 1;
    *(xdb_i16 *)(state_space.bytes + 0x022c) = 10;
    *(xdb_i16 *)(state_space.bytes + 0x028a) = 20;
    *(xdb_i16 *)(state_space.bytes + 0x02e8) = 30;
    xdb_alien_callback_countdown = 0;
    xdb_alien_control_latch = 0;
    xdb_test_set_slot1_selection_state(1);
    xdb_croolis_slot2_selection_init(state, context);
    if (state->callback != xdb_croolis_slot2_selection_update) {
        return "FAIL croolis selection callback";
    }
    if (xdb_test_croolis_slot2_selected_state() != state) {
        return "FAIL croolis selection pointer";
    }
    if (xdb_test_slot2_active() != 0) {
        return "FAIL croolis selection active";
    }
    if (state->field_052 != 0x30 || state->field_050 != 3) {
        return "FAIL croolis selection steering";
    }
    if (state->field_054 != 0x32 || state->field_056 != 0x70
            || xdb_alien_callback_countdown != 1) {
        return "FAIL croolis selection phase";
    }
    if (*(xdb_i16 *)(state_space.bytes + 0x0220) != 0x7a
            || *(xdb_i16 *)(state_space.bytes + 0x027e) != 0x84
            || *(xdb_i16 *)(state_space.bytes + 0x02dc) != 0x8e) {
        return "FAIL croolis selection visuals";
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
    state->field_040 = 0x01f4;
    *(xdb_i16 *)(state_space.bytes + 0x022c) = 11;
    *(xdb_i16 *)(state_space.bytes + 0x028a) = 22;
    *(xdb_i16 *)(state_space.bytes + 0x02e8) = 33;
    xdb_test_set_slot1_selection_state(0);
    xdb_test_set_slot2_active(1);
    xdb_test_set_croolis_slot2_selected_state(state);
    xdb_croolis_slot2_selection_update(state, context);
    if (state->callback != xdb_croolis_slot2_update
            || xdb_test_croolis_slot2_selected_state() != NULL
            || xdb_test_slot2_active() != 0
            || context->continuation.croolis_slot2.field_03a != -0x10
            || *(xdb_i16 *)(state_space.bytes + 0x0220) != 11
            || *(xdb_i16 *)(state_space.bytes + 0x027e) != 22
            || *(xdb_i16 *)(state_space.bytes + 0x02dc) != 33) {
        return "FAIL croolis slot2 selection reset";
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
    xdb_croolis_slot2_reset_or_camera(state, context, 0x03e8L);
    if (state->position_x != 0x11110002L
            || state->position_y != 0x22220003L
            || state->position_z != 0x33330006L
            || state->field_04e != 0x12
            || state->field_050 != 0x34
            || state->field_052 != 0
            || state->field_054 != 0x016c
            || state->field_058 != 0x016c
            || context->continuation.croolis_slot2.duration != 8) {
        return "FAIL croolis slot2 camera reset";
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
    xdb_croolis_unreferenced_steering_update(context);
    if (state->field_054 != 0x0a
            || state->field_058 != (xdb_u16)-8
            || state->field_050 != 92) {
        return "FAIL croolis unreferenced steering";
    }
    return NULL;
}

int main(void)
{
    const char *error;

    error = check_countdown_motion();
    if (error != NULL) return write_result(error);
    error = check_fade();
    if (error != NULL) return write_result(error);
    error = check_selection();
    if (error != NULL) return write_result(error);
    error = check_selection_reset();
    if (error != NULL) return write_result(error);
    error = check_camera_reset();
    if (error != NULL) return write_result(error);
    error = check_unreferenced_steering();
    if (error != NULL) return write_result(error);
    return write_result("PASS croolis slot2 callbacks");
}
