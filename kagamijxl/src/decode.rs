use libjxl_sys::*;
pub use libjxl_sys::{JxlBasicInfo as BasicInfo};

macro_rules! try_dec {
    ($left:expr) => {{
        if $left != JXL_DEC_SUCCESS {
            return Err("Decoder failed");
        }
    }};
}

unsafe fn decode_loop(
    dec: *mut JxlDecoderStruct,
    data: &[u8],
    pixel_format: &JxlPixelFormat,
    event_flags: JxlDecoderStatus,
) -> Result<DecodeResult, &'static str> {
    try_dec!(JxlDecoderSubscribeEvents(dec, event_flags as i32));

    JxlDecoderSetInput(dec, data.as_ptr(), data.len());

    let mut result = DecodeResult::default();

    loop {
        let status = JxlDecoderProcessInput(dec);

        match status {
            JXL_DEC_ERROR => return Err("Decoder error"),
            JXL_DEC_NEED_MORE_INPUT => return Err("Error, already provided all input"),

            // Get the basic info
            JXL_DEC_BASIC_INFO => {
                try_dec!(JxlDecoderGetBasicInfo(dec, &mut result.basic_info));
            }

            JXL_DEC_COLOR_ENCODING => {
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
            }

            JXL_DEC_FRAME => {
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
            }

            JXL_DEC_NEED_PREVIEW_OUT_BUFFER => {
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
            }

            JXL_DEC_NEED_DC_OUT_BUFFER => {
                let mut buffer_size = 0usize;
                try_dec!(JxlDecoderDCOutBufferSize(
                    dec,
                    pixel_format,
                    &mut buffer_size
                ));

                if buffer_size > (result.basic_info.xsize * result.basic_info.ysize * 4) as usize {
                    return Err(
                        "DC out buffer size is unexpectedly larger than the full buffer size",
                    );
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
            }

            // Get the output buffer
            JXL_DEC_NEED_IMAGE_OUT_BUFFER => {
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
            }

            JXL_DEC_DC_IMAGE => continue,
            JXL_DEC_FULL_IMAGE => {
                // Nothing to do. Do not yet return. If the image is an animation, more
                // full frames may be decoded. This example only keeps the last one.
                continue;
            }
            JXL_DEC_SUCCESS => {
                // All decoding successfully finished.
                return Ok(result);
            }
            _ => return Err("Unknown decoder status"),
        }
    }
}

fn get_event_subscription_flags(dec: &Decoder) -> JxlDecoderStatus {
    let mut flags: JxlDecoderStatus = JXL_DEC_BASIC_INFO;
    if dec.need_color_profile {
        flags |= JXL_DEC_COLOR_ENCODING;
    }
    if dec.need_optional_preview {
        flags |= JXL_DEC_PREVIEW_IMAGE;
    }
    if dec.need_dc_frame {
        flags |= JXL_DEC_FRAME | JXL_DEC_DC_IMAGE;
    }
    if !dec.no_full_frame {
        flags |= JXL_DEC_FRAME | JXL_DEC_FULL_IMAGE;
    }
    flags
}

pub unsafe fn decode_oneshot(data: &[u8], dec: &Decoder) -> Result<DecodeResult, &'static str> {
    let dec_raw = JxlDecoderCreate(std::ptr::null());
    if let Some(keep_orientation) = dec.keep_orientation {
        JxlDecoderSetKeepOrientation(dec_raw, keep_orientation as i32);
    }

    // Multi-threaded parallel runner.
    let runner = JxlThreadParallelRunnerCreate(
        std::ptr::null(),
        JxlThreadParallelRunnerDefaultNumWorkerThreads(),
    );

    if JXL_DEC_SUCCESS
        != JxlDecoderSetParallelRunner(dec_raw, Some(JxlThreadParallelRunner), runner)
    {
        JxlThreadParallelRunnerDestroy(runner);
        JxlDecoderDestroy(dec_raw);
        return Err("JxlDecoderSubscribeEvents failed");
    }

    let event_flags = get_event_subscription_flags(&dec);
    // TODO: Support different pixel format
    // Not sure how to type the output vector properly
    let pixel_format = JxlPixelFormat {
        num_channels: 4,
        data_type: JXL_TYPE_UINT8,
        endianness: JXL_NATIVE_ENDIAN,
        align: 0,
    };
    let result = decode_loop(dec_raw, data, &pixel_format, event_flags);

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
    pub need_dc_frame: bool,
    pub no_full_frame: bool,
}

impl Decoder {
    pub fn decode(&self, data: &[u8]) -> Result<DecodeResult, &'static str> {
        unsafe { decode_oneshot(data, &self) }
    }

    // decode_iter()?
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
