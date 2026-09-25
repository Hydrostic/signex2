//! Values set by #CONFIG, before mutable runtime config is constructed.

use crate::gameexe::{GArray, gameexe};

#[gameexe]
#[derive(Debug)]
pub struct ConfigDefaults {
    #[gameexe(default = 0)]
    pub window_mode: i32,
    pub volume: ConfigVolume,
    #[gameexe(default = 192, min = 0, max = 255)]
    pub bgmfade_volume: i32,
    #[gameexe(default = true)]
    pub bgmfade_onoff: bool,
    /// Red, green, blue, alpha, as written in Gameexe.ini.
    #[gameexe(default = (0, 0, 0, 128))]
    pub filter_color: (i32, i32, i32, i32),
    pub font: ConfigFont,
    #[gameexe(default = 20)]
    pub message_speed: i32,
    pub message_speed_nowait: ConfigSwitch,
    #[gameexe(default_with = enabled_switch)]
    pub message_chrcolor: ConfigSwitch,
    pub mouse_cursor_hide_onoff: bool,
    #[gameexe(default = 5000)]
    pub mouse_cursor_hide_time: i32,
    #[gameexe(array(count = "", default_count = 4, count_min = 4, count_max = 4, item_default = |_| ConfigSwitch { onoff: true }))]
    pub object_disp: GArray<ConfigSwitch>,
    #[gameexe(array(count = "", default_count = 4, count_min = 4, count_max = 4, item_default = |_| ConfigSwitch { onoff: true }))]
    pub global_extra_switch: GArray<ConfigSwitch>,
    #[gameexe(array(count = "", default_count = 4, count_min = 4, count_max = 4))]
    pub global_extra_mode: GArray<ConfigMode>,
    pub sleep: ConfigSwitch,
    pub no_wipe_anime: ConfigSwitch,
    #[gameexe(default_with = enabled_switch)]
    pub skip_wipe_anime: ConfigSwitch,
    pub no_mwnd_anime: ConfigSwitch,
    #[gameexe(default_with = enabled_switch)]
    pub wheel_next_message: ConfigSwitch,
    pub koe_dont_stop: ConfigSwitch,
    pub skip_unread_message: ConfigSwitch,
    pub play_silent_sound: ConfigSwitch,
}

fn enabled_switch() -> ConfigSwitch {
    ConfigSwitch { onoff: true }
}

#[gameexe]
#[derive(Debug)]
pub struct ConfigVolume {
    #[gameexe(default = 255, min = 0, max = 255)]
    pub all: i32,
    #[gameexe(default = 255, min = 0, max = 255)]
    pub bgm: i32,
    #[gameexe(default = 255, min = 0, max = 255)]
    pub koe: i32,
    #[gameexe(default = 255, min = 0, max = 255)]
    pub pcm: i32,
    #[gameexe(default = 255, min = 0, max = 255)]
    pub se: i32,
    #[gameexe(default = 255, min = 0, max = 255)]
    pub mov: i32,
}

#[gameexe]
#[derive(Debug)]
pub struct ConfigFont {
    #[gameexe(name = "TYPE")]
    pub r#type: i32,
    #[gameexe(default = String::from("ＭＳ ゴシック"))]
    pub name: String,
    pub futoku: bool,
    #[gameexe(default = 2)]
    pub shadow: i32,
    #[gameexe(default = String::from("「あいう漢字カナ薔薇」"))]
    pub sample_str_short: String,
    #[gameexe(default = String::from("「あいう漢字カナＡＢＣ０１２薔薇」"))]
    pub sample_str_long: String,
}

#[gameexe]
#[derive(Debug)]
pub struct ConfigSwitch {
    pub onoff: bool,
}

#[gameexe]
#[derive(Debug)]
pub struct ConfigMode {
    pub mode: i32,
}
