# libjxl-src

Builds the bundled [JPEG XL reference library](https://gitlab.com/wg1/jpeg-xl) 0.2 (version 945ad0ce, 2021-01-22).

Build requires GCC/Clang and CMake, while Windows additionally requires MSVC, Clang, and Ninja.

## Note

The crate builds instantly by default, but a build dependency only builds in dev profile. To do a release build, use `default-features = false` in Cargo.toml and call `libjxl_src::build()` in your `build.rs`.
