fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("FileDescription", "A blazing-fast, lightweight, and cross-platform (Windows, macOS, Linux) Discord Rich Presence (RPC) and OBS Overlay client for T_Music_Bot, rewritten in native Rust for maximum performance and efficiency.");
        res.set("ProductName", "T_Music_Bot RPC");
        res.set("CompanyName", "T_Bot Team");
        res.set("LegalCopyright", "Copyright © 2026 T_Bot Team");
        res.set("InternalName", "t-music-bot-rpc");
        res.compile().unwrap();
    }
}
