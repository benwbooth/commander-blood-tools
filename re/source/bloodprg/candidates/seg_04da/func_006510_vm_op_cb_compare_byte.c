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

bloodprg_vm_image_ptr CB_NEAR vm_op_cb_compare_byte(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_u8 operator;
    const volatile bloodprg_vm_date_literal CB_FAR *date;
    bloodprg_vm_month_day month_day;

    operator = *script_bytes++;
    date = (const volatile bloodprg_vm_date_literal CB_FAR *)script_bytes;
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
    return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
}
