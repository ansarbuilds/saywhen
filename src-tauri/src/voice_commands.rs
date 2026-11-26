//! Whole-utterance commands that change saywhen instead of being typed.
//! They only match when the cleaned text is the command and nothing else, so
//! "git status and pause listening" is still dictated.

use crate::turn_taking::{SendMode, VisionPowerMode};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceCommand {
    PauseListening,
    ResumeListening,
    SetSendMode(SendMode),
    SetVisionPower(VisionPowerMode),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceCommandEvent {
    pub message: String,
}

pub fn parse_voice_command(text: &str) -> Option<VoiceCommand> {
    let normalized = normalize(text);
    match normalized.as_str() {
        "pause listening" | "stop listening" | "saywhen pause" => {
            Some(VoiceCommand::PauseListening)
        }
        "resume listening" | "start listening" | "saywhen resume" => {
            Some(VoiceCommand::ResumeListening)
        }
        "pause mode" | "switch to pause" => Some(VoiceCommand::SetSendMode(SendMode::Pause)),
        "nod mode" | "switch to nod" => Some(VoiceCommand::SetSendMode(SendMode::Nod)),
        "phrase mode" | "switch to phrase" => Some(VoiceCommand::SetSendMode(SendMode::Phrase)),
        "type only mode" | "paste only mode" | "switch to type only" => {
            Some(VoiceCommand::SetSendMode(SendMode::PasteOnly))
        }
        "low power mode" | "battery saver on" => {
            Some(VoiceCommand::SetVisionPower(VisionPowerMode::Low))
        }
        "full power mode" | "battery saver off" => {
            Some(VoiceCommand::SetVisionPower(VisionPowerMode::Full))
        }
        "auto power mode" => Some(VoiceCommand::SetVisionPower(VisionPowerMode::Auto)),
        _ => None,
    }
}

pub fn describe_voice_command(command: VoiceCommand) -> String {
    match command {
        VoiceCommand::PauseListening => "Hands-free listening is paused.".to_string(),
        VoiceCommand::ResumeListening => "Hands-free listening is on.".to_string(),
        VoiceCommand::SetSendMode(SendMode::Pause) => "Send style is now Just pause.".to_string(),
        VoiceCommand::SetSendMode(SendMode::Nod) => "Send style is now Nod to send.".to_string(),
        VoiceCommand::SetSendMode(SendMode::Phrase) => "Send style is now Say send it.".to_string(),
        VoiceCommand::SetSendMode(SendMode::PasteOnly) => {
            "Send style is now Type only.".to_string()
        }
        VoiceCommand::SetVisionPower(VisionPowerMode::Low) => {
            "Camera power is now Low.".to_string()
        }
        VoiceCommand::SetVisionPower(VisionPowerMode::Full) => {
            "Camera power is now Full.".to_string()
        }
        VoiceCommand::SetVisionPower(VisionPowerMode::Auto) => {
            "Camera power is now Auto.".to_string()
        }
    }
}

fn normalize(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch.is_whitespace() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_only_a_whole_command() {
        assert_eq!(
            parse_voice_command("Pause listening."),
            Some(VoiceCommand::PauseListening)
        );
        assert_eq!(
            parse_voice_command("switch to type only"),
            Some(VoiceCommand::SetSendMode(SendMode::PasteOnly))
        );
        assert_eq!(
            parse_voice_command("low power mode"),
            Some(VoiceCommand::SetVisionPower(VisionPowerMode::Low))
        );
        assert_eq!(
            parse_voice_command("git status and then pause listening"),
            None
        );
        assert_eq!(parse_voice_command("please pause listening now"), None);
    }
}
