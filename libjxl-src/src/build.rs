use cmake::Config;
use std::env;
use std::path::Path;

pub fn build() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("submodules/libjxl");
    let mut config = Config::new(path);
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
        .define("JPEGXL_ENABLE_EXAMPLES", "OFF")
        .build();
}
