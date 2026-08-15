#include "../include/bloodprg_vm.h"

typedef union bloodprg_vm_month_day {
    cb_u16 word;
    struct {
        cb_i8 day;
        cb_i8 month;
    } date;
} bloodprg_vm_month_day;

typedef struct bloodprg_vm_date_literal {
    bloodprg_vm_month_day month_day;
    cb_u16 encoded_year;
} bloodprg_vm_date_literal;

const cb_u8 CB_NEAR *CB_NEAR vm_op_cb_compare_byte(
    const cb_u8 CB_NEAR *script_bytes)
{
    cb_u8 operator;
    const bloodprg_vm_date_literal CB_NEAR *date;
    bloodprg_vm_month_day month_day;

    operator = *script_bytes++;
    date = (const bloodprg_vm_date_literal CB_NEAR *)script_bytes;
    month_day.word = date->month_day.word;
    script_bytes += sizeof(*date);

    if (operator == 0xf1u) {
        if (month_day.date.month < (cb_i8)rtc_month) {
            goto failed;
        }
        if (month_day.date.month > (cb_i8)rtc_month) {
            return script_bytes;
        }
        if (month_day.date.day <= (cb_i8)rtc_day) {
            goto failed;
        }
        return script_bytes;
    } else if (operator == 0xf2u) {
        if (month_day.date.month > (cb_i8)rtc_month) {
            goto failed;
        }
        if (month_day.date.month < (cb_i8)rtc_month) {
            return script_bytes;
        }
        if (month_day.date.day >= (cb_i8)rtc_day) {
            goto failed;
        }
        return script_bytes;
    } else {
        if (month_day.date.month != (cb_i8)rtc_month) {
            goto failed;
        }
        if (month_day.date.day == (cb_i8)rtc_day) {
            return script_bytes;
        }
    }

failed:
    return (const cb_u8 CB_NEAR *)vm_branch_fail();
}
