use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use indexmap::IndexMap;
use regex::Regex;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PadButton {
    A,
    B,
    X,
    Y,
    Lb,
    Rb,
    Lt,
    Rt,
    Back,
    Start,
    L3,
    R3,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}

impl PadButton {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_uppercase().as_str() {
            "A" => Some(Self::A),
            "B" => Some(Self::B),
            "X" => Some(Self::X),
            "Y" => Some(Self::Y),
            "LB" | "L" | "LEFT_BUMPER" => Some(Self::Lb),
            "RB" | "R" | "RIGHT_BUMPER" => Some(Self::Rb),
            "LT" | "LEFT_TRIGGER" => Some(Self::Lt),
            "RT" | "RIGHT_TRIGGER" => Some(Self::Rt),
            "BACK" | "SELECT" => Some(Self::Back),
            "START" => Some(Self::Start),
            "L3" | "LEFT_THUMB" => Some(Self::L3),
            "R3" | "RIGHT_THUMB" => Some(Self::R3),
            "DPAD_UP" | "DPADUP" | "UP" => Some(Self::DpadUp),
            "DPAD_DOWN" | "DPADDOWN" | "DOWN" => Some(Self::DpadDown),
            "DPAD_LEFT" | "DPADLEFT" | "LEFT" => Some(Self::DpadLeft),
            "DPAD_RIGHT" | "DPADRIGHT" | "RIGHT" => Some(Self::DpadRight),
            _ => None,
        }
    }
}

const PAD_BUTTON_HELP: &str = "supported buttons: A, B, X, Y, LB, RB, LT, RT, Back, Start, L3, R3, DpadUp, DpadDown, DpadLeft, DpadRight";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    Tap,
    Hold,
}

#[derive(Clone, Debug)]
pub struct CompiledConfig {
    pub controller_player: u32,
    pub nintendo_layout: bool,
    pub profiles: Vec<CompiledProfile>,
}

#[derive(Clone, Debug)]
pub struct CompiledProfile {
    pub name: String,
    pub title_regex: String,
    pub regex: Regex,
    pub mappings: Vec<CompiledMapping>,
}

#[derive(Clone, Debug)]
pub struct CompiledMapping {
    pub id: usize,
    pub buttons: BTreeSet<PadButton>,
    pub mode: OutputMode,
    pub keys: Vec<u16>,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    schema_version: Option<u32>,
    controller_player: Option<u32>,
    nintendo_layout: Option<bool>,
    profiles: IndexMap<String, RawProfile>,
}

#[derive(Debug, Deserialize)]
struct RawProfile {
    name: Option<String>,
    title_regex: String,
    map: IndexMap<String, RawAction>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawAction {
    Simple(String),
    Detailed(RawActionDetail),
}

#[derive(Debug, Deserialize)]
struct RawActionDetail {
    mode: Option<OutputMode>,
    send: String,
}

pub fn load_config(path: &Path) -> Result<CompiledConfig> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read config: {}", path.display()))?;
    parse_config(&source)
}

pub fn parse_config(source: &str) -> Result<CompiledConfig> {
    let raw: RawConfig =
        toml::from_str(source).map_err(|err| anyhow!("invalid config.toml: {err}"))?;

    if raw.schema_version.unwrap_or(1) != 1 {
        bail!("schema_version must be 1");
    }

    let controller_player = raw.controller_player.unwrap_or(1);
    if !(1..=4).contains(&controller_player) {
        bail!("controller_player must be in 1..=4");
    }

    if raw.profiles.is_empty() {
        bail!("config.toml must include at least one profile");
    }

    let mut next_mapping_id = 1usize;
    let mut profiles = Vec::with_capacity(raw.profiles.len());
    for (profile_key, raw_profile) in raw.profiles {
        if raw_profile.map.is_empty() {
            bail!("profile '{profile_key}' has no mappings");
        }

        let regex = Regex::new(&raw_profile.title_regex)
            .with_context(|| format!("invalid title_regex for profile '{profile_key}'"))?;
        let mut mappings = Vec::with_capacity(raw_profile.map.len());

        for (when, action) in raw_profile.map {
            let mut buttons = BTreeSet::new();
            for button_name in split_chord(&when) {
                let button = PadButton::from_name(button_name).ok_or_else(|| {
                    anyhow!(
                        "profiles.{profile_key}.map.{when}: unknown pad button '{button_name}' ({PAD_BUTTON_HELP})"
                    )
                })?;
                buttons.insert(button);
            }
            if buttons.is_empty() {
                bail!("profiles.{profile_key}.map.{when}: empty controller input");
            }

            let (mode, send) = match action {
                RawAction::Simple(send) => (OutputMode::Tap, send),
                RawAction::Detailed(detail) => {
                    (detail.mode.unwrap_or(OutputMode::Tap), detail.send)
                }
            };

            let send_parts = split_chord(&send);
            if send_parts.is_empty() {
                bail!("profiles.{profile_key}.map.{when}: empty send chord");
            }

            let mut keys = Vec::with_capacity(send_parts.len());
            for key_name in send_parts {
                keys.push(
                    parse_key_name(key_name)
                        .map_err(|err| anyhow!("profiles.{profile_key}.map.{when}: {err}"))?,
                );
            }

            mappings.push(CompiledMapping {
                id: next_mapping_id,
                buttons,
                mode,
                keys,
            });
            next_mapping_id += 1;
        }

        profiles.push(CompiledProfile {
            name: raw_profile
                .name
                .unwrap_or_else(|| pretty_profile_name(&profile_key)),
            title_regex: raw_profile.title_regex,
            regex,
            mappings,
        });
    }

    Ok(CompiledConfig {
        controller_player,
        nintendo_layout: raw.nintendo_layout.unwrap_or(false),
        profiles,
    })
}

fn split_chord(value: &str) -> Vec<&str> {
    value
        .split('+')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect()
}

fn pretty_profile_name(key: &str) -> String {
    key.split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = String::new();
                    word.extend(first.to_uppercase());
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_key_name(name: &str) -> Result<u16> {
    let upper = name.trim().to_ascii_uppercase();
    let code = match upper.as_str() {
        "CTRL" | "CONTROL" => 0x11,
        "ALT" | "MENU" => 0x12,
        "SHIFT" => 0x10,
        "WIN" | "LWIN" => 0x5B,
        "RWIN" => 0x5C,
        "ENTER" | "RETURN" => 0x0D,
        "ESC" | "ESCAPE" => 0x1B,
        "SPACE" => 0x20,
        "TAB" => 0x09,
        "BACKSPACE" => 0x08,
        "DELETE" | "DEL" => 0x2E,
        "INSERT" | "INS" => 0x2D,
        "HOME" => 0x24,
        "END" => 0x23,
        "PAGEUP" | "PGUP" => 0x21,
        "PAGEDOWN" | "PGDN" => 0x22,
        "UP" => 0x26,
        "DOWN" => 0x28,
        "LEFT" => 0x25,
        "RIGHT" => 0x27,
        "CAPSLOCK" => 0x14,
        "COMMA" => 0xBC,
        "[" | "LBRACKET" | "LEFTBRACKET" => 0xDB,
        "]" | "RBRACKET" | "RIGHTBRACKET" => 0xDD,
        "\\" | "BACKSLASH" => 0xDC,
        "-" | "MINUS" => 0xBD,
        "=" | "EQUALS" | "PLUS" => 0xBB,
        "PERIOD" | "DOT" => 0xBE,
        "SLASH" => 0xBF,
        "SEMICOLON" => 0xBA,
        "APOSTROPHE" | "QUOTE" => 0xDE,
        _ => {
            if upper.len() == 1 {
                let byte = upper.as_bytes()[0];
                if byte.is_ascii_uppercase() || byte.is_ascii_digit() {
                    return Ok(byte as u16);
                }
            }

            if let Some(rest) = upper.strip_prefix('F') {
                if let Ok(function_key) = rest.parse::<u16>() {
                    if (1..=24).contains(&function_key) {
                        return Ok(0x6F + function_key);
                    }
                }
            }

            return Err(anyhow!(
                "unknown key name '{name}' (examples: Ctrl, Alt, Shift, Space, Tab, F5, [, ], -, =, A-Z, 0-9)"
            ));
        }
    };
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::parse_config;

    #[test]
    fn bundled_config_parses() {
        let source = std::fs::read_to_string("config.toml").expect("read config.toml");
        let config = parse_config(&source).expect("parse config.toml");
        assert!(!config.profiles.is_empty());
    }

    #[test]
    fn invalid_button_error_includes_profile_and_mapping() {
        let source = r#"
schema_version = 1

[profiles.test]
title_regex = "Test"

[profiles.test.map]
"LB+Nope" = "P"
"#;

        let err = parse_config(source).expect_err("config should fail");
        let message = format!("{err:#}");
        assert!(message.contains("profiles.test.map.LB+Nope"));
        assert!(message.contains("unknown pad button"));
    }

    #[test]
    fn invalid_key_error_includes_profile_and_mapping() {
        let source = r#"
schema_version = 1

[profiles.test]
title_regex = "Test"

[profiles.test.map]
A = "Ctrrl+Z"
"#;

        let err = parse_config(source).expect_err("config should fail");
        let message = format!("{err:#}");
        assert!(message.contains("profiles.test.map.A"));
        assert!(message.contains("unknown key name"));
    }
}
