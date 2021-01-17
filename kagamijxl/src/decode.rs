use libjxl_sys::*;

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
) -> Result<(JxlBasicInfo, Vec<u8>, Vec<u8>), &'static str> {
    let next_in = &mut data.as_ptr();
    let mut avail_in = data.len();

    let pixel_format = JxlPixelFormat {
        num_channels: 4,
        data_type: JXL_TYPE_UINT8,
        endianness: JXL_NATIVE_ENDIAN,
        align: 0,
    };

    let mut basic_info = JxlBasicInfo::default();
    let mut pixels_buffer: Vec<u8> = Vec::new();
    let mut icc_profile: Vec<u8> = Vec::new();

    try_dec!(JxlDecoderSubscribeEvents(
        dec,
        (JXL_DEC_BASIC_INFO | JXL_DEC_COLOR_ENCODING | JXL_DEC_FULL_IMAGE) as i32
    ));

    loop {
        let status = JxlDecoderProcessInput(dec, next_in, &mut avail_in);

        match status {
            JXL_DEC_ERROR => return Err("Decoder error"),
            JXL_DEC_NEED_MORE_INPUT => return Err("Error, already provided all input"),

            // Get the basic info
            JXL_DEC_BASIC_INFO => {
                try_dec!(JxlDecoderGetBasicInfo(dec, &mut basic_info));
            }

            JXL_DEC_COLOR_ENCODING => {
                // Get the ICC color profile of the pixel data
                let mut icc_size = 0usize;
                try_dec!(JxlDecoderGetICCProfileSize(
                    dec,
                    &pixel_format,
                    JXL_COLOR_PROFILE_TARGET_DATA,
                    &mut icc_size
                ));
                icc_profile.resize(icc_size, 0);
                try_dec!(JxlDecoderGetColorAsICCProfile(
                    dec,
                    &pixel_format,
                    JXL_COLOR_PROFILE_TARGET_DATA,
                    icc_profile.as_mut_ptr(),
                    icc_size
                ));
            }

            // Get the output buffer
            JXL_DEC_NEED_IMAGE_OUT_BUFFER => {
                let mut buffer_size = 0usize;
                try_dec!(JxlDecoderImageOutBufferSize(
                    dec,
                    &pixel_format,
                    &mut buffer_size
                ));

                if buffer_size != (basic_info.xsize * basic_info.ysize * 4) as usize {
                    return Err("Invalid out buffer size");
                }

                pixels_buffer.resize(buffer_size as usize, 0);
                try_dec!(JxlDecoderSetImageOutBuffer(
                    dec,
                    &pixel_format,
                    pixels_buffer.as_mut_ptr() as *mut std::ffi::c_void,
                    buffer_size,
                ));
            }

            JXL_DEC_FULL_IMAGE => {
                // Nothing to do. Do not yet return. If the image is an animation, more
                // full frames may be decoded. This example only keeps the last one.
                continue;
            }
            JXL_DEC_SUCCESS => {
                // All decoding successfully finished.
                return Ok((basic_info, pixels_buffer, icc_profile));
            }
            _ => return Err("Unknown decoder status"),
        }
    }
}

pub unsafe fn decode_oneshot(
    data: &[u8],
    dec: *mut JxlDecoderStruct,
) -> Result<(JxlBasicInfo, Vec<u8>, Vec<u8>), &'static str> {
    // Multi-threaded parallel runner.
    let runner = JxlThreadParallelRunnerCreate(
        std::ptr::null(),
        JxlThreadParallelRunnerDefaultNumWorkerThreads(),
    );

    if JXL_DEC_SUCCESS != JxlDecoderSetParallelRunner(dec, Some(JxlThreadParallelRunner), runner) {
        JxlThreadParallelRunnerDestroy(runner);
        JxlDecoderDestroy(dec);
        return Err("JxlDecoderSubscribeEvents failed");
    }

    let result = decode_loop(dec, data);

    JxlThreadParallelRunnerDestroy(runner);
    JxlDecoderDestroy(dec);

    result
}

#[derive(Default)]
pub struct Decoder {
    pub keep_orientation: Option<bool>
}

impl Decoder {
    pub fn decode(&self, data: &[u8]) -> Result<(JxlBasicInfo, Vec<u8>, Vec<u8>), &'static str> {
        unsafe {
            let dec = JxlDecoderCreate(std::ptr::null());
            if let Some(keep_orientation) = self.keep_orientation {
                JxlDecoderSetKeepOrientation(dec, keep_orientation as i32);
            }
            decode_oneshot(data, dec)
        }
    }
}

pub struct DecoderState {
    dec: *mut JxlDecoderStruct

}

impl DecoderState {
    pub fn get_basic_info() {

    }
    pub fn get_first_frame() {

    }
}
