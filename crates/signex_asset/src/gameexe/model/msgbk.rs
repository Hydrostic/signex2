//! Message history menu fields from C_tnm_ini::S_msg_back.

use crate::gameexe::gameexe;

#[gameexe]
#[derive(Debug)]
pub struct MessageBack {
    #[gameexe(default = 10000)]
    pub order: i32,
    #[gameexe(default = 256, min = 0, max = 1024)]
    pub history_cnt: i32,
    #[gameexe(default = (10, 10))]
    pub window_pos: (i32, i32),
    #[gameexe(default = (780, 580))]
    pub window_size: (i32, i32),
    #[gameexe(default = (20, 20, 20, 20))]
    pub disp_margin: (i32, i32, i32, i32),
    #[gameexe(name = "MESSAGE_POS", default = 30)]
    pub msg_pos: i32,
    #[gameexe(default = (20, 15))]
    pub moji_cnt: (i32, i32),
    #[gameexe(default = 24)]
    pub moji_size: i32,
    #[gameexe(default = (-1, 10))]
    pub moji_space: (i32, i32),
    #[gameexe(default = 1)]
    pub moji_color: i32,
    pub moji_shadow_color: i32,
    pub moji_fuchi_color: i32,
    #[gameexe(default = 7)]
    pub active_moji_color: i32,
    pub active_moji_shadow_color: i32,
    pub active_moji_fuchi_color: i32,
    #[gameexe(default = 5)]
    pub debug_moji_color: i32,
    pub debug_moji_shadow_color: i32,
    pub debug_moji_fuchi_color: i32,
    #[gameexe(default = -1)]
    pub name_disp_mode: i32,
    #[gameexe(name = "NAME_BRACKET", default = 1)]
    pub name_bracket_type: i32,
    #[gameexe(name = "BACK_FILE")]
    pub waku_file: String,
    pub filter_file: String,
    pub filter_margin: (i32, i32, i32, i32),
    /// INI order is red, green, blue, alpha.
    pub filter_color: (i32, i32, i32, i32),
    pub separator_file: String,
    pub separator_top_file: String,
    pub separator_bottom_file: String,
    #[gameexe(default = 1)]
    pub msg_click_action: i32,
    pub load_call: (String, String),
}

#[gameexe]
#[derive(Debug)]
pub struct MessageBackItems {
    pub slider: MessageBackSlider,
    pub close_btn: MessageBackButton,
    pub msg_up_btn: MessageBackButton,
    pub msg_down_btn: MessageBackButton,
    #[gameexe(default_with = koe_button_default)]
    pub koe_btn: MessageBackButton,
    #[gameexe(default_with = load_button_default)]
    pub load_btn: MessageBackButton,
    pub ex_btn_1: MessageBackExtraButton,
    pub ex_btn_2: MessageBackExtraButton,
    pub ex_btn_3: MessageBackExtraButton,
    pub ex_btn_4: MessageBackExtraButton,
}

fn koe_button_default() -> MessageBackButton {
    MessageBackButton {
        file: String::new(),
        pos: (-20, -10),
        action: 0,
        se: 0,
    }
}

fn load_button_default() -> MessageBackButton {
    MessageBackButton {
        file: String::new(),
        pos: (-20, 0),
        action: 0,
        se: 0,
    }
}

#[gameexe]
#[derive(Debug)]
pub struct MessageBackSlider {
    pub file: String,
    /// INI supplies x, y, and height; the original stores rect(x, y, x, height).
    pub pos: (i32, i32, i32),
    pub action: i32,
    pub se: i32,
}

#[gameexe]
#[derive(Debug)]
pub struct MessageBackButton {
    pub file: String,
    pub pos: (i32, i32),
    pub action: i32,
    pub se: i32,
}

#[gameexe]
#[derive(Debug)]
pub struct MessageBackExtraButton {
    pub file: String,
    pub pos: (i32, i32),
    pub action: i32,
    pub se: i32,
    pub call: (String, String),
}
