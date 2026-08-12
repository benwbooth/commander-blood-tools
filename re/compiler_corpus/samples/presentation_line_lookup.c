/*
 * Codegen probe for BLOODPRG 0x009F80.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

typedef struct presentation_line_record {
    u16 flags;
} presentation_line_record;

typedef struct presentation_line_index_entry {
    presentation_line_record *record;
    u16 asset_name_offset;
} presentation_line_index_entry;

extern volatile presentation_line_index_entry presentation_line_index[];

#if defined(__WATCOMC__)
#pragma aux presentation_line_lookup_probe parm [ax] value [bx] modify [bx]
#endif

presentation_line_record *NEAR presentation_line_lookup_probe(u16 index)
{
    return presentation_line_index[index].record;
}
