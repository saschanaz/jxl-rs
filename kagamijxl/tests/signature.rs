use kagamijxl::check_signature;
use libjxl_sys::JXL_SIG_CODESTREAM;

#[test]
fn test_signature_check() {
    // Resolve path manually or it will fail when running each test
    let sample_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/sample.jxl");

    let data = std::fs::read(sample_path).expect("Failed to read the sample image");
    let result = check_signature(&data);

    assert_eq!(result, JXL_SIG_CODESTREAM);
}
