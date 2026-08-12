#ifndef BLOODPRG_EMS_H
#define BLOODPRG_EMS_H

#include "bloodprg_common.h"

extern volatile cb_u8 ems_transfer_mode; /* GS:0x0B9F */

void CB_NEAR ems_map_page_and_copy(cb_u16 handle,
        volatile cb_u8 CB_FAR *destination); /* 0x00BD26 */
void CB_NEAR ems_buffer_setup(cb_u16 page,
        volatile cb_u8 CB_FAR *destination); /* 0x00BD4E */
void CB_NEAR ems_page_offset_split(cb_u16 offset,
        volatile cb_u8 CB_FAR *destination); /* 0x00BD8D */
void CB_NEAR ems_transfer_dispatch(cb_u16 value,
        volatile cb_u8 CB_FAR *destination); /* 0x00BD09 */

#endif
