#ifndef BLOODPRG_MANU3_H
#define BLOODPRG_MANU3_H

#include "bloodprg_common.h"

typedef struct bloodprg_manu3_cursor_position {
    cb_i16 x;
    cb_i16 y;
} bloodprg_manu3_cursor_position;

typedef struct bloodprg_manu3_api_request {
    bloodprg_manu3_cursor_position cursor;
    cb_u16 animation_selector;
    cb_u16 framebuffer_window_offset;
} bloodprg_manu3_api_request;

typedef char bloodprg_manu3_api_request_size_must_be_8[
        sizeof(bloodprg_manu3_api_request) == 8 ? 1 : -1];

typedef void (CB_FAR *bloodprg_manu3_entry)(
        const volatile bloodprg_manu3_api_request CB_FAR *request);

extern bloodprg_manu3_entry manu3_overlay_entry; /* DS:0x0A96 */
/* The binary addresses this through SS:BP and starts with SS == DS. */
extern volatile bloodprg_manu3_api_request
        manu3_api_request; /* SS=DS:0x0AB4 */
extern volatile cb_u16 manu3_animation_selector_request; /* DS:0x0A32 */
extern volatile cb_u16 manu3_animation_selector_current; /* DS:0x0A34 */
extern volatile cb_u8 manu3_frame_delay; /* DS:0x0AE7 */

void CB_NEAR manu3_hand_frame_dispatch(void); /* 0x001610 */

#endif
