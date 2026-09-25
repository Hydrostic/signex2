use crate::gameexe::{GArray, GameexeError, GameexeValue, SpannedToken, Token, gameexe};
use signex_util::fullwidth;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneRef {
    pub name: String,
    pub z: i32,
}

impl SceneRef {
    pub(crate) fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            z: 0,
        }
    }

    pub(crate) fn parse_required_z(
        value: &[SpannedToken<'_>],
        path: &str,
    ) -> Result<Self, GameexeError> {
        if value.len() != 3 {
            return Err(GameexeError::new(
                value.first().map_or(0..0, |token| token.span.clone()),
                path,
                "expected scene name and Z",
            ));
        }
        Self::parse_value(value, path)
    }
}

impl GameexeValue for SceneRef {
    fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError> {
        let (name, z) = match value {
            [name] => (std::slice::from_ref(name), None),
            [name, comma, z] if comma.token == Token::Comma => {
                (std::slice::from_ref(name), Some(std::slice::from_ref(z)))
            }
            _ => {
                return Err(GameexeError::new(
                    value.first().map_or(0..0, |v| v.span.clone()),
                    path,
                    "expected scene name[, Z]",
                ));
            }
        };
        Ok(Self {
            name: String::parse_value(name, path)?,
            z: match z {
                Some(z) => i32::parse_value(z, path)?,
                None => 0,
            },
        })
    }

    fn update_value(&mut self, value: &[SpannedToken<'_>], path: &str) -> Result<(), GameexeError> {
        if value.len() == 1 {
            self.name = String::parse_value(value, path)?;
        } else {
            *self = Self::parse_value(value, path)?;
        }
        Ok(())
    }
}

pub(crate) fn color_table_default(index: usize) -> (i32, i32, i32) {
    match index {
        1 => (0, 0, 0),
        2 => (255, 0, 0),
        3 => (0, 255, 0),
        4 => (0, 0, 255),
        5 => (255, 255, 0),
        6 => (255, 0, 255),
        7 => (0, 255, 255),
        _ => (255, 255, 255),
    }
}

#[gameexe]
#[derive(Debug)]
pub struct World {
    pub layer: i32,
}

#[gameexe]
#[derive(Debug)]
pub struct FlickScene {
    #[gameexe(value, parse_with = SceneRef::parse_required_z, default = SceneRef::new(""))]
    pub scene: SceneRef,
    pub angle: i32,
}

#[gameexe]
#[derive(Debug)]
pub struct Icon {
    #[gameexe(name = "FILE")]
    pub file_name: String,
    #[gameexe(name = "CNT", default = 1)]
    pub anime_pat_cnt: i32,
    #[gameexe(default = 100)]
    pub speed: i32,
}

#[gameexe]
#[derive(Debug)]
pub struct SystemValues {
    #[gameexe(array(count = "CNT", default_count = 0, count_min = 0, count_max = 256))]
    pub extra_int_value: GArray<i32>,
    #[gameexe(array(count = "CNT", default_count = 0, count_min = 0, count_max = 256))]
    pub extra_str_value: GArray<String>,
}

#[gameexe]
#[derive(Debug)]
pub struct MouseCursorConfig {
    #[gameexe(name = "DEFAULT", default = -1, validate_with_context = MouseCursorConfig::validate_default)]
    pub default_index: i32,
    #[gameexe(
        flatten,
        array(count = "CNT", default_count = 16, count_min = 0, count_max = 256)
    )]
    pub items: GArray<MouseCursor>,
}

impl MouseCursorConfig {
    fn validate_default(&self, index: &i32) -> Result<(), &'static str> {
        if *index == -1 || (*index >= 0 && (*index as usize) < self.items.len()) {
            Ok(())
        } else {
            Err("default cursor index outside current count")
        }
    }
}

#[gameexe]
#[derive(Debug)]
pub struct MouseCursor {
    #[gameexe(name = "FILE")]
    pub file_name: String,
    #[gameexe(default = 100)]
    pub speed: i32,
}

#[gameexe]
#[derive(Debug)]
pub struct Object {
    #[gameexe(name = "USE", default = true)]
    pub use_flag: bool,
    #[gameexe(default = true)]
    pub save: bool,
    #[gameexe(default = false)]
    pub space_hide: bool,
    #[gameexe(default = -1)]
    pub object_disp_no: i32,
}

#[gameexe]
#[derive(Debug)]
pub struct Selbtn {
    pub base_file: String,
    #[gameexe(name = "BACK_FILE")]
    pub filter_file: String,
    pub base_pos: (i32, i32),
    pub rep_pos: (i32, i32),
    pub align: (i32, i32),
    pub max_y_cnt: i32,
    #[gameexe(default = 100)]
    pub line_width: i32,
    #[gameexe(default = (25, 0, 0, 0))]
    pub moji_size: (i32, i32, i32, i32),
    pub moji_pos: (i32, i32),
    pub moji_align: (i32, i32),
    pub moji_color: i32,
    #[gameexe(default = 5)]
    pub moji_hit_color: i32,
    // The following fields until `moji_hit_fuchi_color` are not used in the old engine,
    // but appear in the new gameexe format, so we include them for compatibility.
    pub moji_shadow_color: i32,
    pub moji_fuchi_color: i32,
    pub moji_hit_shadow_color: i32,
    pub moji_hit_fuchi_color: i32,

    pub btn_action: i32,
    #[gameexe(default = (1, 500))]
    pub open_anime: (i32, i32),
    #[gameexe(default = (1, 500))]
    pub close_anime: (i32, i32),
    #[gameexe(default = (1, 500))]
    pub decide_anime: (i32, i32),
}

#[gameexe(value_field = "entry")]
#[derive(Debug)]
pub struct Bgm {
    #[gameexe(default = (String::new(), String::new(), 0, -1, 0), validate_with = validate_bgm)]
    pub entry: (String, String, i32, i32, i32),
}

fn validate_bgm(entry: &(String, String, i32, i32, i32)) -> Result<(), &'static str> {
    if entry.3 == 0 {
        Err("BGM end position cannot be zero")
    } else {
        Ok(())
    }
}

impl Bgm {
    pub fn regist_name(&self) -> &str {
        &self.entry.0
    }
    pub fn file_name(&self) -> &str {
        &self.entry.1
    }
    pub fn start_pos(&self) -> i32 {
        self.entry.2
    }
    pub fn end_pos(&self) -> i32 {
        self.entry.3
    }
    pub fn repeat_pos(&self) -> i32 {
        self.entry.4
    }
}

#[gameexe(value_field = "file_name")]
#[derive(Debug)]
pub struct Se {
    #[gameexe(name = "FILE")]
    pub file_name: String,
}

#[gameexe]
#[derive(Debug)]
pub struct SyscomMenu {
    #[gameexe(default = (true, true, String::from("既読文章を早送り")))]
    pub read_skip: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("文章を早送り")))]
    pub unread_skip: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("自動早送り")))]
    pub auto_skip: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("オートモード")))]
    pub auto_mode: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("メッセージを隠す")))]
    pub hide_mwnd: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("メッセージログを開く")))]
    pub msg_back: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("セーブ")))]
    pub save: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("ロード")))]
    pub load: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("前の選択肢に戻る")))]
    pub return_sel: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("環境設定")))]
    pub config: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("マニュアル")))]
    pub manual: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("バージョン情報")))]
    pub version: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("タイトルに戻る")))]
    pub return_menu: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("ゲームを終了する")))]
    pub game_end: (bool, bool, String),
    #[gameexe(default = (true, true, String::from("キャンセル")))]
    pub cancel: (bool, bool, String),
    #[gameexe(array(count = "", default_count = 4, count_min = 4, count_max = 4, item_default = LocalExtraSwitch::at))]
    pub local_extra_switch: GArray<LocalExtraSwitch>,
    #[gameexe(array(count = "", default_count = 4, count_min = 4, count_max = 4, item_default = LocalExtraMode::at))]
    pub local_extra_mode: GArray<LocalExtraMode>,
}

#[gameexe(value_field = "header")]
#[derive(Debug)]
pub struct LocalExtraSwitch {
    #[gameexe(default = (true, true, true, String::new()))]
    pub header: (bool, bool, bool, String),
}

impl LocalExtraSwitch {
    fn at(index: usize) -> Self {
        Self {
            header: (
                index < 4,
                true,
                true,
                format!("ローカル汎用スイッチ{}番", fullwidth(index)),
            ),
        }
    }
}

#[gameexe(value_field = "header")]
#[derive(Debug)]
pub struct LocalExtraMode {
    #[gameexe(default = (true, true, 0, String::new()))]
    pub header: (bool, bool, i32, String),
    #[gameexe(default = 3, min = 0, max = 8)]
    pub item_cnt: i32,
    #[gameexe(array(count = "", default_count = 8, count_min = 8, count_max = 8, item_default = ModeItem::at))]
    pub item: GArray<ModeItem>,
}

impl LocalExtraMode {
    fn at(index: usize) -> Self {
        let mut value = <Self as crate::gameexe::GameexeNode>::gameexe_default();
        value.header.3 = format!("ローカル汎用モード{}番", fullwidth(index));
        value
    }
}

#[gameexe]
#[derive(Debug)]
pub struct ModeItem {
    pub str: String,
}

impl ModeItem {
    pub(crate) fn at(index: usize) -> Self {
        Self {
            str: format!("モード{}", fullwidth(index)),
        }
    }
}
