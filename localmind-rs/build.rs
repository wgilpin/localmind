fn main() {
    // Disable Windows resource compilation in debug mode
    if cfg!(debug_assertions) {
        println!("cargo:rustc-cfg=desktop");
    } else {
        tauri_build::build()
    }
}
