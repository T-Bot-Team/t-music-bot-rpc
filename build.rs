fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("FileDescription", "A blazing-fast, lightweight, and cross-platform (Windows, macOS, Linux) Discord Rich Presence (RPC) and OBS Overlay client for T_Music_Bot, rewritten in native Rust for maximum performance and efficiency.");
        res.set("ProductName", "T_Music_Bot RPC");
        res.set("CompanyName", "T_Bot Team");
        res.set("LegalCopyright", "Copyright © 2026 T_Bot Team");
        res.set("InternalName", "t-music-bot-rpc");
        res.set("FileVersion", "2.1.0.0");
        res.set("ProductVersion", "2.1.0.0");
        res.set_version_info(winres::VersionInfo::FILEVERSION, 0x0002_0001_0000_0000);
        res.set_version_info(winres::VersionInfo::PRODUCTVERSION, 0x0002_0001_0000_0000);

        // Embedding the icon/version needs a Windows resource compiler
        // (rc.exe on Windows hosts). Cross `cargo check`s from Linux/macOS
        // don't have one — warn and continue there, so the Windows target
        // can still be type-checked from any OS. Native Windows builds stay
        // strict: a broken toolchain must not ship a resource-less exe.
        let host = std::env::var("HOST").unwrap_or_default();
        if let Err(e) = res.compile() {
            if host.contains("windows") {
                panic!("failed to embed Windows resources: {e}");
            }
            println!("cargo:warning=Windows resources skipped while cross-checking: {e}");
        }
    }
}
