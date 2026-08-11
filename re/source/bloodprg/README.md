# BLOODPRG Natural C Notes

No BLOODPRG routines are accepted as natural C yet.

The removed attempt treated segments as byte buffers and wrapped word loads and
stores. That is the wrong target for this project. The next accepted C source
must look like original program logic: real declarations, real globals, real
arrays/structs, and compiler-shaped calling conventions.

Current high-risk ABI patterns to resolve before writing source:

- AX-only inputs with no stack frame.
- Carry flag status returns.
- FS/GS-backed globals and tables.
- Far returns and mixed near/far calls.
- 32-bit operations inside a 16-bit DOS binary.
