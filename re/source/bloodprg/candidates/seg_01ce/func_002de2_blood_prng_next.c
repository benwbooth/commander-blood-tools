#include "../include/bloodprg_random.h"

volatile cb_u16 CB_CODE_DATA blood_prng_seed_word = 0;
volatile cb_u8 CB_CODE_DATA blood_prng_mix_low = 0;
volatile cb_u8 CB_CODE_DATA blood_prng_mix_high = 0;
volatile cb_u8 CB_CODE_DATA blood_prng_counter = 0;

cb_u16 CB_FAR blood_prng_next(cb_u16 modulus)
{
    cb_u8 low;
    cb_u8 high;
    cb_u8 step;
    cb_u16 value;
    cb_u16 round;

    low = blood_prng_mix_low;
    high = blood_prng_mix_high;
    value = 0;
    for (round = 0; round < 8u; ++round) {
        value = (cb_u16)((value << 1) | (low & 1u));
        low = (cb_u8)(low >> 1);
        value = (cb_u16)((value << 1) | (high >> 7));
        high = (cb_u8)(high << 1);
    }

    value ^= blood_prng_seed_word;

    step = (cb_u8)(blood_prng_counter + 1u);
    blood_prng_counter = step;
    blood_prng_mix_high = (cb_u8)(blood_prng_mix_high - step);
    blood_prng_mix_low ^= (cb_u8)((step << 1) | (step >> 7));

    if (modulus != 0) {
        while (value >= modulus) {
            value = (cb_u16)(value - modulus);
        }
    }
    return value;
}
