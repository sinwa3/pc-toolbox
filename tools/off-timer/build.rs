// Embeds assets/icon.ico as the exe icon (Explorer, Start menu shortcut).
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("FileDescription", "끄기 타이머");
        res.set("ProductName", "끄기 타이머");
        res.compile().expect("failed to embed the icon (needs rc.exe from the Windows SDK)");
    }
}
