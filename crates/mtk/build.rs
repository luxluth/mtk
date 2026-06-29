use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/c/muse.h");
    println!("cargo:rerun-if-changed=src/c/muse.c");

    cc::Build::new()
        .file("src/c/muse.c")
        .include("src/c")
        .opt_level(3)
        .compile("libmuse");

    let bindings = bindgen::Builder::default()
        .header("src/c/muse.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .layout_tests(false)
        .blocklist_item("FP_NAN")
        .blocklist_item("FP_INFINITE")
        .blocklist_item("FP_ZERO")
        .blocklist_item("FP_SUBNORMAL")
        .blocklist_item("FP_NORMAL")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");
}
