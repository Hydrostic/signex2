//! Character voice declarations from #CHRKOE.

use super::helpers::split_comma_separated;
use crate::gameexe::{GArray, GameexeError, GameexeValue, SpannedToken, Token, gameexe};

#[gameexe]
#[derive(Debug)]
pub struct ChrkoeConfig {
    #[gameexe(default = String::from("？？？"))]
    pub not_look_name_str: String,
    #[gameexe(flatten, array(value = true, count = "CNT", default_count = 64, count_min = 0, count_max = 256, item_default = Chrkoe::at))]
    pub entries: GArray<Chrkoe>,
}

#[derive(Debug)]
pub struct Chrkoe {
    pub name: String,
    pub check_mode: i32,
    pub check_name: String,
    pub onoff: bool,
    pub volume: i32,
    pub character_numbers: Vec<i32>,
}

impl Chrkoe {
    fn at(_: usize) -> Self {
        Self {
            name: String::new(),
            check_mode: 1,
            check_name: String::new(),
            onoff: true,
            volume: 255,
            character_numbers: Vec::new(),
        }
    }
}

impl GameexeValue for Chrkoe {
    fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError> {
        let mut item = Self::at(0);
        item.update_value(value, path)?;
        Ok(item)
    }

    fn update_value(&mut self, value: &[SpannedToken<'_>], path: &str) -> Result<(), GameexeError> {
        let Some(open) = value
            .iter()
            .position(|token| token.token == Token::LeftParen)
        else {
            return Err(GameexeError::new(
                value.first().map_or(0..0, |token| token.span.clone()),
                path,
                "expected character number list",
            ));
        };
        if open == 0 || value[open - 1].token != Token::Comma {
            return Err(GameexeError::new(
                value.first().map_or(0..0, |token| token.span.clone()),
                path,
                "expected character number list",
            ));
        }
        if !matches!(value.last(), Some(token) if token.token == Token::RightParen)
            || open + 2 > value.len()
        {
            return Err(GameexeError::new(
                value.first().map_or(0..0, |token| token.span.clone()),
                path,
                "invalid character number list",
            ));
        }
        let parts = split_comma_separated(&value[..open - 1], path)?;
        if !matches!(parts.len(), 3 | 5) {
            return Err(GameexeError::new(
                value[open].span.clone(),
                path,
                "expected name, mode, check name[, onoff, volume], list",
            ));
        }
        self.name = String::parse_value(parts[0], path)?;
        self.check_mode = i32::parse_value(parts[1], path)?;
        self.check_name = String::parse_value(parts[2], path)?;
        if parts.len() == 5 {
            self.onoff = bool::parse_value(parts[3], path)?;
            self.volume = i32::parse_value(parts[4], path)?.clamp(0, 255);
        }
        for part in split_comma_separated(&value[open + 1..value.len() - 1], path)? {
            self.character_numbers.push(i32::parse_value(part, path)?);
        }
        Ok(())
    }
}
