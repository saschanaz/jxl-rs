use std::path::PathBuf;

use kagamijxl::{decode_memory, Decoder};
use libjxl_sys::JXL_ORIENT_IDENTITY;

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

fn get_sample_image() -> Vec<u8> {
    // Resolve path manually or it will fail when running each test
    let sample_path = PathBuf::from(MANIFEST_DIR).join("tests/sample.jxl");
    std::fs::read(sample_path).expect("Failed to read the sample image")
}

fn get_sample_animation() -> Vec<u8> {
    // Resolve path manually or it will fail when running each test
    let sample_path = PathBuf::from(MANIFEST_DIR).join("tests/spinfox.jxl");
    std::fs::read(sample_path).expect("Failed to read the sample image")
}

#[test]
fn test_decode_memory() {
    let data = get_sample_image();

    let result = decode_memory(&data).expect("Failed to decode the sample image");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 1404);
    assert_eq!(basic_info.ysize, 936);
    assert_eq!(basic_info.have_container, 0);
    assert_eq!(basic_info.orientation, JXL_ORIENT_IDENTITY);
    assert_eq!(result.preview.len(), 0);
    assert_eq!(result.color_profile.len(), 0);
    assert_eq!(result.frames.len(), 1);
    assert_eq!(result.frames[0].name, "");
    assert_ne!(result.frames[0].data.len(), 0);
    assert_eq!(result.frames[0].dc.len(), 0);
}

#[test]
fn test_decode_default() {
    let data = get_sample_image();

    let result = Decoder::default()
        .decode(&data)
        .expect("Failed to decode the sample image");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 1404);
    assert_eq!(basic_info.ysize, 936);
}

#[test]
fn test_decode_new() {
    let data = get_sample_image();

    let result = Decoder::new()
        .decode(&data)
        .expect("Failed to decode the sample image");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 1404);
    assert_eq!(basic_info.ysize, 936);
}

#[test]
fn test_decode_no_frame() {
    let data = get_sample_image();

    let mut decoder = Decoder::default();
    decoder.no_full_frame = true;

    let result = decoder
        .decode(&data)
        .expect("Failed to decode the sample image");
    assert_eq!(result.frames.len(), 0);
}

#[test]
fn test_decode_dc_frame() {
    let data = get_sample_image();

    let mut decoder = Decoder::default();
    decoder.need_dc_frame = true;

    let result = decoder
        .decode(&data)
        .expect("Failed to decode the sample image");
    assert_eq!(result.frames.len(), 1);
    assert_ne!(result.frames[0].dc.len(), 0);
}

#[test]
fn test_decode_color_profile() {
    let data = get_sample_image();

    let mut decoder = Decoder::default();
    decoder.need_color_profile = true;

    let result = decoder
        .decode(&data)
        .expect("Failed to decode the sample image");
    assert_ne!(result.color_profile.len(), 0);
}

#[test]
fn test_decode_animation() {
    let data = get_sample_animation();

    let result = decode_memory(&data).expect("Failed to decode the sample image");
    assert_eq!(result.frames.len(), 25);
    for frame in result.frames {
        assert_ne!(frame.data.len(), 0);
    }
}

#[test]
fn test_decode_animation_first() {
    let data = get_sample_animation();

    let mut decoder = Decoder::default();
    decoder.max_frames = Some(1);

    let result = decoder
        .decode(&data)
        .expect("Failed to decode the sample image");

    assert_eq!(result.frames.len(), 1);
    assert_ne!(result.frames[0].data.len(), 0);
}
