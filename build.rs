//! Embeds assets/icon.ico into the Windows .exe (Explorer, Start, taskbar pins).
//! To change the icon, replace assets/icon.ico and assets/icon-64.png.
//! Also bundles assets/wiki.json (the Pokédex data; build.bat downloads a
//! fresh copy). Without one, an empty placeholder is bundled and the app
//! downloads the data itself on first start.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/wiki.json");
    println!("cargo:rerun-if-changed=build.rs");

    // The version the app shows and compares: Cargo's "0.6.1-2" (Cargo only
    // takes three numbers) is shown as "0.6.1.2", the rare grass data builds,
    // and a hotfix's "0.7.2-a" as "0.7.2a" (a lowercase letter on the patch
    // number: a, b, c… in order).
    let v = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let shown = match v.rsplit_once('-') {
        Some((base, l)) if l.len() == 1 && l.chars().all(|c| c.is_ascii_lowercase()) => format!("{base}{l}"),
        _ => v.replace('-', "."),
    };
    println!("cargo:rustc-env=APP_VERSION={shown}");

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
