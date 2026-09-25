//! Button action and sound defaults.

use crate::gameexe::{GArray, GameexeError, GameexeValue, SpannedToken, gameexe};

#[gameexe]
#[derive(Debug)]
pub struct ButtonConfig {
    #[gameexe(array(count = "CNT", default_count = 16, count_min = 0, count_max = 256))]
    pub action: GArray<ButtonAction>,
    #[gameexe(array(count = "CNT", default_count = 16, count_min = 0, count_max = 256))]
    pub se: GArray<ButtonSe>,
}

#[gameexe]
#[derive(Debug)]
pub struct ButtonAction {
    #[gameexe(value, default = ButtonVisual::new(0, 0, 0, 255, 0, 0))]
    pub normal: ButtonVisual,
    #[gameexe(value, default = ButtonVisual::new(0, 0, 0, 255, 32, 0))]
    pub hit: ButtonVisual,
    #[gameexe(value, default = ButtonVisual::new(0, 1, 1, 255, 32, 0))]
    pub push: ButtonVisual,
    #[gameexe(value, default = ButtonVisual::new(0, 0, 0, 255, 0, 0))]
    pub select: ButtonVisual,
    #[gameexe(value, default = ButtonVisual::new(0, 0, 0, 255, 0, 0))]
    pub disable: ButtonVisual,
}

#[derive(Debug)]
pub struct ButtonVisual {
    pub pattern: i32,
    pub position: (i32, i32),
    pub transparency: i32,
    pub brightness: i32,
    pub darkness: i32,
}

impl ButtonVisual {
    fn new(
        pattern: i32,
        x: i32,
        y: i32,
        transparency: i32,
        brightness: i32,
        darkness: i32,
    ) -> Self {
        Self {
            pattern,
            position: (x, y),
            transparency,
            brightness,
            darkness,
        }
    }
}

impl GameexeValue for ButtonVisual {
    fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError> {
        let (pattern, x, y, transparency, brightness, darkness) =
            <(i32, i32, i32, i32, i32, i32)>::parse_value(value, path)?;
        Ok(Self::new(
            pattern,
            x,
            y,
            transparency.clamp(0, 255),
            brightness.clamp(0, 255),
            darkness.clamp(0, 255),
        ))
    }
}

#[gameexe]
#[derive(Debug)]
pub struct ButtonSe {
    pub hit: i32,
    #[gameexe(default = -1)]
    pub push: i32,
    #[gameexe(default = 1)]
    pub decide: i32,
}
