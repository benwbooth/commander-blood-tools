#ifndef BLOODPRG_RANDOM_H
#define BLOODPRG_RANDOM_H

#include "bloodprg_common.h"

extern volatile cb_u16 blood_prng_seed_word; /* CS:0x0AEE */
extern volatile cb_u8 blood_prng_mix_low;    /* CS:0x0AF0 */
extern volatile cb_u8 blood_prng_mix_high;   /* CS:0x0AF1 */
extern volatile cb_u8 blood_prng_counter;    /* CS:0x0AF2 */

cb_u16 CB_FAR blood_prng_next(cb_u16 modulus); /* 0x002DE2 */

#if defined(__WATCOMC__)
#pragma aux blood_prng_next parm [ax] value [ax] modify exact [ax]
#endif

#endif
