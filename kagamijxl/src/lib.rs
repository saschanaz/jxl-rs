mod decode;
mod encode;
pub use decode::Decoder;
pub use encode::Encoder;
pub use libjxl_sys::JxlBasicInfo;

pub fn decode_memory(data: &[u8]) -> Result<(JxlBasicInfo, Vec<u8>, Vec<u8>), &'static str> {
    let decoder = Decoder::default();
    decoder.decode(&data)
}

pub fn check_signature(data: &[u8]) -> libjxl_sys::JxlSignature {
    unsafe { libjxl_sys::JxlSignatureCheck(data.as_ptr(), data.len()) }
}

pub fn encode_memory(data: &[u8], xsize: usize, ysize: usize) -> Result<Vec<u8>, &'static str> {
    let encoder = Encoder::default();
    encoder.encode(&data, xsize, ysize)
}
