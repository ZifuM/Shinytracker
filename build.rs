//! Embeds assets/icon.ico into the Windows .exe (Explorer, Start, taskbar pins).
//! To change the icon, replace assets/icon.ico and assets/icon-64.png.
//! Also bundles assets/wiki.json (the Pokédex data; build.bat downloads a
//! fresh copy). Without one, an empty placeholder is bundled and the app
//! downloads the data itself on first start.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/wiki.json");
    println!("cargo:rerun-if-changed=build.rs");

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("wiki.json");
    let data = std::fs::read("assets/wiki.json").unwrap_or_else(|_| {
        println!("cargo:warning=assets/wiki.json not found: the app will download the Pokedex data on first start.");
        b"{}".to_vec()
    });
    std::fs::write(&out, data).expect("write bundled wiki.json");

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
