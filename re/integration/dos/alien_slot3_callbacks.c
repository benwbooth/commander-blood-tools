#include <stdio.h>
#include <string.h>

#include "xdb_alien.h"

#define RESULT_FILE "RESULT.TXT"

#if defined(TEST_AMER)
#define MODULE_NAME "amer"
#define SLOT3_TIMER xdb_amer_slot3_timer
#define RESUME_COUNTDOWN xdb_amer_slot3_resume_countdown
#define RESUME_STATE xdb_amer_slot3_resume_state
#define SLOT3_RING xdb_amer_slot3_ring
#define INITIAL_UPDATE xdb_amer_slot3_initial_update
#define UPDATE xdb_amer_slot3_update
#define RESTART_UPDATE xdb_amer_slot3_restart_initial_update
#define RESUME_CALLBACK xdb_amer_slot3_resume_callback
#define CAPTURE_STATE xdb_amer_slot3_capture_resume_state
#define RING_ZERO_CALLBACK xdb_amer_slot3_ring_zero_callback
#define RESUME_MAIN xdb_amer_resume_1c34
#define APPLY_OBJECT_DELTA xdb_amer_resume_apply_object_delta
#define RESUME_STAGE_PAIR xdb_amer_resume_stage_pair
#define RESUME_STAGE_TIMEOUT xdb_amer_resume_stage_timeout
#define RESUME_STAGE_FINAL xdb_amer_resume_stage_final
#define RESUME_PAIR_OUTSIDE xdb_amer_resume_pair_outside
#define SLOT1_WAVE_UPDATE xdb_amer_slot1_wave_update
#define SLOT1_STATE_UPDATE xdb_amer_slot1_state_update
#define SLOT11_QUEUE_CURSOR xdb_amer_slot11_queue_cursor
#define SLOT11_CURRENT_STATE xdb_amer_slot11_current_state
#define SLOT11_STATE_QUEUE xdb_amer_slot11_state_queue
#define SLOT1_SELECTION_STATE xdb_amer_slot1_selection_state
#define POSITION_X 0L
#define POSITION_Y 0x06a4L
#define POSITION_Z 0L
#define EMPTY_POSITION_AC ((xdb_i16)-0x20)
#define EMPTY_POSITION_AE ((xdb_i16)15)
#define OBJECT_PLUS_0 0x0002u
#define OBJECT_MINUS_0 0x0426u
#define OBJECT_PLUS_1 0x02beu
#define OBJECT_MINUS_1 0x01f6u
#define OBJECT_PAIR_COUNT 2
#elif defined(TEST_CROOLIS)
#define MODULE_NAME "croolis"
#define SLOT3_TIMER xdb_croolis_slot3_timer
#define RESUME_COUNTDOWN xdb_croolis_slot3_resume_countdown
#define RESUME_STATE xdb_croolis_slot3_resume_state
#define SLOT3_RING xdb_croolis_slot3_ring
#define INITIAL_UPDATE xdb_croolis_slot3_initial_update
#define UPDATE xdb_croolis_slot3_update
#define RESTART_UPDATE xdb_croolis_slot3_restart_initial_update
#define RESUME_CALLBACK xdb_croolis_slot3_resume_callback
#define CAPTURE_STATE xdb_croolis_slot3_capture_resume_state
#define RING_ZERO_CALLBACK xdb_croolis_slot3_ring_zero_callback
#define RESUME_MAIN xdb_croolis_resume_1b85
#define APPLY_OBJECT_DELTA xdb_croolis_resume_apply_object_delta
#define RESUME_STAGE_PAIR xdb_croolis_resume_stage_pair
#define RESUME_STAGE_TIMEOUT xdb_croolis_resume_stage_timeout
#define RESUME_STAGE_FINAL xdb_croolis_resume_stage_final
#define RESUME_PAIR_OUTSIDE xdb_croolis_resume_pair_outside
#define SLOT1_WAVE_UPDATE xdb_croolis_slot1_wave_update
#define SLOT1_STATE_UPDATE xdb_croolis_slot1_state_update
#define SLOT11_QUEUE_CURSOR xdb_croolis_slot11_queue_cursor
#define SLOT11_CURRENT_STATE xdb_croolis_slot11_current_state
#define SLOT11_STATE_QUEUE xdb_croolis_slot11_state_queue
#define SLOT1_SELECTION_STATE xdb_croolis_slot1_selection_state
#define POSITION_X 0L
#define POSITION_Y 0x06a4L
#define POSITION_Z 0L
#define EMPTY_POSITION_AC ((xdb_i16)0x20)
#define EMPTY_POSITION_AE ((xdb_i16)7)
#define OBJECT_PLUS_0 0x0002u
#define OBJECT_MINUS_0 0x01f4u
#define OBJECT_PAIR_COUNT 1
#elif defined(TEST_SCRUT)
#define MODULE_NAME "scrut"
#define SLOT3_TIMER xdb_scrut_slot3_timer
#define RESUME_COUNTDOWN xdb_scrut_slot3_resume_countdown
#define RESUME_STATE xdb_scrut_slot3_resume_state
#define SLOT3_RING xdb_scrut_slot3_ring
#define INITIAL_UPDATE xdb_scrut_slot3_initial_update
#define UPDATE xdb_scrut_slot3_update
#define RESTART_UPDATE xdb_scrut_slot3_restart_initial_update
#define RESUME_CALLBACK xdb_scrut_slot3_resume_callback
#define CAPTURE_STATE xdb_scrut_slot3_capture_resume_state
#define RING_ZERO_CALLBACK xdb_scrut_slot3_ring_zero_callback
#define RESUME_MAIN xdb_scrut_resume_1c45
#define APPLY_OBJECT_DELTA xdb_scrut_resume_apply_object_delta
#define RESUME_STAGE_PAIR xdb_scrut_resume_stage_pair
#define RESUME_STAGE_TIMEOUT xdb_scrut_resume_stage_timeout
#define RESUME_STAGE_FINAL xdb_scrut_resume_stage_final
#define RESUME_PAIR_OUTSIDE xdb_scrut_resume_pair_outside
#define SLOT1_WAVE_UPDATE xdb_scrut_slot1_wave_update
#define SLOT1_STATE_UPDATE xdb_scrut_slot1_state_update
#define SLOT11_QUEUE_CURSOR xdb_scrut_slot11_queue_cursor
#define SLOT11_CURRENT_STATE xdb_scrut_slot11_current_state
#define SLOT11_STATE_QUEUE xdb_scrut_slot11_state_queue
#define SLOT1_SELECTION_STATE xdb_scrut_slot1_selection_state
#define POSITION_X 0x06a4L
#define POSITION_Y 0L
#define POSITION_Z 0L
#define EMPTY_POSITION_AC ((xdb_i16)0x20)
#define EMPTY_POSITION_AE ((xdb_i16)7)
#define OBJECT_PLUS_0 0x0002u
#define OBJECT_MINUS_0 0x035eu
#define OBJECT_PLUS_1 0x034au
#define OBJECT_MINUS_1 0x01f6u
#define OBJECT_PAIR_COUNT 2
#else
#error Select one alien module
#endif

xdb_u16 XDB_NEAR xdb_test_slot3_resume_countdown(void);
xdb_alien_cursor XDB_NEAR xdb_test_slot3_resume_state(void);
xdb_i16 XDB_NEAR xdb_test_slot3_ring_field_006(xdb_u16 ring_cursor);
xdb_u16 XDB_NEAR xdb_test_slot11_queue_cursor(void);
xdb_u16 XDB_NEAR xdb_test_slot11_queue_read_cursor(void);
xdb_u16 XDB_NEAR xdb_test_slot11_current_state(void);
xdb_u16 XDB_NEAR xdb_test_slot11_state_at(xdb_u16 queue_cursor);
void XDB_NEAR xdb_test_set_slot3_resume_countdown(xdb_u16 countdown);
void XDB_NEAR xdb_test_set_slot11_cursor(xdb_alien_cursor state);
void XDB_NEAR xdb_test_set_slot11_queue_read_cursor(xdb_u16 queue_cursor);
void XDB_NEAR xdb_test_set_slot11_current_state(xdb_u16 state);
void XDB_NEAR xdb_test_set_slot11_state_at(
        xdb_u16 queue_cursor,
        xdb_u16 state);

typedef union xdb_test_object_space {
    xdb_u16 alignment;
    xdb_u8 bytes[0x0500];
} xdb_test_object_space;

typedef union xdb_test_state_space {
    xdb_u16 alignment;
    xdb_u8 bytes[XDB_ALIEN_CURSOR_BIAS + sizeof(xdb_alien_biased_state)];
} xdb_test_state_space;

static xdb_test_object_space object_space;
static xdb_test_state_space pair_state_space;
static xdb_alien_biased_state pair_other;
static xdb_alien_biased_state slot11_target;
static xdb_u16 slot1_state_update_calls;

void XDB_NEAR SLOT1_WAVE_UPDATE(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    (void)state;
    (void)context;
}

void XDB_NEAR SLOT1_STATE_UPDATE(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    (void)state;
    (void)context;
    ++slot1_state_update_calls;
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

static int check_positions(const xdb_alien_biased_state *state)
{
    return state->position_x == POSITION_X
            && state->position_y == POSITION_Y
            && state->position_z == POSITION_Z
            && state->field_04e == 0
            && state->field_050 == 0
            && state->field_052 == 0
            && state->field_054 == 0;
}

static xdb_i16 *object_word(xdb_u16 offset)
{
    return (xdb_i16 *)(object_space.bytes + offset);
}

static void prepare_object_context(xdb_alien_method_context *context)
{
    memset(&object_space, 0, sizeof(object_space));
    xdb_alien_object_segment = FP_SEG(object_space.bytes);
    context->object_offset = FP_OFF(object_space.bytes);
}

static void initialize_object_words(xdb_i16 value)
{
    *object_word(OBJECT_PLUS_0) = value;
    *object_word(OBJECT_MINUS_0) = value;
#if OBJECT_PAIR_COUNT == 2
    *object_word(OBJECT_PLUS_1) = value;
    *object_word(OBJECT_MINUS_1) = value;
#endif
}

static int check_object_words(xdb_i16 added, xdb_i16 subtracted)
{
    if (*object_word(OBJECT_PLUS_0) != added
            || *object_word(OBJECT_MINUS_0) != subtracted) {
        return 0;
    }
#if OBJECT_PAIR_COUNT == 2
    if (*object_word(OBJECT_PLUS_1) != added
            || *object_word(OBJECT_MINUS_1) != subtracted) {
        return 0;
    }
#endif
    return 1;
}

static const char *check_resume_routines(void)
{
    xdb_alien_method_context context;
    xdb_alien_state queue_state;
    xdb_alien_biased_state *current;

    memset(&context, 0, sizeof(context));
    prepare_object_context(&context);
    initialize_object_words(1000);
    context.continuation.resume_state.phase = 0x0102;
    context.continuation.resume_state.paired_state = 0x7777;
    context.continuation.resume_state.resumed_state = 0x6666;
    APPLY_OBJECT_DELTA(&context);
    if (context.continuation.resume_state.phase != 0x0302
            || context.continuation.resume_state.paired_state != 0x7777
            || context.continuation.resume_state.resumed_state != 0x6666
            || !check_object_words(1002, 998)) {
        return "FAIL object delta";
    }

    memset(&context, 0, sizeof(context));
    memset(&queue_state, 0, sizeof(queue_state));
    context.state = &queue_state;
    queue_state.field_0ae = 7;
    xdb_test_set_slot11_queue_read_cursor(0);
    xdb_test_set_slot11_state_at(0, 0);
    RESUME_MAIN(&context);
    if (xdb_test_slot11_queue_read_cursor() != 2
            || queue_state.field_0ac != EMPTY_POSITION_AC
            || queue_state.field_0ae != EMPTY_POSITION_AE) {
        return "FAIL resume empty queue";
    }

    memset(&pair_other, 0, sizeof(pair_other));
    context.continuation.resume_state.phase = 0x0102;
    context.continuation.resume_state.paired_state = 0;
    context.continuation.resume_state.resumed_state = 0x6666;
    xdb_test_set_slot11_queue_read_cursor(4);
    xdb_test_set_slot11_current_state(0x7777);
    xdb_test_set_slot11_state_at(4, (xdb_u16)(size_t)&pair_other);
    RESUME_MAIN(&context);
    if (xdb_test_slot11_queue_read_cursor() != 4
            || xdb_test_slot11_current_state() != 0
            || xdb_test_slot11_state_at(4) != 0
            || context.control.resume != RESUME_STAGE_PAIR
            || context.continuation.resume_state.phase != 0x0102
            || context.continuation.resume_state.paired_state
                    != (xdb_u16)(size_t)&pair_other
            || context.continuation.resume_state.resumed_state != 0x6666) {
        return "FAIL resume queued state";
    }

    memset(&pair_state_space, 0, sizeof(pair_state_space));
    memset(&pair_other, 0, sizeof(pair_other));
    current = (xdb_alien_biased_state *)(
            pair_state_space.bytes + XDB_ALIEN_CURSOR_BIAS);
    xdb_alien_angle_table[0].cosine = 1;
    xdb_alien_angle_table[0].sine = 0;
    pair_other.position_x = 0x00010010L;
    if (RESUME_PAIR_OUTSIDE(current, &pair_other) != 0) {
        return "FAIL pair low-word bounds";
    }
    pair_other.position_x = 300;
    if (RESUME_PAIR_OUTSIDE(current, &pair_other) != 1
            || current->field_050 != 0x0010) {
        return "FAIL pair steering";
    }

    memset(&context, 0, sizeof(context));
    memset(&pair_state_space, 0, sizeof(pair_state_space));
    memset(&pair_other, 0, sizeof(pair_other));
    current = (xdb_alien_biased_state *)(
            pair_state_space.bytes + XDB_ALIEN_CURSOR_BIAS);
    context.state = (xdb_alien_state *)pair_state_space.bytes;
    context.control.resume = RESUME_STAGE_PAIR;
    context.continuation.resume_state.phase = 0x0102;
    context.continuation.resume_state.paired_state =
            (xdb_u16)(size_t)&pair_other;
    context.continuation.resume_state.resumed_state = 0x6666;
    current->field_054 = 40;
    pair_other.field_054 = 20;
    pair_other.position_x = 300;
    pair_other.callback = INITIAL_UPDATE;
    prepare_object_context(&context);
    initialize_object_words(1000);
    xdb_test_set_slot3_resume_countdown(9);
    RESUME_STAGE_PAIR(&context);
    if (context.control.resume != RESUME_STAGE_PAIR
            || current->field_054 != 35
            || current->field_050 != 0x0010
            || pair_other.callback != INITIAL_UPDATE
            || xdb_test_slot3_resume_countdown() != 9
            || context.continuation.resume_state.resumed_state != 0x6666
            || context.continuation.resume_state.phase != 0x0302
            || !check_object_words(1002, 998)) {
        return "FAIL pair approach stage";
    }

    memset(&context, 0, sizeof(context));
    memset(&pair_state_space, 0, sizeof(pair_state_space));
    memset(&pair_other, 0, sizeof(pair_other));
    current = (xdb_alien_biased_state *)(
            pair_state_space.bytes + XDB_ALIEN_CURSOR_BIAS);
    context.state = (xdb_alien_state *)pair_state_space.bytes;
    context.control.resume = RESUME_STAGE_PAIR;
    context.continuation.resume_state.phase = 0x0102;
    context.continuation.resume_state.paired_state =
            (xdb_u16)(size_t)&pair_other;
    context.continuation.resume_state.resumed_state = 0x6666;
    current->field_054 = 40;
    pair_other.field_054 = 20;
    pair_other.callback = INITIAL_UPDATE;
    prepare_object_context(&context);
    initialize_object_words(1000);
    xdb_test_set_slot3_resume_countdown(9);
    RESUME_STAGE_PAIR(&context);
    if (context.control.resume != RESUME_STAGE_TIMEOUT
            || current->field_054 != 0
            || pair_other.callback != RESUME_CALLBACK
            || xdb_test_slot3_resume_countdown() != 0x18
            || context.continuation.resume_state.paired_state
                    != (xdb_u16)(size_t)&pair_other
            || context.continuation.resume_state.resumed_state
                    != (xdb_u16)(size_t)&pair_other
            || context.continuation.resume_state.phase != 0x0302
            || !check_object_words(1002, 998)) {
        return "FAIL pair matched stage";
    }

    prepare_object_context(&context);
    initialize_object_words(1000);
    context.control.resume = RESUME_STAGE_TIMEOUT;
    context.continuation.resume_state.phase = 0x0102;
    xdb_test_set_slot3_resume_countdown(0);
    RESUME_STAGE_TIMEOUT(&context);
    if (context.control.resume != RESUME_STAGE_FINAL
            || xdb_test_slot3_resume_countdown() != 0xffff
            || context.continuation.resume_state.phase != 0x0302
            || !check_object_words(1002, 998)) {
        return "FAIL timeout stage";
    }

    memset(&context, 0, sizeof(context));
    memset(&pair_state_space, 0, sizeof(pair_state_space));
    memset(&pair_other, 0, sizeof(pair_other));
    memset(&slot11_target, 0, sizeof(slot11_target));
    current = (xdb_alien_biased_state *)(
            pair_state_space.bytes + XDB_ALIEN_CURSOR_BIAS);
    context.state = (xdb_alien_state *)pair_state_space.bytes;
    context.control.resume = RESUME_STAGE_FINAL;
    context.continuation.resume_state.paired_state =
            (xdb_u16)(size_t)&pair_other;
    pair_other.callback = INITIAL_UPDATE;
    slot11_target.callback = RESUME_CALLBACK;
    xdb_test_set_slot11_cursor((xdb_alien_cursor)&slot11_target);
    RESUME_STAGE_FINAL(&context);
    if (context.control.resume != RESUME_MAIN
            || current->field_054 != 0
            || pair_other.callback != RESTART_UPDATE
            || slot11_target.callback != RESUME_CALLBACK) {
        return "FAIL final stage";
    }

    return NULL;
}

int main(void)
{
    xdb_alien_biased_state state;
    xdb_alien_method_context context;
    volatile xdb_alien_ring_entry XDB_CODE_DATA *ring;
    const char *resume_error;

    memset(&state, 0, sizeof(state));
    memset(&context, 0, sizeof(context));
    memset((void *)&SLOT3_RING[0], 0, sizeof(SLOT3_RING[0]) * 2u);
    state.field_056 = 2;
    state.position_x = 1;
    state.position_y = -0x03e8L;
    state.position_z = 1;
    SLOT3_TIMER = 0;
    INITIAL_UPDATE(&state, &context);
    if (state.ring_offset != 8u
            || state.field_056 != 0
            || SLOT3_RING[1].field_000 != -0x0040
            || SLOT3_RING[1].field_002 != 0
            || SLOT3_RING[1].field_004 != 0) {
        return write_result("FAIL initial bounds correction");
    }

    memset(&state, 0, sizeof(state));
    memset(&context, 0, sizeof(context));
    state.ring_offset = 0x0198;
    state.field_052 = 0x3333;
    state.field_054 = 0x4444;
    state.field_056 = 0x5555;
    state.field_05c = 0x6666;
    ring = &SLOT3_RING[state.ring_offset >> 3];
    ring->field_004 = 0x7777;
    ring->field_006 = (xdb_i16)0x8888u;
    xdb_alien_random_state = 0x1234;
    RESTART_UPDATE(&state, &context);
    if (ring->field_004 != 8 || ring->field_006 != 0
            || state.callback != INITIAL_UPDATE
            || state.field_052 != 0 || state.field_054 != 8
            || state.field_056 != 0x1e
            || state.field_05c != (xdb_i16)0x8245u
            || xdb_alien_random_state != 0x8245) {
        return write_result("FAIL restart");
    }

    memset(&state, 0x5a, sizeof(state));
    state.ring_offset = 0x03fc;
    ring = &SLOT3_RING[state.ring_offset >> 3];
    ring->field_000 = 1;
    ring->field_002 = 2;
    ring->field_004 = 3;
    ring->field_006 = 4;
    RESUME_CALLBACK(&state, &context);
    if (!check_positions(&state)
            || state.callback != RING_ZERO_CALLBACK
            || ring->field_000 != 0 || ring->field_002 != 0
            || ring->field_004 != 0 || ring->field_006 != 2) {
        return write_result("FAIL resume");
    }

    memset(&state, 0x3c, sizeof(state));
    state.ring_offset = 0x0198;
    state.callback = INITIAL_UPDATE;
    ring = &SLOT3_RING[state.ring_offset >> 3];
    ring->field_006 = 0x2468;
    RESUME_COUNTDOWN = 0;
    RESUME_STATE = 0;
    CAPTURE_STATE(&state, &context);
    if (!check_positions(&state)) {
        return write_result("FAIL capture positions");
    }
    if (state.callback != INITIAL_UPDATE) {
        return write_result("FAIL capture callback");
    }
    if (xdb_test_slot3_resume_countdown() != 0x12) {
        return write_result("FAIL capture countdown");
    }
    if (xdb_test_slot3_resume_state() != (xdb_alien_cursor)&state) {
        return write_result("FAIL capture state");
    }
    if (ring->field_006 != 0x2468) {
        return write_result("FAIL capture ring");
    }

    state.ring_offset = 0x0198;
    ring = &SLOT3_RING[(state.ring_offset + 8u) >> 3];
    ring->field_000 = 1;
    ring->field_002 = 2;
    ring->field_004 = 3;
    ring->field_006 = 4;
    SLOT3_TIMER = 1;
    RING_ZERO_CALLBACK(&state, &context);
    if (state.ring_offset != 0x0198 || ring->field_006 != 4) {
        return write_result("FAIL timer gate");
    }
    SLOT3_TIMER = 0;
    RING_ZERO_CALLBACK(&state, &context);
    if (state.ring_offset != 0x01a0
            || ring->field_000 != 0 || ring->field_002 != 0
            || ring->field_004 != 0 || ring->field_006 != 0) {
        return write_result("FAIL ring clear");
    }

    state.ring_offset = 0x03f8;
    SLOT3_RING[0].field_006 = 0x1357;
    RING_ZERO_CALLBACK(&state, &context);
    if (state.ring_offset != 0 || xdb_test_slot3_ring_field_006(0) != 0) {
        return write_result("FAIL ring wrap");
    }

    memset(&state, 0x5a, sizeof(state));
    memset(&context, 0, sizeof(context));
    state.ring_offset = 0x0198;
    state.callback = INITIAL_UPDATE;
    ring = &SLOT3_RING[(state.ring_offset + 8u) >> 3];
    ring->field_006 = 2;
    SLOT3_TIMER = 0;
    RESUME_COUNTDOWN = 0;
    RESUME_STATE = 0;
    UPDATE(&state, &context);
    if (state.ring_offset != 0x01a0
            || state.callback != INITIAL_UPDATE
            || !check_positions(&state)
            || xdb_test_slot3_resume_countdown() != 0x12
            || xdb_test_slot3_resume_state() != (xdb_alien_cursor)&state) {
        return write_result("FAIL update capture dispatch");
    }

    memset(&state, 0, sizeof(state));
    state.ring_offset = 0x01a0;
    state.field_05c = 1;
    ring = &SLOT3_RING[(state.ring_offset + 8u) >> 3];
    ring->field_006 = 1;
    SLOT3_TIMER = 0;
    SLOT11_QUEUE_CURSOR = 6;
    SLOT11_CURRENT_STATE = 0;
    SLOT11_STATE_QUEUE[3] = 0;
    xdb_alien_random_state = 0x1234;
    UPDATE(&state, &context);
    if (state.ring_offset != 0x01a8
            || state.callback != INITIAL_UPDATE
            || ring->field_004 != 8 || ring->field_006 != 0
            || state.field_054 != 8 || state.field_056 != 0x1e
            || state.field_05c != (xdb_i16)0x8245u
            || xdb_alien_random_state != 0x8245
            || xdb_test_slot11_queue_cursor() != 6
            || xdb_test_slot11_current_state() != (xdb_u16)(size_t)&state
            || xdb_test_slot11_state_at(6) != (xdb_u16)(size_t)&state) {
        return write_result("FAIL update restart dispatch");
    }

    memset(&state, 0, sizeof(state));
    memset(&context, 0, sizeof(context));
    state.ring_offset = 0x01a8;
    ring = &SLOT3_RING[(state.ring_offset + 8u) >> 3];
    memset(ring, 0, sizeof(*ring));
    SLOT3_TIMER = 0;
    SLOT1_SELECTION_STATE = 0;
    xdb_alien_callback_countdown = 0;
    slot1_state_update_calls = 0;
    UPDATE(&state, &context);
    if (slot1_state_update_calls != 1
            || ring->field_004 != 8 || ring->field_006 != 1
            || xdb_alien_callback_countdown != 2) {
        return write_result("FAIL update slot1 dispatch");
    }

    resume_error = check_resume_routines();
    if (resume_error != NULL) {
        return write_result(resume_error);
    }

    return write_result("PASS " MODULE_NAME " slot3 callbacks");
}
