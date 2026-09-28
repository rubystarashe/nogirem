fn main() {
    let manifest_dir = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest_dir.parent().unwrap();
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let binary_dir = out_dir.ancestors().nth(3).unwrap();
    // Dioxus resolves startup assets beside the executable before component hooks
    // exist. Include only renderer assets, never backend source or user data.
    copy_assets(&root.join("public"), &binary_dir.join("public"));
    for file in [
        "styles.css",
        "update-preview.css",
        "blackbox-editor.css",
        "blackbox-editor.js",
        "window-escape.js",
        "smooth-wheel-scroll.mjs",
        "nexon-lv2-gothic.woff",
        "creator-channel-avatar.jpg",
        "direct-donation-logo.svg",
    ] {
        let source = root.join("web").join(file);
        let target = binary_dir.join("web").join(file);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(&source, target).unwrap();
        println!("cargo:rerun-if-changed={}", source.display());
    }
    std::fs::copy(
        root.join("blackbox-editor.html"),
        binary_dir.join("blackbox-editor.html"),
    )
    .unwrap();
    println!(
        "cargo:rerun-if-changed={}",
        root.join("blackbox-editor.html").display()
    );
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let icon = root.join("icon.ico");
        let rc = out_dir.join("app-icon.rc");
        let res = out_dir.join("app-icon.res");
        std::fs::write(&rc, format!("1 ICON \"{}\"\n", icon.display().to_string().replace('\\', "/"))).unwrap();
        let kits = std::path::PathBuf::from(std::env::var_os("ProgramFiles(x86)").expect("Windows SDK location")).join("Windows Kits/10/bin");
        let mut compilers: Vec<_> = std::fs::read_dir(&kits).expect("Windows SDK installed").filter_map(Result::ok).map(|e|e.path().join("x64/rc.exe")).filter(|p|p.is_file()).collect();
        compilers.sort();
        let compiler = compilers.last().expect("Windows resource compiler");
        assert!(std::process::Command::new(compiler).arg("/nologo").arg("/fo").arg(&res).arg(&rc).status().unwrap().success(), "Icon resource compilation failed");
        println!("cargo:rustc-link-arg={}",res.display());
        println!("cargo:rerun-if-changed={}",icon.display());
        let source = manifest_dir.join("windows.manifest");
        let manifest = out_dir.join("windows.manifest");
        let contents = std::fs::read_to_string(source).unwrap();
        std::fs::write(&manifest, contents).unwrap();
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTUAC:NO");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        println!("cargo:rerun-if-changed=windows.manifest");
    }
}

fn copy_assets(source: &std::path::Path, target: &std::path::Path) {
    std::fs::create_dir_all(target).unwrap();
    println!("cargo:rerun-if-changed={}", source.display());
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_assets(&entry.path(), &destination);
        } else {
            std::fs::copy(entry.path(), destination).unwrap();
        }
    }
}
