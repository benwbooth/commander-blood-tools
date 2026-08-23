#include <stdio.h>
#include <string.h>

#include "xdb_alien.h"

#define RESULT_FILE "RESULT.TXT"

static int camera_calls;

xdb_u16 XDB_NEAR xdb_test_slot1_selection_state(void);
void XDB_NEAR xdb_test_set_slot1_selection_state(xdb_u16 state);
xdb_i16 XDB_NEAR xdb_test_amer_slot1_current_sample(void);
void XDB_NEAR xdb_test_set_amer_slot1_current_sample(xdb_i16 sample);
void XDB_NEAR xdb_test_set_amer_slot1_selected_state(
        xdb_alien_state_cursor state);
void XDB_NEAR xdb_test_set_slot2_active(xdb_u16 active);

void XDB_NEAR xdb_amer_slot1_camera_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    (void)state;
    (void)context;
    ++camera_calls;
}

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

static const char *check_state_head(void)
{
    xdb_alien_biased_state state;
    xdb_alien_method_context context;

    memset(&state, 0, sizeof(state));
    memset(&context, 0, sizeof(context));
    state.position_x = 0x12345678L;
    state.position_y = (xdb_i32)0x87654321UL;
    state.position_z = (xdb_i32)0xabcdef01UL;
    xdb_test_set_slot1_selection_state(0);
    xdb_alien_palette_pulse_1.value = 0x0100;
    xdb_alien_palette_pulse_2.value = 0x0200;
    xdb_alien_callback_countdown = 0;
    xdb_alien_control_latch = 0;
    xdb_amer_slot1_state_update(&state, &context);
    if (state.position_x != 0x12340000L) {
        return "FAIL slot1 position x";
    }
    if (state.position_y != (xdb_i32)0x87650000UL) {
        return "FAIL slot1 position y";
    }
    if (state.position_z != (xdb_i32)0xabcd0020UL) {
        return "FAIL slot1 position z";
    }
    if (state.owner_offset != 0x25a8u || state.field_054 != 0) {
        return "FAIL slot1 state fields";
    }
    if (state.callback != xdb_amer_slot1_wave_update) {
        return "FAIL slot1 callback";
    }
    if (xdb_test_slot1_selection_state() != 1) {
        if (xdb_test_slot1_selection_state() == 0) {
            return "FAIL slot1 selection zero";
        }
        if (xdb_test_slot1_selection_state() == 2) {
            return "FAIL slot1 selection two";
        }
        return "FAIL slot1 selection";
    }
    if (xdb_alien_control_latch != 1) {
        return "FAIL slot1 control latch";
    }
    if (xdb_alien_callback_countdown != 5) {
        return "FAIL slot1 countdown";
    }
    if (xdb_alien_palette_pulse_1.value != 0x011e
            || xdb_alien_palette_pulse_2.value != 0x0223) {
        return "FAIL slot1 palette";
    }

    memset(&state, 0, sizeof(state));
    xdb_test_set_slot1_selection_state(2);
    camera_calls = 0;
    xdb_amer_slot1_state_update(&state, &context);
    if (camera_calls != 1 || state.callback != NULL) {
        return "FAIL slot1 camera dispatch";
    }
    return NULL;
}

static const char *check_fixed_motion(
        xdb_i16 view_x,
        xdb_i16 view_z,
        xdb_i16 expected_step,
        xdb_i16 expected_position_step)
{
    xdb_alien_biased_state state;
    xdb_alien_method_context context;

    memset(&state, 0, sizeof(state));
    memset(&context, 0, sizeof(context));
    state.field_010 = 0;
    state.field_052 = 0x2222;
    state.field_05c = 0x1234;
    xdb_alien_view_x = view_x;
    xdb_alien_view_z = view_z;
    xdb_amer_slot1_motion_continuation(&state, &context);
    if (state.field_010 != 0x10
            || state.field_056 != expected_step
            || (xdb_i16)state.ring_offset != expected_step
            || state.field_052 != expected_step
            || (xdb_i16)state.field_050 != expected_position_step
            || state.field_058 != 0x80
            || state.field_054 != 0x0c
            || state.field_05c != 0x1234) {
        return "FAIL slot1 staged distance";
    }
    return NULL;
}

int main(void)
{
    const char *error;
    xdb_alien_biased_state state;
    xdb_alien_biased_state selected;
    xdb_alien_method_context context;
    char status[80];

    error = check_state_head();
    if (error != NULL) {
        return write_result(error);
    }
    error = check_fixed_motion(0, 0x03e9, 16, 0);
    if (error != NULL) {
        return write_result(error);
    }
    error = check_fixed_motion((xdb_i16)0xfc17u, 0, 8, 0);
    if (error != NULL) {
        return write_result(error);
    }
    error = check_fixed_motion(0x03e8, 0, -8, -1);
    if (error != NULL) {
        return write_result(error);
    }

    memset(&state, 0, sizeof(state));
    memset(&selected, 0, sizeof(selected));
    memset(&context, 0, sizeof(context));
    xdb_test_set_slot2_active(0);
    xdb_test_set_slot1_selection_state(2);
    xdb_test_set_amer_slot1_selected_state((xdb_alien_state_cursor)&selected);
    xdb_test_set_amer_slot1_current_sample(0x0077);
    xdb_alien_callback_countdown = 0;
    xdb_alien_palette_pulse_1.value = 0x0100;
    xdb_alien_palette_pulse_2.value = 0x0200;
    xdb_amer_slot1_wave_update(&state, &context);
    if (xdb_test_amer_slot1_current_sample() != 0x007f) {
        sprintf(status, "FAIL slot1 wave sample %d", xdb_test_amer_slot1_current_sample());
        return write_result(status);
    }
    if (state.owner_offset != (xdb_u16)(size_t)&selected) {
        return write_result("FAIL slot1 wave owner");
    }
    if (state.callback != xdb_amer_slot1_finish_update) {
        return write_result("FAIL slot1 wave callback");
    }
    if (xdb_test_slot1_selection_state() != 0) {
        return write_result("FAIL slot1 wave selection");
    }
    if (xdb_alien_callback_countdown != 4) {
        return write_result("FAIL slot1 wave countdown");
    }
    if (xdb_alien_palette_pulse_1.value != 0x00e2
            || xdb_alien_palette_pulse_2.value != 0x01dd) {
        return write_result("FAIL slot1 wave palette");
    }

    memset(&state, 0, sizeof(state));
    state.field_054 = 1;
    state.callback = xdb_amer_slot1_return_update;
    xdb_amer_slot1_return_update(&state, &context);
    if (state.field_054 != 0 || state.callback != xdb_amer_slot1_state_update) {
        return write_result("FAIL slot1 return transition");
    }
    return write_result("PASS amer slot1 callbacks");
}
