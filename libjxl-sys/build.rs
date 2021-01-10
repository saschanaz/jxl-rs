extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");
    libjxl_src::print_cargo_link();

    let include_dir = format!("{}/include", libjxl_src::out_dir());
    println!("cargo:include={}", include_dir);

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        // Tell where to find the jxl/ headers.
        .clang_arg("-I")
        .clang_arg(include_dir)
        // Reduce noise from system libs.
        .whitelist_function("Jxl.*")
        // #[derive(Default)] for struct initialization.
        .derive_default(true)
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
