use std::env;
use cmake::Config;

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
    run_cmake();
}
