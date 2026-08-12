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

extern volatile cb_u8 ems_transfer_mode; /* GS:0x0B9F */
extern volatile cb_i16 resource_xms_handle; /* game data:0x0A56 */
extern volatile cb_i16 resource_ems_handle; /* game data:0x0A58 */
extern volatile bloodprg_xms_read_request
        resource_xms_read_request; /* game data:0x0A6C */
extern volatile cb_u8 CB_FAR ems_page_frame[]; /* segment at GS:0x0A66 */

void CB_NEAR cb_ems_map_page(cb_u16 handle, cb_u16 logical_page,
        cb_u8 physical_page);
void CB_NEAR cb_xms_move(volatile bloodprg_xms_read_request *request);
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

#endif
