use super::helpers::split_comma_separated;
use crate::gameexe::{GameexeError, GameexeValue, SpannedToken, gameexe};

#[gameexe]
#[derive(Debug)]
pub struct TwitterConfig {
    pub api_key: String,
    pub api_secret: String,
    pub callback_url: String,
    pub initial_tweet_text: String,
    pub overlap_image: String,
}

#[gameexe]
#[derive(Debug)]
pub struct SaveThumb {
    #[gameexe(name = "USE")]
    pub use_thumb: bool,
    #[gameexe(name = "TYPE")]
    pub r#type: i32,
    #[gameexe(default = (200, 150))]
    pub size: (i32, i32),
}

#[gameexe]
#[derive(Debug)]
pub struct LoadConfig {
    #[gameexe(default = (0, 1000))]
    pub wipe: (i32, i32),
}

#[derive(Debug, Default)]
pub struct NamaeList(pub Vec<Namae>);

#[derive(Debug)]
pub struct Namae {
    pub regist_name: String,
    pub change_name: String,
    pub color_mode: i32,
    pub moji_color: i32,
    pub shadow_color: i32,
    pub fuchi_color: i32,
}

impl GameexeValue for NamaeList {
    fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError> {
        let mut list = Self::default();
        list.update_value(value, path)?;
        Ok(list)
    }

    fn update_value(&mut self, value: &[SpannedToken<'_>], path: &str) -> Result<(), GameexeError> {
        let parts = split_comma_separated(value, path)?;
        if parts.len() != 5 && parts.len() != 6 {
            return Err(GameexeError::new(
                value.first().map_or(0..0, |token| token.span.clone()),
                path,
                "expected five or six NAMAE values",
            ));
        }
        let fuchi_color = if parts.len() == 6 {
            i32::parse_value(parts[5], path)?
        } else {
            -1
        };
        self.0.push(Namae {
            regist_name: String::parse_value(parts[0], path)?,
            change_name: String::parse_value(parts[1], path)?,
            color_mode: i32::parse_value(parts[2], path)?,
            moji_color: i32::parse_value(parts[3], path)?,
            shadow_color: i32::parse_value(parts[4], path)?,
            fuchi_color,
        });
        Ok(())
    }
}
