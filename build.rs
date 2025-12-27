use std::env;
use std::path::Path;

fn main() {
    if env::var("CARGO_FEATURE_EMBED_ASSETS").is_ok() {
        bevy_embedded_assets::embed_assets!();
    }

    if env::var("CARGO_CFG_TARGET_OS").ok().as_deref() == Some("windows") {
        let mut res = winres::WindowsResource::new();
        // The path must be relative to the Cargo.toml file
        if Path::new("assets/icon.ico").exists() {
            res.set_icon("assets/icon.ico");
        }

        if let Err(e) = res.compile() {
            println!("cargo:warning=Failed to compile Windows resources: {}", e);
        }
    }
}
