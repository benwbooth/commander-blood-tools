/*
 * Codegen probe for XDB alien method-table slot 13.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

typedef struct alien_method_context alien_method_context;
typedef void NEAR alien_resume_function(
        alien_method_context NEAR *context);
typedef alien_resume_function NEAR *alien_resume_callback;

struct alien_method_context {
    u8 field_000[0x36];
    alien_resume_callback resume;
    u16 resume_step;
    u16 resume_value;
};

extern alien_resume_function initial_resume;

void NEAR xdb_resume_or_init_probe(
        alien_method_context NEAR *context);

#if defined(__WATCOMC__)
#pragma aux alien_resume_function parm [di]
#pragma aux xdb_resume_or_init_probe parm [di]
#endif

void NEAR xdb_resume_or_init_probe(
        alien_method_context NEAR *context)
{
    alien_resume_callback resume;

    resume = context->resume;
    if (resume != 0) {
        resume(context);
        return;
    }

    context->resume = initial_resume;
    context->resume_step = 0;
    context->resume_value = 0;
}
