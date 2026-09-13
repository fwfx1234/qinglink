//! Shared data types for custom infrared remotes.
//!
//! The protocol carries the already decoded carrier/pulse representation.  A
//! platform is responsible for checking whether the requested carrier is
//! supported by its IR emitter before transmitting it.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Maximum number of on/off durations accepted for one command.
pub const IR_MAX_PATTERN_ITEMS: usize = 4096;
/// Maximum duration of one command, in microseconds.
pub const IR_MAX_PATTERN_DURATION_US: u64 = 2_000_000;
/// Maximum number of times a command can be repeated.
pub const IR_MAX_REPEAT_COUNT: u8 = 5;
/// Maximum number of steps in a macro.
pub const IR_MAX_MACRO_STEPS: usize = 32;
/// Maximum delay after a macro step, in milliseconds.
pub const IR_MAX_STEP_DELAY_MS: u32 = 5_000;
/// Default delay after a macro step, in milliseconds.
pub const IR_DEFAULT_STEP_DELAY_MS: u32 = 300;

fn default_repeat_count() -> u8 {
    1
}

fn default_step_delay_ms() -> u32 {
    IR_DEFAULT_STEP_DELAY_MS
}

/// A raw infrared command.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct IrCommand {
    pub id: String,
    pub name: String,
    /// Carrier frequency in hertz (for example, 38_000).
    pub carrier_hz: u32,
    /// Alternating mark/space durations in microseconds, beginning with a mark.
    pub pattern_us: Vec<u32>,
    #[serde(default = "default_repeat_count")]
    pub repeat_count: u8,
}

impl IrCommand {
    /// Validate command data independent of a particular phone's capabilities.
    pub fn validate(&self) -> Result<(), IrValidationError> {
        if self.id.trim().is_empty() {
            return Err(IrValidationError::EmptyId);
        }
        if self.name.trim().is_empty() {
            return Err(IrValidationError::EmptyName);
        }
        if self.carrier_hz == 0 {
            return Err(IrValidationError::InvalidCarrier);
        }
        if self.pattern_us.is_empty() {
            return Err(IrValidationError::EmptyPattern);
        }
        if self.pattern_us.len() % 2 != 0 {
            return Err(IrValidationError::OddPatternLength {
                len: self.pattern_us.len(),
            });
        }
        if self.pattern_us.len() > IR_MAX_PATTERN_ITEMS {
            return Err(IrValidationError::PatternTooLong {
                len: self.pattern_us.len(),
            });
        }
        if self.pattern_us.iter().any(|&duration| duration == 0) {
            return Err(IrValidationError::NonPositiveDuration);
        }
        let total_duration_us: u64 = self
            .pattern_us
            .iter()
            .map(|&duration| u64::from(duration))
            .sum();
        if total_duration_us > IR_MAX_PATTERN_DURATION_US {
            return Err(IrValidationError::PatternDurationTooLong {
                duration_us: total_duration_us,
            });
        }
        if !(1..=IR_MAX_REPEAT_COUNT).contains(&self.repeat_count) {
            return Err(IrValidationError::InvalidRepeatCount {
                repeat_count: self.repeat_count,
            });
        }
        Ok(())
    }

    /// Check that the carrier falls in one of the ranges reported by
    /// `ConsumerIrManager#getCarrierFrequencies`.
    pub fn validate_carrier_ranges(
        &self,
        supported_ranges_hz: &[(u32, u32)],
    ) -> Result<(), IrValidationError> {
        self.validate()?;
        if supported_ranges_hz
            .iter()
            .any(|&(min, max)| min <= self.carrier_hz && self.carrier_hz <= max)
        {
            Ok(())
        } else {
            Err(IrValidationError::UnsupportedCarrier {
                carrier_hz: self.carrier_hz,
            })
        }
    }
}

/// A named collection of infrared commands.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct IrRemoteProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub commands: Vec<IrCommand>,
}

impl IrRemoteProfile {
    /// Validate the profile and all commands it contains.
    pub fn validate(&self) -> Result<(), IrValidationError> {
        if self.id.trim().is_empty() {
            return Err(IrValidationError::EmptyId);
        }
        if self.name.trim().is_empty() {
            return Err(IrValidationError::EmptyName);
        }
        for command in &self.commands {
            command.validate()?;
        }
        for (index, command) in self.commands.iter().enumerate() {
            if self.commands[..index]
                .iter()
                .any(|previous| previous.id == command.id)
            {
                return Err(IrValidationError::DuplicateCommandId {
                    id: command.id.clone(),
                });
            }
        }
        Ok(())
    }
}

/// One ordered step in an infrared macro.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct IrMacroStep {
    pub command_id: String,
    #[serde(default = "default_step_delay_ms")]
    pub delay_after_ms: u32,
}

/// An ordered sequence of commands.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct IrMacro {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub steps: Vec<IrMacroStep>,
}

impl IrMacro {
    /// Validate the macro and ensure every referenced command exists.
    pub fn validate_against(&self, profile: &IrRemoteProfile) -> Result<(), IrValidationError> {
        if self.id.trim().is_empty() {
            return Err(IrValidationError::EmptyId);
        }
        if self.name.trim().is_empty() {
            return Err(IrValidationError::EmptyName);
        }
        if self.steps.len() > IR_MAX_MACRO_STEPS {
            return Err(IrValidationError::MacroTooLong {
                len: self.steps.len(),
            });
        }
        for (index, step) in self.steps.iter().enumerate() {
            if step.command_id.trim().is_empty() {
                return Err(IrValidationError::EmptyCommandReference { index });
            }
            if !profile
                .commands
                .iter()
                .any(|command| command.id == step.command_id)
            {
                return Err(IrValidationError::UnknownCommandReference {
                    command_id: step.command_id.clone(),
                    index,
                });
            }
            if step.delay_after_ms > IR_MAX_STEP_DELAY_MS {
                return Err(IrValidationError::StepDelayTooLong {
                    delay_ms: step.delay_after_ms,
                    index,
                });
            }
        }
        Ok(())
    }
}

/// Validation failures for custom infrared data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrValidationError {
    EmptyId,
    EmptyName,
    InvalidCarrier,
    UnsupportedCarrier { carrier_hz: u32 },
    EmptyPattern,
    OddPatternLength { len: usize },
    PatternTooLong { len: usize },
    NonPositiveDuration,
    PatternDurationTooLong { duration_us: u64 },
    InvalidRepeatCount { repeat_count: u8 },
    DuplicateCommandId { id: String },
    MacroTooLong { len: usize },
    EmptyCommandReference { index: usize },
    UnknownCommandReference { command_id: String, index: usize },
    StepDelayTooLong { delay_ms: u32, index: usize },
}

impl fmt::Display for IrValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => f.write_str("id must not be empty"),
            Self::EmptyName => f.write_str("name must not be empty"),
            Self::InvalidCarrier => f.write_str("carrier frequency must be positive"),
            Self::UnsupportedCarrier { carrier_hz } => {
                write!(f, "carrier frequency {carrier_hz} Hz is unsupported")
            }
            Self::EmptyPattern => f.write_str("pattern must not be empty"),
            Self::OddPatternLength { len } => write!(f, "pattern length {len} must be even"),
            Self::PatternTooLong { len } => {
                write!(f, "pattern length {len} exceeds {IR_MAX_PATTERN_ITEMS}")
            }
            Self::NonPositiveDuration => f.write_str("pattern durations must be positive"),
            Self::PatternDurationTooLong { duration_us } => write!(
                f,
                "pattern duration {duration_us} us exceeds {IR_MAX_PATTERN_DURATION_US} us"
            ),
            Self::InvalidRepeatCount { repeat_count } => {
                write!(
                    f,
                    "repeat count {repeat_count} must be between 1 and {IR_MAX_REPEAT_COUNT}"
                )
            }
            Self::DuplicateCommandId { id } => write!(f, "duplicate command id {id}"),
            Self::MacroTooLong { len } => {
                write!(f, "macro length {len} exceeds {IR_MAX_MACRO_STEPS}")
            }
            Self::EmptyCommandReference { index } => {
                write!(f, "macro step {index} has an empty command reference")
            }
            Self::UnknownCommandReference { command_id, index } => {
                write!(
                    f,
                    "macro step {index} references unknown command {command_id}"
                )
            }
            Self::StepDelayTooLong { delay_ms, index } => write!(
                f,
                "macro step {index} delay {delay_ms} ms exceeds {IR_MAX_STEP_DELAY_MS} ms"
            ),
        }
    }
}

impl std::error::Error for IrValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn command() -> IrCommand {
        IrCommand {
            id: "power".into(),
            name: "Power".into(),
            carrier_hz: 38_000,
            pattern_us: vec![9000, 4500, 560, 560],
            repeat_count: 1,
        }
    }

    #[test]
    fn command_round_trips_with_snake_case_wire_fields() {
        let value = serde_json::to_value(command()).unwrap();
        assert_eq!(value["carrier_hz"], 38_000);
        assert_eq!(value["pattern_us"][0], 9000);
        let decoded: IrCommand = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, command());
    }

    #[test]
    fn command_defaults_repeat_count() {
        let decoded: IrCommand =
            serde_json::from_str(r#"{"id":"x","name":"X","carrier_hz":38000,"pattern_us":[1,1]}"#)
                .unwrap();
        assert_eq!(decoded.repeat_count, 1);
    }

    #[test]
    fn command_validation_rejects_invalid_patterns() {
        let mut invalid = command();
        invalid.pattern_us = vec![1, 2, 3];
        assert!(matches!(
            invalid.validate(),
            Err(IrValidationError::OddPatternLength { .. })
        ));
        invalid.pattern_us = vec![0, 2];
        assert_eq!(
            invalid.validate(),
            Err(IrValidationError::NonPositiveDuration)
        );
        invalid.pattern_us = vec![1, 2];
        invalid.repeat_count = 6;
        assert!(matches!(
            invalid.validate(),
            Err(IrValidationError::InvalidRepeatCount { .. })
        ));
    }

    #[test]
    fn carrier_validation_uses_reported_ranges() {
        let command = command();
        assert!(command.validate_carrier_ranges(&[(30_000, 40_000)]).is_ok());
        assert!(matches!(
            command.validate_carrier_ranges(&[(36_000, 37_000)]),
            Err(IrValidationError::UnsupportedCarrier { carrier_hz: 38_000 })
        ));
    }

    #[test]
    fn macro_validation_rejects_missing_commands_and_large_delays() {
        let profile = IrRemoteProfile {
            id: "tv".into(),
            name: "TV".into(),
            commands: vec![command()],
        };
        let missing = IrMacro {
            id: "m".into(),
            name: "Macro".into(),
            steps: vec![IrMacroStep {
                command_id: "missing".into(),
                delay_after_ms: 0,
            }],
        };
        assert!(matches!(
            missing.validate_against(&profile),
            Err(IrValidationError::UnknownCommandReference { .. })
        ));
        let too_long = IrMacro {
            id: "m".into(),
            name: "Macro".into(),
            steps: vec![IrMacroStep {
                command_id: "power".into(),
                delay_after_ms: IR_MAX_STEP_DELAY_MS + 1,
            }],
        };
        assert!(matches!(
            too_long.validate_against(&profile),
            Err(IrValidationError::StepDelayTooLong { .. })
        ));
    }

    #[test]
    fn macro_step_defaults_delay() {
        let step: IrMacroStep = serde_json::from_str(r#"{"command_id":"power"}"#).unwrap();
        assert_eq!(step.delay_after_ms, IR_DEFAULT_STEP_DELAY_MS);
    }
}
