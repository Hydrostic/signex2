# Gameexe.ini structural parser

`#[gameexe]` generates ordered field dispatch and default construction. Pass decoded UTF-8 Gameexe.ini text to `parse::<T>()`; decoding `Gameexe.dat` is a separate asset step. Parsing stops at the first error and never publishes a partial `T`.

```rust
use signex_asset::gameexe::{gameexe, parse, GArray};

#[gameexe]
struct Gameexe {
    syscommenu: SyscomMenu,
    #[gameexe(array(count = "CNT", default_count = 16, count_min = 0, count_max = 256))]
    selbtn: GArray<Selbtn>,
}

#[gameexe]
struct SyscomMenu {
    #[gameexe(default = (1, 1, String::new()))]
    read_skip: (i32, i32, String),
}

#[gameexe]
struct Selbtn {
    #[gameexe(default = (25, 0, 0, 0))]
    moji_size: (i32, i32, i32, i32),
}
```

Names default to uppercase Rust field names. `name = "BACK_FILE"` overrides a name. Scalar values accept `min` and `max`; tuples accept `bounds = [(min, max), ...]`. `validate_with = path` calls a function of the form `fn(&T) -> Result<(), impl Display>`. `default_with = path` calls a zero-argument constructor. Without either default attribute, scalar and tuple fields use `Default`; nested `#[gameexe]` structs construct their declared defaults recursively.

`GArray<T>` starts at `default_count`. `CNT` changes its length within the inclusive `count_min..=count_max` limits. Each numeric key segment must be less than the current length. `000-099` applies to each indexed struct field in order; later entries may overwrite earlier ones. `item_default = path` accepts `fn(usize) -> T` for index-dependent defaults. An array of values such as `GArray<Vec<(i32, i32, i32)>>` accepts a direct key and consecutive parenthesized groups. A struct that has both a direct value and child paths can use `#[gameexe(value_field = "header")]`.

The macro implements the routing and declared checks. The schema author must still transcribe original defaults, bounds, field names, and special validators from the engine. Unknown keys are errors, so a partial schema cannot claim to parse an entire title file. The integration tests exercise selected entries from the Rewrite example file when `SIGNEX_GAMEEXE_FIXTURE` points to it.

`model.rs` and its companion modules contain the public `Gameexe` model for all top-level parser sections in the reviewed SiglusEngine source, including startup scenes and identity, configuration and dialogs, voice and name tables, message windows and history, buttons, objects, effects, fonts, shortcuts, WAKU/MWND data, BGM, and SE. It uses the Japanese localization table for SYSCOMMENU and dialog defaults. The integration suite parses the complete Rewrite fixture when `SIGNEX_GAMEEXE_FIXTURE` points to it; unknown keys remain errors so newly introduced engine fields are detected instead of silently discarded.

To see fixture scan and model parse times, run the integration test with `SIGNEX_GAMEEXE_FIXTURE` set to a Gameexe.ini path and pass `-- --nocapture` to `cargo test`. The reported intervals exclude file reading and selection of supported entries.
