/* Codegen probe for BLOODPRG 0x00A1B4. */
typedef signed char i8;
typedef unsigned char u8;
typedef unsigned int u16;

#define FAR far
#define NEAR near

extern volatile u8 resource_source_is_banked_probe;
extern volatile u16 resource_flags_probe;
extern volatile u16 list_file_handle_probe;
extern volatile u16 list_palette_offset_probe;
extern volatile u8 list_rollover_state_probe;

int NEAR list_activate_ready_probe(void);
int NEAR list_advance_due_probe(void);
void NEAR list_refill_probe(u16 link_target_offset);
void NEAR list_refill_with_latch_probe(u16 link_target_offset);
volatile u8 FAR *NEAR list_palette_apply_probe(void);
void FAR list_active_present_probe(void);
void NEAR queue_consume_probe(void);

void NEAR ems_resource_flush_probe(u16 link_target_offset)
{
    for (;;) {
        if ((resource_source_is_banked_probe & 1u) == 0u) {
            if (list_file_handle_probe == 0u) {
                list_rollover_state_probe = 0u;
                return;
            }
            if ((i8)(u8)resource_flags_probe < 0) {
                list_refill_with_latch_probe(link_target_offset);
                return;
            }
        }

        if (!list_activate_ready_probe()) {
            list_refill_probe(link_target_offset);
            continue;
        }

        if (list_advance_due_probe()) {
            if (list_palette_offset_probe != 0xFFFFu) {
                (void)list_palette_apply_probe();
            }
            list_active_present_probe();
            queue_consume_probe();
        }
        list_refill_with_latch_probe(link_target_offset);
        return;
    }
}
