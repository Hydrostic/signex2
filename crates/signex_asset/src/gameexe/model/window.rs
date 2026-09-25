//! Message windows and their geometry.

use crate::gameexe::{GArray, gameexe};

#[gameexe]
#[derive(Debug)]
pub struct MwndConfig {
    pub default_mwnd_no: i32,
    #[gameexe(default = 1)]
    pub default_sel_mwnd_no: i32,
    #[gameexe(default = 1)]
    pub order: i32,
    pub filter_layer_rep: i32,
    #[gameexe(default = 1)]
    pub waku_layer_rep: i32,
    #[gameexe(default = 3)]
    pub shadow_layer_rep: i32,
    #[gameexe(default = 4)]
    pub fuchi_layer_rep: i32,
    #[gameexe(default = 5)]
    pub moji_layer_rep: i32,
    #[gameexe(default = 2)]
    pub face_layer_rep: i32,
    #[gameexe(default = 1)]
    pub shadow_color: i32,
    #[gameexe(default = 1)]
    pub fuchi_color: i32,
    pub moji_color: i32,
    #[gameexe(
        flatten,
        array(count = "CNT", default_count = 2, count_min = 0, count_max = 256)
    )]
    pub windows: GArray<MwndWindow>,
}

#[gameexe]
#[derive(Debug)]
pub struct MwndWindow {
    pub novel_mode: i32,
    pub extend_type: i32,
    #[gameexe(default = (50, 400))]
    pub window_pos: (i32, i32),
    #[gameexe(default = (700, 150))]
    pub window_size: (i32, i32),
    #[gameexe(name = "MESSAGE_POS", default = (20, 20))]
    pub msg_pos: (i32, i32),
    #[gameexe(name = "MESSAGE_MARGIN", default = (20, 20, 20, 20))]
    pub msg_margin: (i32, i32, i32, i32),
    #[gameexe(default = (26, 3))]
    pub moji_cnt: (i32, i32),
    #[gameexe(default = 25)]
    pub moji_size: i32,
    #[gameexe(default = (-1, 10))]
    pub moji_space: (i32, i32),
    #[gameexe(default = -1)]
    pub moji_color: i32,
    #[gameexe(default = -1)]
    pub shadow_color: i32,
    #[gameexe(default = -1)]
    pub fuchi_color: i32,
    #[gameexe(default = 10)]
    pub ruby_size: i32,
    #[gameexe(default = 1)]
    pub ruby_space: i32,
    pub waku_no: i32,
    pub waku_pos: (i32, i32),
    pub name_disp_mode: i32,
    pub name_newline: i32,
    pub name_bracket: i32,
    #[gameexe(default = -1)]
    pub name_moji_color: i32,
    #[gameexe(default = -1)]
    pub name_shadow_color: i32,
    #[gameexe(default = -1)]
    pub name_fuchi_color: i32,
    pub talk_margin: (i32, i32, i32, i32),
    #[gameexe(name = "OVERFLOW_CHECK_SIZE")]
    pub over_flow_check_size: i32,
    pub msg_back_insert_nl: i32,
    pub name_extend_type: i32,
    pub name_window_align: i32,
    #[gameexe(default = (0, -100))]
    pub name_window_pos: (i32, i32),
    #[gameexe(default = (300, 100))]
    pub name_window_size: (i32, i32),
    #[gameexe(name = "NAME_MESSAGE_POS", default = (8, 8))]
    pub name_msg_pos: (i32, i32),
    #[gameexe(name = "NAME_MESSAGE_POS_REP")]
    pub name_msg_pos_rep: (i32, i32),
    #[gameexe(name = "NAME_MESSAGE_MARGIN", default = (8, 8, 8, 8))]
    pub name_msg_margin: (i32, i32, i32, i32),
    #[gameexe(default = 16)]
    pub name_moji_size: i32,
    #[gameexe(default = (-1, 8))]
    pub name_moji_space: (i32, i32),
    #[gameexe(default = 8)]
    pub name_moji_cnt: i32,
    #[gameexe(default = -1)]
    pub name_waku_no: i32,
    pub face_hide_name: i32,
    pub open_anime_type: i32,
    pub open_anime_time: i32,
    pub close_anime_type: i32,
    pub close_anime_time: i32,
}
