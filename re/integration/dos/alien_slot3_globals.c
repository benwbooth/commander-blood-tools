#include "xdb_alien.h"

#if defined(TEST_AMER)
#define SLOT3_TIMER xdb_amer_slot3_timer
#define RESUME_COUNTDOWN xdb_amer_slot3_resume_countdown
#define RESUME_STATE xdb_amer_slot3_resume_state
#define SLOT3_RING xdb_amer_slot3_ring
#define SLOT11_CURSOR xdb_amer_slot11_cursor
#define SLOT11_QUEUE_CURSOR xdb_amer_slot11_queue_cursor
#define SLOT11_QUEUE_READ_CURSOR xdb_amer_slot11_queue_read_cursor
#define SLOT11_CURRENT_STATE xdb_amer_slot11_current_state
#define SLOT11_STATE_QUEUE xdb_amer_slot11_state_queue
#define SLOT1_SELECTION_STATE xdb_amer_slot1_selection_state
#define SLOT2_ACTIVE xdb_amer_slot2_active
#elif defined(TEST_CROOLIS)
#define SLOT3_TIMER xdb_croolis_slot3_timer
#define RESUME_COUNTDOWN xdb_croolis_slot3_resume_countdown
#define RESUME_STATE xdb_croolis_slot3_resume_state
#define SLOT3_RING xdb_croolis_slot3_ring
#define SLOT11_CURSOR xdb_croolis_slot11_cursor
#define SLOT11_QUEUE_CURSOR xdb_croolis_slot11_queue_cursor
#define SLOT11_QUEUE_READ_CURSOR xdb_croolis_slot11_queue_read_cursor
#define SLOT11_CURRENT_STATE xdb_croolis_slot11_current_state
#define SLOT11_STATE_QUEUE xdb_croolis_slot11_state_queue
#define SLOT1_SELECTION_STATE xdb_croolis_slot1_selection_state
#elif defined(TEST_SCRUT)
#define SLOT3_TIMER xdb_scrut_slot3_timer
#define RESUME_COUNTDOWN xdb_scrut_slot3_resume_countdown
#define RESUME_STATE xdb_scrut_slot3_resume_state
#define SLOT3_RING xdb_scrut_slot3_ring
#define SLOT11_CURSOR xdb_scrut_slot11_cursor
#define SLOT11_QUEUE_CURSOR xdb_scrut_slot11_queue_cursor
#define SLOT11_QUEUE_READ_CURSOR xdb_scrut_slot11_queue_read_cursor
#define SLOT11_CURRENT_STATE xdb_scrut_slot11_current_state
#define SLOT11_STATE_QUEUE xdb_scrut_slot11_state_queue
#define SLOT1_SELECTION_STATE xdb_scrut_slot1_selection_state
#else
#error Select one alien module
#endif

volatile xdb_u16 xdb_alien_object_segment;
volatile xdb_u16 xdb_alien_callback_countdown;
volatile xdb_u8 xdb_alien_motion_samples[0x1000];
volatile xdb_alien_trig_sample xdb_alien_angle_table[0x400];
volatile xdb_u16 xdb_alien_control_latch;
volatile xdb_u16 xdb_alien_random_state;
volatile xdb_alien_palette_pulse xdb_alien_palette_pulse_0;
volatile xdb_alien_palette_pulse xdb_alien_palette_pulse_1;
volatile xdb_alien_palette_pulse xdb_alien_palette_pulse_2;
volatile xdb_i16 xdb_alien_view_x;
volatile xdb_i16 xdb_alien_view_y;
volatile xdb_i16 xdb_alien_view_z;
volatile xdb_i32 xdb_alien_camera_matrix[9];
volatile xdb_i32 xdb_alien_camera_position[3];
volatile xdb_i16 xdb_alien_camera_pan;
volatile xdb_i16 xdb_alien_camera_depth_step;
volatile xdb_i16 XDB_CODE_DATA xdb_alien_method_delta;
#if defined(TEST_AMER)
volatile xdb_u16 XDB_CODE_DATA SLOT2_ACTIVE;
#endif
volatile xdb_u16 XDB_CODE_DATA SLOT3_TIMER;
volatile xdb_u16 XDB_CODE_DATA RESUME_COUNTDOWN;
xdb_alien_cursor XDB_CODE_DATA RESUME_STATE;
volatile xdb_alien_ring_entry XDB_CODE_DATA SLOT3_RING[128];
xdb_alien_cursor XDB_CODE_DATA SLOT11_CURSOR;
volatile xdb_u16 XDB_CODE_DATA SLOT11_QUEUE_CURSOR;
volatile xdb_u16 XDB_CODE_DATA SLOT11_QUEUE_READ_CURSOR;
volatile xdb_u16 XDB_CODE_DATA SLOT11_CURRENT_STATE;
volatile xdb_u16 XDB_CODE_DATA SLOT11_STATE_QUEUE[8];
volatile xdb_u16 XDB_CODE_DATA SLOT1_SELECTION_STATE;

xdb_u16 XDB_NEAR xdb_test_slot3_resume_countdown(void)
{
    return RESUME_COUNTDOWN;
}

xdb_alien_cursor XDB_NEAR xdb_test_slot3_resume_state(void)
{
    return RESUME_STATE;
}

xdb_i16 XDB_NEAR xdb_test_slot3_ring_field_006(xdb_u16 ring_cursor)
{
    return SLOT3_RING[ring_cursor >> 3].field_006;
}

xdb_u16 XDB_NEAR xdb_test_slot11_queue_cursor(void)
{
    return SLOT11_QUEUE_CURSOR;
}

xdb_u16 XDB_NEAR xdb_test_slot11_queue_read_cursor(void)
{
    return SLOT11_QUEUE_READ_CURSOR;
}

xdb_u16 XDB_NEAR xdb_test_slot11_current_state(void)
{
    return SLOT11_CURRENT_STATE;
}

xdb_u16 XDB_NEAR xdb_test_slot11_state_at(xdb_u16 queue_cursor)
{
    return SLOT11_STATE_QUEUE[queue_cursor >> 1];
}

xdb_u16 XDB_NEAR xdb_test_slot1_selection_state(void)
{
    return SLOT1_SELECTION_STATE;
}

void XDB_NEAR xdb_test_set_slot3_resume_countdown(xdb_u16 countdown)
{
    RESUME_COUNTDOWN = countdown;
}

void XDB_NEAR xdb_test_set_slot11_cursor(xdb_alien_cursor state)
{
    SLOT11_CURSOR = state;
}

void XDB_NEAR xdb_test_set_slot11_queue_read_cursor(xdb_u16 queue_cursor)
{
    SLOT11_QUEUE_READ_CURSOR = queue_cursor;
}

void XDB_NEAR xdb_test_set_slot11_current_state(xdb_u16 state)
{
    SLOT11_CURRENT_STATE = state;
}

void XDB_NEAR xdb_test_set_slot11_state_at(
        xdb_u16 queue_cursor,
        xdb_u16 state)
{
    SLOT11_STATE_QUEUE[queue_cursor >> 1] = state;
}

void XDB_NEAR xdb_test_set_slot1_selection_state(xdb_u16 state)
{
    SLOT1_SELECTION_STATE = state;
}

void XDB_NEAR xdb_test_set_method_delta(xdb_i16 delta)
{
    xdb_alien_method_delta = delta;
}

#if defined(TEST_AMER)
xdb_u16 XDB_NEAR xdb_test_slot2_active(void)
{
    return SLOT2_ACTIVE;
}

void XDB_NEAR xdb_test_set_slot2_active(xdb_u16 active)
{
    SLOT2_ACTIVE = active;
}
#endif
