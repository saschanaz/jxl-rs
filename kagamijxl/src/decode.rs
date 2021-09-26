use std::{
    ffi::c_void,
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{contiguous_buffer::ContiguousBuffer, BasicInfo};
use libjxl_sys::*;

macro_rules! try_dec {
    ($left:expr) => {{
        if $left != JXL_DEC_SUCCESS {
            return Err("Decoder failed");
        }
    }};
}

unsafe fn read_basic_info(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeResult,
) -> Result<(), &'static str> {
    // Get the basic info
    try_dec!(JxlDecoderGetBasicInfo(dec, &mut result.basic_info));
    Ok(())
}

unsafe fn read_color_encoding(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeResult,
    pixel_format: &JxlPixelFormat,
) -> Result<(), &'static str> {
    // Get the ICC color profile of the pixel data
    let mut icc_size = 0usize;
    try_dec!(JxlDecoderGetICCProfileSize(
        dec,
        pixel_format,
        JXL_COLOR_PROFILE_TARGET_DATA,
        &mut icc_size
    ));
    result.color_profile.resize(icc_size, 0);
    try_dec!(JxlDecoderGetColorAsICCProfile(
        dec,
        pixel_format,
        JXL_COLOR_PROFILE_TARGET_DATA,
        result.color_profile.as_mut_ptr(),
        icc_size
    ));
    Ok(())
}

unsafe fn prepare_frame(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeResult,
) -> Result<(), &'static str> {
    let mut header = JxlFrameHeader::default();
    try_dec!(JxlDecoderGetFrameHeader(dec, &mut header));

    let mut name_vec: Vec<u8> = Vec::new();
    name_vec.resize((header.name_length + 1) as usize, 0);
    try_dec!(JxlDecoderGetFrameName(
        dec,
        name_vec.as_mut_ptr() as *mut _,
        name_vec.len()
    ));

    name_vec.pop(); // The string ends with null which is redundant in Rust

    let frame = Frame {
        name: String::from_utf8(name_vec).map_err(|_| "Couldn't decode frame name")?,
        duration: header.duration,
        timecode: header.timecode,
        is_last: header.is_last != 0,
        ..Default::default()
    };
    result.frames.push(frame);
    Ok(())
}

unsafe fn prepare_preview_out_buffer(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeResult,
    pixel_format: &JxlPixelFormat,
) -> Result<(), &'static str> {
    let mut buffer_size = 0usize;
    try_dec!(JxlDecoderPreviewOutBufferSize(
        dec,
        pixel_format,
        &mut buffer_size
    ));

    if buffer_size != (result.basic_info.xsize * result.basic_info.ysize * 4) as usize {
        return Err("Invalid preview out buffer size");
    }

    let buffer = &mut result.preview;

    buffer.resize(buffer_size as usize, 0);
    try_dec!(JxlDecoderSetPreviewOutBuffer(
        dec,
        pixel_format,
        buffer.as_mut_ptr() as *mut _,
        buffer_size,
    ));
    Ok(())
}

unsafe fn prepare_dc_out_buffer(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeResult,
    pixel_format: &JxlPixelFormat,
) -> Result<(), &'static str> {
    let mut buffer_size = 0usize;
    try_dec!(JxlDecoderDCOutBufferSize(
        dec,
        pixel_format,
        &mut buffer_size
    ));

    if buffer_size > (result.basic_info.xsize * result.basic_info.ysize * 4) as usize {
        return Err("DC out buffer size is unexpectedly larger than the full buffer size");
    }

    let buffer = &mut result
        .frames
        .last_mut()
        .expect("Frames vector is unexpectedly empty")
        .dc;

    buffer.resize(buffer_size as usize, 0);
    try_dec!(JxlDecoderSetDCOutBuffer(
        dec,
        pixel_format,
        buffer.as_mut_ptr() as *mut _,
        buffer_size,
    ));
    Ok(())
}

unsafe fn prepare_image_out_buffer(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeResult,
    pixel_format: &JxlPixelFormat,
) -> Result<(), &'static str> {
    let mut buffer_size = 0usize;
    try_dec!(JxlDecoderImageOutBufferSize(
        dec,
        pixel_format,
        &mut buffer_size
    ));

    if buffer_size != (result.basic_info.xsize * result.basic_info.ysize * 4) as usize {
        return Err("Invalid out buffer size");
    }

    let buffer = &mut result
        .frames
        .last_mut()
        .expect("Frames vector is unexpectedly empty")
        .data;

    buffer.resize(buffer_size as usize, 0);
    try_dec!(JxlDecoderSetImageOutBuffer(
        dec,
        pixel_format,
        buffer.as_mut_ptr() as *mut _,
        buffer_size,
    ));
    Ok(())
}

unsafe fn decode_loop(
    dec: *mut JxlDecoderStruct,
    data: impl BufRead,
    pixel_format: &JxlPixelFormat,
    event_flags: JxlDecoderStatus,
    max_frames: Option<usize>,
    allow_partial: bool,
) -> Result<DecodeResult, &'static str> {
    try_dec!(JxlDecoderSubscribeEvents(dec, event_flags as i32));

    let mut buffer = ContiguousBuffer::new(data);
    try_dec!(JxlDecoderSetInput(dec, buffer.as_ptr(), buffer.len()));

    let mut result = DecodeResult::default();

    loop {
        let status = JxlDecoderProcessInput(dec);

        match status {
            JXL_DEC_ERROR => return Err("Decoder error"),
            JXL_DEC_NEED_MORE_INPUT => {
                let remaining = JxlDecoderReleaseInput(dec);
                let consumed = buffer.len() - remaining;
                buffer.consume(consumed);

                if buffer.more_buf().is_err() {
                    if allow_partial {
                        prepare_image_out_buffer(dec, &mut result, pixel_format)?;
                        try_dec!(JxlDecoderFlushImage(dec));
                        break;
                    } else {
                        return Err("Couldn't read more buffer");
                    }
                }

                try_dec!(JxlDecoderSetInput(dec, buffer.as_ptr(), buffer.len()));
            }

            JXL_DEC_BASIC_INFO => read_basic_info(dec, &mut result)?,

            JXL_DEC_COLOR_ENCODING => read_color_encoding(dec, &mut result, pixel_format)?,

            JXL_DEC_FRAME => prepare_frame(dec, &mut result)?,

            JXL_DEC_NEED_PREVIEW_OUT_BUFFER => {
                prepare_preview_out_buffer(dec, &mut result, pixel_format)?
            }

            JXL_DEC_NEED_DC_OUT_BUFFER => prepare_dc_out_buffer(dec, &mut result, pixel_format)?,

            // Get the output buffer
            JXL_DEC_NEED_IMAGE_OUT_BUFFER => {
                prepare_image_out_buffer(dec, &mut result, pixel_format)?
            }

            JXL_DEC_DC_IMAGE => continue,
            JXL_DEC_FULL_IMAGE => {
                // Nothing to do. Do not yet return. If the image is an animation, more
                // full frames may be decoded.
                if max_frames.is_some() && result.frames.len() == max_frames.unwrap() {
                    break;
                }
            }
            JXL_DEC_SUCCESS => {
                // All decoding successfully finished.
                break;
            }
            _ => return Err("Unknown decoder status"),
        }
    }

    Ok(result)
}

fn get_event_subscription_flags(dec: &Decoder) -> JxlDecoderStatus {
    let mut flags: JxlDecoderStatus = JXL_DEC_BASIC_INFO;
    if dec.need_color_profile {
        flags |= JXL_DEC_COLOR_ENCODING;
    }
    if dec.need_optional_preview {
        flags |= JXL_DEC_PREVIEW_IMAGE;
    }
    if dec.need_optional_dc_frame {
        flags |= JXL_DEC_FRAME | JXL_DEC_DC_IMAGE;
    }
    if !dec.no_full_frame {
        flags |= JXL_DEC_FRAME | JXL_DEC_FULL_IMAGE;
    }
    flags
}

unsafe fn prepare_decoder(
    dec: &Decoder,
    dec_raw: *mut JxlDecoderStruct,
    runner: *mut c_void,
) -> Result<(), &'static str> {
    if let Some(keep_orientation) = dec.keep_orientation {
        try_dec!(JxlDecoderSetKeepOrientation(
            dec_raw,
            keep_orientation as i32
        ));
    }
    try_dec!(JxlDecoderSetParallelRunner(
        dec_raw,
        Some(JxlThreadParallelRunner),
        runner
    ));
    Ok(())
}

pub unsafe fn decode_oneshot(
    data: impl BufRead,
    dec: &Decoder,
) -> Result<DecodeResult, &'static str> {
    let dec_raw = JxlDecoderCreate(std::ptr::null());

    // Multi-threaded parallel runner.
    let runner = JxlThreadParallelRunnerCreate(
        std::ptr::null(),
        JxlThreadParallelRunnerDefaultNumWorkerThreads(),
    );

    let preparation = prepare_decoder(dec, dec_raw, runner);
    if preparation.is_err() {
        JxlThreadParallelRunnerDestroy(runner);
        JxlDecoderDestroy(dec_raw);
        return Err("Couldn't prepare the decoder");
    }

    let event_flags = get_event_subscription_flags(dec);
    // TODO: Support different pixel format
    // Not sure how to type the output vector properly
    let pixel_format = JxlPixelFormat {
        num_channels: 4,
        data_type: JXL_TYPE_UINT8,
        endianness: JXL_NATIVE_ENDIAN,
        align: 0,
    };
    let result = decode_loop(
        dec_raw,
        data,
        &pixel_format,
        event_flags,
        dec.max_frames,
        dec.allow_partial,
    );

    JxlThreadParallelRunnerDestroy(runner);
    JxlDecoderDestroy(dec_raw);

    result
}

#[derive(Default)]
pub struct Decoder {
    pub keep_orientation: Option<bool>,

    // pub pixel_format: Option<JxlPixelFormat>,
    pub need_color_profile: bool,
    pub need_optional_preview: bool,
    pub need_optional_dc_frame: bool,
    pub no_full_frame: bool,

    /** Specify when you need at most N frames */
    pub max_frames: Option<usize>,
    /** Specify when partial input is expected */
    pub allow_partial: bool,
}

impl Decoder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decode(&self, data: &[u8]) -> Result<DecodeResult, &'static str> {
        // Just a helpful alias of decode_buffer for Vec which doesn't implement BufRead by itself
        self.decode_buffer(data)
    }

    pub fn decode_file(&self, file: &File) -> Result<DecodeResult, &'static str> {
        self.decode_buffer(BufReader::new(file))
    }

    pub fn decode_buffer(&self, buffer: impl BufRead) -> Result<DecodeResult, &'static str> {
        unsafe { decode_oneshot(buffer, self) }
    }
}

#[derive(Default)]
pub struct DecodeResult {
    pub basic_info: BasicInfo,
    /** Can be empty unless `need_color_profile` is specified */
    pub color_profile: Vec<u8>,
    /** Can be empty unless `need_optional_preview` is specified */
    pub preview: Vec<u8>,
    /** Can be empty if neither of `need_frame_header`, `need_dc_frame`, nor `need_frame` is specified */
    pub frames: Vec<Frame>,
}

#[derive(Default)]
pub struct Frame {
    pub name: String,
    pub duration: u32,
    pub timecode: u32,
    pub is_last: bool,

    /** Can be empty unless `need_dc_frame` is specified *and* there is a DC frame. */
    pub dc: Vec<u8>,
    /** Can be empty when `no_full_frame` is specified */
    pub data: Vec<u8>,
}
