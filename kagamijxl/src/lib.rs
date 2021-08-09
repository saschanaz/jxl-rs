mod contiguous_buffer;
mod coupled_bufread;
mod decode;
mod encode;
pub use decode::{DecodeResult, Decoder, Frame};
pub use encode::{BitmapFrame, Encoder, JpegFrame};
pub use libjxl_sys::JxlBasicInfo as BasicInfo;

pub fn decode_memory(data: &[u8]) -> Result<DecodeResult, &'static str> {
    let decoder = Decoder::default();
    decoder.decode(data)
}

pub fn check_signature(data: &[u8]) -> libjxl_sys::JxlSignature {
    unsafe { libjxl_sys::JxlSignatureCheck(data.as_ptr(), data.len()) }
}

pub fn encode_memory(data: &[u8], xsize: usize, ysize: usize) -> Result<Vec<u8>, &'static str> {
    let mut encoder = Encoder::default();
    encoder.basic_info.xsize = xsize as u32;
    encoder.basic_info.ysize = ysize as u32;
    encoder.encode(data)
}
