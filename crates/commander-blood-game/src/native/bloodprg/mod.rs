//! Native BLOODPRG game logic translated to flat, typed Rust data.

mod numbers;
mod startup;
mod text;
mod vm;

pub use numbers::{
    append_decimal_i16, append_decimal_i32, packed_bcd_to_binary, parse_startup_audio_number,
    STARTUP_AUDIO_NUMBER_LENGTH,
};
pub use startup::{
    apply_startup_option, tokenize_startup_command, StartupAudioConfiguration, StartupAudioDriver,
    StartupConfiguration,
};
pub use text::{nul_terminated_byte_len, nul_terminated_bytes_equal};
pub use vm::{
    count_positive_operands, object_before_threshold, resolve_dictionary_object,
    script_field_offset, ScriptFieldSelector,
};
