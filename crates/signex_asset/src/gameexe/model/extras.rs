//! Remaining source-level root groups.

use crate::gameexe::{GameexeError, GameexeValue, SpannedToken, Token, gameexe};

#[gameexe]
#[derive(Debug)]
pub struct BgmFade2 {
    pub in_start_time: i32,
    #[gameexe(default = 500)]
    pub in_time_len: i32,
    pub out_start_time: i32,
    #[gameexe(default = 500)]
    pub out_time_len: i32,
    pub volume: i32,
}

#[gameexe]
#[derive(Debug)]
pub struct ExcallConfig {
    #[gameexe(default = 20000)]
    pub order: i32,
}

#[gameexe(value_field = "entry")]
#[derive(Debug)]
pub struct Emoji {
    pub entry: (String, i32),
}

#[gameexe]
#[derive(Debug)]
pub struct FontFile {
    pub name: String,
}

#[gameexe]
#[derive(Debug)]
pub struct MessageButton {
    pub normal: MessageButtonState,
    #[gameexe(default_with = MessageButtonState::highlighted)]
    pub hit: MessageButtonState,
    #[gameexe(default_with = MessageButtonState::highlighted)]
    pub push: MessageButtonState,
}

#[gameexe]
#[derive(Debug)]
pub struct MessageButtonState {
    pub color: i32,
}

impl MessageButtonState {
    fn highlighted() -> Self {
        Self { color: 2 }
    }
}

#[gameexe]
#[derive(Debug)]
pub struct Shortcut {
    #[gameexe(default = -1)]
    pub key: i32,
    pub cond: i32,
    #[gameexe(value, update, default = ShortcutScene::default())]
    pub scene: ShortcutScene,
}

#[derive(Debug, Default)]
pub struct ShortcutScene {
    pub name: String,
    pub z: i32,
    pub command: String,
}

impl GameexeValue for ShortcutScene {
    fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError> {
        let mut result = Self::default();
        result.update_value(value, path)?;
        Ok(result)
    }

    fn update_value(&mut self, value: &[SpannedToken<'_>], path: &str) -> Result<(), GameexeError> {
        match value {
            [name] => {
                self.name = String::parse_value(std::slice::from_ref(name), path)?;
                self.z = 0;
            }
            [name, comma, other] if comma.token == Token::Comma => {
                self.name = String::parse_value(std::slice::from_ref(name), path)?;
                match &other.token {
                    Token::String(_) => {
                        self.command = String::parse_value(std::slice::from_ref(other), path)?
                    }
                    Token::Integer(_) => {
                        self.z = i32::parse_value(std::slice::from_ref(other), path)?
                    }
                    _ => {
                        return Err(GameexeError::new(
                            other.span.clone(),
                            path,
                            "invalid shortcut target",
                        ));
                    }
                }
            }
            _ => {
                return Err(GameexeError::new(
                    value.first().map_or(0..0, |token| token.span.clone()),
                    path,
                    "invalid shortcut scene",
                ));
            }
        }
        Ok(())
    }
}
