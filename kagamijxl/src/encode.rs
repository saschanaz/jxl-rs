use std::{ffi::c_void, os::raw::c_int};

use libjxl_sys::*;

macro_rules! try_enc {
    ($left:expr, $right:expr) => {{
        if unsafe { $left } != JXL_ENC_SUCCESS {
            return Err($right);
        }
    }};
}

macro_rules! try_enc_fatal {
    ($left:expr) => {{
        if unsafe { $left } != JXL_ENC_SUCCESS {
            panic!("A fatal error occurred in kagamijxl::Encoder");
        }
    }};
}

#[derive(Debug)]
pub enum JxlEncodeError {
    UnsupportedValue(String),
}

unsafe fn encode_loop(enc: *mut JxlEncoderStruct) -> Vec<u8> {
    let mut compressed: Vec<u8> = Vec::new();
    compressed.resize(64, 0);
    let mut next_out = compressed.as_mut_ptr();
    let mut avail_out = compressed.len();
    loop {
        let process_result = JxlEncoderProcessOutput(enc, &mut next_out, &mut avail_out);
        match process_result {
            JXL_ENC_NEED_MORE_OUTPUT => {
                let offset = next_out.offset_from(compressed.as_ptr());
                compressed.resize(compressed.len() * 2, 0);
                next_out = compressed.as_mut_ptr().offset(offset);
                avail_out = compressed.len() - offset as usize;
            }
            JXL_ENC_SUCCESS => {
                compressed.resize(compressed.len() - avail_out, 0);
                return compressed;
            }

            JXL_ENC_ERROR => panic!("Encoder reported an unexpected error during processing"),
            _ => panic!("Unknown JXL encoding status found: {}", process_result),
        }
    }
}

fn prepare_encoder(
    enc: &Encoder,
    enc_raw: *mut JxlEncoderStruct,
    basic_info: &JxlBasicInfo,
    runner: *mut c_void,
    frame: &dyn InputFrame,
) -> Result<(), JxlEncodeError> {
    try_enc_fatal!(JxlEncoderSetParallelRunner(
        enc_raw,
        Some(JxlThreadParallelRunner),
        runner
    ));

    try_enc_fatal!(JxlEncoderSetBasicInfo(enc_raw, basic_info));

    let mut color_encoding = JxlColorEncoding::default();
    unsafe { JxlColorEncodingSetToSRGB(&mut color_encoding, 0) };
    try_enc_fatal!(JxlEncoderSetColorEncoding(enc_raw, &color_encoding));

    let options = enc.create_options(enc_raw)?;

    match frame.get_type() {
        FrameType::Bitmap => {
            let pixel_format = JxlPixelFormat {
                num_channels: 4,
                data_type: JXL_TYPE_UINT8,
                endianness: JXL_NATIVE_ENDIAN,
                align: 0,
            };
            try_enc_fatal!(JxlEncoderAddImageFrame(
                options,
                &pixel_format,
                frame.get_data().as_ptr() as *mut std::ffi::c_void,
                frame.get_data().len(),
            ));
        }
        FrameType::Jpeg => {
            try_enc_fatal!(JxlEncoderStoreJPEGMetadata(enc_raw, 1));
            try_enc_fatal!(JxlEncoderAddJPEGFrame(
                options,
                frame.get_data().as_ptr(),
                frame.get_data().len(),
            ))
        }
    }

    unsafe { JxlEncoderCloseInput(enc_raw) };

    Ok(())
}

pub unsafe fn encode_oneshot(
    frame: &dyn InputFrame,
    enc: &Encoder,
) -> Result<Vec<u8>, JxlEncodeError> {
    let runner = JxlThreadParallelRunnerCreate(
        std::ptr::null(),
        JxlThreadParallelRunnerDefaultNumWorkerThreads(),
    );

    let enc_raw = JxlEncoderCreate(std::ptr::null());

    prepare_encoder(enc, enc_raw, &enc.basic_info, runner, frame)?;

    let result = encode_loop(enc_raw);

    JxlThreadParallelRunnerDestroy(runner);
    JxlEncoderDestroy(enc_raw);

    Ok(result)
}

pub enum FrameType {
    Bitmap,
    Jpeg,
}

pub trait InputFrame<'a> {
    fn get_type(&self) -> FrameType;
    fn get_data(&self) -> &'a [u8];
}

pub struct BitmapFrame<'a> {
    pub data: &'a [u8],
}

impl<'a> InputFrame<'a> for BitmapFrame<'a> {
    fn get_type(&self) -> FrameType {
        FrameType::Bitmap
    }

    fn get_data(&self) -> &'a [u8] {
        self.data
    }
}

pub struct JpegFrame<'a> {
    pub data: &'a [u8],
}

impl<'a> InputFrame<'a> for JpegFrame<'a> {
    fn get_type(&self) -> FrameType {
        FrameType::Jpeg
    }

    fn get_data(&self) -> &'a [u8] {
        self.data
    }
}

pub struct Encoder {
    pub lossless: Option<bool>,
    pub effort: Option<i32>,
    pub distance: Option<f32>,
    pub basic_info: JxlBasicInfo,
}

impl Encoder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    fn create_options(
        &self,
        enc_raw: *mut JxlEncoderStruct,
    ) -> Result<*mut JxlEncoderFrameSettings, JxlEncodeError> {
        let options = unsafe { JxlEncoderOptionsCreate(enc_raw, std::ptr::null()) };

        if let Some(lossless) = self.lossless {
            try_enc_fatal!(JxlEncoderOptionsSetLossless(options, lossless as i32));
        }
        if let Some(effort) = self.effort {
            try_enc!(
                JxlEncoderOptionsSetEffort(options, effort as c_int),
                JxlEncodeError::UnsupportedValue(format!("Effort value {} is unsupported", effort))
            );
        }
        if let Some(distance) = self.distance {
            try_enc!(
                JxlEncoderOptionsSetDistance(options, distance),
                JxlEncodeError::UnsupportedValue(format!(
                    "Distance value {} is unsupported",
                    distance
                ))
            );
        }

        Ok(options)
    }

    pub fn encode(&self, data: &[u8]) -> Result<Vec<u8>, JxlEncodeError> {
        let frame = BitmapFrame { data };
        self.encode_frame(&frame)
    }

    pub fn encode_frame(&self, frame: &dyn InputFrame) -> Result<Vec<u8>, JxlEncodeError> {
        unsafe { encode_oneshot(frame, self) }
    }
}

impl Default for Encoder {
    fn default() -> Self {
        let mut basic_info = JxlBasicInfo::default();
        unsafe {
            JxlEncoderInitBasicInfo(&mut basic_info);
        }
        basic_info.alpha_bits = 8;
        basic_info.num_extra_channels = 1;
        basic_info.uses_original_profile = true as _;

        Self {
            lossless: None,
            effort: None,
            distance: None,
            basic_info,
        }
    }
}
