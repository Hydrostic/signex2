use signex_asset::gameexe::{GArray, GameexeError, GameexeNode, gameexe, parse};
use std::time::Instant;

#[gameexe]
#[derive(Debug)]
struct Gameexe {
    syscommenu: SyscomMenu,
    #[gameexe(array(count = "CNT", default_count = 16, count_min = 0, count_max = 256))]
    selbtn: GArray<Selbtn>,
    #[gameexe(array(
        count = "CNT",
        default_count = 256,
        count_min = 0,
        count_max = 1024,
        range = true
    ))]
    object: GArray<Object>,
    #[gameexe(array(count = "CNT", default_count = 16, count_min = 0, count_max = 256))]
    shake: GArray<Vec<(i32, i32, i32)>>,
}

#[gameexe]
#[derive(Debug)]
struct SyscomMenu {
    #[gameexe(default_with = default_read_skip, validate_with = check_read_skip)]
    read_skip: (i32, i32, String),
    #[gameexe(array(count = "CNT", default_count = 4, count_min = 4, count_max = 4))]
    local_extra_mode: GArray<LocalExtraMode>,
}

#[gameexe(value_field = "header")]
#[derive(Debug)]
struct LocalExtraMode {
    #[gameexe(default = (1, 1, 0, String::new()))]
    header: (i32, i32, i32, String),
    #[gameexe(default = 3, min = 0, max = 16)]
    item_cnt: i32,
}

fn default_read_skip() -> (i32, i32, String) {
    (1, 1, "default".into())
}
fn check_read_skip(value: &(i32, i32, String)) -> Result<(), &'static str> {
    if (0..=1).contains(&value.0) && (0..=1).contains(&value.1) {
        Ok(())
    } else {
        Err("invalid flags")
    }
}

#[gameexe]
#[derive(Debug)]
struct Selbtn {
    #[gameexe(default = (25, 0, 0, 0), bounds = [(0, 255), (-128, 128), (-128, 128), (0, 255)])]
    moji_size: (i32, i32, i32, i32),
}

#[gameexe]
#[derive(Debug)]
struct Object {
    #[gameexe(name = "USE", default = 1, min = 0, max = 1)]
    use_flag: i32,
}

#[test]
fn defaults_and_sparse_index() {
    let game = parse::<Gameexe>("#SELBTN.CNT = 15\n#SELBTN.010.MOJI_SIZE = 30,0,0,0\n").unwrap();
    assert_eq!(game.selbtn.len(), 15);
    assert_eq!(game.selbtn.get(10).unwrap().moji_size, (30, 0, 0, 0));
    assert_eq!(game.selbtn.get(9).unwrap().moji_size, (25, 0, 0, 0));
    assert_eq!(game.syscommenu.read_skip.2, "default");
}

#[test]
fn tuple_array_range_and_order() {
    let game = parse::<Gameexe>("#SYSCOMMENU.READ_SKIP = 1, 1, \"既読\"\n#SYSCOMMENU.LOCAL_EXTRA_MODE.000=1,1,0,\"mode\"\n#SYSCOMMENU.LOCAL_EXTRA_MODE.000.ITEM_CNT=2\n#OBJECT.CNT=100\n#OBJECT.000-099.USE=0\n#OBJECT.070.USE=1\n#SHAKE.000=(0,8,64)(0,-8,64)\n").unwrap();
    assert_eq!(game.syscommenu.read_skip.2, "既読");
    assert_eq!(
        game.syscommenu.local_extra_mode.get(0).unwrap().header.3,
        "mode"
    );
    assert_eq!(game.syscommenu.local_extra_mode.get(0).unwrap().item_cnt, 2);
    assert_eq!(game.object.get(69).unwrap().use_flag, 0);
    assert_eq!(game.object.get(70).unwrap().use_flag, 1);
    assert_eq!(game.shake.get(0).unwrap(), &vec![(0, 8, 64), (0, -8, 64)]);
}

#[test]
fn count_and_index_bounds() {
    assert!(
        parse::<Gameexe>("#SELBTN.CNT=257\n")
            .unwrap_err()
            .message
            .contains("count")
    );
    assert!(
        parse::<Gameexe>("#SELBTN.CNT=-1\n")
            .unwrap_err()
            .message
            .contains("count")
    );
    assert!(
        parse::<Gameexe>("#SELBTN.CNT=15\n#SELBTN.015.MOJI_SIZE=30,0,0,0\n")
            .unwrap_err()
            .message
            .contains("index")
    );
    assert!(
        parse::<Gameexe>("#OBJECT.000-999.USE=0\n")
            .unwrap_err()
            .message
            .contains("index")
    );
}

#[test]
fn value_checks_and_comments() {
    let game =
        parse::<Gameexe>("/* before */\n#SELBTN.000.MOJI_SIZE=30,0,0,0 ; trailing\n// ignored\n")
            .unwrap();
    assert_eq!(game.selbtn.get(0).unwrap().moji_size.0, 30);
    assert!(
        parse::<Gameexe>("#SELBTN.000.MOJI_SIZE=300,0,0,0\n")
            .unwrap_err()
            .message
            .contains("bounds")
    );
    assert!(
        parse::<Gameexe>("#SYSCOMMENU.READ_SKIP=2,1,\"x\"\n")
            .unwrap_err()
            .message
            .contains("flags")
    );
    assert!(
        parse::<Gameexe>("#SELBTN.000.MOJI_SIZE=1,2,3\n")
            .unwrap_err()
            .message
            .contains("arity")
    );
}

#[test]
fn reference_lexer_reads_full_fixture() {
    if let Ok(path) = std::env::var("SIGNEX_GAMEEXE_FIXTURE") {
        let source = std::fs::read_to_string(path).unwrap();
        let started = Instant::now();
        let mut token_count = 0;
        for token in signex_asset::gameexe::Lexer::new(&source) {
            token.unwrap();
            token_count += 1;
        }
        eprintln!(
            "Gameexe.ini lexer: {} bytes, {token_count} tokens, {:?}",
            source.len(),
            started.elapsed()
        );
    }
    assert_eq!(Gameexe::gameexe_default().selbtn.len(), 16);
}

#[test]
fn parses_matching_entries_from_reference_file() {
    let Ok(path) = std::env::var("SIGNEX_GAMEEXE_FIXTURE") else {
        return;
    };
    let source = std::fs::read_to_string(path).unwrap();
    let keys = [
        "#SYSCOMMENU.READ_SKIP",
        "#SELBTN.CNT",
        "#SELBTN.000.MOJI_SIZE",
        "#OBJECT.CNT",
        "#OBJECT.000-099.USE",
        "#SHAKE.CNT",
        "#SHAKE.000",
    ];
    let selected: String = source
        .lines()
        .filter(|line| keys.iter().any(|key| line.trim_start().starts_with(key)))
        .map(|line| format!("{line}\n"))
        .collect();
    let game = parse::<Gameexe>(&selected).unwrap();
    assert_eq!(game.selbtn.len(), 15);
    assert_eq!(game.selbtn.get(0).unwrap().moji_size, (30, 0, 0, 0));
    assert_eq!(game.object.len(), 100);
    assert_eq!(game.object.get(99).unwrap().use_flag, 1);
    assert_eq!(game.shake.get(0).unwrap().len(), 5);
}

#[test]
fn source_model_parses_covered_reference_sections() {
    let Ok(path) = std::env::var("SIGNEX_GAMEEXE_FIXTURE") else {
        return;
    };
    let source = std::fs::read_to_string(path).unwrap();
    let prefixes = [
        "#GAMENAME",
        "#GAMEVERSION",
        "#DISCMARK",
        "#SCREEN_SIZE",
        "#START_SCENE",
        "#MENU_SCENE",
        "#CONFIG_SCENE",
        "#SAVE_SCENE",
        "#LOAD_SCENE",
        "#LOAD_AFTER_CALL",
        "#SYSCOMMENU.",
        "#SELBTN.",
        "#OBJECT.",
        "#SHAKE.",
        "#BGM.",
        "#ICON.",
        "#SE.",
        "#MSGBK.",
        "#MSGBK_ITEM.",
    ];
    let selected: String = source
        .lines()
        .filter(|line| {
            prefixes
                .iter()
                .any(|prefix| line.trim_start().starts_with(prefix))
        })
        .map(|line| format!("{line}\n"))
        .collect();
    let started = Instant::now();
    let game = parse::<signex_asset::gameexe::model::Gameexe>(&selected).unwrap();
    eprintln!(
        "Gameexe.ini model: {} bytes, {} entries, {:?}",
        selected.len(),
        selected.lines().count(),
        started.elapsed()
    );
    assert_eq!(game.gamename, "Rewrite+");
    assert_eq!(game.screen_size, (1280, 720));
    assert_eq!(game.selbtn.len(), 15);
    assert_eq!(game.selbtn.get(0).unwrap().moji_size, (30, 0, 0, 0));
    assert_eq!(game.object.len(), 100);
    assert_eq!(game.shake.get(0).unwrap().len(), 5);
    assert_eq!(game.bgm.get(1).unwrap().end_pos(), -1);
    assert_eq!(game.icon.get(0).unwrap().speed, 60);
    assert_eq!(game.msgbk.history_cnt, 256);
    assert_eq!(game.msgbk_item.slider.action, 1);
}

#[test]
fn source_model_defaults_and_bgm_rule() {
    let defaults = <signex_asset::gameexe::model::Gameexe as GameexeNode>::gameexe_default();
    assert_eq!(defaults.start_scene.name, "_start");
    assert_eq!(defaults.object.len(), 256);
    assert_eq!(defaults.system.extra_int_value.len(), 0);
    assert_eq!(defaults.system.extra_str_value.len(), 0);
    assert_eq!(defaults.selbtn.get(0).unwrap().line_width, 100);
    assert_eq!(defaults.selbtn.get(0).unwrap().moji_size, (25, 0, 0, 0));
    assert_eq!(defaults.bgm.len(), 32);
    assert_eq!(defaults.bgm.get(0).unwrap().end_pos(), -1);
    assert_eq!(
        defaults
            .syscommenu
            .local_extra_mode
            .get(0)
            .unwrap()
            .item
            .len(),
        8
    );

    let err =
        parse::<signex_asset::gameexe::model::Gameexe>("#BGM.000=\"x\",\"x\",0,0,0\n").unwrap_err();
    assert_eq!(err.path, "BGM.000");
    assert!(err.message.contains("end position"));
}

#[test]
fn source_model_preserves_legacy_updates() {
    let source = concat!(
        "#START_SCENE=\"first\",7\n",
        "#START_SCENE=\"second\"\n",
        "#OBJECT.005.USE=0\n",
        "#OBJECT.CNT=3\n",
        "#OBJECT.CNT=6\n",
        "#SHAKE.000=(1,2,3)\n",
        "#SHAKE.000=(4,5,6)\n",
    );
    let game = parse::<signex_asset::gameexe::model::Gameexe>(source).unwrap();
    assert_eq!(game.start_scene.name, "second");
    assert_eq!(game.start_scene.z, 7);
    assert!(!game.object.get(5).unwrap().use_flag);
    assert_eq!(game.shake.get(0).unwrap(), &vec![(1, 2, 3), (4, 5, 6)]);
}

#[test]
fn namae_accepts_legacy_five_and_extended_six_value_forms() {
    let gameexe: signex_asset::gameexe::model::Gameexe =
        parse("#NAMAE = \"A\", \"B\", 1, 2, 3\n#NAMAE = \"C\", \"D\", 4, 5, 6, 7\n").unwrap();
    assert_eq!(gameexe.namae.0[0].fuchi_color, -1);
    assert_eq!(gameexe.namae.0[1].fuchi_color, 7);
}

#[test]
fn source_model_limits_array_key_forms() {
    type SourceGameexe = signex_asset::gameexe::model::Gameexe;
    assert!(parse::<SourceGameexe>("#SELBTN.000-001.MOJI_COLOR=1\n").is_err());
    assert!(parse::<SourceGameexe>("#SYSCOMMENU.LOCAL_EXTRA_SWITCH.CNT=4\n").is_err());
    assert!(parse::<SourceGameexe>("#SYSCOMMENU.LOCAL_EXTRA_MODE.000.ITEM.CNT=8\n").is_err());
    assert!(parse::<SourceGameexe>("#FLICK_SCENE.000.SCENE=\"x\"\n").is_err());
    assert!(parse::<SourceGameexe>("#OBJECT.005-003.USE=0\n").is_ok());
}

#[test]
fn source_model_parses_full_reference_file() {
    let Ok(path) = std::env::var("SIGNEX_GAMEEXE_FIXTURE") else {
        return;
    };
    let source = std::fs::read_to_string(path).unwrap();
    let game = parse::<signex_asset::gameexe::model::Gameexe>(&source).unwrap();
    assert_eq!(game.gamename, "Rewrite+");
}

#[test]
fn gameexe_error_keeps_its_diagnostic_format() {
    let error = GameexeError::new(3..8, "BGM.000", "invalid value");
    assert_eq!(error.to_string(), "BGM.000 at byte 3 (8): invalid value");
}

#[test]
fn accepts_original_path_spacing_signs_and_comment_splicing() {
    let game = parse::<Gameexe>(
        "#SEL/*comment*/BTN . CNT = + 15\n#SELBTN . 010 . MOJI_SIZE = +30,0,0,0\n#OBJECT . 000 - 099 . USE = 1\n",
    ).unwrap();
    assert_eq!(game.selbtn.len(), 15);
    assert_eq!(game.selbtn.get(10).unwrap().moji_size.0, 30);
    assert_eq!(game.object.get(99).unwrap().use_flag, 1);
    assert_eq!(
        parse::<Gameexe>("# SELBTN.CNT=15\n").unwrap_err().message,
        "space after #"
    );
    assert!(
        parse::<Gameexe>("#SEL BTN.CNT=15\n")
            .unwrap_err()
            .message
            .contains("space inside path")
    );
}

#[test]
fn bare_carriage_return_does_not_start_another_entry() {
    assert_eq!(
        parse::<Gameexe>("#SELBTN.CNT=15\r").unwrap().selbtn.len(),
        15
    );
    assert_eq!(
        parse::<Gameexe>("#SELBTN.CNT=15\r#SELBTN.CNT=16")
            .unwrap()
            .selbtn
            .len(),
        15
    );
    assert_eq!(
        parse::<Gameexe>("#SELBTN.CNT=15\r\n").unwrap().selbtn.len(),
        15
    );
}
