use kagamijxl::{decode_memory, encode_memory, BitmapFrame, Encoder, JpegFrame};
use std::path::PathBuf;

#[rustfmt::skip]
const RGBA_DATA: [u8; 36] = [
    0x25, 0xae, 0x8e, 0x05, 0xa2, 0xad, 0x9c, 0x6c, 0xb0, 0xc1, 0xd7, 0x7c,
    0xf3, 0xa6, 0x34, 0xed, 0xb7, 0x8c, 0xda, 0x80, 0xd0, 0x2d, 0x7e, 0xda,
    0x48, 0x5a, 0xf7, 0x62, 0xce, 0xd8, 0x38, 0x35, 0x24, 0xd1, 0x33, 0xe9,
];

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

fn sample_jpeg_path() -> PathBuf {
    // Resolve path manually or it will fail when running each test
    PathBuf::from(MANIFEST_DIR).join("tests/sample.jpg")
}

fn get_sample_jpeg() -> Vec<u8> {
    std::fs::read(sample_jpeg_path()).expect("Failed to read the sample image")
}

#[test]
fn test_encode_memory() {
    let encoded = encode_memory(&RGBA_DATA, 3, 3).expect("Failed to encode");

    let result = decode_memory(&encoded).expect("Failed to decode again");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 3);
    assert_eq!(basic_info.ysize, 3);
}

#[test]
fn test_encode_default() {
    let mut encoder = Encoder::default();
    encoder.basic_info.xsize = 3;
    encoder.basic_info.ysize = 3;

    let encoded = encoder.encode(&RGBA_DATA).expect("Failed to encode");

    let result = decode_memory(&encoded).expect("Failed to decode again");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 3);
    assert_eq!(basic_info.ysize, 3);
}

#[test]
fn test_encode_new() {
    let encoded = encode_memory(&RGBA_DATA, 3, 3).expect("Failed to encode");

    let result = decode_memory(&encoded).expect("Failed to decode again");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 3);
    assert_eq!(basic_info.ysize, 3);
}

#[test]
fn test_encode_lossless() {
    let mut encoder = Encoder::default();
    encoder.lossless = Some(true);
    encoder.basic_info.xsize = 3;
    encoder.basic_info.ysize = 3;

    let encoded = encoder.encode(&RGBA_DATA).expect("Failed to encode");

    let result = decode_memory(&encoded).expect("Failed to decode again");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 3);
    assert_eq!(basic_info.ysize, 3);
    assert_eq!(result.frames[0].data[..], RGBA_DATA);
}

#[test]
fn test_encode_frame() {
    let mut encoder = Encoder::default();
    encoder.lossless = Some(true);
    encoder.basic_info.xsize = 3;
    encoder.basic_info.ysize = 3;

    let frame = BitmapFrame { data: &RGBA_DATA };
    let encoded = encoder.encode_frame(&frame).expect("Failed to encode");

    let result = decode_memory(&encoded).expect("Failed to decode again");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 3);
    assert_eq!(basic_info.ysize, 3);
    assert_eq!(result.frames.len(), 1);
    assert_eq!(result.frames[0].data[..], RGBA_DATA);
}

#[test]
fn test_encode_jpeg_frame() {
    let mut encoder = Encoder::default();
    encoder.basic_info.xsize = 800;
    encoder.basic_info.ysize = 533;
    encoder.basic_info.alpha_bits = 0; // TODO: this must be implied

    let frame = JpegFrame {
        data: &get_sample_jpeg()[..],
    };
    let encoded = encoder.encode_frame(&frame).expect("Failed to encode");

    let result = decode_memory(&encoded).expect("Failed to decode again");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 800);
    assert_eq!(basic_info.ysize, 533);
    assert_eq!(result.frames.len(), 1);
    assert_eq!(result.frames[0].data[0], 57);
}

// https://gitlab.com/wg1/jpeg-xl/-/issues/167
// This fails on C++ side and thus can't run
// #[test]
// fn test_encode_jpeg_frame_lossless() {
//     let mut encoder = Encoder::default();
//     encoder.lossless = Some(true);
//     encoder.basic_info.xsize = 800;
//     encoder.basic_info.ysize = 533;
//     encoder.basic_info.alpha_bits = 0;

//     let frame = JpegFrame {
//         data: &get_sample_jpeg()[..],
//     };
//     let encoded = encoder.encode_frame(&frame).expect("Failed to encode");

//     let result = decode_memory(&encoded).expect("Failed to decode again");
//     let basic_info = &result.basic_info;

//     assert_eq!(basic_info.xsize, 800);
//     assert_eq!(basic_info.ysize, 533);
//     assert_eq!(result.frames.len(), 1);
//     assert_eq!(result.frames[0].data[0], 57);
// }
