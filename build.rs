fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Try pkg-config first
    if pkg_config::Config::new().atleast_version("1.4.0").probe("ibus-1.0").is_err() {
        // Fallback for systems where pkg-config file might be missing but libraries exist
        println!("cargo:rustc-link-lib=ibus-1.0");
        println!("cargo:rustc-link-lib=gobject-2.0");
        println!("cargo:rustc-link-lib=glib-2.0");
    }
}
