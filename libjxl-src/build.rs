fn main() {
    #[cfg(feature = "instant-build")]
    {
        #[path = "src/build.rs"]
        mod build;
        build::build()
    }
}
