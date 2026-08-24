#include <dos.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_byte_parser.h"
#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_list.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define RECORD_KIND_C1 0x00c1u
#define RECORD_KIND_C2 0x00c2u
#define RECORD_KIND_C3 0x00c3u
#define RECORD_KIND_C4 0x00c4u
#define RECORD_KIND_C6 0x00c6u
#define RECORD_KIND_C9 0x00c9u
#define RECORD_KIND_CD 0x00cdu

#define OBJECT_KIND_ACTOR 0x0001u
#define OBJECT_KIND_CHARACTER 0x0002u
#define OBJECT_KIND_SHIP 0x0010u
#define OBJECT_KIND_LINK_MARKER 0x0020u
#define OBJECT_KIND_SPECIAL 0x0200u
#define OBJECT_KIND_DESCRIPT 0x0400u

#define FIELD_SELECTOR_COUNTER 0x0008u
#define FIELD_SELECTOR_POSITION_A 0x0009u
#define FIELD_SELECTOR_POSITION_B 0x000au
#define FIELD_SELECTOR_POSITION 0x000bu
#define FIELD_SELECTOR_COMPARISON 0x000cu
#define FIELD_SELECTOR_RELATION_MATCH 0x000du
#define FIELD_SELECTOR_RELATION 0x000eu
#define FIELD_SELECTOR_PARENT 0x0011u
#define FIELD_SELECTOR_RECIPROCAL 0x0013u

#define UI_PRESENTATION_ACTIVE 0x0001u
#define UI_CAMERA_TRANSITION 0x0004u
#define PRESENTATION_REQUEST_DESCRIPT 0x02u
#define OBJECT_FLAG_ACTIVE 0x0001u
#define OBJECT_FLAG_POST_UPDATE 0x8000u
#define PRESENTATION_MUSIC_PATH_OFFSET 0x0d2du

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define RECORD_AT(segment, offset, type) \
    ((volatile type CB_FAR *)MK_FP((segment), (cb_u16)(offset)))
#define RECORD_OFFSET(pointer) ((cb_u16)FP_OFF(pointer))
#else
#define RECORD_AT(segment, offset, type) \
    ((volatile type CB_FAR *)(vm_record_base_gs + (cb_u16)(offset)))
#define RECORD_OFFSET(pointer) \
    ((cb_u16)((volatile cb_u8 CB_FAR *)(pointer) - vm_record_base_gs))
#endif

void CB_NEAR record_c1_ship3d_action(
        volatile bloodprg_vm_scan_object CB_FAR *object,
        volatile bloodprg_vm_record_triple CB_FAR *record)
{
    volatile bloodprg_vm_scan_object CB_FAR *related;
    volatile bloodprg_vm_scan_object CB_FAR *prior;
    volatile bloodprg_vm_record_triple CB_FAR *reciprocal;
    volatile bloodprg_vm_record_triple CB_FAR *primary;
    volatile ship_3d_position_field CB_FAR *source_position;
    volatile ship_3d_position_field CB_FAR *owner_position;
    volatile ship_3d_position_field CB_FAR *related_position;
    volatile cb_u16 CB_FAR *word_field;
    bloodprg_graphics_buffer_ptr volatile saved_framebuffer;
    cb_u16 record_segment;
    cb_u16 owner_offset;
    cb_u16 related_offset;
    cb_u16 owner_kind;
    cb_u16 related_kind;
    cb_u16 field_offset;
    cb_u16 position_compare_word;
    cb_u16 relation;

    record_segment = FP_SEG(object);
    owner_offset = RECORD_OFFSET(object);
    owner_kind = object->kind;

    switch (record->kind) {
    case RECORD_KIND_C1:
        if (owner_offset == vm_arche_record_offset_gs
                && nav_camera_approach_phase_gs < 4u) {
            return;
        }

        field_offset = (cb_u16)vm_field_offset(
                FIELD_SELECTOR_PARENT, owner_kind);
        position_compare_word = field_offset;
        word_field = RECORD_AT(
                record_segment, owner_offset + field_offset, cb_u16);
        prior = RECORD_AT(record_segment, *word_field,
                bloodprg_vm_scan_object);
        if (prior->kind == OBJECT_KIND_LINK_MARKER) {
            prior->flags &= (cb_u16)~OBJECT_FLAG_ACTIVE;
        }

        related_offset = record->related;
        related = RECORD_AT(record_segment, related_offset,
                bloodprg_vm_scan_object);
        related_kind = related->kind;
        (void)(related_kind == owner_kind);
        *word_field = related_offset;
        record->kind = 0u;

        if (owner_kind == OBJECT_KIND_SPECIAL) {
            if ((vm_ship_active_flags_gs & 1u) != 0u) {
                if (related_offset != ship_3d_current_target_gs) {
                    if (!vm_c2_descript_lookup(related->name)) {
                        goto copy_position;
                    }
                    if ((music_voc_name_changed & 1u) != 0u) {
                        saved_framebuffer = graphics_draw_framebuffer;
                        graphics_draw_framebuffer = graphics_screen_buffer;
                        ship_3d_plane_band_copy();
                        graphics_draw_framebuffer = saved_framebuffer;
                        snd_driver_call();
                        snd_stream_source_load(
                                presentation_music_voc_path_gs);
                        snd_stream_start();
                        /* The shipped routine reuses SI for this pathname and
                         * then treats that offset as the position owner. */
                        owner_offset = PRESENTATION_MUSIC_PATH_OFFSET;
                    }
                }

                primary = RECORD_AT(
                        record_segment,
                        vm_primary_c4_record_gs,
                        bloodprg_vm_record_triple);
                if (primary->kind == RECORD_KIND_C4) {
                    related_offset = primary->related;
                    primary->kind = 0u;
                    primary->related = 0u;
                    related = RECORD_AT(record_segment, related_offset,
                            bloodprg_vm_scan_object);
                    field_offset = (cb_u16)vm_field_offset(
                            FIELD_SELECTOR_RECIPROCAL, related->kind);
                    reciprocal = RECORD_AT(
                            record_segment,
                            related_offset + field_offset,
                            bloodprg_vm_record_triple);
                    reciprocal->kind = 0u;
                    reciprocal->related = 0u;
                    reciprocal->value = 0u;
                }

                related_offset = record->related;
                ship_3d_current_target_gs = related_offset;
                vm_ship_active_flags_gs = 9u;
                vm_presentation_word_buffer_gs[0] = 0u;
                vm_word_choice_active_gs = 0u;
                vm_presentation_request_flags_gs = 0u;
                vm_c2_presentation_gate_gs = 0u;
                ship_3d_hud_initialized_word_gs = 1u;
                vm_bridge_redraw_pending_gs = 0u;
                resource_vertical_offset_gs = byte_parser_word_1fa5;
                vm_active_line_gs = 3u;
            }
        } else if (owner_kind != OBJECT_KIND_SHIP) {
            return;
        }

copy_position:
        related_offset = record->related;
        source_position = ship_3d_position_field_resolve(
                RECORD_AT(record_segment, related_offset,
                    bloodprg_vm_object_header),
                position_compare_word);
        if (source_position == 0) {
            return;
        }
        field_offset = (cb_u16)vm_field_offset(
                FIELD_SELECTOR_POSITION, owner_kind);
        owner_position = RECORD_AT(
                record_segment,
                owner_offset + field_offset,
                ship_3d_position_field);
        owner_position->x = source_position->x;
        owner_position->y = source_position->y;
        return;

    case RECORD_KIND_C2:
        related_offset = record->related;
        if (!vm_special_slot_insert(related_offset)) {
            return;
        }
        related = RECORD_AT(record_segment, related_offset,
                bloodprg_vm_scan_object);
        related_kind = related->kind;
        field_offset = (cb_u16)vm_field_offset(
                FIELD_SELECTOR_PARENT, related_kind);
        word_field = RECORD_AT(
                record_segment, related_offset + field_offset, cb_u16);
        *word_field = 0xffffu;
        record->kind = 0u;

        if ((vm_ui_state_gs.word & UI_PRESENTATION_ACTIVE) == 0u
                && (vm_presentation_request_flags_gs
                    & PRESENTATION_REQUEST_DESCRIPT) == 0u) {
            if (related_kind == OBJECT_KIND_CHARACTER) {
                vm_c2_presentation_gate_gs = 0u;
                vm_active_line_gs = 0x0027u;
            } else if (related_kind == OBJECT_KIND_DESCRIPT
                    && vm_c2_descript_lookup(related->name)) {
                vm_c2_presentation_gate_gs = 0u;
                vm_active_line_gs = 0x002bu;
                vm_presentation_request_flags_gs |=
                        PRESENTATION_REQUEST_DESCRIPT;
            }
        }
        /* The shipped success tail has an unmatched POP ES at 0x005D33.
         * No shipped VAR image or native deferred-type writer supplies a C2
         * action record, so the corrupt tail is dormant in original data.
         * Preserve its intended state changes and return safely. */
        return;

    case RECORD_KIND_C3:
        related_offset = record->related;
        if (related_offset != vm_wildcard_ref_value_gs) {
            record->kind = RECORD_KIND_C4;
            record->value = 0u;
            return;
        }
        nav_pending_record_link_gs = owner_offset;
        if ((vm_ui_state_gs.word & UI_PRESENTATION_ACTIVE) == 0u) {
            return;
        }
        if ((voc_playback_enabled_gs & 1u) == 0u) {
            snd_clip_enable_request_gs |= 1u;
        }
        if (snd_clip_playback_state_gs == 0u) {
            snd_play_clip(6);
            snd_clip_playback_state_gs = 2u;
        }
        return;

    case RECORD_KIND_C4:
        if (record->value != 0u
                || (vm_presentation_pair_write_disabled & 1u) != 0u) {
            return;
        }
        record->value = 0xffffu;
        related_offset = record->related;
        related = RECORD_AT(record_segment, related_offset,
                bloodprg_vm_scan_object);
        related_kind = related->kind;

        if (owner_kind == OBJECT_KIND_ACTOR) {
            nav_pending_record_link_gs = 0u;
            field_offset = (cb_u16)vm_field_offset(
                    FIELD_SELECTOR_COUNTER, related_kind);
            if (field_offset != 0u) {
                word_field = RECORD_AT(
                        record_segment,
                        related_offset + field_offset,
                        cb_u16);
                ++*word_field;
                object->flags |= OBJECT_FLAG_POST_UPDATE;
                vm_post_update_record_offset = related_offset;
                (void)vm_cod_scan(related_offset);
                goto write_reciprocal;
            }
        }
        if (related_kind == OBJECT_KIND_ACTOR) {
            field_offset = (cb_u16)vm_field_offset(
                    FIELD_SELECTOR_COUNTER, owner_kind);
            if (field_offset != 0u) {
                word_field = RECORD_AT(
                        record_segment,
                        owner_offset + field_offset,
                        cb_u16);
                ++*word_field;
                object->flags |= OBJECT_FLAG_POST_UPDATE;
                vm_post_update_record_offset = owner_offset;
                (void)vm_cod_scan(owner_offset);
            }
        }

write_reciprocal:
        field_offset = (cb_u16)vm_field_offset(
                FIELD_SELECTOR_RECIPROCAL, related_kind);
        reciprocal = RECORD_AT(
                record_segment,
                related_offset + field_offset,
                bloodprg_vm_record_triple);
        reciprocal->kind = RECORD_KIND_C4;
        reciprocal->related = owner_offset;
        reciprocal->value = 0xffffu;
        return;

    case RECORD_KIND_C6:
        if (nav_actor_transition_phase_gs == 0u) {
            if (nav_actor_0_busy_gs != 1u) {
                return;
            }
            ++nav_actor_transition_phase_gs;
            nav_camera_view_state_gs = 8u;
            entity_flag_state_transition(4u);
            return;
        }
        if (nav_camera_view_state_gs != 0u) {
            return;
        }
        if (nav_actor_transition_phase_gs == 1u) {
            ++nav_actor_transition_phase_gs;
            nav_actor_0_busy_gs = 0u;
            nav_camera_view_active_gs = 0u;
            vm_active_line_gs = 0x002cu;
            return;
        }
        if ((vm_c2_presentation_gate_gs & 1u) != 0u) {
            return;
        }

        nav_actor_transition_phase_gs = 0u;
        nav_screen_rebuild_pending_gs = 1u;
        ship_3d_hud_palette_snapshot_and_camera_reset();
        vm_ui_state_gs.bytes.flags &= (cb_u8)~UI_CAMERA_TRANSITION;
        related_offset = record->related;
        record->kind = 0u;
        record->related = 0u;
        record->value = 0u;
        related = RECORD_AT(record_segment, related_offset,
                bloodprg_vm_scan_object);
        related_kind = related->kind;

        field_offset = (cb_u16)vm_field_offset(
                FIELD_SELECTOR_RELATION, owner_kind);
        word_field = RECORD_AT(
                record_segment, owner_offset + field_offset, cb_u16);
        relation = *word_field;
        field_offset = (cb_u16)vm_field_offset(
                FIELD_SELECTOR_POSITION, owner_kind);
        owner_position = RECORD_AT(
                record_segment,
                owner_offset + field_offset,
                ship_3d_position_field);

        field_offset = (cb_u16)vm_field_offset(
                FIELD_SELECTOR_COMPARISON, related_kind);
        if (relation == *RECORD_AT(
                record_segment, related_offset + field_offset, cb_u16)) {
            field_offset = (cb_u16)vm_field_offset(
                    FIELD_SELECTOR_RELATION_MATCH, related_kind);
            relation = *RECORD_AT(
                    record_segment,
                    related_offset + field_offset,
                    cb_u16);
            field_offset = (cb_u16)vm_field_offset(
                    FIELD_SELECTOR_POSITION_B, related_kind);
        } else {
            field_offset = (cb_u16)vm_field_offset(
                    FIELD_SELECTOR_COMPARISON, related_kind);
            relation = *RECORD_AT(
                    record_segment,
                    related_offset + field_offset,
                    cb_u16);
            field_offset = (cb_u16)vm_field_offset(
                    FIELD_SELECTOR_POSITION_A, related_kind);
        }
        related_position = RECORD_AT(
                record_segment,
                related_offset + field_offset,
                ship_3d_position_field);
        owner_position->x = related_position->x;
        owner_position->y = related_position->y;
        *word_field = relation;
        return;

    case RECORD_KIND_C9:
        related_offset = record->related;
        record->kind = 0u;
        record->related = 0u;
        record->value = 0u;
        related = RECORD_AT(record_segment, related_offset,
                bloodprg_vm_scan_object);
        field_offset = (cb_u16)vm_field_offset(
                FIELD_SELECTOR_RECIPROCAL, related->kind);
        reciprocal = RECORD_AT(
                record_segment,
                related_offset + field_offset,
                bloodprg_vm_record_triple);
        if (reciprocal->kind == RECORD_KIND_C4
                && reciprocal->related == owner_offset) {
            reciprocal->kind = 0u;
            reciprocal->related = 0u;
            reciprocal->value = 0u;
        }
        return;

    case RECORD_KIND_CD:
        related_offset = record->related;
        (void)vm_special_slot_remove(related_offset);
        related = RECORD_AT(record_segment, related_offset,
                bloodprg_vm_scan_object);
        related_kind = related->kind;
        field_offset = (cb_u16)vm_field_offset(
                FIELD_SELECTOR_PARENT, related_kind);
        word_field = RECORD_AT(
                record_segment, related_offset + field_offset, cb_u16);
        *word_field = record->value;
        record->kind = vm_cd_replacement_kind_gs;
        record->related = vm_cd_replacement_related_gs;
        record->value = 0u;

        if ((vm_ui_state_gs.word & UI_PRESENTATION_ACTIVE) == 0u
                && (vm_presentation_request_flags_gs
                    & PRESENTATION_REQUEST_DESCRIPT) == 0u
                && related_kind == OBJECT_KIND_DESCRIPT
                && vm_c2_descript_lookup(related->name)) {
            vm_c2_presentation_gate_gs = 0u;
            vm_active_line_gs = 0x002bu;
            vm_presentation_request_flags_gs |=
                    PRESENTATION_REQUEST_DESCRIPT;
        }
        return;
    }
}

#undef RECORD_AT
#undef RECORD_OFFSET
