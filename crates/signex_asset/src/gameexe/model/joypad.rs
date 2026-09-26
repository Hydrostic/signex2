
use crate::gameexe::{GArray, gameexe, model::SceneRef};

#[gameexe]
#[derive(Debug)]
pub struct JoypadConfig {
    pub allow_joypad_mode: i32,
    pub dpad_control_mode: i32,
    pub left_stick_control_mode: i32,

    pub msgbk_allow_joypad_mode: i32,
    pub msgbk_dpad_control_mode: i32,
    pub msgbk_left_stick_control_mode: i32,
    #[gameexe(array(count = "CNT", default_count = 0, count_min = 0, count_max = usize::MAX))]
    pub shortcut: GArray<JoypadShortcut>,
}

#[gameexe]
#[derive(Debug)]
pub struct JoypadShortcut {
    pub key: i32,
    pub cond: i32,
    #[gameexe(value, update, default = SceneRef::new(""))]
    pub scene: SceneRef,
}