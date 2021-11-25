#[cfg(not(feature = "bindgen"))]
fn main() {
    #[cfg(not(feature = "rest"))]
    // nothing to do.
    println!("cargo:rustc-link-lib=taos");
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-search=C:\\TDengine\\driver");
    }
}

#[cfg(feature = "bindgen")]
fn main() {
    use std::env;
    use std::path::PathBuf;

    println!("cargo:rustc-link-lib=taos");

    println!("cargo:rerun-if-changed=wrapper.h");

    let bindings = if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-search=C:\\TDengine\\driver");
        bindgen::Builder::default()
            .header("wrapper.h")
            .clang_args(&["-IC:\\TDengine\\include"])
            .parse_callbacks(Box::new(bindgen::CargoCallbacks))
            .generate()
            .expect("Unable to generate bindings")
    } else {
        bindgen::Builder::default()
            .header("wrapper.h")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks))
            .generate()
            .expect("Unable to generate bindings")
    };

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
