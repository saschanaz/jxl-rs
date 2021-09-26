extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    libjxl_src::build();

    let out_dir = std::env::var("OUT_DIR").unwrap();

    println!("cargo:rerun-if-changed=wrapper.h");
    libjxl_src::print_cargo_link_from(&out_dir);

    let include_dir = format!("{}/include", out_dir);
    println!("cargo:include={}", include_dir);

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
        .allowlist_function("Jxl.*")
        // #[derive(Default)] for struct initialization.
        .derive_default(true)
        // `size_t` is `usize` almost everywhere
        // https://github.com/rust-lang/rust-bindgen/issues/1901
        .size_t_is_usize(true)
        // libjxl already adds appropriate prefixes
        .prepend_enum_name(false)
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
