//! Top-level typed Gameexe.ini model.

use super::{GArray, gameexe};

mod button;
mod config;
mod core;
mod counts;
mod dialog;
mod extras;
mod helpers;
mod misc;
mod msgbk;
mod ui_strings;
mod voice;
mod waku;
mod window;
mod joypad;

pub use button::*;
pub use config::*;
pub use core::*;
pub use counts::*;
pub use dialog::*;
pub use extras::*;
pub use misc::*;
pub use msgbk::{MessageBack, MessageBackItems};
pub use ui_strings::*;
pub use voice::*;
pub use waku::*;
pub use window::*;
pub use joypad::*;

#[gameexe]
#[derive(Debug)]
pub struct Gameexe {
    #[gameexe(default = 0)]
    pub debug_error_patno_out_of_range: i32,
    #[gameexe(default = String::from("SampleProject"))]
    pub gameid: String,
    #[gameexe(default = String::from("サンプルプロジェクト"))]
    pub gamename: String,
    #[gameexe(default = String::new())]
    pub gameversion: String,
    #[gameexe(default = String::from("SampleProject.env"))]
    pub discmark: String,
    #[gameexe(default = String::new())]
    pub manual_path: String,
    #[gameexe(default = String::from("（ゲーム名）のディスクを入れてください。\n\n※プロテクトの誤認識ではありません。"))]
    pub dummy_check_str: String,
    #[gameexe(default = String::from("この度は（ゲーム名）をお買い上げいただき、ありがとうございます。"))]
    pub dummy_check_ok_str: String,
    #[gameexe(default = (800, 600))]
    pub screen_size: (i32, i32),
    #[gameexe(value, update, default = SceneRef::new("_start"))]
    pub start_scene: SceneRef,
    #[gameexe(value, update, default = SceneRef::new("_menu"))]
    pub menu_scene: SceneRef,
    // `x_menu_scene` is not used in old engine
    #[gameexe(value, update, default = SceneRef::new(""))]
    pub x_menu_scene: SceneRef,
    #[gameexe(value, update, default = SceneRef::new(""))]
    pub cancel_scene: SceneRef,
    #[gameexe(value, update, default = SceneRef::new(""))]
    pub config_scene: SceneRef,
    #[gameexe(value, update, default = SceneRef::new(""))]
    pub save_scene: SceneRef,
    #[gameexe(value, update, default = SceneRef::new(""))]
    pub load_scene: SceneRef,
    #[gameexe(value, update, default = SceneRef::new(""))]
    pub load_after_call: SceneRef,
    pub system: SystemValues,
    #[gameexe(array(
        count = "CNT",
        default_count = 0,
        count_min = 0,
        count_max = 256,
        index_by_capacity = true
    ))]
    pub flick_scene: GArray<FlickScene>,
    #[gameexe(array(count = "CNT", default_count = 16, count_min = 0, count_max = 256))]
    pub icon: GArray<Icon>,
    pub joypad: JoypadConfig,
    pub mouse_cursor: MouseCursorConfig,
    pub syscommenu: SyscomMenu,
    pub msgbk: MessageBack,
    pub msgbk_item: MessageBackItems,
    pub twitter: TwitterConfig,
    pub config: ConfigDefaults,
    pub dialog_style: DialogStyle,
    pub dialog_tab_exist: DialogTabs,
    pub dialog_exist: DialogExist,
    pub dialog: DialogConfig,
    pub warninginfo: WarningInfo,
    pub saveload_dialog: SaveLoadDialog,
    #[gameexe(default_with = SaveLoadInfo::save_defaults)]
    pub saveinfo: SaveLoadInfo,
    #[gameexe(default_with = SaveLoadInfo::load_defaults)]
    pub loadinfo: SaveLoadInfo,
    pub chrkoe: ChrkoeConfig,
    pub save_thumb: SaveThumb,
    pub load: LoadConfig,
    pub thumbtable_file: String,
    pub tonecurve_file: String,
    pub cgtable_file: String,
    pub tateyoko_mode: i32,
    #[gameexe(value, update, default = NamaeList::default())]
    pub namae: NamaeList,
    pub save: SaveConfig,
    pub quick_save: QuickSaveCount,
    pub end_save: EndSaveCount,
    pub inner_save: InnerSaveCount,
    pub message_back_save: MessageBackSaveConfig,
    pub save_history: SaveHistoryCount,
    pub sel_save: SelSaveCount,
    pub flag: FlagCount,
    pub global_flag: GlobalFlagCount,
    pub call_flag: CallFlagCount,
    pub counter: CounterCount,
    #[gameexe(array(count = "CNT", default_count = 0, count_min = 0, count_max = 256))]
    pub database: GArray<String>,
    #[gameexe(array(count = "CNT", default_count = 256, count_min = 256, count_max = 256, item_default = color_table_default))]
    pub color_table: GArray<(i32, i32, i32)>,
    pub pcmch: PcmchCount,
    pub pcmevent: PcmEventCount,
    pub effect: EffectCount,
    pub quake: QuakeCount,
    pub frame_action_ch: FrameActionCount,
    pub editbox: EditboxCount,
    pub g00buf: G00BufferCount,
    pub mask: MaskCount,
    pub objbtngroup: ObjectButtonGroupCount,
    pub button: ButtonConfig,
    pub mwnd: MwndConfig,
    pub waku: WakuConfig,
    pub bgmfade2: BgmFade2,
    pub excall: ExcallConfig,
    #[gameexe(array(count = "CNT", default_count = 16, count_min = 0, count_max = 256))]
    pub emoji: GArray<Emoji>,
    #[gameexe(array(count = "CNT", default_count = 0, count_min = 0, count_max = 256))]
    pub font_file: GArray<FontFile>,
    #[gameexe(array(count = "CNT", default_count = 0, count_min = 0, count_max = 256))]
    pub private_font_file: GArray<FontFile>,
    #[gameexe(array(count = "CNT", default_count = 16, count_min = 0, count_max = 256))]
    pub msgbtn: GArray<MessageButton>,
    #[gameexe(array(
        count = "CNT",
        default_count = 0,
        count_min = 0,
        count_max = 256,
        index_by_capacity = true
    ))]
    pub shortcut: GArray<Shortcut>,
    #[gameexe(array(count = "CNT", default_count = 1, count_min = 0, count_max = 256))]
    pub world: GArray<World>,
    #[gameexe(default = 1000, min = 0, max = 10000)]
    pub cgtable_flag_cnt: i32,
    #[gameexe(array(
        count = "CNT",
        default_count = 256,
        count_min = 0,
        count_max = 1024,
        range = true
    ))]
    pub object: GArray<Object>,
    #[gameexe(array(count = "CNT", default_count = 16, count_min = 0, count_max = 256))]
    pub selbtn: GArray<Selbtn>,
    #[gameexe(array(count = "CNT", default_count = 16, count_min = 0, count_max = 256))]
    pub shake: GArray<Vec<(i32, i32, i32)>>,
    #[gameexe(array(count = "CNT", default_count = 32, count_min = 0, count_max = 256))]
    pub bgm: GArray<Bgm>,
    #[gameexe(array(count = "CNT", default_count = 16, count_min = 8, count_max = 256))]
    pub se: GArray<Se>,
}
