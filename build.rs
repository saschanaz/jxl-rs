use std::env;
use cmake::Config;

fn run_cmake() {
    let mut config = Config::new("submodules/jpeg-xl");
    config.define("JPEGXL_ENABLE_OPENEXR", "OFF");
    config.define("JPEGXL_ENABLE_BENCHMARK", "OFF");

    let target = env::var("TARGET").unwrap();
    if target.contains("msvc") {
        config
            // MSVC is not supported, force clang
            .define("CMAKE_C_COMPILER", "clang-cl")
            .define("CMAKE_CXX_COMPILER", "clang-cl")
            // Force Ninja or VS will ignore CMAKE_*_COMPILER
            .generator("Ninja");
    }

    config
        .define("JPEGXL_STATIC", "ON")
        .define("BUILD_TESTING", "OFF")
        .build();
}

fn main() {
    // TODO: Add libgif/libjpeg/libpng/zlib
    run_cmake();
}
