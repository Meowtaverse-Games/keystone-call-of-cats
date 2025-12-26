fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        // The path must be relative to the Cargo.toml file
        // We will assume the icon is generated at assets/icon.ico
        // Or we can point to a location in target/ or similar.
        // Let's use 'assets/icon.ico' as the standard location for the generated icon.
        // We will need to ensure this file exists before build.
        res.set_icon("assets/icon.ico");
        res.compile().unwrap();
    }
}
