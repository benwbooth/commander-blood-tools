//! Commander Blood — reimplementation and reverse-engineering tools.
//!
//! Two things live side by side here, and they follow **opposite** conventions:
//!
//! * [`recomp`] is the bit-exact x86 *emulator* (the verification reference). It legitimately
//!   models registers, flags, and real-mode `segment:offset` memory, because its whole job is to
//!   reproduce the original instruction stream.
//! * **Every other module is the hand-written *port*.** The port is clean-room game code derived
//!   from the reverse-engineering notes (`re/REVERSE.md`, `re/labels.csv`), not transliterated
//!   from x86. Port code obeys these house rules:
//!     1. **Flat memory** — ordinary typed values and flat-indexed slices; never `segment:offset`,
//!        `seg * 16 + off`, or a shared byte pool standing in for RAM. It must not depend on
//!        [`recomp`]'s `Machine`.
//!     2. **No register names** — functions take and return meaningful values; no `ax`/`si`/`es`
//!        identifiers or flag bits. (Short math names like `dx` for a delta or `si` for a source
//!        index are fine when that is genuinely what they mean.)
//!     3. **Named numbers** — non-trivial constants are named `const`s or `enum` variants; bare
//!        literals only for self-evident quantities.
//!     4. **Decimal by default** — hexadecimal only where it is genuinely clearer (bit masks,
//!        packed fields).
//!
//! Behaviour of ported code is validated against the emulator/oracle, not against the instruction
//! stream.

pub mod audio;
pub mod bloodprg;
pub mod croolis;
pub mod decompress;
pub mod descript;
pub mod engine;
pub mod entity;
pub mod ext;
pub mod font;
pub mod hnm;
pub mod recomp;
pub mod lbm;
pub mod levels;
pub mod manu3;
pub mod palette;
pub mod script;
pub mod ship3d;
pub mod snd;
pub mod sprite;
pub mod util;
pub mod vm;

pub const VIEWPORT_W: usize = 320;
pub const VIEWPORT_H: usize = 200;
pub const HNM_FPS: u32 = 15;
