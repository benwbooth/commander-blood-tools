//! Command-tail tokenization and startup-option handling recovered from BLOODPRG.

use commander_blood_formats::code::ScriptDialect;

use super::{STARTUP_AUDIO_NUMBER_LENGTH, parse_startup_audio_number};

const OPTION_PREFIX_LENGTH: usize = 3;
const AUDIO_TRAILING_DIGIT_INDEX: usize = STARTUP_AUDIO_NUMBER_LENGTH;
const AUDIO_CONFIGURATION_SHIFT: u32 = 4;
const ASCII_ZERO: u8 = b'0';
const SHIPPED_STARTUP_OPTION_COUNT: usize = 6;
const COMMAND_DELIMITER_LENGTH: usize = 1;

/// Audio driver selected by the original startup option table.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupAudioDriver {
    /// Gravis-compatible startup driver identifier.
    Gravis = 1,
    /// Sound Blaster-compatible startup driver identifier.
    SoundBlaster = 42,
}

impl StartupAudioDriver {
    /// Return the numeric identifier consumed by the recovered audio initialization.
    pub const fn original_id(self) -> u8 {
        self as u8
    }
}

/// Packed audio selection published by the original startup option handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartupAudioConfiguration {
    /// Selected driver family.
    pub driver: StartupAudioDriver,
    /// Original packed configuration word consumed by the recovered audio setup.
    pub packed_value: u16,
}

/// Host-owned startup settings produced from the DOS command tail.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartupConfiguration {
    /// Optional write directory with the caller-supplied trailing separator removed.
    pub write_directory: Option<Vec<u8>>,
    /// Optional original packed audio configuration.
    pub audio: Option<StartupAudioConfiguration>,
}

#[derive(Clone, Copy)]
struct StartupOption {
    prefix: [u8; OPTION_PREFIX_LENGTH],
    copies_directory: bool,
    configures_audio: bool,
    driver: StartupAudioDriver,
}

const SHIPPED_STARTUP_OPTIONS: [StartupOption; SHIPPED_STARTUP_OPTION_COUNT] = [
    StartupOption::audio(*b"S16", StartupAudioDriver::SoundBlaster),
    StartupOption::none(*b"MID"),
    StartupOption::audio(*b"SDB", StartupAudioDriver::SoundBlaster),
    StartupOption::audio(*b"SBP", StartupAudioDriver::SoundBlaster),
    StartupOption::audio(*b"GRV", StartupAudioDriver::Gravis),
    StartupOption::directory(*b"WRI"),
];

const BIG_BUG_BANG_STARTUP_OPTIONS: [StartupOption; 5] = [
    StartupOption::audio(*b"S16", StartupAudioDriver::SoundBlaster),
    StartupOption::none(*b"MID"),
    StartupOption::audio(*b"SDB", StartupAudioDriver::SoundBlaster),
    StartupOption::audio(*b"SBP", StartupAudioDriver::SoundBlaster),
    StartupOption::directory(*b"WRI"),
];

impl StartupOption {
    const fn none(prefix: [u8; OPTION_PREFIX_LENGTH]) -> Self {
        Self {
            prefix,
            copies_directory: false,
            configures_audio: false,
            driver: StartupAudioDriver::Gravis,
        }
    }

    const fn audio(prefix: [u8; OPTION_PREFIX_LENGTH], driver: StartupAudioDriver) -> Self {
        Self {
            prefix,
            copies_directory: false,
            configures_audio: true,
            driver,
        }
    }

    const fn directory(prefix: [u8; OPTION_PREFIX_LENGTH]) -> Self {
        Self {
            prefix,
            copies_directory: true,
            configures_audio: false,
            driver: StartupAudioDriver::Gravis,
        }
    }
}

/// Split the counted startup command into the exact token sequence used by BLOODPRG.
///
/// This translates `startup_command_line_parse` at file offset `0x0006f1`.
/// Leading and repeated spaces produce empty tokens, while a trailing space does
/// not produce a final empty token.
pub fn tokenize_startup_command(command: &[u8]) -> Vec<Vec<u8>> {
    let mut tokens = Vec::new();
    let mut token_start = usize::MIN;

    for (index, byte) in command.iter().copied().enumerate() {
        if byte == b' ' {
            tokens.push(command[token_start..index].to_vec());
            token_start = index.saturating_add(COMMAND_DELIMITER_LENGTH);
        }
    }
    if token_start < command.len() {
        tokens.push(command[token_start..].to_vec());
    }
    tokens
}

/// Apply one token using Commander Blood's shipped startup option table.
///
/// This translates `startup_option_apply` at BLOODPRG file offset `0x000726`.
/// The result owns paths and typed driver state; the original global buffers and
/// packed table addresses have no runtime representation.
pub fn apply_startup_option(token: &[u8], configuration: &mut StartupConfiguration) {
    apply_startup_option_for_dialect(token, configuration, ScriptDialect::CommanderBlood);
}

/// Apply one token using the selected executable's shipped startup option table.
pub fn apply_startup_option_for_dialect(
    token: &[u8],
    configuration: &mut StartupConfiguration,
    dialect: ScriptDialect,
) {
    let options = match dialect {
        ScriptDialect::CommanderBlood => &SHIPPED_STARTUP_OPTIONS[..],
        ScriptDialect::BigBugBang => &BIG_BUG_BANG_STARTUP_OPTIONS[..],
    };
    apply_startup_option_from(options, token, configuration, dialect);
}

fn apply_startup_option_from(
    options: &[StartupOption],
    token: &[u8],
    configuration: &mut StartupConfiguration,
    dialect: ScriptDialect,
) {
    let Some(option) = options
        .iter()
        .find(|option| token.starts_with(&option.prefix))
    else {
        return;
    };
    let suffix = &token[OPTION_PREFIX_LENGTH..];

    if option.copies_directory {
        if let Some((_separator, path)) = suffix.split_last() {
            configuration.write_directory = Some(path.to_vec());
        }
    } else if option.configures_audio {
        let mut number = [u8::MIN; STARTUP_AUDIO_NUMBER_LENGTH];
        for (destination, source) in number.iter_mut().zip(suffix.iter().copied()) {
            *destination = source;
        }
        let mut trailing_digit = suffix
            .get(AUDIO_TRAILING_DIGIT_INDEX)
            .copied()
            .unwrap_or(u8::MIN)
            .wrapping_sub(ASCII_ZERO);
        if dialect == ScriptDialect::BigBugBang && trailing_digit > 9 {
            trailing_digit = trailing_digit.wrapping_sub(7);
        }
        let parsed = parse_startup_audio_number(&number) as u16;
        let packed_value =
            parsed.wrapping_shl(AUDIO_CONFIGURATION_SHIFT) | u16::from(trailing_digit);
        configuration.audio = Some(StartupAudioConfiguration {
            driver: option.driver,
            packed_value,
        });
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const COMMAND_ORACLE_VECTOR_COUNT: usize = 5;
    const OPTION_ORACLE_VECTOR_COUNT: usize = 10;

    #[derive(Deserialize)]
    struct CommandOracleVector {
        command: String,
        calls: Vec<CommandOracleCall>,
    }

    #[derive(Deserialize)]
    struct CommandOracleCall {
        token: String,
    }

    #[derive(Deserialize)]
    struct OptionOracleVector {
        name: String,
        token_before: String,
        driver_id: u8,
        configuration: u16,
    }

    #[derive(Deserialize)]
    struct BigBugBangStartupOracle {
        format: String,
        executable_sha256: String,
        routines: Vec<BigBugBangRoutine>,
        startup_options: Vec<BigBugBangOption>,
        command_cases: Vec<CommandOracleVector>,
        option_cases: Vec<BigBugBangOptionCase>,
    }

    #[derive(Deserialize)]
    struct BigBugBangRoutine {
        entry: String,
    }

    #[derive(Deserialize)]
    struct BigBugBangOption {
        prefix: String,
        flags: u8,
        driver_id: u8,
    }

    #[derive(Deserialize)]
    struct BigBugBangOptionCase {
        name: String,
        token_before: String,
        owned_write_directory: String,
        raw_directory_underflow_write: bool,
        driver_id: u8,
        configuration: u16,
    }

    #[test]
    fn tokenizer_matches_every_original_command_tail_vector() {
        let vectors: Vec<CommandOracleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_06f1_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), COMMAND_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let expected = vector
                .calls
                .iter()
                .map(|call| call.token.as_bytes().to_vec())
                .collect::<Vec<_>>();
            assert_eq!(
                tokenize_startup_command(vector.command.as_bytes()),
                expected
            );
        }
    }

    #[test]
    fn option_dispatch_matches_every_original_semantic_vector() {
        let vectors: Vec<OptionOracleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_0726_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), OPTION_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let initial = StartupConfiguration {
                write_directory: Some(b"cccccccccccccccccccc".to_vec()),
                audio: Some(StartupAudioConfiguration {
                    driver: StartupAudioDriver::Gravis,
                    packed_value: 4_660,
                }),
            };
            let mut actual = initial.clone();
            let custom_options;
            let options = match vector.name.as_str() {
                "empty_table" => &[][..],
                "copy_precedes_audio" => {
                    custom_options = vec![StartupOption {
                        prefix: *b"WRI",
                        copies_directory: true,
                        configures_audio: true,
                        driver: StartupAudioDriver::SoundBlaster,
                    }];
                    &custom_options
                }
                "first_match_wins" => {
                    custom_options = vec![
                        StartupOption::none(*b"S16"),
                        StartupOption::audio(*b"S16", StartupAudioDriver::SoundBlaster),
                    ];
                    &custom_options
                }
                _ => &SHIPPED_STARTUP_OPTIONS,
            };
            apply_startup_option_from(
                options,
                vector.token_before.as_bytes(),
                &mut actual,
                ScriptDialect::CommanderBlood,
            );

            match vector.name.as_str() {
                "write_directory" => {
                    assert_eq!(actual.write_directory.as_deref(), Some(&b"C:\\cblood"[..]));
                    assert_eq!(actual.audio, initial.audio);
                }
                "copy_precedes_audio" => {
                    assert_eq!(actual.write_directory.as_deref(), Some(&b""[..]));
                    assert_eq!(actual.audio, initial.audio);
                }
                "sb16_audio" | "gravis_audio" | "signed_wrapping_audio" => {
                    let audio = actual.audio.unwrap();
                    assert_eq!(audio.driver.original_id(), vector.driver_id);
                    assert_eq!(audio.packed_value, vector.configuration);
                    assert_eq!(actual.write_directory, initial.write_directory);
                }
                _ => assert_eq!(actual, initial),
            }
        }
    }

    #[test]
    fn sequel_startup_handlers_match_original_executable_vectors() {
        const EXECUTABLE_SHA256: &str =
            "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834";
        let fixture: BigBugBangStartupOracle = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/big_bug_bang_startup_command.json"
        ))
        .unwrap();
        assert_eq!(fixture.format, "big_bug_bang_startup_command_v1");
        assert_eq!(fixture.executable_sha256, EXECUTABLE_SHA256);
        assert_eq!(
            fixture
                .routines
                .iter()
                .map(|routine| routine.entry.as_str())
                .collect::<Vec<_>>(),
            ["0x08ef", "0x0924", "0x2992"]
        );
        assert_eq!(
            fixture
                .startup_options
                .iter()
                .map(|option| (option.prefix.as_str(), option.flags, option.driver_id))
                .collect::<Vec<_>>(),
            [
                ("S16", 2, 42),
                ("MID", 0, 1),
                ("SDB", 2, 42),
                ("SBP", 2, 42),
                ("WRI", 1, 0),
            ]
        );

        for vector in fixture.command_cases {
            let expected = vector
                .calls
                .iter()
                .map(|call| call.token.as_bytes().to_vec())
                .collect::<Vec<_>>();
            assert_eq!(
                tokenize_startup_command(vector.command.as_bytes()),
                expected
            );
        }

        for vector in fixture.option_cases {
            let initial = StartupConfiguration {
                write_directory: Some(vec![b'c'; 32]),
                audio: Some(StartupAudioConfiguration {
                    driver: StartupAudioDriver::Gravis,
                    packed_value: 4_660,
                }),
            };
            let mut actual = initial.clone();
            apply_startup_option_for_dialect(
                vector.token_before.as_bytes(),
                &mut actual,
                ScriptDialect::BigBugBang,
            );
            assert_eq!(
                actual.write_directory.as_deref(),
                Some(vector.owned_write_directory.as_bytes()),
                "{}",
                vector.name
            );
            if vector.configuration == 4_660 {
                assert_eq!(vector.driver_id, 0x69, "{}", vector.name);
                assert_eq!(actual.audio, initial.audio, "{}", vector.name);
            } else {
                assert_eq!(
                    actual.audio,
                    Some(StartupAudioConfiguration {
                        driver: StartupAudioDriver::SoundBlaster,
                        packed_value: vector.configuration,
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(vector.driver_id, 42, "{}", vector.name);
            }
            assert_eq!(
                vector.raw_directory_underflow_write,
                vector.name == "empty_write_directory"
            );
        }
    }
}
