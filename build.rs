//! Embeds assets/icon.ico into the Windows .exe (Explorer, Start, taskbar pins).
//! To change the icon, replace assets/icon.ico and assets/icon-64.png.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            // Not fatal: the window still gets its icon at runtime.
            println!("cargo:warning=Could not embed the .exe icon ({e}). The app still works.");
        }
    }
}
