use signex_asset::gameexe::{GameexeDecodeError, decrypt_and_parse_gameexe, model::Gameexe};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn sample_root(name: &str) -> PathBuf {
    let variable = format!("SIGNEX_GAMEEXE_SAMPLE_{}", name.to_ascii_uppercase());
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..\\..\\samples")
                .join(name)
        })
}

fn key_from_toml(path: &Path) -> Option<[u8; 16]> {
    let source = fs::read_to_string(path).ok()?;
    let value: toml::Value = toml::from_str(&source).ok()?;
    let array = value.get("key")?.as_array()?;
    let bytes = array.iter().filter_map(|v| v.as_integer()).map(|i| i as u8).collect::<Vec<_>>();
    bytes.try_into().ok()
}

fn parse_sample(name: &str) {
    let start = std::time::Instant::now();
    let root = sample_root(name);
    let gameexe_path = root.join("Gameexe.dat");
    let key_path = root.join("key.toml");
    if !gameexe_path.is_file() || !key_path.is_file() {
        eprintln!(
            "skipping {name}: expected {} and {}",
            gameexe_path.display(),
            key_path.display()
        );
        return;
    }
    let key = key_from_toml(&key_path).expect("key.toml must contain key = [16 byte values]");
    let bytes = fs::read(&gameexe_path).expect("Gameexe.dat must be readable");
    let gameexe: Gameexe = decrypt_and_parse_gameexe(&bytes, Some(&key))
        .unwrap_or_else(|error| panic!("failed to decode {}: {error}", gameexe_path.display()));
    assert!(!gameexe.gamename.is_empty(), "{name} has no game name");
    println!("parsed {name} in {:?}", start.elapsed());
}

#[test]
fn parses_anemoi_gameexe_dat_when_available() {
    parse_sample("Anemoi");
}

#[test]
fn parses_rewrite_gameexe_dat_when_available() {
    parse_sample("Rewrite");
}

#[test]
fn missing_key_is_reported() {
    let error = GameexeDecodeError::MissingKey;
    assert_eq!(
        error.to_string(),
        "Gameexe.dat requires the 16-byte EXE_EL key"
    );
}
