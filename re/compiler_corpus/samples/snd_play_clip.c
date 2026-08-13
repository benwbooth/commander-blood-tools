/*
 * Codegen probe for BLOODPRG 0x00B8CD.
 * This is not recovered game source.
 */
#include <dos.h>

typedef unsigned char u8;
typedef signed char i8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef struct stream_buffer {
    volatile u8 FAR *data;
    u16 byte_count;
    u8 state;
    u8 reserved;
} stream_buffer;

typedef struct memory_clip {
    u16 offset;
    u16 byte_count;
} memory_clip;

typedef struct clip_descriptor {
    volatile u8 FAR *data;
    u16 byte_count;
} clip_descriptor;

typedef union xms_address {
    u32 offset;
    volatile u8 FAR *pointer;
} xms_address;

typedef struct xms_move_request {
    u32 length;
    u16 source_handle;
    xms_address source;
    u16 destination_handle;
    xms_address destination;
} xms_move_request;

typedef u16 (FAR *position_callback)(void);

extern volatile u8 playback_enabled;
extern volatile u8 driver_pending;
extern volatile u8 header_mode;
extern volatile i16 secondary_ems_handle;
extern volatile i16 secondary_xms_handle;
extern volatile u16 voice_file_handle;
extern volatile u8 FAR *bank_memory;
extern volatile u8 FAR *stream_storage;
extern volatile u8 FAR *graphics_work_surface;
extern volatile u8 FAR ems_page_frame[];
extern volatile memory_clip memory_clips[];
extern volatile u32 streamed_offsets[];
extern volatile stream_buffer stream_buffers[2];
extern volatile clip_descriptor shared_clip_descriptor;
extern volatile xms_move_request shared_xms_request;
extern position_callback position_probe;

void FAR driver_stop_probe(void);
void NEAR ems_map_probe(u16 handle, u16 logical_page, u8 physical_page);
void FAR far_memmove_probe(volatile u8 FAR *destination,
        const volatile u8 FAR *source, u32 byte_count);
void NEAR xms_move_probe(volatile xms_move_request *request);
void NEAR dos_seek_probe(u16 handle, u32 offset);
u16 NEAR dos_read_probe(u16 handle,
        volatile u8 FAR *destination, u16 byte_count);
void NEAR driver_play_probe(u16 command, volatile clip_descriptor *clip);

#if defined(__WATCOMC__)
#pragma aux snd_play_clip_probe parm [ax] modify exact []
#pragma aux driver_play_probe parm [ax] [si]
#endif

#define CLIP_HEADER_BYTES 6u
#define STREAMED_INDEX_MASK 0x3fffu

void FAR snd_play_clip_probe(i16 clip_index)
{
    volatile stream_buffer *buffer;
    volatile stream_buffer *other_buffer;
    volatile memory_clip *memory_clip;
    volatile u8 FAR *source;
    volatile u8 FAR *destination;
    volatile u8 FAR *staging;
    u32 clip_start;
    u32 clip_end;
    u32 clip_length;
    u16 streamed_index;
    u16 logical_page;
    u16 source_bytes;
    u16 position;
    u16 available;
    u16 remaining;
    u16 mix_count;
    u8 physical_page;
    u8 sample;
    u8 packed;

    if ((playback_enabled & 1u) == 0) {
        return;
    }

    if ((driver_pending & 2u) == 0) {
        driver_stop_probe();
        if (clip_index >= 0) {
            memory_clip = &memory_clips[(u16)clip_index];
            shared_clip_descriptor.data =
                    bank_memory + memory_clip->offset;
            shared_clip_descriptor.byte_count = memory_clip->byte_count;
        } else {
            streamed_index = (u16)clip_index & STREAMED_INDEX_MASK;
            clip_start = streamed_offsets[streamed_index];
            clip_end = streamed_offsets[streamed_index + 1u];
            clip_length = clip_end - clip_start;

            if (secondary_ems_handle != -1) {
                logical_page = (u16)(clip_start >> 14);
                for (physical_page = 0; physical_page < 4u;
                        ++physical_page) {
                    ems_map_probe((u16)secondary_ems_handle,
                            logical_page++, physical_page);
                }
                far_memmove_probe(stream_storage,
                        ems_page_frame + (u16)(clip_start & 0x3fffu),
                        clip_length);
                shared_clip_descriptor.data = stream_storage;
                shared_clip_descriptor.byte_count = (u16)clip_length;
            } else if (secondary_xms_handle != -1) {
                shared_xms_request.length = clip_length +
                        ((u8)clip_length & 1u);
                shared_xms_request.source_handle =
                        (u16)secondary_xms_handle;
                shared_xms_request.source.offset = clip_start;
                shared_xms_request.destination_handle = 0;
                shared_xms_request.destination.pointer = stream_storage;
                xms_move_probe(&shared_xms_request);
                shared_clip_descriptor.data = stream_storage;
                shared_clip_descriptor.byte_count = (u16)clip_length;
            } else {
                dos_seek_probe(voice_file_handle, clip_start);
                shared_clip_descriptor.data = stream_storage;
                shared_clip_descriptor.byte_count = dos_read_probe(
                        voice_file_handle, stream_storage, (u16)clip_length);
            }
        }
        driver_play_probe(0u, &shared_clip_descriptor);
        return;
    }

    if (clip_index >= 0) {
        memory_clip = &memory_clips[(u16)clip_index];
        source = bank_memory + memory_clip->offset + CLIP_HEADER_BYTES;
        source_bytes = memory_clip->byte_count;
    } else {
        streamed_index = (u16)clip_index & STREAMED_INDEX_MASK;
        clip_start = streamed_offsets[streamed_index];
        clip_end = streamed_offsets[streamed_index + 1u];
        clip_length = clip_end - clip_start;

        if (secondary_ems_handle != -1) {
            logical_page = (u16)(clip_start >> 14);
            for (physical_page = 0; physical_page < 4u;
                    ++physical_page) {
                ems_map_probe((u16)secondary_ems_handle,
                        logical_page++, physical_page);
            }
            source = ems_page_frame + (u16)(clip_start & 0x3fffu) +
                    CLIP_HEADER_BYTES;
            source_bytes = (u16)clip_length - CLIP_HEADER_BYTES;
        } else {
            if (secondary_xms_handle != -1) {
                staging = graphics_work_surface + 0x7d00u;
                shared_xms_request.length = clip_length +
                        ((u8)clip_length & 1u);
                shared_xms_request.source_handle =
                        (u16)secondary_xms_handle;
                shared_xms_request.source.offset = clip_start;
                shared_xms_request.destination_handle = 0;
                shared_xms_request.destination.pointer = staging;
                xms_move_probe(&shared_xms_request);
                source_bytes = (u16)clip_length - CLIP_HEADER_BYTES;
            } else {
                staging = (volatile u8 FAR *)MK_FP(
                        FP_SEG(graphics_work_surface), 0x7d00u);
                dos_seek_probe(voice_file_handle, clip_start);
                source_bytes = dos_read_probe(voice_file_handle,
                        staging, (u16)clip_length);
                source_bytes -= CLIP_HEADER_BYTES;
            }
            source = staging + CLIP_HEADER_BYTES;
        }
    }

    packed = header_mode & 1u;
    if (packed != 0) {
        source_bytes = (u16)(source_bytes + source_bytes);
    }

    buffer = &stream_buffers[0];
    other_buffer = &stream_buffers[1];
    if (buffer->state != 3u) {
        buffer = &stream_buffers[1];
        other_buffer = &stream_buffers[0];
        if (buffer->state != 3u) {
            return;
        }
    }

    destination = buffer->data + CLIP_HEADER_BYTES;
    position = position_probe();
    if (position == 0xffffu) {
        return;
    }
    position = (u16)(position - buffer->byte_count);
    if ((i16)position < 0) {
        position = (u16)(0u - position);
    }

    remaining = source_bytes;
    if (position < buffer->byte_count) {
        destination += position;
        available = (u16)(buffer->byte_count - position);
        remaining = (u16)(remaining - available);
        mix_count = (i16)remaining >= 0 ? available : source_bytes;
        mix_count = (u16)(mix_count - 1u);
        if ((i16)mix_count > 0) {
            do {
                sample = *source;
                if (packed != 0) {
                    if ((mix_count & 1u) == 0) {
                        ++source;
                    }
                } else {
                    ++source;
                }
                *destination = (u8)(((u16)sample + *destination) >> 1);
                ++destination;
                --mix_count;
            } while (mix_count != 0);
        }
    }

    if ((i16)remaining <= 0) {
        return;
    }
    mix_count = remaining;
    if (mix_count > other_buffer->byte_count) {
        mix_count = other_buffer->byte_count;
    }
    destination = other_buffer->data + CLIP_HEADER_BYTES;
    mix_count = (u16)(mix_count - 1u);
    if ((i16)mix_count <= 0) {
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
        *destination = (u8)(((u16)sample + *destination) >> 1);
        ++destination;
        --mix_count;
    } while (mix_count != 0);
}
