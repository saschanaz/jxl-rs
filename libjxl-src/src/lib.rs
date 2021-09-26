mod build;
pub use build::build;

pub fn out_dir() -> &'static str {
    std::env!("OUT_DIR")
}

pub fn print_cargo_link() {
    print_cargo_link_from(out_dir())
}

/**
 * @param dst Pass OUT_DIR environment variable value.
 */
pub fn print_cargo_link_from(dst: &str) {
    #[cfg(all(windows, debug_assertions))]
    // Prevents "undefined symbol _CrtDbgReport" linker error
    println!("cargo:rustc-link-lib=dylib=msvcrtd");

    println!("cargo:rustc-link-search=native={}/lib", dst);
    println!("cargo:rustc-link-search=native={}/build/third_party", dst);
    println!(
        "cargo:rustc-link-search=native={}/build/third_party/brotli",
        dst
    );
    println!(
        "cargo:rustc-link-search=native={}/build/third_party/highway",
        dst
    );

    if cfg!(windows) {
        println!("cargo:rustc-link-lib=static=jxl-static");
        println!("cargo:rustc-link-lib=static=jxl_threads-static");
    } else {
        println!("cargo:rustc-link-lib=static=jxl");
        println!("cargo:rustc-link-lib=static=jxl_threads");
    }
    println!("cargo:rustc-link-lib=static=brotlicommon-static");
    println!("cargo:rustc-link-lib=static=brotlidec-static");
    println!("cargo:rustc-link-lib=static=brotlienc-static");
    println!("cargo:rustc-link-lib=static=hwy");

    #[cfg(not(windows))]
    // The order matters; this should be after other libs or the linker fails
    println!("cargo:rustc-link-lib=dylib=stdc++");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_print() {
        super::print_cargo_link();
    }
}
