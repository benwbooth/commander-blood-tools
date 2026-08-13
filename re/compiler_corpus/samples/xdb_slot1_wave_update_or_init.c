/* Codegen probe for the complete alien XDB slot-1 wave method. */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;
typedef signed long i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#define NEAR near
#define FAR_AT(type, segment, offset) \
    ((type FAR *)MK_FP((segment), (offset)))
#else
#define FAR
#define NEAR
#endif

#if defined(__WATCOMC__)
#define CODE_DATA __based(__segname("_CODE"))
#else
#define CODE_DATA
#endif

typedef struct alien_biased_state {
    u8 field_000[0x42];
    i32 position_x;
    i32 position_y;
    i32 position_z;
    i16 field_04e;
    u16 field_050;
    i16 field_052;
    i16 field_054;
} alien_biased_state;

typedef struct alien_state {
    u8 field_000[0x0b0];
} alien_state;

typedef struct alien_wave_object {
    u8 field_000[0x04];
    i16 distance;
    i16 motion;
    u16 phase;
    u8 field_00a[0x0a];
} alien_wave_object;

typedef struct alien_method_context {
    u8 field_000[0x16];
    alien_state NEAR *state;
    u8 field_018[0x04];
    u16 object_offset;
    u16 field_01e;
    u16 object_count;
    u8 field_022[0x14];
    i16 control_state;
    u16 primary_phase;
    i16 primary_step;
    u16 secondary_phase;
    u16 secondary_step;
} alien_method_context;

typedef volatile alien_biased_state NEAR *alien_state_cursor;

extern volatile u16 object_segment;
extern volatile u8 motion_samples[];
extern volatile i16 view_x;
extern volatile i16 view_y;
extern volatile i16 view_z;
extern volatile u16 CODE_DATA selection_state;
extern alien_state_cursor CODE_DATA selected_state;
extern volatile i16 CODE_DATA current_sample;

void NEAR xdb_slot1_wave_update_or_init_probe(
        alien_method_context NEAR *context);

#if defined(__WATCOMC__)
#pragma aux xdb_slot1_wave_update_or_init_probe \
        parm [di] modify exact [ax bx cx dx si di bp]
#endif

void NEAR xdb_slot1_wave_update_or_init_probe(
        alien_method_context NEAR *context)
{
    volatile alien_biased_state NEAR *state =
            (volatile alien_biased_state NEAR *)
            ((volatile u8 NEAR *)context->state + 0x005e);
    alien_wave_object FAR *object;
    u16 count;
    u16 phase;
    u16 sample_offset;
    u16 scale;
    i16 sample;
    i16 step;
    i16 distance;
    i32 product;

    if (context->control_state == 0) {
        context->control_state = 1;
        selection_state = 0;
        context->primary_phase = 4;
        context->primary_step = 0x30;
        context->secondary_phase = 4;
        context->secondary_step = 0x10;
        state->field_054 = 0x0c;
        state->field_04e = 0;
        state->field_050 = 0;
        state->field_052 = 0;
        return;
    }

    ++state->field_050;
    phase = context->primary_phase & 0x0ffcu;
    sample = *(volatile i16 NEAR *)(motion_samples + phase);
    sample >>= 8;
    current_sample = sample;
    if ((selection_state & 1u) != 0u) {
        i16 value = (i16)(
                (u16)sample - 60u +
                (u16)(i16)state->position_y + (u16)view_y);
        if (value >= 0 && value <= 128) {
            value = (i16)((u16)(i16)state->position_x + (u16)view_x);
            if (value >= -256 && value <= 256) {
                value = (i16)((u16)(i16)state->position_z + (u16)view_z);
                if (value >= -256 && value <= 256) {
                    selection_state = 2;
                    selected_state = state;
                    context->primary_step = 0x170;
                }
            }
        }
    }

    step = context->primary_step;
    if (step > 0x30) {
        step = (i16)((u16)step - 4u);
        context->primary_step = step;
    }
    phase = context->primary_phase;
    context->primary_phase = (u16)(phase + (u16)step);
    object = FAR_AT(alien_wave_object, object_segment, context->object_offset);
    count = context->object_count;
    do {
        sample_offset = ((u16)(object->phase * 2u) + phase) & 0x0ffcu;
        sample = *(volatile i16 NEAR *)(motion_samples + sample_offset);
        object->motion = (i16)(
                (u16)object->motion - (u16)(sample >> 8));
        sample_offset = (sample_offset + (u16)step) & 0x0ffcu;
        sample = *(volatile i16 NEAR *)(motion_samples + sample_offset);
        object->motion = (i16)(
                (u16)object->motion + (u16)(sample >> 8));
        ++object;
    } while (--count != 0u);

    step = (i16)context->secondary_step;
    phase = (u16)(context->secondary_phase + (u16)step);
    context->secondary_phase = phase;
    object = FAR_AT(alien_wave_object, object_segment, context->object_offset);
    count = context->object_count;
    do {
        distance = (i16)((u16)object->distance - 25u);
        if (distance < 0) {
            distance = (i16)(0u - (u16)distance);
            distance = (i16)((u16)distance - 50u);
        }
        if (distance >= 0) {
            scale = (u16)distance * 2u;
            sample_offset = (scale + phase) & 0x0ffcu;
            sample = *(volatile i16 NEAR *)(motion_samples + sample_offset);
            product = (i32)sample * (u16)scale;
            object->motion = (i16)(
                    (u16)object->motion - (u16)(i16)(product >> 17));
            sample_offset = (sample_offset + (u16)step) & 0x0ffcu;
            sample = *(volatile i16 NEAR *)(motion_samples + sample_offset);
            product = (i32)sample * (u16)scale;
            object->motion = (i16)(
                    (u16)object->motion + (u16)(i16)(product >> 17));
        }
        ++object;
    } while (--count != 0u);
}
