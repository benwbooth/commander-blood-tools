#include "../include/bloodprg_common.h"

/* CS:0x02DA, immediately before ascii_digit_parse in the recovered image. */
static const cb_u16 decimal_digit_place_values[] = {
    0u, 1u, 2u, 3u, 4u, 5u, 6u, 7u, 8u, 9u,
    0u, 10u, 20u, 30u, 40u, 50u, 60u, 70u, 80u, 90u,
    0u, 100u, 200u, 300u, 400u, 500u, 600u, 700u, 800u, 900u,
    0u, 1000u, 2000u, 3000u, 4000u, 5000u, 6000u, 7000u, 8000u, 9000u,
    0u, 10000u, 20000u, 30000u
};

cb_i16 CB_FAR ascii_digit_parse(const char CB_NEAR *text)
{
    const char CB_NEAR *cursor;
    cb_u16 digit_count;
    cb_u16 place;
    cb_u16 value;
    int negative;

    cursor = text;
    negative = 0;
    if ((cb_i8)*cursor < (cb_i8)'0') {
        if (*cursor != '+' && *cursor != '-') {
            return 0;
        }
        negative = *cursor == '-';
        ++cursor;
    } else if ((cb_i8)*cursor > (cb_i8)'9') {
        return 0;
    }

    digit_count = 0;
    while ((cb_i8)*cursor >= (cb_i8)'0' &&
           (cb_i8)*cursor <= (cb_i8)'9') {
        ++cursor;
        ++digit_count;
    }

    value = 0;
    place = 0;
    while (digit_count != 0) {
        cb_u16 digit;

        --cursor;
        digit = (cb_u16)((cb_u8)*cursor - (cb_u8)'0');
        value = (cb_u16)(
            value + decimal_digit_place_values[place * 10u + digit]
        );
        ++place;
        --digit_count;
    }

    if (negative) {
        value = (cb_u16)(0u - value);
    }
    return (cb_i16)value;
}
