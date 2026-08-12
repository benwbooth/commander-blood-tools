#ifndef BLOODPRG_NAV_H
#define BLOODPRG_NAV_H

#include "bloodprg_common.h"

extern volatile cb_u8 nav_choice_phase;       /* GS:0x2565 */
extern volatile cb_u16 nav_choice_honk_record; /* GS:0x6754 */
extern volatile cb_u16 nav_choice_radio_record; /* GS:0x6756 */
extern volatile cb_u16 nav_deferred_record_type; /* GS:0x6768 */
extern volatile cb_u16 nav_deferred_record_link; /* GS:0x676A */
extern volatile char nav_radio_snd_path[];    /* GS:0x0D16 */

#endif
