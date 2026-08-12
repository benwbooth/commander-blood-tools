#ifndef BLOODPRG_GRAPHICS_H
#define BLOODPRG_GRAPHICS_H

#include "bloodprg_common.h"

extern volatile cb_u8 CB_FAR *graphics_work_surface; /* GS:0x0ABC */
extern volatile cb_u8 CB_FAR *graphics_display_buffer; /* GS:0x5221 */
extern volatile cb_u8 CB_FAR *graphics_back_buffer; /* GS:0x5229 */

#endif
