use kagamijxl::decode_memory;
use libjxl_sys::JXL_ORIENT_IDENTITY;

#[test]
fn test_decode_memory() {
    // Resolve path manually or it will fail when running each test
    let sample_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/sample.jxl");

    let data = std::fs::read(sample_path).expect("Failed to read the sample image");
    let (basic_info, _, _) = decode_memory(&data).expect("Failed to decode the sample image");

    assert_eq!(basic_info.xsize, 1404);
    assert_eq!(basic_info.ysize, 936);
    assert_eq!(basic_info.have_container, 0);
    assert_eq!(basic_info.orientation, JXL_ORIENT_IDENTITY);
}
