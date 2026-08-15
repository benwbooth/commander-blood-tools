#include <dos.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_ems.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_resource.h"

#define SND_CLIP_HEADER_BYTES 6u
#define SND_STREAMED_INDEX_MASK 0x3fffu

void CB_SAVE_REGS CB_FAR snd_play_clip(cb_i16 clip_index)
{
    volatile bloodprg_snd_stream_buffer CB_GAME_DATA *buffer;
    volatile bloodprg_snd_stream_buffer CB_GAME_DATA *other_buffer;
    volatile bloodprg_snd_memory_clip CB_GAME_DATA *memory_clip;
    volatile cb_u8 CB_FAR *source;
    volatile cb_u8 CB_FAR *destination;
    volatile cb_u8 CB_FAR *staging;
    cb_u32 clip_start;
    cb_u32 clip_end;
    cb_u32 clip_length;
    cb_u16 streamed_index;
    cb_u16 logical_page;
    cb_u16 source_bytes;
    cb_u16 position;
    cb_u16 available;
    cb_u16 remaining;
    cb_u16 mix_count;
    cb_u8 physical_page;
    cb_u8 sample;
    cb_u8 packed;

    if ((voc_playback_enabled_gs & 1u) == 0) {
        return;
    }

    if ((snd_driver_pending_flag & 2u) == 0) {
        snd_driver_call();

        if (clip_index >= 0) {
            memory_clip = &snd_memory_clips[(cb_u16)clip_index];
            snd_clip_descriptor.data = (volatile cb_u8 CB_FAR *)MK_FP(
                    FP_SEG(snd_bank_memory), memory_clip->offset);
            snd_clip_descriptor.byte_count = memory_clip->byte_count;
        } else {
            streamed_index = (cb_u16)clip_index & SND_STREAMED_INDEX_MASK;
            clip_start = snd_streamed_offsets[streamed_index];
            clip_end = snd_streamed_offsets[streamed_index + 1u];
            clip_length = clip_end - clip_start;

            if (secondary_ems_handle != -1) {
                logical_page = (cb_u16)(clip_start >> 14);
                for (physical_page = 0; physical_page < 4u;
                        ++physical_page) {
                    cb_ems_map_page((cb_u16)secondary_ems_handle,
                            logical_page++, physical_page);
                }
                far_memmove(snd_stream_storage,
                        ems_page_frame +
                            (cb_u16)(clip_start & 0x3fffu),
                        clip_length);
                snd_clip_descriptor.data = snd_stream_storage;
                snd_clip_descriptor.byte_count = (cb_u16)clip_length;
            } else if (secondary_xms_handle != -1) {
                xms_move_request.length = clip_length +
                        ((cb_u8)clip_length & 1u);
                xms_move_request.source_handle =
                        (cb_u16)secondary_xms_handle;
                xms_move_request.source.offset = clip_start;
                xms_move_request.destination_handle = 0;
                xms_move_request.destination.pointer = snd_stream_storage;
                cb_xms_move(&xms_move_request);
                snd_clip_descriptor.data = snd_stream_storage;
                snd_clip_descriptor.byte_count = (cb_u16)clip_length;
            } else {
                cb_dos_seek_absolute(snd_voice_file_handle, clip_start);
                snd_clip_descriptor.data = snd_stream_storage;
                snd_clip_descriptor.byte_count = cb_dos_read(
                        snd_voice_file_handle,
                        snd_stream_storage,
                        (cb_u16)clip_length);
            }
        }

        cb_snd_clip_play(0u, &snd_clip_descriptor);
        return;
    }

    if (clip_index >= 0) {
        memory_clip = &snd_memory_clips[(cb_u16)clip_index];
        source = snd_bank_memory + memory_clip->offset +
                SND_CLIP_HEADER_BYTES;
        source_bytes = memory_clip->byte_count;
    } else {
        streamed_index = (cb_u16)clip_index & SND_STREAMED_INDEX_MASK;
        clip_start = snd_streamed_offsets[streamed_index];
        clip_end = snd_streamed_offsets[streamed_index + 1u];
        clip_length = clip_end - clip_start;

        if (secondary_ems_handle != -1) {
            logical_page = (cb_u16)(clip_start >> 14);
            for (physical_page = 0; physical_page < 4u;
                    ++physical_page) {
                cb_ems_map_page((cb_u16)secondary_ems_handle,
                        logical_page++, physical_page);
            }
            source = ems_page_frame +
                    (cb_u16)(clip_start & 0x3fffu) +
                    SND_CLIP_HEADER_BYTES;
            source_bytes = (cb_u16)clip_length - SND_CLIP_HEADER_BYTES;
        } else {
            if (secondary_xms_handle != -1) {
                staging = graphics_work_surface + 0x7d00u;
                xms_move_request.length = clip_length +
                        ((cb_u8)clip_length & 1u);
                xms_move_request.source_handle =
                        (cb_u16)secondary_xms_handle;
                xms_move_request.source.offset = clip_start;
                xms_move_request.destination_handle = 0;
                xms_move_request.destination.pointer = staging;
                cb_xms_move(&xms_move_request);
                source_bytes = (cb_u16)clip_length -
                        SND_CLIP_HEADER_BYTES;
            } else {
                staging = (volatile cb_u8 CB_FAR *)MK_FP(
                        FP_SEG(graphics_work_surface), 0x7d00u);
                cb_dos_seek_absolute(snd_voice_file_handle, clip_start);
                source_bytes = cb_dos_read(snd_voice_file_handle,
                        staging, (cb_u16)clip_length);
                source_bytes -= SND_CLIP_HEADER_BYTES;
            }
            source = staging + SND_CLIP_HEADER_BYTES;
        }
    }

    packed = snd_stream_header_mode & 1u;
    if (packed != 0) {
        source_bytes = (cb_u16)(source_bytes + source_bytes);
    }

    buffer = &snd_stream_buffers[0];
    other_buffer = &snd_stream_buffers[1];
    if (buffer->state != 3u) {
        buffer = &snd_stream_buffers[1];
        other_buffer = &snd_stream_buffers[0];
        if (buffer->state != 3u) {
            return;
        }
    }

    destination = buffer->data + SND_CLIP_HEADER_BYTES;
    position = audio_position_callback();
    if (position == 0xffffu) {
        return;
    }
    position = (cb_u16)(position - buffer->byte_count);
    if ((cb_i16)position < 0) {
        position = (cb_u16)(0u - position);
    }

    remaining = source_bytes;
    if (position < buffer->byte_count) {
        destination += position;
        available = (cb_u16)(buffer->byte_count - position);
        remaining = (cb_u16)(remaining - available);
        mix_count = (cb_i16)remaining >= 0 ? available : source_bytes;
        mix_count = (cb_u16)(mix_count - 1u);

        if ((cb_i16)mix_count > 0) {
            do {
                sample = *source;
                if (packed != 0) {
                    if ((mix_count & 1u) == 0) {
                        ++source;
                    }
                } else {
                    ++source;
                }
                *destination = (cb_u8)(
                        ((cb_u16)sample + *destination) >> 1);
                ++destination;
                --mix_count;
            } while (mix_count != 0);
        }
    }

    if ((cb_i16)remaining <= 0) {
        return;
    }

    mix_count = remaining;
    if (mix_count > other_buffer->byte_count) {
        mix_count = other_buffer->byte_count;
    }
    destination = other_buffer->data + SND_CLIP_HEADER_BYTES;
    mix_count = (cb_u16)(mix_count - 1u);
    if ((cb_i16)mix_count <= 0) {
        return;
    }

    do {
        sample = *source;
        if (packed != 0) {
            if ((mix_count & 1u) == 0) {
                ++source;
            }
        } else {
            ++source;
        }
        *destination = (cb_u8)(
                ((cb_u16)sample + *destination) >> 1);
        ++destination;
        --mix_count;
    } while (mix_count != 0);
}
