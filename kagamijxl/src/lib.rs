mod contiguous_buffer;
mod coupled_bufread;
mod decode;
mod encode;
pub use decode::{DecodeProgress, Decoder, Frame, JxlDecodeError};
pub use encode::{BitmapFrame, Encoder, JpegFrame, JxlEncodeError};
pub use libjxl_sys::JxlBasicInfo as BasicInfo;

pub fn decode_memory(data: &[u8]) -> Result<DecodeProgress, JxlDecodeError> {
    let decoder = Decoder::default();
    decoder.decode(data)
}

pub fn check_signature(data: &[u8]) -> libjxl_sys::JxlSignature {
    unsafe { libjxl_sys::JxlSignatureCheck(data.as_ptr(), data.len()) }
}

pub fn encode_memory(data: &[u8], xsize: usize, ysize: usize) -> Result<Vec<u8>, JxlEncodeError> {
    let mut encoder = Encoder::default();
    encoder.basic_info.xsize = xsize as u32;
    encoder.basic_info.ysize = ysize as u32;
    encoder.encode(data)
}
