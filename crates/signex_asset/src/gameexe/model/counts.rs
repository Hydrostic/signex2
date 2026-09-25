//! Count-only configuration groups and save slot settings.

use crate::gameexe::gameexe;

macro_rules! count_group {
    ($name:ident, $default:expr, $min:expr, $max:expr) => {
        #[gameexe]
        #[derive(Debug)]
        pub struct $name {
            #[gameexe(default = $default, min = $min, max = $max)]
            pub cnt: i32,
        }
    };
}

count_group!(QuickSaveCount, 3, 0, 10000);
count_group!(EndSaveCount, 0, 0, 1);
count_group!(InnerSaveCount, 0, 0, 100);
count_group!(SaveHistoryCount, 100, 0, 10000);
count_group!(SelSaveCount, 1, 1, 100);
count_group!(FlagCount, 1000, 0, 10000);
count_group!(GlobalFlagCount, 1000, 0, 10000);
count_group!(CallFlagCount, 50, 0, 256);
count_group!(CounterCount, 16, 0, 256);
count_group!(PcmchCount, 16, 0, 256);
count_group!(PcmEventCount, 16, 0, 256);
count_group!(EffectCount, 4, 0, 256);
count_group!(QuakeCount, 16, 0, 256);
count_group!(FrameActionCount, 4, 0, 256);
count_group!(EditboxCount, 4, 0, 256);
count_group!(G00BufferCount, 16, 0, 256);
count_group!(MaskCount, 16, 0, 256);
count_group!(ObjectButtonGroupCount, 4, 0, 256);

#[gameexe]
#[derive(Debug)]
pub struct SaveConfig {
    #[gameexe(default = 10, min = 0, max = 10000)]
    pub cnt: i32,
    #[gameexe(default = String::from("データがありません。"))]
    pub no_data_str: String,
}

#[gameexe]
#[derive(Debug)]
pub struct MessageBackSaveConfig {
    #[gameexe(default = 0, min = 0, max = 10000)]
    pub cnt: i32,
    #[gameexe(default = 1)]
    pub interval: i32,
}
