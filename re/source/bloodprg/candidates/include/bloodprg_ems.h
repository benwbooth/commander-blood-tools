#ifndef BLOODPRG_EMS_H
#define BLOODPRG_EMS_H

#include "bloodprg_common.h"

typedef struct bloodprg_xms_read_request {
    cb_u32 length;
    cb_u16 source_handle;
    cb_u32 source_offset;
    cb_u16 destination_handle;
    volatile cb_u8 CB_FAR *destination;
} bloodprg_xms_read_request;

typedef void (CB_FAR *bloodprg_xms_driver_entry)(void);

/* Both recovered callers establish DS=GS before this dispatch. */
extern volatile cb_u8 ems_transfer_mode; /* DS=GS:0x0B9F */
extern bloodprg_xms_driver_entry CB_GAME_DATA xms_driver_entry; /* GS:0x0A4A */
extern volatile cb_i16 CB_GAME_DATA resource_xms_handle; /* GS:0x0A56 */
extern volatile cb_i16 CB_GAME_DATA resource_ems_handle; /* GS:0x0A58 */
extern volatile cb_i16 CB_GAME_DATA secondary_xms_handle; /* GS:0x0A5A */
extern volatile cb_i16 CB_GAME_DATA secondary_ems_handle; /* GS:0x0A5C */
extern volatile cb_i16 CB_GAME_DATA archive_xms_handle; /* GS:0x0A5E */
extern volatile cb_i16 CB_GAME_DATA archive_ems_handle; /* GS:0x0A60 */
extern volatile cb_i16 CB_GAME_DATA small_xms_handle; /* GS:0x0A62 */
extern volatile cb_i16 CB_GAME_DATA small_ems_handle; /* GS:0x0A64 */
extern volatile cb_u16 CB_GAME_DATA ems_page_frame_segment; /* GS:0x0A66 */
extern volatile bloodprg_xms_read_request
        resource_xms_read_request; /* game data:0x0A6C */
extern volatile cb_u8 CB_FAR ems_page_frame[]; /* segment at GS:0x0A66 */

void CB_NEAR cb_ems_map_page(cb_u16 handle, cb_u16 logical_page,
        cb_u8 physical_page);
void CB_NEAR cb_xms_move(volatile bloodprg_xms_read_request *request);
int CB_NEAR cb_xms_allocate_kb(cb_u16 kilobytes, cb_u16 *handle);
void CB_NEAR cb_xms_release(cb_u16 handle);
void CB_FAR far_memmove(volatile cb_u8 CB_FAR *destination,
        const volatile cb_u8 CB_FAR *source, cb_u32 byte_count); /* 01CE:0B93 */

void CB_NEAR ems_map_page_and_copy(cb_u16 handle,
        volatile cb_u8 CB_FAR *destination); /* 0x00BD26 */
void CB_NEAR ems_buffer_setup(cb_u16 page,
        volatile cb_u8 CB_FAR *destination); /* 0x00BD4E */
void CB_NEAR ems_page_offset_split(cb_u16 offset,
        volatile cb_u8 CB_FAR *destination); /* 0x00BD8D */
void CB_NEAR ems_transfer_dispatch(cb_u16 value,
        volatile cb_u8 CB_FAR *destination); /* 0x00BD09 */
void CB_FAR extended_memory_backends_init(void); /* 0x00099F */
void CB_FAR extended_memory_backends_release(void); /* 0x000A99 */

#if defined(__WATCOMC__)
#pragma aux ems_map_page_and_copy parm [ax] [es di] modify exact []
#pragma aux ems_buffer_setup parm [ax] [es di] modify exact []
#pragma aux ems_page_offset_split parm [ax] [es di] modify exact []
#pragma aux ems_transfer_dispatch parm [ax] [es di]
#endif

#endif
