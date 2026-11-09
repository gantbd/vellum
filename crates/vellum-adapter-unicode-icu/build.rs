use std::process::Command;

fn main() {
    for name in [
        "PKG_CONFIG_PATH",
        "PKG_CONFIG_LIBDIR",
        "PKG_CONFIG_SYSROOT_DIR",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    println!("cargo:rerun-if-changed=build.rs");
    let output = Command::new("pkg-config")
        .args(["--modversion", "icu-i18n"])
        .output()
        .expect("native Unicode provider requires pkg-config and explicitly selected ICU4C 79.1 headers/libraries");
    assert!(
        output.status.success(),
        "ICU4C configuration unavailable; select the qualified installation with PKG_CONFIG_PATH"
    );
    assert_eq!(
        std::str::from_utf8(&output.stdout).unwrap().trim(),
        "79.1",
        "unsupported native ICU version; this profile requires ICU4C 79.1"
    );
}
