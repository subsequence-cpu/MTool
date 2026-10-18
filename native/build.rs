fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        build_finder_extension();
    }
    tauri_build::build();
}

fn build_finder_extension() {
    use std::{fs, path::PathBuf, process::Command};

    for file in ["FinderSync.swift", "Info.plist", "Entitlements.plist"] {
        println!("cargo:rerun-if-changed=finder-extension/{file}");
    }
    println!("cargo:rerun-if-env-changed=APPLE_SIGNING_IDENTITY");
    let architecture = match std::env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        other => panic!("Unsupported Finder extension architecture: {other}"),
    };
    let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let contents = PathBuf::from("gen/finder/MToolFinder.appex/Contents");
    fs::create_dir_all(contents.join("MacOS")).expect("create Finder extension bundle");
    fs::copy("finder-extension/Info.plist", contents.join("Info.plist"))
        .expect("copy Finder extension Info.plist");
    let status = Command::new("xcrun")
        .args([
            "swiftc",
            "-target",
            &format!("{architecture}-apple-macos13.0"),
            "-module-cache-path",
        ])
        .arg(output.join("swift-cache"))
        .args([
            "-parse-as-library",
            "-application-extension",
            "-warnings-as-errors",
            "-O",
            "-emit-executable",
            "-Xlinker",
            "-e",
            "-Xlinker",
            "_NSExtensionMain",
            "finder-extension/FinderSync.swift",
            "-o",
        ])
        .arg(contents.join("MacOS/MToolFinder"))
        .status()
        .expect("run Swift compiler for Finder extension");
    assert!(status.success(), "Finder extension compilation failed");
    let identity = std::env::var("APPLE_SIGNING_IDENTITY").unwrap_or_else(|_| "-".into());
    let status = Command::new("codesign")
        .args([
            "--force",
            "--sign",
            &identity,
            "--options",
            "runtime",
            "--entitlements",
        ])
        .arg("finder-extension/Entitlements.plist")
        .arg(contents.parent().unwrap())
        .status()
        .expect("sign Finder extension");
    assert!(status.success(), "Finder extension signing failed");
}
