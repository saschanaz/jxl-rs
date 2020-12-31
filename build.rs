extern crate bindgen;

use std::env;
use std::path::PathBuf;
use std::process::Command;
use cmake::Config;

fn submodule_update() {
    // --recursive because jpeg-xl also has its own submodules
    Command::new("git")
        .args(&["submodule", "update", "--init", "--recursive"])
        .output()
        .expect("Failed to update submodules");
}

fn run_cmake() {
    let clang = if cfg!(windows) {
        ("clang-cl", "clang-cl")
    } else {
        ("clang", "clang++")
    };

    // TODO: Add libgif/libjpeg/libpng/zlib
    let dst = Config::new("submodules/jpeg-xl")
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("CMAKE_C_COMPILER", clang.0)
        .define("CMAKE_CXX_COMPILER", clang.1)
        .generator("Ninja")
        .define("JPEGXL_STATIC", "ON")
        .define("BUILD_TESTING", "OFF")
        .build();
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-search=native={}/build/third_party", dst.display());
    println!("cargo:rustc-link-search=native={}/build/third_party/brotli", dst.display());
    println!("cargo:rustc-link-search=native={}/build/third_party/highway", dst.display());
    println!("cargo:rustc-link-lib=static=jxl-static");
    println!("cargo:rustc-link-lib=static=brotlicommon-static");
    println!("cargo:rustc-link-lib=static=brotlidec-static");
    println!("cargo:rustc-link-lib=static=brotlienc-static");
    println!("cargo:rustc-link-lib=static=hwy");
    println!("cargo:rustc-link-lib=static=skcms");
}

fn build_bindings() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .clang_arg("-I").clang_arg(out_path.join("include").to_str().unwrap())
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn main() {
    submodule_update();
    run_cmake();
    build_bindings();
}
