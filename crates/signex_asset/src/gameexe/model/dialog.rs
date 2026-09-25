//! Configuration dialog presence, labels, and mode inventories.

use super::ModeItem;
use crate::gameexe::{GArray, gameexe};
use signex_util::fullwidth;

#[gameexe]
#[derive(Debug)]
pub struct DialogStyle {
    pub volume: i32,
    pub koe: i32,
}

#[gameexe]
#[derive(Debug)]
pub struct DialogTabs {
    #[gameexe(default = true)]
    pub screen: bool,
    #[gameexe(default = true)]
    pub volume: bool,
    #[gameexe(default = true)]
    pub message: bool,
    #[gameexe(default = true)]
    pub mwndbk: bool,
    #[gameexe(default = true)]
    pub koe: bool,
    #[gameexe(default = true)]
    pub automode: bool,
    #[gameexe(default = true)]
    pub jitan: bool,
    #[gameexe(name = "ELSE", default = true)]
    pub r#else: bool,
    #[gameexe(default = true)]
    pub system: bool,
}

#[gameexe]
#[derive(Debug)]
pub struct DialogExist {
    #[gameexe(default = true)]
    pub bgm: bool,
    #[gameexe(default = true)]
    pub koe: bool,
    #[gameexe(default = true)]
    pub pcm: bool,
    #[gameexe(default = true)]
    pub se: bool,
    #[gameexe(default = true)]
    pub movie: bool,
}

#[gameexe]
#[derive(Debug)]
pub struct DialogConfig {
    #[gameexe(default_with = message_chrcolor_default)]
    pub message_chrcolor: DialogLabel,
    #[gameexe(array(count = "", default_count = 4, count_min = 4, count_max = 4, item_default = object_disp_default))]
    pub object_disp: GArray<DialogLabel>,
    #[gameexe(array(count = "", default_count = 4, count_min = 4, count_max = 4, item_default = global_switch_default))]
    pub global_extra_switch: GArray<DialogLabel>,
    #[gameexe(array(count = "", default_count = 4, count_min = 4, count_max = 4, item_default = DialogMode::at))]
    pub global_extra_mode: GArray<DialogMode>,
    #[gameexe(default_with = sleep_default)]
    pub sleep: DialogLabel,
    #[gameexe(default_with = no_wipe_default)]
    pub no_wipe_anime: DialogLabel,
    #[gameexe(default_with = skip_wipe_default)]
    pub skip_wipe_anime: DialogLabel,
    #[gameexe(default_with = no_mwnd_default)]
    pub no_mwnd_anime: DialogLabel,
    #[gameexe(default_with = wheel_default)]
    pub wheel_next_message: DialogLabel,
    #[gameexe(default_with = koe_default)]
    pub koe_dont_stop: DialogLabel,
    #[gameexe(default_with = unread_default)]
    pub skip_unread_message: DialogLabel,
    #[gameexe(default_with = silent_default)]
    pub play_silent_sound: DialogLabel,
}

#[gameexe]
#[derive(Debug)]
pub struct DialogLabel {
    #[gameexe(default = true)]
    pub exist: bool,
    pub str: String,
}

fn label(exist: bool, name: &str) -> DialogLabel {
    DialogLabel {
        exist,
        str: name.into(),
    }
}
fn message_chrcolor_default() -> DialogLabel {
    label(true, "文章を色分けする。")
}
fn object_disp_default(index: usize) -> DialogLabel {
    label(
        index < 2,
        &format!("オブジェクト表示{}番を表示する。", fullwidth(index)),
    )
}
fn global_switch_default(index: usize) -> DialogLabel {
    label(
        index < 2,
        &format!("グローバル汎用スイッチ{}番を使用する。", fullwidth(index)),
    )
}
fn sleep_default() -> DialogLabel {
    label(
        true,
        "本プログラムの動作を遅くして、他のプログラムがスムーズに動作するようにする。",
    )
}
fn no_wipe_default() -> DialogLabel {
    label(true, "画面暗転効果のアニメを無効にする。")
}
fn skip_wipe_default() -> DialogLabel {
    label(true, "画面暗転効果をマウスクリックで飛ばす。")
}
fn no_mwnd_default() -> DialogLabel {
    label(true, "メッセージウィンドウの開閉時のアニメを無効にする。")
}
fn wheel_default() -> DialogLabel {
    label(true, "マウスのホイールボタンの下回しで文章を読み進める。")
}
fn koe_default() -> DialogLabel {
    label(true, "声の再生中に次の文章に進んでも再生を続ける。")
}
fn unread_default() -> DialogLabel {
    label(true, "未読の文章も早送りできるようにする。")
}
fn silent_default() -> DialogLabel {
    label(true, "サウンド再生時に雑音が入る場合はチェックして下さい。")
}

#[gameexe]
#[derive(Debug)]
pub struct DialogMode {
    #[gameexe(default = true)]
    pub exist: bool,
    pub str: String,
    #[gameexe(min = 0, max = 8)]
    pub item_cnt: i32,
    #[gameexe(array(count = "", default_count = 8, count_min = 8, count_max = 8, item_default = ModeItem::at))]
    pub item: GArray<ModeItem>,
}

impl DialogMode {
    fn at(index: usize) -> Self {
        let mut result = <Self as crate::gameexe::GameexeNode>::gameexe_default();
        result.str = format!("グローバル汎用モード{}番", fullwidth(index));
        result.item_cnt = if index < 2 { 3 } else { 1 };
        result
    }
}
