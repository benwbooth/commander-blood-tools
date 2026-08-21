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

typedef void (CB_FAR *bloodprg_alien_frame_callback)(
        cb_u16 event,
        cb_u32 clock);

typedef struct bloodprg_alien_api_request {
    volatile cb_u16 CB_FAR *timing_scale;
    bloodprg_alien_frame_callback frame_callback;
} bloodprg_alien_api_request;

typedef void (CB_FAR *bloodprg_alien_overlay_entry)(
        const volatile bloodprg_alien_api_request CB_FAR *request);

typedef void (CB_FAR *bloodprg_overlay_entry_raw)(void);

typedef union bloodprg_overlay_slot {
    volatile cb_u8 CB_FAR *load_buffer;
    bloodprg_alien_overlay_entry alien_entry;
    bloodprg_manu3_entry manu3_entry;
} bloodprg_overlay_slot;

typedef char bloodprg_alien_api_request_size_must_be_8[
        sizeof(bloodprg_alien_api_request) == 8 ? 1 : -1];
typedef char bloodprg_overlay_slot_size_must_be_4[
        sizeof(bloodprg_overlay_slot) == 4 ? 1 : -1];

extern bloodprg_manu3_entry manu3_overlay_entry; /* DS:0x0A96 */
extern bloodprg_overlay_slot alien_overlay_slot; /* DS:0x0A96 alias */
/* The binary writes the first far pointer through SS:BP; runtime SS == DS. */
extern volatile bloodprg_alien_api_request
        alien_overlay_request; /* SS=DS:0x0AE8 */
extern volatile char CB_NEAR * const
        alien_overlay_paths[3]; /* DS:0x0ACC */
extern volatile cb_u8 alien_overlay_index; /* DS:0x0AE5 */
extern volatile char manu3_overlay_path[]; /* DS:0x0113 */
/* The binary addresses this through SS:BP and starts with SS == DS. */
extern volatile bloodprg_manu3_api_request
        manu3_api_request; /* SS=DS:0x0AB4 */
extern volatile cb_u16 manu3_animation_selector_request; /* DS:0x0A32 */
extern volatile cb_u16 manu3_animation_selector_current; /* DS:0x0A34 */
extern volatile cb_u8 manu3_frame_delay; /* DS:0x0AE7 */

#if defined(BLOODPRG_RELINKED_RUNTIME)
void CB_NEAR cb_overlay_call_inherited_bp(
        bloodprg_overlay_entry_raw entry,
        const volatile void CB_NEAR *request);
#endif

void CB_NEAR manu3_hand_frame_dispatch(void); /* 0x001610 */

#endif
