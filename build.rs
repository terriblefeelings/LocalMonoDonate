fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        let _ = res.set_icon("icon.ico");
        res.set("ProductName", "LocalMonoDonate");
        res.set("FileDescription", "LocalMonoDonate");
        res.set("OriginalFilename", "LocalMonoDonate.exe");
        let _ = res.compile();
    }
}
