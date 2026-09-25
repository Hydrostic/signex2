//! Save/load and warning UI text configured by Gameexe.ini.

use crate::gameexe::gameexe;

#[gameexe]
#[derive(Debug)]
pub struct WarningInfo {
    #[gameexe(default = String::from("最初から始めてもよろしいですか？"))]
    pub restart_warning_str: String,
    #[gameexe(default = String::from("途中から始めてもよろしいですか？"))]
    pub scenestart_warning_str: String,
    #[gameexe(default = String::from("タイトルに戻ってもよろしいですか？"))]
    pub returnmenu_warning_str: String,
    #[gameexe(default = String::from("前の選択肢に戻ってもよろしいですか？"))]
    pub returnsel_warning_str: String,
    #[gameexe(default = String::from("終了してもよろしいですか？"))]
    pub gameend_warning_str: String,
}

#[gameexe]
#[derive(Debug)]
pub struct SaveLoadDialog {
    #[gameexe(default = 10, min = 1, max = 1000)]
    pub data_cnt_par_page: i32,
}

#[gameexe]
#[derive(Debug)]
pub struct SaveLoadInfo {
    pub dlgwnd_caption_title_str: String,
    pub dlgwnd_deside_button_str: String,
    pub dlgwnd_datalist_nameheader_str: String,
    pub dlgwnd_warning_chkbox_str: String,
    pub dlgwnd_dblclick_chkbox_str: String,
    pub warning_str: String,
    pub quick_warning_str: String,
    pub msgbk_warning_str: String,
}

impl SaveLoadInfo {
    pub fn save_defaults() -> Self {
        Self {
            dlgwnd_caption_title_str: "セーブ".into(),
            dlgwnd_deside_button_str: "セーブ".into(),
            dlgwnd_datalist_nameheader_str: "セーブ".into(),
            dlgwnd_warning_chkbox_str: "セーブする前に上書きの確認ウィンドウを表示する。".into(),
            dlgwnd_dblclick_chkbox_str: "リストをダブルクリックでセーブする。".into(),
            warning_str: "セーブしてもよろしいですか？".into(),
            quick_warning_str: "クイックセーブしてもよろしいですか？".into(),
            msgbk_warning_str: String::new(),
        }
    }

    pub fn load_defaults() -> Self {
        Self {
            dlgwnd_caption_title_str: "ロード".into(),
            dlgwnd_deside_button_str: "ロード".into(),
            dlgwnd_datalist_nameheader_str: "ロード".into(),
            dlgwnd_warning_chkbox_str: "ロードする前に確認ウィンドウを表示する。".into(),
            dlgwnd_dblclick_chkbox_str: "リストをダブルクリックでロードする。".into(),
            warning_str: "ロードしてもよろしいですか？".into(),
            quick_warning_str: "クイックロードしてもよろしいですか？".into(),
            msgbk_warning_str: "ロードしてもよろしいですか？".into(),
        }
    }
}
