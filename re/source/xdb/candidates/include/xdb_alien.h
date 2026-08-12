#ifndef XDB_ALIEN_H
#define XDB_ALIEN_H

#include "xdb_common.h"

#define XDB_ALIEN_CURSOR_BIAS 0x005eu
#define XDB_ALIEN_FIELD_DELTA 0x000fu

typedef struct xdb_alien_biased_state {
    xdb_u8 field_000[0x42];
    xdb_i32 position_x;
    xdb_i32 position_y;
    xdb_i32 position_z;
    xdb_u8 field_04e[0x04];
    xdb_i16 field_052;
    xdb_u8 field_054[0x0a];
} xdb_alien_biased_state;

typedef struct xdb_alien_state {
    xdb_u8 field_000[0x0b0];
    xdb_i16 field_0b0;
} xdb_alien_state;

typedef struct xdb_alien_object_record {
    volatile xdb_i16 position;
    xdb_u8 field_002[0x12];
} xdb_alien_object_record;

typedef struct xdb_alien_method_context xdb_alien_method_context;
typedef void XDB_NEAR xdb_alien_resume_function(
        xdb_alien_method_context XDB_NEAR *context);
typedef xdb_alien_resume_function XDB_NEAR *xdb_alien_resume_callback;

struct xdb_alien_method_context {
    xdb_u8 field_00[0x16];
    volatile xdb_alien_state XDB_NEAR *state;
    xdb_u8 field_018[0x02];
    xdb_u16 state_count;
    xdb_u16 object_offset;
    xdb_u16 field_01e;
    xdb_u16 object_count;
    xdb_u8 field_022[0x14];
    xdb_alien_resume_callback resume;
    union {
        struct {
            xdb_u16 step;
            xdb_u16 value;
        } resume_state;
        struct {
            xdb_u16 cursor;
            xdb_i16 previous;
        } sample_state;
    } continuation;
};

typedef volatile xdb_u8 XDB_NEAR *xdb_alien_cursor;

extern volatile xdb_i16 XDB_CODE_DATA xdb_alien_method_delta; /* CS:0x0099 */
extern volatile xdb_u16 xdb_alien_object_segment; /* DS:0x0002 */
extern volatile xdb_u8 xdb_alien_motion_samples[]; /* DS:0x0036 */
extern volatile xdb_i16 xdb_alien_view_x; /* DS:0x22EC */
extern volatile xdb_i16 xdb_alien_view_y; /* DS:0x22F0 */
extern volatile xdb_i16 xdb_alien_view_z; /* DS:0x22F4 */
extern xdb_alien_cursor XDB_CODE_DATA
        xdb_amer_slot11_cursor; /* AMER CS:0x1BC2 */
extern xdb_alien_cursor XDB_CODE_DATA
        xdb_croolis_slot11_cursor; /* CROOLIS CS:0x1B2E */
extern xdb_alien_cursor XDB_CODE_DATA
        xdb_scrut_slot11_cursor; /* SCRUT CS:0x1BE3 */

volatile xdb_u8 XDB_NEAR *XDB_NEAR xdb_amer_method_slot_11_anchor_state(
        const xdb_alien_method_context XDB_NEAR *context);
volatile xdb_u8 XDB_NEAR *XDB_NEAR xdb_croolis_method_slot_11_anchor_state(
        const xdb_alien_method_context XDB_NEAR *context);
volatile xdb_u8 XDB_NEAR *XDB_NEAR xdb_scrut_method_slot_11_anchor_state(
        const xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_amer_method_slot_12_apply_delta(
        const xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_croolis_method_slot_12_apply_delta(
        const xdb_alien_method_context XDB_NEAR *context);
volatile xdb_u8 XDB_NEAR *XDB_NEAR xdb_scrut_method_slot_12_lower_state(
        const xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_method_slot_13_resume_or_init(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_method_slot_13_resume_or_init(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_method_slot_13_resume_or_init(
        xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_amer_method_slot_8_apply_sample_delta(
        xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_croolis_method_slot_8_apply_sample_delta(
        xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_scrut_method_slot_8_apply_sample_delta(
        xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_amer_method_slot_9_apply_scaled_sample_delta(
        xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_croolis_method_slot_9_apply_scaled_sample_delta(
        xdb_alien_method_context XDB_NEAR *context);
xdb_i16 XDB_NEAR xdb_scrut_method_slot_9_apply_scaled_sample_delta(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_amer_method_slot_6_wrap_positions(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_method_slot_6_wrap_positions(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_method_slot_6_wrap_positions(
        xdb_alien_method_context XDB_NEAR *context);

void XDB_NEAR xdb_amer_resume_1c34(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_croolis_resume_1b85(
        xdb_alien_method_context XDB_NEAR *context);
void XDB_NEAR xdb_scrut_resume_1c45(
        xdb_alien_method_context XDB_NEAR *context);

#if defined(__WATCOMC__)
#pragma aux xdb_alien_resume_function parm [di]
#pragma aux xdb_amer_method_slot_11_anchor_state parm [di] value [si] modify exact [si]
#pragma aux xdb_croolis_method_slot_11_anchor_state parm [di] value [si] modify exact [si]
#pragma aux xdb_scrut_method_slot_11_anchor_state parm [di] value [si] modify exact [si]
#pragma aux xdb_amer_method_slot_12_apply_delta parm [di] value [ax] modify exact [ax si]
#pragma aux xdb_croolis_method_slot_12_apply_delta parm [di] value [ax] modify exact [ax si]
#pragma aux xdb_scrut_method_slot_12_lower_state parm [di] value [si] modify exact [si]
#pragma aux xdb_amer_method_slot_13_resume_or_init parm [di]
#pragma aux xdb_croolis_method_slot_13_resume_or_init parm [di]
#pragma aux xdb_scrut_method_slot_13_resume_or_init parm [di]
#pragma aux xdb_amer_method_slot_8_apply_sample_delta \
        parm [di] value [ax] modify exact [ax bx cx si]
#pragma aux xdb_croolis_method_slot_8_apply_sample_delta \
        parm [di] value [ax] modify exact [ax bx cx si]
#pragma aux xdb_scrut_method_slot_8_apply_sample_delta \
        parm [di] value [ax] modify exact [ax bx cx si]
#pragma aux xdb_amer_method_slot_9_apply_scaled_sample_delta \
        parm [di] value [ax] modify exact [ax bx cx si]
#pragma aux xdb_croolis_method_slot_9_apply_scaled_sample_delta \
        parm [di] value [ax] modify exact [ax bx cx si]
#pragma aux xdb_scrut_method_slot_9_apply_scaled_sample_delta \
        parm [di] value [ax] modify exact [ax bx cx si]
#pragma aux xdb_amer_method_slot_6_wrap_positions \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_croolis_method_slot_6_wrap_positions \
        parm [di] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_scrut_method_slot_6_wrap_positions \
        parm [di] modify exact [ax bx cx dx si di bp]
#endif

#endif
