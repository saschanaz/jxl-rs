use std::env;
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
    let target = env::var("TARGET").unwrap();
    let clang = if target.contains("msvc") {
        // MSVC is not supported, force clang
        ("clang-cl", "clang-cl")
    } else {
        ("clang", "clang++")
    };

    Config::new("submodules/jpeg-xl")
        .define("CMAKE_C_COMPILER", clang.0)
        .define("CMAKE_CXX_COMPILER", clang.1)
        .generator("Ninja")
        .define("JPEGXL_STATIC", "ON")
        .define("BUILD_TESTING", "OFF")
        .build();
}

fn main() {
    // TODO: Add libgif/libjpeg/libpng/zlib
    submodule_update();
    run_cmake();
}
