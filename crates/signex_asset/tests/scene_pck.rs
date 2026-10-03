use signex_asset::scene::decode_scene;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

fn sample_root(name: &str) -> PathBuf {
    let variable = format!("SIGNEX_SCENE_SAMPLE_{}", name.to_ascii_uppercase());
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
    let bytes = value
        .get("key")?
        .as_array()?
        .iter()
        .filter_map(|value| value.as_integer())
        .map(|value| value as u8)
        .collect::<Vec<_>>();
    bytes.try_into().ok()
}

fn parse_sample(name: &str) {
    let root = sample_root(name);
    let scene_path = root.join("Scene.pck");
    let key_path = root.join("key.toml");
    if !scene_path.is_file() || !key_path.is_file() {
        eprintln!(
            "skipping {name}: expected {} and {}",
            scene_path.display(),
            key_path.display()
        );
        return;
    }

    let key = key_from_toml(&key_path).expect("key.toml must contain key = [16 byte values]");
    let bytes = fs::read(&scene_path).expect("Scene.pck must be readable");
    let bytes_len = bytes.len();
    let start = Instant::now();
    let pack = decode_scene(&bytes, Some(&key))
        .unwrap_or_else(|error| panic!("failed to decode {}: {error}", scene_path.display()));
    let elapsed = start.elapsed();
    assert!(pack.scene_count() > 0, "{name} Scene.pck has no scenes");
    println!(
        "decoded {name}: {} scenes, {} bytes, {:?}",
        pack.scene_count(),
        bytes_len,
        elapsed
    );
}

#[test]
fn decodes_anemoi_scene_pck_when_available() {
    parse_sample("Anemoi");
}

#[test]
fn decodes_rewrite_scene_pck_when_available() {
    parse_sample("Rewrite");
}
