//! На Windows вшивает в trolllauncher.exe иконку и описание файла.
fn main() {
    println!("cargo::rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico").set("FileDescription", "TrollLauncher").set("ProductName", "TrollLauncher");
        if let Err(e) = res.compile() {
            println!("cargo::warning=иконка exe не вшита: {e}");
        }
    }
}
