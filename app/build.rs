fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=src/fullscreen.m");
        cc::Build::new()
            .file("src/fullscreen.m")
            .flag("-fobjc-arc")
            .compile("player_fullscreen");
    }
    tauri_build::build()
}
