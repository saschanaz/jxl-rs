mod decode;
mod encode;
pub use decode::{DecodeResult, Decoder, Frame, BasicInfo};
pub use encode::Encoder;

pub fn decode_memory(data: &[u8]) -> Result<DecodeResult, &'static str> {
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
