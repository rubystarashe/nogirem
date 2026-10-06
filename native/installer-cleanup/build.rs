fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    winresource::WindowsResource::new()
        .set("ProductName", "마비노기 렘 부스터")
        .set("FileDescription", "마비노기 렘 부스터 설치 정리 도구")
        .set("FileVersion", env!("CARGO_PKG_VERSION"))
        .set("ProductVersion", env!("CARGO_PKG_VERSION"))
        .set("InternalName", "nogirem-installer-cleanup")
        .set("OriginalFilename", "installer-cleanup.exe")
        .set("LegalCopyright", "Copyright (C) 2026 Nogirem contributors")
        .set_icon("../../icon.ico")
        .compile()
        .expect("Windows 리소스를 생성하지 못했습니다");
}
