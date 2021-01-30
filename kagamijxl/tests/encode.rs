use kagamijxl::{decode_memory, encode_memory, Encoder};

#[rustfmt::skip]
const RGBA_DATA: [u8; 36] = [
    0x25, 0xae, 0x8e, 0x05, 0xa2, 0xad, 0x9c, 0x6c, 0xb0, 0xc1, 0xd7, 0x7c,
    0xf3, 0xa6, 0x34, 0xed, 0xb7, 0x8c, 0xda, 0x80, 0xd0, 0x2d, 0x7e, 0xda,
    0x48, 0x5a, 0xf7, 0x62, 0xce, 0xd8, 0x38, 0x35, 0x24, 0xd1, 0x33, 0xe9,
];

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
    let encoded = Encoder::default()
        .encode(&RGBA_DATA, 3, 3)
        .expect("Failed to encode");

    let result = decode_memory(&encoded).expect("Failed to decode again");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 3);
    assert_eq!(basic_info.ysize, 3);
}

#[test]
fn test_encode_new() {
    let encoded = Encoder::new()
        .encode(&RGBA_DATA, 3, 3)
        .expect("Failed to encode");

    let result = decode_memory(&encoded).expect("Failed to decode again");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 3);
    assert_eq!(basic_info.ysize, 3);
}

#[test]
fn test_encode_lossless() {
    let mut encoder = Encoder::default();
    encoder.lossless = Some(true);

    let encoded = encoder.encode(&RGBA_DATA, 3, 3).expect("Failed to encode");

    let result = decode_memory(&encoded).expect("Failed to decode again");
    let basic_info = &result.basic_info;

    assert_eq!(basic_info.xsize, 3);
    assert_eq!(basic_info.ysize, 3);
    assert_eq!(RGBA_DATA, result.frames[0].data[..]);
}
