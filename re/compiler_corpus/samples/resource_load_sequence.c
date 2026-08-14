/* Codegen probe for BLOODPRG 0x00A15F. */
typedef unsigned int u16;

#define FAR far
#define NEAR near

#define INITIAL_REFILL_COUNT 50u
#define SKIP_INITIAL_REFILL 0x0040u

extern volatile u16 FAR *list_tail_pointer_probe;
extern volatile u16 list_default_entry_segment_probe;
extern volatile u16 list_read_wrap_index_probe;
extern volatile u16 list_sequence_index_probe;
extern volatile u16 list_wrap_count_probe;
extern volatile u16 resource_flags_probe;
extern volatile u16 timer_tick_probe;
extern volatile u16 list_previous_tick_probe;

int NEAR resource_switch_probe(u16 resource_id);
int NEAR banked_list_load_probe(void);
void NEAR list_activate_entry_probe(
        u16 entry_extent,
        volatile u16 FAR *entry,
        u16 storage_segment);
void FAR list_active_present_probe(void);
void FAR list_init_probe(void);
u16 NEAR list_refill_probe(u16 link_target_offset);

void NEAR resource_load_sequence_probe(u16 resource_id)
{
    volatile u16 FAR *entry;
    u16 entry_extent;
    u16 link_target_offset;
    u16 refill_count;

    if (!resource_switch_probe(resource_id)) {
        return;
    }
    if (!banked_list_load_probe()) {
        return;
    }

    entry = list_tail_pointer_probe;
    entry_extent = *entry++;
    link_target_offset = list_default_entry_segment_probe;
    list_activate_entry_probe(entry_extent, entry, link_target_offset);
    list_active_present_probe();
    list_init_probe();

    ++list_read_wrap_index_probe;
    ++list_sequence_index_probe;
    ++list_wrap_count_probe;

    if ((resource_flags_probe & SKIP_INITIAL_REFILL) == 0u) {
        for (refill_count = 0u;
                refill_count < INITIAL_REFILL_COUNT;
                ++refill_count) {
            link_target_offset = list_refill_probe(link_target_offset);
        }
    }
    list_previous_tick_probe = timer_tick_probe;
}
