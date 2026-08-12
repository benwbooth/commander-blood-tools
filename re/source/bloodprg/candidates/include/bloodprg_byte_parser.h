#ifndef BLOODPRG_BYTE_PARSER_H
#define BLOODPRG_BYTE_PARSER_H

#include "bloodprg_common.h"

extern volatile cb_u8 byte_parser_b16_flag;  /* GS:0x0B16 */
extern volatile char byte_parser_text_20b8[]; /* GS:0x20B8 */
extern volatile char credit_text_buffer[];   /* GS:0x0E18 */
extern volatile cb_u8 credit_reveal_active;  /* GS:0x5E64 */
extern volatile cb_u16 credit_reveal_timer;  /* GS:0x5E58 */

#endif
