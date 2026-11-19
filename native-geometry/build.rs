use std::{env, fs, path::Path};

const TOKEN: &str = "--shell-header-height:";

fn main() {
    let tokens = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("../browser/tokens.css");
    println!("cargo:rerun-if-changed={}", tokens.display());
    let css = fs::read_to_string(&tokens)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", tokens.display()));
    let values: Vec<f64> = css
        .lines()
        .filter_map(|line| line.trim().strip_prefix(TOKEN))
        .map(|value| {
            value
                .trim()
                .strip_suffix("px;")
                .and_then(|number| number.trim().parse().ok())
                .unwrap_or_else(|| panic!("{TOKEN} must be a pixel length, found {value:?}"))
        })
        .collect();
    let [height] = values[..] else {
        panic!(
            "{} must declare {TOKEN} exactly once, found {}",
            tokens.display(),
            values.len()
        );
    };
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("tokens.rs");
    fs::write(out, format!("pub const HEADER_HEIGHT: f64 = {height:?};\n")).unwrap();
}
