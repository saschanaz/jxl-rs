use std::{fs::File, io::BufReader, path::PathBuf};

use kagamijxl::{decode_memory, Decoder, JxlError};
use libjxl_sys::JXL_ORIENT_IDENTITY;

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

fn sample_image_path() -> PathBuf {
    // Resolve path manually or it will fail when running each test
    PathBuf::from(MANIFEST_DIR).join("tests/resources/sample.jxl")
}

fn get_sample_image() -> Vec<u8> {
    std::fs::read(sample_image_path()).expect("Failed to read the sample image")
}

fn get_sample_image_file() -> File {
    File::open(sample_image_path()).expect("Failed to read the sample image")
}

fn get_sample_animation() -> Vec<u8> {
    // Resolve path manually or it will fail when running each test
    let sample_path = PathBuf::from(MANIFEST_DIR).join("tests/resources/spinfox.jxl");
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
}

#[test]
fn test_decode_default() {
    let data = get_sample_image();

    let result = Decoder::default()
        .decode(&data)
        .expect("Failed to decode the sample image");
    let basic_info = &result.basic_info;

    assert!(!result.is_partial());
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
fn test_decode_file() {
    let file = get_sample_image_file();

    let result = Decoder::default()
        .decode_file(&file)
        .expect("Failed to decode the sample image");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 1404);
    assert_eq!(basic_info.ysize, 936);
}

#[test]
fn test_decode_need_more_input() {
    let path = PathBuf::from(MANIFEST_DIR).join("tests/resources/needmoreinput.jxl");
    let file = File::open(path).expect("Failed to open the sample image");

    let result = Decoder::default()
        .decode_file(&file)
        .expect("Failed to decode the sample image");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 3264);
    assert_eq!(basic_info.ysize, 1836);
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
    decoder.stop_on_frame = true;

    let result = decoder
        .decode(&data)
        .expect("Failed to decode the sample image");

    assert_eq!(result.frames.len(), 1);
    assert_ne!(result.frames[0].data.len(), 0);
}

#[test]
fn test_decode_partial() {
    let data = get_sample_image();

    let mut decoder = Decoder::default();
    decoder.allow_partial = true;

    let mut result = decoder
        .decode(&data[..3])
        .expect("Failed to decode the sample image");

    assert!(result.is_partial());
    assert_eq!(result.frames.len(), 0);

    result
        .proceed(&data[3..40960], true, false)
        .expect("Should be able to proceed");

    assert!(result.is_partial());
    assert_eq!(result.frames.len(), 1);
    assert_ne!(result.frames[0].data.len(), 0);

    result
        .proceed(&data[40960..], true, false)
        .expect("Should be able to proceed");
    assert!(!result.is_partial());

    let err = result.proceed(&[0xff][..], true, false).unwrap_err();
    assert!(matches!(err, JxlError::AlreadyFinished));
}

#[test]
fn test_decode_partial_flush() {
    let data = get_sample_image();

    let mut decoder = Decoder::default();
    decoder.allow_partial = true;

    let mut result = decoder
        .decode(&data[..40960])
        .expect("Failed to decode the sample image");

    assert!(result.is_partial());
    assert_eq!(result.frames.len(), 1);

    {
        let first_frame_data = &result.frames[0].data;
        assert_ne!(first_frame_data.len(), 0);
        assert_eq!(first_frame_data[first_frame_data.len() - 10..], [0; 10]);
    }

    result.flush();
    {
        let first_frame_data = &result.frames[0].data;
        assert_ne!(first_frame_data[first_frame_data.len() - 10..], [0; 10]);
    }
}

#[test]
fn test_decode_partial_buffer() {
    let data = get_sample_image();

    let mut buffer = BufReader::new(&data[..40960]);

    let mut decoder = Decoder::default();
    decoder.allow_partial = true;

    let result = decoder
        .decode_buffer(&mut buffer)
        .expect("Failed to decode the sample image");

    assert!(result.is_partial());
    assert_eq!(result.frames.len(), 1);
    assert_ne!(result.frames[0].data.len(), 0);
    assert_eq!(buffer.buffer().len(), 0, "Buffer should be consumed");
}

#[test]
fn test_decode_partial_fail() {
    let data = get_sample_image();

    let err = decode_memory(&data[..40960]).unwrap_err();
    assert!(matches!(err, JxlError::InputNotComplete));
}

#[test]
fn test_decode_partial_fail_buffer() {
    let err = decode_memory(&[0xff, 0x0a]).unwrap_err();
    assert!(matches!(err, JxlError::InputNotComplete));
}
